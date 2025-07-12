use std::{io::Write, str::FromStr, sync::Arc};

use app::{
    AuthChallengeListener, CallbackError, ClosedRecurringPaymentListener, Mnemonic,
    PaymentRequestListener, PortalApp, RecurringPaymentRequest, RelayStatus, RelayStatusListener,
    RelayUrl, SinglePaymentRequest, auth::AuthChallengeEvent, db::PortalDB,
};
use nwc::nostr;
use portal::{
    nostr::nips::{nip19::ToBech32, nip47::PayInvoiceRequest},
    profile::Profile,
    protocol::{
        key_handshake::KeyHandshakeUrl,
        model::{
            auth::AuthResponseStatus,
            bindings::PublicKey,
            payment::{
                CloseRecurringPaymentResponse, PaymentResponseContent, PaymentStatus,
                RecurringPaymentResponseContent, RecurringPaymentStatus,
            },
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

struct ApproveLogin;

#[async_trait::async_trait]
impl AuthChallengeListener for ApproveLogin {
    async fn on_auth_challenge(
        &self,
        event: AuthChallengeEvent,
    ) -> Result<AuthResponseStatus, CallbackError> {
        log::info!("Received auth challenge: {:?}", event);
        Ok(AuthResponseStatus::Approved {
            granted_permissions: vec![],
            session_token: String::from("ABC"),
        })
    }
}

struct ApprovePayment(Arc<nwc::NWC>);

#[async_trait::async_trait]
impl PaymentRequestListener for ApprovePayment {
    async fn on_single_payment_request(
        &self,
        event: SinglePaymentRequest,
    ) -> Result<PaymentResponseContent, CallbackError> {
        log::info!("Received single payment request: {:?}", event);

        let nwc = self.0.clone();
        tokio::task::spawn(async move {
            let payment_result = nwc
                .pay_invoice(PayInvoiceRequest {
                    id: None,
                    invoice: event.content.invoice,
                    amount: None,
                })
                .await;
            log::info!("Payment result: {:?}", payment_result);
        });

        Ok(PaymentResponseContent {
            status: PaymentStatus::Pending,
            request_id: event.content.request_id,
        })
    }

    async fn on_recurring_payment_request(
        &self,
        event: RecurringPaymentRequest,
    ) -> Result<RecurringPaymentResponseContent, CallbackError> {
        log::info!("Received recurring payment request: {:?}", event);
        Ok(RecurringPaymentResponseContent {
            status: RecurringPaymentStatus::Confirmed {
                subscription_id: "randomid".to_string(),
                authorized_amount: event.content.amount,
                authorized_currency: event.content.currency,
                authorized_recurrence: event.content.recurrence,
            },
            request_id: event.content.request_id,
        })
    }
}

struct LogClosedRecurringPayment;

#[async_trait::async_trait]
impl ClosedRecurringPaymentListener for LogClosedRecurringPayment {
    async fn on_closed_recurring_payment(
        &self,
        event: CloseRecurringPaymentResponse,
    ) -> Result<(), CallbackError> {
        log::warn!("Received closed recurring payment: {:?}", event);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let mnemonic = Mnemonic::new(
        "mass derive myself benefit shed true girl orange family spawn device theme",
    )?;
    // let mnemonic = generate_mnemonic()?;
    let keypair = Arc::new(mnemonic.get_keypair()?);

    // Testing database so commented for now
    let nwc_str = std::env::var("CLI_NWC_URL").expect("CLI_NWC_URL is not set");
    let nwc = nwc::NWC::new(nwc_str.parse()?);

    log::info!(
        "Public key: {:?}",
        keypair.public_key().to_bech32().unwrap()
    );

    let db = PortalDB::new(
        keypair.clone(),
        vec![
            "wss://relay.nostr.net".to_string(),
            "wss://relay.damus.io".to_string(),
        ],
    )
    .await?;

    // Testing database
    let age_example = 1.to_string();
    db.store("age".to_string(), &age_example).await?;
    let age = db.read("age".to_string()).await?;
    if age != age_example {
        // error
        log::error!("Failed to set or get value from database: {:?}", age);
    }

    let history = db.read_history("age".to_string()).await?;
    log::info!("History of age: {:?}", history);

    let app = PortalApp::new(
        keypair,
        vec![
            "wss://relay.nostr.net".to_string(),
            // "wss://relay.damus.io".to_string(),
        ],
        Arc::new(LogRelayStatusChange),
    )
    .await?;

    let _app = Arc::clone(&app);

    tokio::spawn(async move {
        _app.listen().await.unwrap();
    });

    app.set_profile(Profile {
        name: Some("John Doe".to_string()),
        display_name: Some("John Doe".to_string()),
        picture: Some("https://tr.rbxcdn.com/180DAY-4d8c678185e70957c8f9b5ca267cd335/420/420/Image/Png/noFilter".to_string()),
        nip05: Some("john.doe@example.com".to_string()),
    }).await?;
    dbg!(
        app.fetch_profile(PublicKey(nostr::PublicKey::parse(
            "1e48492f5515d70e4fb40841894701cd97a35d7ea5bf93c84d2eac300ce4c25c"
        )?))
        .await?
    );

    let _app = Arc::clone(&app);
    tokio::spawn(async move {
        _app.listen_for_auth_challenge(Arc::new(ApproveLogin))
            .await
            .unwrap();
    });

    let _app = Arc::clone(&app);
    tokio::spawn(async move {
        _app.listen_for_payment_request(Arc::new(ApprovePayment(Arc::new(nwc))))
            .await
            .unwrap();
    });

    let _app = Arc::clone(&app);
    tokio::spawn(async move {
        _app.listen_closed_recurring_payment(Arc::new(LogClosedRecurringPayment))
            .await
            .unwrap();
    });

    let _app = Arc::clone(&app);
    tokio::spawn(async move {
        _app.register_nip05("phantomsto".to_owned()).await.unwrap();
    });

    // let _app = Arc::clone(&app);
    // tokio::spawn(async move {
    //     let base_64_img = "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";
    //     _app.register_img(byte_img.to_vec())
    //         .await
    //         .unwrap();
    // });

    // app.close_recurring_payment(
    //     portal::protocol::model::bindings::PublicKey(
    //         portal::nostr::PublicKey::from_str(
    //             "npub1ek206p7gwgqzgc6s7sfedmlu87cz9894jzzq0283t72lhz3uuxwsgn9stz",
    //         )
    //         .unwrap(),
    //     ),
    //     "randomid".to_string(),
    // )
    // .await?;

    println!("\nEnter the auth init URL:");
    std::io::stdout().flush()?;

    let mut key_handshake_url = String::new();
    std::io::stdin().read_line(&mut key_handshake_url)?;
    let url = KeyHandshakeUrl::from_str(key_handshake_url.trim())?;
    app.send_key_handshake(url).await?;

    tokio::time::sleep(std::time::Duration::from_secs(600)).await;

    Ok(())
}
