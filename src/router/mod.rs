use std::{
    collections::{HashMap, HashSet},
};

use adapters::ConversationWithNotification;
use channel::Channel;
use futures::StreamExt;
use nostr::{
    event::{Event, EventBuilder, EventId, Kind, Tags},
    filter::Filter,
    key::PublicKey,
    message::{RelayMessage, SubscriptionId},
    nips::nip44,
};
use nostr_relay_pool::RelayPoolNotification;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::{Mutex, RwLock, RwLockReadGuard, mpsc};

use crate::{
    protocol::{model::event_kinds::SUBKEY_PROOF, LocalKeypair},
    router::{conversation::{
        ConversationFilterId, ConversationId, ConversationRelaysContext, ConversationState
    }, node::{FilterNode, RelayNode}, notification::NotificationStream, response::Response, filter_merge::{can_merge_filters, merge_filters}},
    utils::random_string,
};

pub mod adapters;
pub mod channel;
pub mod conversation;
pub mod notification;
pub mod response;
pub mod node;
pub mod filter_merge;

pub use adapters::multi_key_listener::{MultiKeyListener, MultiKeyListenerAdapter};
pub use adapters::multi_key_sender::{MultiKeySender, MultiKeySenderAdapter};
pub use conversation::{Conversation, ConversationError, ConversationMessage, DynConversation};

// TODO: update expiry at every message

