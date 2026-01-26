use std::{sync::Arc, time::Duration as StdDuration};

use portal_app::{CallbackError, CashuRequestListener};
use portal_cli::{CliError, create_app_instance, create_sdk_instance};
use portal::protocol::model::{
    Timestamp,
    payment::{CashuRequestContent, CashuRequestContentWithKey, CashuResponseStatus},
};

struct LogCashuRequestListener;

#[async_trait::async_trait]
impl CashuRequestListener for LogCashuRequestListener {
    async fn on_cashu_request(
        &self,
        event: CashuRequestContentWithKey,
    ) -> Result<CashuResponseStatus, CallbackError> {
        log::info!("Received Cashu request: {:?}", event);
        // Always approve for test
        Ok(CashuResponseStatus::Success {
            token: "testtoken123".to_string(),
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
        log::info!("Receiver: Setting up Cashu request listener");
        _receiver
            .listen_cashu_requests(Arc::new(LogCashuRequestListener))
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

    let request_content = CashuRequestContent {
        request_id: "cashu_test_1".to_string(),
        mint_url: "https://mint.example.com".to_string(),
        unit: "msat".to_string(),
        amount: 12345,
        expires_at: Timestamp::now_plus_seconds(300),
    };

    let response = sender_sdk
        .request_cashu(receiver_key.public_key().0, vec![], request_content)
        .await;

    match response {
        Ok(Some(resp)) => match resp.status {
            CashuResponseStatus::Success { token } => {
                log::info!("Sender: Received Cashu token: {}", token);
            }
            CashuResponseStatus::InsufficientFunds => {
                log::info!("Sender: Insufficient funds");
            }
            CashuResponseStatus::Rejected { reason } => {
                log::info!("Sender: Cashu request rejected: {:?}", reason);
            }
        },
        Ok(None) => {
            log::info!("Sender: No response received");
        }
        Err(e) => {
            log::error!("Sender: Error requesting Cashu: {}", e);
        }
    }

    tokio::time::sleep(StdDuration::from_secs(8)).await;
    Ok(())
}
