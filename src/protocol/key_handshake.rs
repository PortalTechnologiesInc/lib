use std::{fmt, str::FromStr};

use nostr::nips::nip19::{FromBech32, ToBech32};
use thiserror::Error;

use super::model::bindings::PublicKey;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "bindings", derive(uniffi::Record))]
pub struct KeyHandshakeUrl {
    pub main_key: PublicKey,
    pub relays: Vec<String>,
    pub token: String,
    pub subkey: Option<PublicKey>,
    pub no_request: bool,
}

impl fmt::Display for KeyHandshakeUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let relays = self
            .relays
            .iter()
            .map(|r| urlencoding::encode(r).into_owned())
            .collect::<Vec<_>>();

        let subkey_part = if let Some(key) = self.subkey.as_ref() {
            match key.to_bech32() {
                Ok(bech32) => format!("&subkey={}", bech32),
                Err(_) => String::new(),
            }
        } else {
            String::new()
        };

        let no_request_part = if self.no_request {
            "&no_request=true"
        } else {
            ""
        };

        match self.main_key.to_bech32() {
            Ok(bech32) => write!(
                f,
                "portal://{}?relays={}&token={}{}{}",
                bech32,
                relays.join(","),
                self.token,
                subkey_part,
                no_request_part
            ),
            Err(_) => Err(fmt::Error),
        }
    }
}

impl KeyHandshakeUrl {
    pub fn send_to(&self) -> nostr::PublicKey {
        if let Some(subkey) = self.subkey {
            subkey.into()
        } else {
            self.main_key.into()
        }
    }

    pub fn all_keys(&self) -> Vec<PublicKey> {
        let mut keys = Vec::new();
        keys.push(self.main_key);
        if let Some(subkey) = self.subkey {
            keys.push(subkey);
        }

        keys
    }
}

impl FromStr for KeyHandshakeUrl {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Trim whitespace / new lines
        let s = s.trim();

        // Check prefix
        if !s.starts_with("portal://") {
            return Err(ParseError::InvalidProtocol);
        }

        // Split URL into base and query
        let s = &s["portal://".len()..];
        let (pubkey, query) = s.split_once('?').ok_or(ParseError::MissingQueryParams)?;

        // Parse main pubkey
        let main_key = nostr::PublicKey::from_bech32(pubkey)?;

        // Parse query parameters
        let mut relays = Vec::new();
        let mut token = None;
        let mut subkey = None;
        let mut no_request = false;

        for param in query.split('&') {
            let (key, value) = param
                .split_once('=')
                .ok_or_else(|| ParseError::InvalidQueryParam("missing value".into()))?;

            match key {
                "relays" => {
                    relays = value
                        .split(',')
                        .map(|r| urlencoding::decode(r).map(|s| s.into_owned()))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| ParseError::InvalidRelayUrl(e.to_string()))?;
                }
                "token" => token = Some(value.to_string()),
                "subkey" => subkey = Some(nostr::PublicKey::from_bech32(value)?),
                "no_request" => no_request = value == "true",
                _ => {
                    return Err(ParseError::InvalidQueryParam(format!(
                        "unknown parameter: {}",
                        key
                    )));
                }
            }
        }

        let token = token.ok_or(ParseError::MissingRequiredParam("token"))?;
        if relays.is_empty() {
            return Err(ParseError::NoRelays);
        }

        Ok(Self {
            main_key: PublicKey::from(main_key),
            relays,
            token,
            subkey: subkey.map(|k| PublicKey::from(k)),
            no_request,
        })
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid protocol")]
    InvalidProtocol,

    #[error("Missing query parameters")]
    MissingQueryParams,

    #[error("Invalid query parameter: {0}")]
    InvalidQueryParam(String),

    #[error("Missing required parameter: {0}")]
    MissingRequiredParam(&'static str),

    #[error("Invalid relay URL: {0}")]
    InvalidRelayUrl(String),

    #[error("No relays specified")]
    NoRelays,

    #[error("Invalid bech32: {0}")]
    Bech32(#[from] nostr::nips::nip19::Error),
}
