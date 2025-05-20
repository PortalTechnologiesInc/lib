use portal::profile::Profile;
use portal::protocol::model::payment::{PaymentResponseContent, RecurringPaymentResponseContent};
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

    #[serde(rename = "auth_init_url")]
    AuthInitUrl { url: String, stream_id: String },

    #[serde(rename = "auth_response")]
    AuthResponse { event: AuthResponseData },

    #[serde(rename = "recurring_payment")]
    RecurringPayment {
        status: RecurringPaymentResponseContent,
    },

    #[serde(rename = "single_payment")]
    SinglePayment {
        status: PaymentResponseContent,
        stream_id: Option<String>,
    },

    #[serde(rename = "profile")]
    ProfileData { profile: Option<Profile> },
}

#[derive(Debug, Serialize)]
pub struct AuthResponseData {
    pub user_key: String,
    pub recipient: String,
    pub challenge: String,
    pub granted_permissions: Vec<String>,
    pub session_token: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum NotificationData {
    #[serde(rename = "auth_init")]
    AuthInit { main_key: String },
    #[serde(rename = "payment_status_update")]
    PaymentStatusUpdate { status: InvoiceStatus },
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum InvoiceStatus {
    Paid { preimage: Option<String> },
    Timeout,
    Error { reason: String },
}
