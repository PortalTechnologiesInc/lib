use lightning_invoice::Bolt11Invoice;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use cdk::amount::SplitTarget;
use cdk::mint_url::MintUrl;
use cdk::nuts::CurrencyUnit;
use cdk::wallet::{SendOptions, Wallet, WalletBuilder};
use cdk_sqlite::wallet::memory;
use chrono::Duration;
#[allow(unused_imports)]
use futures::StreamExt;
use portal::nostr::key::PublicKey;
use portal::nostr_relay_pool::RelayOptions;
use portal::protocol::calendar::Calendar;
use portal::protocol::jwt::CustomClaims;
use portal::protocol::model::payment::{
    CashuDirectContent, CashuRequestContent, Currency, ExchangeRate, FiatCents, FiatCurrency,
    InvoiceRequest, Millisats, MillisatsCurrency, PaymentStatus, RecurringPaymentRequest,
    SinglePaymentRequest,
};
use portal::protocol::model::Timestamp;
use portal::utils::fetch_nip05_profile as portal_fetch_nip05;
use rand::RngCore;
use serde::Deserialize;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::command::*;
use crate::events::StreamMetadata;
use crate::response::*;
use crate::AppState;

// ---- Helper types ----

type ApiResult<T> = Result<(StatusCode, Json<ApiResponse<T>>), (StatusCode, Json<ApiResponse<()>>)>;

fn ok<T: serde::Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
    (StatusCode::OK, Json(ApiResponse::ok(data)))
}

fn created<T: serde::Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
    (StatusCode::CREATED, Json(ApiResponse::ok(data)))
}

fn err(status: StatusCode, msg: impl Into<String>) -> (StatusCode, Json<ApiResponse<()>>) {
    (status, Json(ApiResponse::error(msg)))
}

fn bad_request(msg: impl Into<String>) -> (StatusCode, Json<ApiResponse<()>>) {
    err(StatusCode::BAD_REQUEST, msg)
}

fn internal_error(msg: impl Into<String>) -> (StatusCode, Json<ApiResponse<()>>) {
    err(StatusCode::INTERNAL_SERVER_ERROR, msg)
}

fn not_found(msg: impl Into<String>) -> (StatusCode, Json<ApiResponse<()>>) {
    err(StatusCode::NOT_FOUND, msg)
}

fn hex_to_pubkey(hex: &str) -> Result<PublicKey, String> {
    hex.parse::<PublicKey>().map_err(|e| e.to_string())
}

fn parse_subkeys(subkeys: &[String]) -> Result<Vec<PublicKey>, String> {
    subkeys.iter().map(|s| hex_to_pubkey(s)).collect()
}

/// Resolve amount and exchange rate: for Millisats returns (amount, None);
/// for Fiat fetches market data and returns (amount_msat, Some(ExchangeRate)).
async fn resolve_amount_and_exchange_rate(
    amount: u64,
    currency: &Currency,
    market_api: Arc<portal_rates::MarketAPI>,
) -> Result<(Millisats, Option<ExchangeRate>), portal_rates::RatesError> {
    match currency {
        Currency::Millisats => Ok((Millisats::new(amount), None)),
        Currency::Fiat(currency_code) => {
            let market_data = market_api.fetch_market_data(currency_code).await?;
            let fiat_amount = amount as f64 / 100.0;
            let msat = (market_data.calculate_millisats(fiat_amount) as i64).max(0) as u64;
            let exchange_rate = ExchangeRate {
                rate: market_data.rate,
                source: market_data.source,
                time: Timestamp::now(),
            };
            Ok((Millisats::new(msat), Some(exchange_rate)))
        }
    }
}

fn extract_invoice_amount_msat(invoice: &str) -> Result<Option<u64>, String> {
    let bolt11 = Bolt11Invoice::from_str(invoice).map_err(|e| e.to_string())?;
    Ok(bolt11.amount_milli_satoshis())
}

async fn get_cashu_wallet(
    mint_url: MintUrl,
    unit: CurrencyUnit,
    static_auth_token: Option<String>,
) -> Result<Wallet, String> {
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);
    let localstore = Arc::new(memory::empty().await.expect("Failed to create localstore"));
    let mut builder = WalletBuilder::new()
        .mint_url(mint_url)
        .unit(unit)
        .localstore(localstore)
        .seed(&seed);
    if let Some(token) = static_auth_token {
        builder = builder.static_token(token);
    }
    let wallet = builder.build().map_err(|e| e.to_string())?;
    wallet.get_mint_info().await.map_err(|e| e.to_string())?;
    Ok(wallet)
}

// ---- Shared helpers ----

