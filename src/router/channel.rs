use nostr::{message::SubscriptionId, types::TryIntoUrl};
use nostr_relay_pool::{
    RelayPool, RelayPoolNotification, SubscribeOptions,
    relay::{FlagCheck, RelayServiceFlags},
};

use crate::router::ids::PortalId;

/// A trait for an abstract channel
///
/// This is modeled around Nostr relays, in which we can subscribe to events matching a filter.
pub trait Channel: Send + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn subscribe(
        &self,
        id: PortalId,
        filter: nostr::Filter,
    ) -> impl std::future::Future<Output = Result<usize, Self::Error>> + Send;

    fn subscribe_to<I, U>(
        &self,
        urls: I,
        id: PortalId,
        filter: nostr::Filter,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send
    where
        <I as IntoIterator>::IntoIter: Send,
        I: IntoIterator<Item = U> + Send,
        U: TryIntoUrl,
        Self::Error: From<<U as TryIntoUrl>::Err>;

    fn unsubscribe(
        &self,
        id: PortalId,
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

    fn shutdown(&self) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}

impl Channel for RelayPool {
    type Error = nostr_relay_pool::pool::Error;

    async fn subscribe(&self, id: PortalId, filter: nostr::Filter) -> Result<usize, Self::Error> {
        self.subscribe_with_id(
            SubscriptionId::new(id.to_string()),
            filter,
            SubscribeOptions::default(),
        )
        .await?;

        let relays = self.__write_relay_urls().await;
        Ok(relays.len())
    }

    async fn subscribe_to<I, U>(
        &self,
        urls: I,
        id: PortalId,
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
            SubscriptionId::new(id.to_string()),
            filter,
            SubscribeOptions::default(),
        )
        .await?;
        Ok(())
    }

    async fn unsubscribe(&self, id: PortalId) -> Result<(), Self::Error> {
        let relays = self
            .relays_with_flag(RelayServiceFlags::READ, FlagCheck::All)
            .await;
        for relay in relays.values() {
            if let Err(e) = relay
                .unsubscribe(&SubscriptionId::new(id.to_string()))
                .await
            {
                log::error!(
                    "Failed to unsubscribe {id} from relay {}: {:?}",
                    relay.url(),
                    e
                );
            }
        }
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

    async fn shutdown(&self) -> Result<(), Self::Error> {
        self.shutdown().await;
        Ok(())
    }
}

impl<C: Channel + Send + Sync> Channel for std::sync::Arc<C> {
    type Error = C::Error;

    async fn subscribe(&self, id: PortalId, filter: nostr::Filter) -> Result<usize, Self::Error> {
        <C as Channel>::subscribe(self, id, filter).await
    }

    async fn subscribe_to<I, U>(
        &self,
        urls: I,
        id: PortalId,
        filter: nostr::Filter,
    ) -> Result<(), Self::Error>
    where
        <I as IntoIterator>::IntoIter: Send,
        I: IntoIterator<Item = U> + Send,
        U: TryIntoUrl,
        Self::Error: From<<U as TryIntoUrl>::Err>,
    {
        <C as Channel>::subscribe_to(self, urls, id, filter).await
    }

    async fn unsubscribe(&self, id: PortalId) -> Result<(), Self::Error> {
        <C as Channel>::unsubscribe(self, id).await
    }

    async fn broadcast(&self, event: nostr::Event) -> Result<(), Self::Error> {
        <C as Channel>::broadcast(self, event).await
    }

    async fn broadcast_to<I, U>(&self, urls: I, event: nostr::Event) -> Result<(), Self::Error>
    where
        <I as IntoIterator>::IntoIter: Send,
        I: IntoIterator<Item = U> + Send,
        U: TryIntoUrl,
        Self::Error: From<<U as TryIntoUrl>::Err>,
    {
        <C as Channel>::broadcast_to(self, urls, event).await
    }

    async fn receive(&self) -> Result<RelayPoolNotification, Self::Error> {
        <C as Channel>::receive(self).await
    }

    async fn shutdown(&self) -> Result<(), Self::Error> {
        <C as Channel>::shutdown(self).await
    }
}
