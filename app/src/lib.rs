pub mod db;
pub mod logger;
pub mod nwc;
pub mod runtime;
pub mod wallet;

use std::{collections::HashMap, sync::Arc};

use bitcoin::{Network, bip32};
use cdk_common::SECP256K1;
use chrono::Duration;
use nostr::event::EventBuilder;
use nostr_relay_pool::monitor::{Monitor, MonitorNotification};
use portal::{
    app::{
        auth::{
            AuthChallengeEvent, AuthChallengeListenerConversation, AuthResponseConversation,
            KeyHandshakeConversation,
        },
        payments::{
            PaymentRequestContent, PaymentRequestEvent, PaymentRequestListenerConversation,
            PaymentStatusSenderConversation, RecurringPaymentStatusSenderConversation,
        },
    },
    cashu::{
        CashuDirectReceiverConversation, CashuRequestReceiverConversation,
        CashuResponseSenderConversation,
    },
    close_subscription::{
        CloseRecurringPaymentConversation, CloseRecurringPaymentReceiverConversation,
    },
    invoice::{InvoiceReceiverConversation, InvoiceRequestConversation, InvoiceSenderConversation},
    nostr::nips::nip19::ToBech32,
    nostr_relay_pool::{RelayOptions, RelayPool},
    profile::{FetchProfileInfoConversation, Profile, SetProfileConversation},
    protocol::{
        jwt::CustomClaims,
        key_handshake::KeyHandshakeUrl,
        model::{
            Timestamp,
            auth::{AuthResponseStatus, SubkeyProof},
            bindings::PublicKey,
            payment::{
                CashuDirectContentWithKey, CashuRequestContentWithKey, CashuResponseContent,
                CashuResponseStatus, CloseRecurringPaymentContent, CloseRecurringPaymentResponse,
                InvoiceRequestContent, InvoiceRequestContentWithKey, InvoiceResponse,
                PaymentResponseContent, RecurringPaymentRequestContent,
                RecurringPaymentResponseContent, SinglePaymentRequestContent,
            },
        },
    },
    router::{
        MessageRouter, MultiKeyListenerAdapter, MultiKeySenderAdapter, NotificationStream,
        adapters::one_shot::OneShotSenderAdapter,
    },
    utils::verify_nip05,
};

pub use portal::app::*;
pub use rates;

use crate::{
    logger::{CallbackLogger, LogCallback, LogLevel},
    runtime::BindingsRuntime,
};

uniffi::setup_scaffolding!();

const PROFILE_SERVICE_URL: &str = "https://profile.getportal.cc";

#[uniffi::export]
pub fn init_logger(callback: Arc<dyn LogCallback>, max_level: LogLevel) -> Result<(), AppError> {
    let callback = CallbackLogger::with_max_level(callback, max_level.into());
    callback
        .init()
        .map_err(|e| AppError::LoggerError(e.to_string()))?;

    log::info!("Logger set");

    std::panic::set_hook(Box::new(|info| {
        log::error!("Panic: {:?}", info);
    }));

    Ok(())
}
use crate::nwc::MakeInvoiceResponse;

#[uniffi::export]
pub fn generate_mnemonic() -> Result<Mnemonic, MnemonicError> {
    let inner = bip39::Mnemonic::generate(12).map_err(|_| MnemonicError::InvalidMnemonic)?;
    Ok(Mnemonic { inner })
}

#[uniffi::export]
pub fn key_to_hex(key: PublicKey) -> String {
    key.to_string()
}

#[derive(uniffi::Object)]
pub struct Mnemonic {
    inner: bip39::Mnemonic,
}

#[uniffi::export]
impl Mnemonic {
    #[uniffi::constructor]
    pub fn new(words: &str) -> Result<Self, MnemonicError> {
        let inner = bip39::Mnemonic::parse(words).map_err(|_| MnemonicError::InvalidMnemonic)?;
        Ok(Self { inner })
    }

