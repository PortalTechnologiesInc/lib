pub mod db;
pub mod logger;
pub mod nwc;
pub mod runtime;

use std::{collections::HashMap, sync::Arc};

use bitcoin::bip32;
use nostr::event::EventBuilder;
use portal::{
    app::{
        auth::{
            AuthChallengeEvent, AuthChallengeListenerConversation, AuthResponseConversation,
            KeyHandshakeConversation,
        },
        payments::{
            PaymentRequestContent, PaymentRequestListenerConversation,
            PaymentStatusSenderConversation, RecurringPaymentStatusSenderConversation,
        },
    },
    close_subscription::{
        CloseRecurringPaymentConversation, CloseRecurringPaymentReceiverConversation,
    },
    invoice::{InvoiceReceiverConversation, InvoiceRequestConversation, InvoiceSenderConversation},
    nostr::nips::nip19::ToBech32,
    nostr_relay_pool::{RelayOptions, RelayPool},
    profile::{FetchProfileInfoConversation, Profile, SetProfileConversation},
    protocol::{
        key_handshake::KeyHandshakeUrl,
        model::{
            Timestamp,
            auth::{AuthResponseStatus, SubkeyProof},
            bindings::PublicKey,
            payment::{
                CloseRecurringPaymentContent, CloseRecurringPaymentResponse, InvoiceRequestContent,
                InvoiceRequestContentWithKey, InvoiceResponse, PaymentResponseContent,
                RecurringPaymentRequestContent, RecurringPaymentResponseContent,
                SinglePaymentRequestContent,
            },
        },
    },
    router::{
        MessageRouter, MultiKeyListenerAdapter, MultiKeySenderAdapter,
        adapters::one_shot::OneShotSenderAdapter,
    },
};

pub use portal::app::*;
use reqwest::StatusCode;

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
    inner: portal::protocol::LocalKeypair,
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
}

#[derive(Debug, PartialEq, thiserror::Error, uniffi::Error)]
pub enum KeypairError {
    #[error("Invalid nsec")]
    InvalidNsec,
}

#[derive(uniffi::Object)]
pub struct PortalApp {
    router: Arc<MessageRouter<RelayPool>>,
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
pub trait PaymentRequestListener: Send + Sync {
    async fn on_single_payment_request(
        &self,
        event: SinglePaymentRequest,
    ) -> Result<PaymentResponseContent, CallbackError>;
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

#[uniffi::export]
impl PortalApp {
    #[uniffi::constructor]
    pub async fn new(keypair: Arc<Keypair>, relays: Vec<String>) -> Result<Arc<Self>, AppError> {
        let relay_pool = RelayPool::new();
        for relay in &relays {
            relay_pool.add_relay(relay, RelayOptions::default()).await?;
        }
        relay_pool.connect().await;

        let keypair = &keypair.inner;
        let router = Arc::new(MessageRouter::new(relay_pool, keypair.clone()));
        let runtime = Arc::new(BindingsRuntime::new());

        Ok(Arc::new(Self { router, runtime }))
    }

    pub async fn listen(&self) -> Result<(), AppError> {
        let _ = futures::join!(self.router.listen(), self.runtime.run());

        Ok(())
    }

    pub async fn send_key_handshake(&self, url: KeyHandshakeUrl) -> Result<(), AppError> {
        let relays = self
            .router
            .channel()
            .relays()
            .await
            .keys()
            .map(|r| r.to_string())
            .collect();
        let _id = self
            .router
            .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                url.send_to(),
                url.subkey.map(|s| vec![s.into()]).unwrap_or_default(),
                KeyHandshakeConversation::new(url, relays),
            )))
            .await?;
        // let rx = self.router.subscribe_to_service_request(id).await?;
        // let response = rx.await_reply().await.map_err(AppError::ConversationError)?;

        Ok(())
    }

    pub async fn listen_for_auth_challenge(
        &self,
        evt: Arc<dyn AuthChallengeListener>,
    ) -> Result<(), AppError> {
        let inner = AuthChallengeListenerConversation::new(self.router.keypair().public_key());
        let mut rx = self
            .router
            .add_and_subscribe(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            ))
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
        let mut rx = self
            .router
            .add_and_subscribe(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            ))
            .await?;

        while let Ok(response) = rx.next().await.ok_or(AppError::ListenerDisconnected)? {
            let evt = Arc::clone(&evt);
            let router = Arc::clone(&self.router);

            let _ = self.runtime.add_task(async move {
                match &response.content {
                    PaymentRequestContent::Single(content) => {
                        let req = SinglePaymentRequest {
                            service_key: response.service_key,
                            recipient: response.recipient,
                            expires_at: response.expires_at,
                            content: content.clone(),
                            event_id: response.event_id.clone(),
                        };
                        let status = evt.on_single_payment_request(req).await?;
                        let conv = PaymentStatusSenderConversation::new(
                            response.service_key.into(),
                            response.recipient.into(),
                            status,
                        );
                        router
                            .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                                response.recipient.into(),
                                vec![],
                                conv,
                            )))
                            .await?;
                    }
                    PaymentRequestContent::Recurring(content) => {
                        let req = RecurringPaymentRequest {
                            service_key: response.service_key,
                            recipient: response.recipient,
                            expires_at: response.expires_at,
                            content: content.clone(),
                            event_id: response.event_id.clone(),
                        };
                        let status = evt.on_recurring_payment_request(req).await?;
                        let conv = RecurringPaymentStatusSenderConversation::new(
                            response.service_key.into(),
                            response.recipient.into(),
                            status,
                        );
                        router
                            .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                                response.recipient.into(),
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
        let mut notification = self.router.add_and_subscribe(conv).await?;
        let metadata = notification
            .next()
            .await
            .ok_or(AppError::ListenerDisconnected)?;

        match metadata {
            Ok(Some(mut profile)) => {
                let checked_profile = async_utility::task::spawn(async move {
                    if let Some(nip05) = &profile.nip05 {
                        let verified =
                            portal::nostr::nips::nip05::verify(&pubkey.into(), &nip05, None).await;
                        if verified.ok() != Some(true) {
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
        let mut rx = self
            .router
            .add_and_subscribe(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            ))
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
        self.router.add_relay(url).await?;
        Ok(())
    }

    pub async fn remove_relay(&self, url: String) -> Result<(), AppError> {
        self.router.remove_relay(url).await?;
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
        let mut rx = self
            .router
            .add_and_subscribe(MultiKeyListenerAdapter::new(
                inner,
                self.router.keypair().subkey_proof().cloned(),
            ))
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

        let mut rx = self
            .router
            .add_and_subscribe(MultiKeySenderAdapter::new_with_user(
                recipient.into(),
                vec![],
                conv,
            ))
            .await?;

        if let Ok(invoice_response) = rx.next().await.ok_or(AppError::ListenerDisconnected)? {
            let _ = evt.on_invoice_response(invoice_response.clone()).await?;
        }
        Ok(())
    }
}

impl PortalApp {
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

#[derive(uniffi::Enum)]
pub enum RelayStatus {
    Initialized,
    Pending,
    Connecting,
    Connected,
    Disconnected,
    Terminated,
    Banned,
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
