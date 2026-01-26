use std::{collections::HashSet, ops::Deref};

use nostr::key::PublicKey;

use crate::router::{Conversation, ConversationError, ConversationMessage, Response};

pub trait OneShotSender: Sized + Send + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn send(_state: &mut OneShotSenderAdapter<Self>) -> Result<Response, Self::Error>;
}

/// A conversation wrapper that sends a message and closes immediately
pub struct OneShotSenderAdapter<Inner> {
    pub user: PublicKey,
    pub subkeys: HashSet<PublicKey>,
    pub inner: Inner,
}

impl<T: OneShotSender> Conversation for OneShotSenderAdapter<T> {
    fn init(&mut self) -> Result<Response, ConversationError> {
        // Call init first, this normally sets up the filters
        let mut response =
            <T as OneShotSender>::send(self).map_err(|e| ConversationError::Inner(Box::new(e)))?;

        // Force the conversation to close immediately
        response = response.finish();

        response.set_recepient_keys(self.user, &self.subkeys);

        Ok(response)
    }

    fn on_message(&mut self, _message: ConversationMessage) -> Result<Response, ConversationError> {
        Ok(Response::default())
    }

    fn is_expired(&self) -> bool {
        false
    }
}

impl<Inner: OneShotSender> OneShotSenderAdapter<Inner> {
    pub fn new_with_user(user: PublicKey, subkeys: Vec<PublicKey>, inner: Inner) -> Self {
        Self {
            user,
            subkeys: subkeys.into_iter().collect(),
            inner,
        }
    }
}

impl<Inner: OneShotSender> Deref for OneShotSenderAdapter<Inner> {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
