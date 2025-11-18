use nostr::{
    event::{EventId, Kind, Tag},
    filter::Filter,
    key::PublicKey,
};

use crate::{
    protocol::model::{
        event_kinds::RECURRING_PAYMENT_CANCEL,
        payment::{CloseRecurringPaymentContent, CloseRecurringPaymentResponse},
    },
    router::{
        ConversationError, MultiKeyListener, MultiKeyListenerAdapter, MultiKeySender, Response,
        adapters::ConversationWithNotification,
    },
};

pub struct CloseRecurringPaymentConversation {
    content: CloseRecurringPaymentContent,
}

impl CloseRecurringPaymentConversation {
    pub fn new(content: CloseRecurringPaymentContent) -> Self {
        Self { content }
    }
}

impl MultiKeySender for CloseRecurringPaymentConversation {
    const VALIDITY_SECONDS: Option<u64> = Some(60 * 5);

    type Error = ConversationError;
    type Message = ();

    fn get_filter(
        _state: &crate::router::MultiKeySenderAdapter<Self>,
    ) -> Result<Filter, Self::Error> {
        // Empty filter that will not match any events
        // TODO: we should avoid subscribing to relays for empty filters
        Ok(Filter::new().id(EventId::all_zeros()))
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
                Kind::Custom(RECURRING_PAYMENT_CANCEL),
                tags,
                state.content.clone(),
            ))
        } else {
            Ok(Response::new().subscribe_to_subkey_proofs().reply_all(
                Kind::Custom(RECURRING_PAYMENT_CANCEL),
                tags,
                state.content.clone(),
            ))
        }
    }

    fn on_message(
        _state: &mut crate::router::MultiKeySenderAdapter<Self>,
        _event: &crate::router::CleartextEvent,
        _message: &Self::Message,
    ) -> Result<Response, Self::Error> {
        Ok(Response::default())
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
    const VALIDITY_SECONDS: Option<u64> = None;

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
        let main_key = match &state.subkey_proof {
            Some(subkey_proof) => subkey_proof.main_key.into(),
            None => event.pubkey.into(),
        };

        let res = CloseRecurringPaymentResponse {
            content: message.clone(),
            main_key,
            recipient: event.pubkey.into(),
        };

        // Note: we never call "finish" here, because we want to keep listening for events
        Ok(Response::new().notify(res))
    }
}

impl ConversationWithNotification
    for MultiKeyListenerAdapter<CloseRecurringPaymentReceiverConversation>
{
    type Notification = CloseRecurringPaymentResponse;
}
