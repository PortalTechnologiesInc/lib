use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use portal::profile::Profile;
use portal::protocol::model::payment::{
    Currency, PaymentResponseContent, RecurringPaymentRequestContent, RecurringPaymentResponseContent, SinglePaymentRequestContent
};
use portal::protocol::model::Timestamp;
use sdk::{PortalSDK};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{AppState, PublicKey};

// Commands that can be sent from client to server
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", content = "params")]
enum Command {
    // Authentication command - must be first command sent
    Auth {
        token: String,
    },

    // SDK methods
    NewAuthInitUrl,
    AuthenticateKey {
        main_key: String,
        subkeys: Vec<String>,
    },
    RequestRecurringPayment {
        main_key: String,
        subkeys: Vec<String>,
        payment_request: RecurringPaymentRequestContent,
    },
    RequestSinglePayment {
        main_key: String,
        subkeys: Vec<String>,
        payment_request: SinglePaymentParams,
    },
    RequestPaymentRaw {
        main_key: String,
        subkeys: Vec<String>,
        payment_request: SinglePaymentRequestContent,
    },
    FetchProfile {
        main_key: String,
    },
    SetProfile {
        profile: Profile,
    },
}

#[derive(Debug, Deserialize)]
struct CommandWithId {
    id: String,
    #[serde(flatten)]
    cmd: Command,
}

// Request parameter structs
#[derive(Debug, Deserialize)]
struct AuthParams {
    token: String,
}

