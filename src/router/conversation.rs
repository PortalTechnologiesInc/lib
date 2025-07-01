use std::{collections::HashSet, fmt::Display};

use nostr::event::Event;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    router::{CleartextEvent, Response},
    utils::random_string,
};

// Conversation Id

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

// END - Conversation Id

// Conversation Trait

pub type DynConversation = Box<dyn Conversation + Send >;

pub trait Conversation {
    fn on_message(&mut self, message: ConversationMessage) -> Result<Response, ConversationError>;
    fn is_expired(&self) -> bool;
    fn init(&mut self) -> Result<Response, ConversationError> {
        Ok(Response::default())
    }
}

// END - Conversation Trait

// Conversation Message

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

    #[error("Filter '{0}' not found")]
    FilterNotFound(ConversationFilterId),

    #[error("Conversation not found")]
    ConversationNotFound,
}

// END - Conversation Message

// Conversation State

pub struct ConversationState {
    pub conversation: DynConversation,
    pub filter: Option<ConversationFilterId>,

    pub aliases: Option<Vec<u64>>,
    end_of_stored_events: Option<usize>,
}

impl ConversationState {
    pub fn new(conversation: DynConversation) -> Self {
        Self {
            conversation,
            filter: None,
            aliases: None,
            end_of_stored_events: None,
        }
    }

    // alias

    pub fn add_alias(&mut self, alias: u64) {
        if let Some(aliases) = &mut self.aliases {
            if !aliases.contains(&alias) {
                aliases.push(alias);
            }
        } else {
            self.aliases = Some(vec![alias]);
        }
    }

    // EOSE

    pub fn increment_eose(&mut self) {
        self.end_of_stored_events = self.end_of_stored_events.map(|v| v + 1);
    }

    pub fn decrease_eose(&mut self) -> Option<usize> {
        self.end_of_stored_events = self.end_of_stored_events.map(|v| v - 1);
        self.end_of_stored_events
    }

    pub fn saturating_sub_eose(&mut self) {
        self.end_of_stored_events = self.end_of_stored_events.map(|v| v.saturating_sub(1));
    }

    pub fn reset_eose(&mut self) {
        self.end_of_stored_events = None;
    }

    pub fn set_eose(&mut self, value: usize) {
        self.end_of_stored_events = Some(value);
    }


}

// END - Conversation State

// Conversation Filter

pub type ConversationFilterId = String;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MergeConversationPolicy {
    SameKind,
}

// END - Conversation Filter


// Conversation Relays Context

#[derive(Debug, Clone)]
pub enum ConversationRelaysContext {
    Global,
    Targeted(HashSet<String>)
}

// END - Conversation Relays Context
// END - Conversation Relays Context
// END - Conversation Relays Context