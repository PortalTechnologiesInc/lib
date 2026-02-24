use std::{io::Write, str::FromStr, sync::Arc};

use app::{
    CallbackError, IncomingPaymentRequest, Mnemonic, PortalApp, RelayStatus, RelayStatusListener,
    RelayUrl, SinglePaymentRequest, get_git_hash, nwc, nwc::NWC, parse_bolt11,
};
use log::{error, info};
use portal::{
    protocol::{
        key_handshake::KeyHandshakeUrl,
        model::{
            auth::AuthResponseStatus,
            nip46::NostrConnectResponseStatus,
            payment::{
                CashuResponseStatus, PaymentResponseContent, PaymentStatus,
                RecurringPaymentResponseContent, RecurringPaymentStatus,
            },
        },
    },
    utils::fetch_nip05_profile,
};

use lightning_invoice::Bolt11Invoice;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let nip05_profile = fetch_nip05_profile("johng@getportal.cc").await?;
    dbg!(nip05_profile);

    let mnemonic = Mnemonic::new(
        "mass derive myself benefit shed true girl orange family spawn device theme",
    )?;
    // let mnemonic = generate_mnemonic()?;
    let keypair = Arc::new(mnemonic.get_keypair()?);

    // Testing database so commented for now
    let nwc_str = std::env::var("CLI_NWC_URL").expect("CLI_NWC_URL is not set");
    let nwc = Arc::new(
        NWC::new(nwc_str.parse()?, Arc::new(LogRelayStatusChange)).unwrap_or_else(|e| {
            dbg!(e);
            panic!();
        }),
    );

    log::info!("Public key: {:?}", keypair.public_key());

    // let db = PortalDB::new(
    //     keypair.clone(),
    //     vec![
    //         "wss://relay.nostr.net".to_string(),
    //         "wss://relay.damus.io".to_string(),
    //     ],
    // )
    // .await?;

    // Testing database
    // let age_example = 1.to_string();
    // db.store("age".to_string(), &age_example).await?;
    // let age = db.read("age".to_string()).await?;
    // if age != age_example {
    //     // error
    //     log::error!("Failed to set or get value from database: {:?}", age);
    // }

    // let history = db.read_history("age".to_string()).await?;
    // log::info!("History of age: {:?}", history);

    let app = PortalApp::new(
        keypair,
        vec![
            "wss://relay.nostr.net".to_string(),
            "wss://relay.getportal.cc".to_string(),
        ],
        Arc::new(LogRelayStatusChange),
    )
    .await?;

    let _app = Arc::clone(&app);

    tokio::spawn(async move {
        _app.listen().await.unwrap();
    });

    // app.set_profile(Profile {
    //     name: Some("John Doe".to_string()),
    //     display_name: Some("John Doe".to_string()),
    //     picture: Some("https://tr.rbxcdn.com/180DAY-4d8c678185e70957c8f9b5ca267cd335/420/420/Image/Png/noFilter".to_string()),
    //     nip05: Some("john.doe@example.com".to_string()),
    // }).await?;
    // dbg!(
    //     app.fetch_profile(PublicKey(nostr::PublicKey::parse(
    //         "1e48492f5515d70e4fb40841894701cd97a35d7ea5bf93c84d2eac300ce4c25c"
    //     )?))
    //     .await?
    // );

    let auth_app = Arc::clone(&app);
    tokio::spawn(async move {
        auth_challenge_loop(auth_app).await;
    });

    let payment_app = Arc::clone(&app);
    let payment_nwc = Arc::clone(&nwc);
    tokio::spawn(async move {
        payment_request_loop(payment_app, payment_nwc).await;
    });

    let closed_app = Arc::clone(&app);
    tokio::spawn(async move {
        closed_recurring_loop(closed_app).await;
    });

    let cashu_request_app = Arc::clone(&app);
    tokio::spawn(async move {
        cashu_request_loop(cashu_request_app).await;
    });

    let cashu_direct_app = Arc::clone(&app);
    tokio::spawn(async move {
        cashu_direct_loop(cashu_direct_app).await;
    });

    let nip46_app = Arc::clone(&app);
    tokio::spawn(async move {
        nip46_loop(nip46_app).await;
    });

    // let _app = Arc::clone(&app);
    // tokio::spawn(async move {
    //     _app.register_nip05("phantomsto".to_owned()).await.unwrap();
    // });

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

    let invoice_str = "lnbc10n1p5s34dcdry2pshjmt9de6zqgrxdaezqum4vfekxunfwp6xjmmwyqmrzvmxvv6kvdpdxg6nsefdx3snwwpdv9jkywfdv43kyd33xuenzv33vscsnp4qwkwylw8hza20g7t7a0d9az7yq0hyckv6hv00nmrpqru53z07mjvvpp5yxjzjjegtzrkeflkt6mx39fpe0n7w8v3gllj757d4gh9swk2w3jqsp5j59vg5wp5cw9f749x4qff5a4qkx6t736t2efaqm6ykdx3hxgctaq9qyysgqcqpcxqyz5vqrzjqvdnqyc82a9maxu6c7mee0shqr33u4z9z04wpdwhf96gxzpln8jcrapyqqqqqqpmqgqqqqqqqqqqqqqq2qy0z70pp236cqspe2kkj7uyk9xp0lwrz9gnsr02z6y9nj9qlf9xdsmg4zqyggqea7nwva22mqn49c45n7x5rzf986krjg25c6gmk7nrcpqalrcl";
    let result = nwc.lookup_invoice(invoice_str.to_string()).await;
    match result {
        Ok(lur) => {
            info!("invoice from lookup_invoice -> {}", lur.invoice.unwrap());
        }
        Err(e) => {
            info!("{}", e);
        }
    }

    let bolt11_invoice = Bolt11Invoice::from_str(invoice_str)?;
    let payment_hash_string = bolt11_invoice.payment_hash().to_string();
    let result = nwc
        .lookup_invoice_from_payment_hash(payment_hash_string)
        .await;
    match result {
        Ok(lur) => {
            info!(
                "invoice from lookup_invoice_with_payment_hash -> {}",
                lur.invoice.unwrap()
            );
        }
        Err(e) => {
            info!("{}", e);
        }
    }

    dbg!(get_git_hash());
    tokio::spawn(async move {
        const INVOICE: &str = "lnbc100n1p5fvqfdsp586d9yz88deyfxm2mxgh39n39lezmpnkcv0a35uh38fvnjzlaxdzqpp59nwc8zac6psv09wysxvulgwj0t23jh3g5r4l5qzgpdsnel94w5zshp5mndu23huxkp6jgynf8agfjfaypgfjs2z8glq8fs9zqjfpnf34jnqcqpjrzjqgc7enr9zr4ju8yhezsep4h2p9ncf2nuxkp423pq2k4v3vsx2nunyz60tsqqj9qqqqqqqqqpqqqqqysqjq9qxpqysgqala28sswmp68uc9axqt893n48lzzt7l3uzkzjzlmlzurczpc647sxn4vrt4hvm30v5vv2ysvxhxeej78j903emrrjh02xdrl6z9alzqqns0w5s";
        match parse_bolt11(INVOICE) {
            Ok(invoice_data) => {
                dbg!(invoice_data);
            }
            Err(e) => {
                error!("Failed to parse bolt11 invoice: {:?}", e);
            }
        }
    });

    println!("\nEnter the auth init URL:");
    std::io::stdout().flush()?;

    let mut key_handshake_url = String::new();
    std::io::stdin().read_line(&mut key_handshake_url)?;
    let url = KeyHandshakeUrl::from_str(key_handshake_url.trim())?;
    app.send_key_handshake(url).await?;

    tokio::time::sleep(std::time::Duration::from_secs(600)).await;

    Ok(())
}

