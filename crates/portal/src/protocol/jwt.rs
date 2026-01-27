use jwt_compact::TimeOptions;
use jwt_compact::{UntrustedToken, alg::Es256k};
use serde::{Deserialize, Serialize};

use chrono::{Duration, Utc};
use jwt_compact::prelude::*;
use secp256k1::{PublicKey, SecretKey, XOnlyPublicKey};
use thiserror::Error;

use crate::protocol::model::bindings;

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Failed to create JWT token: {0}")]
    TokenCreation(String),

    #[error("Failed to parse JWT token: {0}")]
    TokenParsing(String),

    #[error("Failed to verify JWT token: {0}")]
    TokenVerification(String),

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Invalid secret key: {0}")]
    InvalidSecretKey(String),

    #[error("Token has expired")]
    TokenExpired,

    #[error("Token is not yet valid")]
    TokenNotYetValid,

    #[error("Invalid token format")]
    InvalidTokenFormat,
}

/// Custom claims encoded in the token.
#[derive(uniffi::Object, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomClaims {
    pub target_key: nostr::key::PublicKey,
    // other fields...
}

#[uniffi::export]
impl CustomClaims {
    #[uniffi::constructor]
    pub fn new(target_key: bindings::PublicKey) -> Self {
        Self {
            target_key: target_key.into(),
        }
    }
}

pub fn encode(
    secret_key: &nostr::key::SecretKey,
    claims: CustomClaims,
    duration: Duration,
) -> Result<String, JwtError> {
    let es256k: Es256k = Es256k::default();

    // Choose time-related options for token creation / validation.
    let time_options = TimeOptions::default();
    // Create a symmetric HMAC key, which will be used both to create and verify tokens.
    // Create a token.
    let header = Header::empty(); /*.with_key_id("my-key");*/
    let claims = Claims::new(claims)
        .set_duration_and_issuance(&time_options, duration)
        .set_not_before(Utc::now());

    // Apply the tweak to the private key
    let mut secret_key = match SecretKey::from_slice(secret_key.as_secret_bytes()) {
        Ok(key) => key,
        Err(e) => return Err(JwtError::InvalidSecretKey(e.to_string())),
    };

    if secret_key
        .public_key(&secp256k1::Secp256k1::new())
        .x_only_public_key()
        .1
        == secp256k1::Parity::Odd
    {
        secret_key = secret_key.negate();
    }

    let token_string = es256k
        .token(&header, &claims, &secret_key)
        .map_err(|e| JwtError::TokenCreation(e.to_string()))?;
    Ok(token_string)
}

pub fn decode(public_key: &nostr::key::PublicKey, token: &str) -> Result<CustomClaims, JwtError> {
    let es256k: Es256k = Es256k::default();

    let token = UntrustedToken::new(&token).map_err(|e| JwtError::TokenParsing(e.to_string()))?;

    let x_public_key = XOnlyPublicKey::from_slice(public_key.as_bytes())
        .map_err(|e| JwtError::InvalidPublicKey(e.to_string()))?;
    let public_key = PublicKey::from_x_only_public_key(x_public_key, secp256k1::Parity::Even);

    let verified = es256k
        .validator::<CustomClaims>(&public_key)
        .validate(&token)
        .map_err(|e| JwtError::TokenVerification(e.to_string()))?;

    let custom_claims = verified.into_parts().1.custom;
    Ok(custom_claims)
}
