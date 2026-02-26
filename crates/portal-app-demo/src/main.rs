//! portal-app-demo: minimal demo for protocol testing.
//!
//! Single app instance from config at ~/.portal-app-demo/config.toml.
//! Breez data is stored under ~/.portal-app-demo/breez.
//!
//! Usage: `cargo run -p portal-app-demo` then open http://127.0.0.1:3030

mod config;
mod constants;
mod error;

use std::net::SocketAddr;
use std::sync::Arc;

use app::{
    CallbackError, IncomingPaymentRequest, Mnemonic, Nsec, PortalApp, RelayStatus, RelayStatusListener,
    RelayUrl, SinglePaymentRequest,
};
use app::nwc::MakeInvoiceResponse;
use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use error::ApiError;
use error::ApiError as Err;
use portal::protocol::model::payment::{
    InvoiceRequestContentWithKey, PaymentResponseContent, PaymentStatus,
};
use portal_wallet::PortalWallet;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

// -----------------------------------------------------------------------------
// State
// -----------------------------------------------------------------------------

struct AppState {
    app: RwLock<Option<Arc<PortalApp>>>,
    pubkey: RwLock<Option<String>>,
    pubkey_hex: RwLock<Option<String>>,
    payment_wallet: RwLock<Option<Arc<dyn PortalWallet>>>,
    pending_request: RwLock<Option<SinglePaymentRequest>>,
    pending_invoice_request: RwLock<Option<InvoiceRequestContentWithKey>>,
    config_path: String,
}

// -----------------------------------------------------------------------------
// Request/response types
// -----------------------------------------------------------------------------

#[derive(Serialize)]
struct StatusResponse {
    ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pubkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pubkey_hex: Option<String>,
    payment_wallet_configured: bool,
    /// Balance in msat (when payment wallet is configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    balance_msat: Option<u64>,
    /// Config file path (e.g. ~/.portal-app-demo/config.toml).
    config_path: String,
}

#[derive(Serialize)]
struct PaymentRequestDto {
    request_id: String,
    event_id: String,
    /// Raw amount (msat for Lightning, or smallest fiat unit e.g. cents for USD).
    amount: u64,
    /// Human-readable amount and currency, e.g. "10.50 USD" or "100,000 msat (100 sats)".
    amount_formatted: String,
    /// True when currency is fiat (e.g. USD).
    is_fiat: bool,
    currency: String,
    /// When fiat, optional exchange rate (rate + source) for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    exchange_rate: Option<ExchangeRateDto>,
    /// When fiat and exchange_rate present, approximate equivalent in sats (for display).
    #[serde(skip_serializing_if = "Option::is_none")]
    equivalent_sats: Option<u64>,
    description: Option<String>,
    service_key: String,
    recipient: String,
    expires_at_secs: u64,
    invoice: Option<String>,
}

#[derive(Serialize)]
struct ExchangeRateDto {
    pub rate: f64,
    pub source: String,
}

#[derive(serde::Deserialize)]
struct RejectBody {
    reason: Option<String>,
}

#[derive(Serialize)]
struct InvoiceRequestDto {
    request_id: String,
    amount: u64,
    amount_formatted: String,
    is_fiat: bool,
    currency: String,
    description: Option<String>,
    recipient: String,
    expires_at_secs: u64,
}

#[derive(Deserialize)]
struct InvoiceReplyBody {
    invoice: String,
    payment_hash: Option<String>,
}

// -----------------------------------------------------------------------------
// Relay status listener (no-op for demo)
// -----------------------------------------------------------------------------

struct NoOpRelayStatusListener;

