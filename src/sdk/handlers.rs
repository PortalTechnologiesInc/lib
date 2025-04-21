use std::time::{Duration, SystemTime};

use nostr::{
    Filter,
    event::{Kind, Tag},
    key::PublicKey,
};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::model::{
        auth::{AuthChallengeContent, AuthInitContent, AuthResponseContent, SubkeyProof}, event_kinds::*, Timestamp
    },
    router::{adapters::ConversationWithNotification, ConversationError, MultiKeyListener, MultiKeyListenerAdapter, MultiKeySender, MultiKeySenderAdapter, Response},
    utils::random_string,
};

pub struct AuthInitReceiverConversation {
    local_key: PublicKey,
    token: String,
}

impl AuthInitReceiverConversation {
    pub fn new(local_key: PublicKey, token: String) -> Self {
        Self { local_key, token }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthInitEvent {
    pub main_key: PublicKey,
}

impl MultiKeyListener for AuthInitReceiverConversation {
    const VALIDITY_SECONDS: u64 = 60 * 5;

    type Error = ConversationError;
    type Message = AuthInitContent;

    fn init(state: &crate::router::MultiKeyListenerAdapter<Self>) -> Result<Response, Self::Error> {
        let mut filter =             Filter::new()
                .kinds(vec![Kind::from(AUTH_INIT)])
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
            Ok(Response::new()
                .notify(AuthInitEvent {
                    main_key: event.pubkey,
                })
                .finish())
        } else {
            Ok(Response::default())
        }
    }
}

impl ConversationWithNotification for MultiKeyListenerAdapter<AuthInitReceiverConversation> {
    type Notification = AuthInitEvent;
}

pub struct AuthChallengeSenderConversation {
    local_key: PublicKey,
    subkey_proof: Option<SubkeyProof>,

    challenge: String,
}

impl AuthChallengeSenderConversation {
    pub fn new(
        local_key: PublicKey,
        subkey_proof: Option<SubkeyProof>,
    ) -> Self {
        Self {
            local_key,
            subkey_proof,
            challenge: random_string(32),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponseEvent {
    pub user_key: PublicKey,
    pub recipient: PublicKey,
    pub challenge: String,
    pub granted_permissions: Vec<String>,
    pub session_token: String,
}

impl MultiKeySender for AuthChallengeSenderConversation {
    const VALIDITY_SECONDS: u64 = 60 * 5;

    type Error = ConversationError;
    type Message = AuthResponseContent;

    fn get_filter(state: &crate::router::MultiKeySenderAdapter<Self>) -> Result<Filter, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::from(AUTH_RESPONSE)])
            .authors(
                state
                    .subkeys
                    .iter()
                    .chain([&state.user])
                    .cloned(),
            )
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

        log::info!("Notifying auth response event");

        Ok(Response::new()
            .notify(AuthResponseEvent {
                user_key,
                recipient: event.pubkey.into(),
                challenge: message.challenge.clone(),
                granted_permissions: message.granted_permissions.clone(),
                session_token: message.session_token.clone(),
            })
            .finish())
    }
}

impl ConversationWithNotification for MultiKeySenderAdapter<AuthChallengeSenderConversation> {
    type Notification = AuthResponseEvent;
}
