use portal::conversation::profile::Profile;
use portal::protocol::model::payment::{
    Currency, RecurrenceInfo, SinglePaymentRequestContent,
};
use portal::protocol::model::Timestamp;
use serde::Deserialize;

// ---- REST request bodies ----

#[derive(Debug, Deserialize)]
pub struct KeyHandshakeRequest {
    pub static_token: Option<String>,
    pub no_request: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AuthenticateKeyRequest {
    pub main_key: String,
    pub subkeys: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RequestRecurringPaymentRequest {
    pub main_key: String,
    pub subkeys: Vec<String>,
    pub payment_request: RecurringPaymentParams,
}

#[derive(Debug, Deserialize)]
pub struct RequestSinglePaymentRequest {
    pub main_key: String,
    pub subkeys: Vec<String>,
    pub payment_request: SinglePaymentParams,
}

#[derive(Debug, Deserialize)]
pub struct RequestPaymentRawRequest {
    pub main_key: String,
    pub subkeys: Vec<String>,
    pub payment_request: SinglePaymentRequestContent,
}

#[derive(Debug, Deserialize)]
pub struct SetProfileRequest {
    pub profile: Profile,
}

#[derive(Debug, Deserialize)]
pub struct CloseRecurringPaymentRequest {
    pub main_key: String,
    pub subkeys: Vec<String>,
    pub subscription_id: String,
}

#[derive(Debug, Deserialize)]
pub struct RequestInvoiceRequest {
    pub recipient_key: String,
    pub subkeys: Vec<String>,
    pub content: RequestInvoiceParams,
}

#[derive(Debug, Deserialize)]
pub struct IssueJwtRequest {
    pub target_key: String,
    pub duration_hours: i64,
}

#[derive(Debug, Deserialize)]
pub struct VerifyJwtRequest {
    pub pubkey: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct RequestCashuRequest {
    pub recipient_key: String,
    pub subkeys: Vec<String>,
    pub mint_url: String,
    pub unit: String,
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
pub struct SendCashuDirectRequest {
    pub main_key: String,
    pub subkeys: Vec<String>,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct MintCashuRequest {
    pub mint_url: String,
    pub unit: String,
    pub static_auth_token: Option<String>,
    pub amount: u64,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BurnCashuRequest {
    pub mint_url: String,
    pub unit: String,
    pub static_auth_token: Option<String>,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct RelayRequest {
    pub relay: String,
}

#[derive(Debug, Deserialize)]
pub struct CalculateNextOccurrenceRequest {
    pub calendar: String,
    pub from: Timestamp,
}

#[derive(Debug, Deserialize)]
pub struct PayInvoiceRequest {
    pub invoice: String,
}

// ---- Shared param types ----

#[derive(Debug, Deserialize)]
pub struct SinglePaymentParams {
    pub description: String,
    pub amount: u64,
    pub currency: Currency,
    pub auth_token: Option<String>,

    pub subscription_id: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecurringPaymentParams {
    pub description: Option<String>,
    pub amount: u64,
    pub currency: Currency,
    pub auth_token: Option<String>,

    pub recurrence: RecurrenceInfo,
    pub expires_at: Timestamp,
}


#[derive(Debug, Deserialize)]
pub struct RequestInvoiceParams {
    pub amount: u64,
    pub currency: Currency,
    pub expires_at: Timestamp,
    pub description: Option<String>,
    pub refund_invoice: Option<String>,
    /// Optional request ID. If not provided, a UUID is generated.
    pub request_id: Option<String>,
}
