use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

use adapters::ConversationWithNotification;
use channel::Channel;
use futures::{Stream, StreamExt};
use nostr::{
    event::{Event, EventBuilder, EventId, Kind, Tags},
    filter::{self, Filter},
    key::PublicKey,
    message::{RelayMessage, SubscriptionId},
    nips::nip44,
};
use nostr_relay_pool::RelayPoolNotification;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, mpsc};

use crate::{
    protocol::{model::event_kinds::SUBKEY_PROOF, LocalKeypair},
    router::conversation::{
        ConversationFilter, ConversationFilterId, ConversationId, ConversationRelaysContext, ConversationState, MergeConversationPolicy
    },
    utils::random_string,
};

pub mod adapters;
pub mod channel;
pub mod conversation;

pub use adapters::multi_key_listener::{MultiKeyListener, MultiKeyListenerAdapter};
pub use adapters::multi_key_sender::{MultiKeySender, MultiKeySenderAdapter};
pub use conversation::{Conversation, ConversationError, ConversationMessage, DynConversation};

pub struct RelayNode {
    conversations: HashSet<ConversationId>,
}

impl RelayNode {
    fn new() -> Self {
        RelayNode {
            conversations: HashSet::new(),
        }
    }
}

pub struct FilterNode {
    filter: ConversationFilter,
    conversations: HashSet<ConversationId>,
}

impl FilterNode {
    fn new(filter: ConversationFilter) -> Self {
        FilterNode {
            filter,
            conversations: HashSet::new(),
        }
    }
}

// TODO: update expiry at every message

/// A router that manages conversations over a Nostr channel.
///
/// The `MessageRouter` is responsible for:
/// - Managing conversations and their lifecycle
/// - Routing incoming messages to the appropriate conversations
/// - Broadcasting outgoing messages to the network
/// - Managing subscriptions to conversation notifications
pub struct MessageRouter<C: Channel> {
    channel: C,
    keypair: LocalKeypair,

    conversations: Mutex<HashMap<ConversationId, ConversationState>>,
    subscribers: Mutex<HashMap<ConversationId, Vec<mpsc::Sender<serde_json::Value>>>>,

    relay_nodes: RwLock<HashMap<String, RelayNode>>,
    global_relay_node: RwLock<RelayNode>,

    filters: RwLock<HashMap<ConversationFilterId, FilterNode>>,
}

