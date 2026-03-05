//! portal-app-demo: multi-session demo for protocol testing.
//!
//! Sessions are created via POST /api/session with key material.
//! Each session runs an actor task that handles payment/invoice requests.
//!
//! Usage: `cargo run -p portal-app-demo` then open http://127.0.0.1:3030

mod config;
mod constants;
mod error;

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use app::{
    parse_key_handshake_url, CallbackError, IncomingPaymentRequest, Mnemonic, Nsec, PortalApp,
    RelayStatus, RelayStatusListener, RelayUrl, SinglePaymentRequest,
};
use app::nwc::MakeInvoiceResponse;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, sse::{Event, KeepAlive, Sse}},
    routing::{get, post},
    Json, Router,
};
use error::ApiError;
use futures::StreamExt;
use portal::conversation::profile::Profile;
use portal::protocol::model::auth::AuthResponseStatus;
use portal::protocol::model::bindings::PublicKey;
use portal::utils::fetch_nip05_profile;
use portal::protocol::model::payment::{
    Currency, ExchangeRate, InvoiceRequestContentWithKey, PaymentResponseContent, PaymentStatus,
};
use portal_wallet::{NwcWallet, PortalWallet};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tokio::time::Instant;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::{Any, CorsLayer};

// ---------------------------------------------------------------------------
// AppConfig
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppConfig {
    listen_port: u16,
    default_relays: Vec<String>,
    max_sessions: usize,
    session_ttl_secs: u64,
}

// ---------------------------------------------------------------------------
// GlobalState
// ---------------------------------------------------------------------------

struct GlobalState {
    sessions: RwLock<HashMap<PublicKey, mpsc::Sender<ActorCmd>>>,
    config: AppConfig,
}

// ---------------------------------------------------------------------------
// Actor commands
// ---------------------------------------------------------------------------

enum ActorCmd {
    Status { reply: oneshot::Sender<StatusDto> },
    Subscribe { reply: oneshot::Sender<broadcast::Receiver<SseEvent>> },
    GetPayment { reply: oneshot::Sender<Option<PaymentRequestDto>> },
    AcceptPayment { reply: oneshot::Sender<Result<(), String>> },
    RejectPayment { reason: Option<String>, reply: oneshot::Sender<Result<(), String>> },
    GetInvoice { reply: oneshot::Sender<Option<InvoiceRequestDto>> },
    ReplyInvoice { invoice: Option<String>, reply: oneshot::Sender<Result<(), String>> },
    RejectInvoice { reply: oneshot::Sender<()> },
    RecentInvoices { reply: oneshot::Sender<Vec<InvoiceRequestLogEntry>> },
    Handshake { url: String, reply: oneshot::Sender<Result<(), String>> },
    FetchProfile { reply: oneshot::Sender<Result<Option<ProfileDto>, String>> },
    Ping,
}

// ---------------------------------------------------------------------------
// SSE events
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
enum SseEvent {
    PaymentRequest(PaymentRequestDto),
    InvoiceRequest(InvoiceRequestDto),
    InvoiceResult {
        request_id: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        invoice: Option<String>,
    },
    Error { message: String },
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
struct StatusDto {
    pubkey: String,
    pubkey_hex: String,
    payment_wallet_configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    balance_msat: Option<u64>,
}

#[derive(Serialize, Clone)]
struct ProfileDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    picture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nip05: Option<String>,
}

impl From<Profile> for ProfileDto {
    fn from(p: Profile) -> Self {
        ProfileDto {
            name: p.name,
            display_name: p.display_name,
            picture: p.picture,
            nip05: p.nip05,
        }
    }
}

#[derive(Serialize, Clone)]
struct PaymentRequestDto {
    request_id: String,
    event_id: String,
    amount: u64,
    amount_formatted: String,
    is_fiat: bool,
    currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    exchange_rate: Option<ExchangeRateDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    equivalent_sats: Option<u64>,
    description: Option<String>,
    service_key: String,
    recipient: String,
    expires_at_secs: u64,
    invoice: Option<String>,
}