    pub fn get_keypair(&self) -> Result<Keypair, MnemonicError> {
        let secp = bitcoin::secp256k1::Secp256k1::new();

        let seed = self.inner.to_seed("");
        let path = format!("m/44'/1237'/0'/0/0");
        let xprv = bip32::Xpriv::new_master(bitcoin::Network::Bitcoin, &seed)
            .map_err(|_| MnemonicError::InvalidMnemonic)?;
        let private_key = xprv
            .derive_priv(&secp, &path.parse::<bip32::DerivationPath>().unwrap())
            .map_err(|_| MnemonicError::InvalidMnemonic)?
            .to_priv();

        let keys = portal::nostr::Keys::new(
            portal::nostr::SecretKey::from_slice(&private_key.to_bytes()).unwrap(),
        );
        Ok(Keypair {
            inner: portal::protocol::LocalKeypair::new(keys, None),
        })
    }

    pub fn derive_cashu(&self) -> Vec<u8> {
        let seed = self.inner.to_seed("");
        let xpriv = bip32::Xpriv::new_master(Network::Bitcoin, &seed).expect("Valid seed");
        let xpriv = xpriv
            .derive_priv(
                &SECP256K1,
                &[
                    bip32::ChildNumber::from_hardened_idx(129372).unwrap(),
                    bip32::ChildNumber::from_hardened_idx(0).unwrap(),
                ],
            )
            .expect("Valid path");

        xpriv.private_key.secret_bytes().to_vec()
    }

    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

#[derive(Debug, PartialEq, thiserror::Error, uniffi::Error)]
pub enum MnemonicError {
    #[error("Invalid mnemonic")]
    InvalidMnemonic,
}

impl From<bip39::Error> for MnemonicError {
    fn from(_: bip39::Error) -> Self {
        MnemonicError::InvalidMnemonic
    }
}

#[derive(uniffi::Object)]
pub struct Keypair {
    pub inner: portal::protocol::LocalKeypair,
}

#[uniffi::export]
impl Keypair {
    #[uniffi::constructor]
    pub fn new(keypair: Arc<Keypair>) -> Result<Self, KeypairError> {
        Ok(Self {
            inner: keypair.inner.clone(),
        })
    }

    pub fn public_key(&self) -> portal::protocol::model::bindings::PublicKey {
        portal::protocol::model::bindings::PublicKey(self.inner.public_key())
    }

    pub fn subkey_proof(&self) -> Option<SubkeyProof> {
        self.inner.subkey_proof().map(|p| p.clone())
    }

    pub fn nsec(&self) -> Result<String, KeypairError> {
        let keys = self.inner.secret_key();
        let nsec = keys.to_bech32().map_err(|_| KeypairError::InvalidNsec)?;
        Ok(nsec)
    }

    pub fn issue_jwt(
        &self,
        target_key: PublicKey,
        expires_in_hours: i64,
    ) -> Result<String, KeypairError> {
        let token = portal::protocol::jwt::encode(
            &self.inner.secret_key(),
            CustomClaims::new(target_key.into()),
            Duration::hours(expires_in_hours),
        )
        .map_err(|e| KeypairError::JwtError(e.to_string()))?;
        Ok(token)
    }

    pub fn verify_jwt(
        &self,
        pubkey: PublicKey,
        token: &str,
    ) -> Result<portal::protocol::jwt::CustomClaims, KeypairError> {
        let claims = portal::protocol::jwt::decode(&pubkey.into(), token)
            .map_err(|e| KeypairError::JwtError(e.to_string()))?;
        Ok(claims)
    }
}

#[derive(Debug, PartialEq, thiserror::Error, uniffi::Error)]
pub enum KeypairError {
    #[error("Invalid nsec")]
    InvalidNsec,

