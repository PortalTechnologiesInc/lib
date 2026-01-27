use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use axum::{
    extract::{State, WebSocketUpgrade},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Json, Router,
};
use portal::protocol::LocalKeypair;
use portal_sdk::PortalSDK;
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

mod command;
mod config;
mod response;
mod ws;

// Re-export the portal types that we need
pub use portal::nostr::key::PublicKey;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use portal_wallet::{BreezSparkWallet, NwcWallet, PortalWallet};

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
struct AppState {
    sdk: Arc<PortalSDK>,
    settings: config::Settings,
    wallet: Option<Arc<dyn PortalWallet>>,
    market_api: Arc<portal_rates::MarketAPI>,
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
    // Skip authentication for WebSocket upgrade requests
    // WebSockets will handle their own authentication via the initial message
    if req.headers().contains_key("upgrade") {
        return Ok(next.run(req).await);
    }

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

async fn health_check() -> &'static str {
    "OK"
}

async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "task-tracing")]
    console_subscriber::init();

    // Set up logging
    #[cfg(not(feature = "task-tracing"))]
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::Layer::default().compact())
        .init();

    let config = config::Settings::load()?;
    info!("Config loaded: {:?}", config);

    // Settings validation
    config.validate()?;

    let keys = portal::nostr::key::Keys::from_str(&config.nostr.private_key)?;

    // Initialize keypair from environment
    // LocalKeypair doesn't have from_hex, need to use the correct initialization method
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

    // Initialize the wallet, based on the env variables set

    let wallet = config.build_wallet().await?;


    let listen_port = config.info.listen_port;

    // Create app state
    let state = AppState {
        sdk: Arc::new(sdk),
        settings: config,
        wallet,
        market_api: portal_rates::MarketAPI::new().expect("Failed to create market API"),
    };

    // Create router with middleware
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(handle_ws_upgrade))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
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
