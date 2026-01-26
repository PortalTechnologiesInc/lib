use std::sync::Arc;

use portal_app::{
    CallbackError, InvoiceRequestListener, InvoiceResponseListener, nwc::MakeInvoiceResponse,
};
use portal_cli::{CliError, create_app_instance};
use portal::protocol::model::{
    Timestamp,
    payment::{InvoiceRequestContent, InvoiceRequestContentWithKey, InvoiceResponse},
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
            payment_hash: Some(String::from("bolt11 hash")),
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

#[tokio::main]
async fn main() -> Result<(), CliError> {
    env_logger::init();

    let relays = vec!["wss://relay.nostr.net".to_string()];

    let (receiver_key, receiver) = create_app_instance(
        "Receiver",
        "mass derive myself benefit shed true girl orange family spawn device theme",
        relays.clone(),
    )
    .await?;
    let _receiver = receiver.clone();

    tokio::spawn(async move {
        log::info!("Receiver: Setting up invoice request listener");
        _receiver
            .listen_invoice_requests(Arc::new(LogInvoiceRequestListener))
            .await
            .expect("Receiver: Error creating listener");
    });

    let (sender_key, sender) = create_app_instance(
        "Sender",
        "draft sunny old taxi chimney ski tilt suffer subway bundle once story",
        relays.clone(),
    )
    .await?;

    log::info!("Apps created, waiting 5 seconds before sending request");

    let _sender = sender.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let result = _sender
            .request_invoice(
                receiver_key.public_key(),
                InvoiceRequestContent {
                    request_id: String::from("my_id"),
                    amount: 5000,
                    currency: portal::protocol::model::payment::Currency::Millisats,
                    current_exchange_rate: None,
                    expires_at: Timestamp::now_plus_seconds(120),
                    description: Some(String::from("Dinner")),
                    refund_invoice: None,
                },
                Arc::new(LogInvoiceResponseListener),
            )
            .await
            .unwrap();
    });

    log::info!("Apps created");

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    Ok(())
}
