use std::sync::Arc;

use business_app::AppError;
use business_app::CallbackError;
use business_app::PortalBusiness;
use business_app::business::KeyHandshakeListener;

use business_app::Keypair;
use business_app::RelayStatus;
use business_app::RelayStatusListener;
use business_app::RelayUrl;
use nostr::key::Keys;
use portal::protocol::LocalKeypair;
use portal::protocol::model::bindings::PublicKey;
use portal::protocol::model::payment::CashuDirectContent;

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

struct LogKeyHandshakeListener {
    tx: tokio::sync::mpsc::Sender<PublicKey>,
}

#[async_trait::async_trait]
impl KeyHandshakeListener for LogKeyHandshakeListener {
    async fn on_key_handshake(&self, pubkey: PublicKey) -> Result<(), AppError> {
        self.tx.send(pubkey).await.unwrap();
        Ok(())
    }
}

// relay status listener

async fn new_business_app()
-> Result<(Arc<Keypair>, Arc<PortalBusiness>), Box<dyn std::error::Error>> {
    // random nostr key

    let keys = Keys::generate();

    let local_keypair = LocalKeypair::new(keys, None);
    let keypair = Arc::new(Keypair {
        inner: local_keypair,
    });

    let app = PortalBusiness::new(
        Arc::clone(&keypair),
        vec![
            "wss://relay.nostr.net".to_string(),
            "wss://relay.getportal.cc".to_string(),
        ],
        Arc::new(LogRelayStatusChange),
    )
    .await?;

    Ok((Arc::clone(&keypair), app))
}

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Starting business app");

    let (keypair, app) = new_business_app().await.unwrap();

    println!("Keypair: {:?}", keypair.public_key());

    let _app = Arc::clone(&app);
    tokio::spawn(async move {
        _app.listen().await.unwrap();
    });

    let _app = Arc::clone(&app);
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        let url = _app
            .listen_for_key_handshake(
                Some(String::from("my-token")),
                Arc::new(LogKeyHandshakeListener { tx }),
            )
            .await
            .unwrap();

        let url_str = url.to_string();
        println!("Key handshake URL: {:?}", url_str);
    });

    // receiving from channel
    while let Some(pubkey) = rx.recv().await {
        log::info!("Key handshake: {:?}", pubkey);

        // retrived key from channel

        app.send_cashu_direct(
            pubkey,
            vec![],
            CashuDirectContent {
                token: "cashu token".to_string(),
            },
        )
        .await
        .unwrap();

        log::info!("Sent cashu direct");
    }

    // wait 1 minute before exiting
    tokio::time::sleep(std::time::Duration::from_secs(60 * 5)).await;

    println!("Exiting...");
}
