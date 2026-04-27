use std::time::{SystemTime, UNIX_EPOCH};

use hex;
use serde::{Deserialize, Serialize};

#[cfg(feature = "bindings")]
use bindings::PublicKey;
#[cfg(not(feature = "bindings"))]
use nostr::PublicKey;

// Event kind ranges:
// Authentication: 27000-27999
// Payments: 28000-28999
// Identity: 29000-29999

pub mod event_kinds {
    // Remote signing request events (24133)
    pub const SIGNING_REQUEST: u16 = 24133;

    // Authentication events (27000-27999)
    pub const AUTH_CHALLENGE: u16 = 27000;
    pub const AUTH_RESPONSE: u16 = 27001;
    pub const AUTH_SUCCESS: u16 = 27002;
    pub const KEY_HANDSHAKE: u16 = 27010;

    // Payment events (28000-28999)
    pub const PAYMENT_REQUEST: u16 = 28000;
    pub const PAYMENT_RESPONSE: u16 = 28001;
    pub const PAYMENT_CONFIRMATION: u16 = 28002;
    pub const PAYMENT_ERROR: u16 = 28003;
    pub const PAYMENT_RECEIPT: u16 = 28004;
    pub const RECURRING_PAYMENT_REQUEST: u16 = 28005;
    pub const RECURRING_PAYMENT_RESPONSE: u16 = 28006;
    pub const RECURRING_PAYMENT_CANCEL: u16 = 28007;

    pub const INVOICE_REQUEST: u16 = 28008;
    pub const INVOICE_RESPONSE: u16 = 28009;

    // Identity events (29000-29499)
    pub const CERTIFICATE_REQUEST: u16 = 29000;
    pub const CERTIFICATE_RESPONSE: u16 = 29001;
    pub const CERTIFICATE_ERROR: u16 = 29002;
    pub const CERTIFICATE_REVOCATION: u16 = 29003;
    pub const CERTIFICATE_VERIFY_REQUEST: u16 = 29004;
    pub const CERTIFICATE_VERIFY_RESPONSE: u16 = 29005;

    // Cashu events (29500-29999)
    pub const CASHU_REQUEST: u16 = 29500;
    pub const CASHU_RESPONSE: u16 = 29501;
    pub const CASHU_DIRECT: u16 = 29502;

    // Control events (30000-30999)
    pub const SUBKEY_PROOF: u16 = 30000;
}

#[derive(Debug, Clone)]
pub struct Nonce([u8; 32]);

