use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use nostr::{
    event::{Event, EventBuilder, Kind},
    filter::{Filter, MatchEventOptions},
    message::{RelayMessage, SubscriptionId},
    nips::nip44,
};
use nostr_relay_pool::RelayPoolNotification;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;

use crate::{
    protocol::{LocalKeypair, model::event_kinds::SUBKEY_PROOF},
    router::{
        CleartextEvent, Conversation, ConversationError, ConversationMessage, NotificationStream,
        PortalId, RelayNode, Response, channel::Channel,
    },
};

type ConversationBox = Box<dyn Conversation + Send + Sync>;

impl std::fmt::Debug for ConversationBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conversation").finish()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MessageRouterActorError {
    #[error("Channel error: {0}")]
    Channel(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("Conversation error: {0}")]
    Conversation(#[from] ConversationError),
    #[error("Receiver error: {0}")]
    Receiver(#[from] oneshot::error::RecvError),
}

#[derive(Debug)]
pub enum MessageRouterActorMessage {
    AddRelay(String, bool, oneshot::Sender<Result<(), ConversationError>>),
    RemoveRelay(String, oneshot::Sender<Result<(), ConversationError>>),
    Shutdown(oneshot::Sender<Result<(), ConversationError>>),
    AddConversation(
        ConversationBox,
        oneshot::Sender<Result<PortalId, ConversationError>>,
    ),
    AddConversationWithRelays(
        ConversationBox,
        Vec<String>,
        oneshot::Sender<Result<PortalId, ConversationError>>,
    ),
    SubscribeToServiceRequest(
        PortalId,
        oneshot::Sender<Result<NotificationStream<serde_json::Value>, ConversationError>>,
    ),
    AddAndSubscribe(
        ConversationBox,
        oneshot::Sender<Result<NotificationStream<serde_json::Value>, ConversationError>>,
    ),
    Ping(oneshot::Sender<()>),

    /// This is used to handle relay pool notifications.
    HandleRelayPoolNotification(RelayPoolNotification),
}

pub struct MessageRouterActor<C>
where
    C: Channel + Send + Sync + 'static,
    C::Error: From<nostr::types::url::Error>,
{
    channel: Arc<C>,
    keypair: LocalKeypair,
    sender: mpsc::Sender<MessageRouterActorMessage>,
}

impl<C> MessageRouterActor<C>
where
    C: Channel + Send + Sync + 'static,
    C::Error: From<nostr::types::url::Error>,
{
    pub fn new(channel: C, keypair: LocalKeypair) -> Self {
        let keypair_clone = keypair.clone();
        let channel = Arc::new(channel);

        let (tx, mut rx) = mpsc::channel(4096);

        let channel_clone = Arc::clone(&channel);
        tokio::spawn(async move {
            let mut state = MessageRouterActorState::new(keypair_clone);
            while let Some(message) = rx.recv().await {
                match message {
                    MessageRouterActorMessage::AddRelay(
                        url,
                        subscribe_existing_conversations,
                        response_tx,
                    ) => {
                        let result = state
                            .add_relay(
                                &channel_clone,
                                url.clone(),
                                subscribe_existing_conversations,
                            )
                            .await;
                        if let Err(e) = response_tx.send(result) {
                            log::error!("Failed to send AddRelay({}) response: {:?}", url, e);
                        }
                    }
                    MessageRouterActorMessage::RemoveRelay(url, response_tx) => {
                        let result = state.remove_relay(&channel_clone, url.clone()).await;
                        if let Err(e) = response_tx.send(result) {
                            log::error!("Failed to send RemoveRelay({}) response: {:?}", url, e);
                        }
                    }
                    MessageRouterActorMessage::Shutdown(response_tx) => {
                        let result = state.shutdown(&channel_clone).await;
                        if let Err(e) = response_tx.send(result) {
                            log::error!("Failed to send Shutdown response: {:?}", e);
                        }
                        break;
                    }
                    MessageRouterActorMessage::AddConversation(conversation, response_tx) => {
                        let result = state.add_conversation(&channel_clone, conversation).await;
                        if let Err(e) = response_tx.send(result) {
                            log::error!("Failed to send AddConversation response: {:?}", e);
                        }
                    }
                    MessageRouterActorMessage::AddConversationWithRelays(
                        conversation,
                        relays,
                        response_tx,
                    ) => {
                        let result = state
                            .add_conversation_with_relays(&channel_clone, conversation, relays)
                            .await;
                        if let Err(e) = response_tx.send(result) {
                            log::error!(
                                "Failed to send AddConversationWithRelays response: {:?}",
                                e
                            );
                        }
                    }
                    MessageRouterActorMessage::SubscribeToServiceRequest(id, response_tx) => {
                        let result = state.subscribe_to_service_request(id);
                        if let Err(e) = response_tx.send(result) {
                            log::error!(
                                "Failed to send SubscribeToServiceRequest response: {:?}",
                                e
                            );
                        }
                    }
                    MessageRouterActorMessage::AddAndSubscribe(conversation, response_tx) => {
                        let result = state
                            .add_and_subscribe::<_, serde_json::Value>(&channel_clone, conversation)
                            .await;
                        if let Err(e) = response_tx.send(result) {
                            log::error!("Failed to send AddAndSubscribe response: {:?}", e);
                        }
                    }
                    MessageRouterActorMessage::Ping(response_tx) => {
                        let _ = response_tx.send(());
                    }

                    MessageRouterActorMessage::HandleRelayPoolNotification(notification) => {
                        // Handle notification directly without response channel
                        if let Err(e) = state
                            .handle_relay_pool_notification(&channel_clone, notification)
                            .await
                        {
                            log::error!("Failed to handle relay pool notification: {:?}", e);
                        }
                    }
                }
            }
        });

        Self {
            channel: Arc::clone(&channel),
            keypair,
            sender: tx,
        }
    }

    pub fn channel(&self) -> Arc<C> {
        Arc::clone(&self.channel)
    }

    pub fn keypair(&self) -> &LocalKeypair {
        &self.keypair
    }

    pub async fn listen(&self) -> Result<(), MessageRouterActorError> {
        while let Ok(notification) = self.channel.receive().await {
            // Send notification directly without oneshot channel
            if let Err(e) = self
                .sender
                .send(MessageRouterActorMessage::HandleRelayPoolNotification(
                    notification,
                ))
                .await
            {
                log::error!("Failed to send HandleRelayPoolNotification: {:?}", e);
                break;
            }
        }
        Ok(())
    }

    // Helper method to reduce channel cloning
    async fn send_message(
        &self,
        message: MessageRouterActorMessage,
    ) -> Result<(), MessageRouterActorError> {
        self.sender
            .send(message)
            .await
            .map_err(|e| MessageRouterActorError::Channel(Box::new(e)))
    }

    pub async fn add_relay(
        &self,
        url: String,
        subscribe_existing_conversations: bool,
    ) -> Result<(), MessageRouterActorError> {
        let (tx, rx) = oneshot::channel();
        self.send_message(MessageRouterActorMessage::AddRelay(
            url,
            subscribe_existing_conversations,
            tx,
        ))
        .await?;
        let result: Result<(), ConversationError> =
            rx.await.map_err(|e| MessageRouterActorError::Receiver(e))?;
        result.map_err(MessageRouterActorError::Conversation)
    }

    pub async fn remove_relay(&self, url: String) -> Result<(), MessageRouterActorError> {
        let (tx, rx) = oneshot::channel();
        self.send_message(MessageRouterActorMessage::RemoveRelay(url, tx))
            .await?;
        let result: Result<(), ConversationError> =
            rx.await.map_err(|e| MessageRouterActorError::Receiver(e))?;
        result.map_err(MessageRouterActorError::Conversation)
    }

    pub async fn shutdown(&self) -> Result<(), MessageRouterActorError> {
        let (tx, rx) = oneshot::channel();
        self.send_message(MessageRouterActorMessage::Shutdown(tx))
            .await?;
        let result: Result<(), ConversationError> =
            rx.await.map_err(|e| MessageRouterActorError::Receiver(e))?;
        result.map_err(MessageRouterActorError::Conversation)
    }

    pub async fn ping(&self) -> Result<(), MessageRouterActorError> {
        let (tx, rx) = oneshot::channel();
        self.send_message(MessageRouterActorMessage::Ping(tx))
            .await?;
        let result = rx.await.map_err(|e| MessageRouterActorError::Receiver(e))?;
        Ok(result)
    }

    pub async fn add_conversation(
        &self,
        conversation: ConversationBox,
    ) -> Result<PortalId, MessageRouterActorError> {
        self.ping().await?;
        self.ping().await?;

        let (tx, rx) = oneshot::channel();
        self.send_message(MessageRouterActorMessage::AddConversation(conversation, tx))
            .await?;
        let result = rx.await.map_err(|e| MessageRouterActorError::Receiver(e))?;
        result.map_err(MessageRouterActorError::Conversation)
    }

    pub async fn add_conversation_with_relays(
        &self,
        conversation: ConversationBox,
        relays: Vec<String>,
    ) -> Result<PortalId, MessageRouterActorError> {
        let (tx, rx) = oneshot::channel();
        self.send_message(MessageRouterActorMessage::AddConversationWithRelays(
            conversation,
            relays,
            tx,
        ))
        .await?;
        let result = rx.await.map_err(|e| MessageRouterActorError::Receiver(e))?;
        result.map_err(MessageRouterActorError::Conversation)
    }

    /// Subscribes to notifications from a conversation with a specific type.
    ///
    /// # Type Parameters
    /// * `T` - The type of notifications to receive, must implement `DeserializeOwned` and `Serialize`
    ///
    /// # Arguments
    /// * `id` - The ID of the conversation to subscribe to
    ///
    /// # Returns
    /// * `Ok(NotificationStream<T>)` - A stream of notifications from the conversation
    /// * `Err(MessageRouterActorError)` if an error occurs during subscription
    pub async fn subscribe_to_service_request<T: DeserializeOwned + Serialize>(
        &self,
        id: PortalId,
    ) -> Result<NotificationStream<T>, MessageRouterActorError> {
        // For the actor pattern, we need to use the raw stream and convert it
        let raw_stream = self.subscribe_to_service_request_raw(id).await?;

        // Convert the stream from serde_json::Value to T
        let NotificationStream { stream } = raw_stream;
        let typed_stream =
            stream.map(|result| result.and_then(|value| serde_json::from_value(value)));

        Ok(NotificationStream::new(typed_stream))
    }

    /// Subscribes to notifications from a conversation with raw JSON values.
    ///
    /// This is the internal method used by the actor pattern.
    ///
    /// # Arguments
    /// * `id` - The ID of the conversation to subscribe to
    ///
    /// # Returns
    /// * `Ok(NotificationStream<serde_json::Value>)` - A stream of raw JSON notifications
    /// * `Err(MessageRouterActorError)` if an error occurs during subscription
    async fn subscribe_to_service_request_raw(
        &self,
        id: PortalId,
    ) -> Result<NotificationStream<serde_json::Value>, MessageRouterActorError> {
        let (tx, rx) = oneshot::channel();
        self.send_message(MessageRouterActorMessage::SubscribeToServiceRequest(id, tx))
            .await?;
        let result = rx.await.map_err(|e| MessageRouterActorError::Receiver(e))?;
        result.map_err(MessageRouterActorError::Conversation)
    }

    /// Adds a conversation and subscribes to its notifications in a single operation (typed).
    pub async fn add_and_subscribe<T: DeserializeOwned + Serialize>(
        &self,
        conversation: ConversationBox,
    ) -> Result<NotificationStream<T>, MessageRouterActorError> {
        let raw_stream = self.add_and_subscribe_raw(conversation).await?;
        let NotificationStream { stream } = raw_stream;
        let typed_stream =
            stream.map(|result| result.and_then(|value| serde_json::from_value(value)));
        Ok(NotificationStream::new(typed_stream))
    }

    /// Adds a conversation and subscribes to its notifications in a single operation (raw Value).
    async fn add_and_subscribe_raw(
        &self,
        conversation: ConversationBox,
    ) -> Result<NotificationStream<serde_json::Value>, MessageRouterActorError> {
        let (tx, rx) = oneshot::channel();
        self.send_message(MessageRouterActorMessage::AddAndSubscribe(conversation, tx))
            .await?;
        let result = rx.await.map_err(|e| MessageRouterActorError::Receiver(e))?;
        result.map_err(MessageRouterActorError::Conversation)
    }
}

pub struct MessageRouterActorState {
    keypair: LocalKeypair,
    conversations: HashMap<PortalId, ConversationBox>,
    aliases: HashMap<PortalId, Vec<u64>>,
    filters: HashMap<PortalId, Filter>,
    subscribers: HashMap<PortalId, Vec<mpsc::Sender<serde_json::Value>>>,
    end_of_stored_events: HashMap<PortalId, usize>,

    relay_nodes: HashMap<String, RelayNode>,
    global_relay_node: RelayNode,
}

impl MessageRouterActorState {
    pub fn new(keypair: LocalKeypair) -> Self {
        Self {
            keypair,
            conversations: HashMap::new(),
            aliases: HashMap::new(),
            filters: HashMap::new(),
            subscribers: HashMap::new(),
            end_of_stored_events: HashMap::new(),
            relay_nodes: HashMap::new(),
            global_relay_node: RelayNode::new(),
        }
    }

    pub async fn add_relay<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        url: String,
        subscribe_existing_conversations: bool,
    ) -> Result<(), ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        // Subscribe existing conversations to new relays
        if subscribe_existing_conversations {
            for conversation_id in self.global_relay_node.conversations.iter() {
                if let Some(filter) = self.filters.get(conversation_id) {
                    log::trace!("Subscribing {} to new relay = {:?}", conversation_id, &url);
                    channel
                        .subscribe_to(vec![url.clone()], conversation_id.clone(), filter.clone())
                        .await
                        .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                    self.end_of_stored_events
                        .get_mut(conversation_id)
                        .and_then(|v| Some(*v += 1));
                }

                if let Some(aliases) = self.aliases.get(conversation_id) {
                    for alias in aliases {
                        let alias_id =
                            PortalId::new_conversation_alias(conversation_id.id(), *alias);
                        if let Some(filter) = self.filters.get(&alias_id) {
                            channel
                                .subscribe_to(vec![url.clone()], alias_id, filter.clone())
                                .await
                                .map_err(|e| ConversationError::Inner(Box::new(e)))?;
                        }
                    }
                }
            }
        }

        {
            self.relay_nodes
                .entry(url.clone())
                .or_insert_with(|| RelayNode::new());
        }

        Ok(())
    }

    pub async fn remove_relay<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        url: String,
    ) -> Result<(), ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        if let Some(node) = self.relay_nodes.remove(&url) {
            for conv in node.conversations.iter() {
                let relays_of_conversation = self.get_relays_by_conversation(conv)?;
                match relays_of_conversation {
                    Some(urls) => {
                        // If conversation it not present in other relays, clean it
                        if urls.is_empty() {
                            self.cleanup_conversation(channel, conv).await?;
                        }
                    }
                    None => {
                        // If conversation is present also on the global node relay, should'nt happen, dont clean it
                    }
                }

                self.end_of_stored_events
                    .get_mut(conv)
                    .and_then(|v| Some(*v = v.saturating_sub(1)));
            }
        }
        Ok(())
    }

    fn get_relays_by_conversation(
        &self,
        conversation_id: &PortalId,
    ) -> Result<Option<HashSet<String>>, ConversationError> {
        if self
            .global_relay_node
            .conversations
            .contains(conversation_id)
        {
            return Ok(None);
        }

        let mut relays = HashSet::new();
        for (url, node) in self.relay_nodes.iter() {
            if node.conversations.contains(conversation_id) {
                relays.insert(url.clone());
            }
        }

        Ok(Some(relays))
    }

    async fn cleanup_conversation<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        conversation: &PortalId,
    ) -> Result<(), ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        // Remove conversation state
        self.conversations.remove(conversation);
        self.subscribers.remove(conversation);
        self.filters.remove(conversation);
        self.end_of_stored_events.remove(conversation);
        let aliases = self.aliases.remove(conversation);

        // Remove from global relay node
        {
            self.global_relay_node.conversations.remove(conversation);
        }

        // Remove from specific relay node
        {
            for (_, relay_node) in self.relay_nodes.iter_mut() {
                relay_node.conversations.remove(conversation);
            }
        }

        // Remove filters from relays
        channel
            .unsubscribe(conversation.clone())
            .await
            .map_err(|e| ConversationError::Inner(Box::new(e)))?;

        if let Some(aliases) = aliases {
            for alias in aliases {
                let alias_id = PortalId::new_conversation_alias(conversation.id(), alias);
                channel
                    .unsubscribe(alias_id)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }
        }
        Ok(())
    }

    /// Shuts down the router and disconnects from all relays.
    pub async fn shutdown<C: Channel>(&mut self, channel: &Arc<C>) -> Result<(), ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        channel
            .shutdown()
            .await
            .map_err(|e| ConversationError::Inner(Box::new(e)))?;

        self.conversations.clear();
        self.subscribers.clear();
        self.aliases.clear();
        self.filters.clear();
        self.end_of_stored_events.clear();
        self.global_relay_node.conversations.clear();
        Ok(())
    }

    async fn handle_relay_pool_notification<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        notification: RelayPoolNotification,
    ) -> Result<(), ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        enum LocalEvent {
            Message(Event),
            EndOfStoredEvents,
        }
        log::trace!("Notification = {:?}", notification);

        let (subscription_id, event): (SubscriptionId, LocalEvent) = match notification {
            RelayPoolNotification::Message {
                message:
                    RelayMessage::Event {
                        subscription_id,
                        event,
                    },
                ..
            } => {
                log::debug!("Received event on subscription: {}", subscription_id);
                (
                    subscription_id.into_owned(),
                    LocalEvent::Message(event.into_owned()),
                )
            }
            RelayPoolNotification::Event {
                event,
                subscription_id,
                ..
            } => {
                log::debug!("Received event on subscription: {}", subscription_id);
                (subscription_id, LocalEvent::Message(*event))
            }
            RelayPoolNotification::Message {
                message: RelayMessage::EndOfStoredEvents(subscription_id),
                ..
            } => {
                // Parse the subscription ID to get the PortalId
                let portal_id = match PortalId::parse(subscription_id.as_str()) {
                    Some(id) => id,
                    None => {
                        log::warn!(
                            "Invalid subscription ID format for EOSE: {:?}",
                            subscription_id
                        );
                        return Ok(());
                    }
                };

                let remaining = self.end_of_stored_events.get_mut(&portal_id).and_then(|v| {
                    *v -= 1;
                    Some(*v)
                });

                log::trace!("{:?} EOSE left for {}", remaining, portal_id);

                if remaining == Some(0) {
                    self.end_of_stored_events.remove(&portal_id);
                    (subscription_id.into_owned(), LocalEvent::EndOfStoredEvents)
                } else {
                    return Ok(());
                }
            }
            _ => return Ok(()),
        };

        let message = match &event {
            LocalEvent::Message(event) => {
                log::debug!("Processing event: {:?}", event.id);
                if event.pubkey == self.keypair.public_key() && event.kind != Kind::Metadata {
                    log::trace!("Ignoring event from self");
                    return Ok(());
                }

                if !event.verify_signature() {
                    log::warn!("Invalid signature for event id: {:?}", event.id);
                    return Ok(());
                }

                if let Ok(content) =
                    nip44::decrypt(&self.keypair.secret_key(), &event.pubkey, &event.content)
                {
                    let cleartext = match CleartextEvent::new(&event, &content) {
                        Ok(cleartext) => cleartext,
                        Err(e) => {
                            log::warn!("Invalid JSON in event: {:?}", e);
                            return Ok(());
                        }
                    };

                    ConversationMessage::Cleartext(cleartext)
                } else if let Ok(cleartext) =
                    serde_json::from_str::<serde_json::Value>(&event.content)
                {
                    ConversationMessage::Cleartext(CleartextEvent::new_json(&event, cleartext))
                } else {
                    ConversationMessage::Encrypted(event.clone())
                }
            }
            LocalEvent::EndOfStoredEvents => ConversationMessage::EndOfStoredEvents,
        };

        self.dispatch_event(channel, subscription_id.clone(), message.clone())
            .await?;

        let mut to_cleanup = vec![];
        let mut other_conversations = vec![];

        // Check if there are other potential conversations to dispatch to
        for (id, filter) in self.filters.iter() {
            if id.to_string() == subscription_id.as_str() {
                continue;
            }

            match self.conversations.get(id) {
                Some(conv) if conv.is_expired() => {
                    to_cleanup.push(id.clone());
                    continue;
                }
                _ => {}
            }

            if let LocalEvent::Message(event) = &event {
                if filter.match_event(&event, MatchEventOptions::default()) {
                    other_conversations.push(id.clone());
                }
            }
        }

        for id in to_cleanup {
            self.cleanup_conversation(channel, &id).await?;
        }

        for id in other_conversations {
            self.dispatch_event(channel, SubscriptionId::new(id.clone()), message.clone())
                .await?;
        }
        Ok(())
    }

    async fn dispatch_event<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        subscription_id: SubscriptionId,
        message: ConversationMessage,
    ) -> Result<(), ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        let subscription_str = subscription_id.as_str();
        log::debug!("Dispatching event to subscription: {}", subscription_str);

        // Parse the subscription ID to get the PortalId
        let conversation_id = match PortalId::parse(subscription_str) {
            Some(id) => id,
            None => {
                log::warn!("Invalid subscription ID format: {:?}", subscription_str);
                return Ok(());
            }
        };

        log::debug!("Looking for conversation: {}", conversation_id);
        let response = match self.conversations.get_mut(&conversation_id) {
            Some(conv) => {
                log::debug!("Found conversation, processing message");
                match conv.on_message(message) {
                    Ok(response) => response,
                    Err(e) => {
                        log::warn!("Error in conversation id {}: {:?}", conversation_id, e);
                        Response::new().finish()
                    }
                }
            }
            None => {
                log::warn!("No conversation found for id: {}", conversation_id);
                channel
                    .unsubscribe(conversation_id)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                return Ok(());
            }
        };

        log::debug!("Processing response for conversation: {}", conversation_id);
        self.process_response(channel, &conversation_id, response)
            .await?;

        Ok(())
    }

    async fn process_response<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        id: &PortalId,
        response: Response,
    ) -> Result<(), ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        log::trace!("Processing response builder for {} = {:?}", id, response);

        let selected_relays_optional = self.get_relays_by_conversation(id)?;

        if !response.filter.is_empty() {
            log::debug!(
                "Adding filter for conversation {}: {:?}",
                id,
                response.filter
            );
            self.filters.insert(id.clone(), response.filter.clone());

            let num_relays = if let Some(selected_relays) = selected_relays_optional.clone() {
                let num_relays = selected_relays.len();
                log::trace!("Subscribing to relays = {:?}", selected_relays);
                channel
                    .subscribe_to(selected_relays, id.clone(), response.filter.clone())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                num_relays
            } else {
                log::trace!("Subscribing to all relays");
                channel
                    .subscribe(id.clone(), response.filter.clone())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?
            };

            self.end_of_stored_events.insert(id.clone(), num_relays);
        }

        let mut events_to_broadcast = vec![];
        for response_entry in response.responses.iter() {
            let build_event = |content: &str| {
                EventBuilder::new(response_entry.kind, content)
                    .tags(response_entry.tags.clone())
                    .sign_with_keys(&self.keypair)
                    .map_err(|e| ConversationError::Inner(Box::new(e)))
            };

            if !response_entry.encrypted {
                let content = serde_json::to_string(&response_entry.content)
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                let event = build_event(&content)?;
                events_to_broadcast.push(event);
            } else {
                for pubkey in response_entry.recepient_keys.iter() {
                    let content = nip44::encrypt(
                        &self.keypair.secret_key(),
                        &pubkey,
                        serde_json::to_string(&response_entry.content)
                            .map_err(|e| ConversationError::Inner(Box::new(e)))?,
                        nip44::Version::V2,
                    )
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                    let event = build_event(&content)?;
                    events_to_broadcast.push(event);
                }
            }
        }

        for notification in response.notifications.iter() {
            log::debug!("Sending notification: {:?}", notification);
            if let Some(senders) = self.subscribers.get_mut(id) {
                for sender in senders.iter_mut() {
                    let _ = sender.send(notification.clone()).await;
                }
            }
        }

        if response.subscribe_to_subkey_proofs {
            let alias_num = rand::random::<u64>();

            self.aliases.entry(id.clone()).or_default().push(alias_num);

            let filter = Filter::new()
                .kinds(vec![Kind::Custom(SUBKEY_PROOF)])
                .events(events_to_broadcast.iter().map(|e| e.id));

            let alias = PortalId::new_conversation_alias(id.id(), alias_num);
            self.filters.insert(alias.clone(), filter.clone());

            if let Some(selected_relays) = selected_relays_optional.clone() {
                channel
                    .subscribe_to(selected_relays, alias, filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            } else {
                // Subscribe to subkey proofs to all

                channel
                    .subscribe(alias, filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }
        }

        // check if Response has selected relays
        if let Some(selected_relays) = selected_relays_optional {
            for event in events_to_broadcast {
                // if selected relays, broadcast to selected relays
                channel
                    .broadcast_to(selected_relays.clone(), event)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }
        } else {
            for event in events_to_broadcast {
                // if not selected relays, broadcast to all relays
                channel
                    .broadcast(event)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }

            // TODO: wait for confirmation from relays
        }

        if response.finished {
            log::info!("Conversation {} finished, cleaning up", id);
            self.cleanup_conversation(channel, id).await?;
        }

        Ok(())
    }

    fn internal_add_with_id(
        &mut self,
        id: &PortalId,
        mut conversation: ConversationBox,
        relays: Option<Vec<String>>,
    ) -> Result<Response, ConversationError> {
        let response = conversation.init()?;

        if let Some(relays) = relays {
            // Update relays node
            // for each relay parameter
            for relay in relays {
                // get relay node associated

                match self.relay_nodes.get_mut(&relay) {
                    Some(found_node) => {
                        found_node.conversations.insert(id.clone());
                    }
                    None => {
                        return Err(ConversationError::RelayNotConnected(relay));
                    }
                }
            }
        } else {
            // Update Global Relay Node
            self.global_relay_node.conversations.insert(id.clone());
        }

        self.conversations.insert(id.clone(), conversation);

        Ok(response)
    }

    /// Adds a new conversation to the router.
    ///
    /// The conversation will be initialized and its initial response will be processed.
    ///
    /// # Arguments
    /// * `conversation` - The conversation to add
    ///
    /// # Returns
    /// * `Ok(PortalId)` - The ID of the added conversation
    /// * `Err(ConversationError)` if an error occurs during initialization
    pub async fn add_conversation<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        conversation: ConversationBox,
    ) -> Result<PortalId, ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        let conversation_id = PortalId::new_conversation();

        let response = self.internal_add_with_id(&conversation_id, conversation, None)?;
        self.process_response(channel, &conversation_id, response)
            .await?;

        Ok(conversation_id)
    }

    pub async fn add_conversation_with_relays<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        conversation: ConversationBox,
        relays: Vec<String>,
    ) -> Result<PortalId, ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        let conversation_id = PortalId::new_conversation();

        let response = self.internal_add_with_id(&conversation_id, conversation, Some(relays))?;
        self.process_response(channel, &conversation_id, response)
            .await?;

        Ok(conversation_id)
    }

    /// Subscribes to notifications from a conversation.
    ///
    /// # Type Parameters
    /// * `T` - The type of notifications to receive, must implement `DeserializeOwned` and `Serialize`
    ///
    /// # Arguments
    /// * `id` - The ID of the conversation to subscribe to
    ///
    /// # Returns
    /// * `Ok(NotificationStream<T>)` - A stream of notifications from the conversation
    /// * `Err(ConversationError)` if an error occurs during subscription
    pub fn subscribe_to_service_request<T: DeserializeOwned + Serialize>(
        &mut self,
        id: PortalId,
    ) -> Result<NotificationStream<T>, ConversationError> {
        let (tx, rx) = mpsc::channel(8);
        self.subscribers.entry(id).or_insert(Vec::new()).push(tx);

        let rx = tokio_stream::wrappers::ReceiverStream::new(rx);
        let rx = rx.map(|content| serde_json::from_value(content));
        let rx = NotificationStream::new(rx);

        Ok(rx)
    }

    /// Adds a conversation and subscribes to its notifications in a single operation.
    ///
    /// This is a convenience method that combines `add_conversation` and `subscribe_to_service_request`
    /// for conversations that implement `ConversationWithNotification`.
    ///
    /// It also performs the subscription *before* adding the conversation to the router,
    /// so the subscriber will not miss any notifications.
    ///
    /// # Type Parameters
    /// * `Conv` - The conversation type, must implement `ConversationWithNotification`
    ///
    /// # Arguments
    /// * `conversation` - The conversation to add
    ///
    /// # Returns
    /// * `Ok(NotificationStream<Conv::Notification>)` - A stream of notifications from the conversation
    /// * `Err(ConversationError)` if an error occurs during initialization or subscription
    pub async fn add_and_subscribe<C: Channel, T: DeserializeOwned + Serialize>(
        &mut self,
        channel: &Arc<C>,
        conversation: ConversationBox,
    ) -> Result<NotificationStream<T>, ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        let conversation_id = PortalId::new_conversation();

        // Subscribe before adding the conversation to ensure we don't miss notifications
        let (tx, rx) = mpsc::channel(8);
        self.subscribers
            .entry(conversation_id.clone())
            .or_insert(Vec::new())
            .push(tx);

        let rx = tokio_stream::wrappers::ReceiverStream::new(rx);
        let rx = rx.map(|content| serde_json::from_value(content));
        let rx = NotificationStream::new(rx);

        // Now add the conversation
        let response = self.internal_add_with_id(&conversation_id, conversation, None)?;
        self.process_response(channel, &conversation_id, response)
            .await?;

        Ok(rx)
    }
}
