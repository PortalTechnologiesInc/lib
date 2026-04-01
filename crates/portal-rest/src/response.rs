use nostr::nips::nip05::Nip05Profile;

use portal::conversation::profile::Profile;
use portal::protocol::model::auth::AuthResponseStatus;
use portal::protocol::model::payment::{
    CashuResponseStatus, RecurringPaymentResponseContent,
};
use portal::protocol::model::Timestamp;
use serde::{Deserialize, Serialize};

/// Generic API response wrapper used for all REST endpoints.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

impl ApiResponse<()> {
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

// ---- Data types returned by endpoints ----

#[derive(Debug, Serialize)]
pub struct KeyHandshakeUrlResponse {
    pub url: String,
    pub stream_id: String,
}

/// Generic response for async stream-based endpoints.
#[derive(Debug, Serialize)]
pub struct StreamResponse {
    pub stream_id: String,
}

#[derive(Debug, Serialize)]
pub struct SinglePaymentResponse {
    pub stream_id: String,
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub profile: Option<Profile>,
}

#[derive(Debug, Serialize)]
pub struct CloseRecurringPaymentResponse {
    pub message: String,
}



#[derive(Debug, Serialize)]
pub struct IssueJwtResponse {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyJwtResponse {
    pub target_key: String,
}



#[derive(Debug, Serialize)]
pub struct SendCashuDirectResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CashuMintResponse {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct CashuBurnResponse {
    pub amount: u64,
}

#[derive(Debug, Serialize)]
pub struct RelayResponse {
    pub relay: String,
}

#[derive(Debug, Serialize)]
pub struct NextOccurrenceResponse {
    pub next_occurrence: Option<Timestamp>,
}

#[derive(Debug, Serialize)]
pub struct PayInvoiceResponse {
    pub preimage: String,
    pub fees_paid_msat: u64,
}

#[derive(Debug, Serialize)]
pub struct Nip05ProfileResponse {
    pub profile: Nip05Profile,
}

#[derive(Debug, Serialize)]
pub struct WalletInfoResponse {
    pub wallet_type: String,
    pub balance_msat: u64,
}

#[derive(Debug, Serialize)]
pub struct VerificationSessionResponse {
    pub session_id: String,
    pub session_url: String,
    pub ephemeral_npub: String,
    pub expires_at: u64,
    pub stream_id: String,
}

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub version: &'static str,
    pub git_commit: &'static str,
}

#[derive(Debug, Serialize)]
pub struct InfoResponse {
    pub public_key: String,
    pub version: &'static str,
    pub git_commit: &'static str,
}

/// NIP-05 `.well-known/nostr.json` content.
#[derive(Debug, Serialize)]
pub struct Nip05WellKnownResponse {
    pub names: std::collections::HashMap<String, String>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub relays: std::collections::HashMap<String, Vec<String>>,
}

// ---- Event / notification types (stored for polling, sent via webhook) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Monotonically increasing index within this stream.
    pub index: u64,
    /// ISO-8601 timestamp of when the event was created.
    pub timestamp: String,
    #[serde(flatten)]
    pub data: NotificationData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationData {
    KeyHandshake {
        main_key: String,
        preferred_relays: Vec<String>,
    },
    PaymentStatusUpdate {
        status: InvoiceStatus,
    },
    ClosedRecurringPayment {
        reason: Option<String>,
        subscription_id: String,
        recipient: String,
        main_key: String,
    },
    AuthenticateKey {
        user_key: String,
        recipient: String,
        challenge: String,
        status: AuthResponseStatus,
    },
    RecurringPaymentResponse {
        status: RecurringPaymentResponseContent,
    },
    InvoiceResponse {
        invoice: String,
        payment_hash: String,
    },
    CashuResponse {
        status: CashuResponseStatus,
    },
    Error {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum InvoiceStatus {
    Paid { preimage: Option<String> },
    Timeout,
    Error { reason: String },
    UserApproved,
    UserSuccess { preimage: Option<String> },
    UserFailed { reason: Option<String> },
    UserRejected { reason: Option<String> },
}

/// Events polling response.
#[derive(Debug, Serialize)]
pub struct EventsResponse {
    pub stream_id: String,
    pub events: Vec<StreamEvent>,
}