#[derive(Serialize, Clone)]
struct ExchangeRateDto {
    pub rate: f64,
    pub source: String,
}

#[derive(Serialize, Clone)]
struct InvoiceRequestDto {
    request_id: String,
    amount: u64,
    amount_formatted: String,
    is_fiat: bool,
    currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    exchange_rate: Option<ExchangeRateDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    equivalent_sats: Option<u64>,
    description: Option<String>,
    recipient: String,
    expires_at_secs: u64,
}

#[derive(Clone, Serialize)]
struct InvoiceRequestLogEntry {
    request_id: String,
    amount_formatted: String,
    currency: String,
    is_fiat: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    equivalent_sats: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    at_secs: u64,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invoice: Option<String>,
}

const MAX_RECENT_INVOICE_REQUESTS: usize = 100;

// ---------------------------------------------------------------------------
// Request/response for POST /api/session
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateSessionRequest {
    nsec: Option<String>,
    mnemonic: Option<String>,
    relays: Vec<String>,
    nwc: Option<String>,
}

#[derive(Serialize)]
struct CreateSessionResponse {
    pubkey: String,
    pubkey_hex: String,
}

#[derive(Deserialize)]
struct RejectBody {
    reason: Option<String>,
}

#[derive(Deserialize)]
struct InvoiceReplyBody {
    invoice: Option<String>,
}

#[derive(Deserialize)]
struct HandshakeBody {
    url: String,
}

// ---------------------------------------------------------------------------
// Relay status listener (no-op)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// DTO helpers
// ---------------------------------------------------------------------------

