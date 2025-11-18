use std::error::Error;
use std::fmt::Debug;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::utils::random_string;

const CONVERSATION_ID_PREFIX: &str = "p1";
const CONVERSATION_ALIAS_ID_PREFIX: &str = "p2";

const SUBSCRIPTION_ID_PREFIX: &str = "p0";


// Subscription ID
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PortalSubscriptionId(String);

impl PortalSubscriptionId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
    pub fn generate() -> Self {
        Self(random_string(30))
    }
}

impl Debug for PortalSubscriptionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}


impl Display for PortalSubscriptionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", SUBSCRIPTION_ID_PREFIX, self.0)
    }
}

impl FromStr for PortalSubscriptionId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 3 {
            return Err(ParseIdError);
        }
        if &s[..2] != SUBSCRIPTION_ID_PREFIX {
            return Err(ParseIdError);
        }
        Ok(PortalSubscriptionId(s[2..].to_string()))
    }
}

// Conversation ID

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum PortalConversationId {
    Conversation(String),
    ConversationAlias(String, u64),
    // Add more ID types here as needed
    // Subscription(String),
    // Payment(String),
}

#[derive(Debug, Clone)]
pub struct ParseIdError;

impl Display for ParseIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid Portal ID format")
    }
}

impl Error for ParseIdError {}

impl Display for PortalConversationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PortalConversationId::Conversation(id) => write!(f, "{}{}", CONVERSATION_ID_PREFIX, id),
            PortalConversationId::ConversationAlias(id, alias) => write!(f, "{}{}_{}", CONVERSATION_ALIAS_ID_PREFIX, id, alias),
        }
    }
}

impl Debug for PortalConversationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<PortalConversationId> for String {
    fn from(id: PortalConversationId) -> Self {
        id.to_string()
    }
}

impl FromStr for PortalConversationId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 3 {
            return Err(ParseIdError);
        }

        match &s[..2] {
            CONVERSATION_ID_PREFIX => {
                let id = &s[2..];
                if id.is_empty() {
                    return Err(ParseIdError);
                }
                Ok(PortalConversationId::Conversation(id.to_string()))
            }
            CONVERSATION_ALIAS_ID_PREFIX => {
                let rest = &s[2..];
                if let Some((id, alias_str)) = rest.split_once('_') {
                    if let Ok(alias) = alias_str.parse::<u64>() {
                        return Ok(PortalConversationId::ConversationAlias(id.to_string(), alias));
                    }
                }
                Err(ParseIdError)
            }
            _ => Err(ParseIdError),
        }
    }
}

impl PortalConversationId {
    /// Create a new conversation ID
    pub fn new_conversation() -> Self {
        Self::Conversation(random_string(30))
    }

    /// Create a new conversation alias ID
    pub fn new_conversation_alias(conversation_id: &str, alias: u64) -> Self {
        PortalConversationId::ConversationAlias(conversation_id.to_string(), alias)
    }

    /// Get the underlying ID string (without prefix)
    pub fn id(&self) -> &str {
        match self {
            PortalConversationId::Conversation(id) => id,
            PortalConversationId::ConversationAlias(id, _) => id,
        }
    }

    /// Check if this is a conversation ID
    pub fn is_conversation(&self) -> bool {
        matches!(self, PortalConversationId::Conversation(_))
    }

    /// Check if this is a conversation alias ID
    pub fn is_conversation_alias(&self) -> bool {
        matches!(self, PortalConversationId::ConversationAlias(_, _))
    }

    /// Get the alias if this is a conversation alias ID
    pub fn alias(&self) -> Option<u64> {
        match self {
            PortalConversationId::ConversationAlias(_, alias) => Some(*alias),
            _ => None,
        }
    }

    /// Parse an ID string into a PortalId
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Example usage of the PortalId system
///
/// ```rust
/// use portal::router::ids::PortalId;
///
/// // Create new conversation IDs
/// let conv_id = PortalId::new_conversation();
/// let alias_id = PortalId::new_conversation_alias("abc123", 42);
///
/// // Parse from strings
/// let parsed: PortalId = "p1abc123".parse().unwrap();
/// let parsed_alias: PortalId = "p2abc123_42".parse().unwrap();
///
/// // Check types
/// assert!(parsed.is_conversation());
/// assert!(parsed_alias.is_conversation_alias());
///
/// // Get underlying data
/// assert_eq!(parsed.id(), "abc123");
/// assert_eq!(parsed_alias.alias(), Some(42));
///
/// // Display
/// assert_eq!(conv_id.to_string().len(), 33); // "p1" + 30 chars
/// assert_eq!(alias_id.to_string(), "p2abc123_42");
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_id_creation() {
        let id = PortalConversationId::new_conversation();
        assert!(id.is_conversation());
        assert!(!id.is_conversation_alias());
    }

    #[test]
    fn test_conversation_id_display() {
        let id = PortalConversationId::Conversation("abc123".to_string());
        assert_eq!(id.to_string(), "p1abc123");
    }

    #[test]
    fn test_conversation_id_parsing() {
        let parsed = PortalConversationId::from_str("p1abc123").unwrap();
        assert!(parsed.is_conversation());
        assert_eq!(parsed.id(), "abc123");
    }

    #[test]
    fn test_conversation_alias_creation() {
        let id = PortalConversationId::new_conversation_alias("abc123", 42);
        assert!(id.is_conversation_alias());
        assert_eq!(id.alias(), Some(42));
    }

    #[test]
    fn test_conversation_alias_display() {
        let id = PortalConversationId::ConversationAlias("abc123".to_string(), 42);
        assert_eq!(id.to_string(), "p2abc123_42");
    }

    #[test]
    fn test_conversation_alias_parsing() {
        let parsed = PortalConversationId::from_str("p2abc123_42").unwrap();
        assert!(parsed.is_conversation_alias());
        assert_eq!(parsed.id(), "abc123");
        assert_eq!(parsed.alias(), Some(42));
    }

    #[test]
    fn test_invalid_parsing() {
        assert!(PortalConversationId::from_str("invalid").is_err());
        assert!(PortalConversationId::from_str("p1").is_err());
        assert!(PortalConversationId::from_str("p2abc").is_err());
        assert!(PortalConversationId::from_str("p2abc_").is_err());
        assert!(PortalConversationId::from_str("p2abc_invalid").is_err());
    }

    #[test]
    fn test_subscription_id_creation() {
        let id = PortalSubscriptionId::new("abc123".to_string());
        assert_eq!(id.to_string(), "p0abc123");
    }

    #[test]
    fn test_subscription_id_parsing() {
        let parsed = PortalSubscriptionId::from_str("p0abc123").unwrap();
        assert_eq!(parsed.0, "abc123");
    }

    #[test]
    fn test_invalid_subscription_id_parsing() {
        assert!(PortalSubscriptionId::from_str("invalid").is_err());
        assert!(PortalSubscriptionId::from_str("p0").is_err());
        assert!(PortalSubscriptionId::from_str("p0abc").is_ok());
        assert!(PortalSubscriptionId::from_str("p1abc123").is_err());

    }
}
