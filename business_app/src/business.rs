use std::sync::Arc;

use async_utility::task::spawn;
use portal::{
    cashu::{CashuDirectSenderConversation, CashuRequestSenderConversation},
    close_subscription::CloseRecurringPaymentConversation,
    protocol::{
        key_handshake::KeyHandshakeUrl,
        model::{
            auth::AuthResponseEvent,
            bindings::PublicKey,
            payment::{
                CashuDirectContent, CashuRequestContent, CashuResponseContent,
                CloseRecurringPaymentContent, PaymentResponseContent,
                RecurringPaymentRequestContent, RecurringPaymentResponseContent,
                SinglePaymentRequestContent,
            },
        },
    },
    router::{
        MultiKeyListenerAdapter, MultiKeySenderAdapter, NotificationStream, channel::Channel,
    },
    sdk::payments::RecurringPaymentRequestSenderConversation,
};

use crate::{AppError, PortalBusiness};

use portal::sdk::{
    auth::{AuthChallengeSenderConversation, KeyHandshakeEvent, KeyHandshakeReceiverConversation},
    payments::SinglePaymentRequestSenderConversation,
};

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait KeyHandshakeListener: Send + Sync {
    async fn on_key_handshake(&self, pubkey: PublicKey) -> Result<(), AppError>;
}

#[uniffi::export]
impl PortalBusiness {
    // PortalBusiness
    pub async fn listen_for_key_handshake(
        &self,
        static_token: Option<String>,
        listener: Arc<dyn KeyHandshakeListener>,
    ) -> Result<KeyHandshakeUrl, AppError> {
        // Generate a static token if not provided
        let static_token = static_token.unwrap_or_else(|| {
            format!(
                "token_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )
        });
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
            token: static_token.clone(),
            subkey: subkey.map(|k| k.into()),
        };

        let inner = KeyHandshakeReceiverConversation::new(
            self.router.keypair().public_key(),
            static_token.clone(),
            true,
        );

        let mut event: NotificationStream<KeyHandshakeEvent> = self
            .router
            .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            )))
            .await?;

        while let Ok(response) = event.next().await.ok_or(AppError::ListenerDisconnected)? {
            log::debug!("Received key handshake: {:?}", response);

            let evt = Arc::clone(&listener);
            let _ = self.runtime.add_task(async move {
                evt.on_key_handshake(response.main_key.into()).await?;
                Ok::<(), AppError>(())
            });
        }

        Ok(url)
    }

    // PortalBusiness
    pub async fn authenticate_key(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
    ) -> Result<AuthResponseEvent, AppError> {
        let conv = AuthChallengeSenderConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
        );

        let mut event: NotificationStream<AuthResponseEvent> = self
            .router
            .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key.into(),
                subkeys.into_iter().map(|k| k.into()).collect(),
                conv,
            )))
            .await?;

        let next_res = event
            .next()
            .await
            .ok_or(AppError::ListenerDisconnected)?
            .map_err(|e| AppError::SerdeError(e.to_string()))?;

        Ok(next_res.into())
    }

    // PortalBusiness
    pub async fn request_recurring_payment(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        payment_request: RecurringPaymentRequestContent,
    ) -> Result<RecurringPaymentResponseContent, AppError> {
        let conv = RecurringPaymentRequestSenderConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
            payment_request,
        );

        let mut event: NotificationStream<RecurringPaymentResponseContent> = self
            .router
            .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key.into(),
                subkeys.into_iter().map(|k| k.into()).collect(),
                conv,
            )))
            .await?;
        Ok(event
            .next()
            .await
            .ok_or(AppError::ListenerDisconnected)?
            .map_err(|e| AppError::SerdeError(e.to_string()))?)
    }

    // PortalBusiness
    pub async fn request_single_payment(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        payment_request: SinglePaymentRequestContent,
    ) -> Result<PaymentResponseContent, AppError> {
        let conv = SinglePaymentRequestSenderConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
            payment_request,
        );

        let mut event: NotificationStream<PaymentResponseContent> = self
            .router
            .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key.into(),
                subkeys.into_iter().map(|k| k.into()).collect(),
                conv,
            )))
            .await?;

        let next = event.next().await.ok_or(AppError::ListenerDisconnected)?;

        Ok(next.map_err(|e| AppError::SerdeError(e.to_string()))?)
    }

    // PortalBusiness
    pub async fn request_cashu(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        content: CashuRequestContent,
    ) -> Result<Option<CashuResponseContent>, AppError> {
        let conv = CashuRequestSenderConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
            content,
        );
        let mut rx: NotificationStream<CashuResponseContent> = self
            .router
            .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key.into(),
                subkeys.into_iter().map(|k| k.into()).collect(),
                conv,
            )))
            .await?;

        if let Ok(cashu_response) = rx.next().await.ok_or(AppError::ListenerDisconnected)? {
            return Ok(Some(cashu_response));
        }
        Ok(None)
    }

    // PortalBusiness
    pub async fn send_cashu_direct(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        content: CashuDirectContent,
    ) -> Result<(), AppError> {
        let conv = CashuDirectSenderConversation::new(content);
        self.router
            .add_conversation(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key.into(),
                subkeys.into_iter().map(|k| k.into()).collect(),
                conv,
            )))
            .await?;
        Ok(())
    }

    // PortalBusiness
    pub async fn close_recurring_payment(
        &self,
        main_key: PublicKey,
        subkeys: Vec<PublicKey>,
        subscription_id: String,
    ) -> Result<(), AppError> {
        let content = CloseRecurringPaymentContent {
            subscription_id,
            reason: None,
            by_service: true,
        };

        let conv = CloseRecurringPaymentConversation::new(content);
        self.router
            .add_conversation(Box::new(MultiKeySenderAdapter::new_with_user(
                main_key.into(),
                subkeys.into_iter().map(|k| k.into()).collect(),
                conv,
            )))
            .await?;
        Ok(())
    }
}
