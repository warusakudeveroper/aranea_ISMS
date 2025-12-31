//! AdmissionController types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stream quality
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StreamQuality {
    Sub,
    Main,
}

impl Default for StreamQuality {
    fn default() -> Self {
        Self::Sub
    }
}

/// Lease request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRequest {
    pub user_id: String,
    pub camera_id: String,
    #[serde(default)]
    pub quality: StreamQuality,
}

/// Modal lease
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalLease {
    pub lease_id: Uuid,
    pub user_id: String,
    pub camera_id: String,
    pub quality: StreamQuality,
    pub cost: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

/// Admission result
#[derive(Debug, Clone)]
pub enum AdmissionResult {
    Admitted(u32), // cost
    Rejected(RejectReason),
}

/// Rejection reason
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RejectReason {
    SystemOverload,
    UserAlreadyHasModal,
    OverCapacity,
}

/// Heartbeat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub ok: bool,
    pub remaining_sec: i64,
}

/// Lease response (for API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseResponse {
    pub lease_id: Uuid,
    pub allowed_quality: StreamQuality,
    pub expires_at: DateTime<Utc>,
    pub stream_url: String,
}

/// Admission status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionStatus {
    pub healthy: bool,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub active_modals: i32,
    pub modal_budget_remaining: i32,
    pub suggest_active: bool,
}