#[async_trait::async_trait]
impl RelayStatusListener for NoOpRelayStatusListener {
    async fn on_relay_status_change(
        &self,
        _relay_url: RelayUrl,
        _status: RelayStatus,
    ) -> Result<(), CallbackError> {
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Entrypoint: load config, create single app instance
// -----------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let settings = config::Settings::load()?;
    settings.validate()?;

    let config_path = constants::portal_app_demo_dir()?
        .join("config.toml")
        .display()
        .to_string();
    log::info!("Config: {}", config_path);

    let keypair = build_keypair_from_config(&settings)?;
    let relays = settings.nostr.relays.clone();
    let listener = Arc::new(NoOpRelayStatusListener);
    let app = PortalApp::new(keypair.clone(), relays, listener)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let pubkey_str = portal::nostr::nips::nip19::ToBech32::to_bech32(&*keypair.public_key())
        .unwrap_or_else(|_| app::key_to_hex(keypair.public_key()));
    let pubkey_hex_str = app::key_to_hex(keypair.public_key());
    let payment_wallet = settings.build_payment_wallet().await?;

    let state = Arc::new(AppState {
        app: RwLock::new(Some(Arc::clone(&app))),
        pubkey: RwLock::new(Some(pubkey_str.clone())),
        pubkey_hex: RwLock::new(Some(pubkey_hex_str)),
        payment_wallet: RwLock::new(payment_wallet),
        pending_request: RwLock::new(None),
        pending_invoice_request: RwLock::new(None),
        config_path: config_path.clone(),
    });

    let app_listen = Arc::clone(&app);
    tokio::spawn(async move {
        let _ = app_listen.listen().await;
    });
    let state_payment = Arc::clone(&state);
    let app_payment = Arc::clone(&app);
    tokio::spawn(async move {
        payment_request_loop(app_payment, state_payment).await;
    });
    let state_invoice = Arc::clone(&state);
    let app_invoice = Arc::clone(&app);
    tokio::spawn(async move {
        invoice_request_loop(app_invoice, state_invoice).await;
    });

    log::info!("Wallet ready. Pubkey: {}", pubkey_str);

    let router = Router::new()
        .route("/", get(serve_index))
        .route("/api/status", get(api_status))
        .route("/api/payment-request", get(api_payment_request))
        .route("/api/payment-request/accept", post(api_accept))
        .route("/api/payment-request/reject", post(api_reject))
        .route("/api/invoice-request", get(api_invoice_request))
        .route("/api/invoice-request/reply", post(api_invoice_request_reply))
        .route("/api/invoice-request/reject", post(api_invoice_request_reject))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], settings.info.listen_port));
    log::info!("portal-app-demo listening on http://{}/", addr);
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}

fn build_keypair_from_config(settings: &config::Settings) -> anyhow::Result<Arc<app::Keypair>> {
    if let Some(ref words) = settings.identity.mnemonic {
        let words = words.trim();
        if !words.is_empty() {
            let mnemonic = Mnemonic::new(words).map_err(|e| anyhow::anyhow!("Invalid mnemonic: {:?}", e))?;
            let kp = mnemonic.get_keypair().map_err(|e| anyhow::anyhow!("Keypair: {:?}", e))?;
            return Ok(Arc::new(kp));
        }
    }
    if let Some(ref nsec) = settings.identity.nsec {
        let nsec = nsec.trim();
        if !nsec.is_empty() {
            let n = Nsec::new(nsec).map_err(|e| anyhow::anyhow!("Invalid nsec: {:?}", e))?;
            return Ok(Arc::new(n.get_keypair()));
        }
    }
    anyhow::bail!("Set identity.mnemonic or identity.nsec in config")
}

// -----------------------------------------------------------------------------
// Handlers
// -----------------------------------------------------------------------------

async fn serve_index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

async fn api_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let pubkey = state.pubkey.read().await.clone();
    let pubkey_hex = state.pubkey_hex.read().await.clone();
    let payment_wallet = state.payment_wallet.read().await.clone();
    let payment_wallet_configured = payment_wallet.is_some();
    let balance_msat = match payment_wallet.as_ref() {
        Some(w) => w.get_balance().await.ok(),
        None => None,
    };
    Json(StatusResponse {
        ready: pubkey.is_some(),
        pubkey,
        pubkey_hex,
        payment_wallet_configured,
        balance_msat,
        config_path: state.config_path.clone(),
    })
}