    #[error("JWT error: {0}")]
    JwtError(String),
}

#[derive(uniffi::Object)]
pub struct PortalApp {
    router: Arc<MessageRouter<Arc<RelayPool>>>,
    relay_pool: Arc<RelayPool>,
    runtime: Arc<BindingsRuntime>,
}

#[uniffi::export]
pub fn parse_key_handshake_url(url: &str) -> Result<KeyHandshakeUrl, ParseError> {
    use std::str::FromStr;
    Ok(KeyHandshakeUrl::from_str(url)?)
}

#[uniffi::export]
pub fn parse_calendar(s: &str) -> Result<portal::protocol::calendar::Calendar, ParseError> {
    use std::str::FromStr;
    Ok(portal::protocol::calendar::Calendar::from_str(s)?)
}

#[derive(Debug, PartialEq, thiserror::Error, uniffi::Error)]
pub enum ParseError {
    #[error("Parse error: {0}")]
    Inner(String),
}
impl From<portal::protocol::key_handshake::ParseError> for ParseError {
    fn from(error: portal::protocol::key_handshake::ParseError) -> Self {
        ParseError::Inner(error.to_string())
    }
}
impl From<portal::protocol::calendar::CalendarError> for ParseError {
    fn from(error: portal::protocol::calendar::CalendarError) -> Self {
        ParseError::Inner(error.to_string())
    }
}

#[derive(Debug, PartialEq, thiserror::Error, uniffi::Error)]
pub enum CallbackError {
    #[error("Callback error: {0}")]
    Error(String),
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait AuthChallengeListener: Send + Sync {
    async fn on_auth_challenge(
        &self,
        event: AuthChallengeEvent,
    ) -> Result<AuthResponseStatus, CallbackError>;
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait PaymentStatusNotifier: Send + Sync {
    async fn notify(&self, status: PaymentResponseContent) -> Result<(), CallbackError>;
}

struct LocalStatusNotifier {
    router: Arc<MessageRouter<Arc<RelayPool>>>,
    request: PaymentRequestEvent,
}

#[async_trait::async_trait]
impl PaymentStatusNotifier for LocalStatusNotifier {
    async fn notify(&self, status: PaymentResponseContent) -> Result<(), CallbackError> {
        let conv = PaymentStatusSenderConversation::new(
            self.request.service_key.into(),
            self.request.recipient.into(),
            status,
        );
        self.router
            .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                self.request.recipient.into(),
                vec![],
                conv,
            )))
            .await
            .map_err(|e| CallbackError::Error(e.to_string()))?;

