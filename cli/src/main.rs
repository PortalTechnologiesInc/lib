use std::{io::Write, str::FromStr, sync::Arc};

use app::{
    auth::AuthChallengeEvent, db::PortalDB, AuthChallengeListener, CallbackError, Mnemonic, PaymentRequestListener, PortalApp, RecurringPaymentRequest, SinglePaymentRequest
};
use portal::{
    nostr::nips::{nip19::ToBech32, nip47::PayInvoiceRequest}, profile::Profile, protocol::{
        auth_init::AuthInitUrl,
        model::payment::{PaymentResponseContent, PaymentStatus, RecurringPaymentResponseContent, RecurringPaymentStatus},
    }
};

struct ApproveLogin;

#[async_trait::async_trait]
impl AuthChallengeListener for ApproveLogin {
    async fn on_auth_challenge(&self, event: AuthChallengeEvent) -> Result<bool, CallbackError> {
        log::info!("Received auth challenge: {:?}", event);
        Ok(true)
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


    let db = PortalDB::new(keypair.clone(), vec![
        "wss://relay.nostr.net".to_string(),
        "wss://relay.damus.io".to_string(),
    ]).await?;

    // Testing database
    let age_example = 1.to_string();
    db.store("age".to_string(), &age_example).await?;
    let age = db.read("age".to_string()).await?;
    if age != age_example {
        // error
        log::error!("Failed to set or get value from database: {:?}", age);
    }

    let history =  db.read_history("age".to_string()).await?;
    log::info!("History of age: {:?}", history);

    let app = PortalApp::new(
        keypair,
        vec![
            "wss://relay.nostr.net".to_string(),
            // "wss://relay.damus.io".to_string(),
        ],
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
    // dbg!(app.fetch_profile(pk.into()).await?);

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
    

    println!("\nEnter the auth init URL:");
    std::io::stdout().flush()?;

    let mut auth_init_url = String::new();
    std::io::stdin().read_line(&mut auth_init_url)?;
    let url = AuthInitUrl::from_str(auth_init_url.trim())?;
    app.send_auth_init(url).await?;

    tokio::time::sleep(std::time::Duration::from_secs(600)).await;

    Ok(())
}
