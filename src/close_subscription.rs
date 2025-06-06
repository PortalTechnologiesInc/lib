use std::{collections::HashSet, ops::Deref};

use nostr::{
    event::{Kind, Tag},
    filter::Filter,
    key::PublicKey,
};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::model::{
        bindings::{self},
        event_kinds::RECURRING_PAYMENT_CANCEL,
        payment::CloseRecurringPaymentContent,
    },
    router::{
        ConversationError, MultiKeyListener, MultiKeyListenerAdapter, Response,
        adapters::{ConversationWithNotification, one_shot::OneShotSender},
    },
};

pub struct CloseRecurringPaymentConversation {
    service_key: PublicKey,
    recipient: PublicKey,
    content: CloseRecurringPaymentContent,
}

impl CloseRecurringPaymentConversation {
    pub fn new(
        service_key: PublicKey,
        recipient: PublicKey,
        content: CloseRecurringPaymentContent,
    ) -> Self {
        Self {
            service_key,
            recipient,
            content,
        }
    }
}

impl OneShotSender for CloseRecurringPaymentConversation {
    type Error = ConversationError;

    fn send(
        state: &mut crate::router::adapters::one_shot::OneShotSenderAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let mut keys = HashSet::new();
        keys.insert(state.service_key);
        keys.insert(state.recipient);

        let tags = keys.iter().map(|k| Tag::public_key(*k.deref())).collect();

        let response = Response::new()
            .reply_to(
                state.recipient,
                Kind::from(RECURRING_PAYMENT_CANCEL),
                tags,
                state.content.clone(),
            )
            .finish();

        Ok(response)
    }
}

// listener

pub struct CloseRecurringPaymentReceiverConversation {
    local_key: PublicKey,
}

impl CloseRecurringPaymentReceiverConversation {
    pub fn new(local_key: PublicKey) -> Self {
        Self { local_key }
    }
}

impl MultiKeyListener for CloseRecurringPaymentReceiverConversation {
    const VALIDITY_SECONDS: u64 = 60 * 5;

    type Error = ConversationError;
    type Message = CloseRecurringPaymentContent;

    fn init(state: &crate::router::MultiKeyListenerAdapter<Self>) -> Result<Response, Self::Error> {
        let mut filter = Filter::new()
            .kinds(vec![Kind::from(RECURRING_PAYMENT_CANCEL)])
            //.pubkey(state.user.ok_or(ConversationError::UserNotSet)?);
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
        Ok(Response::new().notify(message.clone()).finish())
    }
}

impl ConversationWithNotification
    for MultiKeyListenerAdapter<CloseRecurringPaymentReceiverConversation>
{
    type Notification = CloseRecurringPaymentContent;
}
