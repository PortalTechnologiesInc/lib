use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::config::WebhookSettings;
use crate::response::{NotificationData, StreamEvent};
use crate::webhook;

/// Stream status in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum StreamStatus {
    InFlight,
    Completed,
    Failed,
}

impl StreamStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InFlight => "in_flight",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "in_flight" => Some(Self::InFlight),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Metadata stored per stream type, used for startup recovery.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "stream_type", rename_all = "snake_case")]
pub enum StreamMetadata {
    KeyHandshake {
        /// The URL token from the handshake URL, if available.
        url: String,
    },
    SinglePayment {
        /// The Lightning invoice for monitoring.
        invoice: String,
        /// Expiry timestamp as Unix seconds.
        expires_at_secs: u64,
    },
    RecurringPaymentClose,
    /// Generic stream with no recovery metadata.
    Other,
}

/// Info about an in-flight stream, used for startup recovery.
#[derive(Debug, Clone)]
pub struct InFlightStream {
    pub stream_id: String,
    pub stream_type: String,
    pub metadata: Option<StreamMetadata>,
}

/// SQLite-backed store for stream events. Events are appended per stream_id and can be
/// polled by clients via `GET /events/{stream_id}?after={index}`.
///
/// When a webhook URL is configured, events are also delivered via HTTP POST.
///
/// Stream IDs and events survive server restarts.
#[derive(Clone)]
pub struct EventStore {
    db: Arc<Mutex<Connection>>,
    webhook_settings: WebhookSettings,
}

impl EventStore {
    /// Open (or create) the SQLite database at `db_path` and initialize the schema.
    pub fn new(db_path: &str, webhook_settings: WebhookSettings) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS streams (
                stream_id TEXT PRIMARY KEY,
                stream_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'in_flight',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                metadata TEXT
            );

