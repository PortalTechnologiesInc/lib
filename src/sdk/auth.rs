use nostr::{
    Filter,
    event::{Kind, Tag},
    key::PublicKey,
};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::model::{
        Timestamp,
        auth::{
            AuthChallengeContent, AuthResponseContent, AuthResponseEvent, AuthResponseStatus,
            KeyHandshakeContent, SubkeyProof,
        },
        event_kinds::*,
    },
    router::{
        ConversationError, MultiKeyListener, MultiKeyListenerAdapter, MultiKeySender,
        MultiKeySenderAdapter, Response, adapters::ConversationWithNotification,
    },
    utils::random_string,
};

#[derive(derive_new::new)]
pub struct KeyHandshakeReceiverConversation {
    local_key: PublicKey,
    token: String,
    persist: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyHandshakeEvent {
    pub main_key: PublicKey,
}

impl MultiKeyListener for KeyHandshakeReceiverConversation {
    const VALIDITY_SECONDS: Option<u64> = Some(5 * 60);

    type Error = ConversationError;
    type Message = KeyHandshakeContent;

    fn init(state: &crate::router::MultiKeyListenerAdapter<Self>) -> Result<Response, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::from(KEY_HANDSHAKE)])
            .since(nostr::types::Timestamp::now())
            .pubkey(state.local_key);

        if let Some(subkey_proof) = &state.subkey_proof {
            filter = filter.pubkey(subkey_proof.main_key.into());
        }

        Ok(Response::new().filter(filter))
    }

    fn on_message(
        state: &mut crate::router::MultiKeyListenerAdapter<Self>,
        event: &crate::router::CleartextEvent,
        message: &Self::Message,
    ) -> Result<Response, Self::Error> {
        if message.token == state.token {
            let mut response = Response::new()
                .notify(KeyHandshakeEvent {
                    main_key: event.pubkey,
                });
            
            if !state.inner.persist {
                response = response.finish();
            }

            Ok(response)
        } else {
            Ok(Response::default())
        }
    }
}

impl ConversationWithNotification for MultiKeyListenerAdapter<KeyHandshakeReceiverConversation> {
    type Notification = KeyHandshakeEvent;
}

pub struct AuthChallengeSenderConversation {
    local_key: PublicKey,
    subkey_proof: Option<SubkeyProof>,

    challenge: String,
}

impl AuthChallengeSenderConversation {
    pub fn new(local_key: PublicKey, subkey_proof: Option<SubkeyProof>) -> Self {
        Self {
            local_key,
            subkey_proof,
            challenge: random_string(32),
        }
    }
}

impl MultiKeySender for AuthChallengeSenderConversation {
    const VALIDITY_SECONDS: Option<u64> = Some(60 * 5);

    type Error = ConversationError;
    type Message = AuthResponseContent;

    fn get_filter(
        state: &crate::router::MultiKeySenderAdapter<Self>,
    ) -> Result<Filter, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::from(AUTH_RESPONSE)])
            .authors(state.subkeys.iter().chain([&state.user]).cloned())
            .pubkey(state.local_key);

        if let Some(subkey_proof) = &state.subkey_proof {
            filter = filter.pubkey(subkey_proof.main_key.into());
        }

        Ok(filter)
    }

    fn build_initial_message(
        state: &mut crate::router::MultiKeySenderAdapter<Self>,
        new_key: Option<PublicKey>,
    ) -> Result<Response, Self::Error> {
        let content = AuthChallengeContent {
            challenge: state.challenge.clone(),
            expires_at: Timestamp::now_plus_seconds(60 * 5), // TODO: should take it from state.expires_at
            required_permissions: vec![],
            subkey_proof: state.subkey_proof.clone(),
        };

        let tags = state
            .subkeys
            .iter()
            .chain([&state.user])
            .map(|k| Tag::public_key(*k))
            .collect();

        if let Some(new_key) = new_key {
            Ok(Response::new().subscribe_to_subkey_proofs().reply_to(
                new_key,
                Kind::from(AUTH_CHALLENGE),
                tags,
                content,
            ))
        } else {
            Ok(Response::new().subscribe_to_subkey_proofs().reply_all(
                Kind::from(AUTH_CHALLENGE),
                tags,
                content,
            ))
        }
    }

    fn on_message(
        state: &mut crate::router::MultiKeySenderAdapter<Self>,
        event: &crate::router::CleartextEvent,
        message: &Self::Message,
    ) -> Result<Response, Self::Error> {
        if message.challenge != state.challenge {
            log::warn!(
                "Ignoring response with invalid challenge: {}",
                message.challenge
            );
            return Ok(Response::default());
        }

        let user_key = if let Some(subkey_proof) = &message.subkey_proof {
            if let Err(e) = subkey_proof.verify(&event.pubkey) {
                log::warn!("Ignoring response with invalid subkey proof: {}", e);
                return Ok(Response::default());
            }

            let main_key = subkey_proof.main_key.into();
            state.user = main_key;
            state.subkeys.insert(event.pubkey);

            main_key
        } else {
            event.pubkey
        };

        Ok(Response::new()
            .notify(AuthResponseEvent {
                user_key: user_key.into(),
                recipient: event.pubkey.into(),
                challenge: message.challenge.clone(),
                status: message.status.clone(),
            })
            .finish())
    }
}

impl ConversationWithNotification for MultiKeySenderAdapter<AuthChallengeSenderConversation> {
    type Notification = AuthResponseEvent;
}
