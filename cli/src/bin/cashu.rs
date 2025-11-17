use std::{sync::Arc, time::Duration as StdDuration};

use cli::{CliError, create_app_instance, create_sdk_instance};
use portal::protocol::model::{
    Timestamp,
    payment::{CashuRequestContent, CashuResponseStatus},
};

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
        log::info!("Receiver: Setting up Cashu request loop");
        loop {
            match receiver_loop.next_cashu_request().await {
                Ok(event) => {
                    log::info!("Receiver: Cashu request {:?}", event);
                    if let Err(e) = receiver_loop
                        .reply_cashu_request(
                            event,
                            CashuResponseStatus::Success {
                                token: "testtoken123".to_string(),
                            },
                        )
                        .await
                    {
                        log::error!("Receiver: Failed to reply to Cashu request: {:?}", e);
                    }
                }
                Err(e) => {
                    log::error!("Receiver: Cashu loop error: {:?}", e);
                    break;
                }
            }
        }
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
