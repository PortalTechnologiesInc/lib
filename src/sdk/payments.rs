use nostr::{
    Filter,
    event::{Kind, Tag},
    key::PublicKey,
};
use serde::{Deserialize, Serialize};
use crate::{
    app::payments::PaymentRequestContent, protocol::model::{
        auth::SubkeyProof, event_kinds::*, payment::{PaymentStatusContent, PaymentResponseContent, RecurringPaymentStatusContent, RecurringPaymentRequestContent, SinglePaymentRequestContent}
    }, router::{
        adapters::ConversationWithNotification, ConversationError, MultiKeySender, MultiKeySenderAdapter, Response
    }
};

pub struct RecurringPaymentRequestSenderConversation {
    local_key: PublicKey,
    subkey_proof: Option<SubkeyProof>,

    payment_request: RecurringPaymentRequestContent,
}

impl RecurringPaymentRequestSenderConversation {
    pub fn new(local_key: PublicKey, subkey_proof: Option<SubkeyProof>, payment_request: RecurringPaymentRequestContent) -> Self {
        Self {
            local_key,
            subkey_proof,
            payment_request,
        }
    }
}

impl MultiKeySender for RecurringPaymentRequestSenderConversation {
    const VALIDITY_SECONDS: u64 = 60 * 5;

    type Error = ConversationError;
    type Message = RecurringPaymentStatusContent;

    fn get_filter(
        state: &crate::router::MultiKeySenderAdapter<Self>,
    ) -> Result<Filter, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::Custom(RECURRING_PAYMENT_RESPONSE)])
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
                Kind::Custom(RECURRING_PAYMENT_REQUEST),
                tags,
                PaymentRequestContent::Recurring(state.payment_request.clone()),
            ))
        } else {
            Ok(Response::new().subscribe_to_subkey_proofs().reply_all(
                Kind::Custom(RECURRING_PAYMENT_REQUEST),
                tags,
                PaymentRequestContent::Recurring(state.payment_request.clone()),
            ))
        }
    }

    fn on_message(
        _state: &mut crate::router::MultiKeySenderAdapter<Self>,
        _event: &crate::router::CleartextEvent,
        message: &Self::Message,
    ) -> Result<Response, Self::Error> {
        log::info!("Notifying payment response event");

        Ok(Response::new()
            .notify(message.clone())
            .finish())
    }
}

impl ConversationWithNotification for MultiKeySenderAdapter<RecurringPaymentRequestSenderConversation> {
    type Notification = RecurringPaymentStatusContent;
}

pub struct SinglePaymentRequestSenderConversation {
    local_key: PublicKey,
    subkey_proof: Option<SubkeyProof>,

    payment_request: SinglePaymentRequestContent,
}

impl SinglePaymentRequestSenderConversation {
    pub fn new(local_key: PublicKey, subkey_proof: Option<SubkeyProof>, payment_request: SinglePaymentRequestContent) -> Self {
        Self {
            local_key,
            subkey_proof,
            payment_request,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SinglePaymentMessage {
    Response(PaymentResponseContent),
    Confirmation(PaymentStatusContent),
}

impl MultiKeySender for SinglePaymentRequestSenderConversation {
    const VALIDITY_SECONDS: u64 = 60 * 5;

    type Error = ConversationError;
    type Message = SinglePaymentMessage;

    fn get_filter(
        state: &crate::router::MultiKeySenderAdapter<Self>,
    ) -> Result<Filter, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::Custom(PAYMENT_RESPONSE), Kind::Custom(PAYMENT_CONFIRMATION)])
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
                Kind::Custom(PAYMENT_REQUEST),
                tags,
                PaymentRequestContent::Single(state.payment_request.clone()),
            ))
        } else {
            Ok(Response::new().subscribe_to_subkey_proofs().reply_all(
                Kind::Custom(PAYMENT_REQUEST),
                tags,
                PaymentRequestContent::Single(state.payment_request.clone()),
            ))
        }
    }

    fn on_message(
        _state: &mut crate::router::MultiKeySenderAdapter<Self>,
        _event: &crate::router::CleartextEvent,
        message: &Self::Message,
    ) -> Result<Response, Self::Error> {
        log::info!("Notifying payment response event");

        Ok(Response::new()
            .notify(message.clone())
            .finish())
    }
}

impl ConversationWithNotification for MultiKeySenderAdapter<SinglePaymentRequestSenderConversation> {
    type Notification = SinglePaymentMessage;
}