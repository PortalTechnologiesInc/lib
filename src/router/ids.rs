use std::error::Error;
use std::fmt::Debug;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::utils::random_string;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ConversationId {
    Standard(String),
    Alias(String, u64),
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

impl Display for ConversationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversationId::Standard(id) => write!(f, "p1{}", id),
            ConversationId::Alias(id, alias) => write!(f, "p2{}_{}", id, alias),
        }
    }
}

impl Debug for ConversationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<ConversationId> for String {
    fn from(id: ConversationId) -> Self {
        id.to_string()
    }
}

impl FromStr for ConversationId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 3 {
            return Err(ParseIdError);
        }

        match &s[..2] {
            "p1" => {
                let id = &s[2..];
                if id.is_empty() {
                    return Err(ParseIdError);
                }
                Ok(ConversationId::Standard(id.to_string()))
            }
            "p2" => {
                let rest = &s[2..];
                if let Some((id, alias_str)) = rest.split_once('_') {
                    if let Ok(alias) = alias_str.parse::<u64>() {
                        return Ok(ConversationId::Alias(id.to_string(), alias));
                    }
                }
                Err(ParseIdError)
            }
            _ => Err(ParseIdError),
        }
    }
}

impl ConversationId {
    /// Create a new conversation ID
    pub fn new_conversation() -> Self {
        Self::Standard(random_string(30))
    }

    /// Create a new conversation alias ID
    pub fn new_conversation_alias(conversation_id: &str, alias: u64) -> Self {
        Self::Alias(conversation_id.to_string(), alias)
    }

    /// Get the underlying ID string (without prefix)
    pub fn id(&self) -> &str {
        match self {
            ConversationId::Standard(id) => id,
            ConversationId::Alias(id, _) => id,
        }
    }

    /// Check if this is a conversation ID
    pub fn is_conversation(&self) -> bool {
        matches!(self, ConversationId::Standard(_))
    }

    /// Check if this is a conversation alias ID
    pub fn is_conversation_alias(&self) -> bool {
        matches!(self, ConversationId::Alias(_, _))
    }

    /// Get the alias if this is a conversation alias ID
    pub fn alias(&self) -> Option<u64> {
        match self {
                ConversationId::Alias(_, alias) => Some(*alias),
            _ => None,
        }
    }

    /// Parse an ID string into a ConversationId
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Example usage of the ConversationId system
///
/// ```rust
/// use portal::router::ids::ConversationId;
///
/// // Create new conversation IDs
/// let conv_id = ConversationId::new_conversation();
/// let alias_id = ConversationId::new_conversation_alias("abc123", 42);
///
/// // Parse from strings
/// let parsed: ConversationId = "p1abc123".parse().unwrap();
/// let parsed_alias: ConversationId = "p2abc123_42".parse().unwrap();
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
        let id = ConversationId::new_conversation();
        assert!(id.is_conversation());
        assert!(!id.is_conversation_alias());
    }

    #[test]
    fn test_conversation_id_display() {
        let id = ConversationId::Standard("abc123".to_string());
        assert_eq!(id.to_string(), "p1abc123");
    }

    #[test]
    fn test_conversation_id_parsing() {
        let parsed = ConversationId::from_str("p1abc123").unwrap();
        assert!(parsed.is_conversation());
        assert_eq!(parsed.id(), "abc123");
    }

    #[test]
    fn test_conversation_alias_creation() {
        let id = ConversationId::new_conversation_alias("abc123", 42);
        assert!(id.is_conversation_alias());
        assert_eq!(id.alias(), Some(42));
    }

    #[test]
    fn test_conversation_alias_display() {
        let id = ConversationId::Alias("abc123".to_string(), 42);
        assert_eq!(id.to_string(), "p2abc123_42");
    }

    #[test]
    fn test_conversation_alias_parsing() {
        let parsed = ConversationId::from_str("p2abc123_42").unwrap();
        assert!(parsed.is_conversation_alias());
        assert_eq!(parsed.id(), "abc123");
        assert_eq!(parsed.alias(), Some(42));
    }

    #[test]
    fn test_invalid_parsing() {
        assert!(ConversationId::from_str("invalid").is_err());
        assert!(ConversationId::from_str("p1").is_err());
        assert!(ConversationId::from_str("p2abc").is_err());
        assert!(ConversationId::from_str("p2abc_").is_err());
        assert!(ConversationId::from_str("p2abc_invalid").is_err());
    }
}
