use std::collections::HashSet;

use nostr::{
    event::{Kind, Tag},
    filter::Filter,
    key::PublicKey,
    nips::nip46::NostrConnectMessage,
};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::model::event_kinds::SIGNING_REQUEST,
    router::{ConversationError, MultiKeyListener, Response, adapters::one_shot::OneShotSender},
};

pub struct SigningRequestListenerConversation {
    local_key: PublicKey,
}

impl SigningRequestListenerConversation {
    pub fn new(local_key: PublicKey) -> Self {
        Self { local_key }
    }
}

pub struct SigningResponseSenderConversation {
    user_pubkey: PublicKey,
    id: String,
    result: String,
}

impl SigningResponseSenderConversation {
    pub fn new(user_pubkey: PublicKey, id: String, result: String) -> Self {
        Self {
            user_pubkey,
            id,
            result,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningRequest {
    pub user_pubkey: PublicKey,
    pub event_id: String,
    pub message: NostrConnectMessage,
}

impl MultiKeyListener for SigningRequestListenerConversation {
    const VALIDITY_SECONDS: Option<u64> = None;

    type Error = ConversationError;
    type Message = NostrConnectMessage;

    fn init(state: &crate::router::MultiKeyListenerAdapter<Self>) -> Result<Response, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::from(SIGNING_REQUEST)])
            .pubkey(state.local_key);

        if let Some(subkey_proof) = &state.subkey_proof {
            filter = filter.pubkey(subkey_proof.main_key.into());
        }

        Ok(Response::new().filter(filter))
    }

    fn on_message(
        _state: &mut crate::router::MultiKeyListenerAdapter<Self>,
        event: &crate::router::CleartextEvent,
        message: &Self::Message,
    ) -> Result<crate::router::Response, Self::Error> {
        log::debug!(
            "Received signing request from {}: {:?}",
            event.pubkey,
            message
        );

        let listener_response = SigningRequest {
            user_pubkey: event.pubkey,
            event_id: event.id.to_string(),
            message: message.clone(),
        };

        Ok(Response::new().notify(listener_response))
    }
}

impl OneShotSender for SigningResponseSenderConversation {
    type Error = ConversationError;

    fn send(
        state: &mut crate::router::adapters::one_shot::OneShotSenderAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let mut keys = HashSet::new();
        keys.insert(state.user);

        let tags = keys.iter().map(|k| Tag::public_key(*k)).collect();

        let content = NostrConnectMessage::Response {
            id: state.id.clone(),
            result: Some(state.result.clone()),
            error: None,
        };
        let response = Response::new()
            .reply_to(
                state.user_pubkey,
                Kind::from(SIGNING_REQUEST),
                tags,
                content,
            )
            .finish();

        Ok(response)
    }
}
