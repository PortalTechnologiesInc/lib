use nostr::{event::{Event, EventId, Kind, Tags}, key::PublicKey};

#[derive(Debug, Clone)]
pub enum ConversationMessage {
    Cleartext(CleartextEvent),
    Encrypted(Event),
    EndOfStoredEvents,
}

#[derive(Debug, Clone)]
pub struct CleartextEvent {
    pub id: EventId,
    pub pubkey: PublicKey,
    pub created_at: nostr::types::Timestamp,
    pub kind: Kind,
    pub tags: Tags,
    pub content: serde_json::Value,
}

impl CleartextEvent {
    pub fn new(event: &Event, decrypted: &str) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id: event.id,
            pubkey: event.pubkey,
            created_at: event.created_at,
            kind: event.kind,
            tags: event.tags.clone(),
            content: serde_json::from_str(decrypted)?,
        })
    }

    pub fn new_json(event: &Event, content: serde_json::Value) -> Self {
        Self {
            id: event.id,
            pubkey: event.pubkey,
            created_at: event.created_at,
            kind: event.kind,
            tags: event.tags.clone(),
            content,
        }
    }
}
