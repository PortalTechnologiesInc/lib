use std::{borrow::Borrow, fmt::Display, ops::Deref};

use nostr::event::Event;
use serde::{Deserialize, Serialize};

use crate::{
    router::{CleartextEvent, Response},
    utils::random_string,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConversationId(String);

impl ConversationId {
    pub fn generate() -> Self {
        Self(random_string(32))
    }

    pub fn from_alias(id: &str, alias: u64) -> Self {
        Self(format!("{}_{}", id, alias))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ConversationId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl Display for ConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/*
impl Deref for ConversationId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
*/

pub type DynConversation = Box<dyn Conversation + Send>;

pub trait Conversation {
    fn on_message(&mut self, message: ConversationMessage) -> Result<Response, ConversationError>;
    fn is_expired(&self) -> bool;
    fn init(&mut self) -> Result<Response, ConversationError> {
        Ok(Response::default())
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

    #[error("Relay '{0}' is not connected")]
    RelayNotConnected(String),
}