/// A router that manages conversations over a Nostr channel.
///
/// The `MessageRouter` is responsible for:
/// - Managing conversations and their lifecycle
/// - Routing incoming messages to the appropriate conversations
/// - Broadcasting outgoing messages to the network
/// - Managing subscriptions to conversation notifications
///
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
    ) -> Result<&'g Filter, ConversationError> {
        let filter_id = conv.filter.as_ref().ok_or(ConversationError::FilterNotFound("No filter set".to_string()))?;

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

        // Collect unique filters to subscribe to and update EOSE counts atomically
        let filter_subscription_tasks = {
            let global_relay_node = self.global_relay_node.read().await;
            let filters = self.filters.read().await;
            let mut conversations = self.conversations.lock().await;
            
            // Collect unique filters used by conversations on global relays
            let mut unique_filters: HashMap<ConversationFilterId, Filter> = HashMap::new();
            
            // Process all conversation IDs first, then process aliases in a separate pass
            let mut conversation_ids_to_process = Vec::new();
            let mut alias_ids_to_process = Vec::new();
            
            for conversation_id in global_relay_node.conversations.iter() {
                conversation_ids_to_process.push(conversation_id.clone());
            }
            
            // First pass: process main conversations
            for conversation_id in &conversation_ids_to_process {
                if let Some(conversation_state) = conversations.get_mut(conversation_id) {
                    // Update EOSE count immediately while we have the lock
                    conversation_state.increment_eose();
                    
                    if let Some(filter_id) = &conversation_state.filter {
                        if let Some(filter_node) = filters.get(filter_id) {
                            unique_filters.insert(filter_id.clone(), filter_node.filter.clone());
                        }
                    }
                    
                    // Collect alias IDs for second pass
                    if let Some(aliases) = &conversation_state.aliases {
                        for alias in aliases {
                            let alias_id = ConversationId::from_alias(conversation_id.as_str(), *alias);
                            alias_ids_to_process.push(alias_id);
                        }
                    }
                }
            }
            
            // Second pass: process aliases
            for alias_id in alias_ids_to_process {
                if let Some(alias_state) = conversations.get_mut(&alias_id) {
                    // Update EOSE count for alias as well
                    alias_state.increment_eose();
                    
                    if let Some(alias_filter_id) = &alias_state.filter {
                        if let Some(alias_filter_node) = filters.get(alias_filter_id) {
                            unique_filters.insert(alias_filter_id.clone(), alias_filter_node.filter.clone());
                        }
                    }
                }
            }
            
            unique_filters
        };

        // Execute subscriptions without holding any locks - subscribe using filter IDs
        for (filter_id, filter) in filter_subscription_tasks {
            log::trace!("Subscribing filter {:?} to new relay = {:?}", filter_id, &url);
            
            self.channel
                .subscribe_to(vec![url.clone()], &filter_id, &filter)
                .await
                .map_err(|e| ConversationError::Inner(Box::new(e)))?;
        }

        // Add the new relay node
        {
            let mut relay_nodes = self.relay_nodes.write().await;
            relay_nodes.entry(url).or_insert_with(|| RelayNode::new());
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
                        // If conversation is not present in other relays, clean it
                        if urls.is_empty() {
                            // Need to drop the lock before calling cleanup_conversation to avoid deadlock
                            drop(conversations);
                            self.cleanup_conversation(conv).await?;
                            conversations = self.conversations.lock().await;
                        }
                    }
                    ConversationRelaysContext::Global => {
                        // If conversation is present on the global relay node, don't clean it
                        // This can happen legitimately when a conversation is on both global and specific relays
                    }
                }

                if let Some(state) = conversations.get_mut(conv) {
                    state.increment_eose();
                }
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
        // Remove conversation state and collect cleanup data
        let conversation_state = self
            .conversations
            .lock()
            .await
            .remove(conversation)
            .ok_or(ConversationError::ConversationNotFound)?;

        let mut filter_ids_to_unsubscribe = Vec::new();

        // Batch all cleanup operations that require locks
        {
            let mut filters_guard = self.filters.write().await;
            let mut global_relay_node = self.global_relay_node.write().await;
            let mut relay_nodes = self.relay_nodes.write().await;

            // Remove from filter node if it exists
            if let Some(filter_id) = &conversation_state.filter {
                if let Some(filter_node) = filters_guard.get_mut(filter_id) {
                    filter_node.conversations.remove(conversation);
                    
                    // If no conversations left using this filter, mark it for unsubscription and removal
                    if filter_node.conversations.is_empty() {
                        filter_ids_to_unsubscribe.push(filter_id.clone());
                        filters_guard.remove(filter_id);
                    }
                }
            }

            // Remove from global relay node
            global_relay_node.conversations.remove(conversation);

            // Remove from all specific relay nodes
            for (_, relay_node) in relay_nodes.iter_mut() {
                relay_node.conversations.remove(conversation);
            }
        }

        // Unsubscribe from filters that are no longer needed (done without locks to avoid holding them during network calls)
        for filter_id in filter_ids_to_unsubscribe {
            self.channel
                .unsubscribe(&filter_id)
                .await
                .map_err(|e| ConversationError::Inner(Box::new(e)))?;
        }

        // Handle aliases - unsubscribe their filters if they're no longer needed
        if let Some(aliases) = conversation_state.aliases {
            for alias in aliases {
                let alias_id = ConversationId::from_alias(conversation.as_str(), alias);
                
                // Remove alias conversation and check if its filter should be unsubscribed
                let alias_state = self.conversations.lock().await.remove(&alias_id);
                if let Some(alias_state) = alias_state {
                    if let Some(alias_filter_id) = alias_state.filter {
                        let should_unsubscribe = {
                            let mut filters_guard = self.filters.write().await;
                            if let Some(filter_node) = filters_guard.get_mut(&alias_filter_id) {
                                filter_node.conversations.remove(&alias_id);
                                
                                let is_empty = filter_node.conversations.is_empty();
                                if is_empty {
                                    filters_guard.remove(&alias_filter_id);
                                }
                                is_empty
                            } else {
                                false
                            }
                        };
                        
                        if should_unsubscribe {
                            self.channel
                                .unsubscribe(&alias_filter_id)
                                .await
                                .map_err(|e| ConversationError::Inner(Box::new(e)))?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    pub async fn purge(&mut self) {
        self.conversations.lock().await.clear();
        self.global_relay_node.write().await.conversations.clear();
        self.filters.write().await.clear();
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
                    // Handle EOSE for filter - affects all conversations using this filter
                    let should_emit_eose = {
                        let mut conversations = self.conversations.lock().await;
                        let filters = self.filters.read().await;
                        let filter_id = subscription_id.as_str();
                        
                        // Find all conversations using this filter
                        if let Some(filter_node) = filters.get(filter_id) {
                            let mut all_ready = true;
                            let mut any_found = false;
                            
                            for conv_id in filter_node.conversations.iter() {
                                if let Some(conversation_state) = conversations.get_mut(conv_id) {
                                    any_found = true;
                                    let remaining = conversation_state.decrease_eose();
                                    log::trace!("{:?} EOSE left for {:?} (filter {})", remaining, conv_id, filter_id);
                                    
                                    if remaining != Some(0) {
                                        all_ready = false;
                                    }
                                }
                            }
                            
                            if any_found && all_ready {
                                // Reset EOSE for all conversations using this filter
                                for conv_id in filter_node.conversations.iter() {
                                    if let Some(conversation_state) = conversations.get_mut(conv_id) {
                                        conversation_state.reset_eose();
                                    }
                                }
                                true
                            } else {
                                false
                            }
                        } else {
                            log::warn!("EOSE for unknown filter: {:?}", subscription_id);
                            false
                        }
                    };

                    if should_emit_eose {
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

            // Dispatch to all conversations using this filter
            let (conversations_to_dispatch, to_cleanup) = {
                let filters = self.filters.read().await;
                let conversations = self.conversations.lock().await;
                let filter_id = subscription_id.as_str();
                
                let mut conversations_to_dispatch = vec![];
                let mut to_cleanup = vec![];

                if let Some(filter_node) = filters.get(filter_id) {
                    for conv_id in filter_node.conversations.iter() {
                        match conversations.get(conv_id) {
                            Some(conv_state) if conv_state.conversation.is_expired() => {
                                to_cleanup.push(conv_id.clone());
                            }
                            Some(_) => {
                                conversations_to_dispatch.push(conv_id.clone());
                            }
                            None => continue,
                        }
                    }
                }
                
                (conversations_to_dispatch, to_cleanup)
            };

            // Process cleanup without holding locks
            for id in to_cleanup {
                self.cleanup_conversation(&id).await?;
            }

            // Dispatch to all conversations using this filter
            for conv_id in conversations_to_dispatch {
                self.dispatch_event(SubscriptionId::new(conv_id.as_str()), message.clone())
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

        // Check if conversation exists and get mutable reference with minimal lock time
        let response = {
            let mut conversations = self.conversations.lock().await;
            
            match conversations.get_mut(&conversation_id) {
                Some(conv_state) => {
                    // Call on_message while holding the lock (but this is unavoidable for mut access)
                    match conv_state.conversation.on_message(message) {
                        Ok(response) => Some(response),
                        Err(e) => {
                            log::warn!("Error in conversation id {}: {}", conversation_id, e);
                            Some(Response::new().finish())
                        }
                    }
                },
                None => {
                    log::warn!("No conversation found for id: {:?}", conversation_id);
                    None
                }
            }
        };

        match response {
            Some(response) => {
                self.process_response(&conversation_id, response).await?;
            }
            None => {
                // Conversation not found - with filter-based subscriptions, 
                // we don't unsubscribe individual conversations here
                log::warn!("No conversation found for id: {:?}", conversation_id);
            }
        }

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
        let subscription_info = if !filter.is_empty() {
            let mut filters = self.filters.write().await;
            let mut merged_with_existing = false;
            let mut filter_id_to_unsubscribe: Option<String> = None;
            
            // Try to merge with existing filters
            for (existing_filter_id, filter_node) in filters.iter_mut() {
                if can_merge_filters(&filter_node.filter, &filter) {
                    log::trace!("Merging conversation {} with existing filter {}", id, existing_filter_id);
                    
                    // Store the old filter for unsubscribing (currently unused but kept for future enhancements)
                    let _old_filter = filter_node.filter.clone();
                    
                    // Merge the filters
                    let merged_filter = merge_filters(&filter_node.filter, &filter);
                    
                    // Add conversation to existing filter node
                    filter_node.conversations.insert(id.clone());
                    filter_node.filter = merged_filter.clone();
                    
                    // Store the filter ID for unsubscribing from old filter
                    filter_id_to_unsubscribe = Some(existing_filter_id.clone());
                    
                    merged_with_existing = true;
                    break;
                }
            }
            
            let num_relays = if !merged_with_existing {
                // No existing filter to merge with, create new filter
                let mut filter_node = FilterNode::new(filter.clone());
                filter_node.conversations.insert(id.clone());
                let filter_id = random_string(32);

                // Add filter node to filters map and update conversation state
                filters.insert(filter_id.clone(), filter_node);
                
                // Update conversation state to reference the filter
                {
                    let mut conversations = self.conversations.lock().await;
                    if let Some(conversation_state) = conversations.get_mut(id) {
                        conversation_state.filter = Some(filter_id.clone());
                    }
                }

                // Determine relay count and perform subscription using filter ID
                if let ConversationRelaysContext::Targeted(selected_relays) = conversation_relays_ctx.clone() {
                    let num_relays = selected_relays.len();
                    log::trace!("Subscribing filter {} to relays = {:?}", filter_id, selected_relays);
                    
                    self.channel
                        .subscribe_to(selected_relays, &filter_id, &filter)
                        .await
                        .map_err(|e| ConversationError::Inner(Box::new(e)))?;
                    
                    num_relays
                } else {
                    log::trace!("Subscribing filter {} to all relays", filter_id);
                    
                    self.channel
                        .subscribe(&filter_id, &filter)
                        .await
                        .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                    self.channel
                        .num_relays()
                        .await
                        .map_err(|e| ConversationError::Inner(Box::new(e)))?
                }
            } else {
                // Merged with existing filter, need to unsubscribe from old and subscribe to new
                let num_relays = if let Some(filter_id) = filter_id_to_unsubscribe {
                    let filter_node = filters.get(&filter_id).ok_or(ConversationError::FilterNotFound(filter_id.clone()))?;
                    let merged_filter = &filter_node.filter;
                    
                    // Unsubscribe from old filter using filter ID
                    self.channel
                        .unsubscribe(&filter_id)
                        .await
                        .map_err(|e| ConversationError::Inner(Box::new(e)))?;
                    
                    // Subscribe to new merged filter using filter ID
                    let relay_count = if let ConversationRelaysContext::Targeted(selected_relays) = conversation_relays_ctx.clone() {
                        let num_relays = selected_relays.len();
                        log::trace!("Subscribing to merged filter {} on relays = {:?}", filter_id, selected_relays);
                        
                        self.channel
                            .subscribe_to(selected_relays.clone(), &filter_id, merged_filter)
                            .await
                            .map_err(|e| ConversationError::Inner(Box::new(e)))?;
                        
                        num_relays
                    } else {
                        log::trace!("Subscribing to merged filter {} on all relays", filter_id);
                        
                        self.channel
                            .subscribe(&filter_id, merged_filter)
                            .await
                            .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                        self.channel
                            .num_relays()
                            .await
                            .map_err(|e| ConversationError::Inner(Box::new(e)))?
                    };
                    
                    // Update conversation state to reference the filter
                    {
                        let mut conversations = self.conversations.lock().await;
                        if let Some(conversation_state) = conversations.get_mut(id) {
                            conversation_state.filter = Some(filter_id.clone());
                        }
                    }
                    
                    relay_count
                } else {
                    0
                };
                
                num_relays
            };

            Some(num_relays)
        } else {
            None
        };

        // Update EOSE count if subscription was created
        if let Some(num_relays) = subscription_info {
            self.conversations
                .lock()
                .await
                .get_mut(id)
                .map(|state| state.set_eose(num_relays));
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

        // Send notifications in batch
        if !response.notifications.is_empty() {
            let mut subscribers = self.subscribers.lock().await;
            if let Some(senders) = subscribers.get_mut(id) {
                for notification in response.notifications.iter() {
                    for sender in senders.iter_mut() {
                        let _ = sender.send(notification.clone()).await;
                    }
                }
            }
        }

        // Handle subkey proof subscription
        if response.subscribe_to_subkey_proofs {
            let alias_num = rand::random::<u64>();
            let alias = ConversationId::from_alias(id.as_str(), alias_num);
            
            // Create filter for subkey proofs
            let filter = Filter::new()
                .kinds(vec![Kind::Custom(SUBKEY_PROOF)])
                .events(events_to_broadcast.iter().map(|e| e.id));

            // Create filter ID and update conversation state and add filter in batch
            let filter_id = random_string(32);
            {
                let mut conversations = self.conversations.lock().await;
                let mut filters = self.filters.write().await;
                
                // Update conversation with alias
                let conversation_state = conversations
                    .get_mut(id)
                    .ok_or(ConversationError::ConversationNotFound)?;
                conversation_state.add_alias(alias_num);

                // Create alias conversation state with reference to the filter
                let mut alias_conversation_state = ConversationState::new(Box::new(DummyConversation));
                alias_conversation_state.filter = Some(filter_id.clone());
                conversations.insert(alias.clone(), alias_conversation_state);

                // Add filter node
                let mut filter_node = FilterNode::new(filter.clone());
                filter_node.conversations.insert(alias.clone());
                filters.insert(filter_id.clone(), filter_node);
            }

            // Subscribe to subkey proofs using filter ID
            if let ConversationRelaysContext::Targeted(selected_relays) = conversation_relays_ctx.clone() {
                log::trace!("Subscribing 'subkey proof' filter {} to relays = {:?}", filter_id, selected_relays);
                self.channel
                    .subscribe_to(selected_relays, &filter_id, &filter)
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            } else {
                log::trace!("Subscribing 'subkey proof' filter {} to all relays", filter_id);
                self.channel
                    .subscribe(&filter_id, &filter)
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

// Dummy conversation for alias conversations that don't need actual conversation logic
struct DummyConversation;

impl Conversation for DummyConversation {
    fn on_message(&mut self, _message: ConversationMessage) -> Result<Response, ConversationError> {
        Ok(Response::default())
    }

    fn is_expired(&self) -> bool {
        false
    }
}

