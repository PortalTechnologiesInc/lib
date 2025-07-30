use std::sync::Arc;

use chrono::Duration;
use portal::{
    cashu::{
        CashuDirectSenderConversation, CashuRequestReceiverConversation,
        CashuRequestSenderConversation, CashuResponseSenderConversation,
    },
    close_subscription::{
        CloseRecurringPaymentConversation, CloseRecurringPaymentReceiverConversation,
    },
    invoice::InvoiceRequestConversation,
    nostr::key::PublicKey,
    nostr_relay_pool::{RelayOptions, RelayPool},
    profile::{FetchProfileInfoConversation, Profile, SetProfileConversation},
    protocol::{
        LocalKeypair,
        key_handshake::KeyHandshakeUrl,
        model::auth::AuthResponseEvent,
        model::payment::{
            CashuDirectContent, CashuRequestContent, CashuRequestContentWithKey,
            CashuResponseContent, CloseRecurringPaymentContent, CloseRecurringPaymentResponse,
            InvoiceRequestContent, InvoiceRequestContentWithKey, InvoiceResponse,
            PaymentResponseContent, RecurringPaymentRequestContent,
            RecurringPaymentResponseContent, SinglePaymentRequestContent,
        },
    },
    router::{
        ConversationError, MessageRouter, MessageRouterActorError, MultiKeyListenerAdapter,
        MultiKeySenderAdapter, NotificationStream, adapters::one_shot::OneShotSenderAdapter,
        channel::Channel,
    },
    sdk::{
        auth::{
            AuthChallengeSenderConversation, KeyHandshakeEvent, KeyHandshakeReceiverConversation,
        },
        payments::{
            RecurringPaymentRequestSenderConversation, SinglePaymentRequestSenderConversation,
        },
    },
};
use tokio::task::JoinHandle;
use uuid::Uuid;

pub struct PortalSDK {
    router: Arc<MessageRouter<RelayPool>>,
    _listener: JoinHandle<Result<(), MessageRouterActorError>>,
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

    pub async fn new_key_handshake_url(
        &self,
        static_token: Option<String>,
    ) -> Result<(KeyHandshakeUrl, NotificationStream<KeyHandshakeEvent>), PortalSDKError> {
        let token = static_token.unwrap_or_else(|| {
            format!(
                "token_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )
        });

        let inner = KeyHandshakeReceiverConversation::new(
            self.router.keypair().public_key(),
            token.clone(),
            false,
        );
        let event = self
            .router
            .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            )))
            .await?;

        let relays = self.router.channel().get_relays().await?;

        let (main_key, subkey) = if let Some(subkey_proof) = self.router.keypair().subkey_proof() {
            (
                subkey_proof.main_key.into(),
                Some(self.router.keypair().public_key()),
            )
        } else {
            (self.router.keypair().public_key(), None)
        };

        let url = KeyHandshakeUrl {
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
            .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key, subkeys, conv,
            )))
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
            .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key, subkeys, conv,
            )))
            .await?;
        Ok(event.next().await.ok_or(PortalSDKError::Timeout)??)
    }

    pub async fn request_single_payment(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        payment_request: SinglePaymentRequestContent,
    ) -> Result<NotificationStream<PaymentResponseContent>, PortalSDKError> {
        let conv = SinglePaymentRequestSenderConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
            payment_request,
        );

        let event = self
            .router
            .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key, subkeys, conv,
            )))
            .await?;
        Ok(event)
    }

    pub async fn fetch_profile(
        &self,
        main_key: PublicKey,
    ) -> Result<Option<Profile>, PortalSDKError> {
        let conv = FetchProfileInfoConversation::new(main_key);
        let mut event = self.router.add_and_subscribe(Box::new(conv)).await?;
        let profile: Option<Profile> = event.next().await.ok_or(PortalSDKError::Timeout)??;

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
        let _ = self
            .router
            .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                self.router.keypair().public_key().into(),
                vec![],
                conv,
            )))
            .await?;

        Ok(())
    }

    pub async fn listen_closed_recurring_payment(
        &self,
    ) -> Result<NotificationStream<CloseRecurringPaymentResponse>, PortalSDKError> {
        let inner =
            CloseRecurringPaymentReceiverConversation::new(self.router.keypair().public_key());
        let event = self
            .router
            .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            )))
            .await?;
        Ok(event)
    }

    pub async fn close_recurring_payment(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        subscription_id: String,
    ) -> Result<(), PortalSDKError> {
        let content = CloseRecurringPaymentContent {
            subscription_id,
            reason: None,
            by_service: true,
        };

        let conv = CloseRecurringPaymentConversation::new(content);
        self.router
            .add_conversation(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key, subkeys, conv,
            )))
            .await?;
        Ok(())
    }

    pub async fn request_invoice(
        &self,
        recipient: PublicKey,
        subkeys: Vec<PublicKey>,
        content: InvoiceRequestContent,
    ) -> Result<Option<InvoiceResponse>, PortalSDKError> {
        let conv = InvoiceRequestConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
            content,
        );
        let mut rx = self
            .router
            .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
                recipient, subkeys, conv,
            )))
            .await?;

        if let Ok(invoice_response) = rx.next().await.ok_or(PortalSDKError::Timeout)? {
            return Ok(Some(invoice_response));
        }

        Ok(None)
    }

    pub fn issue_jwt(
        &self,
        claims: portal::protocol::jwt::CustomClaims,
        duration: Duration,
    ) -> Result<String, PortalSDKError> {
        let token =
            portal::protocol::jwt::encode(&self.router.keypair().secret_key(), claims, duration)
                .map_err(PortalSDKError::JwtError)?;
        Ok(token)
    }

    pub fn verify_jwt(
        &self,
        public_key: PublicKey,
        token: &str,
    ) -> Result<portal::protocol::jwt::CustomClaims, PortalSDKError> {
        let claims =
            portal::protocol::jwt::decode(&public_key, token).map_err(PortalSDKError::JwtError)?;
        Ok(claims)
    }

    pub async fn request_cashu(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        content: CashuRequestContent,
    ) -> Result<Option<CashuResponseContent>, PortalSDKError> {
        let conv = CashuRequestSenderConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
            content,
        );
        let mut rx: NotificationStream<CashuResponseContent> = self
            .router
            .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key, subkeys, conv,
            )))
            .await?;

        if let Ok(cashu_response) = rx.next().await.ok_or(PortalSDKError::Timeout)? {
            return Ok(Some(cashu_response));
        }
        Ok(None)
    }

    pub async fn send_cashu_direct(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        content: CashuDirectContent,
    ) -> Result<(), PortalSDKError> {
        let conv = CashuDirectSenderConversation::new(content);
        self.router
            .add_conversation(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key, subkeys, conv,
            )))
            .await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PortalSDKError {
    #[error("Relay pool error: {0}")]
    RelayPool(#[from] portal::nostr_relay_pool::pool::Error),

    #[error("Conversation error: {0}")]
    Conversation(#[from] ConversationError),

    #[error("Message router actor error: {0}")]
    MessageRouterActor(#[from] MessageRouterActorError),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Master key required")]
    MasterKeyRequired,

    #[error("JWT error: {0}")]
    JwtError(#[from] portal::protocol::jwt::JwtError),
}
