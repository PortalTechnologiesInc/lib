use std::sync::Arc;

use app::nwc::MakeInvoiceResponse;
use cli::{CliError, create_app_instance};
use portal::protocol::model::{Timestamp, payment::InvoiceRequestContent};

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
    let receiver_loop = Arc::clone(&receiver);

    tokio::spawn(async move {
        log::info!("Receiver: Setting up invoice request loop");
        loop {
            match receiver_loop.next_invoice_request().await {
                Ok(request) => {
                    if let Err(e) = receiver_loop
                        .reply_invoice_request(
                            request,
                            MakeInvoiceResponse {
                                invoice: String::from("bolt11"),
                                payment_hash: Some(String::from("bolt11 hash")),
                            },
                        )
                        .await
                    {
                        log::error!("Receiver: Failed to reply to invoice request: {:?}", e);
                    }
                }
                Err(e) => {
                    log::error!("Receiver: Invoice loop error: {:?}", e);
                    break;
                }
            }
        }
    });

    let (_sender_key, sender) = create_app_instance(
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
            )
            .await
            .unwrap();

        log::info!("Sender: Invoice response {:?}", result);
    });

    log::info!("Apps created");

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    Ok(())
}
