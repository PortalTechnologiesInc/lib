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

        match self.main_key.to_bech32() {
            Ok(bech32) => write!(
                f,
                "portal://{}?relays={}&token={}{}",
                bech32,
                relays.join(","),
                self.token,
                subkey_part
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

#[cfg(test)]
mod tests {
    use super::*;
    use nostr::nips::nip19::ToBech32;
    use std::str::FromStr;

    fn create_test_keys() -> (nostr::PublicKey, nostr::PublicKey) {
        let main_key = nostr::Keys::generate().public_key();
        let sub_key = nostr::Keys::generate().public_key();
        (main_key, sub_key)
    }

    #[test]
    fn test_valid_url_basic() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        let url = format!(
            "portal://{}?relays=wss://relay1.example.com,wss://relay2.example.com&token=test_token",
            main_key_bech32
        );

        let url = "portal://npub1f85r7zp3zrlxgxxlufuxuv3x7jeda3ttsnp7jwgvhm8pjzc0950ssy3hal?relays=wss%3A%2F%2Frelay.getportal.cc&token=token_1759248229913662731";
        let url = "portal://npub1f85r7zp3zrlxgxxlufuxuv3x7jeda3ttsnp7jwgvhm8pjzc0950ssy3hal?relays=wss://relay.getportal.cc&token=token_1759305502990630137";
        
        let parsed = KeyHandshakeUrl::from_str(&url).unwrap();
        dbg!(&parsed);
        
        assert_eq!(parsed.main_key, PublicKey::from(nostr::PublicKey::from_str("npub1f85r7zp3zrlxgxxlufuxuv3x7jeda3ttsnp7jwgvhm8pjzc0950ssy3hal").unwrap()));
        assert_eq!(parsed.relays, vec!["wss://relay.getportal.cc"]);
        assert_eq!(parsed.token, "token_1759305502990630137");
        assert_eq!(parsed.subkey, None);
    }

    #[test]
    fn test_valid_url_with_subkey() {
        let (main_key, sub_key) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        let sub_key_bech32 = sub_key.to_bech32().unwrap();
        
        let url = format!(
            "portal://{}?relays=wss://relay.example.com&token=test_token&subkey={}",
            main_key_bech32, sub_key_bech32
        );
        
        let parsed = KeyHandshakeUrl::from_str(&url).unwrap();
        
        assert_eq!(parsed.main_key, PublicKey::from(main_key));
        assert_eq!(parsed.relays, vec!["wss://relay.example.com"]);
        assert_eq!(parsed.token, "test_token");
        assert_eq!(parsed.subkey, Some(PublicKey::from(sub_key)));
    }

    #[test]
    fn test_invalid_protocol() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        let url = format!(
            "https://{}?relays=wss://relay.example.com&token=test_token",
            main_key_bech32
        );
        
        let result = KeyHandshakeUrl::from_str(&url);
        assert!(matches!(result, Err(ParseError::InvalidProtocol)));
    }

    #[test]
    fn test_missing_query_params() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        let url = format!("portal://{}", main_key_bech32);
        
        let result = KeyHandshakeUrl::from_str(&url);
        assert!(matches!(result, Err(ParseError::MissingQueryParams)));
    }

    #[test]
    fn test_invalid_pubkey() {
        let url = "portal://invalid_pubkey?relays=wss://relay.example.com&token=test_token";
        
        let result = KeyHandshakeUrl::from_str(url);
        assert!(matches!(result, Err(ParseError::Bech32(_))));
    }

    #[test]
    fn test_missing_token() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        let url = format!(
            "portal://{}?relays=wss://relay.example.com",
            main_key_bech32
        );
        
        let result = KeyHandshakeUrl::from_str(&url);
        assert!(matches!(result, Err(ParseError::MissingRequiredParam("token"))));
    }

    #[test]
    fn test_no_relays() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        let url = format!(
            "portal://{}?token=test_token",
            main_key_bech32
        );
        
        let result = KeyHandshakeUrl::from_str(&url);
        assert!(matches!(result, Err(ParseError::NoRelays)));
    }

    #[test]
    fn test_empty_relays() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        let url = format!(
            "portal://{}?relays=&token=test_token",
            main_key_bech32
        );
        
        let result = KeyHandshakeUrl::from_str(&url);
        assert!(matches!(result, Err(ParseError::NoRelays)));
    }

    #[test]
    fn test_invalid_query_param() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        let url = format!(
            "portal://{}?relays=wss://relay.example.com&token=test_token&unknown_param=value",
            main_key_bech32
        );
        
        let result = KeyHandshakeUrl::from_str(&url);
        assert!(matches!(result, Err(ParseError::InvalidQueryParam(_))));
    }

    #[test]
    fn test_malformed_query_param() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        let url = format!(
            "portal://{}?relays=wss://relay.example.com&token&invalid_param",
            main_key_bech32
        );
        
        let result = KeyHandshakeUrl::from_str(&url);
        assert!(matches!(result, Err(ParseError::InvalidQueryParam(_))));
    }

    #[test]
    fn test_url_encoded_relays() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        // URL encode a relay with special characters
        let encoded_relay = urlencoding::encode("wss://relay.example.com/path?param=value");
        let url = format!(
            "portal://{}?relays={}&token=test_token",
            main_key_bech32, encoded_relay
        );
        
        let parsed = KeyHandshakeUrl::from_str(&url).unwrap();
        
        assert_eq!(parsed.relays, vec!["wss://relay.example.com/path?param=value"]);
    }

    #[test]
    fn test_multiple_relays() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        let url = format!(
            "portal://{}?relays=wss://relay1.example.com,wss://relay2.example.com,wss://relay3.example.com&token=test_token",
            main_key_bech32
        );
        
        let parsed = KeyHandshakeUrl::from_str(&url).unwrap();
        
        assert_eq!(parsed.relays, vec![
            "wss://relay1.example.com",
            "wss://relay2.example.com", 
            "wss://relay3.example.com"
        ]);
    }

    #[test]
    fn test_invalid_subkey() {
        let (main_key, _) = create_test_keys();
        let main_key_bech32 = main_key.to_bech32().unwrap();
        
        let url = format!(
            "portal://{}?relays=wss://relay.example.com&token=test_token&subkey=invalid_subkey",
            main_key_bech32
        );
        
        let result = KeyHandshakeUrl::from_str(&url);
        assert!(matches!(result, Err(ParseError::Bech32(_))));
    }

    #[test]
    fn test_roundtrip_serialization() {
        let (main_key, sub_key) = create_test_keys();
        let original = KeyHandshakeUrl {
            main_key: PublicKey::from(main_key),
            relays: vec!["wss://relay1.example.com".to_string(), "wss://relay2.example.com".to_string()],
            token: "test_token".to_string(),
            subkey: Some(PublicKey::from(sub_key)),
        };
        
        let url_string = original.to_string();
        let parsed = KeyHandshakeUrl::from_str(&url_string).unwrap();
        
        assert_eq!(parsed.main_key, original.main_key);
        assert_eq!(parsed.relays, original.relays);
        assert_eq!(parsed.token, original.token);
        assert_eq!(parsed.subkey, original.subkey);
    }

    #[test]
    fn test_send_to_method() {
        let (main_key, sub_key) = create_test_keys();
        
        // Test without subkey - should return main key
        let url_without_subkey = KeyHandshakeUrl {
            main_key: PublicKey::from(main_key),
            relays: vec!["wss://relay.example.com".to_string()],
            token: "test_token".to_string(),
            subkey: None,
        };
        assert_eq!(url_without_subkey.send_to(), main_key);
        
        // Test with subkey - should return subkey
        let url_with_subkey = KeyHandshakeUrl {
            main_key: PublicKey::from(main_key),
            relays: vec!["wss://relay.example.com".to_string()],
            token: "test_token".to_string(),
            subkey: Some(PublicKey::from(sub_key)),
        };
        assert_eq!(url_with_subkey.send_to(), sub_key);
    }

    #[test]
    fn test_all_keys_method() {
        let (main_key, sub_key) = create_test_keys();
        
        // Test without subkey - should return only main key
        let url_without_subkey = KeyHandshakeUrl {
            main_key: PublicKey::from(main_key),
            relays: vec!["wss://relay.example.com".to_string()],
            token: "test_token".to_string(),
            subkey: None,
        };
        assert_eq!(url_without_subkey.all_keys(), vec![PublicKey::from(main_key)]);
        
        // Test with subkey - should return both keys
        let url_with_subkey = KeyHandshakeUrl {
            main_key: PublicKey::from(main_key),
            relays: vec!["wss://relay.example.com".to_string()],
            token: "test_token".to_string(),
            subkey: Some(PublicKey::from(sub_key)),
        };
        assert_eq!(url_with_subkey.all_keys(), vec![PublicKey::from(main_key), PublicKey::from(sub_key)]);
    }
}
