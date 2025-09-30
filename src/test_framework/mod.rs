use std::{collections::HashMap, sync::Arc};

use nostr::{
    RelayUrl,
    event::Event,
    filter::{Filter, MatchEventOptions},
    message::SubscriptionId,
};
use nostr_relay_pool::RelayPoolNotification;
use tokio::sync::{Mutex, RwLock, mpsc};

use crate::{
    protocol::LocalKeypair,
    router::{
        Conversation, ConversationError, MessageRouter, MessageRouterActorError, ConversationId,
        channel::Channel,
    },
};

pub mod logger;

/// A simulated channel that broadcasts messages to all connected nodes
pub struct SimulatedChannel {
    subscribers: Arc<RwLock<HashMap<ConversationId, (Filter, mpsc::Sender<RelayPoolNotification>)>>>,
    messages: Arc<Mutex<Vec<Event>>>,
    senders: Arc<Mutex<Vec<mpsc::Sender<RelayPoolNotification>>>>,
    receiver: Mutex<mpsc::Receiver<RelayPoolNotification>>,
    my_sender: mpsc::Sender<RelayPoolNotification>,
}

impl SimulatedChannel {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(32);
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            messages: Arc::new(Mutex::new(Vec::new())),
            senders: Arc::new(Mutex::new(vec![tx.clone()])),
            receiver: Mutex::new(rx),
            my_sender: tx,
        }
    }
}

