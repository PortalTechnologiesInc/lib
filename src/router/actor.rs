use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    str::FromStr,
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
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio_stream::StreamExt;

use crate::{
    protocol::{LocalKeypair, model::event_kinds::SUBKEY_PROOF},
    router::{
        NotificationStream,
        channel::Channel,
        conversation::{
            Conversation, ConversationBox, ConversationError,
            message::{CleartextEvent, ConversationMessage},
            response::Response,
        },
        ids::{ConversationId, PortalSubscriptionId},
        state::MessageRouterState,
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
enum MessageRouterActorMessage {
    AddRelay(String, bool, oneshot::Sender<Result<(), ConversationError>>),
    RemoveRelay(String, oneshot::Sender<Result<(), ConversationError>>),
    Shutdown(oneshot::Sender<Result<(), ConversationError>>),
    AddConversation(
        ConversationBox,
        oneshot::Sender<Result<ConversationId, ConversationError>>,
    ),
    AddConversationWithRelays(
        ConversationBox,
        Vec<String>,
        oneshot::Sender<Result<ConversationId, ConversationError>>,
    ),
    SubscribeToServiceRequest(
        ConversationId,
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
            let mut state = MessageRouterState::new(keypair_clone);
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
    ) -> Result<ConversationId, MessageRouterActorError> {
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
    ) -> Result<ConversationId, MessageRouterActorError> {
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
        id: ConversationId,
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
        id: ConversationId,
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