async fn auth_challenge_loop(app: Arc<PortalApp>) {
    loop {
        match app.next_auth_challenge().await {
            Ok(event) => {
                info!("Received auth challenge: {:?}", event);
                let _ = app.fetch_profile(event.service_key.clone()).await;
                let status = AuthResponseStatus::Approved {
                    granted_permissions: vec![],
                    session_token: String::from("ABC"),
                };
                if let Err(e) = app.reply_auth_challenge(event, status).await {
                    error!("Failed to reply to auth challenge: {:?}", e);
                }
            }
            Err(e) => {
                error!("Auth challenge loop error: {:?}", e);
                break;
            }
        }
    }
}

async fn payment_request_loop(app: Arc<PortalApp>, nwc: Arc<nwc::NWC>) {
    loop {
        match app.next_payment_request().await {
            Ok(IncomingPaymentRequest::Single(request)) => {
                info!("Received single payment request: {:?}", request);
                let single_app = Arc::clone(&app);
                let single_nwc = Arc::clone(&nwc);
                tokio::spawn(async move {
                    process_single_payment_request(single_app, single_nwc, request).await;
                });
            }
            Ok(IncomingPaymentRequest::Recurring(request)) => {
                info!("Received recurring payment request: {:?}", request);
                let content = request.content.clone();
                let status = RecurringPaymentResponseContent {
                    status: RecurringPaymentStatus::Confirmed {
                        subscription_id: "randomid".to_string(),
                        authorized_amount: content.amount,
                        authorized_currency: content.currency,
                        authorized_recurrence: content.recurrence,
                    },
                    request_id: content.request_id,
                };
                if let Err(e) = app.reply_recurring_payment_request(request, status).await {
                    error!("Failed to reply to recurring payment request: {:?}", e);
                }
            }
            Err(e) => {
                error!("Payment request loop error: {:?}", e);
                break;
            }
        }
    }
}