/// Poll a Lightning invoice until it is paid, times out, or errors.
/// Pushes a `PaymentStatusUpdate` notification to the event store when done.
pub async fn monitor_invoice_until_paid(
    wallet: Arc<dyn portal_wallet::PortalWallet>,
    events: crate::events::EventStore,
    stream_id: String,
    invoice: String,
    expires_at: portal::protocol::model::Timestamp,
) {
    let notification = loop {
        if portal::protocol::model::Timestamp::now() > expires_at {
            break NotificationData::PaymentStatusUpdate {
                status: InvoiceStatus::Timeout,
            };
        }
        match wallet.is_invoice_paid(invoice.clone()).await {
            Ok((true, preimage)) => {
                break NotificationData::PaymentStatusUpdate {
                    status: InvoiceStatus::Paid { preimage },
                };
            }
            Ok((false, _)) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
            Err(e) => {
                tracing::error!("Failed to check invoice for stream {stream_id}: {e}");
                break NotificationData::PaymentStatusUpdate {
                    status: InvoiceStatus::Error { reason: e.to_string() },
                };
            }
        }
    };
    events.push(&stream_id, notification).await;
}

// ---- Route handlers ----

pub async fn health_check() -> &'static str {
    "OK"
}

pub async fn version() -> (StatusCode, Json<ApiResponse<VersionResponse>>) {
    ok(VersionResponse {
        version: crate::APP_VERSION,
        git_commit: crate::GIT_COMMIT,
    })
}

pub async fn info(
    State(state): State<AppState>,
) -> ApiResult<InfoResponse> {
    Ok(ok(InfoResponse {
        public_key: state.public_key.clone(),
        version: crate::APP_VERSION,
        git_commit: crate::GIT_COMMIT,
    }))
}

// GET /well-known/nostr.json
pub async fn well_known_nostr_json(
    State(state): State<AppState>,
) -> ApiResult<Nip05WellKnownResponse> {
    let name = state
        .settings
        .profile
        .name
        .clone()
        .unwrap_or_else(|| "_".to_string());

    let mut names = std::collections::HashMap::new();
    names.insert(name, state.public_key.clone());

    let mut relays = std::collections::HashMap::new();
    if !state.settings.nostr.relays.is_empty() {
        relays.insert(
            state.public_key.clone(),
            state.settings.nostr.relays.clone(),
        );
    }

    Ok(ok(Nip05WellKnownResponse { names, relays }))
}

// POST /key-handshake
pub async fn new_key_handshake_url(
    State(state): State<AppState>,
    Json(req): Json<KeyHandshakeRequest>,
) -> ApiResult<KeyHandshakeUrlResponse> {
    let (url, notification_stream) = state
        .sdk
        .new_key_handshake_url(req.static_token, req.no_request)
        .await
        .map_err(|e| internal_error(format!("Failed to create key handshake URL: {e}")))?;

    let metadata = StreamMetadata::KeyHandshake {
        url: url.to_string(),
    };
    let stream_id = state.events.new_stream("key_handshake", Some(&metadata)).await;

    // Spawn background task to collect notifications
    let events = state.events.clone();
    let relay_pool = state.sdk.relay_pool();
    let sid = stream_id.clone();
    tokio::spawn(async move {
        let mut stream = notification_stream;
        while let Some(Ok(event)) = stream.next().await {
            debug!("Got key handshake event: {:?}", event);

            let preferred_relays = event.relays.clone();
            for relay in &preferred_relays {
                match relay_pool.add_relay(relay, RelayOptions::default()).await {
                    Ok(false) => continue,
                    Err(e) => {
                        warn!("Failed to add relay {relay}: {e}");
                        continue;
                    }
                    _ => {}
                }
                if let Err(e) = relay_pool.connect_relay(relay).await {
                    warn!("Failed to connect to relay {relay}: {e}");
                    continue;
                }
            }

            events
                .push(
                    &sid,
                    NotificationData::KeyHandshake {
                        main_key: event.main_key.to_string(),
                        preferred_relays,
                    },
                )
                .await;
        }
        debug!("Key handshake stream ended for {sid}");
    });

    Ok(created(KeyHandshakeUrlResponse {
        url: url.to_string(),
        stream_id,
    }))
}

// POST /authenticate-key
pub async fn authenticate_key(
    State(state): State<AppState>,
    Json(req): Json<AuthenticateKeyRequest>,
) -> ApiResult<StreamResponse> {
    let main_key = hex_to_pubkey(&req.main_key).map_err(|e| bad_request(format!("Invalid main key: {e}")))?;
    let subkeys = parse_subkeys(&req.subkeys).map_err(|e| bad_request(format!("Invalid subkeys: {e}")))?;

    let stream_id = state.events.new_stream("authenticate_key", None).await;

    let sdk = state.sdk.clone();
    let events = state.events.clone();
    let sid = stream_id.clone();
    tokio::spawn(async move {
        match sdk.authenticate_key(main_key, subkeys).await {
            Ok(event) => {
                events
                    .push(
                        &sid,
                        NotificationData::AuthenticateKey {
                            user_key: event.user_key.to_string(),
                            recipient: event.recipient.to_string(),
                            challenge: event.challenge,
                            status: event.status,
                        },
                    )
                    .await;
            }
            Err(e) => {
                events
                    .push(
                        &sid,
                        NotificationData::Error {
                            reason: format!("Failed to authenticate key: {e}"),
                        },
                    )
                    .await;
            }
        }
    });

    Ok(created(StreamResponse { stream_id }))
}

