use portal_cli::CliError;



#[tokio::main]
async fn main() -> Result<(), CliError> {
    env_logger::init();

    let git_hash = portal_macros::fetch_git_hash!();
    println!("Git hash: {}", git_hash);

    Ok(())
}