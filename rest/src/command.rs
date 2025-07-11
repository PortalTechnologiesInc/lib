use std::borrow::Cow;
use portal::profile::Profile;
use portal::protocol::model::payment::{
    Currency, InvoiceRequestContent, InvoiceRequestContentWithKey, RecurringPaymentRequestContent,
    SinglePaymentRequestContent,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CommandWithId<'a> {
    #[serde(borrow)]
    pub id: Cow<'a, str>,
    #[serde(flatten)]
    pub cmd: Command<'a>,
}

impl<'a> CommandWithId<'a> {
    pub fn into_owned(self) -> OwnedCommandWithId {
        OwnedCommandWithId {
            id: Cow::Owned(self.id.into_owned()),
            cmd: self.cmd.into_owned(),
        }
    }

    pub fn parse(input: &'a str) -> serde_json::error::Result<CommandWithId<'a>> {
        serde_json::from_str(input)
    }
    
}

// Commands that can be sent from client to server
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", content = "params")]
pub enum Command<'a> {
    // Authentication command - must be first command sent
    Auth {
        #[serde(borrow)]
        token: Cow<'a, str>,
    },

    // SDK methods
    NewKeyHandshakeUrl {
        static_token: Option<Cow<'a, str>>,
    },
    AuthenticateKey {
        #[serde(borrow)]
        main_key: Cow<'a, str>,
        #[serde(borrow)]
        subkeys: Vec<Cow<'a, str>>,
    },
    RequestRecurringPayment {
        #[serde(borrow)]
        main_key: Cow<'a, str>,
        #[serde(borrow)]
        subkeys: Vec<Cow<'a, str>>,
        payment_request: RecurringPaymentRequestContent,
    },
    RequestSinglePayment {
        #[serde(borrow)]
        main_key: Cow<'a, str>,
        #[serde(borrow)]
        subkeys: Vec<Cow<'a, str>>,
        payment_request: SinglePaymentParams<'a>,
    },
    RequestPaymentRaw {
        #[serde(borrow)]
        main_key: Cow<'a, str>,
        #[serde(borrow)]
        subkeys: Vec<Cow<'a, str>>,
        payment_request: SinglePaymentRequestContent,
    },
    FetchProfile {
        #[serde(borrow)]
        main_key: Cow<'a, str>,
    },
    SetProfile {
        profile: Profile,
    },
    CloseRecurringPayment {
        #[serde(borrow)]
        main_key: Cow<'a, str>,
        #[serde(borrow)]
        subkeys: Vec<Cow<'a, str>>,
        #[serde(borrow)]
        subscription_id: Cow<'a, str>,
    },
    ListenClosedRecurringPayment,
    RequestInvoice {
        #[serde(borrow)]
        recipient_key: Cow<'a, str>,
        #[serde(borrow)]
        subkeys: Vec<Cow<'a, str>>,
        content: InvoiceRequestContent,
    },
}

#[derive(Debug, Deserialize)]
pub struct SinglePaymentParams<'a> {
    #[serde(borrow)]
    pub description: Cow<'a, str>,
    pub amount: u64,
    pub currency: Currency,
    #[serde(borrow)]
    pub subscription_id: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub auth_token: Option<Cow<'a, str>>,
}

// Type aliases for backward compatibility and convenience
pub type OwnedCommandWithId = CommandWithId<'static>;
pub type OwnedCommand = Command<'static>;
pub type OwnedSinglePaymentParams = SinglePaymentParams<'static>;

// Helper trait for converting borrowed commands to owned
pub trait IntoOwned<T> {
    fn into_owned(self) -> T;
}

impl<'a> IntoOwned<OwnedCommandWithId> for CommandWithId<'a> {
    fn into_owned(self) -> OwnedCommandWithId {
        OwnedCommandWithId {
            id: Cow::Owned(self.id.into_owned()),
            cmd: self.cmd.into_owned(),
        }
    }
}

impl<'a> IntoOwned<OwnedCommand> for Command<'a> {
    fn into_owned(self) -> OwnedCommand {
        match self {
            Command::Auth { token } => Command::Auth {
                token: Cow::Owned(token.into_owned()),
            },
            Command::NewKeyHandshakeUrl { static_token } => Command::NewKeyHandshakeUrl {
                static_token: static_token.map(|s| Cow::Owned(s.into_owned())),
            },
            Command::AuthenticateKey { main_key, subkeys } => Command::AuthenticateKey {
                main_key: Cow::Owned(main_key.into_owned()),
                subkeys: subkeys.into_iter().map(|s| Cow::Owned(s.into_owned())).collect(),
            },
            Command::RequestRecurringPayment {
                main_key,
                subkeys,
                payment_request,
            } => Command::RequestRecurringPayment {
                main_key: Cow::Owned(main_key.into_owned()),
                subkeys: subkeys.into_iter().map(|s| Cow::Owned(s.into_owned())).collect(),
                payment_request,
            },
            Command::RequestSinglePayment {
                main_key,
                subkeys,
                payment_request,
            } => Command::RequestSinglePayment {
                main_key: Cow::Owned(main_key.into_owned()),
                subkeys: subkeys.into_iter().map(|s| Cow::Owned(s.into_owned())).collect(),
                payment_request: payment_request.into_owned(),
            },
            Command::RequestPaymentRaw {
                main_key,
                subkeys,
                payment_request,
            } => Command::RequestPaymentRaw {
                main_key: Cow::Owned(main_key.into_owned()),
                subkeys: subkeys.into_iter().map(|s| Cow::Owned(s.into_owned())).collect(),
                payment_request,
            },
            Command::FetchProfile { main_key } => Command::FetchProfile {
                main_key: Cow::Owned(main_key.into_owned()),
            },
            Command::SetProfile { profile } => Command::SetProfile { profile },
            Command::CloseRecurringPayment {
                main_key,
                subkeys,
                subscription_id,
            } => Command::CloseRecurringPayment {
                main_key: Cow::Owned(main_key.into_owned()),
                subkeys: subkeys.into_iter().map(|s| Cow::Owned(s.into_owned())).collect(),
                subscription_id: Cow::Owned(subscription_id.into_owned()),
            },
            Command::ListenClosedRecurringPayment => Command::ListenClosedRecurringPayment,
            Command::RequestInvoice {
                recipient_key,
                subkeys,
                content,
            } => Command::RequestInvoice {
                recipient_key: Cow::Owned(recipient_key.into_owned()),
                subkeys: subkeys.into_iter().map(|s| Cow::Owned(s.into_owned())).collect(),
                content,
            },
        }
    }
}

impl<'a> IntoOwned<OwnedSinglePaymentParams> for SinglePaymentParams<'a> {
    fn into_owned(self) -> OwnedSinglePaymentParams {
        OwnedSinglePaymentParams {
            description: Cow::Owned(self.description.into_owned()),
            amount: self.amount,
            currency: self.currency,
            subscription_id: self.subscription_id.map(|s| Cow::Owned(s.into_owned())),
            auth_token: self.auth_token.map(|s| Cow::Owned(s.into_owned())),
        }
    }
}

