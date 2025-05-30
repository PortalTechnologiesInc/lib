pub mod db;
pub mod nwc;

use std::{collections::HashMap, sync::Arc};

use bitcoin::bip32;
use portal::{
    app::{
        auth::{
            AuthChallengeEvent, AuthChallengeListenerConversation, AuthInitConversation,
            AuthResponseConversation,
        },
        payments::{
            CloseRecurringPaymentConversation, PaymentRequestContent, PaymentRequestListenerConversation, PaymentStatusSenderConversation, RecurringPaymentStatusSenderConversation
        },
    },
    nostr::nips::nip19::ToBech32,
    nostr_relay_pool::{RelayOptions, RelayPool},
    profile::{FetchProfileInfoConversation, Profile, SetProfileConversation},
    protocol::{
        auth_init::AuthInitUrl,
        model::{
            auth::SubkeyProof, bindings::PublicKey, payment::{
                CloseRecurringPaymentContent, PaymentResponseContent, RecurringPaymentRequestContent, RecurringPaymentResponseContent, RecurringPaymentStatus, SinglePaymentRequestContent
            }, Timestamp
        },
    },
    router::{
        adapters::one_shot::OneShotSenderAdapter, MessageRouter, MultiKeyListenerAdapter
    },
};

uniffi::setup_scaffolding!();

#[uniffi::export]
pub fn init_logger() {
    use android_logger::Config;
    use log::LevelFilter;

    android_logger::init_once(Config::default().with_max_level(LevelFilter::Trace));

    log::info!("Logger initialized");
}

pub use portal::app::*;

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
}

#[uniffi::export]
pub fn parse_auth_init_url(url: &str) -> Result<AuthInitUrl, ParseError> {
    use std::str::FromStr;
    Ok(AuthInitUrl::from_str(url)?)
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
impl From<portal::protocol::auth_init::ParseError> for ParseError {
    fn from(error: portal::protocol::auth_init::ParseError) -> Self {
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
    async fn on_auth_challenge(&self, event: AuthChallengeEvent) -> Result<bool, CallbackError>;
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

        Ok(Arc::new(Self { router }))
    }


    pub async fn listen(&self) -> Result<(), AppError> {
        self.router.listen().await.unwrap();
        Ok(())
    }

    pub async fn send_auth_init(&self, url: AuthInitUrl) -> Result<(), AppError> {
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
                AuthInitConversation { url, relays },
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
            log::debug!("Received auth challenge: {:?}", response);

            let result = evt.on_auth_challenge(response.clone()).await?;
            log::debug!("Auth challenge callback result: {:?}", result);

            if result {
                let approve = AuthResponseConversation::new(
                    response.clone(),
                    vec![],
                    self.router.keypair().subkey_proof().cloned(),
                );
                self.router
                    .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                        response.recipient.into(),
                        vec![],
                        approve,
                    )))
                    .await?;
            } else {
                // TODO: send explicit rejection
            }
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
                    let conv = PaymentStatusSenderConversation::new(response.service_key.into(), response.recipient.into(), status);
                    self.router
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
                    let conv =
                        RecurringPaymentStatusSenderConversation::new(response.service_key.into(), response.recipient.into(), status);
                    self.router
                        .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                            response.recipient.into(),
                            vec![],
                            conv,
                        )))
                        .await?;
                }
            }
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
            Ok(Some(profile)) => {
                // if let Some(nip05) = &profile.nip05 {
                //     let verified = portal::nostr::nips::nip05::verify(&pubkey.into(), &nip05, None).await;
                //     if verified.ok() != Some(true) {
                //         profile.nip05 = None;
                //     }
                // }

                Ok(Some(profile))
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
        relays.into_iter().map(|(u, r)| (RelayUrl(u), RelayStatus::from(r.status()))).collect()
    }

    pub async fn close_recurring_payment(
        &self,
        service_key: PublicKey,
        subscription_id: String,
    ) -> Result<(), AppError> {
        let content = CloseRecurringPaymentContent{
            subscription_id,
            reason: None,
        };

        let recipient = self.router.keypair().public_key();
        let conv = CloseRecurringPaymentConversation::new(service_key.into(), recipient, content);
        self.router
            .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
                recipient,
                vec![],
                conv,
            )))
            .await?;
        Ok(())
    }
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