        Ok(())
    }
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait PaymentRequestListener: Send + Sync {
    async fn on_single_payment_request(
        &self,
        event: SinglePaymentRequest,
        notifier: Arc<dyn PaymentStatusNotifier>,
    ) -> Result<(), CallbackError>;
    async fn on_recurring_payment_request(
        &self,
        event: RecurringPaymentRequest,
    ) -> Result<RecurringPaymentResponseContent, CallbackError>;
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait ClosedRecurringPaymentListener: Send + Sync {
    async fn on_closed_recurring_payment(
        &self,
        event: CloseRecurringPaymentResponse,
    ) -> Result<(), CallbackError>;
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait InvoiceRequestListener: Send + Sync {
    async fn on_invoice_requests(
        &self,
        event: InvoiceRequestContentWithKey,
    ) -> Result<MakeInvoiceResponse, CallbackError>;
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait InvoiceResponseListener: Send + Sync {
    async fn on_invoice_response(&self, event: InvoiceResponse) -> Result<(), CallbackError>;
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait RelayStatusListener: Send + Sync {
    async fn on_relay_status_change(
        &self,
        relay_url: RelayUrl,
        status: RelayStatus,
    ) -> Result<(), CallbackError>;
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait CashuRequestListener: Send + Sync {
    async fn on_cashu_request(
        &self,
        event: CashuRequestContentWithKey,
    ) -> Result<CashuResponseStatus, CallbackError>;
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait CashuDirectListener: Send + Sync {
    async fn on_cashu_direct(&self, event: CashuDirectContentWithKey) -> Result<(), CallbackError>;
}

#[uniffi::export]
impl PortalApp {
    #[uniffi::constructor]
    pub async fn new(
        keypair: Arc<Keypair>,
        relays: Vec<String>,
        relay_status_listener: Arc<dyn RelayStatusListener>,
    ) -> Result<Arc<Self>, AppError> {
        // Initialize relay pool with monitoring
        let relay_pool = RelayPool::builder().monitor(Monitor::new(4096)).build();
        let notifications = relay_pool.monitor().unwrap().subscribe();

        // Add relays to the pool
        for relay in &relays {
            relay_pool
                .add_relay(relay, RelayOptions::default().reconnect(false))
                .await?;
        }
        relay_pool.connect().await;
        let relay_pool = Arc::new(relay_pool);

        // Initialize runtime
        let runtime = Arc::new(BindingsRuntime::new());

        // Set up relay status monitoring
        Self::setup_relay_status_monitoring(
            Arc::clone(&runtime),
            notifications,
            relay_status_listener,
        );

        // Create router with keypair
        let keypair = keypair.inner.clone();
        let relay_pool_clone = Arc::clone(&relay_pool);
        let router = async_utility::task::spawn(async move {
            let router = MessageRouter::new(relay_pool_clone, keypair);
            Arc::new(router)
        })
        .join()
        .await
        .map_err(|_| AppError::ConversationError("Failed to start router actor".to_string()))?;

        // Ensure the actor is ready
        log::debug!("Pinging router actor to ensure it's ready...");
        router.ping().await?;
        log::debug!("Router actor is ready");

        Ok(Arc::new(Self {
            router,
            relay_pool,
            runtime,
        }))
    }

    /// Reconnect to all relays
    ///
    /// This method disconnects all relays and then connects them again.
    pub async fn reconnect(&self) -> Result<(), AppError> {
        let router = self.router.channel();

        // 1. Disconnect all relays (sets them to Terminated)
        router.disconnect().await;

        // 2. Reset all relay connection stats
        // let relays = router.relays().await;
        // for relay in relays.values() {
        //     relay.stats().reset_attempts();
        // }

        // 3. Connect all relays (spawns fresh tasks)
        router.connect().await;

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), AppError> {
        self.router.shutdown().await?;

        self.runtime.shutdown();
        Ok(())
    }

    pub async fn listen(&self) -> Result<(), AppError> {
        let _ = futures::join!(self.router.listen(), self.runtime.run());

        Ok(())
    }

    pub async fn send_key_handshake(&self, url: KeyHandshakeUrl) -> Result<(), AppError> {
        let our_relays = self
            .relay_pool
            .relays()
            .await
            .iter()
            .filter_map(|(url, relay)| {
                // Only add relays from which we are listening
                if relay.flags().has_read() {
                    Some(url.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut new_relays = vec![];

        // Connect to the new relays but without subscribing (READ is disabled)
        for relay in &url.relays {
            let is_new = self
                .relay_pool
                .add_relay(relay, RelayOptions::default().read(false).reconnect(false))
                .await?;
            if is_new {
                new_relays.push(relay.clone());

                self.relay_pool.connect_relay(relay).await?;
                self.router.add_relay(relay.clone(), false).await?;
            }
        }

        let _id = self
            .router
            .add_conversation_with_relays(
                Box::new(OneShotSenderAdapter::new_with_user(
                    url.send_to(),
                    url.subkey.map(|s| vec![s.into()]).unwrap_or_default(),
                    KeyHandshakeConversation::new(url.clone(), our_relays),
                )),
                url.relays,
            )
            .await?;

        // Disconnect the new relays, only if they still don't have the READ flag (maybe it was added in the meantime)
        let pool_relays = self.relay_pool.all_relays().await;
        for relay in new_relays {
            let relay = pool_relays.iter().find(|(url, _)| url.to_string() == relay);
            if let Some((url, relay)) = relay {
                if !relay.flags().has_read() {
                    self.router.remove_relay(url.to_string()).await?;
                    self.relay_pool.disconnect_relay(url).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn listen_for_auth_challenge(
        &self,
        evt: Arc<dyn AuthChallengeListener>,
    ) -> Result<(), AppError> {
        let inner = AuthChallengeListenerConversation::new(self.router.keypair().public_key());
        let mut rx: NotificationStream<portal::app::auth::AuthChallengeEvent> = self
            .router
            .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            )))
            .await?;

        while let Ok(response) = rx.next().await.ok_or(AppError::ListenerDisconnected)? {
            let evt = Arc::clone(&evt);
            let router = Arc::clone(&self.router);

            let _ = self.runtime.add_task(async move {
                log::debug!("Received auth challenge: {:?}", response);

                let status = evt.on_auth_challenge(response.clone()).await?;
                log::debug!("Auth challenge callback result: {:?}", status);

                let conv = AuthResponseConversation::new(
                    response.clone(),
                    router.keypair().subkey_proof().cloned(),
                    status,
                );
                router
                    .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                        response.recipient.into(),
                        vec![],
                        conv,
                    )))
                    .await?;

                Ok::<(), AppError>(())
            });
        }

        Ok(())
    }

    pub async fn listen_for_payment_request(
        &self,
        evt: Arc<dyn PaymentRequestListener>,
    ) -> Result<(), AppError> {
        let inner = PaymentRequestListenerConversation::new(self.router.keypair().public_key());
        let mut rx: NotificationStream<portal::app::payments::PaymentRequestEvent> = self
            .router
            .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            )))
            .await?;

        while let Ok(request) = rx.next().await.ok_or(AppError::ListenerDisconnected)? {
            let evt = Arc::clone(&evt);
            let router = Arc::clone(&self.router);

            let _ = self.runtime.add_task(async move {
                match &request.content {
                    PaymentRequestContent::Single(content) => {
                        let req = SinglePaymentRequest {
                            service_key: request.service_key,
                            recipient: request.recipient,
                            expires_at: request.expires_at,
                            content: content.clone(),
                            event_id: request.event_id.clone(),
                        };
                        evt.on_single_payment_request(
                            req,
                            Arc::new(LocalStatusNotifier { router, request }),
                        )
                        .await?;
                    }
                    PaymentRequestContent::Recurring(content) => {
                        let req = RecurringPaymentRequest {
                            service_key: request.service_key,
                            recipient: request.recipient,
                            expires_at: request.expires_at,
                            content: content.clone(),
                            event_id: request.event_id.clone(),
                        };
                        let status = evt.on_recurring_payment_request(req).await?;
                        let conv = RecurringPaymentStatusSenderConversation::new(
                            request.service_key.into(),
                            request.recipient.into(),
                            status,
                        );
                        router
                            .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                                request.recipient.into(),
                                vec![],
                                conv,
                            )))
                            .await?;
                    }
                }

                Ok::<(), AppError>(())
            });
        }

        Ok(())
    }

    pub async fn fetch_profile(&self, pubkey: PublicKey) -> Result<Option<Profile>, AppError> {
        let conv = FetchProfileInfoConversation::new(pubkey.into());
        let mut notification: NotificationStream<Option<portal::profile::Profile>> =
            self.router.add_and_subscribe(Box::new(conv)).await?;
        let metadata = notification
            .next()
            .await
            .ok_or(AppError::ListenerDisconnected)?;

        match metadata {
            Ok(Some(mut profile)) => {
                let checked_profile = async_utility::task::spawn(async move {
                    if let Some(nip05) = &profile.nip05 {
                        let verified = verify_nip05(nip05, &pubkey).await;
                        if !verified {
                            profile.nip05 = None;
                        }
                    }
                    profile
                })
                .join()
                .await
                .map_err(|_| {
                    AppError::ProfileFetchingError("Failed to send request".to_string())
                })?;

                Ok(Some(checked_profile))
            }
            _ => Ok(None),
        }
    }

    pub async fn set_profile(&self, profile: Profile) -> Result<(), AppError> {
        if self.router.keypair().subkey_proof().is_some() {
            return Err(AppError::MasterKeyRequired);
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

    pub async fn connection_status(&self) -> HashMap<RelayUrl, RelayStatus> {
        let relays = self.router.channel().relays().await;
        relays
            .into_iter()
            .map(|(u, r)| (RelayUrl(u), RelayStatus::from(r.status())))
            .collect()
    }

    pub async fn close_recurring_payment(
        &self,
        service_key: PublicKey,
        subscription_id: String,
    ) -> Result<(), AppError> {
        let content = CloseRecurringPaymentContent {
            subscription_id,
            reason: None,
            by_service: false,
        };

        let conv = CloseRecurringPaymentConversation::new(content);
        self.router
            .add_conversation(Box::new(MultiKeySenderAdapter::new_with_user(
                service_key.into(),
                vec![],
                conv,
            )))
            .await?;
        Ok(())
    }

    pub async fn listen_closed_recurring_payment(
        &self,
        evt: Arc<dyn ClosedRecurringPaymentListener>,
    ) -> Result<(), AppError> {
        let inner =
            CloseRecurringPaymentReceiverConversation::new(self.router.keypair().public_key());
        let mut rx: NotificationStream<
            portal::protocol::model::payment::CloseRecurringPaymentResponse,
        > = self
            .router
            .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            )))
            .await?;

        while let Ok(response) = rx.next().await.ok_or(AppError::ListenerDisconnected)? {
            let evt = Arc::clone(&evt);

            let _ = self.runtime.add_task(async move {
                log::debug!("Received closed recurring payment: {:?}", response);

                let _ = evt.on_closed_recurring_payment(response).await?;

                Ok::<(), AppError>(())
            });
        }
        Ok(())
    }

    pub async fn add_relay(&self, url: String) -> Result<(), AppError> {
        self.relay_pool
            .add_relay(&url, RelayOptions::default().reconnect(false))
            .await?;
        self.relay_pool.connect_relay(&url).await?;
        self.router.add_relay(url, true).await?;
        Ok(())
    }

    pub async fn remove_relay(&self, url: String) -> Result<(), AppError> {
        self.relay_pool.remove_relay(&url).await?;
        self.router.remove_relay(url).await?;
        Ok(())
    }

    pub async fn reconnect_relay(&self, url: String) -> Result<(), AppError> {
        self.relay_pool.connect_relay(url).await?;
        Ok(())
    }

    pub async fn register_nip05(&self, local_part: String) -> Result<(), AppError> {
        self.post_request_profile_service(EventContent {
            nip_05: Some(local_part),
            img: None,
        })
        .await?;
        Ok(())
    }

    pub async fn listen_invoice_requests(
        &self,
        evt: Arc<dyn InvoiceRequestListener>,
    ) -> Result<(), AppError> {
        let inner = InvoiceReceiverConversation::new(self.router.keypair().public_key());
        let mut rx: NotificationStream<
            portal::protocol::model::payment::InvoiceRequestContentWithKey,
        > = self
            .router
            .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            )))
            .await?;