// POST /payments/recurring
pub async fn request_recurring_payment(
    State(state): State<AppState>,
    Json(req): Json<RequestRecurringPaymentRequest>,
) -> ApiResult<StreamResponse> {
    let main_key = hex_to_pubkey(&req.main_key).map_err(|e| bad_request(format!("Invalid main key: {e}")))?;
    let subkeys = parse_subkeys(&req.subkeys).map_err(|e| bad_request(format!("Invalid subkeys: {e}")))?;

    let (_, current_exchange_rate) = resolve_amount_and_exchange_rate(
        req.payment_request.amount,
        &req.payment_request.currency,
        state.market_api.clone(),
    )
    .await
    .map_err(|e| internal_error(format!("Failed to fetch market data: {e}")))?;

    enum AnyRecurringPaymentRequest {
        Millisats(RecurringPaymentRequest<MillisatsCurrency>),
        Fiat(RecurringPaymentRequest<FiatCurrency>),
    }

    let payment_request = match req.payment_request.currency {
        Currency::Millisats => AnyRecurringPaymentRequest::Millisats(RecurringPaymentRequest {
            description: req.payment_request.description,
            amount: Millisats::new(req.payment_request.amount),
            currency: MillisatsCurrency,
            auth_token: req.payment_request.auth_token,
            recurrence: req.payment_request.recurrence,
            expires_at: req.payment_request.expires_at,
            request_id: Uuid::new_v4().to_string(),
            current_exchange_rate,
        }),
        Currency::Fiat(code) => AnyRecurringPaymentRequest::Fiat(RecurringPaymentRequest {
            description: req.payment_request.description,
            amount: FiatCents::new(req.payment_request.amount),
            currency: FiatCurrency { code },
            auth_token: req.payment_request.auth_token,
            recurrence: req.payment_request.recurrence,
            expires_at: req.payment_request.expires_at,
            request_id: Uuid::new_v4().to_string(),
            current_exchange_rate,
        }),
    };

    let stream_id = state.events.new_stream("recurring_payment", None).await;

    let sdk = state.sdk.clone();
    let events = state.events.clone();
    let sid = stream_id.clone();
    tokio::spawn(async move {
        let result = match payment_request {
            AnyRecurringPaymentRequest::Millisats(req) => {
                sdk.request_recurring_payment(main_key, subkeys, req).await
            }
            AnyRecurringPaymentRequest::Fiat(req) => {
                sdk.request_recurring_payment(main_key, subkeys, req).await
            }
        };

        match result {
            Ok(status) => {
                events
                    .push(
                        &sid,
                        NotificationData::RecurringPaymentResponse { status },
                    )
                    .await;
            }
            Err(e) => {
                events
                    .push(
                        &sid,
                        NotificationData::Error {
                            reason: format!("Failed to request recurring payment: {e}"),
                        },
                    )
                    .await;
            }
        }
    });

    Ok(created(StreamResponse { stream_id }))
}

