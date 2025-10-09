use std::collections::HashSet;

use nostr::{
    event::{Kind, Tag},
    filter::Filter,
    key::PublicKey,
};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::model::{
        Timestamp,
        bindings::{self},
        event_kinds::{
            PAYMENT_REQUEST, PAYMENT_RESPONSE, RECURRING_PAYMENT_REQUEST,
            RECURRING_PAYMENT_RESPONSE,
        },
        payment::{
            PaymentRequestContent, PaymentResponseContent, RecurringPaymentRequestContent,
            RecurringPaymentResponseContent, SinglePaymentRequestContent,
        },
    },
    router::conversation::{
        ConversationError, ConversationWithNotification, MultiKeyListener, MultiKeyListenerAdapter,
        OneShotSender, response::Response,
    },
};

pub struct PaymentRequestListenerConversation {
    local_key: PublicKey,
}

impl ToString for PaymentRequestListenerConversation {
    fn to_string(&self) -> String {
        format!(
            "PaymentRequestListenerConversation{{local_key: {:?}}}",
            self.local_key
        )
    }
}

impl PaymentRequestListenerConversation {
    pub fn new(local_key: PublicKey) -> Self {
        Self { local_key }
    }
}

impl MultiKeyListener for PaymentRequestListenerConversation {
    const VALIDITY_SECONDS: Option<u64> = None;

    type Error = ConversationError;
    type Message = PaymentRequestContent;

    fn init(
        state: &crate::router::conversation::MultiKeyListenerAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![
                Kind::Custom(PAYMENT_REQUEST),
                Kind::Custom(RECURRING_PAYMENT_REQUEST),
            ])
            .pubkey(state.local_key);

        if let Some(subkey_proof) = &state.subkey_proof {
            filter = filter.pubkey(subkey_proof.main_key.into());
        }

        Ok(Response::new().filter(filter))
    }

    fn on_message(
        state: &mut crate::router::conversation::MultiKeyListenerAdapter<Self>,
        event: &crate::router::conversation::message::CleartextEvent,
        content: &Self::Message,
    ) -> Result<Response, Self::Error> {
        log::debug!(
            "Received payment request from {}: {:?}",
            event.pubkey,
            content
        );

        if content.expires_at().as_u64() < nostr::Timestamp::now().as_u64() {
            log::warn!("Ignoring expired auth challenge");
            return Ok(Response::default());
        }

        let service_key = if let Some(subkey_proof) = state.subkey_proof.clone() {
            if let Err(e) = subkey_proof.verify(&event.pubkey) {
                log::warn!("Ignoring request with invalid subkey proof: {}", e);
                return Ok(Response::default());
            }

            subkey_proof.main_key
        } else {
            event.pubkey.into()
        };

        let response = Response::new().notify(PaymentRequestEvent {
            service_key,
            recipient: event.pubkey.into(),
            expires_at: content.expires_at(),
            content: content.clone(),
            event_id: event.id.to_string(),
        });

        Ok(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(uniffi::Record))]
pub struct PaymentRequestEvent {
    pub service_key: bindings::PublicKey,
    pub recipient: bindings::PublicKey,
    pub expires_at: Timestamp,
    pub content: PaymentRequestContent,
    pub event_id: String,
}

impl ConversationWithNotification for MultiKeyListenerAdapter<PaymentRequestListenerConversation> {
    type Notification = PaymentRequestEvent;
}

pub struct PaymentStatusSenderConversation {
    service_key: PublicKey,
    recipient: PublicKey,
    response: PaymentResponseContent,
}

impl ToString for PaymentStatusSenderConversation {
    fn to_string(&self) -> String {
        format!(
            "PaymentStatusSenderConversation{{service_key: {:?}, recipient: {:?}, response: {:?}}}",
            self.service_key, self.recipient, self.response
        )
    }
}

impl PaymentStatusSenderConversation {
    pub fn new(
        service_key: PublicKey,
        recipient: PublicKey,
        response: PaymentResponseContent,
    ) -> Self {
        Self {
            service_key,
            recipient,
            response,
        }
    }
}

impl OneShotSender for PaymentStatusSenderConversation {
    type Error = ConversationError;

    fn send(
        state: &mut crate::router::conversation::OneShotSenderAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let mut keys = HashSet::new();
        keys.insert(state.service_key);
        keys.insert(state.recipient);

        let tags = keys.iter().map(|k| Tag::public_key(*k)).collect();
        let response = Response::new()
            .reply_to(
                state.recipient,
                Kind::from(PAYMENT_RESPONSE),
                tags,
                state.response.clone(),
            )
            .finish();

        Ok(response)
    }
}

pub struct RecurringPaymentStatusSenderConversation {
    service_key: PublicKey,
    recipient: PublicKey,
    response: RecurringPaymentResponseContent,
}

impl ToString for RecurringPaymentStatusSenderConversation {
    fn to_string(&self) -> String {
        format!(
            "RecurringPaymentStatusSenderConversation{{service_key: {:?}, recipient: {:?}, response: {:?}}}",
            self.service_key, self.recipient, self.response
        )
    }
}

impl RecurringPaymentStatusSenderConversation {
    pub fn new(
        service_key: PublicKey,
        recipient: PublicKey,
        response: RecurringPaymentResponseContent,
    ) -> Self {
        Self {
            service_key,
            recipient,
            response,
        }
    }
}

impl OneShotSender for RecurringPaymentStatusSenderConversation {
    type Error = ConversationError;

    fn send(
        state: &mut crate::router::conversation::OneShotSenderAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let mut keys = HashSet::new();
        keys.insert(state.service_key);
        keys.insert(state.recipient);

        let tags = keys.iter().map(|k| Tag::public_key(*k)).collect();

        let response = Response::new()
            .reply_to(
                state.recipient,
                Kind::from(RECURRING_PAYMENT_RESPONSE),
                tags,
                state.response.clone(),
            )
            .finish();

        Ok(response)
    }
}
