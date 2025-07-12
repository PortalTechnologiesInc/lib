use std::{io::Write, str::FromStr, sync::Arc};

use app::{
    AuthChallengeListener, CallbackError, ClosedRecurringPaymentListener, InvoiceRequestListener,
    InvoiceResponseListener, Mnemonic, PaymentRequestListener, PortalApp, RecurringPaymentRequest,
    RelayStatus, RelayStatusListener, RelayUrl, SinglePaymentRequest, auth::AuthChallengeEvent,
    db::PortalDB, nwc::MakeInvoiceResponse,
};
use cli::{CliError, create_app_instance};
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

    tokio::time::sleep(std::time::Duration::from_secs(600)).await;
    Ok(())
}