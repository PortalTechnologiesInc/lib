use portal::conversation::profile::Profile;
use portal::protocol::model::payment::{
    Currency, InvoiceRequestContent, RecurrenceInfo,
    SinglePaymentRequestContent,
};
use portal::protocol::model::Timestamp;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CommandWithId {
    pub id: String,
    #[serde(flatten)]
    pub cmd: Command,
}

// Commands that can be sent from client to server
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", content = "params")]
pub enum Command {
    // Authentication command - must be first command sent
    Auth {
        token: String,
    },

    // SDK methods
    NewKeyHandshakeUrl {
        static_token: Option<String>,
        no_request: Option<bool>,
    },
    AuthenticateKey {
        main_key: String,
        subkeys: Vec<String>,
    },
    RequestRecurringPayment {
        main_key: String,
        subkeys: Vec<String>,
        payment_request: RecurringPaymentParams,
    },
    RequestSinglePayment {
        main_key: String,
        subkeys: Vec<String>,
        payment_request: SinglePaymentParams,
    },
    RequestPaymentRaw {
        main_key: String,
        subkeys: Vec<String>,
        payment_request: SinglePaymentRequestContent,
    },
    FetchProfile {
        main_key: String,
    },
    SetProfile {
        profile: Profile,
    },
    CloseRecurringPayment {
        main_key: String,
        subkeys: Vec<String>,
        subscription_id: String,
    },
    ListenClosedRecurringPayment,
    RequestInvoice {
        recipient_key: String,
        subkeys: Vec<String>,
        content: InvoiceRequestContent,
    },
    IssueJwt {
        target_key: String,
        duration_hours: i64,
    },
    VerifyJwt {
        pubkey: String,
        token: String,
    },
    RequestCashu {
        recipient_key: String,
        subkeys: Vec<String>,
        mint_url: String,
        unit: String,
        amount: u64,
    },
    SendCashuDirect {
        main_key: String,
        subkeys: Vec<String>,
        token: String,
    },
    MintCashu {
        mint_url: String,
        unit: String,
        static_auth_token: Option<String>,
        amount: u64,
        description: Option<String>,
    },
    BurnCashu {
        mint_url: String,
        unit: String,
        static_auth_token: Option<String>,
        token: String,
    },
    AddRelay {
        relay: String,
    },
    RemoveRelay {
        relay: String,
    },
    CalculateNextOccurrence {
        calendar: String,
        from: Timestamp,
    },
    PayInvoice {
        invoice: String,
    },
    FetchNip05Profile {
        nip05: String,
    },
    GetWalletInfo,
    CreateWebVerificationSession {
        relay_urls: Option<Vec<String>>,
    },
    RequestToken {
        npub: String,
        amount: Option<u64>,
        relays: Option<Vec<String>>,
    },
}

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
