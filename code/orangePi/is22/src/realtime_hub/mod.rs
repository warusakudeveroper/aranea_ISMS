//! RealtimeHub - WebSocket/SSE Distribution
//!
//! ## Responsibilities
//!
//! - WebSocket connection management
//! - Event broadcasting (detection events to AI Event Log)
//! - Snapshot update notifications (triggers CameraGrid to fetch new image)
//! - Suggest state distribution
//!
//! Note: Only snapshot update NOTIFICATIONS are sent via WebSocket (camera_id + timestamp).
//! Actual image data is fetched via HTTP GET /api/snapshots/{camera_id}/latest.jpg

use crate::suggest_engine::GlobalSuggestState;
use futures::stream::SplitSink;
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Hub message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum HubMessage {
    SuggestUpdate(GlobalSuggestState),
    EventLog(EventLogMessage),
    SystemStatus(SystemStatusMessage),
    CameraStatus(CameraStatusMessage),
    /// Notification that a camera's snapshot has been updated
    /// Client should fetch new image via HTTP GET
    SnapshotUpdated(SnapshotUpdatedMessage),
}

/// Event log message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLogMessage {
    pub event_id: u64,
    pub camera_id: String,
    pub primary_event: String,
    pub severity: i32,
    pub timestamp: String,
}

/// System status message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusMessage {
    pub healthy: bool,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub active_streams: i32,
}

/// Camera status message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraStatusMessage {
    pub camera_id: String,
    pub online: bool,
    pub last_frame_at: Option<String>,
}

/// Snapshot updated notification
/// Sent when PollingOrchestrator successfully captures a new frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotUpdatedMessage {
    pub camera_id: String,
    pub timestamp: String,
    /// Primary detection event (if IS21 inference was run)
    pub primary_event: Option<String>,
    /// Detection severity (0-3)
    pub severity: Option<i32>,
}

/// Client connection
struct ClientConnection {
    id: Uuid,
    user_id: String,
    tx: mpsc::UnboundedSender<String>,
}

/// RealtimeHub instance
pub struct RealtimeHub {
    connections: RwLock<HashMap<Uuid, ClientConnection>>,
    connection_count: AtomicU64,
}

impl RealtimeHub {
    /// Create new RealtimeHub
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            connection_count: AtomicU64::new(0),
        }
    }

    /// Register a new client
    pub async fn register(&self, user_id: String) -> (Uuid, mpsc::UnboundedReceiver<String>) {
        let id = Uuid::new_v4();
        let (tx, rx) = mpsc::unbounded_channel();

        let conn = ClientConnection { id, user_id, tx };

        {
            let mut connections = self.connections.write().await;
            connections.insert(id, conn);
        }

        self.connection_count.fetch_add(1, Ordering::Relaxed);

        tracing::info!(connection_id = %id, "Client connected");

        (id, rx)
    }

    /// Unregister a client
    pub async fn unregister(&self, id: &Uuid) {
        let mut connections = self.connections.write().await;
        if connections.remove(id).is_some() {
            self.connection_count.fetch_sub(1, Ordering::Relaxed);
            tracing::info!(connection_id = %id, "Client disconnected");
        }
    }

    /// Broadcast message to all clients
    pub async fn broadcast(&self, message: HubMessage) {
        let json = match serde_json::to_string(&message) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize message");
                return;
            }
        };

        let connections = self.connections.read().await;
        for conn in connections.values() {
            if let Err(e) = conn.tx.send(json.clone()) {
                tracing::warn!(connection_id = %conn.id, error = %e, "Failed to send message");
            }
        }
    }

    /// Send message to specific user
    pub async fn send_to_user(&self, user_id: &str, message: HubMessage) {
        let json = match serde_json::to_string(&message) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize message");
                return;
            }
        };

        let connections = self.connections.read().await;
        for conn in connections.values() {
            if conn.user_id == user_id {
                if let Err(e) = conn.tx.send(json.clone()) {
                    tracing::warn!(connection_id = %conn.id, error = %e, "Failed to send message");
                }
            }
        }
    }

    /// Get connection count
    pub fn connection_count(&self) -> u64 {
        self.connection_count.load(Ordering::Relaxed)
    }
}

impl Default for RealtimeHub {
    fn default() -> Self {
        Self::new()
    }
}