            CREATE TABLE IF NOT EXISTS stream_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                stream_id TEXT NOT NULL REFERENCES streams(stream_id),
                event_index INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                data TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_stream_events_stream_id
                ON stream_events(stream_id, event_index);",
        )?;

        info!("SQLite database opened at {db_path}");

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            webhook_settings,
        })
    }

    /// Create a new stream entry in the database.
    pub async fn create_stream(
        &self,
        stream_id: &str,
        stream_type: &str,
        metadata: Option<&StreamMetadata>,
    ) {
        let now = chrono::Utc::now().timestamp();
        let meta_json = metadata.and_then(|m| serde_json::to_string(m).ok());
        let db = self.db.lock().await;
        if let Err(e) = db.execute(
            "INSERT INTO streams (stream_id, stream_type, status, created_at, updated_at, metadata)
             VALUES (?1, ?2, 'in_flight', ?3, ?3, ?4)",
            rusqlite::params![stream_id, stream_type, now, meta_json],
        ) {
            error!("Failed to create stream {stream_id}: {e}");
        }
    }

    /// Push an event to a stream. Returns the assigned event index.
    pub async fn push(&self, stream_id: &str, data: NotificationData) -> u64 {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let data_json = match serde_json::to_string(&data) {
            Ok(j) => j,
            Err(e) => {
                error!("Failed to serialize event data: {e}");
                return 0;
            }
        };

        let index = {
            let db = self.db.lock().await;

            // Get next event index for this stream
            let next_index: u64 = db
                .query_row(
                    "SELECT COALESCE(MAX(event_index), -1) + 1 FROM stream_events WHERE stream_id = ?1",
                    rusqlite::params![stream_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            if let Err(e) = db.execute(
                "INSERT INTO stream_events (stream_id, event_index, timestamp, data)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![stream_id, next_index, timestamp, data_json],
            ) {
                error!("Failed to persist event for stream {stream_id}: {e}");
            }

            // Update the stream's updated_at
            let now = chrono::Utc::now().timestamp();
            let _ = db.execute(
                "UPDATE streams SET updated_at = ?1 WHERE stream_id = ?2",
                rusqlite::params![now, stream_id],
            );

            next_index
        };

        // Check if this is a terminal event and update stream status
        let terminal_status = Self::terminal_status(&data);
        if let Some(status) = terminal_status {
            self.update_stream_status(stream_id, status).await;
        }

        // Fire-and-forget webhook delivery
        let settings = self.webhook_settings.clone();
        let sid = stream_id.to_string();
        let data_clone = data;
        tokio::spawn(async move {
            webhook::deliver(&settings, &sid, &data_clone).await;
        });

        index
    }

    /// Get events for a stream, optionally filtering to those with index > after.
    pub async fn get(&self, stream_id: &str, after: Option<u64>) -> Vec<StreamEvent> {
        let db = self.db.lock().await;

        let (query, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(after_idx) = after {
            (
                "SELECT event_index, timestamp, data FROM stream_events
                 WHERE stream_id = ?1 AND event_index > ?2
                 ORDER BY event_index ASC",
                vec![
                    Box::new(stream_id.to_string()),
                    Box::new(after_idx as i64),
                ],
            )
        } else {
            (
                "SELECT event_index, timestamp, data FROM stream_events
                 WHERE stream_id = ?1
                 ORDER BY event_index ASC",
                vec![Box::new(stream_id.to_string())],
            )
        };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = match db.prepare(query) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to prepare get events query: {e}");
                return vec![];
            }
        };

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let index: u64 = row.get(0)?;
            let timestamp: String = row.get(1)?;
            let data_json: String = row.get(2)?;
            Ok((index, timestamp, data_json))
        });

        match rows {
            Ok(rows) => rows
                .filter_map(|r| {
                    let (index, timestamp, data_json) = r.ok()?;
                    let data: NotificationData = serde_json::from_str(&data_json).ok()?;
                    Some(StreamEvent {
                        index,
                        timestamp,
                        data,
                    })
                })
                .collect(),
            Err(e) => {
                error!("Failed to query events for stream {stream_id}: {e}");
                vec![]
            }
        }
    }

    /// Check if a stream exists.
    pub async fn exists(&self, stream_id: &str) -> bool {
        let db = self.db.lock().await;
        db.query_row(
            "SELECT 1 FROM streams WHERE stream_id = ?1",
            rusqlite::params![stream_id],
            |_| Ok(()),
        )
        .is_ok()
    }

    /// Update the status of a stream.
    pub async fn update_stream_status(&self, stream_id: &str, status: StreamStatus) {
        let now = chrono::Utc::now().timestamp();
        let db = self.db.lock().await;
        if let Err(e) = db.execute(
            "UPDATE streams SET status = ?1, updated_at = ?2 WHERE stream_id = ?3",
            rusqlite::params![status.as_str(), now, stream_id],
        ) {
            error!("Failed to update stream status for {stream_id}: {e}");
        }
    }

    /// Get all in-flight streams for startup recovery.
    pub async fn get_in_flight_streams(&self) -> Vec<InFlightStream> {
        let db = self.db.lock().await;
        let mut stmt = match db.prepare(
            "SELECT stream_id, stream_type, metadata FROM streams WHERE status = 'in_flight'",
        ) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to query in-flight streams: {e}");
                return vec![];
            }
        };

        let rows = stmt.query_map([], |row| {
            let stream_id: String = row.get(0)?;
            let stream_type: String = row.get(1)?;
            let metadata_json: Option<String> = row.get(2)?;
            Ok((stream_id, stream_type, metadata_json))
        });

        match rows {
            Ok(rows) => rows
                .filter_map(|r| {
                    let (stream_id, stream_type, metadata_json) = r.ok()?;
                    let metadata = metadata_json
                        .and_then(|j| serde_json::from_str::<StreamMetadata>(&j).ok());
                    Some(InFlightStream {
                        stream_id,
                        stream_type,
                        metadata,
                    })
                })
                .collect(),
            Err(e) => {
                error!("Failed to iterate in-flight streams: {e}");
                vec![]
            }
        }
    }

    /// Determine if a notification represents a terminal state for a stream.
    fn terminal_status(data: &NotificationData) -> Option<StreamStatus> {
        use crate::response::InvoiceStatus;
        match data {
            NotificationData::PaymentStatusUpdate { status } => match status {
                InvoiceStatus::Paid { .. } | InvoiceStatus::UserSuccess { .. } => {
                    Some(StreamStatus::Completed)
                }
                InvoiceStatus::Timeout
                | InvoiceStatus::Error { .. }
                | InvoiceStatus::UserFailed { .. }
                | InvoiceStatus::UserRejected { .. } => Some(StreamStatus::Failed),
                InvoiceStatus::UserApproved => None,
            },
            // Key handshake and recurring close events don't have a terminal state
            // they just keep streaming until the connection ends
            _ => None,
        }
    }
}
