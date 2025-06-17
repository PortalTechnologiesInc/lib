use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::command::{Command, CommandWithId};
use crate::response::*;
use crate::{AppState, PublicKey};
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use portal::protocol::model::payment::SinglePaymentRequestContent;
use portal::protocol::model::Timestamp;
use sdk::PortalSDK;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

struct SocketContext {
    sdk: Arc<PortalSDK>,
    nwc: Option<Arc<nwc::NWC>>,
    tx_message: mpsc::Sender<Message>,
    tx_notification: mpsc::Sender<Response>,
    active_streams: ActiveStreams,
}

impl SocketContext {
    fn new(
        sdk: Arc<PortalSDK>,
        nwc: Option<Arc<nwc::NWC>>,
        tx_message: mpsc::Sender<Message>,
        tx_notification: mpsc::Sender<Response>,
    ) -> Self {
        Self {
            sdk,
            nwc,
            tx_message,
            tx_notification,
            active_streams: ActiveStreams::new(),
        }
    }

    /// Helper to send a message to the client
    async fn send_message(&self, msg: Response) -> bool {
        match serde_json::to_string(&msg) {
            Ok(json) => match self.tx_message.send(Message::Text(json)).await {
                Ok(_) => true,
                Err(e) => {
                    error!("Error sending message: {}", e);
                    false
                }
            },
            Err(e) => {
                error!("Failed to serialize message: {}", e);
                false
            }
        }
    }

    async fn send_error_message(&self, request_id: &str, message: &str) -> bool {
        let response = Response::Error {
            id: request_id.to_string(),
            message: message.to_string(),
        };

        match serde_json::to_string(&response) {
            Ok(json) => match self.tx_message.send(Message::Text(json)).await {
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

    async fn create_outgoing_task(
        mut sender: SplitSink<WebSocket, Message>,
        mut rx_message: mpsc::Receiver<Message>,
    ) {
        while let Some(msg) = rx_message.recv().await {
            if let Err(e) = sender.send(msg).await {
                error!("Failed to send message to client: {}", e);
                break;
            }
        }
        debug!("Message forwarder task ending");
    }

    async fn create_notification_task(
        tx_message: mpsc::Sender<Message>,
        mut rx_notification: mpsc::Receiver<Response>,
    ) {
        while let Some(notification) = rx_notification.recv().await {
            match serde_json::to_string(&notification) {
                Ok(json) => {
                    if let Err(e) = tx_message.send(Message::Text(json)).await {
                        error!("Failed to forward notification: {}", e);
                        break;
                    }
                }
                Err(e) => error!("Failed to serialize notification: {}", e),
            }
        }
        debug!("Notification forwarder task ending");
    }
}

// Struct to track active notification streams
struct ActiveStreams {
    // Map of stream ID to cancellation handle
    tasks: DashMap<String, JoinHandle<()>>,
}

impl ActiveStreams {
    fn new() -> Self {
        Self {
            tasks: DashMap::new(),
        }
    }

    fn add_task(&self, id: String, handle: JoinHandle<()>) {
        if let Some(old_handle) = self.tasks.insert(id, handle) {
            old_handle.abort();
        }
    }

    fn remove_task(&mut self, id: &str) {
        if let Some((_, handle)) = self.tasks.remove(id) {
            handle.abort();
        }
    }
}

pub async fn handle_socket(socket: WebSocket, state: AppState) {
    let (sender, mut receiver) = socket.split();

    let (tx_notification, rx_notification) = mpsc::channel(32);
    let (tx_message, rx_message) = mpsc::channel(32);

    let ctx = Arc::new(SocketContext::new(
        state.sdk.clone(),
        state.nwc,
        tx_message.clone(),
        tx_notification,
    ));

    // Spawn a task to forward messages to the client
    let message_forward_task =
        tokio::spawn(SocketContext::create_outgoing_task(sender, rx_message));

    // Spawn a task to handle notifications
    let notification_task = tokio::spawn(SocketContext::create_notification_task(
        tx_message,
        rx_notification,
    ));

    let mut authenticated = false;

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

                        if !ctx.send_message(response).await {
                            break;
                        }
                    } else {
                        let _ = ctx.send_error_message(&id, "Authentication failed").await;
                        break; // Close connection on auth failure
                    }
                }
                Ok(command) => {
                    if !authenticated {
                        let _ = ctx
                            .send_error_message(&command.id, "Not authenticated")
                            .await;
                        break; // Close connection
                    }

                    let ctx_clone = ctx.clone();

                    tokio::task::spawn(async move {
                        // Handle authenticated commands
                        handle_command(command, ctx_clone).await;
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

                    if !ctx
                        .send_error_message(&id, &format!("Invalid command format: {}", e))
                        .await
                    {
                        break;
                    }
                }
            }
        }
    }

