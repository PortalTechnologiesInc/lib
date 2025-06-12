use std::error::Error;

use nostr::{message::SubscriptionId, types::TryIntoUrl};
use nostr_relay_pool::{RelayOptions, RelayPool, RelayPoolNotification, SubscribeOptions};

/// A trait for an abstract channel
///
/// This is modeled around Nostr relays, in which we can subscribe to events matching a filter.
pub trait Channel: Send + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn subscribe(
        &self,
        id: String,
        filter: nostr::Filter,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

    fn subscribe_to<I, U>(
        &self,
        urls: I,
        id: String,
        filter: nostr::Filter,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send
    where
        <I as IntoIterator>::IntoIter: Send,
        I: IntoIterator<Item = U> + Send,
        U: TryIntoUrl,
        Self::Error: From<<U as TryIntoUrl>::Err>;

    fn unsubscribe(
        &self,
        id: String,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

    fn broadcast(
        &self,
        event: nostr::Event,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
    fn broadcast_to<I, U>(
        &self,
        urls: I,
        event: nostr::Event,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send
    where
        <I as IntoIterator>::IntoIter: Send,
        I: IntoIterator<Item = U> + Send,
        U: TryIntoUrl,
        Self::Error: From<<U as TryIntoUrl>::Err>;

    fn receive(
        &self,
    ) -> impl std::future::Future<Output = Result<RelayPoolNotification, Self::Error>> + Send;

    fn add_relay(
        &self,
        url: String,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

    fn remove_relay(
        &self,
        url: String,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}

impl Channel for RelayPool {
    type Error = nostr_relay_pool::pool::Error;

    async fn subscribe(&self, id: String, filter: nostr::Filter) -> Result<(), Self::Error> {
        self.subscribe_with_id(SubscriptionId::new(id), filter, SubscribeOptions::default())
            .await?;
        Ok(())
    }

    async fn subscribe_to<I, U>(
        &self,
        urls: I,
        id: String,
        filter: nostr::Filter,
    ) -> Result<(), Self::Error>
    where
        <I as IntoIterator>::IntoIter: Send,
        I: IntoIterator<Item = U> + Send,
        U: TryIntoUrl,
        Self::Error: From<<U as TryIntoUrl>::Err>,
    {
        self.subscribe_with_id_to(
            urls,
            SubscriptionId::new(id),
            filter,
            SubscribeOptions::default(),
        )
        .await?;
        Ok(())
    }

    async fn unsubscribe(&self, id: String) -> Result<(), Self::Error> {
        self.unsubscribe(&SubscriptionId::new(id)).await;
        Ok(())
    }

    async fn broadcast(&self, event: nostr::Event) -> Result<(), Self::Error> {
        self.send_event(&event).await?;
        Ok(())
    }
    async fn broadcast_to<I, U>(&self, urls: I, event: nostr::Event) -> Result<(), Self::Error>
    where
        <I as IntoIterator>::IntoIter: Send,
        I: IntoIterator<Item = U> + Send,
        U: TryIntoUrl,
        Self::Error: From<<U as TryIntoUrl>::Err>,
    {
        self.send_event_to(urls, &event).await?;
        Ok(())
    }

    async fn receive(&self) -> Result<RelayPoolNotification, Self::Error> {
        self.notifications()
            .recv()
            .await
            .map_err(|_| nostr_relay_pool::pool::Error::Shutdown)
    }

    async fn add_relay(&self, url: String) -> Result<(), Self::Error> {
        self.add_relay(&url, RelayOptions::default()).await?;
        self.connect_relay(url).await?;
        Ok(())
    }

    async fn remove_relay(&self, url: String) -> Result<(), Self::Error> {
        self.remove_relay(url).await?;
        Ok(())
    }
}
