use std::{collections::HashSet, ops::Deref};

use nostr::{Tag, event::Kind, filter::Filter, key::PublicKey};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::{
        auth_init::AuthInitUrl,
        model::{
            auth::{
                AuthChallengeContent, AuthInitContent, AuthResponseContent, ClientInfo, SubkeyProof,
            },
            bindings,
            event_kinds::{AUTH_CHALLENGE, AUTH_INIT, AUTH_RESPONSE},
        },
    },
    router::{
        ConversationError, MultiKeyListener, MultiKeyListenerAdapter, Response,
        adapters::{ConversationWithNotification, one_shot::OneShotSender},
    },
};

pub struct AuthInitConversation {
    pub url: AuthInitUrl,
    pub relays: Vec<String>,
}

impl OneShotSender for AuthInitConversation {
    type Error = ConversationError;

    fn send(
        state: &mut crate::router::adapters::one_shot::OneShotSenderAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let content = AuthInitContent {
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
            .reply_to(state.url.send_to(), Kind::from(AUTH_INIT), tags, content)
            .finish();

        Ok(response)
    }
}

pub struct AuthChallengeListenerConversation {
    local_key: PublicKey,
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
    const VALIDITY_SECONDS: u64 = 60 * 5;

    type Error = ConversationError;
    type Message = AuthChallengeContent;

    fn init(state: &crate::router::MultiKeyListenerAdapter<Self>) -> Result<Response, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::from(AUTH_CHALLENGE)])
            .pubkey(state.local_key);

        if let Some(subkey_proof) = &state.subkey_proof {
            filter = filter.pubkey(subkey_proof.main_key.into());
        }

        Ok(Response::new().filter(filter))
    }

    fn on_message(
        _state: &mut crate::router::MultiKeyListenerAdapter<Self>,
        event: &crate::router::CleartextEvent,
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
    granted_permissions: Vec<String>,
    subkey_proof: Option<SubkeyProof>,
}

impl AuthResponseConversation {
    pub fn new(
        event: AuthChallengeEvent,
        granted_permissions: Vec<String>,
        subkey_proof: Option<SubkeyProof>,
    ) -> Self {
        Self {
            event,
            granted_permissions,
            subkey_proof,
        }
    }
}

impl OneShotSender for AuthResponseConversation {
    type Error = ConversationError;

    fn send(
        state: &mut crate::router::adapters::one_shot::OneShotSenderAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let content = AuthResponseContent {
            challenge: state.event.challenge.clone(),
            granted_permissions: state.granted_permissions.clone(),
            session_token: "randomlygeneratedtoken".to_string(),
            subkey_proof: state.subkey_proof.clone(),
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
