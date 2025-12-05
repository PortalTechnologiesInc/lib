use cli::{CliError, create_app_instance};
use portal::protocol::model::payment::{Currency, SinglePaymentRequestContent};

#[tokio::main]
async fn main() -> Result<(), CliError> {
    env_logger::init();

    let relays = vec![
        "wss://relay.getportal.cc".to_string(),
        "wss://relay.damus.io".to_string(),
    ];

    let (_receiver_key, app) = create_app_instance(
        "Receiver",
        "mass derive myself benefit shed true girl orange family spawn device theme",
        relays.clone(),
    )
    .await?;

    let npub = "ae35195222dbcc280d85a966204bc0e1ce743e99cbcb76216c922edf90cdd3e5";
    let invoice = "lnbc100n1p5nrm73pp5qjsms2tw3mm0jjn8jqex2l9mshnl978d779nstg08fcn2yq3256ssp5rdkyjl7lyewqpfhzyl0w5z6ferjuwxk67p587qv49kmq9zd9vskqxq9z0rgqnp4qvyndeaqzman7h898jxm98dzkm0mlrsx36s93smrur7h0azyyuxc5rzjq25carzepgd4vqsyn44jrk85ezrpju92xyrk9apw4cdjh6yrwt5jgqqqqrt49lmtcqqqqqqqqqqq86qq9qrzjqvx2wstnex2mhar83v3c8gwrrnpek8kfyjznh2ulxt35q03tzrzf3apyqr6zgqqqq8hxk2qqae4jsqyugqcqzpgdqv2pshjmt9de6q9qyyssqcsvavq75ksw6u03nunfyjyeygxk27mph4406vsf9jmdqgcaz59k5emrnh5dctvnzshd6d5x6xrqckrq8rn9a2rc83ywe0sut5829ecqpxrtegn";
    let amount = 10;

    app.single_payment_request(
        npub,
        SinglePaymentRequestContent {
            amount: amount * 1000,
            invoice: invoice.into(),
            auth_token: None,
            currency: Currency::Millisats,
            current_exchange_rate: None,
            description: Some("".into()),
            request_id: "test".into(),
            subscription_id: None,
            expires_at: portal::protocol::model::Timestamp::now_plus_seconds(86400),
        },
    )
    .await?;

    Ok(())
}
