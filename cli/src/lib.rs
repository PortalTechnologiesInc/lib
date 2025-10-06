use std::sync::Arc;

use app::{
    CallbackError, Keypair, Mnemonic, PortalApp, RelayStatus, RelayStatusListener, RelayUrl,
};
use sdk::PortalSDK;

struct LogRelayStatusChange;

#[async_trait::async_trait]
impl RelayStatusListener for LogRelayStatusChange {
    async fn on_relay_status_change(
        &self,
        relay_url: RelayUrl,
        status: RelayStatus,
    ) -> Result<(), CallbackError> {
        log::info!("Relay {:?} status changed: {:?}", relay_url.0, status);
        Ok(())
    }
}

pub type CliError = Box<dyn std::error::Error>;

pub async fn create_app_instance(
    name: &str,
    mnemonic: &str,
    _relays: Vec<String>,
) -> Result<(Arc<Keypair>, Arc<PortalApp>), CliError> {
    log::info!("{}: Creating app instance", name);

    let mnemonic = Mnemonic::new(mnemonic)?;

    // let mnemonic = generate_mnemonic()?;
    let keypair = Arc::new(mnemonic.get_keypair()?);

    let app = PortalApp::new(
        keypair.clone(),
        _relays,
        Arc::new(LogRelayStatusChange),
    )
    .await?;

    let _app = Arc::clone(&app);
    tokio::spawn(async move {
        _app.listen().await.unwrap();
    });

    /*
    app.set_profile(Profile {
        name: Some("John Doe".to_string()),
        display_name: Some("John Doe".to_string()),
        picture: Some("https://tr.rbxcdn.com/180DAY-4d8c678185e70957c8f9b5ca267cd335/420/420/Image/Png/noFilter".to_string()),
        nip05: Some("john.doe@example.com".to_string()),
    }).await?;

    */
    log::info!("{}: Created app instance", name);

    Ok((keypair, app))
}

pub async fn create_sdk_instance(
    mnemonic: &str,
    relays: Vec<String>,
) -> Result<PortalSDK, CliError> {
    let mnemonic = Mnemonic::new(mnemonic)?;

    // let mnemonic = generate_mnemonic()?;
    let keypair = mnemonic.get_keypair()?;

    let sdk = PortalSDK::new(keypair.inner, relays).await?;
    Ok(sdk)
}
