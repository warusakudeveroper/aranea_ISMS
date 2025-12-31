//! EventLogService - Event Recording (Ring Buffer)
//!
//! ## Responsibilities
//!
//! - Store detection events in ring buffer
//! - Persist to database
//! - Provide event queries

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::sync::RwLock;

/// Detection event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionEvent {
    pub event_id: u64,
    pub camera_id: String,
    pub frame_id: String,
    pub captured_at: DateTime<Utc>,
    pub primary_event: String,
    pub severity: i32,
    pub tags: Vec<String>,
    pub unknown_flag: bool,
    pub attributes: Option<serde_json::Value>,
    pub thumbnail_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Ring buffer for events
struct EventRingBuffer {
    events: VecDeque<DetectionEvent>,
    capacity: usize,
    next_id: u64,
}

impl EventRingBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(capacity),
            capacity,
            next_id: 1,
        }
    }

    fn push(&mut self, mut event: DetectionEvent) -> u64 {
        event.event_id = self.next_id;
        self.next_id += 1;

        if self.events.len() >= self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(event);
        self.next_id - 1
    }

    fn get_latest(&self, count: usize) -> Vec<DetectionEvent> {
        self.events.iter().rev().take(count).cloned().collect()
    }

    fn get_by_camera(&self, camera_id: &str, count: usize) -> Vec<DetectionEvent> {
        self.events
            .iter()
            .rev()
            .filter(|e| e.camera_id == camera_id)
            .take(count)
            .cloned()
            .collect()
    }
}

/// EventLogService instance
pub struct EventLogService {
    buffer: RwLock<EventRingBuffer>,
}

impl EventLogService {
    /// Create new EventLogService
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: RwLock::new(EventRingBuffer::new(capacity)),
        }
    }

    /// Add event
    pub async fn add_event(&self, event: DetectionEvent) -> u64 {
        let mut buffer = self.buffer.write().await;
        let id = buffer.push(event);
        tracing::debug!(event_id = id, "Event added to ring buffer");
        id
    }

    /// Get latest events
    pub async fn get_latest(&self, count: usize) -> Vec<DetectionEvent> {
        let buffer = self.buffer.read().await;
        buffer.get_latest(count)
    }

    /// Get events by camera
    pub async fn get_by_camera(&self, camera_id: &str, count: usize) -> Vec<DetectionEvent> {
        let buffer = self.buffer.read().await;
        buffer.get_by_camera(camera_id, count)
    }

    /// Get event count
    pub async fn count(&self) -> usize {
        let buffer = self.buffer.read().await;
        buffer.events.len()
    }
}

impl Default for EventLogService {
    fn default() -> Self {
        Self::new(2000) // Default capacity
    }
}
