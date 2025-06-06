use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

use adapters::ConversationWithNotification;
use channel::Channel;
use futures::{Stream, StreamExt};
use nostr_relay_pool::RelayPoolNotification;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::{Mutex, RwLock, mpsc};

use nostr::{
    event::{Event, EventBuilder, EventId, Kind, Tags},
    filter::Filter,
    key::PublicKey,
    message::{RelayMessage, SubscriptionId},
    nips::nip44,
    types::TryIntoUrl,
};

use crate::{
    protocol::{LocalKeypair, model::event_kinds::SUBKEY_PROOF},
    utils::random_string,
};

pub mod adapters;
pub mod channel;

pub use adapters::multi_key_listener::{MultiKeyListener, MultiKeyListenerAdapter};
pub use adapters::multi_key_sender::{MultiKeySender, MultiKeySenderAdapter};

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
    conversations: Mutex<HashMap<String, Box<dyn Conversation + Send>>>,
    aliases: Mutex<HashMap<String, Vec<u64>>>,
    filters: RwLock<HashMap<String, Filter>>,
    subscribers: Mutex<HashMap<String, Vec<mpsc::Sender<serde_json::Value>>>>,

    conversation_relays: Mutex<HashMap<String, Vec<String>>>,
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
            aliases: Mutex::new(HashMap::new()),
            subscribers: Mutex::new(HashMap::new()),
            filters: RwLock::new(HashMap::new()),
            conversation_relays: Mutex::new(HashMap::new()),
        }
    }


    /// Get all conversations that are subscribed to a specific relay.
    pub async fn get_conversations_by_relay(
        &self,
        relay: String,
    ) -> Result<Vec<String>, ConversationError> {
        let conversations = self.conversation_relays.lock().await;
        let mut result = Vec::new();

        for (id, relays) in conversations.iter() {
            if relays.contains(&relay) {
                result.push(id.clone());
            }
        }

        Ok(result)
    }

    pub async fn cleanup_conversation(&self, conversation: &str) -> Result<(), ConversationError> {
        // Remove conversation state
        self.conversations.lock().await.remove(conversation);
        self.subscribers.lock().await.remove(conversation);
        self.filters.write().await.remove(conversation);
        let aliases = self.aliases.lock().await.remove(conversation);
        self.conversation_relays.lock().await.remove(conversation);

        // Remove filters from relays
        self.channel
            .unsubscribe(conversation.to_string())
            .await
            .map_err(|e| ConversationError::Inner(Box::new(e)))?;

        if let Some(aliases) = aliases {
            for alias in aliases {
                self.channel
                    .unsubscribe(format!("{}_{}", conversation, alias))
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
            }
        }
        Ok(())
    }

    pub async fn purge(&mut self) {
        self.conversations.lock().await.clear();
        self.subscribers.lock().await.clear();
        self.aliases.lock().await.clear();
        self.filters.write().await.clear();
        self.conversation_relays.lock().await.clear();
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
                    // TODO: we should only send this event when we know all the relays have replied with EndOfStoredEvents. There's another
                    // TODO in the `process_response` function to check for confirmation from relays after sending a message. Once we have that
                    // and we know which relays have received the message, we can then know if all relays have replied with EOSE here.

                    (subscription_id.into_owned(), LocalEvent::EndOfStoredEvents)
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
            for (id, filter) in self.filters.read().await.iter() {
                if id == subscription_id.as_str() {
                    continue;
                }

                match self.conversations.lock().await.get(id) {
                    Some(conv) if conv.is_expired() => {
                        to_cleanup.push(id.clone());
                        continue;
                    }
                    _ => {}
                }

                if let LocalEvent::Message(event) = &event {
                    if filter.match_event(&event) {
                        log::warn!("Dispatching event to {:?}", id);
                        other_conversations.push(id.clone());
                    }
                }
            }

            for id in to_cleanup {
                self.cleanup_conversation(&id).await?;
            }

            for id in other_conversations {
                log::warn!("Dispatching event to {:?}", id);
                self.dispatch_event(SubscriptionId::new(id.clone()), message.clone())
                    .await?;
                log::warn!("Finished dispatching event to {:?}", id);
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

        let response = match self.conversations.lock().await.get_mut(conversation_id) {
            Some(conv) => match conv.on_message(message) {
                Ok(response) => response,
                Err(e) => {
                    log::warn!("Error in conversation id {:?}: {:?}", conversation_id, e);
                    Response::new().finish()
                }
            },
            None => {
                log::warn!("No conversation found for id: {:?}", conversation_id);
                self.channel
                    .unsubscribe(conversation_id.to_string())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                return Ok(());
            }
        };

        self.process_response(conversation_id, response).await?;

        Ok(())
    }

    async fn process_response(
        &self,
        id: &str,
        response: Response,
    ) -> Result<(), ConversationError> {
        log::trace!("Processing response builder for {} = {:?}", id, response);




        if !response.filter.is_empty() {
            if let Some(selected_relays) = response.selected_relays.clone() {
                log::trace!("Subscribing to relays = {:?}", selected_relays);
                self.channel
                    .subscribe_to(selected_relays, id.to_string(), response.filter.clone())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
                self.filters
                    .write()
                    .await
                    .insert(id.to_string(), response.filter.clone());
            } else {
                log::trace!("Subscribing to all relays");
                self.channel
                    .subscribe(id.to_string(), response.filter.clone())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
                self.filters
                    .write()
                    .await
                    .insert(id.to_string(), response.filter.clone());
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

            self.aliases
                .lock()
                .await
                .entry(id.to_string())
                .or_default()
                .push(alias_num);

            let filter = Filter::new()
                .kinds(vec![Kind::Custom(SUBKEY_PROOF)])
                .events(events_to_broadcast.iter().map(|e| e.id));

            let alias = format!("{}_{}", id, alias_num);
            if let Some(selected_relays) = response.selected_relays.clone() {
                log::trace!(
                    "Subscribing 'subkey proof' to relays = {:?}",
                    selected_relays
                );
                self.channel
                    .subscribe_to(selected_relays, alias.clone(), filter.clone())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
                self.filters.write().await.insert(alias, filter);
            } else {
                log::trace!("Subscribing 'subkey proof' to all relays");
                // Subscribe to subkey proofs to all

                self.channel
                    .subscribe(alias.clone(), filter.clone())
                    .await
                    .map_err(|e| ConversationError::Inner(Box::new(e)))?;
                self.filters.write().await.insert(alias, filter);
            }
        }

        // check if Response has selected relays
        if let Some(selected_relays) = response.selected_relays {
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
        id: &str,
        mut conversation: Box<dyn Conversation + Send>,
    ) -> Result<Response, ConversationError> {
        let response = conversation.init()?;

        self.conversations
            .lock()
            .await
            .insert(id.to_string(), conversation);


        if let Some(selected_relays) = response.selected_relays.clone() {
            self.conversation_relays
                .lock()
                .await
                .insert(id.to_string(), selected_relays);
        }
        
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
        conversation: Box<dyn Conversation + Send>,
    ) -> Result<String, ConversationError> {
        let conversation_id = random_string(32);

        let response = self
            .internal_add_with_id(&conversation_id, conversation)
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
        id: String,
    ) -> Result<NotificationStream<T>, ConversationError> {
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
        let conversation_id = random_string(32);
        let delayed_reply = self
            .subscribe_to_service_request::<Conv::Notification>(conversation_id.clone())
            .await?;
        let response = self
            .internal_add_with_id(&conversation_id, Box::new(conversation))
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
    filter: Filter,
    responses: Vec<ResponseEntry>,
    notifications: Vec<serde_json::Value>,
    finished: bool,
    subscribe_to_subkey_proofs: bool,
    selected_relays: Option<Vec<String>>,
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
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }

    /// Sets the selected relays for this response.
    ///
    ///  # Arguments
    /// * `relays` - The list of relays to select
    pub fn selected_relays(mut self, relays: Vec<String>) -> Self {
        self.selected_relays = Some(relays);
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
pub enum ConversationMessage {
    Cleartext(CleartextEvent),
    Encrypted(Event),
    EndOfStoredEvents,
}

#[derive(thiserror::Error, Debug)]
pub enum ConversationError {
    #[error("Encrypted messages not supported")]
    Encrypted,

    #[error("User not set")]
    UserNotSet,

    #[error("Inner error: {0}")]
    Inner(Box<dyn std::error::Error + Send + Sync>),
}

pub trait Conversation {
    fn on_message(&mut self, message: ConversationMessage) -> Result<Response, ConversationError>;
    fn is_expired(&self) -> bool;
    fn init(&mut self) -> Result<Response, ConversationError> {
        Ok(Response::default())
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