fn single_request_to_dto(r: &SinglePaymentRequest) -> PaymentRequestDto {
    let amount = r.content.amount;
    let (currency, amount_formatted, is_fiat, exchange_rate, equivalent_sats) = match &r.content.currency {
        Currency::Millisats => {
            let sats = amount / 1000;
            (String::from("msat"), format!("{} msat ({} sats)", amount, sats), false, None, None)
        }
        Currency::Fiat(code) => {
            let major = amount as f64 / 100.0;
            let formatted = format!("{:.2} {}", major, code);
            let (er, eq) = match &r.content.current_exchange_rate {
                Some(ExchangeRate { rate, source, .. }) => {
                    let eq_sats = ((major / *rate) * 100_000_000.0) as u64;
                    (Some(ExchangeRateDto { rate: *rate, source: source.clone() }), Some(eq_sats))
                }
                None => (None, None),
            };
            (code.clone(), formatted, true, er, eq)
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

fn invoice_request_to_dto(r: &InvoiceRequestContentWithKey) -> InvoiceRequestDto {
    let c = &r.inner;
    let amount = c.amount;
    let (currency, amount_formatted, is_fiat, exchange_rate, equivalent_sats) = match &c.currency {
        Currency::Millisats => {
            let sats = amount / 1000;
            (String::from("msat"), format!("{} msat ({} sats)", amount, sats), false, None, None)
        }
        Currency::Fiat(code) => {
            let major = amount as f64 / 100.0;
            let formatted = format!("{:.2} {}", major, code);
            let (er, eq) = match &c.current_exchange_rate {
                Some(ExchangeRate { rate, source, .. }) => {
                    let eq_sats = ((major / *rate) * 100_000_000.0) as u64;
                    (Some(ExchangeRateDto { rate: *rate, source: source.clone() }), Some(eq_sats))
                }
                None => (None, None),
            };
            (code.clone(), formatted, true, er, eq)
        }
    };
    InvoiceRequestDto {
        request_id: c.request_id.clone(),
        amount,
        amount_formatted,
        is_fiat,
        currency,
        exchange_rate,
        equivalent_sats,
        description: c.description.clone(),
        recipient: r.recipient.to_string(),
        expires_at_secs: c.expires_at.as_u64(),
    }
}

fn invoice_request_amount_msat(r: &InvoiceRequestContentWithKey) -> Option<u64> {
    let c = &r.inner;
    match &c.currency {
        Currency::Millisats => Some(c.amount),
        Currency::Fiat(_) => {
            let major = c.amount as f64 / 100.0;
            c.current_exchange_rate.as_ref().map(|er| ((major / er.rate) * 100_000_000_000.0) as u64)
        }
    }
}

// ---------------------------------------------------------------------------
// Actor
// ---------------------------------------------------------------------------

async fn run_actor(
    pubkey: PublicKey,
    mut rx: mpsc::Receiver<ActorCmd>,
    app: Arc<PortalApp>,
    wallet: Option<Arc<dyn PortalWallet>>,
    global: Arc<GlobalState>,
) {
    let mut pending_payment: Option<SinglePaymentRequest> = None;
    let mut pending_invoice: Option<InvoiceRequestContentWithKey> = None;
    let mut recent_invoices: Vec<InvoiceRequestLogEntry> = vec![];
    let (sse_tx, _) = broadcast::channel::<SseEvent>(64);
    let ttl = Duration::from_secs(global.config.session_ttl_secs);
    let mut deadline = Instant::now() + ttl;

    // Start listening on the app
    let app_listen = Arc::clone(&app);
    tokio::spawn(async move {
        let _ = app_listen.listen().await;
    });

    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {
                log::info!("Session expired for {}", app::key_to_hex(pubkey));
                break;
            }
            result = app.next_payment_request() => {
                match result {
                    Ok(IncomingPaymentRequest::Single(request)) => {
                        log::info!("Payment request received");
                        let dto = single_request_to_dto(&request);
                        let _ = sse_tx.send(SseEvent::PaymentRequest(dto));
                        pending_payment = Some(request);
                    }
                    Ok(IncomingPaymentRequest::Recurring(_)) => {
                        log::debug!("Ignoring recurring payment request");
                    }
                    Err(e) => {
                        log::error!("next_payment_request error: {:?}", e);
                        let _ = sse_tx.send(SseEvent::Error { message: format!("Payment listener error: {}", e) });
                        break;
                    }
                }
            }
            result = app.next_invoice_request() => {
                match result {
                    Ok(request) => {
                        log::info!("Invoice request received");
                        // Deduplicate
                        let request_id = request.inner.request_id.as_str();
                        let now_secs = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        if recent_invoices.iter().any(|e| e.request_id == request_id && now_secs.saturating_sub(e.at_secs) < 30) {
                            log::debug!("Skipping duplicate invoice request: {}", request_id);
                            continue;
                        }

                        let dto = invoice_request_to_dto(&request);
                        let _ = sse_tx.send(SseEvent::InvoiceRequest(dto.clone()));

                        let amount_msat = invoice_request_amount_msat(&request);

                        // Auto-accept if wallet is configured
                        let (status, error, invoice_opt) = match (wallet.as_ref(), amount_msat) {
                            (Some(w), Some(msat)) if msat > 0 => {
                                match w.make_invoice(msat, request.inner.description.clone()).await {
                                    Ok(invoice) => {
                                        match app.reply_invoice_request(
                                            request.clone(),
                                            MakeInvoiceResponse { invoice: invoice.clone(), payment_hash: None },
                                        ).await {
                                            Ok(_) => {
                                                log::info!("Invoice request auto-accepted");
                                                ("accepted", None, Some(invoice))
                                            }
                                            Err(e) => {
                                                log::warn!("Invoice reply failed: {}", e);
                                                pending_invoice = Some(request);
                                                ("failed", Some(format!("Reply failed: {}", e)), None)
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!("make_invoice failed: {}", e);
                                        pending_invoice = Some(request);
                                        ("failed", Some(e.to_string()), None)
                                    }
                                }
                            }
                            _ => {
                                // No wallet or zero amount or no exchange rate — store pending
                                pending_invoice = Some(request);
                                continue; // Don't log yet, wait for manual action
                            }
                        };

                        let entry = InvoiceRequestLogEntry {
                            request_id: dto.request_id.clone(),
                            amount_formatted: dto.amount_formatted.clone(),
                            currency: dto.currency.clone(),
                            is_fiat: dto.is_fiat,
                            equivalent_sats: dto.equivalent_sats,
                            description: dto.description.clone(),
                            at_secs: now_secs,
                            status: status.to_string(),
                            error: error.clone(),
                            invoice: invoice_opt.clone(),
                        };

                        let _ = sse_tx.send(SseEvent::InvoiceResult {
                            request_id: dto.request_id.clone(),
                            status: status.to_string(),
                            error,
                            invoice: invoice_opt,
                        });

                        recent_invoices.insert(0, entry);
                        if recent_invoices.len() > MAX_RECENT_INVOICE_REQUESTS {
                            recent_invoices.truncate(MAX_RECENT_INVOICE_REQUESTS);
                        }
                    }
                    Err(e) => {
                        log::error!("next_invoice_request error: {:?}", e);
                        let _ = sse_tx.send(SseEvent::Error { message: format!("Invoice listener error: {}", e) });
                        break;
                    }
                }
            }
            Some(cmd) = rx.recv() => {
                deadline = Instant::now() + ttl;
                match cmd {
                    ActorCmd::Ping => {}
                    ActorCmd::Status { reply } => {
                        let pubkey_bech32 = portal::nostr::nips::nip19::ToBech32::to_bech32(&*pubkey)
                            .unwrap_or_else(|_| app::key_to_hex(pubkey));
                        let balance = match wallet.as_ref() {
                            Some(w) => w.get_balance().await.ok(),
                            None => None,
                        };
                        let _ = reply.send(StatusDto {
                            pubkey: pubkey_bech32,
                            pubkey_hex: app::key_to_hex(pubkey),
                            payment_wallet_configured: wallet.is_some(),
                            balance_msat: balance,
                        });
                    }
                    ActorCmd::Subscribe { reply } => {
                        let _ = reply.send(sse_tx.subscribe());
                    }
                    ActorCmd::GetPayment { reply } => {
                        let _ = reply.send(pending_payment.as_ref().map(single_request_to_dto));
                    }
                    ActorCmd::AcceptPayment { reply } => {
                        let Some(request) = pending_payment.take() else {
                            let _ = reply.send(Err("No pending payment request".into()));
                            continue;
                        };
                        let request_id = request.content.request_id.clone();
                        let invoice = request.content.invoice.clone();

                        let approved = PaymentResponseContent {
                            request_id: request_id.clone(),
                            status: PaymentStatus::Approved,
                        };
                        if let Err(e) = app.reply_single_payment_request(request.clone(), approved).await {
                            let _ = reply.send(Err(format!("Approve failed: {}", e)));
                            pending_payment = Some(request);
                            continue;
                        }

                        let final_status = if let Some(ref pw) = wallet {
                            match pw.pay_invoice(invoice).await {
                                Ok((preimage, _)) => PaymentResponseContent {
                                    request_id: request_id.clone(),
                                    status: PaymentStatus::Success { preimage: Some(preimage) },
                                },
                                Err(e) => {
                                    log::error!("Payment failed: {}", e);
                                    PaymentResponseContent {
                                        request_id: request_id.clone(),
                                        status: PaymentStatus::Failed { reason: Some(e.to_string()) },
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

                        if let Err(e) = app.reply_single_payment_request(request, final_status).await {
                            let _ = reply.send(Err(format!("Final reply failed: {}", e)));
                        } else {
                            let _ = reply.send(Ok(()));
                        }
                    }
                    ActorCmd::RejectPayment { reason, reply } => {
                        let Some(request) = pending_payment.take() else {
                            let _ = reply.send(Err("No pending payment request".into()));
                            continue;
                        };
                        let content = PaymentResponseContent {
                            request_id: request.content.request_id.clone(),
                            status: PaymentStatus::Rejected { reason },
                        };
                        match app.reply_single_payment_request(request, content).await {
                            Ok(_) => { let _ = reply.send(Ok(())); }
                            Err(e) => { let _ = reply.send(Err(e.to_string())); }
                        }
                    }
                    ActorCmd::GetInvoice { reply } => {
                        let _ = reply.send(pending_invoice.as_ref().map(invoice_request_to_dto));
                    }
                    ActorCmd::ReplyInvoice { invoice, reply } => {
                        let Some(request) = pending_invoice.take() else {
                            let _ = reply.send(Err("No pending invoice request".into()));
                            continue;
                        };
                        let inv = match invoice {
                            Some(inv) if !inv.trim().is_empty() => inv.trim().to_string(),
                            _ => {
                                // Create via wallet
                                let Some(ref w) = wallet else {
                                    let _ = reply.send(Err("No wallet configured; provide invoice".into()));
                                    pending_invoice = Some(request);
                                    continue;
                                };
                                let Some(msat) = invoice_request_amount_msat(&request).filter(|&m| m > 0) else {
                                    let _ = reply.send(Err("Cannot create invoice: no exchange rate or zero amount".into()));
                                    pending_invoice = Some(request);
                                    continue;
                                };
                                match w.make_invoice(msat, request.inner.description.clone()).await {
                                    Ok(inv) => inv,
                                    Err(e) => {
                                        let _ = reply.send(Err(e.to_string()));
                                        pending_invoice = Some(request);
                                        continue;
                                    }
                                }
                            }
                        };
                        match app.reply_invoice_request(
                            request.clone(),
                            MakeInvoiceResponse { invoice: inv.clone(), payment_hash: None },
                        ).await {
                            Ok(_) => {
                                let dto = invoice_request_to_dto(&request);
                                let now_secs = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                let entry = InvoiceRequestLogEntry {
                                    request_id: dto.request_id.clone(),
                                    amount_formatted: dto.amount_formatted,
                                    currency: dto.currency,
                                    is_fiat: dto.is_fiat,
                                    equivalent_sats: dto.equivalent_sats,
                                    description: dto.description,
                                    at_secs: now_secs,
                                    status: "accepted".into(),
                                    error: None,
                                    invoice: Some(inv.clone()),
                                };
                                let _ = sse_tx.send(SseEvent::InvoiceResult {
                                    request_id: dto.request_id,
                                    status: "accepted".into(),
                                    error: None,
                                    invoice: Some(inv),
                                });
                                recent_invoices.insert(0, entry);
                                if recent_invoices.len() > MAX_RECENT_INVOICE_REQUESTS {
                                    recent_invoices.truncate(MAX_RECENT_INVOICE_REQUESTS);
                                }
                                let _ = reply.send(Ok(()));
                            }
                            Err(e) => {
                                pending_invoice = Some(request);
                                let _ = reply.send(Err(e.to_string()));
                            }
                        }
                    }
                    ActorCmd::RejectInvoice { reply } => {
                        let _ = pending_invoice.take();
                        let _ = reply.send(());
                    }
                    ActorCmd::RecentInvoices { reply } => {
                        let _ = reply.send(recent_invoices.clone());
                    }
                    ActorCmd::Handshake { url, reply } => {
                        let result = run_handshake(&app, &url).await;
                        let _ = reply.send(result);
                    }
                    ActorCmd::FetchProfile { reply } => {
                        let result = app
                            .fetch_profile(pubkey)
                            .await
                            .map(|p| p.map(ProfileDto::from))
                            .map_err(|e| e.to_string());
                        let _ = reply.send(result);
                    }
                }
            }
            else => break,
        }
    }

    // Cleanup
    global.sessions.write().await.remove(&pubkey);
    log::info!("Actor shut down for {}", app::key_to_hex(pubkey));
}

async fn run_handshake(app: &PortalApp, url: &str) -> Result<(), String> {
    let parsed = parse_key_handshake_url(url)
        .map_err(|e| format!("Invalid handshake URL: {}", e))?;

    app.send_key_handshake(parsed).await
        .map_err(|e| format!("Handshake send failed: {}", e))?;

    let challenge = tokio::time::timeout(
        Duration::from_secs(15),
        app.next_auth_challenge(),
    )
    .await
    .map_err(|_| "Timed out waiting for auth challenge".to_string())?
    .map_err(|e| format!("Auth challenge error: {}", e))?;

    let session_token = format!(
        "demo-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    app.reply_auth_challenge(
        challenge,
        AuthResponseStatus::Approved {
            granted_permissions: vec![],
            session_token,
        },
    )
    .await
    .map_err(|e| format!("Auth reply failed: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Helper: send command to actor
// ---------------------------------------------------------------------------

async fn send_cmd<T>(
    global: &GlobalState,
    pubkey_hex: &str,
    make_cmd: impl FnOnce(oneshot::Sender<T>) -> ActorCmd,
) -> Result<T, ApiError> {
    let nostr_pk = portal::nostr::PublicKey::from_hex(pubkey_hex)
        .map_err(|_| ApiError::bad_request("Invalid pubkey"))?;
    let pubkey = PublicKey(nostr_pk);
    let sessions = global.sessions.read().await;
    let tx = sessions
        .get(&pubkey)
        .ok_or_else(|| ApiError::not_found("Session not found or expired"))?
        .clone();
    drop(sessions);
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(make_cmd(reply_tx))
        .await
        .map_err(|_| ApiError::internal("Actor not running"))?;
    reply_rx
        .await
        .map_err(|_| ApiError::internal("Actor dropped reply"))
}

// ---------------------------------------------------------------------------
// Entrypoint
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let settings = config::Settings::load()?;

    let global = Arc::new(GlobalState {
        sessions: RwLock::new(HashMap::new()),
        config: AppConfig {
            listen_port: settings.info.listen_port,
            default_relays: settings.nostr.default_relays.clone(),
            max_sessions: settings.session.max_sessions,
            session_ttl_secs: settings.session.ttl_secs,
        },
    });

    let router = Router::new()
        .route("/", get(serve_index))
        .route("/api/config", get(api_config))
        .route("/api/session", post(api_create_session))
        .route("/api/session/ping/:pubkey_hex", post(api_ping))
        .route("/api/events/:pubkey_hex", get(api_events))
        .route("/api/status/:pubkey_hex", get(api_status))
        .route("/api/profile/:pubkey_hex", get(api_profile))
        .route("/api/nip05-lookup", get(api_nip05_lookup))
        .route("/api/handshake/:pubkey_hex", post(api_handshake))
        .route("/api/payment-request/:pubkey_hex", get(api_payment_request))
        .route("/api/payment-request/:pubkey_hex/accept", post(api_accept))
        .route("/api/payment-request/:pubkey_hex/reject", post(api_reject))
        .route("/api/invoice-request/:pubkey_hex", get(api_invoice_request))
        .route("/api/invoice-request/:pubkey_hex/reply", post(api_invoice_reply))
        .route("/api/invoice-request/:pubkey_hex/reject", post(api_invoice_reject))
        .route("/api/invoice-requests/recent/:pubkey_hex", get(api_recent_invoices))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(global.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], global.config.listen_port));
    log::info!("portal-app-demo listening on http://{}/", addr);
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn serve_index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

#[derive(Serialize)]
struct ConfigResponse {
    default_relays: Vec<String>,
}

async fn api_config(State(global): State<Arc<GlobalState>>) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        default_relays: global.config.default_relays.clone(),
    })
}

async fn api_create_session(
    State(global): State<Arc<GlobalState>>,
    Json(body): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, ApiError> {
    // Validate relays
    for r in &body.relays {
        if !r.starts_with("wss://") && !r.starts_with("ws://") {
            return Err(ApiError::bad_request(format!("Invalid relay URL: {}", r)));
        }
    }
    if body.relays.is_empty() {
        return Err(ApiError::bad_request("At least one relay is required"));
    }

    // Build keypair
    let keypair = if let Some(ref nsec) = body.nsec {
        let nsec = nsec.trim();
        if nsec.is_empty() {
            return Err(ApiError::bad_request("nsec is empty"));
        }
        let n = Nsec::new(nsec).map_err(|e| ApiError::bad_request(format!("Invalid nsec: {:?}", e)))?;
        Arc::new(n.get_keypair())
    } else if let Some(ref words) = body.mnemonic {
        let words = words.trim();
        if words.is_empty() {
            return Err(ApiError::bad_request("mnemonic is empty"));
        }
        let m = Mnemonic::new(words).map_err(|e| ApiError::bad_request(format!("Invalid mnemonic: {:?}", e)))?;
        Arc::new(m.get_keypair().map_err(|e| ApiError::bad_request(format!("Keypair from mnemonic: {:?}", e)))?)
    } else {
        return Err(ApiError::bad_request("Provide nsec or mnemonic"));
    };

    let pubkey = keypair.public_key();
    let pubkey_bech32 = portal::nostr::nips::nip19::ToBech32::to_bech32(&*pubkey)
        .unwrap_or_else(|_| app::key_to_hex(pubkey));
    let pubkey_hex = app::key_to_hex(pubkey);

    // Check existing session
    {
        let sessions = global.sessions.read().await;
        if let Some(tx) = sessions.get(&pubkey) {
            // Send ping to keep alive, return existing
            let _ = tx.send(ActorCmd::Ping).await;
            return Ok(Json(CreateSessionResponse {
                pubkey: pubkey_bech32,
                pubkey_hex,
            }));
        }
        if sessions.len() >= global.config.max_sessions {
            return Err(ApiError::too_many_requests("Max sessions reached"));
        }
    }

    // Create app
    let listener = Arc::new(NoOpRelayStatusListener);
    let app = PortalApp::new(keypair, body.relays, listener)
        .await
        .map_err(|e| ApiError::internal(format!("PortalApp init: {}", e)))?;

    // Build wallet
    let wallet: Option<Arc<dyn PortalWallet>> = match body.nwc {
        Some(ref nwc_str) if !nwc_str.trim().is_empty() => {
            let w = NwcWallet::new(nwc_str.trim().to_string())
                .map_err(|e| ApiError::bad_request(format!("Invalid NWC: {}", e)))?;
            Some(Arc::new(w))
        }
        _ => None,
    };

    // Spawn actor
    let (tx, rx) = mpsc::channel(32);
    let g = Arc::clone(&global);
    tokio::spawn(run_actor(pubkey, rx, app, wallet, g));

    global.sessions.write().await.insert(pubkey, tx);

    Ok(Json(CreateSessionResponse {
        pubkey: pubkey_bech32,
        pubkey_hex,
    }))
}

async fn api_ping(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
) -> Result<StatusCode, ApiError> {
    let nostr_pk = portal::nostr::PublicKey::from_hex(&pubkey_hex)
        .map_err(|_| ApiError::bad_request("Invalid pubkey"))?;
    let pubkey = PublicKey(nostr_pk);
    let sessions = global.sessions.read().await;
    let tx = sessions
        .get(&pubkey)
        .ok_or_else(|| ApiError::not_found("Session not found"))?
        .clone();
    drop(sessions);
    tx.send(ActorCmd::Ping)
        .await
        .map_err(|_| ApiError::internal("Actor not running"))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn api_events(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let rx = send_cmd(&global, &pubkey_hex, |r| ActorCmd::Subscribe { reply: r }).await?;
    let stream = BroadcastStream::new(rx)
        .filter_map(|r| async move { r.ok() })
        .map(|evt| Ok(Event::default().data(serde_json::to_string(&evt).unwrap())));
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn api_status(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
) -> Result<Json<StatusDto>, ApiError> {
    let status = send_cmd(&global, &pubkey_hex, |r| ActorCmd::Status { reply: r }).await?;
    Ok(Json(status))
}

#[derive(Deserialize)]
struct Nip05LookupQuery {
    nip05: String,
}

async fn api_profile(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
) -> Result<Json<Option<ProfileDto>>, ApiError> {
    let result = send_cmd(&global, &pubkey_hex, |r| ActorCmd::FetchProfile { reply: r }).await?;
    let profile_opt = result.map_err(ApiError::internal)?;
    Ok(Json(profile_opt))
}

async fn api_nip05_lookup(
    Query(q): Query<Nip05LookupQuery>,
) -> Result<Json<portal::nostr::nips::nip05::Nip05Profile>, ApiError> {
    let nip05 = q.nip05.trim();
    if nip05.is_empty() {
        return Err(ApiError::bad_request("Missing nip05 query parameter"));
    }
    let profile = fetch_nip05_profile(nip05)
        .await
        .map_err(|e| ApiError::internal(format!("NIP-05 lookup failed: {}", e)))?;
    Ok(Json(profile))
}

async fn api_handshake(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
    Json(body): Json<HandshakeBody>,
) -> Result<StatusCode, ApiError> {
    let result = send_cmd(&global, &pubkey_hex, |r| ActorCmd::Handshake {
        url: body.url,
        reply: r,
    })
    .await?;
    result.map_err(|e| ApiError::internal(e))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn api_payment_request(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
) -> Result<Json<Option<PaymentRequestDto>>, ApiError> {
    let dto = send_cmd(&global, &pubkey_hex, |r| ActorCmd::GetPayment { reply: r }).await?;
    Ok(Json(dto))
}

async fn api_accept(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
) -> Result<StatusCode, ApiError> {
    let result = send_cmd(&global, &pubkey_hex, |r| ActorCmd::AcceptPayment { reply: r }).await?;
    result.map_err(|e| ApiError::internal(e))?;
    Ok(StatusCode::OK)
}

async fn api_reject(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
    Json(body): Json<RejectBody>,
) -> Result<StatusCode, ApiError> {
    let result = send_cmd(&global, &pubkey_hex, |r| ActorCmd::RejectPayment {
        reason: body.reason,
        reply: r,
    })
    .await?;
    result.map_err(|e| ApiError::internal(e))?;
    Ok(StatusCode::OK)
}

async fn api_invoice_request(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
) -> Result<Json<Option<InvoiceRequestDto>>, ApiError> {
    let dto = send_cmd(&global, &pubkey_hex, |r| ActorCmd::GetInvoice { reply: r }).await?;
    Ok(Json(dto))
}

async fn api_invoice_reply(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
    Json(body): Json<InvoiceReplyBody>,
) -> Result<StatusCode, ApiError> {
    let result = send_cmd(&global, &pubkey_hex, |r| ActorCmd::ReplyInvoice {
        invoice: body.invoice,
        reply: r,
    })
    .await?;
    result.map_err(|e| ApiError::internal(e))?;
    Ok(StatusCode::OK)
}

async fn api_invoice_reject(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
) -> Result<StatusCode, ApiError> {
    send_cmd(&global, &pubkey_hex, |r| ActorCmd::RejectInvoice { reply: r }).await?;
    Ok(StatusCode::OK)
}

async fn api_recent_invoices(
    State(global): State<Arc<GlobalState>>,
    Path(pubkey_hex): Path<String>,
) -> Result<Json<Vec<InvoiceRequestLogEntry>>, ApiError> {
    let list = send_cmd(&global, &pubkey_hex, |r| ActorCmd::RecentInvoices { reply: r }).await?;
    Ok(Json(list))
}
