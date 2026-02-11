use std::{
    collections::{HashMap, HashSet}, str::FromStr, sync::Arc
};

use nostr::{
    event::{Event, EventBuilder, Kind},
    filter::{Filter, MatchEventOptions},
    message::{RelayMessage, SubscriptionId},
    nips::nip44,
    types::RelayUrl,
};
use nostr_relay_pool::RelayPoolNotification;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;

use crate::{
    protocol::{LocalKeypair, model::event_kinds::SUBKEY_PROOF},
    router::{
        CleartextEvent, Conversation, ConversationError, ConversationMessage, NotificationStream, PortalConversationId, PortalSubscriptionId, Response, channel::Channel
    },
};

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
        oneshot::Sender<Result<PortalConversationId, ConversationError>>,
    ),
    AddConversationWithRelays(
        ConversationBox,
        Vec<String>,
        oneshot::Sender<Result<PortalConversationId, ConversationError>>,
    ),
    SubscribeToServiceRequest(
        PortalConversationId,
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

    pub async fn inject_event(&self, event: Event) -> Result<(), MessageRouterActorError> {
        self.sender
            .send(MessageRouterActorMessage::HandleRelayPoolNotification(
                RelayPoolNotification::Event {
                    event: Box::new(event),
                    subscription_id: SubscriptionId::new("".to_string()),
                    relay_url: RelayUrl::parse("wss://localhost").unwrap(),
                },
            ))
            .await
            .map_err(|e| MessageRouterActorError::Channel(Box::new(e)))?;
        Ok(())
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
    ) -> Result<PortalConversationId, MessageRouterActorError> {
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
    ) -> Result<PortalConversationId, MessageRouterActorError> {
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
        id: PortalConversationId,
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
        id: PortalConversationId,
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
    /// All conversation states
    conversations: HashMap<PortalConversationId, ConversationState>,
}

impl MessageRouterActorState {
    pub fn new(keypair: LocalKeypair) -> Self {
        Self {
            keypair,
            conversations: HashMap::new(),
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
        // Subscribe existing global conversations to new relay
        if subscribe_existing_conversations {
            // First, collect all global conversations and their aliases
            let mut global_conversations = Vec::new();
            let mut aliases_to_subscribe = Vec::new();

            for (conversation_id, conv_state) in self.conversations.iter() {
                if conv_state.is_global() {
                    if let Some(filter) = conv_state.filter.as_ref() {
                        global_conversations.push((conversation_id.clone(), filter.clone(), conv_state.subscription_id.clone()));
                    }

                    // Collect aliases
                    for alias in conv_state.aliases() {
                        if let Some(alias_state) = self.conversations.get(&alias) {
                            if let Some(filter) = &alias_state.filter {
                                aliases_to_subscribe.push((alias, filter.clone(), conv_state.subscription_id.clone()));
                            }
                        }
                    }
                }
            }

            // Subscribe to the new relay
            for (conversation_id, filter, subscription_id) in global_conversations {
                log::trace!("Subscribing {} to new relay = {:?}", conversation_id, &url);
                channel
                    .subscribe_to(vec![url.clone()], subscription_id.clone(), filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }

            for (_, filter, subscription_id) in aliases_to_subscribe {
                channel
                    .subscribe_to(vec![url.clone()], subscription_id.clone(), filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }

            // Increment EOSE counters for all global conversations
            for (conversation_id, conv_state) in self.conversations.iter_mut() {
                if conv_state.is_global() {
                    conv_state.increment_eose();
                    log::trace!(
                        "EOSE counter incremented for conversation {}",
                        conversation_id
                    );
                }
            }
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
        let mut conversations_to_cleanup = Vec::new();

        // Find conversations that are affected by this relay removal
        for (conversation_id, conv_state) in self.conversations.iter_mut() {
            if conv_state.relay_urls().contains(&url) {
                // Remove the relay from this conversation
                conv_state.remove_relay(&url);

                // Decrement EOSE counter
                conv_state.decrement_eose();

                // If conversation has no relays left and is not global, mark for cleanup
                if conv_state.relay_urls().is_empty() && !conv_state.is_global() {
                    conversations_to_cleanup.push(conversation_id.clone());
                }
            } else if conv_state.is_global() {
                // Global conversations are affected by any relay removal
                conv_state.decrement_eose();
            }
        }

        // Clean up conversations that have no relays left
        for conversation_id in conversations_to_cleanup {
            self.cleanup_conversation(channel, &conversation_id).await?;
        }

        Ok(())
    }

    fn get_relays_by_conversation(
        &self,
        conversation_id: &PortalConversationId,
    ) -> Result<Option<HashSet<String>>, ConversationError> {
        if let Some(conv_state) = self.conversations.get(conversation_id) {
            if conv_state.is_global() {
                return Ok(None); // Global conversations use all relays
            } else {
                return Ok(Some(conv_state.relay_urls().clone()));
            }
        }
        Err(ConversationError::ConversationNotFound)
    }

    async fn cleanup_conversation<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        conversation: &PortalConversationId,
    ) -> Result<(), ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        // Remove conversation state
        if let Some(conv_state) = self.conversations.remove(conversation) {
            // Remove filters from relays
            channel
                .unsubscribe(conv_state.subscription_id.clone())
                .await
                .map_err(|e| ConversationError::Inner(Box::new(e)))?;

            // Remove aliases
            for alias in conv_state.aliases() {
                channel
                    .unsubscribe(conv_state.subscription_id.clone())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                // Also remove the alias conversation state
                self.conversations.remove(alias);
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
                let portal_subscription_id = match PortalSubscriptionId::from_str(subscription_id.as_str()) {
                    Ok(id) => id,
                    Err(e) => {
                        log::warn!("Invalid subscription ID format: {:?}: {}", e, subscription_id);
                        return Ok(());
                    }
                };

                let remaining = if let Some((_, conv_state)) = self.conversations.iter_mut().find(|(_, conv_state)| conv_state.subscription_id == portal_subscription_id) {
                    let remaining = conv_state.decrement_eose();
                    if remaining == Some(0) {
                        conv_state.clear_eose();
                    }
                    remaining
                } else {
                    None
                };

                log::trace!("{:?} EOSE left for {}", remaining, portal_subscription_id);

                if remaining == Some(0) {
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

        let subscription_id = match PortalSubscriptionId::from_str(subscription_id.as_str()) {
            Ok(id) => id,
            Err(e) => {
                log::warn!("Invalid subscription ID format: {:?}: {}", e, subscription_id);
                return Ok(());
            }
        };

        self.dispatch_event(channel, subscription_id.clone(), message.clone())
            .await?;

        let mut to_cleanup = vec![];
        let mut other_conversations = vec![];

        // Check if there are other potential conversations to dispatch to
        for (id, conv_state) in self.conversations.iter() {
            if conv_state.subscription_id == subscription_id {
                continue;
            }

            if conv_state.conversation.is_expired() {
                to_cleanup.push(id.clone());
                continue;
            }

            if let LocalEvent::Message(event) = &event {
                if let Some(filter) = &conv_state.filter {
                    if filter.match_event(&event, MatchEventOptions::default()) {
                        other_conversations.push(conv_state.subscription_id.clone());
                    }
                }
            }
        }

        for id in to_cleanup {
            self.cleanup_conversation(channel, &id).await?;
        }

        for id in other_conversations {
            self.dispatch_event(channel, id, message.clone())
                .await?;
        }
        Ok(())
    }

    async fn dispatch_event<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        subscription_id: PortalSubscriptionId,
        message: ConversationMessage,
    ) -> Result<(), ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        log::debug!("Dispatching event to subscription: {}", subscription_id);


        let (response, conversation_id) = match self.conversations.iter_mut().find(|(_, conv_state)| conv_state.subscription_id == subscription_id) {
            Some((conversation_id, conv_state)) => {
                log::debug!("Found conversation, processing message");
                
                let response = match conv_state.conversation.on_message(message) {
                    Ok(response) => response,
                    Err(e) => {
                        log::warn!("Error in conversation id {}: {:?}", conversation_id, e);
                        Response::new().finish()
                    }
                };

                (response, conversation_id.clone())
            }
            None => {
                log::warn!("No conversation found for subscription id: {}", subscription_id);
                channel
                    .unsubscribe(subscription_id.clone())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                return Ok(());
            }
        };

        log::debug!("Processing response for conversation: {}", conversation_id);
        self.process_response(channel, &conversation_id, response, subscription_id)
            .await?;

        Ok(())
    }

    async fn process_response<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        id: &PortalConversationId,
        response: Response,
        subscription_id: PortalSubscriptionId,
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

            let num_relays = if let Some(selected_relays) = selected_relays_optional.clone() {
                let num_relays = selected_relays.len();
                log::trace!("Subscribing to relays = {:?}", selected_relays);
                channel
                    .subscribe_to(selected_relays, subscription_id.clone(), response.filter.clone())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                num_relays
            } else {
                log::trace!("Subscribing to all relays");
                channel
                    .subscribe(subscription_id.clone(), response.filter.clone())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?
            };

            if let Some(conv_state) = self.conversations.get_mut(id) {
                conv_state.filter = Some(response.filter.clone());
                conv_state.set_eose_counter(num_relays);
            }
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

        // Send all notifications
        if let Some(conv_state) = self.conversations.get_mut(id) {
            for notification in response.notifications.iter() {
                log::debug!("Sending notification: {:?}", notification);
                let sent_count = conv_state.send_notification(notification).await;
                log::trace!(
                    "Notification sent to {} subscribers for conversation {}",
                    sent_count,
                    id
                );
            }
        }

        // Handle subkey proof subscription if needed
        if response.subscribe_to_subkey_proofs {
            let alias_num = rand::random::<u64>();

            let alias = PortalConversationId::new_conversation_alias(id.id(), alias_num);

            if let Some(conv_state) = self.conversations.get_mut(id) {
                conv_state.add_alias(alias.clone());
            }

            let filter = Filter::new()
                .kinds(vec![Kind::Custom(SUBKEY_PROOF)])
                .events(events_to_broadcast.iter().map(|e| e.id));

            let subscription_id = PortalSubscriptionId::generate();
            // Create a new ConversationState for the alias
            self.conversations.insert(
                alias.clone(),
                ConversationState::new_alias(alias.clone(), filter.clone(), subscription_id.clone()),
            );

            if let Some(selected_relays) = selected_relays_optional.clone() {
                channel
                    .subscribe_to(selected_relays, subscription_id.clone(), filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            } else {
                // Subscribe to subkey proofs to all
                channel
                    .subscribe(subscription_id.clone(), filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }
        }

        // check if Response has selected relays
        if let Some(selected_relays) = selected_relays_optional {
            for event in events_to_broadcast {
                self.queue_event(channel, event, Some(selected_relays.clone()))
                    .await?;
            }
        } else {
            for event in events_to_broadcast {
                self.queue_event(channel, event, None).await?;
            }

            // TODO: wait for confirmation from relays
        }

        if response.finished {
            log::info!("Conversation {} finished, cleaning up", id);
            self.cleanup_conversation(channel, id).await?;
        }

        Ok(())
    }

    async fn queue_event<C: Channel>(
        &self,
        channel: &Arc<C>,
        event: Event,
        relays: Option<HashSet<String>>,
    ) -> Result<(), ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        if let Some(relays) = relays {
            // if selected relays, broadcast to selected relays
            channel
                .broadcast_to(relays, event)
                .await
                .map_err(|e| ConversationError::Inner(Box::new(e)))?;
        } else {
            // if not selected relays, broadcast to all relays
            channel
                .broadcast(event)
                .await
                .map_err(|e| ConversationError::Inner(Box::new(e)))?;
        }
        Ok(())
    }

    fn internal_add_with_id(
        &mut self,
        id: &PortalConversationId,
        subscription_id: PortalSubscriptionId,
        mut conversation: ConversationBox,
        relays: Option<Vec<String>>,
        subscriber: Option<mpsc::Sender<serde_json::Value>>,
    ) -> Result<Response, ConversationError> {
        let response = conversation.init()?;

        let conv_state = if let Some(relays) = relays {
            // Create conversation with specific relays
            let relay_set: HashSet<String> = relays.into_iter().collect();
            let mut conv_state =
                ConversationState::new_with_relays(id.clone(), conversation, relay_set, subscription_id.clone());
            if let Some(subscriber) = subscriber {
                conv_state.add_subscriber(subscriber);
            }
            conv_state
        } else {
            // Create global conversation (subscribed to all relays)
            let mut conv_state = ConversationState::new(id.clone(), conversation, subscription_id.clone());
            if let Some(subscriber) = subscriber {
                conv_state.add_subscriber(subscriber);
            }
            conv_state
        };

        self.conversations.insert(id.clone(), conv_state);

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
    ) -> Result<PortalConversationId, ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        let conversation_id = PortalConversationId::new_conversation();

        let subscription_id = PortalSubscriptionId::generate();
        let response = self.internal_add_with_id(&conversation_id, subscription_id.clone(), conversation, None, None)?;
        self.process_response(channel, &conversation_id, response, subscription_id)
            .await?;

        Ok(conversation_id)
    }

    pub async fn add_conversation_with_relays<C: Channel>(
        &mut self,
        channel: &Arc<C>,
        conversation: ConversationBox,
        relays: Vec<String>,
    ) -> Result<PortalConversationId, ConversationError>
    where
        C::Error: From<nostr::types::url::Error>,
    {
        let conversation_id = PortalConversationId::new_conversation();

        let subscription_id = PortalSubscriptionId::generate();
        let response =
            self.internal_add_with_id(&conversation_id, subscription_id.clone(), conversation, Some(relays), None)?;
        self.process_response(channel, &conversation_id, response, subscription_id)
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
        id: PortalConversationId,
    ) -> Result<NotificationStream<T>, ConversationError> {
        let (tx, rx) = mpsc::channel(8);

        if let Some(conv_state) = self.conversations.get_mut(&id) {
            conv_state.add_subscriber(tx);
        }

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
        let conversation_id = PortalConversationId::new_conversation();

        // Subscribe before adding the conversation to ensure we don't miss notifications
        let (tx, rx) = mpsc::channel(8);

        let rx = tokio_stream::wrappers::ReceiverStream::new(rx);
        let rx = rx.map(|content| serde_json::from_value(content));
        let rx = NotificationStream::new(rx);

        // Now add the conversation
        let subscription_id = PortalSubscriptionId::generate();
        let response = self.internal_add_with_id(&conversation_id, subscription_id.clone(), conversation, None, Some(tx))?;
        self.process_response(channel, &conversation_id, response, subscription_id)
            .await?;

        Ok(rx)
    }
}

/// Encapsulates all state related to a single conversation.
/// This consolidates the multiple HashMaps that were previously keyed by PortalConversationId.
#[derive(Debug)]
struct ConversationState {
    id: PortalConversationId,
    /// The actual conversation object
    conversation: InnerConversationState,
    /// Aliases for subkey proof subscriptions
    aliases: Vec<PortalConversationId>,
    /// Nostr filter for this conversation
    filter: Option<Filter>,
    /// Notification subscribers for this conversation
    subscribers: Vec<mpsc::Sender<serde_json::Value>>,
    /// Number of EOSE events remaining for this conversation
    end_of_stored_events: Option<usize>,
    /// Which specific relays this conversation is subscribed to
    relay_urls: HashSet<String>,
    /// Whether this conversation is subscribed to all relays (global)
    is_global: bool,
    /// The subscription ID for this conversation
    subscription_id: PortalSubscriptionId,
}

#[derive(Debug)]
enum InnerConversationState {
    Standard(ConversationBox),
    Alias,
}

impl InnerConversationState {
    fn on_message(&mut self, message: ConversationMessage) -> Result<Response, ConversationError> {
        match self {
            InnerConversationState::Standard(conversation) => conversation.on_message(message),
            InnerConversationState::Alias => Ok(Response::default()),
        }
    }

    fn is_expired(&self) -> bool {
        match self {
            InnerConversationState::Standard(conversation) => conversation.is_expired(),
            InnerConversationState::Alias => false,
        }
    }
}

impl ConversationState {
    fn new(id: PortalConversationId, conversation: ConversationBox, subscription_id: PortalSubscriptionId) -> Self {
        Self {
            id,
            conversation: InnerConversationState::Standard(conversation),
            aliases: Vec::new(),
            filter: None,
            subscribers: Vec::new(),
            end_of_stored_events: None,
            relay_urls: HashSet::new(),
            is_global: true, // Default to global subscription
            subscription_id,
        }
    }

    fn new_with_relays(
        id: PortalConversationId,
        conversation: ConversationBox,
        relay_urls: HashSet<String>,
        subscription_id: PortalSubscriptionId,
    ) -> Self {
        Self {
            id,
            conversation: InnerConversationState::Standard(conversation),
            aliases: Vec::new(),
            filter: None,
            subscribers: Vec::new(),
            end_of_stored_events: None,
            relay_urls,
            is_global: false,
            subscription_id,
        }
    }

    fn new_alias(id: PortalConversationId, filter: Filter, subscription_id: PortalSubscriptionId) -> Self {
        Self {
            id,
            conversation: InnerConversationState::Alias,
            aliases: Vec::new(),
            filter: Some(filter),
            subscribers: Vec::new(),
            end_of_stored_events: None,
            relay_urls: HashSet::new(),
            is_global: true, // Aliases default to global
            subscription_id,
        }
    }

    /// Add a subscriber to this conversation
    fn add_subscriber(&mut self, subscriber: mpsc::Sender<serde_json::Value>) {
        self.subscribers.push(subscriber);
    }

    /// Set the EOSE counter for this conversation
    fn set_eose_counter(&mut self, count: usize) {
        self.end_of_stored_events = Some(count);
    }

    /// Decrement the EOSE counter and return the remaining count
    fn decrement_eose(&mut self) -> Option<usize> {
        if let Some(ref mut count) = self.end_of_stored_events {
            *count = count.saturating_sub(1);
            Some(*count)
        } else {
            None
        }
    }

    /// Increment the EOSE counter
    fn increment_eose(&mut self) {
        if let Some(ref mut count) = self.end_of_stored_events {
            *count = count.saturating_add(1);
        }
    }

    /// Clear the EOSE counter
    fn clear_eose(&mut self) {
        self.end_of_stored_events = None;
    }

    /// Add an alias for subkey proof subscriptions
    fn add_alias(&mut self, alias: PortalConversationId) {
        // Prevent duplicate aliases
        if !self.aliases.contains(&alias) {
            self.aliases.push(alias);
        }
    }

    /// Get a reference to the aliases
    fn aliases(&self) -> &[PortalConversationId] {
        &self.aliases
    }

    /// Get the relay URLs for this conversation
    fn relay_urls(&self) -> &HashSet<String> {
        &self.relay_urls
    }

    /// Check if this conversation is subscribed globally
    fn is_global(&self) -> bool {
        self.is_global
    }

    /// Remove a relay URL from this conversation
    fn remove_relay(&mut self, relay_url: &str) {
        self.relay_urls.remove(relay_url);
    }

    /// Send notification to all subscribers and clean up dead ones
    async fn send_notification(&mut self, notification: &serde_json::Value) -> usize {
        let mut sent_count = 0;
        // Collect alive subscribers into a new vector
        let mut alive_subscribers = Vec::new();
        for sender in self.subscribers.drain(..) {
            match sender.send(notification.clone()).await {
                Ok(_) => {
                    sent_count += 1;
                    alive_subscribers.push(sender);
                }
                Err(mpsc::error::SendError(_)) => {
                    // Channel is closed, remove dead subscriber
                    // Do not push to alive_subscribers
                    log::warn!(
                        "Removing subscriber from conversation {} because channel is closed",
                        self.id
                    );
                }
            }
        }
        self.subscribers = alive_subscribers;
        sent_count
    }
}

type ConversationBox = Box<dyn Conversation + Send + Sync>;

impl std::fmt::Debug for ConversationBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conversation").finish()
    }
}
