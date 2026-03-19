use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use portal::protocol::LocalKeypair;
use portal_sdk::PortalSDK;
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn, error};

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
    public_key: String,
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

    // Constant-time comparison to prevent timing attacks
    use subtle::ConstantTimeEq;
    let valid = token.as_bytes().ct_eq(state.settings.auth.auth_token.as_bytes());
    if !bool::from(valid) {
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

    let public_key = keypair.public_key().to_string();
    info!("Running with keypair: {}", public_key);

    // Initialize SDK
    let sdk = PortalSDK::new(keypair, config.nostr.relays.clone()).await?;

    // Initialize the wallet
    let wallet = config.build_wallet().await?;

    let listen_port = config.info.listen_port;

    // Resolve database path (relative paths are relative to ~/.portal-rest/)
    let db_path = if std::path::Path::new(&config.database.path).is_relative() {
        let rest_dir = constants::portal_rest_dir()?;
        std::fs::create_dir_all(&rest_dir)?;
        rest_dir
            .join(&config.database.path)
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?
            .to_string()
    } else {
        config.database.path.clone()
    };

    // Create event store with SQLite persistence
    let event_store = events::EventStore::new(&db_path, config.webhook.clone())?;

    // Create app state
    let state = AppState {
        sdk: Arc::new(sdk),
        public_key,
        settings: config,
        wallet,
        market_api: portal_rates::MarketAPI::new().expect("Failed to create market API"),
        events: event_store,
    };

    // ---- Startup recovery: resume in-flight streams ----
    {
        let in_flight = state.events.get_in_flight_streams().await;
        if !in_flight.is_empty() {
            info!("Recovering {} in-flight stream(s) from database", in_flight.len());
        }
        for stream in in_flight {
            match stream.stream_type.as_str() {
                "single_payment" => {
                    if let Some(events::StreamMetadata::SinglePayment { invoice, expires_at_secs }) =
                        stream.metadata
                    {
                        if let Some(wallet) = state.wallet.clone() {
                            let events_store = state.events.clone();
                            let sid = stream.stream_id.clone();
                            info!("Recovering single_payment stream {sid}");

                            tokio::spawn(async move {
                                let now_secs = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                let expired = now_secs > expires_at_secs;

                                if expired {
                                    // Check if it was paid before expiry
                                    match wallet.is_invoice_paid(invoice).await {
                                        Ok((true, preimage)) => {
                                            events_store
                                                .push(
                                                    &sid,
                                                    response::NotificationData::PaymentStatusUpdate {
                                                        status: response::InvoiceStatus::Paid {
                                                            preimage,
                                                        },
                                                    },
                                                )
                                                .await;
                                        }
                                        _ => {
                                            events_store
                                                .push(
                                                    &sid,
                                                    response::NotificationData::PaymentStatusUpdate {
                                                        status: response::InvoiceStatus::Timeout,
                                                    },
                                                )
                                                .await;
                                        }
                                    }
                                } else {
                                    // Still pending — restart monitoring loop
                                    loop {
                                        let now_secs = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs();
                                        if now_secs > expires_at_secs {
                                            events_store
                                                .push(
                                                    &sid,
                                                    response::NotificationData::PaymentStatusUpdate {
                                                        status: response::InvoiceStatus::Timeout,
                                                    },
                                                )
                                                .await;
                                            break;
                                        }
                                        match wallet.is_invoice_paid(invoice.clone()).await {
                                            Ok((true, preimage)) => {
                                                events_store
                                                    .push(
                                                        &sid,
                                                        response::NotificationData::PaymentStatusUpdate {
                                                            status: response::InvoiceStatus::Paid {
                                                                preimage,
                                                            },
                                                        },
                                                    )
                                                    .await;
                                                break;
                                            }
                                            Ok((false, _)) => {
                                                tokio::time::sleep(
                                                    tokio::time::Duration::from_millis(1000),
                                                )
                                                .await;
                                            }
                                            Err(e) => {
                                                error!("Recovery: failed to check invoice for {sid}: {e}");
                                                events_store
                                                    .push(
                                                        &sid,
                                                        response::NotificationData::PaymentStatusUpdate {
                                                            status: response::InvoiceStatus::Error {
                                                                reason: e.to_string(),
                                                            },
                                                        },
                                                    )
                                                    .await;
                                                break;
                                            }
                                        }
                                    }
                                }
                            });
                        } else {
                            warn!("Cannot recover single_payment stream {} — no wallet configured", stream.stream_id);
                            state
                                .events
                                .update_stream_status(
                                    &stream.stream_id,
                                    events::StreamStatus::Failed,
                                )
                                .await;
                        }
                    } else {
                        warn!("Cannot recover single_payment stream {} — missing metadata", stream.stream_id);
                        state
                            .events
                            .update_stream_status(
                                &stream.stream_id,
                                events::StreamStatus::Failed,
                            )
                            .await;
                    }
                }
                "recurring_payment_close" => {
                    // This stream is backed by the long-running SDK listener started below, so
                    // there's nothing to "recover" here. Also avoid noisy warnings on restart.
                }
                "key_handshake" | "authenticate_key" | "recurring_payment"
                | "invoice_request" | "cashu_request" | "raw_payment" => {
                    // These streams rely on ephemeral SDK conversation state and
                    // cannot be resumed after restart. Mark as failed.
                    warn!(
                        "Cannot recover {} stream {} — marking as failed",
                        stream.stream_type, stream.stream_id
                    );
                    state
                        .events
                        .update_stream_status(
                            &stream.stream_id,
                            events::StreamStatus::Failed,
                        )
                        .await;
                }
                other => {
                    warn!("Unknown stream type '{other}' for stream {} — marking as failed", stream.stream_id);
                    state
                        .events
                        .update_stream_status(
                            &stream.stream_id,
                            events::StreamStatus::Failed,
                        )
                        .await;
                }
            }
        }
    }

    // Always start the recurring payment close listener at startup
    {
        let stream_id = "recurring-payment-close";
        state
            .events
            .create_stream(stream_id, "recurring_payment_close", None)
            .await;
        match state.sdk.listen_closed_recurring_payment().await {
            Ok(notification_stream) => {
                info!("Started recurring payment close listener (stream {stream_id})");
                let events_store = state.events.clone();
                let sid = stream_id.to_string();
                tokio::spawn(async move {
                    let mut stream = notification_stream;
                    while let Some(Ok(event)) = stream.next().await {
                        events_store
                            .push(
                                &sid,
                                response::NotificationData::ClosedRecurringPayment {
                                    reason: event.content.reason,
                                    subscription_id: event.content.subscription_id,
                                    main_key: event.main_key.to_string(),
                                    recipient: event.recipient.to_string(),
                                },
                            )
                            .await;
                    }
                });
            }
            Err(e) => {
                error!("Failed to start recurring payment close listener: {e}");
            }
        }
    }

    // Apply profile from config if any fields are set
    if state.settings.profile.is_set() {
        let profile = portal::conversation::profile::Profile {
            name: state.settings.profile.name.clone(),
            display_name: state.settings.profile.display_name.clone(),
            picture: state.settings.profile.picture.clone(),
            nip05: state.settings.profile.nip05.clone(),
        };
        match state.sdk.set_profile(profile).await {
            Ok(_) => info!("Profile set from config"),
            Err(e) => error!("Failed to set profile from config: {e}"),
        }
    }

    // Public routes (no auth): health, version, and NIP-05 well-known for Docker/orchestrators and support.
    let public = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/version", get(handlers::version))
        .route("/well-known/nostr.json", get(handlers::well_known_nostr_json));

    // Authenticated REST API routes
    let api = Router::new()
        .route("/info", get(handlers::info))
        // Key handshake & auth
        .route("/key-handshake", post(handlers::new_key_handshake_url))
        .route("/authenticate-key", post(handlers::authenticate_key))
        // Payments
        .route("/payments/single", post(handlers::request_single_payment))
        .route("/payments/raw", post(handlers::request_payment_raw))
        .route("/payments/recurring", post(handlers::request_recurring_payment))
        .route("/payments/recurring/close", post(handlers::close_recurring_payment))
        // Profiles
        .route("/profile/:main_key", get(handlers::fetch_profile))

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
