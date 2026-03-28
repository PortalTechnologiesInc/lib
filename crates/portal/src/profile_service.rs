use nostr::event::EventBuilder;

const PROFILE_SERVICE_URL: &str = "https://profile.getportal.cc";
const GETPORTAL_DOMAIN: &str = "getportal.cc";

#[derive(Debug, serde::Serialize)]
struct ProfileServiceContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    nip_05: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    img: Option<String>,
}

/// Register a NIP-05 identifier with the Portal profile service.
///
/// Only performs the HTTP registration if the domain is `getportal.cc`.
/// `nip05` is the full identifier, e.g. "alice@getportal.cc" or "test@google.com".
/// Returns Ok(true) if registered, Ok(false) if skipped (non-getportal.cc domain).
pub async fn register_nip05(keys: &nostr::Keys, nip05: &str) -> Result<bool, String> {
    // Parse "local@domain"
    let mut parts = nip05.splitn(2, '@');
    let local_part = parts.next().unwrap_or("").trim().to_lowercase();
    let domain = parts.next().unwrap_or("").trim().to_lowercase();

    if domain != GETPORTAL_DOMAIN {
        // Not a getportal.cc address — nothing to register, user manages their own .well-known
        return Ok(false);
    }

    if local_part.is_empty() {
        return Err("NIP-05 local part is empty".to_string());
    }

    let content = ProfileServiceContent {
        nip_05: Some(local_part),
        img: None,
    };
    let content_json = serde_json::to_string(&content).map_err(|e| e.to_string())?;

    let event = EventBuilder::text_note(&content_json)
        .sign_with_keys(keys)
        .map_err(|e| e.to_string())?;

    let event_json = serde_json::to_string(&event).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    let resp = client
        .post(PROFILE_SERVICE_URL)
        .header("Content-Type", "application/json")
        .body(event_json)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        Ok(true)
    } else {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Profile service error {}: {}", status, body))
    }
}
