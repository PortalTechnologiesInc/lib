use std::net::SocketAddr;
use std::sync::Arc;
use std::{env, str::FromStr};

use axum::{
    extract::{State, WebSocketUpgrade},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Json, Router,
};
use portal::protocol::LocalKeypair;
use sdk::PortalSDK;
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, Level};

mod ws;
mod command;

// Re-export the portal types that we need
pub use portal::nostr::key::PublicKey;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Environment error: {0}")]
    EnvError(String),

    #[error("SDK error: {0}")]
    SdkError(#[from] sdk::PortalSDKError),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

impl From<ApiError> for (StatusCode, Json<ErrorResponse>) {
    fn from(error: ApiError) -> Self {
        let status = match &error {
            ApiError::AuthenticationError(_) => StatusCode::UNAUTHORIZED,
            ApiError::EnvError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::SdkError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
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

type Result<T> = std::result::Result<T, ApiError>;

#[derive(Clone)]
struct AppState {
    sdk: Arc<PortalSDK>,
    auth_token: String,
    nwc: Option<Arc<nwc::NWC>>,
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

    if token != state.auth_token {
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

    // Load .env file if it exists
    dotenv::dotenv().ok();

    // Set up logging
    #[cfg(not(feature = "task-tracing"))]
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::Layer::default().compact())
        .init();

    // Get environment variables
    let auth_token = env::var("AUTH_TOKEN").expect("AUTH_TOKEN environment variable is required");
    let nwc_url = env::var("NWC_URL").ok();
    let nostr_key = env::var("NOSTR_KEY").expect("NOSTR_KEY environment variable is required");
    let nostr_subkey_proof = env::var("NOSTR_SUBKEY_PROOF").ok();

    // Only use default relays if NOSTR_RELAYS is not set or empty
    let relays: Vec<String> = match env::var("NOSTR_RELAYS") {
        Ok(relays_str) if !relays_str.trim().is_empty() => relays_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect(),
        _ => {
            // Default relays as fallback
            info!("NOSTR_RELAYS not set or empty, using default relays");
            vec![
                "wss://relay.nostr.net".to_string(),
                "wss://relay.damus.io".to_string(),
            ]
        }
    };

    let keys = portal::nostr::key::Keys::from_str(&nostr_key)?;

    // Initialize keypair from environment
    // LocalKeypair doesn't have from_hex, need to use the correct initialization method
    let keypair = LocalKeypair::new(
        keys,
        nostr_subkey_proof.map(|s| serde_json::from_str(&s).expect("Failed to parse subkey proof")),
    );

    info!("Running with keypair: {}", keypair.public_key());

    // Initialize SDK
    let sdk = PortalSDK::new(keypair, relays).await?;

    // Initialize NWC
    let nwc =
        nwc_url.map(|url| Arc::new(nwc::NWC::new(url.parse().expect("Failed to parse NWC_URL"))));
    let nwc_clone = nwc.clone();

    tokio::spawn(async move {
        if let Some(nwc) = nwc_clone {
            let info = nwc.get_info().await;
            info!("NWC info: {:?}", info);
        }
    });

    // Create app state
    let state = AppState {
        sdk: Arc::new(sdk),
        auth_token,
        nwc,
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
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Starting server on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