// POST /payments/single
pub async fn request_single_payment(
    State(state): State<AppState>,
    Json(req): Json<RequestSinglePaymentRequest>,
) -> ApiResult<SinglePaymentResponse> {
    let wallet = state
        .wallet
        .as_ref()
        .ok_or_else(|| bad_request("Backend wallet not available: set NWC_URL or BREEZ_MNEMONIC"))?;

    let main_key = hex_to_pubkey(&req.main_key).map_err(|e| bad_request(format!("Invalid main key: {e}")))?;
    let subkeys = parse_subkeys(&req.subkeys).map_err(|e| bad_request(format!("Invalid subkeys: {e}")))?;

    let amount = req.payment_request.amount;
    let (msat_amount, current_exchange_rate) = resolve_amount_and_exchange_rate(
        amount,
        &req.payment_request.currency,
        state.market_api.clone(),
    )
    .await
    .map_err(|e| internal_error(format!("Failed to fetch market data: {e}")))?;

    let invoice = wallet
        .make_invoice_msat(msat_amount, Some(req.payment_request.description.clone()))
        .await
        .map_err(|e| internal_error(format!("Failed to make invoice: {e}")))?;

    let request_id = req
        .payment_request
        .request_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let expires_at = Timestamp::now_plus_seconds(300);
    enum AnySinglePaymentRequest {
        Millisats(SinglePaymentRequest<MillisatsCurrency>),
        Fiat(SinglePaymentRequest<FiatCurrency>),
    }

    let payment_request = match req.payment_request.currency {
        Currency::Millisats => AnySinglePaymentRequest::Millisats(SinglePaymentRequest {
            amount: Millisats::new(amount),
            currency: MillisatsCurrency,
            expires_at,
            invoice: invoice.clone(),
            current_exchange_rate,
            subscription_id: req.payment_request.subscription_id,
            auth_token: req.payment_request.auth_token,
            request_id,
            description: Some(req.payment_request.description),
        }),
        Currency::Fiat(code) => AnySinglePaymentRequest::Fiat(SinglePaymentRequest {
            amount: FiatCents::new(amount),
            currency: FiatCurrency { code },
            expires_at,
            invoice: invoice.clone(),
            current_exchange_rate,
            subscription_id: req.payment_request.subscription_id,
            auth_token: req.payment_request.auth_token,
            request_id,
            description: Some(req.payment_request.description),
        }),
    };

    let mut notifications = match payment_request {
        AnySinglePaymentRequest::Millisats(req) => state
            .sdk
            .request_single_payment(main_key, subkeys.clone(), req)
            .await
            .map_err(|e| internal_error(format!("Failed to request single payment: {e}")))?,
        AnySinglePaymentRequest::Fiat(req) => state
            .sdk
            .request_single_payment(main_key, subkeys, req)
            .await
            .map_err(|e| internal_error(format!("Failed to request single payment: {e}")))?,
    };

    let metadata = StreamMetadata::SinglePayment {
        invoice: invoice.clone(),
        expires_at_secs: expires_at.as_u64(),
    };
    let stream_id = state.events.new_stream("single_payment", Some(&metadata)).await;

    let events = state.events.clone();
    let sid = stream_id.clone();
    let wallet_clone = wallet.clone();
    let invoice_clone = invoice;

    tokio::spawn(async move {
        let monitor_started = std::sync::atomic::AtomicBool::new(false);
        while let Some(notification) = notifications.next().await {
            match notification {
                Ok(status) => {
                    let notif_data = match &status.status {
                        PaymentStatus::Failed { reason } => NotificationData::PaymentStatusUpdate {
                            status: InvoiceStatus::UserFailed {
                                reason: reason.clone(),
                            },
                        },
                        PaymentStatus::Rejected { reason } => NotificationData::PaymentStatusUpdate {
                            status: InvoiceStatus::UserRejected {
                                reason: reason.clone(),
                            },
                        },
                        PaymentStatus::Success { preimage } => NotificationData::PaymentStatusUpdate {
                            status: InvoiceStatus::UserSuccess {
                                preimage: preimage.clone(),
                            },
                        },
                        PaymentStatus::Approved => NotificationData::PaymentStatusUpdate {
                            status: InvoiceStatus::UserApproved,
                        },
                    };

                    events.push(&sid, notif_data).await;

                    if status.status.is_final() {
                        return;
                    }

                    if monitor_started.swap(true, std::sync::atomic::Ordering::SeqCst) {
                        continue;
                    }

                    // Start invoice monitoring
                    let events2 = events.clone();
                    let sid2 = sid.clone();
                    let wallet2 = wallet_clone.clone();
                    let inv2 = invoice_clone.clone();
                    tokio::spawn(monitor_invoice_until_paid(wallet2, events2, sid2, inv2, expires_at));
                }
                Err(e) => {
                    error!("Payment notification error: {e}");
                }
            }
        }
    });

    Ok(created(SinglePaymentResponse { stream_id }))
}

// POST /payments/raw
pub async fn request_payment_raw(
    State(state): State<AppState>,
    Json(req): Json<RequestPaymentRawRequest>,
) -> ApiResult<SinglePaymentResponse> {
    let main_key = hex_to_pubkey(&req.main_key).map_err(|e| bad_request(format!("Invalid main key: {e}")))?;
    let subkeys = parse_subkeys(&req.subkeys).map_err(|e| bad_request(format!("Invalid subkeys: {e}")))?;

    enum AnySinglePaymentRequest {
        Millisats(SinglePaymentRequest<MillisatsCurrency>),
        Fiat(SinglePaymentRequest<FiatCurrency>),
    }

    let payment_request = match req.payment_request.currency {
        Currency::Millisats => AnySinglePaymentRequest::Millisats(SinglePaymentRequest {
            amount: Millisats::new(req.payment_request.amount),
            currency: MillisatsCurrency,
            current_exchange_rate: req.payment_request.current_exchange_rate,
            invoice: req.payment_request.invoice,
            auth_token: req.payment_request.auth_token,
            expires_at: req.payment_request.expires_at,
            subscription_id: req.payment_request.subscription_id,
            description: req.payment_request.description,
            request_id: req.payment_request.request_id,
        }),
        Currency::Fiat(code) => AnySinglePaymentRequest::Fiat(SinglePaymentRequest {
            amount: FiatCents::new(req.payment_request.amount),
            currency: FiatCurrency { code },
            current_exchange_rate: req.payment_request.current_exchange_rate,
            invoice: req.payment_request.invoice,
            auth_token: req.payment_request.auth_token,
            expires_at: req.payment_request.expires_at,
            subscription_id: req.payment_request.subscription_id,
            description: req.payment_request.description,
            request_id: req.payment_request.request_id,
        }),
    };

    let mut notifications = match payment_request {
        AnySinglePaymentRequest::Millisats(req) => state
            .sdk
            .request_single_payment(main_key, subkeys.clone(), req)
            .await
            .map_err(|e| internal_error(format!("Failed to request payment: {e}")))?,
        AnySinglePaymentRequest::Fiat(req) => state
            .sdk
            .request_single_payment(main_key, subkeys, req)
            .await
            .map_err(|e| internal_error(format!("Failed to request payment: {e}")))?,
    };

    let stream_id = state.events.new_stream("raw_payment", None).await;

    let events = state.events.clone();
    let sid = stream_id.clone();

    tokio::spawn(async move {
        while let Some(notification) = notifications.next().await {
            match notification {
                Ok(status) => {
                    let notif_data = match status.status {
                        PaymentStatus::Failed { reason } => NotificationData::PaymentStatusUpdate {
                            status: InvoiceStatus::UserFailed { reason },
                        },
                        PaymentStatus::Rejected { reason } => NotificationData::PaymentStatusUpdate {
                            status: InvoiceStatus::UserRejected { reason },
                        },
                        PaymentStatus::Success { preimage } => NotificationData::PaymentStatusUpdate {
                            status: InvoiceStatus::UserSuccess { preimage },
                        },
                        PaymentStatus::Approved => NotificationData::PaymentStatusUpdate {
                            status: InvoiceStatus::UserApproved,
                        },
                    };
                    events.push(&sid, notif_data).await;
                }
                Err(e) => {
                    error!("Payment notification error: {e}");
                }
            }
        }
    });

    Ok(created(SinglePaymentResponse { stream_id }))
}

