use std::{
    ops::{Deref, DerefMut},
};

use futures::Stream;
use serde::Serialize;

pub mod actor;
pub mod channel;
pub mod conversation;
pub mod filters;
pub mod ids;


// Re-export MessageRouterActor as MessageRouter for backward compatibility
pub use actor::{MessageRouterActor as MessageRouter, MessageRouterActorError};


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

impl<T: Serialize> std::fmt::Debug for NotificationStream<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotificationStream").finish()
    }
}
