use nostr::nips::nip05::Nip05Profile;

use portal::conversation::profile::Profile;
use portal::protocol::model::auth::AuthResponseStatus;
use portal::protocol::model::payment::{CashuResponseStatus, RecurringPaymentResponseContent};
use portal::protocol::model::Timestamp;
use serde::Serialize;

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

#[derive(Debug, Serialize)]
pub struct AuthResponseData {
    pub user_key: String,
    pub recipient: String,
    pub challenge: String,
    pub status: AuthResponseStatus,
}

#[derive(Debug, Serialize)]
pub struct AuthKeyResponse {
    pub event: AuthResponseData,
}

#[derive(Debug, Serialize)]
pub struct RecurringPaymentResponse {
    pub status: RecurringPaymentResponseContent,
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
pub struct ListenClosedRecurringPaymentResponse {
    pub stream_id: String,
}

#[derive(Debug, Serialize)]
pub struct InvoicePaymentResponse {
    pub invoice: String,
    pub payment_hash: Option<String>,
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
pub struct CashuResponse {
    pub status: CashuResponseStatus,
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
pub struct VersionResponse {
    pub version: &'static str,
    pub git_commit: &'static str,
}

// ---- Event / notification types (stored for polling, sent via webhook) ----

#[derive(Debug, Clone, Serialize)]
pub struct StreamEvent {
    /// Monotonically increasing index within this stream.
    pub index: u64,
    /// ISO-8601 timestamp of when the event was created.
    pub timestamp: String,
    #[serde(flatten)]
    pub data: NotificationData,
}

#[derive(Debug, Clone, Serialize)]
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
}

#[derive(Debug, Clone, Serialize)]
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