// GET /profile/:main_key
pub async fn fetch_profile(
    State(state): State<AppState>,
    Path(main_key): Path<String>,
) -> ApiResult<ProfileResponse> {
    let main_key = hex_to_pubkey(&main_key).map_err(|e| bad_request(format!("Invalid main key: {e}")))?;

    let profile = state
        .sdk
        .fetch_profile(main_key)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch profile: {e}")))?;

    Ok(ok(ProfileResponse { profile }))
}

// POST /payments/recurring/close
pub async fn close_recurring_payment(
    State(state): State<AppState>,
    Json(req): Json<CloseRecurringPaymentRequest>,
) -> ApiResult<CloseRecurringPaymentResponse> {
    let main_key = hex_to_pubkey(&req.main_key).map_err(|e| bad_request(format!("Invalid main key: {e}")))?;
    let subkeys = parse_subkeys(&req.subkeys).map_err(|e| bad_request(format!("Invalid subkeys: {e}")))?;

    state
        .sdk
        .close_recurring_payment(main_key, subkeys, req.subscription_id)
        .await
        .map_err(|e| internal_error(format!("Failed to close recurring payment: {e}")))?;

    Ok(ok(CloseRecurringPaymentResponse {
        message: "Recurring payment closed".to_string(),
    }))
}

// POST /invoices/request
pub async fn request_invoice(
    State(state): State<AppState>,
    Json(req): Json<RequestInvoiceRequest>,
) -> ApiResult<StreamResponse> {
    let recipient_key = hex_to_pubkey(&req.recipient_key).map_err(|e| bad_request(format!("Invalid recipient key: {e}")))?;
    let subkeys = parse_subkeys(&req.subkeys).map_err(|e| bad_request(format!("Invalid subkeys: {e}")))?;

    let request_id = req.content.request_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

    // Resolve amount/exchange rate synchronously — errors returned as 400
    let (expected_amount_msat, current_exchange_rate) = resolve_amount_and_exchange_rate(
        req.content.amount,
        &req.content.currency,
        state.market_api.clone(),
    )
    .await
    .map_err(|e| internal_error(format!("Failed to fetch market data: {e}")))?;

    enum AnyInvoiceRequest {
        Millisats(InvoiceRequest<MillisatsCurrency>),
        Fiat(InvoiceRequest<FiatCurrency>),
    }

    let sdk_content = match req.content.currency.clone() {
        Currency::Millisats => AnyInvoiceRequest::Millisats(InvoiceRequest {
            request_id,
            amount: Millisats::new(req.content.amount),
            currency: MillisatsCurrency,
            current_exchange_rate,
            expires_at: req.content.expires_at,
            description: req.content.description.clone(),
            refund_invoice: req.content.refund_invoice.clone(),
        }),
        Currency::Fiat(code) => AnyInvoiceRequest::Fiat(InvoiceRequest {
            request_id,
            amount: FiatCents::new(req.content.amount),
            currency: FiatCurrency { code },
            current_exchange_rate,
            expires_at: req.content.expires_at,
            description: req.content.description.clone(),
            refund_invoice: req.content.refund_invoice.clone(),
        }),
    };

    let stream_id = state.events.new_stream("invoice_request", None).await;

    let sdk = state.sdk.clone();
    let events = state.events.clone();
    let sid = stream_id.clone();
    tokio::spawn(async move {
        let result = match sdk_content {
            AnyInvoiceRequest::Millisats(req) => {
                sdk.request_invoice(recipient_key.into(), subkeys, req).await
            }
            AnyInvoiceRequest::Fiat(req) => {
                sdk.request_invoice(recipient_key.into(), subkeys, req).await
            }
        };

        match result {
            Ok(Some(resp)) => {
                // Validate amount
                let invoice_amount_msat = match extract_invoice_amount_msat(&resp.invoice) {
                    Ok(Some(amt)) => amt,
                    Ok(None) => {
                        events
                            .push(
                                &sid,
                                NotificationData::Error {
                                    reason: "Invoice has no amount (zero-amount invoice not allowed)".to_string(),
                                },
                            )
                            .await;
                        return;
                    }
                    Err(e) => {
                        events
                            .push(
                                &sid,
                                NotificationData::Error {
                                    reason: format!("Invalid invoice: {e}"),
                                },
                            )
                            .await;
                        return;
                    }
                };

                let expected_amount_msat_u64 = expected_amount_msat.as_u64();
                let amount_diff =
                    (invoice_amount_msat as i128 - expected_amount_msat_u64 as i128).abs();
                if amount_diff > 1 {
                    events
                        .push(
                            &sid,
                            NotificationData::Error {
                                reason: format!(
                                    "Invoice amount mismatch: got {invoice_amount_msat} msat, expected {expected_amount_msat_u64} msat (diff: {amount_diff} msat)"
                                ),
                            },
                        )
                        .await;
                    return;
                }

                events
                    .push(
                        &sid,
                        NotificationData::InvoiceResponse {
                            invoice: resp.invoice,
                            payment_hash: resp.payment_hash.unwrap_or_default(),
                        },
                    )
                    .await;
            }
            Ok(None) => {
                events
                    .push(
                        &sid,
                        NotificationData::Error {
                            reason: "Recipient did not reply with an invoice".to_string(),
                        },
                    )
                    .await;
            }
            Err(e) => {
                events
                    .push(
                        &sid,
                        NotificationData::Error {
                            reason: format!("Failed to request invoice: {e}"),
                        },
                    )
                    .await;
            }
        }
    });

    Ok(created(StreamResponse { stream_id }))
}

