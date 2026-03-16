use std::sync::Arc;

use dashmap::DashMap;

use crate::config::WebhookSettings;
use crate::response::{NotificationData, StreamEvent};
use crate::webhook;

/// In-memory store for stream events. Events are appended per stream_id and can be
/// polled by clients via `GET /events/{stream_id}?after={index}`.
///
/// When a webhook URL is configured, events are also delivered via HTTP POST.
#[derive(Clone)]
pub struct EventStore {
    inner: Arc<DashMap<String, Vec<StreamEvent>>>,
    webhook_settings: WebhookSettings,
}

impl EventStore {
    pub fn new(webhook_settings: WebhookSettings) -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            webhook_settings,
        }
    }

    /// Push an event to a stream. Returns the assigned event index.
    pub async fn push(&self, stream_id: &str, data: NotificationData) -> u64 {
        let index = {
            let mut entry = self.inner.entry(stream_id.to_string()).or_default();
            let idx = entry.len() as u64;
            entry.push(StreamEvent {
                index: idx,
                timestamp: chrono::Utc::now().to_rfc3339(),
                data: data.clone(),
            });
            idx
        };

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
    pub fn get(&self, stream_id: &str, after: Option<u64>) -> Vec<StreamEvent> {
        match self.inner.get(stream_id) {
            Some(events) => {
                let after_idx = after.unwrap_or(0);
                if after.is_some() {
                    events
                        .iter()
                        .filter(|e| e.index > after_idx)
                        .cloned()
                        .collect()
                } else {
                    events.clone()
                }
            }
            None => vec![],
        }
    }

    /// Check if a stream exists.
    pub fn exists(&self, stream_id: &str) -> bool {
        self.inner.contains_key(stream_id)
    }

    /// Create an empty stream entry (reserves the stream_id).
    pub fn create_stream(&self, stream_id: &str) {
        self.inner.entry(stream_id.to_string()).or_default();
    }
}
