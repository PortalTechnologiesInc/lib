use std::sync::Arc;

use portal::{
    nostr::key::PublicKey,
    nostr_relay_pool::{RelayOptions, RelayPool},
    profile::{FetchProfileInfoConversation, Profile, SetProfileConversation},
    protocol::{
        auth_init::AuthInitUrl, model::payment::{
            CloseRecurringPaymentContent, PaymentResponseContent, RecurringPaymentRequestContent, RecurringPaymentResponseContent, SinglePaymentRequestContent
        }, LocalKeypair
    },
    router::{
        adapters::one_shot::OneShotSenderAdapter, ConversationError, MessageRouter, MultiKeyListenerAdapter, MultiKeySenderAdapter, NotificationStream
    },
    sdk::{
        auth::{
            AuthChallengeSenderConversation, AuthInitEvent, AuthInitReceiverConversation,
            AuthResponseEvent,
        },
        payments::{
            CloseRecurringPaymentReceiverConversation, RecurringPaymentRequestSenderConversation, SinglePaymentRequestSenderConversation
        },
    },
};
use tokio::task::JoinHandle;
use uuid::Uuid;

pub struct PortalSDK {
    router: Arc<MessageRouter<RelayPool>>,
    _listener: JoinHandle<Result<(), ConversationError>>,
}

impl PortalSDK {
    pub async fn new(keypair: LocalKeypair, relays: Vec<String>) -> Result<Self, PortalSDKError> {
        let relay_pool = RelayPool::new();
        for relay in relays {
            relay_pool.add_relay(relay, RelayOptions::default()).await?;
        }
        relay_pool.connect().await;

        let router = Arc::new(MessageRouter::new(relay_pool, keypair.clone()));

        let _router = Arc::clone(&router);
        let _listener = tokio::spawn(async move { _router.listen().await });

        Ok(Self { router, _listener })
    }

    pub async fn new_auth_init_url(
        &self,
    ) -> Result<(AuthInitUrl, NotificationStream<AuthInitEvent>), PortalSDKError> {
        let token = Uuid::new_v4().to_string();

        let inner =
            AuthInitReceiverConversation::new(self.router.keypair().public_key(), token.clone());
        let event = self
            .router
            .add_and_subscribe(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            ))
            .await?;

        let relays = self
            .router
            .channel()
            .relays()
            .await
            .keys()
            .map(|r| r.to_string())
            .collect::<Vec<_>>();

        let (main_key, subkey) = if let Some(subkey_proof) = self.router.keypair().subkey_proof() {
            (
                subkey_proof.main_key.into(),
                Some(self.router.keypair().public_key()),
            )
        } else {
            (self.router.keypair().public_key(), None)
        };

        let url = AuthInitUrl {
            main_key: main_key.into(),
            relays,
            token: token.clone(),
            subkey: subkey.map(|k| k.into()),
        };

        Ok((url, event))
    }

    pub async fn authenticate_key(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
    ) -> Result<AuthResponseEvent, PortalSDKError> {
        let conv = AuthChallengeSenderConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
        );

        let mut event = self
            .router
            .add_and_subscribe(MultiKeySenderAdapter::new_with_user(
                main_key, subkeys, conv,
            ))
            .await?;
        Ok(event.next().await.ok_or(PortalSDKError::Timeout)??)
    }

    pub async fn request_recurring_payment(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        payment_request: RecurringPaymentRequestContent,
    ) -> Result<RecurringPaymentResponseContent, PortalSDKError> {
        let conv = RecurringPaymentRequestSenderConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
            payment_request,
        );

        let mut event = self
            .router
            .add_and_subscribe(MultiKeySenderAdapter::new_with_user(
                main_key, subkeys, conv,
            ))
            .await?;
        Ok(event.next().await.ok_or(PortalSDKError::Timeout)??)
    }

    pub async fn request_single_payment(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        payment_request: SinglePaymentRequestContent,
    ) -> Result<PaymentResponseContent, PortalSDKError> {
        let conv = SinglePaymentRequestSenderConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
            payment_request,
        );

        let mut event = self
            .router
            .add_and_subscribe(MultiKeySenderAdapter::new_with_user(
                main_key, subkeys, conv,
            ))
            .await?;
        Ok(event.next().await.ok_or(PortalSDKError::Timeout)??)
    }

    pub async fn fetch_profile(&self, main_key: PublicKey) -> Result<Option<Profile>, PortalSDKError> {
        let conv = FetchProfileInfoConversation::new(main_key);
        let mut event = self.router.add_and_subscribe(conv).await?;
        let profile = event.next().await.ok_or(PortalSDKError::Timeout)??;

        if let Some(mut profile) = profile {
            if let Some(nip05) = &profile.nip05 {
                let verified = portal::nostr::nips::nip05::verify(&main_key, &nip05, None).await;
                if verified.ok() != Some(true) {
                    profile.nip05 = None;
                }
            }
            Ok(Some(profile))
        } else {
            Ok(None)
        }
    }

    pub async fn set_profile(&self, profile: Profile) -> Result<(), PortalSDKError> {
        if self.router.keypair().subkey_proof().is_some() {
            return Err(PortalSDKError::MasterKeyRequired);
        }

        let conv = SetProfileConversation::new(profile);
        let _ = self.router.add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
            self.router.keypair().public_key().into(),
            vec![],
            conv,
        ))).await?;

        Ok(())
    }

    pub async fn listen_closed_subscriptions(
        &self,
    ) -> Result<NotificationStream<CloseRecurringPaymentContent>, PortalSDKError> {
        let inner = CloseRecurringPaymentReceiverConversation::new(self.router.keypair().public_key());
        let event = self
            .router
            .add_and_subscribe(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            ))
            .await?;
        Ok(event)
    }

}

#[derive(Debug, thiserror::Error)]
pub enum PortalSDKError {
    #[error("Relay pool error: {0}")]
    RelayPool(#[from] portal::nostr_relay_pool::pool::Error),

    #[error("Conversation error: {0}")]
    Conversation(#[from] ConversationError),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Master key required")]
    MasterKeyRequired,
}
