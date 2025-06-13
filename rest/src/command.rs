use portal::profile::Profile;
use portal::protocol::model::payment::{
    Currency, RecurringPaymentRequestContent, SinglePaymentRequestContent,
};
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
    NewAuthInitUrl,
    AuthenticateKey {
        main_key: String,
        subkeys: Vec<String>,
    },
    RequestRecurringPayment {
        main_key: String,
        subkeys: Vec<String>,
        payment_request: RecurringPaymentRequestContent,
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
}

#[derive(Debug, Deserialize)]
pub struct SinglePaymentParams {
    pub description: String,
    pub amount: u64,
    pub currency: Currency,
    pub subscription_id: Option<String>,
    pub auth_token: Option<String>,
}