// POST /jwt/issue
pub async fn issue_jwt(
    State(state): State<AppState>,
    Json(req): Json<IssueJwtRequest>,
) -> ApiResult<IssueJwtResponse> {
    let target_key = hex_to_pubkey(&req.target_key).map_err(|e| bad_request(format!("Invalid target_key: {e}")))?;

    let token = state
        .sdk
        .issue_jwt(CustomClaims::new(target_key.into()), Duration::hours(req.duration_hours))
        .map_err(|e| internal_error(format!("Failed to issue JWT: {e}")))?;

    Ok(ok(IssueJwtResponse { token }))
}

// POST /jwt/verify
pub async fn verify_jwt(
    State(state): State<AppState>,
    Json(req): Json<VerifyJwtRequest>,
) -> ApiResult<VerifyJwtResponse> {
    let pubkey = hex_to_pubkey(&req.pubkey).map_err(|e| bad_request(format!("Invalid pubkey: {e}")))?;

    let claims = state
        .sdk
        .verify_jwt(pubkey, &req.token)
        .map_err(|e| bad_request(format!("Failed to verify JWT: {e}")))?;

    Ok(ok(VerifyJwtResponse {
        target_key: claims.target_key.to_string(),
    }))
}

// POST /cashu/request
pub async fn request_cashu(
    State(state): State<AppState>,
    Json(req): Json<RequestCashuRequest>,
) -> ApiResult<StreamResponse> {
    let recipient_key = hex_to_pubkey(&req.recipient_key).map_err(|e| bad_request(format!("Invalid recipient key: {e}")))?;
    let subkeys = parse_subkeys(&req.subkeys).map_err(|e| bad_request(format!("Invalid subkeys: {e}")))?;

    let expires_at = Timestamp::now_plus_seconds(300);
    let content = CashuRequestContent {
        mint_url: req.mint_url,
        unit: req.unit,
        amount: req.amount,
        request_id: Uuid::new_v4().to_string(),
        expires_at,
    };

    let stream_id = state.events.new_stream("cashu_request", None).await;

    let sdk = state.sdk.clone();
    let events = state.events.clone();
    let sid = stream_id.clone();
    tokio::spawn(async move {
        match sdk.request_cashu(recipient_key, subkeys, content).await {
            Ok(Some(r)) => {
                events
                    .push(&sid, NotificationData::CashuResponse { status: r.status })
                    .await;
            }
            Ok(None) => {
                events
                    .push(
                        &sid,
                        NotificationData::Error {
                            reason: "No response from recipient".to_string(),
                        },
                    )
                    .await;
            }
            Err(e) => {
                events
                    .push(
                        &sid,
                        NotificationData::Error {
                            reason: format!("Failed to request cashu: {e}"),
                        },
                    )
                    .await;
            }
        }
    });

    Ok(created(StreamResponse { stream_id }))
}

// POST /cashu/send-direct
pub async fn send_cashu_direct(
    State(state): State<AppState>,
    Json(req): Json<SendCashuDirectRequest>,
) -> ApiResult<SendCashuDirectResponse> {
    let main_key = hex_to_pubkey(&req.main_key).map_err(|e| bad_request(format!("Invalid main key: {e}")))?;
    let subkeys = parse_subkeys(&req.subkeys).map_err(|e| bad_request(format!("Invalid subkeys: {e}")))?;

    state
        .sdk
        .send_cashu_direct(main_key, subkeys, CashuDirectContent { token: req.token })
        .await
        .map_err(|e| internal_error(format!("Failed to send cashu direct: {e}")))?;

    Ok(ok(SendCashuDirectResponse {
        message: "Cashu direct sent".to_string(),
    }))
}

