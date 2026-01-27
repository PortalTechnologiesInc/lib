use portal_cli::CliError;
use portal_cli::create_app_instance;

#[tokio::main]
async fn main() -> Result<(), CliError> {
    env_logger::init();

    let (keypair0, app0) = create_app_instance(
        "Sender",
        "mass derive myself benefit shed true girl orange family spawn device theme",
        vec!["wss://relay.nostr.net".to_string()],
    )
    .await?;

    let (keypair1, app1) = create_app_instance(
        "Receiver",
        "draft sunny old taxi chimney ski tilt suffer subway bundle once story",
        vec!["wss://relay.nostr.net".to_string()],
    )
    .await?;

    let token = keypair0.issue_jwt(keypair1.public_key(), 1 as i64)?;

    log::info!("Token: {}", token);

    let claims = keypair1.verify_jwt(keypair0.public_key(), &token)?;
    log::info!("Claims: {:?}", claims);

    Ok(())
}
