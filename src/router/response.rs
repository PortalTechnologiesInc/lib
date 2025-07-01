use std::collections::HashSet;

use nostr::{event::{Kind, Tags}, filter::Filter, key::PublicKey};

#[derive(Debug)]
pub struct ResponseEntry {
    pub recepient_keys: Vec<PublicKey>,
    pub kind: Kind,
    pub tags: Tags,
    pub content: serde_json::Value,
    pub encrypted: bool,
}

/// A response from a conversation.
///
/// Responses can include:
/// - Filters for subscribing to specific message types
/// - Replies to send to specific recipients or broadcast to all participants in the conversation
/// - Notifications to send to subscribers
/// - A flag indicating if the conversation is finished. If set, the conversation will be removed from the router.
///
/// # Example
/// ```rust,no_run
/// use portal::router::Response;
/// use nostr::{Filter, Kind, Tags};
///
/// let response = Response::new()
///     .filter(Filter::new().kinds(vec![Kind::from(27000)]))
///     .reply_to(pubkey, Kind::from(27001), Tags::new(), content)
///     .notify(notification)
///     .finish();
/// ```
#[derive(Debug, Default)]
pub struct Response {
    pub filter: Filter,
    pub responses: Vec<ResponseEntry>,
    pub notifications: Vec<serde_json::Value>,
    pub finished: bool,
    pub subscribe_to_subkey_proofs: bool,
}

impl Response {
    /// Creates a new empty response.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the filter for this response.
    ///
    /// The filter will be used to subscribe to specific message types with the relays.
    ///
    /// # Arguments
    /// * `filter` - The filter to set
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }

    /// Adds a reply to be sent to all recipients.
    ///
    /// # Arguments
    /// * `kind` - The kind of message to send
    /// * `tags` - The tags to include in the message
    /// * `content` - The content to send, must be serializable
    pub fn reply_all<S: serde::Serialize>(mut self, kind: Kind, tags: Tags, content: S) -> Self {
        let content = serde_json::to_value(&content).unwrap();
        self.responses.push(ResponseEntry {
            recepient_keys: vec![],
            kind,
            tags,
            content,
            encrypted: true,
        });
        self
    }

    /// Adds a reply to be sent to a specific recipient.
    ///
    /// # Arguments
    /// * `pubkey` - The public key of the recipient
    /// * `kind` - The kind of message to send
    /// * `tags` - The tags to include in the message
    /// * `content` - The content to send, must be serializable
    pub fn reply_to<S: serde::Serialize>(
        mut self,
        pubkey: PublicKey,
        kind: Kind,
        tags: Tags,
        content: S,
    ) -> Self {
        let content = serde_json::to_value(&content).unwrap();
        self.responses.push(ResponseEntry {
            recepient_keys: vec![pubkey],
            kind,
            tags,
            content,
            encrypted: true,
        });
        self
    }

    /// Adds a notification to be sent to subscribers.
    ///
    /// # Arguments
    /// * `data` - The notification data to send, must be serializable
    pub fn notify<S: serde::Serialize>(mut self, data: S) -> Self {
        let content = serde_json::to_value(&data).unwrap();
        self.notifications.push(content);
        self
    }

    /// Marks the conversation as finished.
    ///
    /// When a conversation is finished, it will be removed from the router.
    pub fn finish(mut self) -> Self {
        self.finished = true;
        self
    }

    /// Subscribe to events that tag our replies via the event_id
    pub fn subscribe_to_subkey_proofs(mut self) -> Self {
        self.subscribe_to_subkey_proofs = true;
        self
    }

    // Broadcast an unencrypted event
    pub fn broadcast_unencrypted<S: serde::Serialize>(
        mut self,
        kind: Kind,
        tags: Tags,
        content: S,
    ) -> Self {
        let content = serde_json::to_value(&content).unwrap();
        self.responses.push(ResponseEntry {
            recepient_keys: vec![],
            kind,
            tags,
            content,
            encrypted: false,
        });
        self
    }

    pub fn set_recepient_keys(&mut self, user: PublicKey, subkeys: &HashSet<PublicKey>) {
        for response in &mut self.responses {
            if response.recepient_keys.is_empty() {
                response.recepient_keys.push(user);
                response.recepient_keys.extend(subkeys.iter().cloned());
            }
        }
    }

    pub fn extend(&mut self, response: Response) {
        self.responses.extend(response.responses);
        self.subscribe_to_subkey_proofs |= response.subscribe_to_subkey_proofs;
    }
}