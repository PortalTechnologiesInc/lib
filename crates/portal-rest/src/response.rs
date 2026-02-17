use nostr::nips::nip05::Nip05Profile;

use portal::conversation::profile::Profile;
use portal::protocol::model::auth::AuthResponseStatus;
use portal::protocol::model::payment::{CashuResponseStatus, RecurringPaymentResponseContent};
use portal::protocol::model::Timestamp;
use serde::Serialize;

// Response structs for each API
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Error { id: String, message: String },

    Success { id: String, data: ResponseData },

    Notification { id: String, data: NotificationData },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseData {
    AuthSuccess {
        message: String,
    },

    KeyHandshakeUrl {
        url: String,
        stream_id: String,
    },

    AuthResponse {
        event: AuthResponseData,
    },

    RecurringPayment {
        status: RecurringPaymentResponseContent,
    },

    SinglePayment {
        stream_id: String,
    },

    #[serde(rename = "profile")]
    ProfileData {
        profile: Option<Profile>,
    },

    CloseRecurringPaymentSuccess {
        message: String,
    },

    ListenClosedRecurringPayment {
        stream_id: String,
    },

    InvoicePayment {
        invoice: String,
        payment_hash: Option<String>,
    },

    IssueJwt {
        token: String,
    },

    VerifyJwt {
        target_key: String,
    },

    CashuResponse {
        status: CashuResponseStatus,
    },

    SendCashuDirectSuccess {
        message: String,
    },

    CashuMint {
        token: String,
    },

    CashuBurn {
        amount: u64,
    },

    AddRelay {
        relay: String,
    },

    RemoveRelay {
        relay: String,
    },

    CalculateNextOccurrence {
        next_occurrence: Option<Timestamp>,
    },

    FetchNip05Profile {
        profile: Nip05Profile,
    },

    WalletInfo {
        wallet_type: String,
        balance_msat: u64,
    },
}

#[derive(Debug, Serialize)]
pub struct AuthResponseData {
    pub user_key: String,
    pub recipient: String,
    pub challenge: String,
    pub status: AuthResponseStatus,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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
