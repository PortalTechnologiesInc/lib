use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use portal::protocol::LocalKeypair;
use portal_sdk::PortalSDK;
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

mod command;
mod config;
mod constants;
mod events;
mod handlers;
mod response;
mod webhook;

// Re-export the portal types that we need
pub use portal::nostr::key::PublicKey;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use portal_wallet::PortalWallet;
use portal_macros::fetch_git_hash;

/// Build-time version from Cargo.toml (used for Docker image tagging and runtime /version endpoint).
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Git commit hash at build time (from portal_macros::fetch_git_hash! or PORTAL_GIT_HASH env).
const GIT_COMMIT: &str = fetch_git_hash!();

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("SDK error: {0}")]
    SdkError(#[from] portal_sdk::PortalSDKError),

    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

impl From<ApiError> for (StatusCode, Json<ErrorResponse>) {
    fn from(error: ApiError) -> Self {
        let status = match &error {
            ApiError::AuthenticationError(_) => StatusCode::UNAUTHORIZED,
            ApiError::SdkError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::AnyhowError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (
            status,
            Json(ErrorResponse {
                error: error.to_string(),
            }),
        )
    }
}

#[derive(Clone)]
pub struct AppState {
    sdk: Arc<PortalSDK>,
    settings: config::Settings,
    wallet: Option<Arc<dyn PortalWallet>>,
    market_api: Arc<portal_rates::MarketAPI>,
    events: events::EventStore,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn auth_middleware<B>(
    State(state): State<AppState>,
    req: Request<B>,
    next: Next<B>,
) -> std::result::Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| -> (StatusCode, Json<ErrorResponse>) {
            ApiError::AuthenticationError("Missing Authorization header".to_string()).into()
        })?;

    let token = auth_header.strip_prefix("Bearer ").ok_or_else(
        || -> (StatusCode, Json<ErrorResponse>) {
            ApiError::AuthenticationError("Invalid Authorization header format".to_string()).into()
        },
    )?;

    if token != state.settings.auth.auth_token {
        return Err(ApiError::AuthenticationError("Invalid token".to_string()).into());
    }

    Ok(next.run(req).await)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "task-tracing")]
    console_subscriber::init();

    // Set up logging (default to info if RUST_LOG is not set)
    #[cfg(not(feature = "task-tracing"))]
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with(tracing_subscriber::fmt::Layer::default().compact())
        .init();

    let config = config::Settings::load()?;
    info!(
        listen_port = config.info.listen_port,
        wallet = format!("{:?}", config.wallet.ln_backend).to_lowercase(),
        relays = config.nostr.relays.join(","),
        webhook_url = config.webhook.url.as_deref().unwrap_or("(none)"),
        "Config loaded",
    );

    // Settings validation
    config.validate()?;

    let keys = portal::nostr::key::Keys::from_str(&config.nostr.private_key)?;

    // Initialize keypair from environment
    let keypair = LocalKeypair::new(
        keys,
        config
            .nostr
            .subkey_proof
            .clone()
            .map(|s| serde_json::from_str(&s).expect("Failed to parse subkey proof")),
    );

    info!("Running with keypair: {}", keypair.public_key());

    // Initialize SDK
    let sdk = PortalSDK::new(keypair, config.nostr.relays.clone()).await?;

    // Initialize the wallet
    let wallet = config.build_wallet().await?;

    let listen_port = config.info.listen_port;

    // Create event store (for polling + webhooks)
    let event_store = events::EventStore::new(config.webhook.clone());

    // Create app state
    let state = AppState {
        sdk: Arc::new(sdk),
        settings: config,
        wallet,
        market_api: portal_rates::MarketAPI::new().expect("Failed to create market API"),
        events: event_store,
    };

    // Public routes (no auth): health and version for Docker/orchestrators and support.
    let public = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/version", get(handlers::version));

    // Authenticated REST API routes
    let api = Router::new()
        // Key handshake & auth
        .route("/key-handshake", post(handlers::new_key_handshake_url))
        .route("/authenticate-key", post(handlers::authenticate_key))
        // Payments
        .route("/payments/single", post(handlers::request_single_payment))
        .route("/payments/raw", post(handlers::request_payment_raw))
        .route("/payments/recurring", post(handlers::request_recurring_payment))
        .route("/payments/recurring/close", post(handlers::close_recurring_payment))
        .route("/payments/recurring/listen", post(handlers::listen_closed_recurring_payment))
        // Profiles
        .route("/profile/:main_key", get(handlers::fetch_profile))
        .route("/profile", put(handlers::set_profile))
        // Invoices
        .route("/invoices/request", post(handlers::request_invoice))
        .route("/invoices/pay", post(handlers::pay_invoice))
        // JWT
        .route("/jwt/issue", post(handlers::issue_jwt))
        .route("/jwt/verify", post(handlers::verify_jwt))
        // Cashu
        .route("/cashu/request", post(handlers::request_cashu))
        .route("/cashu/send-direct", post(handlers::send_cashu_direct))
        .route("/cashu/mint", post(handlers::mint_cashu))
        .route("/cashu/burn", post(handlers::burn_cashu))
        // Relays
        .route("/relays", post(handlers::add_relay))
        .route("/relays", delete(handlers::remove_relay))
        // Calendar
        .route("/calendar/next-occurrence", post(handlers::calculate_next_occurrence))
        // NIP-05
        .route("/nip05/:nip05", get(handlers::fetch_nip05_profile))
        // Wallet
        .route("/wallet/info", get(handlers::get_wallet_info))
        // Event polling
        .route("/events/:stream_id", get(handlers::get_events))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let app = Router::new()
        .merge(public)
        .merge(api)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], listen_port));
    info!("Starting server on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