async fn process_single_payment_request(
    app: Arc<PortalApp>,
    nwc: Arc<nwc::NWC>,
    request: SinglePaymentRequest,
) {
    if let Err(e) = app
        .reply_single_payment_request(
            request.clone(),
            PaymentResponseContent {
                status: PaymentStatus::Approved,
                request_id: request.content.request_id.clone(),
            },
        )
        .await
    {
        error!("Failed to send approval for payment request: {:?}", e);
        return;
    }

    let payment_result = nwc.pay_invoice(request.content.invoice.clone()).await;
    info!("Payment result: {:?}", payment_result);

    let status = match payment_result {
        Ok(preimage) => PaymentResponseContent {
            status: PaymentStatus::Success {
                preimage: Some(preimage.preimage),
            },
            request_id: request.content.request_id.clone(),
        },
        Err(e) => {
            error!("Payment failed: {:?}", e);
            PaymentResponseContent {
                status: PaymentStatus::Failed {
                    reason: Some(e.to_string()),
                },
                request_id: request.content.request_id.clone(),
            }
        }
    };

    if let Err(e) = app.reply_single_payment_request(request, status).await {
        error!("Failed to send payment status update: {:?}", e);
    }
}

async fn closed_recurring_loop(app: Arc<PortalApp>) {
    loop {
        match app.next_closed_recurring_payment().await {
            Ok(event) => info!("Received closed recurring payment: {:?}", event),
            Err(e) => {
                error!("Closed recurring payment loop error: {:?}", e);
                break;
            }
        }
    }
}

async fn cashu_request_loop(app: Arc<PortalApp>) {
    loop {
        match app.next_cashu_request().await {
            Ok(event) => {
                info!("Received Cashu request: {:?}", event);
                if let Err(e) = app
                    .reply_cashu_request(
                        event,
                        CashuResponseStatus::Success {
                            token: "testtoken123".to_string(),
                        },
                    )
                    .await
                {
                    error!("Failed to reply to Cashu request: {:?}", e);
                }
            }
            Err(e) => {
                error!("Cashu request loop error: {:?}", e);
                break;
            }
        }
    }
}

async fn cashu_direct_loop(app: Arc<PortalApp>) {
    loop {
        match app.next_cashu_direct().await {
            Ok(event) => info!("Received Cashu direct: {:?}", event),
            Err(e) => {
                error!("Cashu direct loop error: {:?}", e);
                break;
            }
        }
    }
}

async fn nip46_loop(app: Arc<PortalApp>) {
    loop {
        match app.next_nip46_request().await {
            Ok(event) => {
                info!("Received auth challenge: {:?}", event);

                let status = NostrConnectResponseStatus::Declined { reason: Some("Don't".to_string()) };
                if let Err(e) = app.reply_nip46_request(event, status).await {
                    error!("Failed to reply to auth challenge: {:?}", e);
                }
            }
            Err(e) => {
                error!("Auth challenge loop error: {:?}", e);
                break;
            }
        }
    }
}