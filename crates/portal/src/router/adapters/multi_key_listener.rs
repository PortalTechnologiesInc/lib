use std::{
    collections::HashSet,
    ops::Deref,
    time::{Duration, SystemTime},
};

use serde::de::DeserializeOwned;

use nostr::{
    event::{Kind, Tag},
    key::PublicKey,
};

use crate::protocol::model::{auth::SubkeyProof, event_kinds::SUBKEY_PROOF};

use crate::router::{
    CleartextEvent, Conversation, ConversationError, ConversationMessage, Response,
};

pub trait MultiKeyListener: Sized + Send + 'static {
    const VALIDITY_SECONDS: Option<u64>;

    type Error: std::error::Error + Send + Sync + 'static;
    type Message: DeserializeOwned;

    fn init(_state: &MultiKeyListenerAdapter<Self>) -> Result<Response, Self::Error>;

    fn on_message(
        _state: &mut MultiKeyListenerAdapter<Self>,
        _event: &CleartextEvent,
        _message: &Self::Message,
    ) -> Result<Response, Self::Error>;
}

/// A listener conversation wrapper that handles key switching
///
/// This is specifically used for listeners which follow this pattern:
///   1. Set some filters to listen for messages of a specific kind
///   2. Potentially receive an encrypted message because it was sent to the main key or another subkey
///   3. Reply with a SUBKEY_PROOF message asking to include us in the conversation
///   4. Wait for the non-encrypted message
pub struct MultiKeyListenerAdapter<Inner> {
    pub user: Option<PublicKey>,
    pub subkey_proof: Option<SubkeyProof>,
    pub expires_at: Option<SystemTime>,
    pub inner: Inner,
}

impl<T: MultiKeyListener> Conversation for MultiKeyListenerAdapter<T>
where
    T::Message: core::fmt::Debug,
{
    fn init(&mut self) -> Result<Response, ConversationError> {
        // Call init first, this normally sets up the filters
        let mut response = <T as MultiKeyListener>::init(self)
            .map_err(|e| ConversationError::Inner(Box::new(e)))?;

        if let Some(user) = self.user {
            response.set_recepient_keys(user, &HashSet::new());
        }

        Ok(response)
    }

    fn on_message(&mut self, message: ConversationMessage) -> Result<Response, ConversationError> {
        match message {
            ConversationMessage::Cleartext(event) => {
                if let Ok(content) = serde_json::from_value(event.content.clone()) {
                    let mut response = <T as MultiKeyListener>::on_message(self, &event, &content)
                        .map_err(|e| ConversationError::Inner(Box::new(e)))?;

                    if let Some(user) = self.user {
                        response.set_recepient_keys(user, &HashSet::new());
                    }

                    Ok(response)
                } else {
                    Ok(Response::default())
                }
            }
            ConversationMessage::Encrypted(event) => {
                if let Some(subkey_proof) = self.subkey_proof.clone() {
                    let tags = vec![Tag::public_key(event.pubkey), Tag::event(event.id)]
                        .into_iter()
                        .collect();
                    Ok(Response::new().reply_to(
                        event.pubkey.into(),
                        Kind::Custom(SUBKEY_PROOF),
                        tags,
                        subkey_proof,
                    ))
                } else {
                    Ok(Response::default())
                }
            }
            ConversationMessage::EndOfStoredEvents => Ok(Response::default()),
        }
    }

    fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => expires_at < SystemTime::now(),
            None => false,
        }
    }
}

impl<Inner: MultiKeyListener> MultiKeyListenerAdapter<Inner> {
    pub fn new(inner: Inner, subkey_proof: Option<SubkeyProof>) -> Self {
        Self {
            user: None,
            subkey_proof,
            expires_at: Inner::VALIDITY_SECONDS
                .map(|seconds| SystemTime::now() + Duration::from_secs(seconds)),
            inner,
        }
    }
}

impl<Inner: MultiKeyListener> Deref for MultiKeyListenerAdapter<Inner> {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