// POST /cashu/mint
pub async fn mint_cashu(
    State(_state): State<AppState>,
    Json(req): Json<MintCashuRequest>,
) -> ApiResult<CashuMintResponse> {
    let mint_url = MintUrl::from_str(&req.mint_url).map_err(|e| bad_request(format!("Invalid mint URL: {e}")))?;
    let currency_unit = CurrencyUnit::from_str(&req.unit).map_err(|e| bad_request(format!("Invalid unit: {e}")))?;

    let wallet = get_cashu_wallet(mint_url, currency_unit, req.static_auth_token)
        .await
        .map_err(|e| internal_error(format!("Failed to create wallet: {e}")))?;

    let quote = wallet
        .mint_quote(req.amount.into(), req.description)
        .await
        .map_err(|e| internal_error(format!("Failed to get mint quote: {e}")))?;

    wallet
        .mint(&quote.id, SplitTarget::None, None)
        .await
        .map_err(|e| internal_error(format!("Failed to mint token: {e}")))?;

    let prepared_send = wallet
        .prepare_send(req.amount.into(), SendOptions::default())
        .await
        .map_err(|e| internal_error(format!("Failed to prepare send: {e}")))?;

    let token = wallet
        .send(prepared_send, None)
        .await
        .map_err(|e| internal_error(format!("Failed to send token: {e}")))?;

    Ok(ok(CashuMintResponse {
        token: token.to_string(),
    }))
}

// POST /cashu/burn
pub async fn burn_cashu(
    State(_state): State<AppState>,
    Json(req): Json<BurnCashuRequest>,
) -> ApiResult<CashuBurnResponse> {
    let mint_url = MintUrl::from_str(&req.mint_url).map_err(|e| bad_request(format!("Invalid mint URL: {e}")))?;
    let currency_unit = CurrencyUnit::from_str(&req.unit).map_err(|e| bad_request(format!("Invalid unit: {e}")))?;

    let wallet = get_cashu_wallet(mint_url, currency_unit, req.static_auth_token)
        .await
        .map_err(|e| internal_error(format!("Failed to create wallet: {e}")))?;

    let receive = wallet
        .receive(&req.token, Default::default())
        .await
        .map_err(|e| internal_error(format!("Failed to receive token: {e}")))?;

    Ok(ok(CashuBurnResponse {
        amount: receive.into(),
    }))
}

// POST /relays
pub async fn add_relay(
    State(state): State<AppState>,
    Json(req): Json<RelayRequest>,
) -> ApiResult<RelayResponse> {
    state
        .sdk
        .add_relay(req.relay.clone())
        .await
        .map_err(|e| internal_error(format!("Failed to add relay {}: {e:?}", req.relay)))?;

    Ok(ok(RelayResponse { relay: req.relay }))
}

// DELETE /relays
pub async fn remove_relay(
    State(state): State<AppState>,
    Json(req): Json<RelayRequest>,
) -> ApiResult<RelayResponse> {
    state
        .sdk
        .remove_relay(req.relay.clone())
        .await
        .map_err(|e| internal_error(format!("Failed to remove relay {}: {e:?}", req.relay)))?;

    Ok(ok(RelayResponse { relay: req.relay }))
}

// POST /calendar/next-occurrence
pub async fn calculate_next_occurrence(
    State(_state): State<AppState>,
    Json(req): Json<CalculateNextOccurrenceRequest>,
) -> ApiResult<NextOccurrenceResponse> {
    let calendar =
        Calendar::from_str(&req.calendar).map_err(|e| bad_request(format!("Invalid calendar: {e}")))?;

    let next_occurrence = calendar.next_occurrence(req.from);

    Ok(ok(NextOccurrenceResponse { next_occurrence }))
}

// POST /invoices/pay
pub async fn pay_invoice(
    State(state): State<AppState>,
    Json(req): Json<PayInvoiceRequest>,
) -> ApiResult<PayInvoiceResponse> {
    let wallet = state
        .wallet
        .as_ref()
        .ok_or_else(|| bad_request("Backend wallet not available: set NWC_URL or BREEZ_MNEMONIC"))?;

    let (preimage, fees_paid_msat) = wallet
        .pay_invoice(req.invoice)
        .await
        .map_err(|e| internal_error(format!("Failed to pay invoice: {e}")))?;

    Ok(ok(PayInvoiceResponse {
        preimage,
        fees_paid_msat,
    }))
}

// GET /nip05/:nip05
pub async fn fetch_nip05_profile(
    State(_state): State<AppState>,
    Path(nip05): Path<String>,
) -> ApiResult<Nip05ProfileResponse> {
    let profile = portal_fetch_nip05(&nip05)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch nip05 profile: {e:?}")))?;

    Ok(ok(Nip05ProfileResponse { profile }))
}

// GET /wallet/info
pub async fn get_wallet_info(
    State(state): State<AppState>,
) -> ApiResult<WalletInfoResponse> {
    let wallet = state
        .wallet
        .as_ref()
        .ok_or_else(|| bad_request("No wallet configured"))?;

    let balance_msat = wallet
        .get_balance()
        .await
        .map_err(|e| internal_error(format!("Failed to get balance: {e}")))?;

    let wallet_type = match &state.settings.wallet.ln_backend {
        crate::config::LnBackend::None => "none",
        crate::config::LnBackend::Nwc => "nwc",
        crate::config::LnBackend::Breez => "breez",
    }
    .to_string();

    Ok(ok(WalletInfoResponse {
        wallet_type,
        balance_msat,
    }))
}

