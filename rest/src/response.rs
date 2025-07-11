use std::borrow::Cow;
use portal::profile::Profile;
use portal::protocol::model::auth::AuthResponseStatus;
use portal::protocol::model::payment::{PaymentResponseContent, RecurringPaymentResponseContent};
use serde::Serialize;

// Response structs for each API
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum Response {
    #[serde(rename = "error")]
    Error { 
        id: Cow<'static, str>, 
        message: Cow<'static, str> 
    },

    #[serde(rename = "success")]
    Success { 
        id: Cow<'static, str>, 
        data: ResponseData 
    },

    #[serde(rename = "notification")]
    Notification { 
        id: Cow<'static, str>, 
        data: NotificationData 
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ResponseData {
    #[serde(rename = "auth_success")]
    AuthSuccess { 
        message: Cow<'static, str> 
    },

    #[serde(rename = "key_handshake_url")]
    KeyHandshakeUrl { 
        url: Cow<'static, str>, 
        stream_id: Cow<'static, str> 
    },

    #[serde(rename = "auth_response")]
    AuthResponse { event: AuthResponseData },

    #[serde(rename = "recurring_payment")]
    RecurringPayment {
        status: RecurringPaymentResponseContent,
    },

    #[serde(rename = "single_payment")]
    SinglePayment {
        status: PaymentResponseContent,
        stream_id: Option<Cow<'static, str>>,
    },

    #[serde(rename = "profile")]
    ProfileData { profile: Option<Profile> },

    #[serde(rename = "close_recurring_payment_success")]
    CloseRecurringPaymentSuccess { 
        message: Cow<'static, str> 
    },

    #[serde(rename = "listen_closed_recurring_payment")]
    ListenClosedRecurringPayment { 
        stream_id: Cow<'static, str> 
    },

    #[serde(rename = "invoice_payment")]
    InvoicePayment {
        invoice: Cow<'static, str>,
        payment_hash: Cow<'static, str>,
    },
}

#[derive(Debug, Serialize)]
pub struct AuthResponseData {
    pub user_key: Cow<'static, str>,
    pub recipient: Cow<'static, str>,
    pub challenge: Cow<'static, str>,
    pub status: AuthResponseStatus,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum NotificationData {
    #[serde(rename = "key_handshake")]
    KeyHandshake { 
        main_key: Cow<'static, str> 
    },
    #[serde(rename = "payment_status_update")]
    PaymentStatusUpdate { status: InvoiceStatus },
    #[serde(rename = "closed_recurring_payment")]
    ClosedRecurringPayment {
        reason: Option<Cow<'static, str>>,
        subscription_id: Cow<'static, str>,
        recipient: Cow<'static, str>,
        main_key: Cow<'static, str>,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum InvoiceStatus {
    Paid { 
        preimage: Option<Cow<'static, str>> 
    },
    Timeout,
    Error { 
        reason: Cow<'static, str> 
    },
}


