use std::sync::Arc;

use app::{
    auth::AuthChallengeEvent, db::PortalDB, nwc::MakeInvoiceResponse, AuthChallengeListener, CallbackError, ClosedRecurringPaymentListener, InvoiceRequestListener, InvoiceResponseListener, Keypair, Mnemonic, PaymentRequestListener, PortalApp, RecurringPaymentRequest, RelayStatus, RelayStatusListener, RelayUrl, SinglePaymentRequest
};
use portal::{
    nostr::nips::{nip19::ToBech32, nip47::PayInvoiceRequest},
    profile::Profile,
    protocol::model::{
        Timestamp,
        auth::AuthResponseStatus,
        bindings::PublicKey,
        payment::{
            CloseRecurringPaymentResponse, InvoiceRequestContent, InvoiceRequestContentWithKey,
            InvoiceResponse, PaymentResponseContent, PaymentStatus,
            RecurringPaymentResponseContent, RecurringPaymentStatus,
        },
    },
};

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
) -> Result<(Arc<Keypair>, Arc<PortalApp>), CliError> {
    log::info!("{}: Creating app instance", name);

    let mnemonic = Mnemonic::new(mnemonic)?;

    // let mnemonic = generate_mnemonic()?;
    let keypair = Arc::new(mnemonic.get_keypair()?);

    let app = PortalApp::new(
        keypair.clone(),
        vec![
            "wss://relay.nostr.net".to_string(),
            "wss://relay.damus.io".to_string(),
        ],
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
