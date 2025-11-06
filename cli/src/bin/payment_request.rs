use std::{sync::Arc, time::Duration as StdDuration};

use app::{CallbackError, CashuRequestListener, PaymentRequestListener, PaymentStatusNotifier, RecurringPaymentRequest, SinglePaymentRequest};
use cli::{CliError, create_app_instance, create_sdk_instance};
use portal::protocol::model::{
    payment::{Currency, RecurringPaymentResponseContent, RecurringPaymentStatus, SinglePaymentRequestContent}, Timestamp
};

struct LogPaymentRequestListener;

#[async_trait::async_trait]
impl PaymentRequestListener for LogPaymentRequestListener {
    async fn on_single_payment_request(
        &self,
        event: SinglePaymentRequest,
        notifier: Arc<dyn PaymentStatusNotifier>,
    ) -> Result<(), CallbackError> {
        log::info!("Receiver: Received Payment request: {:?}", event);
        // Always approve for test
        Ok(())
    }
    async fn on_recurring_payment_request(
        &self,
        event: RecurringPaymentRequest,
    ) -> Result<RecurringPaymentResponseContent, CallbackError> {
        log::info!("Receiver: Received Recurring Payment request: {:?}", event);
        Ok(RecurringPaymentResponseContent {
            request_id: event.content.request_id,
            status: RecurringPaymentStatus::Rejected { reason: Some("User rejected".to_string()) },
        })
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
        log::info!("Receiver: Setting up Payment request listener");
        _receiver
            .listen_for_payment_request(Arc::new(LogPaymentRequestListener))
            .await
            .expect("Receiver: Error creating listener");
    });

    let sender_sdk = create_sdk_instance(
        "draft sunny old taxi chimney ski tilt suffer subway bundle once story",
        relays.clone(),
    )
    .await?;

    log::info!("Apps created, waiting 5 seconds before sending request");
    tokio::time::sleep(StdDuration::from_secs(5)).await;

    let request_content = SinglePaymentRequestContent {
        amount: 1000,
        currency: Currency::Millisats,
        current_exchange_rate: None,
        invoice: "invoice".to_string(),
        auth_token: None,
        expires_at: Timestamp::now_plus_seconds(300),
        subscription_id: None,
        description: Some("test".to_string()),
        request_id: "test".to_string(),
    };

    let mut response = sender_sdk
        .request_single_payment(receiver_key.public_key().0, vec![], request_content)
        .await
        .unwrap();

    while let Some(resp) = response.next().await {

        match resp {
            Ok(resp) => {
                log::info!("Sender: Received payment: {:?}", resp);
            }
            Err(e) => {
                log::error!("Sender: Error receiving payment: {}", e);
            }
        }
    }

    Ok(())
}
