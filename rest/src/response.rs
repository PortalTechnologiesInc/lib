use portal::profile::Profile;
use portal::protocol::model::auth::AuthResponseStatus;
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

    #[serde(rename = "close_recurring_payment_success")]
    CloseRecurringPaymentSuccess { message: String },

    #[serde(rename = "listen_closed_recurring_payment")]
    ListenClosedRecurringPayment { stream_id: String },

    #[serde(rename = "invoice_payment")]
    InvoicePayment {
        invoice: String,
        payment_hash: String,
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
#[serde(tag = "type")]
pub enum NotificationData {
    #[serde(rename = "auth_init")]
    AuthInit { main_key: String },
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
}
