//! Adapters for conversations
//!
//! This module contains adapters for conversations that follow specific patterns.

use serde::{Serialize, de::DeserializeOwned};

use super::Conversation;

pub mod multi_key_listener;
pub mod multi_key_sender;
pub mod one_shot;

/// A trait for conversations that send notifications
///
/// This can be used to add a conversation and immediately subscribe to its notifications within
/// the message router with [`crate::router::MessageRouter::add_and_subscribe`]
pub trait ConversationWithNotification: Conversation {
    type Notification: Serialize + DeserializeOwned;
}