impl SimulatedChannel {
    async fn clone(&self) -> Self {
        let (tx, rx) = mpsc::channel(32);

        // Add the new sender and receiver to their respective lists
        let mut senders = self.senders.lock().await;
        senders.push(tx.clone());

        Self {
            subscribers: self.subscribers.clone(),
            messages: self.messages.clone(),
            receiver: Mutex::new(rx),
            my_sender: tx,
            senders: self.senders.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SimulatedChannelError {
    #[error("Channel closed")]
    ChannelClosed,

    #[error("URL error: {0}")]
    UrlError(#[from] nostr::types::url::Error),
}

impl Channel for SimulatedChannel {
    type Error = SimulatedChannelError;

    async fn subscribe(&self, id: ConversationId, filter: Filter) -> Result<usize, Self::Error> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(id.clone(), (filter, self.my_sender.clone()));
        Ok(subscribers.len())

        // Send any existing messages that match the filter
        // let messages = self.messages.lock().await;
        // for event in messages.iter() {
        //     if filter.match_event(event) {
        //         let relay_url = RelayUrl::parse("wss://simulated").unwrap();
        //         let notification = RelayPoolNotification::Event {
        //             event: Box::new(event.clone()),
        //             subscription_id: SubscriptionId::new(id.to_string()),
        //             relay_url,
        //         };
        //         let _ = self.broadcast_notification(notification).await;
        //     }
        // }
    }

    async fn subscribe_to<I, U>(
        &self,
        urls: I,
        id: ConversationId,
        filter: nostr::Filter,
    ) -> Result<(), Self::Error>
    where
        <I as IntoIterator>::IntoIter: Send,
        I: IntoIterator<Item = U> + Send,
        U: nostr::types::TryIntoUrl,
        Self::Error: From<<U as nostr::types::TryIntoUrl>::Err>,
    {
        // TODO: use the urls to create a filter
        self.subscribers
            .write()
            .await
            .insert(id.clone(), (filter, self.my_sender.clone()));
        Ok(())
    }

    async fn unsubscribe(&self, id: ConversationId) -> Result<(), Self::Error> {
        self.subscribers.write().await.remove(&id);
        Ok(())
    }

    async fn broadcast(&self, event: Event) -> Result<(), Self::Error> {
        // Store the event
        self.messages.lock().await.push(event.clone());

        // Broadcast to all subscribers
        let subscribers = self.subscribers.write().await;
        for (subscription_id, (filter, sender)) in subscribers.iter() {
            if filter.match_event(&event, MatchEventOptions::default()) {
                let relay_url = RelayUrl::parse("wss://simulated").unwrap();
                let notification = RelayPoolNotification::Event {
                    event: Box::new(event.clone()),
                    subscription_id: SubscriptionId::new(subscription_id.to_string()),
                    relay_url,
                };
                let _ = sender.send(notification).await;
            }
        }

        Ok(())
    }

    async fn broadcast_to<I, U>(&self, urls: I, event: Event) -> Result<(), Self::Error>
    where
        <I as IntoIterator>::IntoIter: Send,
        I: IntoIterator<Item = U> + Send,
        U: nostr::types::TryIntoUrl,
        Self::Error: From<<U as nostr::types::TryIntoUrl>::Err>,
    {
        // Store the event
        self.messages.lock().await.push(event.clone());

        // Broadcast to all subscribers
        let subscribers = self.subscribers.write().await;
        for (subscription_id, (filter, sender)) in subscribers.iter() {
            if filter.match_event(&event, MatchEventOptions::default()) {
                let relay_url = RelayUrl::parse("wss://simulated").unwrap();
                let notification = RelayPoolNotification::Event {
                    event: Box::new(event.clone()),
                    subscription_id: SubscriptionId::new(subscription_id.to_string()),
                    relay_url,
                };
                let _ = sender.send(notification).await;
            }
        }

        Ok(())
    }

    async fn receive(&self) -> Result<RelayPoolNotification, Self::Error> {
        // Try to receive from our receiver
        let mut receiver = self.receiver.lock().await;
        if let Some(notification) = receiver.recv().await {
            return Ok(notification);
        } else {
            Err(SimulatedChannelError::ChannelClosed)
        }
    }

    async fn shutdown(&self) -> Result<(), Self::Error> {
        // For simulated channel, just clear the senders
        self.senders.lock().await.clear();
        Ok(())
    }
}

/// A simulated network of Nostr nodes
pub struct SimulatedNetwork {
    channel: SimulatedChannel,
    nodes: HashMap<String, Arc<MessageRouter<SimulatedChannel>>>,
}

impl SimulatedNetwork {
    pub fn new() -> Self {
        Self {
            channel: SimulatedChannel::new(),
            nodes: HashMap::new(),
        }
    }

    /// Add a new node to the network
    pub async fn add_node(
        &mut self,
        id: String,
        keypair: LocalKeypair,
    ) -> Arc<MessageRouter<SimulatedChannel>> {
        let router = Arc::new(MessageRouter::new(self.channel.clone().await, keypair));
        self.nodes.insert(id, Arc::clone(&router));
        router
    }

    /// Get a node by its ID
    pub fn get_node(&self, id: &str) -> Option<&Arc<MessageRouter<SimulatedChannel>>> {
        self.nodes.get(id)
    }

    /// Start listening for messages on all nodes
    pub async fn start(&self) {
        for (_, router) in self.nodes.iter() {
            let router = Arc::clone(router);
            tokio::spawn(async move {
                let _ = router.listen().await;
            });
        }
    }
}

/// Helper to create test scenarios
pub struct ScenarioBuilder {
    network: SimulatedNetwork,
}

impl ScenarioBuilder {
    pub fn new() -> Self {
        Self {
            network: SimulatedNetwork::new(),
        }
    }

    pub async fn with_node(mut self, id: String, keypair: LocalKeypair) -> Self {
        self.network.add_node(id, keypair).await;
        self
    }

    pub async fn with_conversation<C: Conversation + Send + Sync + 'static>(
        self,
        node_id: &str,
        conversation: C,
    ) -> Result<Self, ConversationError> {
        if let Some(router) = self.network.get_node(node_id) {
            router
                .add_conversation(Box::new(conversation))
                .await
                .map_err(|e| match e {
                    MessageRouterActorError::Conversation(ce) => ce,
                    MessageRouterActorError::Channel(_) => ConversationError::Inner(Box::new(
                        std::io::Error::new(std::io::ErrorKind::Other, "Channel error"),
                    )),
                    MessageRouterActorError::Receiver(_) => ConversationError::Inner(Box::new(
                        std::io::Error::new(std::io::ErrorKind::Other, "Receiver error"),
                    )),
                })?;
        }
        Ok(self)
    }

    pub async fn run(self) -> SimulatedNetwork {
        let network = self.network;
        network.start().await;
        network
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr::Keys;

    #[tokio::test]
    async fn test_basic_scenario() {
        let keys1 = Keys::generate();
        let keys2 = Keys::generate();

        let network = ScenarioBuilder::new()
            .with_node("node1".to_string(), LocalKeypair::new(keys1, None))
            .await
            .with_node("node2".to_string(), LocalKeypair::new(keys2, None))
            .await
            .run()
            .await;

        // Test that both nodes exist
        assert!(network.get_node("node1").is_some());
        assert!(network.get_node("node2").is_some());
    }

    pub mod auth_scenario;
}
