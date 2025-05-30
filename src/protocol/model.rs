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
    // Authentication events (27000-27999)
    pub const AUTH_CHALLENGE: u16 = 27000;
    pub const AUTH_RESPONSE: u16 = 27001;
    pub const AUTH_SUCCESS: u16 = 27002;
    pub const AUTH_INIT: u16 = 27010;

    // Payment events (28000-28999)
    pub const PAYMENT_REQUEST: u16 = 28000;
    pub const PAYMENT_RESPONSE: u16 = 28001;
    pub const PAYMENT_CONFIRMATION: u16 = 28002;
    pub const PAYMENT_ERROR: u16 = 28003;
    pub const PAYMENT_RECEIPT: u16 = 28004;
    pub const RECURRING_PAYMENT_REQUEST: u16 = 28005;
    pub const RECURRING_PAYMENT_RESPONSE: u16 = 28006;
    pub const RECURRING_PAYMENT_CANCEL: u16 = 28007;

    // Identity events (29000-29999)
    pub const CERTIFICATE_REQUEST: u16 = 29000;
    pub const CERTIFICATE_RESPONSE: u16 = 29001;
    pub const CERTIFICATE_ERROR: u16 = 29002;
    pub const CERTIFICATE_REVOCATION: u16 = 29003;
    pub const CERTIFICATE_VERIFY_REQUEST: u16 = 29004;
    pub const CERTIFICATE_VERIFY_RESPONSE: u16 = 29005;

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
    pub struct AuthInitContent {
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
        pub granted_permissions: Vec<String>,
        pub session_token: String,
        pub subkey_proof: Option<SubkeyProof>,
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
    #[serde(rename_all = "snake_case")]
    pub enum PaymentStatus {
        Pending,
        Rejected { reason: Option<String> },
        Failed { reason: Option<String> },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
    pub enum Currency {
        Millisats,
        Fiat(String),
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "bindings", derive(uniffi::Record))]
    pub struct CloseRecurringPaymentContent {
        pub subscription_id: String,
        pub reason: Option<String>,
    }
}

#[cfg(feature = "bindings")]
pub mod bindings {
    use nostr::nips::nip19::ToBech32;
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
}
