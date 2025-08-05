use portal::profile::Profile;
use portal::protocol::model::auth::AuthResponseStatus;
use portal::protocol::model::payment::{CashuResponseStatus, RecurringPaymentResponseContent};
use serde::Serialize;

// Response structs for each API
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum Response {
    #[serde(rename = "error")]
    Error { id: String, message: String },

    #[serde(rename = "success")]
    Success { id: String, data: ResponseData },

    #[serde(rename = "notification")]
    Notification { id: String, data: NotificationData },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ResponseData {
    #[serde(rename = "auth_success")]
    AuthSuccess { message: String },

    #[serde(rename = "key_handshake_url")]
    KeyHandshakeUrl { url: String, stream_id: String },

    #[serde(rename = "auth_response")]
    AuthResponse { event: AuthResponseData },

    #[serde(rename = "recurring_payment")]
    RecurringPayment {
        status: RecurringPaymentResponseContent,
    },

    #[serde(rename = "single_payment")]
    SinglePayment { stream_id: String },

    #[serde(rename = "profile")]
    ProfileData { profile: Option<Profile> },

    #[serde(rename = "close_recurring_payment_success")]
    CloseRecurringPaymentSuccess { message: String },

    #[serde(rename = "listen_closed_recurring_payment")]
    ListenClosedRecurringPayment { stream_id: String },

    #[serde(rename = "invoice_payment")]
    InvoicePayment {
        invoice: String,
        payment_hash: String,
    },

    #[serde(rename = "issue_jwt")]
    IssueJwt { token: String },

    #[serde(rename = "verify_jwt")]
    VerifyJwt { target_key: String },

    #[serde(rename = "cashu_response")]
    CashuResponse { status: CashuResponseStatus },

    #[serde(rename = "send_cashu_direct_success")]
    SendCashuDirectSuccess { message: String },

    #[serde(rename = "cashu_mint")]
    CashuMint { token: String },

    #[serde(rename = "cashu_burn")]
    CashuBurn { amount: u64 },
}

#[derive(Debug, Serialize)]
pub struct AuthResponseData {
    pub user_key: String,
    pub recipient: String,
    pub challenge: String,
    pub status: AuthResponseStatus,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum NotificationData {
    #[serde(rename = "key_handshake")]
    KeyHandshake {
        main_key: String,
        preferred_relays: Vec<String>,
    },
    #[serde(rename = "payment_status_update")]
    PaymentStatusUpdate { status: InvoiceStatus },
    #[serde(rename = "closed_recurring_payment")]
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