impl Nonce {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Serialize for Nonce {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Convert bytes to hex string
        let hex = hex::encode(self.0);
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for Nonce {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let hex_str = String::deserialize(deserializer)?;

        // Convert hex string back to bytes
        let bytes = hex::decode(&hex_str)
            .map_err(|e| Error::custom(format!("Invalid hex string: {}", e)))?;

        if bytes.len() != 32 {
            return Err(Error::custom(format!(
                "Invalid nonce length: expected 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Nonce(arr))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn new(timestamp: u64) -> Self {
        Self(timestamp)
    }

    pub fn now() -> Self {
        Self(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )
    }

    pub fn now_plus_seconds(seconds: u64) -> Self {
        let mut ts = Self::now();
        ts.0 += seconds;
        ts
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as string to avoid precision loss
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let s = String::deserialize(deserializer)?;

        s.parse::<u64>()
            .map(Timestamp::new)
            .map_err(|e| Error::custom(format!("Invalid timestamp: {}", e)))
    }
}

pub mod auth {
    use crate::protocol::subkey::{PublicSubkeyVerifier, SubkeyError, SubkeyMetadata};

    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct ServiceInformation {
        pub service_pubkey: PublicKey,
        pub relays: Vec<String>,
        pub token: String,
        pub subkey: Option<PublicKey>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct KeyHandshakeContent {
        pub token: String,
        pub client_info: ClientInfo,
        pub preferred_relays: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ClientInfo {
        pub name: String,
        pub version: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AuthChallengeContent {
        pub challenge: String,
        pub expires_at: Timestamp,
        pub required_permissions: Vec<String>,
        pub subkey_proof: Option<SubkeyProof>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct SubkeyProof {
        pub main_key: PublicKey,
        pub metadata: SubkeyMetadata,
    }

    impl SubkeyProof {
        pub fn verify(&self, subkey: &nostr::PublicKey) -> Result<(), SubkeyError> {
            self.main_key.verify_subkey(subkey, &self.metadata)
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AuthResponseContent {
        pub challenge: String,
        pub subkey_proof: Option<SubkeyProof>,
        pub status: AuthResponseStatus,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, uniffi::Enum)]
    #[serde(rename_all = "snake_case", tag = "status")]
    pub enum AuthResponseStatus {
        Approved {
            granted_permissions: Vec<String>,
            session_token: String,
        },
        Declined {
            reason: Option<String>,
        },
    }
}

pub mod identity {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CertificateRequestContent {
        pub requested_types: Vec<String>,
        pub requested_fields: Vec<String>,
        pub purpose: String,
        pub require_status_proofs: Option<bool>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CertificateResponseContent {
        pub certificates: std::collections::HashMap<String, serde_json::Value>,
        pub status_proofs: Option<std::collections::HashMap<String, serde_json::Value>>,
    }
}

pub mod payment {
    use crate::protocol::calendar::CalendarWrapper;

    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct SinglePaymentRequestContent {
        pub amount: u64,
        pub currency: Currency,
        pub current_exchange_rate: Option<ExchangeRate>,
        pub invoice: String,
        pub auth_token: Option<String>,
        pub expires_at: Timestamp,
        pub subscription_id: Option<String>,
        pub description: Option<String>,
        pub request_id: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    #[serde(rename_all = "snake_case")]
    pub struct PaymentResponseContent {
        pub request_id: String,
        pub status: PaymentStatus,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
    #[serde(rename_all = "snake_case")]
    pub enum PaymentStatus {
        Approved,
        Success { preimage: Option<String> },
        Rejected { reason: Option<String> },
        Failed { reason: Option<String> },
    }

    impl PaymentStatus {
        pub fn is_final(&self) -> bool {
            matches!(
                self,
                Self::Success { .. } | Self::Rejected { .. } | Self::Failed { .. }
            )
        }
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[serde(transparent)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct Millisats {
        pub value: u64,
    }

    impl Millisats {
        pub const fn new(value: u64) -> Self {
            Self { value }
        }

        pub const fn as_u64(self) -> u64 {
            self.value
        }
    }

    impl From<u64> for Millisats {
        fn from(value: u64) -> Self {
            Self::new(value)
        }
    }

    impl From<Millisats> for u64 {
        fn from(value: Millisats) -> Self {
            value.value
        }
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[serde(transparent)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct FiatCents {
        pub value: u64,
    }

    impl FiatCents {
        pub const fn new(value: u64) -> Self {
            Self { value }
        }

        pub const fn as_u64(self) -> u64 {
            self.value
        }
    }

    impl From<u64> for FiatCents {
        fn from(value: u64) -> Self {
            Self::new(value)
        }
    }

    impl From<FiatCents> for u64 {
        fn from(value: FiatCents) -> Self {
            value.value
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
    pub enum Currency {
        Millisats,
        #[serde(untagged)]
        Fiat(String),
    }

    /// Type-system mapping between currency and amount unit.
    pub trait PaymentCurrencyKind {
        type Amount: Into<u64> + Copy;
        fn into_currency(self) -> Currency;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct MillisatsCurrency;

    impl PaymentCurrencyKind for MillisatsCurrency {
        type Amount = Millisats;

        fn into_currency(self) -> Currency {
            Currency::Millisats
        }
    }

    #[derive(Debug, Clone)]
    pub struct FiatCurrency {
        pub code: String,
    }

    impl PaymentCurrencyKind for FiatCurrency {
        type Amount = FiatCents;

        fn into_currency(self) -> Currency {
            Currency::Fiat(self.code)
        }
    }

    #[derive(Debug, Clone)]
    pub struct SinglePaymentRequestTyped<C: PaymentCurrencyKind> {
        pub amount: C::Amount,
        pub currency: C,
        pub current_exchange_rate: Option<ExchangeRate>,
        pub invoice: String,
        pub auth_token: Option<String>,
        pub expires_at: Timestamp,
        pub subscription_id: Option<String>,
        pub description: Option<String>,
        pub request_id: String,
    }

    impl<C: PaymentCurrencyKind> From<SinglePaymentRequestTyped<C>> for SinglePaymentRequestContent {
        fn from(value: SinglePaymentRequestTyped<C>) -> Self {
            Self {
                amount: value.amount.into(),
                currency: value.currency.into_currency(),
                current_exchange_rate: value.current_exchange_rate,
                invoice: value.invoice,
                auth_token: value.auth_token,
                expires_at: value.expires_at,
                subscription_id: value.subscription_id,
                description: value.description,
                request_id: value.request_id,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct RecurringPaymentRequestTyped<C: PaymentCurrencyKind> {
        pub amount: C::Amount,
        pub currency: C,
        pub recurrence: RecurrenceInfo,
        pub current_exchange_rate: Option<ExchangeRate>,
        pub expires_at: Timestamp,
        pub auth_token: Option<String>,
        pub description: Option<String>,
        pub request_id: String,
    }

    impl<C: PaymentCurrencyKind> From<RecurringPaymentRequestTyped<C>> for RecurringPaymentRequestContent {
        fn from(value: RecurringPaymentRequestTyped<C>) -> Self {
            Self {
                amount: value.amount.into(),
                currency: value.currency.into_currency(),
                recurrence: value.recurrence,
                current_exchange_rate: value.current_exchange_rate,
                expires_at: value.expires_at,
                auth_token: value.auth_token,
                description: value.description,
                request_id: value.request_id,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct InvoiceRequestTyped<C: PaymentCurrencyKind> {
        pub request_id: String,
        pub amount: C::Amount,
        pub currency: C,
        pub current_exchange_rate: Option<ExchangeRate>,
        pub expires_at: Timestamp,
        pub description: Option<String>,
        pub refund_invoice: Option<String>,
    }

    impl<C: PaymentCurrencyKind> From<InvoiceRequestTyped<C>> for InvoiceRequestContent {
        fn from(value: InvoiceRequestTyped<C>) -> Self {
            Self {
                request_id: value.request_id,
                amount: value.amount.into(),
                currency: value.currency.into_currency(),
                current_exchange_rate: value.current_exchange_rate,
                expires_at: value.expires_at,
                description: value.description,
                refund_invoice: value.refund_invoice,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct RecurringPaymentRequestContent {
        pub amount: u64,
        pub currency: Currency,
        pub recurrence: RecurrenceInfo,
        pub current_exchange_rate: Option<ExchangeRate>,
        pub expires_at: Timestamp,
        pub auth_token: Option<String>,
        pub description: Option<String>,
        pub request_id: String,
    }

    impl SinglePaymentRequestContent {
        pub fn amount_millisats(&self) -> Option<Millisats> {
            match self.currency {
                Currency::Millisats => Some(Millisats::new(self.amount)),
                Currency::Fiat(_) => None,
            }
        }

        pub fn amount_fiat_cents(&self) -> Option<FiatCents> {
            match self.currency {
                Currency::Fiat(_) => Some(FiatCents::new(self.amount)),
                Currency::Millisats => None,
            }
        }
    }

    impl RecurringPaymentRequestContent {
        pub fn amount_millisats(&self) -> Option<Millisats> {
            match self.currency {
                Currency::Millisats => Some(Millisats::new(self.amount)),
                Currency::Fiat(_) => None,
            }
        }

        pub fn amount_fiat_cents(&self) -> Option<FiatCents> {
            match self.currency {
                Currency::Fiat(_) => Some(FiatCents::new(self.amount)),
                Currency::Millisats => None,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    #[serde(rename_all = "snake_case")]
    pub struct RecurringPaymentResponseContent {
        pub request_id: String,
        pub status: RecurringPaymentStatus,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct ExchangeRate {
        pub rate: f64,
        pub source: String,
        pub time: Timestamp,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct RecurrenceInfo {
        pub until: Option<Timestamp>,
        pub calendar: CalendarWrapper,
        pub max_payments: Option<u32>,
        pub first_payment_due: Timestamp,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
    #[serde(rename_all = "snake_case", tag = "status")]
    pub enum RecurringPaymentStatus {
        Confirmed {
            subscription_id: String,
            authorized_amount: u64,
            authorized_currency: Currency,
            authorized_recurrence: RecurrenceInfo,
        },
        Rejected {
            reason: Option<String>,
        },
        /* Use RecurringPaymentStatusSenderConversation
        Cancelled {
            subscription_id: String,
            reason: Option<String>,
        },
        */
    }

    impl RecurringPaymentStatus {
        pub fn authorized_amount_millisats(&self) -> Option<Millisats> {
            match self {
                Self::Confirmed {
                    authorized_amount,
                    authorized_currency: Currency::Millisats,
                    ..
                } => Some(Millisats::new(*authorized_amount)),
                _ => None,
            }
        }

        pub fn authorized_amount_fiat_cents(&self) -> Option<FiatCents> {
            match self {
                Self::Confirmed {
                    authorized_amount,
                    authorized_currency: Currency::Fiat(_),
                    ..
                } => Some(FiatCents::new(*authorized_amount)),
                _ => None,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct CloseRecurringPaymentContent {
        pub subscription_id: String,
        pub reason: Option<String>,
        pub by_service: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct CloseRecurringPaymentResponse {
        pub content: CloseRecurringPaymentContent,
        pub main_key: PublicKey,
        pub recipient: PublicKey,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct InvoiceRequestContent {
        pub request_id: String,
        pub amount: u64,
        pub currency: Currency,
        pub current_exchange_rate: Option<ExchangeRate>,
        pub expires_at: Timestamp,
        pub description: Option<String>,
        pub refund_invoice: Option<String>,
    }

    impl InvoiceRequestContent {
        pub fn amount_millisats(&self) -> Option<Millisats> {
            match self.currency {
                Currency::Millisats => Some(Millisats::new(self.amount)),
                Currency::Fiat(_) => None,
            }
        }

        pub fn amount_fiat_cents(&self) -> Option<FiatCents> {
            match self.currency {
                Currency::Fiat(_) => Some(FiatCents::new(self.amount)),
                Currency::Millisats => None,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct InvoiceRequestContentWithKey {
        pub inner: InvoiceRequestContent,
        pub main_key: PublicKey,
        pub recipient: PublicKey,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct InvoiceResponse {
        pub request: InvoiceRequestContentWithKey,
        pub invoice: String,
        pub payment_hash: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct CashuRequestContent {
        pub request_id: String,
        pub mint_url: String,
        pub unit: String,
        pub amount: u64,
        pub expires_at: Timestamp,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct CashuRequestContentWithKey {
        pub inner: CashuRequestContent,
        pub main_key: PublicKey,
        pub recipient: PublicKey,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct CashuResponseContent {
        pub request: CashuRequestContentWithKey,
        pub status: CashuResponseStatus,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
    #[serde(rename_all = "snake_case", tag = "status")]
    pub enum CashuResponseStatus {
        Success { token: String },
        InsufficientFunds,
        Rejected { reason: Option<String> },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct CashuDirectContent {
        pub token: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct CashuDirectContentWithKey {
        pub inner: CashuDirectContent,
        pub main_key: PublicKey,
        pub recipient: PublicKey,
    }
}

pub mod nip46 {
    use serde::{Deserialize, Serialize};

    use crate::protocol::model::bindings::{NostrConnectMessage, PublicKey};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct NostrConnectEvent {
        pub nostr_client_pubkey: PublicKey,
        pub message: NostrConnectMessage,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
    #[serde(rename_all = "snake_case", tag = "status")]
    pub enum NostrConnectResponseStatus {
        Approved,
        Declined { reason: Option<String> },
    }
}

#[cfg(feature = "bindings")]
pub mod bindings {
    use nostr::nips::{
        nip19::ToBech32,
        nip46::{NostrConnectMessage as CoreMessage, NostrConnectMethod as CoreMethod},
    };
    use serde::{Deserialize, Serialize};
    use std::ops::Deref;

    use super::*;

    #[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub struct PublicKey(pub nostr::PublicKey);

    uniffi::custom_type!(PublicKey, String, {
        try_lift: |val| Ok(PublicKey(nostr::PublicKey::parse(&val)?)),
        lower: |obj| obj.0.to_bech32().unwrap(),
    });

    impl From<nostr::PublicKey> for PublicKey {
        fn from(key: nostr::PublicKey) -> Self {
            PublicKey(key)
        }
    }
    impl Into<nostr::PublicKey> for PublicKey {
        fn into(self) -> nostr::PublicKey {
            self.0
        }
    }

    impl Deref for PublicKey {
        type Target = nostr::PublicKey;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    uniffi::custom_type!(Nonce, String, {
        try_lift: |val| Ok(Nonce(hex::decode(&val)?.try_into().map_err(|_| anyhow::anyhow!("Invalid nonce length"))?)),
        lower: |obj| hex::encode(obj.0),
    });
    uniffi::custom_type!(Timestamp, u64, {
        try_lift: |val| Ok(Timestamp(val)),
        lower: |obj| obj.0,
    });

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
    pub enum NostrConnectMethod {
        Connect,
        GetPublicKey,
        SignEvent,
        Nip04Encrypt,
        Nip04Decrypt,
        Nip44Encrypt,
        Nip44Decrypt,
        Ping,
    }

    impl From<CoreMethod> for NostrConnectMethod {
        fn from(value: CoreMethod) -> Self {
            match value {
                CoreMethod::Connect => Self::Connect,
                CoreMethod::GetPublicKey => Self::GetPublicKey,
                CoreMethod::SignEvent => Self::SignEvent,
                CoreMethod::Nip04Encrypt => Self::Nip04Encrypt,
                CoreMethod::Nip04Decrypt => Self::Nip04Decrypt,
                CoreMethod::Nip44Encrypt => Self::Nip44Encrypt,
                CoreMethod::Nip44Decrypt => Self::Nip44Decrypt,
                CoreMethod::Ping => Self::Ping,
            }
        }
    }

    impl From<NostrConnectMethod> for CoreMethod {
        fn from(value: NostrConnectMethod) -> Self {
            match value {
                NostrConnectMethod::Connect => Self::Connect,
                NostrConnectMethod::GetPublicKey => Self::GetPublicKey,
                NostrConnectMethod::SignEvent => Self::SignEvent,
                NostrConnectMethod::Nip04Encrypt => Self::Nip04Encrypt,
                NostrConnectMethod::Nip04Decrypt => Self::Nip04Decrypt,
                NostrConnectMethod::Nip44Encrypt => Self::Nip44Encrypt,
                NostrConnectMethod::Nip44Decrypt => Self::Nip44Decrypt,
                NostrConnectMethod::Ping => Self::Ping,
            }
        }
    }

    #[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
    pub struct NostrConnectRequest {
        pub id: String,
        pub method: NostrConnectMethod,
        pub params: Vec<String>,
    }

    #[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
    pub struct NostrConnectResponse {
        pub id: String,
        pub result: Option<String>,
        pub error: Option<String>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
    pub enum NostrConnectMessage {
        Request(NostrConnectRequest),
        Response(NostrConnectResponse),
    }

    impl From<CoreMessage> for NostrConnectMessage {
        fn from(message: nostr::nips::nip46::NostrConnectMessage) -> Self {
            match message {
                CoreMessage::Request { id, method, params } => {
                    Self::Request(NostrConnectRequest {
                        id,
                        method: method.into(),
                        params,
                    })
                }
                CoreMessage::Response { id, result, error } => {
                    Self::Response(NostrConnectResponse { id, result, error })
                }
            }
        }
    }

    impl From<NostrConnectMessage> for nostr::nips::nip46::NostrConnectMessage {
        fn from(repr: NostrConnectMessage) -> Self {
            match repr {
                NostrConnectMessage::Request(request) => {
                    nostr::nips::nip46::NostrConnectMessage::Request {
                        id: request.id,
                        method: request.method.into(),
                        params: request.params,
                    }
                }
                NostrConnectMessage::Response(response) => {
                    nostr::nips::nip46::NostrConnectMessage::Response {
                        id: response.id,
                        result: response.result,
                        error: response.error,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[test]
fn test_serialization() {
    let c = payment::Currency::Millisats;
    let s = serde_json::to_string(&c).unwrap();
    println!("{}", s);
    let c2: payment::Currency = serde_json::from_str(&s).unwrap();
    assert_eq!(c, c2);

    let c = payment::Currency::Fiat("USD".to_string());
    let s = serde_json::to_string(&c).unwrap();
    println!("{}", s);
    let c2: payment::Currency = serde_json::from_str(&s).unwrap();
    assert_eq!(c, c2);
}
