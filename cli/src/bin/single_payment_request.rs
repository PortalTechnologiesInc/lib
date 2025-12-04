use cli::{CliError, create_app_instance};
use portal::protocol::model::payment::{Currency, SinglePaymentRequestContent};

#[tokio::main]
async fn main() -> Result<(), CliError> {
    env_logger::init();

    let relays = vec!["wss://relay.getportal.cc".to_string(), "wss://relay.damus.io".to_string()];

    let (_receiver_key, app) = create_app_instance(
        "Receiver",
        "mass derive myself benefit shed true girl orange family spawn device theme",
        relays.clone(),
    )
    .await?;

    let npub = "0a518ae8474a9959283f8153dbae8440a7c203e14131bcf8a764167562eb96b1";
    let invoice = "lnbc300n1p5nr0z8pp5gr654uklxctfk9g2hzarejnvyleddlk9z22v6hffaa0lls6mqcxssp5c2fywt5x7cxat6zw6ksgc38xzazwl56lpjele60p8e2xa3g67ezsxq9z0rgqnp4qvyndeaqzman7h898jxm98dzkm0mlrsx36s93smrur7h0azyyuxc5rzjq25carzepgd4vqsyn44jrk85ezrpju92xyrk9apw4cdjh6yrwt5jgqqqqrt49lmtcqqqqqqqqqqq86qq9qrzjqg3emy2wygxcwgl2vhgex0xl7nj2fkw9a0vwxv4um8dusssvqc0gnapyqr6zgqqqq8hxk2qqae4jsqyugqcqzpgdqv2pshjmt9de6q9qyyssqpfr5w3tq9cukwqsx2appujgk22dj0gldwj6hpvxdnaahltkueky3ejmet42qg3syg9g0pvag70uj3wz3n5hrjqsqm4p4rjqc9alks5sqc2kqgc";
    let amount = 30;

    app.single_payment_request(npub, SinglePaymentRequestContent {
        amount: amount * 1000,
        invoice: invoice.into(),
        auth_token: None,
        currency: Currency::Millisats,
        current_exchange_rate: None,
        description: Some("".into()),
        request_id: "test".into(),
        subscription_id: None,
        expires_at: portal::protocol::model::Timestamp::now_plus_seconds(86400),
    }).await?;

    Ok(())
}