#[derive(Debug, Deserialize)]
struct KeyParams {
    main_key: String,
    subkeys: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SinglePaymentParams {
    description: String,
    amount: u64,
    currency: Currency,
    subscription_id: Option<String>,
    auth_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileParams {
    main_key: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum InvoiceStatus {
    Paid { preimage: Option<String> },
    Timeout,
    Error { reason: String },
}

// Response structs for each API
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Response {
    #[serde(rename = "error")]
    Error { id: String, message: String },

    #[serde(rename = "success")]
    Success { id: String, data: ResponseData },

    #[serde(rename = "notification")]
    Notification { id: String, data: NotificationData },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ResponseData {
    #[serde(rename = "auth_success")]
    AuthSuccess { message: String },

    #[serde(rename = "auth_init_url")]
    AuthInitUrl { url: String, stream_id: String },

    #[serde(rename = "auth_response")]
    AuthResponse { event: AuthResponseData },

    #[serde(rename = "recurring_payment")]
    RecurringPayment {
        status: RecurringPaymentResponseContent,
    },

    #[serde(rename = "single_payment")]
    SinglePayment {
        status: PaymentResponseContent,
        stream_id: Option<String>,
    },

    #[serde(rename = "profile")]
    ProfileData { profile: Option<Profile> },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum NotificationData {
    #[serde(rename = "auth_init")]
    AuthInit { main_key: String },
    #[serde(rename = "payment_status_update")]
    PaymentStatusUpdate { status: InvoiceStatus },
}

#[derive(Debug, Serialize)]
struct AuthInitUrlResponse {
    main_key: String,
    relays: Vec<String>,
    token: String,
    subkey: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuthResponseData {
    user_key: String,
    recipient: String,
    challenge: String,
    granted_permissions: Vec<String>,
    session_token: String,
}

// Struct to track active notification streams
struct ActiveStreams {
    // Map of stream ID to cancellation handle
    tasks: HashMap<String, JoinHandle<()>>,
}

impl ActiveStreams {
    fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    fn add_task(&mut self, id: String, handle: JoinHandle<()>) {
        if let Some(old_handle) = self.tasks.insert(id, handle) {
            old_handle.abort();
        }
    }

    fn remove_task(&mut self, id: &str) {
        if let Some(handle) = self.tasks.remove(id) {
            handle.abort();
        }
    }
}

pub async fn handle_socket(socket: WebSocket, state: AppState) {
    let (sender, mut receiver) = socket.split();

    // Authentication state
    let mut authenticated = false;

    // Track active notification streams
    let active_streams = Arc::new(Mutex::new(ActiveStreams::new()));

    // Create channels for sending messages to client
    let (tx_notification, mut rx_notification) = mpsc::channel(32);
    let (tx_message, mut rx_message) = mpsc::channel(32);

    // Spawn a task to forward messages to the client
    let message_forward_task = {
        tokio::spawn(async move {
            let mut sender_sink = sender;
            while let Some(msg) = rx_message.recv().await {
                if let Err(e) = sender_sink.send(msg).await {
                    error!("Failed to send message to client: {}", e);
                    break;
                }
            }
            debug!("Message forwarder task ending");
        })
    };

    // Spawn a task to handle notifications
    let notification_task = {
        let tx_message_clone = tx_message.clone();
        tokio::spawn(async move {
            while let Some(notification) = rx_notification.recv().await {
                match serde_json::to_string(&notification) {
                    Ok(json) => {
                        if let Err(e) = tx_message_clone.send(Message::Text(json)).await {
                            error!("Failed to forward notification: {}", e);
                            break;
                        }
                    }
                    Err(e) => error!("Failed to serialize notification: {}", e),
                }
            }
            debug!("Notification forwarder task ending");
        })
    };

    // Helper to send a message to the client
    let send_message = |msg: Response| {
        let tx = tx_message.clone();
        async move {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if let Err(e) = tx.send(Message::Text(json)).await {
                        error!("Failed to send message: {}", e);
                        false
                    } else {
                        true
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    false
                }
            }
        }
    };

    // Process incoming messages
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            debug!("Received message: {}", text);

            // Try to parse the command
            match serde_json::from_str(&text) {
                Ok(CommandWithId {
                    id,
                    cmd: Command::Auth { token },
                }) => {
                    if token == state.auth_token {
                        authenticated = true;
                        let response = Response::Success {
                            id: id.clone(),
                            data: ResponseData::AuthSuccess {
                                message: "Authenticated successfully".to_string(),
                            },
                        };

                        if !send_message(response).await {
                            break;
                        }
                    } else {
                        let response = Response::Error {
                            id,
                            message: "Authentication failed".to_string(),
                        };

                        let _ = send_message(response).await;
                        break; // Close connection on auth failure
                    }
                }
                Ok(command) => {
                    if !authenticated {
                        let response = Response::Error {
                            id: command.id,
                            message: "Not authenticated".to_string(),
                        };

                        let _ = send_message(response).await;
                        break; // Close connection
                    }

                    let tx_message_clone = tx_message.clone();
                    let active_streams_clone = active_streams.clone();
                    let tx_notification_clone = tx_notification.clone();
                    let sdk_clone = state.sdk.clone();
                    let nwc_clone = state.nwc.clone();
                    tokio::task::spawn(async move {
                        // Handle authenticated commands
                        handle_command(
                            command,
                            &sdk_clone,
                            &nwc_clone,
                            tx_message_clone,
                            &active_streams_clone,
                            tx_notification_clone,
                        )
                        .await;
                    });
                }
                Err(e) => {
                    // Still try to get a request id from the command
                    let command = serde_json::from_str::<serde_json::Value>(&text);
                    let id = command
                        .ok()
                        .and_then(|v| v.get("id").cloned())
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_default();

                    warn!("Failed to parse command: {}", e);
                    let response = Response::Error {
                        id,
                        message: format!("Invalid command format: {}", e),
                    };

                    if !send_message(response).await {
                        break;
                    }
                }
            }
        }
    }

    // Clean up notification streams when the socket is closed
    {
        let mut active_streams = active_streams.lock().unwrap();
        for (_, handle) in active_streams.tasks.drain() {
            handle.abort();
        }
    }

    // Also abort all tasks
    notification_task.abort();
    message_forward_task.abort();

    info!("WebSocket connection closed");
}

async fn handle_command(
    command: CommandWithId,
    sdk: &Arc<PortalSDK>,
    nwc: &Option<Arc<nwc::NWC>>,
    tx_message: mpsc::Sender<Message>,
    active_streams: &Arc<Mutex<ActiveStreams>>,
    tx_notification: mpsc::Sender<Response>,
) {
    // Helper to send a message to the client
    let send_message = |msg: Response| {
        let tx = tx_message.clone();
        async move {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if let Err(e) = tx.send(Message::Text(json)).await {
                        error!("Failed to send message: {}", e);
                        false
                    } else {
                        true
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    false
                }
            }
        }
    };

    match command.cmd {
        Command::Auth { .. } => {
            // Already handled in the outer function
        }
        Command::NewAuthInitUrl => {
            match sdk.new_auth_init_url().await {
                Ok((url, notification_stream)) => {
                    // Generate a unique stream ID
                    let stream_id = Uuid::new_v4().to_string();

                    // Setup notification forwarding
                    let tx_clone = tx_notification.clone();
                    let stream_id_clone = stream_id.clone();

                    // Create a task to handle the notification stream
                    let task = tokio::spawn(async move {
                        let mut stream = notification_stream;

                        // Process notifications from the stream
                        while let Some(Ok(event)) = stream.next().await {
                            debug!("Got auth init event: {:?}", event);

                            // Convert the event to a notification response
                            let notification = Response::Notification {
                                id: stream_id_clone.clone(),
                                data: NotificationData::AuthInit {
                                    main_key: event.main_key.to_string(),
                                },
                            };

                            // Send the notification to the client
                            if let Err(e) = tx_clone.send(notification).await {
                                error!("Failed to forward auth init event: {}", e);
                                break;
                            }
                        }

                        debug!("Auth init stream ended for stream_id: {}", stream_id_clone);
                    });

                    // Store the task
                    active_streams
                        .lock()
                        .unwrap()
                        .add_task(stream_id.clone(), task);

                    // Convert the URL to a proper response struct
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::AuthInitUrl {
                            url: url.to_string(),
                            stream_id,
                        },
                    };

                    let _ = send_message(response).await;
                }
                Err(e) => {
                    let response = Response::Error {
                        id: command.id.to_string(),
                        message: format!("Failed to create auth init URL: {}", e),
                    };

                    let _ = send_message(response).await;
                }
            }
        }
        Command::AuthenticateKey { main_key, subkeys } => {
            // Parse keys
            let main_key = match hex_to_pubkey(&main_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Invalid main key: {}", e),
                    )
                    .await;
                    return;
                }
            };

            let subkeys = match parse_subkeys(&subkeys) {
                Ok(keys) => keys,
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Invalid subkeys: {}", e),
                    )
                    .await;
                    return;
                }
            };

            match sdk.authenticate_key(main_key, subkeys).await {
                Ok(event) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::AuthResponse {
                            event: AuthResponseData {
                                user_key: event.user_key.to_string(),
                                recipient: event.recipient.to_string(),
                                challenge: event.challenge,
                                granted_permissions: event.granted_permissions,
                                session_token: event.session_token,
                            },
                        },
                    };

                    let _ = send_message(response).await;
                }
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Failed to authenticate key: {}", e),
                    )
                    .await;
                }
            }
        }
        Command::RequestRecurringPayment {
            main_key,
            subkeys,
            payment_request,
        } => {
            // Parse keys
            let main_key = match hex_to_pubkey(&main_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Invalid main key: {}", e),
                    )
                    .await;
                    return;
                }
            };

            let subkeys = match parse_subkeys(&subkeys) {
                Ok(keys) => keys,
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Invalid subkeys: {}", e),
                    )
                    .await;
                    return;
                }
            };

            match sdk
                .request_recurring_payment(main_key, subkeys, payment_request)
                .await
            {
                Ok(status) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::RecurringPayment { status },
                    };

                    let _ = send_message(response).await;
                }
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Failed to request recurring payment: {}", e),
                    )
                    .await;
                }
            }
        }
        Command::RequestSinglePayment {
            main_key,
            subkeys,
            payment_request,
        } => {
            let nwc = match nwc {
                Some(nwc) => nwc,
                None => {
                    let _ = send_error(tx_message.clone(), &command.id, "Nostr Wallet Connect is not available: set the NWC_URL environment variable to enable it").await;
                    return;
                }
            };

            // Parse keys
            let main_key = match hex_to_pubkey(&main_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Invalid main key: {}", e),
                    )
                    .await;
                    return;
                }
            };

            let subkeys = match parse_subkeys(&subkeys) {
                Ok(keys) => keys,
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Invalid subkeys: {}", e),
                    )
                    .await;
                    return;
                }
            };

            // TODO: fetch and apply fiat exchange rate

            let invoice = match nwc
                .make_invoice(portal::nostr::nips::nip47::MakeInvoiceRequest {
                    amount: payment_request.amount,
                    description: Some(payment_request.description.clone()),
                    description_hash: None,
                    expiry: None,
                })
                .await
            {
                Ok(invoice) => invoice,
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Failed to make invoice: {}", e),
                    )
                    .await;
                    return;
                }
            };

            let expires_at = Timestamp::now_plus_seconds(300);
            let payment_request = SinglePaymentRequestContent {
                amount: payment_request.amount,
                currency: payment_request.currency,
                expires_at,
                invoice: invoice.invoice.clone(),
                current_exchange_rate: None,
                subscription_id: payment_request.subscription_id,
                auth_token: payment_request.auth_token,
                request_id: command.id.clone(),
                description: Some(payment_request.description),
            };

            match sdk
                .request_single_payment(main_key, subkeys, payment_request)
                .await
            {
                Ok(status) => {
                    // Generate a unique stream ID
                    let stream_id = Uuid::new_v4().to_string();

                    // Setup notification forwarding
                    let tx_clone = tx_notification.clone();
                    let stream_id_clone = stream_id.clone();
                    let nwc_clone = nwc.clone();

                    // Create a task to handle the notification stream
                    let task = tokio::spawn(async move {
                        let mut count = 0;
                        let notification = loop {
                            if Timestamp::now() > expires_at {
                                break NotificationData::PaymentStatusUpdate {
                                    status: InvoiceStatus::Timeout,
                                };
                            }

                            count += 1;
                            if std::env::var("FAKE_PAYMENTS").is_ok() && count > 3 {
                                break NotificationData::PaymentStatusUpdate {
                                    status: InvoiceStatus::Paid { preimage: None },
                                };
                            }

                            let invoice = nwc_clone
                                .lookup_invoice(portal::nostr::nips::nip47::LookupInvoiceRequest {
                                    invoice: Some(invoice.invoice.clone()),
                                    payment_hash: None,
                                })
                                .await;

                            match invoice {
                                Ok(invoice) => {
                                    if invoice.settled_at.is_some() {
                                        break NotificationData::PaymentStatusUpdate {
                                            status: InvoiceStatus::Paid {
                                                preimage: invoice.preimage,
                                            },
                                        };
                                    } else {
                                        // TODO: incremental delay
                                        tokio::time::sleep(tokio::time::Duration::from_millis(
                                            1000,
                                        ))
                                        .await;

                                        continue;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to lookup invoice: {}", e);
                                    break NotificationData::PaymentStatusUpdate {
                                        status: InvoiceStatus::Error {
                                            reason: e.to_string(),
                                        },
                                    };
                                }
                            }
                        };

                        // Convert the event to a notification response
                        let notification = Response::Notification {
                            id: stream_id_clone.clone(),
                            data: notification,
                        };

                        // Send the notification to the client
                        if let Err(e) = tx_clone.send(notification).await {
                            error!("Failed to forward payment event: {}", e);
                        }
                    });

                    // Store the task
                    active_streams
                        .lock()
                        .unwrap()
                        .add_task(stream_id.clone(), task);

                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::SinglePayment {
                            status,
                            stream_id: Some(stream_id),
                        },
                    };

                    let _ = send_message(response).await;
                }
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Failed to request single payment: {}", e),
                    )
                    .await;
                }
            }
        }
        Command::RequestPaymentRaw {
            main_key,
            subkeys,
            payment_request,
        } => {
            // Parse keys
            let main_key = match hex_to_pubkey(&main_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Invalid main key: {}", e),
                    )
                    .await;
                    return;
                }
            };

            let subkeys = match parse_subkeys(&subkeys) {
                Ok(keys) => keys,
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Invalid subkeys: {}", e),
                    )
                    .await;
                    return;
                }
            };

            match sdk
                .request_single_payment(main_key, subkeys, payment_request)
                .await
            {
                Ok(status) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::SinglePayment {
                            status,
                            stream_id: None,
                        },
                    };

                    let _ = send_message(response).await;
                }
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Failed to request single payment: {}", e),
                    )
                    .await;
                }
            }
        }
        Command::FetchProfile { main_key } => {
            // Parse key
            let main_key = match hex_to_pubkey(&main_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Invalid main key: {}", e),
                    )
                    .await;
                    return;
                }
            };

            match sdk.fetch_profile(main_key).await {
                Ok(profile) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::ProfileData { profile },
                    };

                    let _ = send_message(response).await;
                }
                Err(e) => {
                    let _ = send_error(
                        tx_message.clone(),
                        &command.id,
                        &format!("Failed to fetch profile: {}", e),
                    )
                    .await;
                }
            }
        }
        Command::SetProfile { profile } => {
            match sdk.set_profile(profile.clone()).await {
                Ok(_) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::ProfileData { profile: Some(profile) },
                    };

                    let _ = send_message(response).await;
                }
                Err(e) => {
                    let _ = send_error(tx_message.clone(), &command.id, &format!("Failed to set profile: {}", e)).await;
                }
            }
        }
    }
}

async fn send_error(tx: mpsc::Sender<Message>, request_id: &str, message: &str) -> bool {
    let response = Response::Error {
        id: request_id.to_string(),
        message: message.to_string(),
    };

    match serde_json::to_string(&response) {
        Ok(json) => match tx.send(Message::Text(json)).await {
            Ok(_) => true,
            Err(e) => {
                error!("Error sending error response: {}", e);
                false
            }
        },
        Err(e) => {
            error!("Failed to serialize error response: {}", e);
            false
        }
    }
}

fn hex_to_pubkey(hex: &str) -> Result<PublicKey, String> {
    hex.parse::<PublicKey>().map_err(|e| e.to_string())
}

fn parse_subkeys(subkeys: &[String]) -> Result<Vec<PublicKey>, String> {
    let mut result = Vec::with_capacity(subkeys.len());
    for subkey in subkeys {
        result.push(hex_to_pubkey(subkey)?);
    }
    Ok(result)
}