async fn payment_request_loop(app: Arc<PortalApp>, state: Arc<AppState>) {
    loop {
        match app.next_payment_request().await {
            Ok(IncomingPaymentRequest::Single(request)) => {
                log::info!("Received single payment request: {:?}", request);
                *state.pending_request.write().await = Some(request);
            }
            Ok(IncomingPaymentRequest::Recurring(_)) => {
                log::debug!("Demo ignores recurring payment requests");
            }
            Err(e) => {
                log::error!("payment_request_loop error: {:?}", e);
                break;
            }
        }
    }
}

async fn invoice_request_loop(app: Arc<PortalApp>, state: Arc<AppState>) {
    loop {
        match app.next_invoice_request().await {
            Ok(request) => {
                log::info!("Received invoice request: {:?}", request);
                *state.pending_invoice_request.write().await = Some(request);
            }
            Err(e) => {
                log::error!("invoice_request_loop error: {:?}", e);
                break;
            }
        }
    }
}

fn invoice_request_to_dto(r: &InvoiceRequestContentWithKey) -> InvoiceRequestDto {
    use portal::protocol::model::payment::Currency;
    let c = &r.inner;
    let amount = c.amount;
    let (currency, amount_formatted, is_fiat) = match &c.currency {
        Currency::Millisats => {
            let sats = amount / 1000;
            (
                String::from("msat"),
                format!("{} msat ({} sats)", amount, sats),
                false,
            )
        }
        Currency::Fiat(code) => {
            let major = amount as f64 / 100.0;
            (
                code.clone(),
                format!("{:.2} {}", major, code),
                true,
            )
        }
    };
    InvoiceRequestDto {
        request_id: c.request_id.clone(),
        amount,
        amount_formatted,
        is_fiat,
        currency,
        description: c.description.clone(),
        recipient: r.recipient.to_string(),
        expires_at_secs: c.expires_at.as_u64(),
    }
}

fn single_request_to_dto(r: &SinglePaymentRequest) -> PaymentRequestDto {
    use portal::protocol::model::payment::{Currency, ExchangeRate};
    let amount = r.content.amount;
    let (currency, amount_formatted, is_fiat, exchange_rate, equivalent_sats) = match &r.content.currency {
        Currency::Millisats => {
            let sats = amount / 1000;
            let formatted = format!("{} msat ({} sats)", amount, sats);
            (String::from("msat"), formatted, false, None, None)
        }
        Currency::Fiat(code) => {
            // Fiat amount is typically in smallest unit (e.g. cents for USD): 1050 = 10.50 USD
            let major = amount as f64 / 100.0;
            let formatted = format!("{:.2} {}", major, code);
            let (exchange_rate_dto, equivalent_sats) = match &r.content.current_exchange_rate {
                Some(ExchangeRate { rate, source, .. }) => {
                    let eq_sats = (major * *rate) as u64;
                    (
                        Some(ExchangeRateDto {
                            rate: *rate,
                            source: source.clone(),
                        }),
                        Some(eq_sats),
                    )
                }
                None => (None, None),
            };
            (code.clone(), formatted, true, exchange_rate_dto, equivalent_sats)
        }
    };
    PaymentRequestDto {
        request_id: r.content.request_id.clone(),
        event_id: r.event_id.clone(),
        amount,
        amount_formatted,
        is_fiat,
        currency,
        exchange_rate,
        equivalent_sats,
        description: r.content.description.clone(),
        service_key: r.service_key.to_string(),
        recipient: r.recipient.to_string(),
        expires_at_secs: r.expires_at.as_u64(),
        invoice: Some(r.content.invoice.clone()),
    }
}

