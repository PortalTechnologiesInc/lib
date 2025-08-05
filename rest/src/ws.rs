use std::str::FromStr;
use std::sync::Arc;

use crate::command::{Command, CommandWithId};
use crate::response::*;
use crate::{AppState, PublicKey};
use axum::extract::ws::{Message, WebSocket};
use cdk::amount::SplitTarget;
use cdk::mint_url::MintUrl;
use cdk::nuts::CurrencyUnit;
use cdk::wallet::{SendOptions, Wallet, WalletBuilder};
use cdk_sqlite::wallet::memory;
use chrono::Duration;
use dashmap::DashMap;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use portal::nostr_relay_pool::RelayOptions;
use portal::protocol::jwt::CustomClaims;
use portal::protocol::model::payment::{
    CashuDirectContent, CashuRequestContent, PaymentStatus, SinglePaymentRequestContent,
};
use portal::protocol::model::Timestamp;
use rand::RngCore;
use sdk::PortalSDK;
use tokio::sync::{mpsc, Mutex};
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

    // fn remove_task(&mut self, id: &str) {
    //     if let Some((_, handle)) = self.tasks.remove(id) {
    //         handle.abort();
    //     }
    // }
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
        Command::NewKeyHandshakeUrl { static_token } => {
            match ctx.sdk.new_key_handshake_url(static_token).await {
                Ok((url, notification_stream)) => {
                    // Generate a unique stream ID
                    let stream_id = Uuid::new_v4().to_string();

                    // Setup notification forwarding
                    let tx_clone = ctx.tx_notification.clone();
                    let relay_pool = ctx.sdk.relay_pool();
                    let stream_id_clone = stream_id.clone();

                    // Create a task to handle the notification stream
                    let task = tokio::spawn(async move {
                        let mut stream = notification_stream;

                        // Process notifications from the stream
                        while let Some(Ok(event)) = stream.next().await {
                            debug!("Got auth init event: {:?}", event);

                            // Connect to relays
                            // TODO: clearly we should have some policies here, we shouldn't connect to all relays unconditionally
                            let preferred_relays = event.relays.clone();
                            for relay in &preferred_relays {
                                match relay_pool.add_relay(relay, RelayOptions::default()).await {
                                    Ok(false) => {
                                        continue;
                                    }
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

                            // Convert the event to a notification response
                            let notification = Response::Notification {
                                id: stream_id_clone.clone(),
                                data: NotificationData::KeyHandshake {
                                    main_key: event.main_key.to_string(),
                                    preferred_relays,
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
                        data: ResponseData::KeyHandshakeUrl {
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

            let mut notifications = match ctx
                .sdk
                .request_single_payment(main_key, subkeys, payment_request)
                .await
            {
                Ok(notifications) => notifications,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
                            &command.id,
                            &format!("Failed to request single payment: {}", e),
                        )
                        .await;
                    return;
                }
            };

            // Generate a unique stream ID
            let stream_id = Uuid::new_v4().to_string();
            let tx_clone = ctx.tx_notification.clone();
            let nwc_clone = nwc.clone();

            let stream_id_clone = stream_id.clone();
            let monitor = Mutex::new(None);
            let task = tokio::spawn(async move {
                while let Some(notification) = notifications.next().await {
                    match notification {
                        Ok(status) => {
                            let notification = match &status.status {
                                PaymentStatus::Failed { reason } => {
                                    NotificationData::PaymentStatusUpdate {
                                        status: InvoiceStatus::UserFailed {
                                            reason: reason.clone(),
                                        },
                                    }
                                }
                                PaymentStatus::Rejected { reason } => {
                                    NotificationData::PaymentStatusUpdate {
                                        status: InvoiceStatus::UserRejected {
                                            reason: reason.clone(),
                                        },
                                    }
                                }
                                PaymentStatus::Success { preimage } => {
                                    NotificationData::PaymentStatusUpdate {
                                        status: InvoiceStatus::UserSuccess {
                                            preimage: preimage.clone(),
                                        },
                                    }
                                }
                                PaymentStatus::Approved => NotificationData::PaymentStatusUpdate {
                                    status: InvoiceStatus::UserApproved,
                                },
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

                            if status.status.is_final() {
                                // If the status is final we can exit now
                                return;
                            }

                            if monitor.lock().await.is_some() {
                                // Monitor already started
                                return;
                            }

                            // Let's start monitoring the invoice via NWC
                            let stream_id_clone = stream_id_clone.clone();
                            let tx_clone = tx_clone.clone();
                            let nwc_clone = nwc_clone.clone();
                            let invoice_clone = invoice.invoice.clone();

                            *monitor.lock().await = Some(tokio::spawn(async move {
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
                                        .lookup_invoice(
                                            portal::nostr::nips::nip47::LookupInvoiceRequest {
                                                invoice: Some(invoice_clone.clone()),
                                                payment_hash: None,
                                            },
                                        )
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
                                                tokio::time::sleep(
                                                    tokio::time::Duration::from_millis(1000),
                                                )
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
                            }));
                        }
                        Err(e) => {
                            error!("Failed to request single payment: {}", e);
                        }
                    }
                }
            });

            // Store the task
            ctx.active_streams.add_task(stream_id.clone(), task);

            let response = Response::Success {
                id: command.id,
                data: ResponseData::SinglePayment { stream_id },
            };

            let _ = ctx.send_message(response).await;
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

            let mut notifications = match ctx
                .sdk
                .request_single_payment(main_key, subkeys, payment_request)
                .await
            {
                Ok(notifications) => notifications,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
                            &command.id,
                            &format!("Failed to request single payment: {}", e),
                        )
                        .await;
                    return;
                }
            };

            // Generate a unique stream ID
            let stream_id = Uuid::new_v4().to_string();
            let tx_clone = ctx.tx_notification.clone();

            // Setup notification forwarding
            let stream_id_clone = stream_id.clone();
            let task = tokio::spawn(async move {
                while let Some(notification) = notifications.next().await {
                    match notification {
                        Ok(status) => {
                            let notification = match status.status {
                                PaymentStatus::Failed { reason } => {
                                    NotificationData::PaymentStatusUpdate {
                                        status: InvoiceStatus::UserFailed { reason },
                                    }
                                }
                                PaymentStatus::Rejected { reason } => {
                                    NotificationData::PaymentStatusUpdate {
                                        status: InvoiceStatus::UserRejected { reason },
                                    }
                                }
                                PaymentStatus::Success { preimage } => {
                                    NotificationData::PaymentStatusUpdate {
                                        status: InvoiceStatus::UserSuccess { preimage },
                                    }
                                }
                                PaymentStatus::Approved => NotificationData::PaymentStatusUpdate {
                                    status: InvoiceStatus::UserApproved,
                                },
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
                        }
                        Err(e) => {
                            error!("Failed to request single payment: {}", e);
                        }
                    }
                }
            });

            // Store the task
            ctx.active_streams.add_task(stream_id.clone(), task);

            let response = Response::Success {
                id: command.id,
                data: ResponseData::SinglePayment { stream_id },
            };

            let _ = ctx.send_message(response).await;
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
        Command::RequestInvoice {
            recipient_key,
            subkeys,
            content,
        } => {
            // Parse keys
            let recipient_key = match hex_to_pubkey(&recipient_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid recipient key: {}", e))
                        .await;
                    return;
                }
            };

            // Parse subkeys
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
                .request_invoice(recipient_key.into(), subkeys, content)
                .await
            {
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
                                        recipient_key
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
        Command::IssueJwt {
            target_key,
            duration_hours,
        } => {
            let target_key = match hex_to_pubkey(&target_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid target_key: {}", e))
                        .await;
                    return;
                }
            };

            match ctx.sdk.issue_jwt(
                CustomClaims::new(target_key.into()),
                Duration::hours(duration_hours),
            ) {
                Ok(token) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::IssueJwt { token },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Err(err) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Failed to issue JWT: {}", err))
                        .await;
                }
            }
        }
        Command::VerifyJwt { pubkey, token } => {
            let public_key = match hex_to_pubkey(&pubkey) {
                Ok(key) => key,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid pubkey: {}", e))
                        .await;
                    return;
                }
            };

            match ctx.sdk.verify_jwt(public_key, &token) {
                Ok(claims) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::VerifyJwt {
                            target_key: claims.target_key.to_string(),
                        },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Err(err) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Failed to verify JWT: {}", err))
                        .await;
                }
            }
        }
        Command::RequestCashu {
            recipient_key,
            subkeys,
            mint_url,
            unit,
            amount,
        } => {
            // Parse keys
            let recipient_key = match hex_to_pubkey(&recipient_key) {
                Ok(key) => key,
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Invalid recipient key: {}", e))
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

            let content = CashuRequestContent {
                mint_url,
                unit,
                amount,
                request_id: Uuid::new_v4().to_string(),
            };
            match ctx.sdk.request_cashu(recipient_key, subkeys, content).await {
                Ok(Some(response)) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::CashuResponse {
                            status: response.status,
                        },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Ok(None) => {
                    let _ = ctx
                        .send_error_message(&command.id, "No response from recipient")
                        .await;
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(&command.id, &format!("Failed to request cashu: {}", e))
                        .await;
                }
            }
        }

        Command::SendCashuDirect {
            main_key,
            subkeys,
            token,
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
                .send_cashu_direct(main_key, subkeys, CashuDirectContent { token })
                .await
            {
                Ok(()) => {
                    let response = Response::Success {
                        id: command.id,
                        data: ResponseData::SendCashuDirectSuccess {
                            message: String::from("Cashu direct sent"),
                        },
                    };

                    let _ = ctx.send_message(response).await;
                }
                Err(e) => {
                    let _ = ctx
                        .send_error_message(
                            &command.id,
                            &format!("Failed to send cashu direct: {}", e),
                        )
                        .await;
                }
            }
        }
        Command::MintCashu {
            mint_url,
            static_auth_token,
            unit,
            amount,
            description,
        } => {
            // Mint tokens using cdk wallet
            let ctx_clone = ctx.clone();
            let command_id = command.id.clone();
            let mint_url = mint_url.clone();
            let unit = unit.clone();
            tokio::task::spawn(async move {
                let mint_url = match MintUrl::from_str(&mint_url) {
                    Ok(url) => url,
                    Err(e) => {
                        let _ = ctx_clone
                            .send_error_message(&command_id, &format!("Invalid mint URL: {}", e))
                            .await;
                        return;
                    }
                };
                let currency_unit = match CurrencyUnit::from_str(&unit) {
                    Ok(u) => u,
                    Err(e) => {
                        let _ = ctx_clone
                            .send_error_message(&command_id, &format!("Invalid unit: {}", e))
                            .await;
                        return;
                    }
                };

                let wallet =
                    match get_cashu_wallet(mint_url, currency_unit, static_auth_token).await {
                        Ok(w) => w,
                        Err(e) => {
                            let _ = ctx_clone
                                .send_error_message(
                                    &command_id,
                                    &format!("Failed to create wallet: {}", e),
                                )
                                .await;
                            return;
                        }
                    };

                // Request minting (this will typically require paying an invoice, but for static-token mints it may be instant)
                let quote = match wallet.mint_quote(amount.into(), description).await {
                    Ok(quote) => quote,
                    Err(e) => {
                        let _ = ctx_clone
                            .send_error_message(
                                &command_id,
                                &format!("Failed to get mint quote: {}", e),
                            )
                            .await;
                        return;
                    }
                };
                let result = wallet.mint(&quote.id, SplitTarget::None, None).await;
                match result {
                    Ok(proofs) => proofs,
                    Err(e) => {
                        let _ = ctx_clone
                            .send_error_message(
                                &command_id,
                                &format!("Failed to mint token: {}", e),
                            )
                            .await;
                        return;
                    }
                };
                let prepared_send = match wallet
                    .prepare_send(amount.into(), SendOptions::default())
                    .await
                {
                    Ok(prepared_send) => prepared_send,
                    Err(e) => {
                        let _ = ctx_clone
                            .send_error_message(
                                &command_id,
                                &format!("Failed to prepare send: {}", e),
                            )
                            .await;
                        return;
                    }
                };
                let token = match wallet.send(prepared_send, None).await {
                    Ok(token) => token,
                    Err(e) => {
                        let _ = ctx_clone
                            .send_error_message(
                                &command_id,
                                &format!("Failed to send token: {}", e),
                            )
                            .await;
                        return;
                    }
                };

                let response = Response::Success {
                    id: command_id,
                    data: ResponseData::CashuMint {
                        token: token.to_string(),
                    },
                };
                let _ = ctx_clone.send_message(response).await;
            });
        }
        Command::BurnCashu {
            mint_url,
            unit,
            token,
            static_auth_token,
        } => {
            // Burn tokens using cdk wallet
            let ctx_clone = ctx.clone();
            let command_id = command.id.clone();
            let mint_url = mint_url.clone();
            let unit = unit.clone();
            tokio::task::spawn(async move {
                let mint_url = match MintUrl::from_str(&mint_url) {
                    Ok(url) => url,
                    Err(e) => {
                        let _ = ctx_clone
                            .send_error_message(&command_id, &format!("Invalid mint URL: {}", e))
                            .await;
                        return;
                    }
                };
                let currency_unit = match CurrencyUnit::from_str(&unit) {
                    Ok(u) => u,
                    Err(e) => {
                        let _ = ctx_clone
                            .send_error_message(&command_id, &format!("Invalid unit: {}", e))
                            .await;
                        return;
                    }
                };

                let wallet =
                    match get_cashu_wallet(mint_url, currency_unit, static_auth_token).await {
                        Ok(w) => w,
                        Err(e) => {
                            let _ = ctx_clone
                                .send_error_message(
                                    &command_id,
                                    &format!("Failed to create wallet: {}", e),
                                )
                                .await;
                            return;
                        }
                    };

                let receive = match wallet.receive(&token, Default::default()).await {
                    Ok(receive) => receive,
                    Err(e) => {
                        let _ = ctx_clone
                            .send_error_message(
                                &command_id,
                                &format!("Failed to receive token: {}", e),
                            )
                            .await;
                        return;
                    }
                };

                let response = Response::Success {
                    id: command_id,
                    data: ResponseData::CashuBurn {
                        amount: receive.into(),
                    },
                };
                let _ = ctx_clone.send_message(response).await;
            });
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

async fn get_cashu_wallet(
    mint_url: MintUrl,
    unit: CurrencyUnit,
    static_auth_token: Option<String>,
) -> Result<Wallet, String> {
    // Generate a random seed for the temporary wallet
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);

    // Use an in-memory localstore (no persistence)
    let localstore = Arc::new(memory::empty().await.expect("Failed to create localstore"));
    let mut builder = WalletBuilder::new()
        .mint_url(mint_url)
        .unit(unit)
        .localstore(localstore)
        .seed(&seed);

    if let Some(static_auth_token) = static_auth_token {
        builder = builder.static_token(static_auth_token);
    }

    let wallet = builder.build().map_err(|e| e.to_string())?;

    // Fetch mint info to cache endpoint auth requirements
    wallet.get_mint_info().await.map_err(|e| e.to_string())?;

    Ok(wallet)
}
