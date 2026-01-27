use std::{collections::HashSet, ops::Deref};

use nostr::{Tag, event::Kind, filter::Filter, key::PublicKey};

use derive_new::new;

use crate::{
    protocol::model::{
        auth::SubkeyProof,
        event_kinds::{INVOICE_REQUEST, INVOICE_RESPONSE},
        payment::{InvoiceRequestContent, InvoiceRequestContentWithKey, InvoiceResponse},
    },
    router::{
        ConversationError, MultiKeyListener, MultiKeyListenerAdapter, MultiKeySender,
        MultiKeySenderAdapter, Response,
        adapters::{ConversationWithNotification, one_shot::OneShotSender},
    },
};

#[derive(new)]
pub struct InvoiceRequestConversation {
    local_key: PublicKey,
    subkey_proof: Option<SubkeyProof>,

    content: InvoiceRequestContent,
}

impl MultiKeySender for InvoiceRequestConversation {
    const VALIDITY_SECONDS: Option<u64> = Some(60 * 5);

    type Error = ConversationError;
    type Message = InvoiceResponse;

    fn get_filter(
        state: &crate::router::MultiKeySenderAdapter<Self>,
    ) -> Result<Filter, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::Custom(INVOICE_RESPONSE)])
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
        let tags = state
            .subkeys
            .iter()
            .chain([&state.user])
            .map(|k| Tag::public_key(*k))
            .collect();

        if let Some(new_key) = new_key {
            Ok(Response::new().subscribe_to_subkey_proofs().reply_to(
                new_key,
                Kind::Custom(INVOICE_REQUEST),
                tags,
                state.content.clone(),
            ))
        } else {
            Ok(Response::new().subscribe_to_subkey_proofs().reply_all(
                Kind::Custom(INVOICE_REQUEST),
                tags,
                state.content.clone(),
            ))
        }
    }

    fn on_message(
        state: &mut crate::router::MultiKeySenderAdapter<Self>,
        _event: &crate::router::CleartextEvent,
        message: &Self::Message,
    ) -> Result<Response, Self::Error> {
        if message.request.inner.request_id == state.content.request_id {
            Ok(Response::new().notify(message.clone()).finish())
        } else {
            Ok(Response::default())
        }
    }
}

impl ConversationWithNotification for MultiKeySenderAdapter<InvoiceRequestConversation> {
    type Notification = InvoiceResponse;
}

#[derive(new)]
pub struct InvoiceReceiverConversation {
    local_key: PublicKey,
}

impl MultiKeyListener for InvoiceReceiverConversation {
    const VALIDITY_SECONDS: Option<u64> = Some(60 * 5);

    type Error = ConversationError;
    type Message = InvoiceRequestContent;

    fn init(state: &crate::router::MultiKeyListenerAdapter<Self>) -> Result<Response, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::Custom(INVOICE_REQUEST)])
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
        let sender_key = if let Some(subkey_proof) = state.subkey_proof.clone() {
            if let Err(_) = subkey_proof.verify(&event.pubkey) {
                return Ok(Response::default());
            }

            subkey_proof.main_key
        } else {
            event.pubkey.into()
        };

        let res = InvoiceRequestContentWithKey {
            inner: message.clone(),
            main_key: sender_key,
            recipient: event.pubkey.into(),
        };

        Ok(Response::new().notify(res))
    }
}

impl ConversationWithNotification for MultiKeyListenerAdapter<InvoiceReceiverConversation> {
    type Notification = InvoiceRequestContentWithKey;
}

// Send invoice to sender
#[derive(new)]
pub struct InvoiceSenderConversation {
    content: InvoiceResponse,
}

impl OneShotSender for InvoiceSenderConversation {
    type Error = ConversationError;

    fn send(
        state: &mut crate::router::adapters::one_shot::OneShotSenderAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let mut keys = HashSet::new();
        keys.insert(state.content.request.recipient);
        keys.insert(state.content.request.main_key);

        let tags = keys.iter().map(|k| Tag::public_key(*k.deref())).collect();
        let response = Response::new()
            .reply_to(
                state.content.request.recipient.into(),
                Kind::from(INVOICE_RESPONSE),
                tags,
                state.content.clone(),
            )
            .finish();

        Ok(response)
    }
}