async fn api_payment_request(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Option<PaymentRequestDto>>, ApiError> {
    let guard = state.pending_request.read().await;
    Ok(Json(guard.as_ref().map(single_request_to_dto)))
}

async fn api_accept(State(state): State<Arc<AppState>>) -> Result<StatusCode, ApiError> {
    let app_guard = state.app.read().await;
    let app = app_guard.as_ref().ok_or_else(|| Err::bad_request("App not ready"))?;
    let request = state.pending_request.write().await.take().ok_or_else(|| Err::not_found("No pending payment request"))?;
    let payment_wallet = state.payment_wallet.read().await.clone();

    let request_id = request.content.request_id.clone();
    let invoice = request.content.invoice.clone();

    let approved = PaymentResponseContent {
        request_id: request_id.clone(),
        status: PaymentStatus::Approved,
    };
    app.reply_single_payment_request(request.clone(), approved)
        .await
        .map_err(|e| Err::internal(e.to_string()))?;

    let status = if let Some(pw) = payment_wallet {
        match pw.pay_invoice(invoice).await {
            Ok((preimage, _)) => PaymentResponseContent {
                request_id: request_id.clone(),
                status: PaymentStatus::Success {
                    preimage: Some(preimage),
                },
            },
            Err(e) => {
                log::error!("Payment failed: {}", e);
                PaymentResponseContent {
                    request_id: request_id.clone(),
                    status: PaymentStatus::Failed {
                        reason: Some(e.to_string()),
                    },
                }
            }
        }
    } else {
        PaymentResponseContent {
            request_id,
            status: PaymentStatus::Success {
                preimage: Some("demo-preimage-for-protocol-testing".to_string()),
            },
        }
    };

    app.reply_single_payment_request(request, status)
        .await
        .map_err(|e| Err::internal(e.to_string()))?;

    log::info!("Payment request accepted");
    Ok(StatusCode::OK)
}

async fn api_reject(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RejectBody>,
) -> Result<StatusCode, ApiError> {
    let app_guard = state.app.read().await;
    let app = app_guard.as_ref().ok_or_else(|| Err::bad_request("App not ready"))?;
    let request = state.pending_request.write().await.take().ok_or_else(|| Err::not_found("No pending payment request"))?;

    let content = PaymentResponseContent {
        request_id: request.content.request_id.clone(),
        status: PaymentStatus::Rejected {
            reason: body.reason,
        },
    };
    app.reply_single_payment_request(request, content)
        .await
        .map_err(|e| Err::internal(e.to_string()))?;

    log::info!("Payment request rejected");
    Ok(StatusCode::OK)
}

async fn api_invoice_request(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Option<InvoiceRequestDto>>, ApiError> {
    let guard = state.pending_invoice_request.read().await;
    Ok(Json(guard.as_ref().map(invoice_request_to_dto)))
}

async fn api_invoice_request_reply(
    State(state): State<Arc<AppState>>,
    Json(body): Json<InvoiceReplyBody>,
) -> Result<StatusCode, ApiError> {
    let app_guard = state.app.read().await;
    let app = app_guard.as_ref().ok_or_else(|| Err::bad_request("App not ready"))?;
    let request = state
        .pending_invoice_request
        .write()
        .await
        .take()
        .ok_or_else(|| Err::not_found("No pending invoice request"))?;

    let invoice = body.invoice.trim();
    let (invoice, payment_hash) = if !invoice.is_empty() {
        (invoice.to_string(), body.payment_hash)
    } else {
        // Try to create invoice via wallet if amount is in msat
        let payment_wallet = state.payment_wallet.read().await.clone();
        match payment_wallet.as_ref() {
            Some(wallet) => {
                use portal::protocol::model::payment::Currency;
                match &request.inner.currency {
                    Currency::Millisats => {
                        let inv = wallet
                            .make_invoice(request.inner.amount, request.inner.description.clone())
                            .await
                            .map_err(|e| Err::internal(e.to_string()))?;
                        (inv, None)
                    }
                    _ => return Err(Err::bad_request("Provide invoice in body for fiat requests, or use msat")),
                }
            }
            None => return Err(Err::bad_request("No payment wallet configured; provide invoice in body")),
        }
    };

    app.reply_invoice_request(
        request,
        MakeInvoiceResponse {
            invoice,
            payment_hash,
        },
    )
    .await
    .map_err(|e| Err::internal(e.to_string()))?;

    log::info!("Invoice request replied");
    Ok(StatusCode::OK)
}

/// Reject: clear the pending invoice request without sending any reply (requester will time out).
async fn api_invoice_request_reject(State(state): State<Arc<AppState>>) -> Result<StatusCode, ApiError> {
    let _ = state.pending_invoice_request.write().await.take();
    log::info!("Invoice request rejected (no reply sent)");
    Ok(StatusCode::OK)
}