    // Clean up notification streams when the socket is closed
    {
        // let mut active_streams = active_streams.lock().unwrap();
        for handle in ctx.active_streams.tasks.iter() {
            handle.abort();
        }
    }

    // Also abort all tasks
    notification_task.abort();
    message_forward_task.abort();

    info!("WebSocket connection closed");
}

async fn handle_command(command: CommandWithId, ctx: Arc<SocketContext>) {
    match command.cmd {
        Command::Auth { .. } => {
            // Already handled in the outer function
        }
        Command::NewAuthInitUrl => {
            match ctx.sdk.new_auth_init_url().await {
                Ok((url, notification_stream)) => {
                    // Generate a unique stream ID
                    let stream_id = Uuid::new_v4().to_string();

                    // Setup notification forwarding
                    let tx_clone = ctx.tx_notification.clone();
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
                    ctx.active_streams.add_task(stream_id.clone(), task);

                    // Convert the URL to a proper response struct
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::AuthInitUrl {
                            url: url.to_string(),
                            stream_id,
                        },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
                            &command.id,
                            &format!("Failed to create auth init URL: {}", e),
                        )
                        .await;
                }
            }
        }
        Command::AuthenticateKey { main_key, subkeys } => {
            // Parse keys
            let main_key = match hex_to_pubkey(&main_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid main key: {}", e))
                        .await;
                    return;
                }
            };

            let subkeys = match parse_subkeys(&subkeys) {
                Ok(keys) => keys,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid subkeys: {}", e))
                        .await;
                    return;
                }
            };

            match ctx.sdk.authenticate_key(main_key, subkeys).await {
                Ok(event) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::AuthResponse {
                            event: AuthResponseData {
                                user_key: event.user_key.to_string(),
                                recipient: event.recipient.to_string(),
                                challenge: event.challenge,
                                status: event.status,
                            },
                        },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
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
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid main key: {}", e))
                        .await;
                    return;
                }
            };

            let subkeys = match parse_subkeys(&subkeys) {
                Ok(keys) => keys,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid subkeys: {}", e))
                        .await;
                    return;
                }
            };

            match ctx
                .sdk
                .request_recurring_payment(main_key, subkeys, payment_request)
                .await
            {
                Ok(status) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::RecurringPayment { status },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
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
            let nwc = match &ctx.nwc {
                Some(nwc) => nwc,
                None => {
                    let _ = ctx.send_error_message(&command.id, "Nostr Wallet Connect is not available: set the NWC_URL environment variable to enable it").await;
                    return;
                }
            };

            // Parse keys
            let main_key = match hex_to_pubkey(&main_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid main key: {}", e))
                        .await;
                    return;
                }
            };

            let subkeys = match parse_subkeys(&subkeys) {
                Ok(keys) => keys,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid subkeys: {}", e))
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
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Failed to make invoice: {}", e))
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

            match ctx
                .sdk
                .request_single_payment(main_key, subkeys, payment_request)
                .await
            {
                Ok(status) => {
                    // Generate a unique stream ID
                    let stream_id = Uuid::new_v4().to_string();

                    // Setup notification forwarding
                    let tx_clone = ctx.tx_notification.clone();
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
                    ctx.active_streams.add_task(stream_id.clone(), task);

                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::SinglePayment {
                            status,
                            stream_id: Some(stream_id),
                        },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
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
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid main key: {}", e))
                        .await;
                    return;
                }
            };

            let subkeys = match parse_subkeys(&subkeys) {
                Ok(keys) => keys,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid subkeys: {}", e))
                        .await;
                    return;
                }
            };

            match ctx
                .sdk
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

                    let _ = ctx.send_message(response).await;
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
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
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid main key: {}", e))
                        .await;
                    return;
                }
            };

            match ctx.sdk.fetch_profile(main_key).await {
                Ok(profile) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::ProfileData { profile },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Failed to fetch profile: {}", e))
                        .await;
                }
            }
        }
        Command::SetProfile { profile } => match ctx.sdk.set_profile(profile.clone()).await {
            Ok(_) => {
                let response = Response::Success {
                    id: command.id,
                    data: ResponseData::ProfileData {
                        profile: Some(profile),
                    },
                };

                let _ = ctx.send_message(response).await;
            }
            Err(e) => {
                let _ = ctx
                    .send_error_message(&command.id, &format!("Failed to set profile: {}", e))
                    .await;
            }
        },
        Command::ListenClosedRecurringPayment => {
            match ctx.sdk.listen_closed_recurring_payment().await {
                Ok(notification_stream) => {
                    // Generate a unique stream ID
                    let stream_id = Uuid::new_v4().to_string();

                    // Setup notification forwarding
                    let tx_clone = ctx.tx_notification.clone();
                    let stream_id_clone = stream_id.clone();

                    // Create a task to handle the notification stream
                    let task = tokio::spawn(async move {
                        let mut stream = notification_stream;

                        // Process notifications from the stream
                        while let Some(Ok(event)) = stream.next().await {
                            debug!("Got close recurring payment event: {:?}", event);

                            // Convert the event to a notification response
                            let notification = Response::Notification {
                                id: stream_id_clone.clone(),
                                data: NotificationData::ClosedRecurringPayment {
                                    reason: event.content.reason,
                                    subscription_id: event.content.subscription_id,
                                    main_key: event.main_key.to_string(),
                                    recipient: event.recipient.to_string(),
                                },
                            };

                            // Send the notification to the client
                            if let Err(e) = tx_clone.send(notification).await {
                                error!("Failed to forward close recurring payment event: {}", e);
                                break;
                            }
                        }

                        debug!(
                            "Closed Recurring Payment stream ended for stream_id: {}",
                            stream_id_clone
                        );
                    });

                    // Store the task
                    ctx.active_streams.add_task(stream_id.clone(), task);

                    // Convert the URL to a proper response struct
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::ListenClosedRecurringPayment { stream_id },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
                            &command.id,
                            &format!("Failed to create closed recurring payment listener: {}", e),
                        )
                        .await;
                }
            }
        }
        Command::CloseRecurringPayment {
            main_key,
            subkeys,
            subscription_id,
        } => {
            // Parse keys
            let main_key = match hex_to_pubkey(&main_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid main key: {}", e))
                        .await;
                    return;
                }
            };

            let subkeys = match parse_subkeys(&subkeys) {
                Ok(keys) => keys,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid subkeys: {}", e))
                        .await;
                    return;
                }
            };

            match ctx
                .sdk
                .close_recurring_payment(main_key, subkeys, subscription_id)
                .await
            {
                Ok(()) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::CloseRecurringPaymentSuccess {
                            message: String::from("Recurring payment closed"),
                        },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
                            &command.id,
                            &format!("Failed to close recurring payment: {}", e),
                        )
                        .await;
                }
            }
        }
        Command::SendInvoicePayment { inner } => {
            let recipient = inner.key.0;

            match ctx.sdk.send_invoice_payment(inner).await {
                Ok(invoice_response) => {
                    match invoice_response {
                        Some(invoice_response) => {
                            let response = Response::Success {
                                id: command.id,
                                data: ResponseData::InvoicePayment {
                                    invoice: invoice_response.invoice,
                                    payment_hash: invoice_response.payment_hash,
                                },
                            };

                            let _ = ctx.send_message(response).await;
                        }
                        None => {
                            // Recipient did not reply with a invoice
                            let _ = ctx
                                .send_error_message(
                                    &command.id,
                                    &format!(
                                        "Recipient '{:?}' did not reply with a invoice",
                                        recipient
                                    ),
                                )
                                .await;
                        }
                    }
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
                            &command.id,
                            &format!("Failed to send invoice payment: {}", e),
                        )
                        .await;
                }
            }
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
