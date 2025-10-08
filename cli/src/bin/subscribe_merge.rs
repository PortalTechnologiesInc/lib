use std::sync::Arc;
use std::time::Duration;

use cli::create_app_instance;

use cli::CliError;
use cli::create_sdk_instance;
use portal::router::NotificationStream;
use portal::conversation::sdk::auth::KeyHandshakeEvent;

/// Reconnect to all relays
///
/// This command disconnects all relays and then connects them again.
#[tokio::main]
pub async fn main() -> Result<(), CliError> {
    env_logger::init();

    let relays = vec![
        "wss://relay.nostr.net".to_string(),
        "wss://relay.damus.io".to_string(),
    ];
    let sdk = Arc::new(
        create_sdk_instance(
            "draft sunny old taxi chimney ski tilt suffer subway bundle once story",
            relays.clone(),
        )
        .await?,
    );

    let sdk_clone = sdk.clone();
    tokio::spawn(async move {
        let (key_handshake_url, event) = sdk_clone
            .new_key_handshake_url(Some("static_token".to_string()), Some(false))
            .await
            .unwrap();
        log::info!("Key handshake url: {}", key_handshake_url);
        let mut event: NotificationStream<KeyHandshakeEvent> = event;
        let notification = event.next().await.unwrap();
        log::info!("Notification: {:?}", notification);
    });

    log::info!("Waiting 5 seconds");
    tokio::time::sleep(Duration::from_secs(5)).await;

    let sdk_clone = sdk.clone();
    tokio::spawn(async move {
        let (key_handshake_url, event) = sdk_clone
            .new_key_handshake_url(Some("static_token2".to_string()), Some(false))
            .await
            .unwrap();
        log::info!("Key handshake url: {}", key_handshake_url);
        let mut event: NotificationStream<KeyHandshakeEvent> = event;
        let notification = event.next().await.unwrap();
        log::info!("Notification: {:?}", notification);
    });

    log::info!("Waiting 300 seconds");
    tokio::time::sleep(Duration::from_secs(300)).await;

    Ok(())
}
