use crate::{
    app::payments::PaymentRequestContent,
    protocol::model::{
        auth::SubkeyProof,
        event_kinds::*,
        payment::{
            PaymentResponseContent, RecurringPaymentRequestContent,
            RecurringPaymentResponseContent, SinglePaymentRequestContent,
        },
    },
    router::{
        ConversationError, MultiKeySender, MultiKeySenderAdapter, Response,
        adapters::ConversationWithNotification,
    },
};
use nostr::{
    Filter,
    event::{Kind, Tag},
    key::PublicKey,
};

pub struct RecurringPaymentRequestSenderConversation {
    local_key: PublicKey,
    subkey_proof: Option<SubkeyProof>,

    payment_request: RecurringPaymentRequestContent,
}

impl RecurringPaymentRequestSenderConversation {
    pub fn new(
        local_key: PublicKey,
        subkey_proof: Option<SubkeyProof>,
        payment_request: RecurringPaymentRequestContent,
    ) -> Self {
        Self {
            local_key,
            subkey_proof,
            payment_request,
        }
    }
}

impl MultiKeySender for RecurringPaymentRequestSenderConversation {
    const VALIDITY_SECONDS: Option<u64> = Some(60 * 5);

    type Error = ConversationError;
    type Message = RecurringPaymentResponseContent;

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
        state: &mut crate::router::MultiKeySenderAdapter<Self>,
        _event: &crate::router::CleartextEvent,
        message: &Self::Message,
    ) -> Result<Response, Self::Error> {
        log::info!("Notifying payment response event");

        if message.request_id == state.payment_request.request_id {
            Ok(Response::new().notify(message.clone()).finish())
        } else {
            Ok(Response::default())
        }
    }
}

impl ConversationWithNotification
    for MultiKeySenderAdapter<RecurringPaymentRequestSenderConversation>
{
    type Notification = RecurringPaymentResponseContent;
}

pub struct SinglePaymentRequestSenderConversation {
    local_key: PublicKey,
    subkey_proof: Option<SubkeyProof>,

    payment_request: SinglePaymentRequestContent,
}

impl SinglePaymentRequestSenderConversation {
    pub fn new(
        local_key: PublicKey,
        subkey_proof: Option<SubkeyProof>,
        payment_request: SinglePaymentRequestContent,
    ) -> Self {
        Self {
            local_key,
            subkey_proof,
            payment_request,
        }
    }
}

impl MultiKeySender for SinglePaymentRequestSenderConversation {
    const VALIDITY_SECONDS: Option<u64> = Some(60 * 5);

    type Error = ConversationError;
    type Message = PaymentResponseContent;

    fn get_filter(
        state: &crate::router::MultiKeySenderAdapter<Self>,
    ) -> Result<Filter, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![
                Kind::Custom(PAYMENT_RESPONSE),
                Kind::Custom(PAYMENT_CONFIRMATION),
            ])
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
        state: &mut crate::router::MultiKeySenderAdapter<Self>,
        _event: &crate::router::CleartextEvent,
        message: &Self::Message,
    ) -> Result<Response, Self::Error> {
        log::info!("Notifying payment response event");

        if message.request_id == state.payment_request.request_id {
            Ok(Response::new().notify(message.clone()).finish())
        } else {
            Ok(Response::default())
        }
    }
}

impl ConversationWithNotification
    for MultiKeySenderAdapter<SinglePaymentRequestSenderConversation>
{
    type Notification = PaymentResponseContent;
}