impl<C: Channel> MessageRouter<C>
where
    <C as Channel>::Error: From<nostr::types::url::Error>,
{
    /// Creates a new `MessageRouter` with the given channel and keypair.
    ///
    /// The router will use the provided channel for all network communication and the keypair
    /// for message encryption/decryption.
    ///
    /// # Arguments
    /// * `channel` - The channel to use for network communication
    /// * `keypair` - The keypair to use for encryption/decryption
    pub fn new(channel: C, keypair: LocalKeypair) -> Self {
        Self {
            channel,
            keypair,
            conversations: Mutex::new(HashMap::new()),
            subscribers: Mutex::new(HashMap::new()),
            relay_nodes: RwLock::new(HashMap::new()),
            global_relay_node: RwLock::new(RelayNode::new()),
            filters: RwLock::new(HashMap::new()),
        }
    }

    /// Get filter from ConversationState
    fn get_filter<'g>(
        filters_guard: &'g RwLockReadGuard<'g, HashMap<ConversationFilterId, FilterNode>>,
        conv: &ConversationState
    ) -> Result<&'g ConversationFilter, ConversationError> {
        let filter_id = conv.filter.as_ref().unwrap();

        match filters_guard.get(filter_id) {
            Some(filter_node) => Ok(&filter_node.filter),
            None => Err(ConversationError::FilterNotFound(filter_id.clone())),
        }
    }

    pub async fn add_relay(&self, url: String) -> Result<(), ConversationError> {
        self.channel()
            .add_relay(url.clone())
            .await
            .map_err(|e| ConversationError::Inner(Box::new(e)))?;

        // Subscribe existing conversations to new relays
        {
            let global_relay_node = self.global_relay_node.read().await;
            let filters = self.filters.read().await;
            let conversation_ids: Vec<_> =
                global_relay_node.conversations.iter().cloned().collect();
            for conversation_id in conversation_ids.iter() {
                // Lock conversations only for this iteration
                let mut conversations = self.conversations.lock().await;

                let conversation_state = conversations
                    .get_mut(conversation_id)
                    .ok_or(ConversationError::ConversationNotFound)?;

                let filter = Self::get_filter(&filters, conversation_state)?;

                log::trace!(
                    "Subscribing {:?} to new relay = {:?}",
                    conversation_id,
                    &url
                );
                self.channel
                    .subscribe_to(vec![url.clone()], conversation_id, &filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                conversation_state.increment_eose();

                if let Some(aliases) = &conversation_state.aliases {
                    for alias in aliases {
                        let alias = ConversationId::from_alias(conversation_id.as_str(), *alias);

                        // If you need to handle aliases, repeat the lock pattern here as needed.

                        let mut conversations = self.conversations.lock().await;
                        let alias_conv = match conversations.get_mut(&alias) {
                            Some(state) => state,
                            None => continue,
                        };
                        let filter = Self::get_filter(&filters, alias_conv)?;
                        self.channel
                            .subscribe_to(vec![url.clone()], &alias, &filter)
                            .await
                            .map_err(|e| ConversationError::Inner(Box::new(e)))?;
                    }
                }
            }
        }

        {
            let mut relay_nodes = self.relay_nodes.write().await;

            relay_nodes
                .entry(url.clone())
                .or_insert_with(|| RelayNode::new());
        }

        Ok(())
    }

    pub async fn remove_relay(&self, url: String) -> Result<(), ConversationError> {
        self.channel()
            .remove_relay(url.clone())
            .await
            .map_err(|e| ConversationError::Inner(Box::new(e)))?;

        let global_relay_guard = self.global_relay_node.read().await;
        let mut relay_nodes_guard = self.relay_nodes.write().await;

        if let Some(node) = relay_nodes_guard.remove(&url) {
            let relay_nodes_guard = relay_nodes_guard.downgrade();
            let mut conversations = self.conversations.lock().await;

            for conv in node.conversations.iter() {
                let relays_of_conversation = self
                    .get_relays_by_conversation(conv, &global_relay_guard, &relay_nodes_guard)
                    .await?;
                match relays_of_conversation {
                    ConversationRelaysContext::Targeted(urls) => {
                        // If conversation it not present in other relays, clean it
                        if urls.is_empty() {
                            self.cleanup_conversation(conv).await?;
                        }
                    }
                    ConversationRelaysContext::Global => {
                        // If conversation is present also on the global node relay, should'nt happen, dont clean it
                    }
                }

                conversations
                    .get_mut(conv)
                    .map(|state| state.increment_eose());
            }
        }

        Ok(())
    }


    async fn get_relays_by_conversation<'g>(
        &self,
        conversation_id: &ConversationId,
        global_relay_guard: &RwLockReadGuard<'g, RelayNode>,
        relay_nodes_guard: &RwLockReadGuard<'g, HashMap<String, RelayNode>>,
    ) -> Result<ConversationRelaysContext, ConversationError> {
        if global_relay_guard.conversations.contains(conversation_id) {
            return Ok(ConversationRelaysContext::Global);
        }

        let mut relays = HashSet::new();
        for (url, node) in relay_nodes_guard.iter() {
            if node.conversations.contains(conversation_id) {
                relays.insert(url.clone());
            }
        }
        Ok(ConversationRelaysContext::Targeted(relays))
    }

    pub async fn cleanup_conversation(
        &self,
        conversation: &ConversationId,
    ) -> Result<(), ConversationError> {
        // Remove conversation state
        let conversation_state = self
            .conversations
            .lock()
            .await
            .remove(conversation)
            .ok_or(ConversationError::ConversationNotFound)?;

        match &conversation_state.filter {
            Some(filter_id) => {
                let mut filters_guard = self.filters.write().await;
                let filter_node = filters_guard
                    .get_mut(filter_id)
                    .ok_or(ConversationError::FilterNotFound(filter_id.clone()))?;
                filter_node.conversations.remove(conversation);

            },
            None => {},
        }

        // Remove from global relay node
        {
            let mut global_relay_node = self.global_relay_node.write().await;
            global_relay_node.conversations.remove(conversation);
        }

        // Remove from specific relay node
        {
            let mut relay_nodes = self.relay_nodes.write().await;
            for (_, relay_node) in relay_nodes.iter_mut() {
                relay_node.conversations.remove(conversation);
            }
        }

        // Remove filters from relays
        self.channel
            .unsubscribe(conversation.into())
            .await
            .map_err(|e| ConversationError::Inner(Box::new(e)))?;

        if let Some(aliases) = conversation_state.aliases {
            for alias in aliases {
                self.channel
                    .unsubscribe(&ConversationId::from_alias(conversation.as_str(), alias).into())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }
        }
        Ok(())
    }

    pub async fn purge(&mut self) {
        self.conversations.lock().await.clear();
        self.global_relay_node.write().await.conversations.clear();
        self.filters.write().await.clear();
    }

    async fn merge_conversation(
        &self,
        conv_id: &ConversationId,
        conv_filter: &ConversationFilter,
    ) -> Result<bool, ConversationError> {
        let mut merged = false;

        if let Some(merge_policy) = &conv_filter.merge_policy {
            match merge_policy {
                MergeConversationPolicy::SameKind => {
                    // get kinds of filter
                    let conv_filter_kinds = conv_filter.inner.kinds.clone().unwrap();

                    // search for subscription with the same kind
                    let mut filters = self.filters.write().await;

                    // for each filter node
                    for (filter_id, filter_node) in filters.iter_mut() {

                        // check if filter kinds are present in the new conversation filter
                        if let Some(filter_kinds) = &filter_node.filter.kinds {

                            // make sure the current conversation is not already in the filter
                            if !filter_node.conversations.contains(conv_id) {

                                // check if there is at least one kind in common
                                if filter_kinds
                                    .iter()
                                    .any(|k| conv_filter_kinds.contains(k))
                                {
                                    log::trace!(
                                        "Merging conversation {} on filter {}",
                                        conv_id,
                                        filter_id
                                    );

                                    // Add the conversation to the filter node
                                    filter_node.conversations.insert(conv_id.clone());

                                    merged = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            };
        }
        Ok(merged)
    }

    /// Starts listening for incoming messages and routes them to the appropriate conversations.
    ///
    /// This method should be spawned in a separate task as it runs indefinitely.
    ///
    /// # Returns
    /// * `Ok(())` if the listener exits normally
    /// * `Err(ConversationError)` if an error occurs while processing messages
    pub async fn listen(&self) -> Result<(), ConversationError> {
        enum LocalEvent {
            Message(Event),
            EndOfStoredEvents,
        }

        while let Ok(notification) = self.channel.receive().await {
            log::trace!("Notification = {:?}", notification);

            let (subscription_id, event): (SubscriptionId, LocalEvent) = match notification {
                RelayPoolNotification::Message {
                    message:
                        RelayMessage::Event {
                            subscription_id,
                            event,
                        },
                    ..
                } => (
                    subscription_id.into_owned(),
                    LocalEvent::Message(event.into_owned()),
                ),
                RelayPoolNotification::Event {
                    event,
                    subscription_id,
                    ..
                } => (subscription_id, LocalEvent::Message(*event)),
                RelayPoolNotification::Message {
                    message: RelayMessage::EndOfStoredEvents(subscription_id),
                    ..
                } => {
                    let mut conversations = self.conversations.lock().await;

                    let conversation_state = conversations
                        .get_mut(&ConversationId::from(subscription_id.as_str()))
                        .ok_or(ConversationError::ConversationNotFound)?;


                    let remaining = conversation_state.decrease_eose();

                    log::trace!("{:?} EOSE left for {:?}", remaining, subscription_id);

                    if remaining == Some(0) {
                        conversation_state.reset_eose();
                        (subscription_id.into_owned(), LocalEvent::EndOfStoredEvents)
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };
            let message = match &event {
                LocalEvent::Message(event) => {
                    if event.pubkey == self.keypair.public_key() && event.kind != Kind::Metadata {
                        log::trace!("Ignoring event from self");
                        continue;
                    }

                    if !event.verify_signature() {
                        log::warn!("Invalid signature for event id: {:?}", event.id);
                        continue;
                    }

                    log::trace!("Decrypting with key = {:?}", self.keypair.public_key());

                    if let Ok(content) =
                        nip44::decrypt(&self.keypair.secret_key(), &event.pubkey, &event.content)
                    {
                        let cleartext = match CleartextEvent::new(&event, &content) {
                            Ok(cleartext) => cleartext,
                            Err(e) => {
                                log::warn!("Invalid JSON in event: {:?}", e);
                                continue;
                            }
                        };

                        log::trace!("Decrypted event: {:?}", cleartext);

                        ConversationMessage::Cleartext(cleartext)
                    } else if let Ok(cleartext) =
                        serde_json::from_str::<serde_json::Value>(&event.content)
                    {
                        log::trace!("Unencrypted event: {:?}", cleartext);
                        ConversationMessage::Cleartext(CleartextEvent::new_json(&event, cleartext))
                    } else {
                        log::warn!("Failed to decrypt event: {:?}", event);
                        ConversationMessage::Encrypted(event.clone())
                    }
                }
                LocalEvent::EndOfStoredEvents => ConversationMessage::EndOfStoredEvents,
            };

            self.dispatch_event(subscription_id.clone(), message.clone())
                .await?;

            let mut to_cleanup = vec![];
            let mut other_conversations = vec![];

            // Check if there are other potential conversations to dispatch to
            for (id, filter_node) in self.filters.read().await.iter() {
                for ele in filter_node.conversations.iter() {
                    if ele.as_str() == subscription_id.as_str() {
                        continue;
                    }

                    match self.conversations.lock().await.get(ele) {
                        Some(conv_state) if conv_state.conversation.is_expired() => {
                            to_cleanup.push(ele.clone());
                            continue;
                        }
                        _ => {}
                    }

                    if let LocalEvent::Message(event) = &event {
                        if filter_node.filter.match_event(&event) {
                            other_conversations.push(ele.clone());
                        }
                    }
                }
            }

            for id in to_cleanup {
                self.cleanup_conversation(&id).await?;
            }

            for id in other_conversations {
                self.dispatch_event(SubscriptionId::new(id.as_str()), message.clone())
                    .await?;
            }
        }

        Ok(())
    }

    async fn dispatch_event(
        &self,
        subscription_id: SubscriptionId,
        message: ConversationMessage,
    ) -> Result<(), ConversationError> {
        let conversation_id = subscription_id.as_str();
        let conversation_id = if let Some((id, _)) = conversation_id.split_once("_") {
            id
        } else {
            conversation_id
        };

        let conversation_id = ConversationId::from(conversation_id);

        let response = match self.conversations.lock().await.get_mut(&conversation_id) {
            Some(conv_state) => match conv_state.conversation.on_message(message) {
                Ok(response) => response,
                Err(e) => {
                    log::warn!("Error in conversation id {}: {}", conversation_id, e);
                    Response::new().finish()
                }
            },
            None => {
                log::warn!("No conversation found for id: {:?}", conversation_id);
                self.channel
                    .unsubscribe(&conversation_id)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                return Ok(());
            }
        };

        self.process_response(&conversation_id, response).await?;

        Ok(())
    }

    async fn process_response(
        &self,
        id: &ConversationId,
        response: Response,
    ) -> Result<(), ConversationError> {
        log::trace!("Processing response builder for {} = {:?}", id, response);
  
        // Get relays of conversation
        let conversation_relays_ctx = {
            let global_relay_guard = self.global_relay_node.read().await;
            let relay_nodes_guard = self.relay_nodes.read().await;

            self.get_relays_by_conversation(id, &global_relay_guard, &relay_nodes_guard)
                .await?
        };
        // 

        let filter = response.filter;

        // If filter is not empty we could subscribe
        if !filter.is_empty() {

            // Try to merge the conversation in other subscriptions if 'merge_policy' is present
            let merged = self
                    .merge_conversation(id, &filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

            if !merged {
                let mut filter_node = FilterNode::new(filter.clone());
                filter_node.conversations.insert(id.clone());

                self.filters
                    .write()
                    .await
                    .insert(random_string(32), filter_node);

                let num_relays = if let ConversationRelaysContext::Targeted(selected_relays) = conversation_relays_ctx.clone() {
                    let num_relays = selected_relays.len();

                    log::trace!("Subscribing to relays = {:?}", selected_relays);
                    self.channel
                        .subscribe_to(selected_relays, id, &filter)
                        .await
                        .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                    num_relays
                } else {
                    log::trace!("Subscribing to all relays");
                    self.channel
                        .subscribe(id, &filter)
                        .await
                        .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                    self.channel
                        .num_relays()
                        .await
                        .map_err(|e| ConversationError::Inner(Box::new(e)))?
                };

                self.conversations
                    .lock()
                    .await
                    .get_mut(id)
                    .map(|state| state.set_eose(num_relays));
            }
        }

        let mut events_to_broadcast = vec![];
        for response_entry in response.responses.iter() {
            log::trace!(
                "Sending event of kind {:?} to {:?}",
                response_entry.kind,
                response_entry.recepient_keys
            );

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
                log::trace!("Unencrypted event: {:?}", event);
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
                    log::trace!("Encrypted event: {:?}", event);
                    events_to_broadcast.push(event);
                }
            }
        }

        for notification in response.notifications.iter() {
            let mut lock = self.subscribers.lock().await;
            if let Some(senders) = lock.get_mut(id) {
                for sender in senders.iter_mut() {
                    let _ = sender.send(notification.clone()).await;
                }
            }
        }

        if response.subscribe_to_subkey_proofs {
            let alias_num = rand::random::<u64>();

            let mut conversations_lock = self.conversations.lock().await;
            let conversation_state = conversations_lock
                .get_mut(id)
                .ok_or(ConversationError::ConversationNotFound)?;


            conversation_state.add_alias(alias_num);

            let filter: ConversationFilter = Filter::new()
                .kinds(vec![Kind::Custom(SUBKEY_PROOF)])
                .events(events_to_broadcast.iter().map(|e| e.id))
                .into();

            let alias = ConversationId::from_alias(id.as_str(), alias_num);
            let mut relay_node = FilterNode::new(filter.clone());
            relay_node.conversations.insert(alias.clone());

            self.filters
                .write()
                .await
                .insert(random_string(32), relay_node);

            if let ConversationRelaysContext::Targeted(selected_relays) = conversation_relays_ctx.clone() {
                log::trace!(
                    "Subscribing 'subkey proof' to relays = {:?}",
                    selected_relays
                );
                self.channel
                    .subscribe_to(selected_relays, &alias, &filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            } else {
                log::trace!("Subscribing 'subkey proof' to all relays");
                // Subscribe to subkey proofs to all

                self.channel
                    .subscribe(&alias, &filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }
        }

        // check if Response has selected relays
        if let ConversationRelaysContext::Targeted(selected_relays) = conversation_relays_ctx {
            for event in events_to_broadcast {
                // if selected relays, broadcast to selected relays
                self.channel
                    .broadcast_to(selected_relays.clone(), event)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }
        } else {
            for event in events_to_broadcast {
                // if not selected relays, broadcast to all relays
                self.channel
                    .broadcast(event)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }

            // TODO: wait for confirmation from relays
        }

        if response.finished {
            self.cleanup_conversation(id).await?;
        }

        Ok(())
    }

    async fn internal_add_with_id(
        &self,
        id: &ConversationId,
        mut conversation: DynConversation,
        relays: Option<Vec<String>>,
    ) -> Result<Response, ConversationError> {
        let response = conversation.init()?;

        let owned_id = id.clone();
        if let Some(relays) = relays {
            // Update relays node
            let mut relay_nodes = self.relay_nodes.write().await;
            // for each relay parameter
            for relay in relays {
                // get relay node associated

                match relay_nodes.get_mut(&relay) {
                    Some(found_node) => {
                        found_node.conversations.insert(owned_id.clone());
                    }
                    None => {
                        return Err(ConversationError::RelayNotConnected(relay));
                    }
                }
            }
        } else {
            // Update Global Relay Node
            self.global_relay_node
                .write()
                .await
                .conversations
                .insert(owned_id.clone());
        }

        self.conversations
            .lock()
            .await
            .insert(owned_id, ConversationState::new(conversation));

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
    /// * `Ok(String)` - The ID of the added conversation
    /// * `Err(ConversationError)` if an error occurs during initialization
    pub async fn add_conversation(
        &self,
        conversation: DynConversation,
    ) -> Result<ConversationId, ConversationError> {
        let conversation_id = ConversationId::generate();

        let response = self
            .internal_add_with_id(&conversation_id, conversation, None)
            .await?;
        self.process_response(&conversation_id, response).await?;

        Ok(conversation_id)
    }

    pub async fn add_conversation_with_relays(
        &self,
        conversation: DynConversation,
        relays: Vec<String>,
    ) -> Result<ConversationId, ConversationError> {
        let conversation_id = ConversationId::generate();

        let response = self
            .internal_add_with_id(&conversation_id, conversation, Some(relays))
            .await?;
        self.process_response(&conversation_id, response).await?;

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
    pub async fn subscribe_to_service_request<T: DeserializeOwned + Serialize>(
        &self,
        id: ConversationId,
    ) -> Result< NotificationStream<T>, ConversationError> {
        let (tx, rx) = mpsc::channel(8);

        self.subscribers
            .lock()
            .await
            .entry(id)
            .or_insert(Vec::new())
            .push(tx);

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
    pub async fn add_and_subscribe<Conv: ConversationWithNotification + Send + 'static>(
        &self,
        conversation: Conv,
    ) -> Result<NotificationStream<Conv::Notification>, ConversationError> {
        let conversation_id = ConversationId::generate();

        // Update Global Relay Node

        {
            let mut global_relay_node = self.global_relay_node.write().await;
            global_relay_node
                .conversations
                .insert(conversation_id.clone());
        }
        


        let response: Response = self
            .internal_add_with_id(&conversation_id, Box::new(conversation), None)
            .await?;
        let delayed_reply = self
            .subscribe_to_service_request::<Conv::Notification>(conversation_id.clone())
            .await?;
        self.process_response(&conversation_id, response).await?;

        Ok(delayed_reply)
    }

    /// Gets a reference to the underlying channel.
    pub fn channel(&self) -> &C {
        &self.channel
    }

    /// Gets a reference to the router's keypair.
    pub fn keypair(&self) -> &LocalKeypair {
        &self.keypair
    }
}

#[derive(Debug)]
struct ResponseEntry {
    pub recepient_keys: Vec<PublicKey>,
    pub kind: Kind,
    pub tags: Tags,
    pub content: serde_json::Value,
    pub encrypted: bool,
}

/// A response from a conversation.
///
/// Responses can include:
/// - Filters for subscribing to specific message types
/// - Replies to send to specific recipients or broadcast to all participants in the conversation
/// - Notifications to send to subscribers
/// - A flag indicating if the conversation is finished. If set, the conversation will be removed from the router.
///
/// # Example
/// ```rust,no_run
/// use portal::router::Response;
/// use nostr::{Filter, Kind, Tags};
///
/// let response = Response::new()
///     .filter(Filter::new().kinds(vec![Kind::from(27000)]))
///     .reply_to(pubkey, Kind::from(27001), Tags::new(), content)
///     .notify(notification)
///     .finish();
/// ```
#[derive(Debug, Default)]
pub struct Response {
    filter: ConversationFilter,
    responses: Vec<ResponseEntry>,
    notifications: Vec<serde_json::Value>,
    finished: bool,
    subscribe_to_subkey_proofs: bool,
}

impl Response {
    /// Creates a new empty response.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the filter for this response.
    ///
    /// The filter will be used to subscribe to specific message types with the relays.
    ///
    /// # Arguments
    /// * `filter` - The filter to set
    #[deprecated(note = "Use `conversation_filter` method instead")]
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = filter.into();
        self
    }

    /// Sets the filter for this response.
    ///
    /// The filter will be used to subscribe to specific message types with the relays.
    ///
    /// # Arguments
    /// * `filter` - The filter to set
    pub fn conversation_filter(mut self, filter: ConversationFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Adds a reply to be sent to all recipients.
    ///
    /// # Arguments
    /// * `kind` - The kind of message to send
    /// * `tags` - The tags to include in the message
    /// * `content` - The content to send, must be serializable
    pub fn reply_all<S: serde::Serialize>(mut self, kind: Kind, tags: Tags, content: S) -> Self {
        let content = serde_json::to_value(&content).unwrap();
        self.responses.push(ResponseEntry {
            recepient_keys: vec![],
            kind,
            tags,
            content,
            encrypted: true,
        });
        self
    }

    /// Adds a reply to be sent to a specific recipient.
    ///
    /// # Arguments
    /// * `pubkey` - The public key of the recipient
    /// * `kind` - The kind of message to send
    /// * `tags` - The tags to include in the message
    /// * `content` - The content to send, must be serializable
    pub fn reply_to<S: serde::Serialize>(
        mut self,
        pubkey: PublicKey,
        kind: Kind,
        tags: Tags,
        content: S,
    ) -> Self {
        let content = serde_json::to_value(&content).unwrap();
        self.responses.push(ResponseEntry {
            recepient_keys: vec![pubkey],
            kind,
            tags,
            content,
            encrypted: true,
        });
        self
    }

    /// Adds a notification to be sent to subscribers.
    ///
    /// # Arguments
    /// * `data` - The notification data to send, must be serializable
    pub fn notify<S: serde::Serialize>(mut self, data: S) -> Self {
        let content = serde_json::to_value(&data).unwrap();
        self.notifications.push(content);
        self
    }

    /// Marks the conversation as finished.
    ///
    /// When a conversation is finished, it will be removed from the router.
    pub fn finish(mut self) -> Self {
        self.finished = true;
        self
    }

    /// Subscribe to events that tag our replies via the event_id
    pub fn subscribe_to_subkey_proofs(mut self) -> Self {
        self.subscribe_to_subkey_proofs = true;
        self
    }

    // Broadcast an unencrypted event
    pub fn broadcast_unencrypted<S: serde::Serialize>(
        mut self,
        kind: Kind,
        tags: Tags,
        content: S,
    ) -> Self {
        let content = serde_json::to_value(&content).unwrap();
        self.responses.push(ResponseEntry {
            recepient_keys: vec![],
            kind,
            tags,
            content,
            encrypted: false,
        });
        self
    }

    fn set_recepient_keys(&mut self, user: PublicKey, subkeys: &HashSet<PublicKey>) {
        for response in &mut self.responses {
            if response.recepient_keys.is_empty() {
                response.recepient_keys.push(user);
                response.recepient_keys.extend(subkeys.iter().cloned());
            }
        }
    }

    fn extend(&mut self, response: Response) {
        self.responses.extend(response.responses);
        self.subscribe_to_subkey_proofs |= response.subscribe_to_subkey_proofs;
    }
}

#[derive(Debug, Clone)]
pub struct CleartextEvent {
    pub id: EventId,
    pub pubkey: PublicKey,
    pub created_at: nostr::types::Timestamp,
    pub kind: Kind,
    pub tags: Tags,
    pub content: serde_json::Value,
}

impl CleartextEvent {
    pub fn new(event: &Event, decrypted: &str) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id: event.id,
            pubkey: event.pubkey,
            created_at: event.created_at,
            kind: event.kind,
            tags: event.tags.clone(),
            content: serde_json::from_str(decrypted)?,
        })
    }

    pub fn new_json(event: &Event, content: serde_json::Value) -> Self {
        Self {
            id: event.id,
            pubkey: event.pubkey,
            created_at: event.created_at,
            kind: event.kind,
            tags: event.tags.clone(),
            content,
        }
    }
}

/// Convenience wrapper around a stream of notifications.
///
/// It's automatically implemented for any stream that implements `Stream<Item = Result<T, serde_json::Error>> + Send + Unpin + 'static`.
pub trait InnerNotificationStream<T: Serialize>:
    Stream<Item = Result<T, serde_json::Error>> + Send + Unpin + 'static
{
}
impl<S, T: Serialize> InnerNotificationStream<T> for S where
    S: Stream<Item = Result<T, serde_json::Error>> + Send + Unpin + 'static
{
}

pub struct NotificationStream<T: Serialize> {
    stream: Box<dyn InnerNotificationStream<T>>,
}

impl<T: Serialize> NotificationStream<T> {
    pub(crate) fn new(stream: impl InnerNotificationStream<T>) -> Self {
        Self {
            stream: Box::new(stream),
        }
    }

    /// Returns the next notification from the stream.
    pub async fn next(&mut self) -> Option<Result<T, serde_json::Error>> {
        use futures::StreamExt;

        self.stream.next().await
    }
}

impl<T: Serialize> Deref for NotificationStream<T> {
    type Target = Box<dyn InnerNotificationStream<T>>;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl<T: Serialize> DerefMut for NotificationStream<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}