// GET /events/:stream_id
#[derive(Deserialize)]
pub struct EventsQuery {
    pub after: Option<u64>,
}

pub async fn get_events(
    State(state): State<AppState>,
    Path(stream_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> ApiResult<EventsResponse> {
    if !state.events.exists(&stream_id).await {
        return Err(not_found(format!("Stream '{stream_id}' not found")));
    }

    let events = state.events.get(&stream_id, query.after).await;

    Ok(ok(EventsResponse { stream_id, events }))
}

// ── Age Verification constants ────────────────────────────────────────────────
const VERIFICATION_SERVICE_URL: &str = "https://verify.getportal.cc/verify/sessions";
const VERIFICATION_MINT_URL: &str = "https://mint.getportal.cc";
const VERIFICATION_TICKET_UNIT: &str = "multi";
const VERIFICATION_TOKEN_AMOUNT: u64 = 1;

// POST /verification/sessions
pub async fn create_verification_session(
    State(state): State<AppState>,
    Json(req): Json<crate::command::CreateVerificationSessionRequest>,
) -> ApiResult<VerificationSessionResponse> {
    let verification = state
        .settings
        .verification
        .as_ref()
        .ok_or_else(|| bad_request("Verification not configured — add [verification] api_key to config"))?;

    let relays = req.relays.unwrap_or_else(|| state.settings.nostr.relays.clone());

    #[derive(serde::Serialize)]
    struct VerifySessionRequest {
        relays: Vec<String>,
    }

    #[derive(serde::Deserialize)]
    struct VerifySessionApiResponse {
        session_id: String,
        session_url: String,
        ephemeral_npub: String,
        expires_at: u64,
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(VERIFICATION_SERVICE_URL)
        .header("X-Api-Key", &verification.api_key)
        .json(&VerifySessionRequest { relays })
        .send()
        .await
        .map_err(|e| internal_error(format!("Failed to call verification service: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(internal_error(format!(
            "Verification service returned {}: {}",
            status, body
        )));
    }

    let api_resp: VerifySessionApiResponse = resp
        .json()
        .await
        .map_err(|e| internal_error(format!("Failed to parse verification response: {e}")))?;

    // Parse the ephemeral npub (bech32 format) into a PublicKey
    let ephemeral_key = PublicKey::parse(&api_resp.ephemeral_npub)
        .map_err(|e| internal_error(format!("Invalid ephemeral_npub from verification service: {e}")))?;

    let stream_id = spawn_verification_token_request(
        state.sdk.clone(),
        &state.events,
        ephemeral_key,
        vec![],
    )
    .await;

    Ok(ok(VerificationSessionResponse {
        session_id: api_resp.session_id,
        session_url: api_resp.session_url,
        ephemeral_npub: api_resp.ephemeral_npub,
        expires_at: api_resp.expires_at,
        stream_id,
    }))
}

/// Shared helper: creates a verification token request stream, spawns a
/// background task that calls `sdk.request_cashu`, and returns the `stream_id`.
async fn spawn_verification_token_request(
    sdk: Arc<portal_sdk::PortalSDK>,
    events: &crate::events::EventStore,
    recipient: PublicKey,
    subkeys: Vec<PublicKey>,
) -> String {
    let expires_at = Timestamp::now_plus_seconds(300);
    let content = CashuRequestContent {
        mint_url: VERIFICATION_MINT_URL.to_string(),
        unit: VERIFICATION_TICKET_UNIT.to_string(),
        amount: VERIFICATION_TOKEN_AMOUNT,
        request_id: Uuid::new_v4().to_string(),
        expires_at,
    };

    let stream_id = events
        .new_stream("cashu_portal_token_request", None)
        .await;

    let events = events.clone();
    let sid = stream_id.clone();
    tokio::spawn(async move {
        match sdk.request_cashu(recipient, subkeys, content).await {
            Ok(Some(r)) => {
                events
                    .push(&sid, NotificationData::CashuResponse { status: r.status })
                    .await;
            }
            Ok(None) => {
                events
                    .push(
                        &sid,
                        NotificationData::Error {
                            reason: "No response from recipient".to_string(),
                        },
                    )
                    .await;
            }
            Err(e) => {
                events
                    .push(
                        &sid,
                        NotificationData::Error {
                            reason: format!("Failed to request verification token: {e}"),
                        },
                    )
                    .await;
            }
        }
    });

    stream_id
}

// POST /verification/token
pub async fn request_verification_token(
    State(state): State<AppState>,
    Json(req): Json<crate::command::RequestVerificationTokenRequest>,
) -> ApiResult<StreamResponse> {
    let recipient_key =
        hex_to_pubkey(&req.recipient_key).map_err(|e| bad_request(format!("Invalid recipient key: {e}")))?;
    let subkeys = parse_subkeys(&req.subkeys).map_err(|e| bad_request(format!("Invalid subkeys: {e}")))?;

    let stream_id = spawn_verification_token_request(
        state.sdk.clone(),
        &state.events,
        recipient_key,
        subkeys,
    )
    .await;

    Ok(created(StreamResponse { stream_id }))
}