        while let Ok(request) = rx.next().await.ok_or(AppError::ListenerDisconnected)? {
            log::debug!("Received invoice request payment: {:?}", request);

            let recipient: nostr::key::PublicKey = request.recipient.into();

            let evt = Arc::clone(&evt);
            let router = Arc::clone(&self.router);

            let _ = self.runtime.add_task(async move {
                let invoice = evt.on_invoice_requests(request.clone()).await?;

                let invoice_response = InvoiceResponse {
                    request: request,
                    invoice: invoice.invoice,
                    payment_hash: invoice.payment_hash,
                };

                let conv = InvoiceSenderConversation::new(invoice_response);

                router
                    .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                        recipient.to_owned(),
                        vec![],
                        conv,
                    )))
                    .await?;

                Ok::<(), AppError>(())
            });
        }

        Ok(())
    }

    pub async fn register_img(&self, img_base64: String) -> Result<(), AppError> {
        self.post_request_profile_service(EventContent {
            nip_05: None,
            img: Some(img_base64),
        })
        .await?;
        Ok(())
    }

    pub async fn request_invoice(
        &self,
        recipient: PublicKey,
        content: InvoiceRequestContent,
        evt: Arc<dyn InvoiceResponseListener>,
    ) -> Result<(), AppError> {
        let conv = InvoiceRequestConversation::new(
            self.router.keypair().public_key(),
            self.router.keypair().subkey_proof().cloned(),
            content,
        );
        let mut rx: NotificationStream<portal::protocol::model::payment::InvoiceResponse> = self
            .router
            .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
                recipient.into(),
                vec![],
                conv,
            )))
            .await?;

        if let Ok(invoice_response) = rx.next().await.ok_or(AppError::ListenerDisconnected)? {
            let _ = evt.on_invoice_response(invoice_response.clone()).await?;
        }
        Ok(())
    }

    pub async fn listen_cashu_requests(
        &self,
        evt: Arc<dyn CashuRequestListener>,
    ) -> Result<(), AppError> {
        let inner = CashuRequestReceiverConversation::new(self.router.keypair().public_key());
        let mut rx: NotificationStream<CashuRequestContentWithKey> = self
            .router
            .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            )))
            .await?;

        while let Ok(request) = rx.next().await.ok_or(AppError::ListenerDisconnected)? {
            let evt = Arc::clone(&evt);
            let router = Arc::clone(&self.router);
            let _ = self.runtime.add_task(async move {
                let status = evt.on_cashu_request(request.clone()).await?;

                let recipient = request.recipient.into();
                let response = CashuResponseContent {
                    request: request,
                    status: status,
                };
                let conv = CashuResponseSenderConversation::new(response);
                router
                    .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                        recipient,
                        vec![],
                        conv,
                    )))
                    .await?;
                Ok::<(), AppError>(())
            });
        }
        Ok(())
    }

    pub async fn listen_cashu_direct(
        &self,
        evt: Arc<dyn CashuDirectListener>,
    ) -> Result<(), AppError> {
        let inner = CashuDirectReceiverConversation::new(self.router.keypair().public_key());
        let mut rx: NotificationStream<CashuDirectContentWithKey> = self
            .router
            .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            )))
            .await?;

        while let Ok(response) = rx.next().await.ok_or(AppError::ListenerDisconnected)? {
            let evt = Arc::clone(&evt);
            let _ = self.runtime.add_task(async move {
                let _ = evt.on_cashu_direct(response.clone()).await?;
                Ok::<(), AppError>(())
            });
        }
        Ok(())
    }
}

