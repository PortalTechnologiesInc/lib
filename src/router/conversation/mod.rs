mod adapters;
pub mod message;
pub mod response;

use message::ConversationMessage;
use response::Response;

// Re-export adapters

pub use adapters::multi_key_listener::MultiKeyListener;
pub use adapters::multi_key_listener::MultiKeyListenerAdapter;

pub use adapters::multi_key_sender::MultiKeySender;
pub use adapters::multi_key_sender::MultiKeySenderAdapter;

pub use adapters::one_shot::OneShotSender;
pub use adapters::one_shot::OneShotSenderAdapter;

pub use adapters::ConversationWithNotification;

/// A box of a conversation
pub type ConversationBox = Box<dyn Conversation + Send + Sync>;

impl std::fmt::Debug for ConversationBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conversation").finish()
    }
}

pub trait Conversation: ToString {
    fn on_message(&mut self, message: ConversationMessage) -> Result<Response, ConversationError>;
    fn is_expired(&self) -> bool;
    fn init(&mut self) -> Result<Response, ConversationError> {
        Ok(Response::default())
    }
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

    #[error("Conversation not found")]
    ConversationNotFound,
}
