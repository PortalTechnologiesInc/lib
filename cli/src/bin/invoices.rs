use std::{io::Write, str::FromStr, sync::Arc};

use app::{
    AuthChallengeListener, CallbackError, ClosedRecurringPaymentListener, InvoiceRequestListener,
    InvoiceResponseListener, Mnemonic, PaymentRequestListener, PortalApp, RecurringPaymentRequest,
    SinglePaymentRequest, auth::AuthChallengeEvent, db::PortalDB, nwc::MakeInvoiceResponse,
};
use portal::{
    nostr::nips::{nip19::ToBech32, nip47::PayInvoiceRequest},
    profile::Profile,
    protocol::{
        auth_init::AuthInitUrl,
        model::{
            Timestamp,
            auth::AuthResponseStatus,
            bindings::PublicKey,
            payment::{
                CloseRecurringPaymentResponse, InvoiceRequestContent, InvoiceRequestContentWithKey,
                InvoiceResponse, PaymentResponseContent, PaymentStatus,
                RecurringPaymentResponseContent, RecurringPaymentStatus,
            },
        },
    },
};

struct LogInvoiceRequestListener;

#[async_trait::async_trait]
impl InvoiceRequestListener for LogInvoiceRequestListener {
    async fn on_invoice_requests(
        &self,
        event: InvoiceRequestContentWithKey,
    ) -> Result<MakeInvoiceResponse, CallbackError> {
        Ok(MakeInvoiceResponse {
            invoice: String::from("bolt11"),
            payment_hash: String::from("bolt11 hash"),
        })
    }
}

struct LogInvoiceResponseListener;

#[async_trait::async_trait]
impl InvoiceResponseListener for LogInvoiceResponseListener {
    async fn on_invoice_response(&self, event: InvoiceResponse) -> Result<(), CallbackError> {
        log::info!("Received an invoice: {:?}", event);
        Ok(())
    }
}

type CliError = Box<dyn std::error::Error>;

#[tokio::main]
async fn main() -> Result<(), CliError> {
    env_logger::init();

    let (receiver_key, receiver) = create_app_instance(
        "Receiver",
        "mass derive myself benefit shed true girl orange family spawn device theme",
    )
    .await?;
    let _receiver = receiver.clone();
    tokio::spawn(async move {
        _receiver
            .listen_invoice_requests(Arc::new(LogInvoiceRequestListener))
            .await
            .expect("Receiver: Error creating listener");
    });

    let (sender_key, sender) = create_app_instance(
        "Sender",
        "draft sunny old taxi chimney ski tilt suffer subway bundle once story",
    )
    .await?;

    let _sender = sender.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        _sender
            .request_invoice(
                receiver_key, 
                InvoiceRequestContent {
                    request_id: String::from("my_id"),
                    amount: 5000,
                    currency: portal::protocol::model::payment::Currency::Millisats,
                    current_exchange_rate: None,
                    expires_at: Timestamp::now_plus_seconds(120),
                    description: Some(String::from("Dinner")),
                },
                Arc::new(LogInvoiceResponseListener),
            )
            .await
            .unwrap();
    });

    log::info!("Apps created");

    tokio::time::sleep(std::time::Duration::from_secs(600)).await;
    Ok(())
}

async fn create_app_instance(
    name: &str,
    mnemonic: &str,
) -> Result<(PublicKey, Arc<PortalApp>), CliError> {
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

    Ok((keypair.public_key(), app))
}