impl PortalApp {
    /// Set up relay status monitoring in a separate task
    fn setup_relay_status_monitoring(
        runtime: Arc<BindingsRuntime>,
        mut notifications: tokio::sync::broadcast::Receiver<MonitorNotification>,
        relay_status_listener: Arc<dyn RelayStatusListener>,
    ) {
        let _ = runtime.add_task(async move {
            while let Ok(notification) = notifications.recv().await {
                match notification {
                    MonitorNotification::StatusChanged { relay_url, status } => {
                        // log::info!("Relay {:?} status changed: {:?}", relay_url, status);

                        let relay_url = RelayUrl(relay_url);
                        let status = RelayStatus::from(status);
                        if let Err(e) = relay_status_listener
                            .on_relay_status_change(relay_url, status)
                            .await
                        {
                            log::error!("Relay status listener error: {:?}", e);
                        }
                    }
                }
            }
            Ok::<(), AppError>(())
        });
    }

    async fn post_request_profile_service(&self, content: EventContent) -> Result<(), AppError> {
        let event = EventBuilder::text_note(serde_json::to_string(&content).unwrap())
            .sign_with_keys(&self.router.keypair().get_keys())
            .map_err(|_| AppError::ProfileRegistrationError("Failed to sign event".to_string()))?;
        let json_string = serde_json::to_string_pretty(&event).map_err(|_| {
            AppError::ProfileRegistrationError("Failed to serialize event".to_string())
        })?;

        let task = async_utility::task::spawn(async move {
            let client = reqwest::Client::new();
            client
                .post(PROFILE_SERVICE_URL)
                .header("Content-Type", "application/json")
                .body(json_string)
                .send()
                .await
                .map_err(|e| match e.status() {
                    Some(status_code) => {
                        AppError::ProfileRegistrationStatusError(status_code.as_u16())
                    }
                    None => AppError::ProfileRegistrationError(format!("Request failed: {}", e)),
                })
        });

        let response = task.join().await.map_err(|_| {
            AppError::ProfileRegistrationError("Failed to send request".to_string())
        })??;

        if let Err(e) = response.error_for_status() {
            return Err(AppError::ProfileRegistrationError(e.to_string()));
        }

        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
struct EventContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nip_05: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub img: Option<String>,
}

#[derive(Hash, Eq, PartialEq)]
pub struct RelayUrl(pub nostr::types::RelayUrl);

uniffi::custom_type!(RelayUrl, String, {
    try_lift: |val| {
        let url = nostr::types::RelayUrl::parse(&val)?;
        Ok(RelayUrl(url))
    },
    lower: |obj| obj.0.as_str().to_string(),
});

#[derive(uniffi::Enum, Debug)]
pub enum RelayStatus {
    Initialized,
    Pending,
    Connecting,
    Connected,
    Disconnected,
    Terminated,
    Banned,
    Sleeping,
}

impl From<nostr_relay_pool::relay::RelayStatus> for RelayStatus {
    fn from(status: nostr_relay_pool::relay::RelayStatus) -> Self {
        match status {
            nostr_relay_pool::relay::RelayStatus::Initialized => RelayStatus::Initialized,
            nostr_relay_pool::relay::RelayStatus::Pending => RelayStatus::Pending,
            nostr_relay_pool::relay::RelayStatus::Connecting => RelayStatus::Connecting,
            nostr_relay_pool::relay::RelayStatus::Connected => RelayStatus::Connected,
            nostr_relay_pool::relay::RelayStatus::Disconnected => RelayStatus::Disconnected,
            nostr_relay_pool::relay::RelayStatus::Terminated => RelayStatus::Terminated,
            nostr_relay_pool::relay::RelayStatus::Banned => RelayStatus::Banned,
            nostr_relay_pool::relay::RelayStatus::Sleeping => RelayStatus::Sleeping,
        }
    }
}

#[derive(Debug, uniffi::Record)]
pub struct SinglePaymentRequest {
    pub service_key: PublicKey,
    pub recipient: PublicKey,
    pub expires_at: Timestamp,
    pub content: SinglePaymentRequestContent,
    pub event_id: String,
}

#[derive(Debug, uniffi::Record)]
pub struct RecurringPaymentRequest {
    pub service_key: PublicKey,
    pub recipient: PublicKey,
    pub expires_at: Timestamp,
    pub content: RecurringPaymentRequestContent,
    pub event_id: String,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum AppError {
    #[error("Failed to connect to relay: {0}")]
    RelayError(String),

    #[error("Failed to send auth init: {0}")]
    ConversationError(String),

    #[error("Listener disconnected")]
    ListenerDisconnected,

    #[error("NWC error: {0}")]
    NWC(String),

    #[error("Callback error: {0}")]
    CallbackError(#[from] CallbackError),

    #[error("Master key required")]
    MasterKeyRequired,

    // database errors
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Logger error: {0}")]
    LoggerError(String),

    #[error("Profile registration error: {0}")]
    ProfileRegistrationError(String),

    #[error("{0}")]
    ProfileRegistrationStatusError(u16),

    #[error("Profile fetching error: {0}")]
    ProfileFetchingError(String),
}

impl From<portal::router::ConversationError> for AppError {
    fn from(error: portal::router::ConversationError) -> Self {
        AppError::ConversationError(error.to_string())
    }
}

impl From<portal::router::MessageRouterActorError> for AppError {
    fn from(error: portal::router::MessageRouterActorError) -> Self {
        AppError::ConversationError(error.to_string())
    }
}

impl From<portal::nostr_relay_pool::pool::Error> for AppError {
    fn from(error: portal::nostr_relay_pool::pool::Error) -> Self {
        AppError::RelayError(error.to_string())
    }
}

impl From<::nwc::Error> for AppError {
    fn from(error: ::nwc::Error) -> Self {
        AppError::NWC(error.to_string())
    }
}
