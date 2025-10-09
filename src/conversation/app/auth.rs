use std::{collections::HashSet, ops::Deref};

use nostr::{Tag, event::Kind, filter::Filter, key::PublicKey};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::{
        key_handshake::KeyHandshakeUrl,
        model::{
            auth::{
                AuthChallengeContent, AuthResponseContent, AuthResponseStatus, ClientInfo,
                KeyHandshakeContent, SubkeyProof,
            },
            bindings,
            event_kinds::{AUTH_CHALLENGE, AUTH_RESPONSE, KEY_HANDSHAKE},
        },
    },
    router::conversation::{
        ConversationError, ConversationWithNotification, MultiKeyListener, MultiKeyListenerAdapter,
        OneShotSender, response::Response,
    },
};

#[derive(derive_new::new)]
pub struct KeyHandshakeConversation {
    pub url: KeyHandshakeUrl,
    pub relays: Vec<String>,
}

impl ToString for KeyHandshakeConversation {
    fn to_string(&self) -> String {
        format!(
            "KeyHandshakeConversation{{url: {:?}, relays: {:?}}}",
            self.url, self.relays
        )
    }
}

impl OneShotSender for KeyHandshakeConversation {
    type Error = ConversationError;

    fn send(
        state: &mut crate::router::conversation::OneShotSenderAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let content = KeyHandshakeContent {
            token: state.url.token.clone(),
            client_info: ClientInfo {
                version: env!("CARGO_PKG_VERSION").to_string(),
                name: "Portal".to_string(),
            },
            preferred_relays: state.relays.clone(),
        };

        let tags = state
            .url
            .all_keys()
            .iter()
            .map(|k| Tag::public_key(*k.deref()))
            .collect();
        let response = Response::new()
            .reply_to(
                state.url.send_to(),
                Kind::from(KEY_HANDSHAKE),
                tags,
                content,
            )
            .finish();

        Ok(response)
    }
}

pub struct AuthChallengeListenerConversation {
    local_key: PublicKey,
}

impl ToString for AuthChallengeListenerConversation {
    fn to_string(&self) -> String {
        format!(
            "AuthChallengeListenerConversation{{local_key: {:?}}}",
            self.local_key
        )
    }
}

impl AuthChallengeListenerConversation {
    pub fn new(local_key: PublicKey) -> Self {
        Self { local_key }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(uniffi::Record))]
pub struct AuthChallengeEvent {
    pub service_key: bindings::PublicKey,
    pub recipient: bindings::PublicKey,
    pub challenge: String,
    pub expires_at: u64,
    pub required_permissions: Vec<String>,
    pub event_id: String,
}

impl MultiKeyListener for AuthChallengeListenerConversation {
    const VALIDITY_SECONDS: Option<u64> = None;

    type Error = ConversationError;
    type Message = AuthChallengeContent;

    fn init(
        state: &crate::router::conversation::MultiKeyListenerAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::from(AUTH_CHALLENGE)])
            .pubkey(state.local_key);

        if let Some(subkey_proof) = &state.subkey_proof {
            filter = filter.pubkey(subkey_proof.main_key.into());
        }

        Ok(Response::new().filter(filter))
    }

    fn on_message(
        _state: &mut crate::router::conversation::MultiKeyListenerAdapter<Self>,
        event: &crate::router::conversation::message::CleartextEvent,
        content: &Self::Message,
    ) -> Result<Response, Self::Error> {
        log::debug!(
            "Received auth challenge from {}: {:?}",
            event.pubkey,
            content
        );

        if content.expires_at.as_u64() < nostr::Timestamp::now().as_u64() {
            log::warn!("Ignoring expired auth challenge");
            return Ok(Response::default());
        }

        let service_key = if let Some(subkey_proof) = &content.subkey_proof {
            if let Err(e) = subkey_proof.verify(&event.pubkey) {
                log::warn!("Ignoring request with invalid subkey proof: {}", e);
                return Ok(Response::default());
            }

            subkey_proof.main_key
        } else {
            event.pubkey.into()
        };

        let response = Response::new().notify(AuthChallengeEvent {
            service_key,
            recipient: event.pubkey.into(),
            challenge: content.challenge.clone(),
            expires_at: content.expires_at.as_u64(),
            required_permissions: content.required_permissions.clone(),
            event_id: event.id.to_string(),
        });

        Ok(response)
    }
}

impl ConversationWithNotification for MultiKeyListenerAdapter<AuthChallengeListenerConversation> {
    type Notification = AuthChallengeEvent;
}

pub struct AuthResponseConversation {
    event: AuthChallengeEvent,
    subkey_proof: Option<SubkeyProof>,
    status: AuthResponseStatus,
}

impl AuthResponseConversation {
    pub fn new(
        event: AuthChallengeEvent,
        subkey_proof: Option<SubkeyProof>,
        status: AuthResponseStatus,
    ) -> Self {
        AuthResponseConversation {
            event,
            subkey_proof,
            status,
        }
    }
}

impl ToString for AuthResponseConversation {
    fn to_string(&self) -> String {
        format!(
            "AuthResponseConversation{{event: {:?}, subkey_proof: {:?}, status: {:?}}}",
            self.event, self.subkey_proof, self.status
        )
    }
}

impl OneShotSender for AuthResponseConversation {
    type Error = ConversationError;

    fn send(
        state: &mut crate::router::conversation::OneShotSenderAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let content = AuthResponseContent {
            challenge: state.event.challenge.clone(),
            subkey_proof: state.subkey_proof.clone(),
            status: state.status.clone(),
        };

        let mut keys = HashSet::new();
        keys.insert(state.event.service_key);
        keys.insert(state.event.recipient);

        let tags = keys.iter().map(|k| Tag::public_key(*k.deref())).collect();
        let response = Response::new()
            .reply_to(
                state.event.recipient.into(),
                Kind::from(AUTH_RESPONSE),
                tags,
                content,
            )
            .finish();

        Ok(response)
    }
}
