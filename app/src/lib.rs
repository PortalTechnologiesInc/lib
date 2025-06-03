pub mod db;
pub mod nwc;

use std::{collections::HashMap, str::FromStr, sync::Arc};

use bitcoin::bip32;
use futures::FutureExt;
use portal::{
    app::{
        auth::{
            AuthChallengeEvent, AuthChallengeListenerConversation, AuthInitConversation,
            AuthResponseConversation,
        },
        payments::{
            PaymentRequestContent, PaymentRequestListenerConversation,
            PaymentStatusSenderConversation, RecurringPaymentStatusSenderConversation,
        },
    },
    nostr::nips::nip19::ToBech32,
    nostr_relay_pool::{RelayOptions, RelayPool},
    profile::{FetchProfileInfoConversation, Profile, SetProfileConversation},
    protocol::{
        auth_init::AuthInitUrl,
        model::{
            auth::SubkeyProof, bindings::PublicKey, payment::{
                PaymentResponseContent, RecurringPaymentRequestContent, RecurringPaymentResponseContent, SinglePaymentRequestContent
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
use tokio::sync::{mpsc::{self, Receiver, Sender}, Mutex, RwLock};

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
    keypair: Arc<Keypair>,
    profiles: RwLock<HashMap<PublicKey, Profile>>,
    auth_challenge_sender: Sender<AuthChallengeEvent>,
    auth_challenge_receiver: Mutex<Receiver<AuthChallengeEvent>>,
    single_payment_sender: Sender<SinglePaymentRequest>,
    single_payment_receiver: Mutex<Receiver<SinglePaymentRequest>>,
    recurring_payment_sender: Sender<RecurringPaymentRequest>,
    recurring_payment_receiver: Mutex<Receiver<RecurringPaymentRequest>>,
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
    pub async fn new(keypair: Arc<Keypair>, _relays: Vec<String>) -> Result<Arc<Self>, AppError> {
        let (auth_challenge_sender, auth_challenge_receiver) = mpsc::channel(100);
        let (single_payment_sender, single_payment_receiver) = mpsc::channel(100);
        let (recurring_payment_sender, recurring_payment_receiver) = mpsc::channel(100);

        Ok(Arc::new(Self {
            keypair,
            profiles: RwLock::new(HashMap::new()),
            auth_challenge_sender,
            auth_challenge_receiver: Mutex::new(auth_challenge_receiver),
            single_payment_sender,
            single_payment_receiver: Mutex::new(single_payment_receiver),
            recurring_payment_sender,
            recurring_payment_receiver: Mutex::new(recurring_payment_receiver),
        }))
    }

    pub async fn listen(&self) -> Result<(), AppError> {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }

    pub async fn send_auth_init(&self, url: AuthInitUrl) -> Result<(), AppError> {
        Ok(())
    }

    pub async fn listen_for_auth_challenge(
        &self,
        evt: Arc<dyn AuthChallengeListener>,
    ) -> Result<(), AppError> {
        let mut receiver = self.auth_challenge_receiver.lock().await;
        while let Some(event) = receiver.recv().await {
            evt.on_auth_challenge(event).await?;
        }

        Ok(())
    }

    pub async fn listen_for_payment_request(
        &self,
        evt: Arc<dyn PaymentRequestListener>,
    ) -> Result<(), AppError> {
        let mut receiver_single = self.single_payment_receiver.lock().await;
        let mut receiver_recurring = self.recurring_payment_receiver.lock().await;

        loop {
            futures::select! {
                request = receiver_single.recv().fuse() => {
                    evt.on_single_payment_request(request.unwrap()).await?;
                }
                request = receiver_recurring.recv().fuse() => {
                    evt.on_recurring_payment_request(request.unwrap()).await?;
                }
            }
        }
    }

    pub async fn fetch_profile(&self, pubkey: PublicKey) -> Result<Option<Profile>, AppError> {
        Ok(self.profiles.read().await.get(&pubkey).cloned())
    }

    pub async fn set_profile(&self, profile: Profile) -> Result<(), AppError> {
        self.profiles.write().await.insert(self.keypair.public_key(), profile);
        Ok(())
    }

    pub async fn connection_status(&self) -> HashMap<RelayUrl, RelayStatus> {
        let relays = vec![
            (RelayUrl(nostr::types::RelayUrl::from_str("wss://relay.damus.io").unwrap()), RelayStatus::Connected),
            (RelayUrl(nostr::types::RelayUrl::from_str("wss://relay.nostr.band").unwrap()), RelayStatus::Pending),
            (RelayUrl(nostr::types::RelayUrl::from_str("wss://relay.snort.net").unwrap()), RelayStatus::Disconnected),
        ];
        relays.into_iter().collect()
    }

    pub async fn schedule(&self, action: ScheduledAction) -> Result<(), AppError> {
        tokio::time::sleep(std::time::Duration::from_secs(delay_seconds)).await;

        match action {
            ScheduledAction::SendAuthChallenge(event) => {
                self.auth_challenge_sender.send(event).await.unwrap();
            }
            ScheduledAction::SendPaymentRequest(request) => {
                self.single_payment_sender.send(request).await.unwrap();
            }
            ScheduledAction::SendRecurringPaymentRequest(request) => {
                self.recurring_payment_sender.send(request).await.unwrap();
            }
        }
        Ok(())
    }
}

#[derive(Debug, uniffi::Enum)]
pub enum ScheduledAction {
    SendAuthChallenge(AuthChallengeEvent),
    SendPaymentRequest(SinglePaymentRequest),
    SendRecurringPaymentRequest(RecurringPaymentRequest),
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
