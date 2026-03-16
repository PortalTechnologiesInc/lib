use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::{debug, error};

use crate::config::WebhookSettings;
use crate::response::NotificationData;

type HmacSha256 = Hmac<Sha256>;

/// Delivers a webhook notification if a webhook URL is configured.
///
/// The payload is JSON-serialised `NotificationData` wrapped in an envelope with the `stream_id`.
/// If a `webhook_secret` is configured, the raw JSON body is signed with HMAC-SHA256 and the
/// hex-encoded signature is sent in the `X-Portal-Signature` header.
pub async fn deliver(
    settings: &WebhookSettings,
    stream_id: &str,
    data: &NotificationData,
) {
    let url = match &settings.url {
        Some(u) if !u.is_empty() => u.clone(),
        _ => return, // No webhook configured
    };

    #[derive(serde::Serialize)]
    struct Envelope<'a> {
        stream_id: &'a str,
        #[serde(flatten)]
        data: &'a NotificationData,
    }

    let envelope = Envelope { stream_id, data };
    let body = match serde_json::to_string(&envelope) {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to serialise webhook payload: {e}");
            return;
        }
    };

    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json");

    // Sign with HMAC-SHA256 if secret is set
    if let Some(secret) = &settings.secret {
        if !secret.is_empty() {
            let mut mac =
                HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
            mac.update(body.as_bytes());
            let signature = hex::encode(mac.finalize().into_bytes());
            req = req.header("X-Portal-Signature", signature);
        }
    }

    debug!("Delivering webhook to {url} for stream {stream_id}");

    match req.body(body).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                error!(
                    "Webhook delivery to {url} returned HTTP {}",
                    resp.status()
                );
            }
        }
        Err(e) => {
            error!("Webhook delivery to {url} failed: {e}");
        }
    }
}
