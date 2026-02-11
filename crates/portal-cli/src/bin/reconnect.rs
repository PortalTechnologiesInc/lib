use std::time::Duration;

use portal_cli::create_app_instance;

use portal_cli::CliError;

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

    let (_keypair, app) = create_app_instance(
        "Reconnect",
        "mass derive myself benefit shed true girl orange family spawn device theme",
        relays.clone(),
    )
    .await?;

    // reconnect in 10 seconds

    tokio::time::sleep(Duration::from_secs(7)).await;

    log::info!("Reconnecting in 10 seconds");
    // print 10 empty lines
    for _ in 0..10 {
        log::info!("");
    }

    tokio::time::sleep(Duration::from_secs(10)).await;

    app.reconnect().await?;

    tokio::time::sleep(Duration::from_secs(30)).await;

    Ok(())
}
