//! IpcamScan types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Scan mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScanMode {
    #[default]
    Discovery,
    Deep,
}

/// Job status
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Partial,
    Success,
    Failed,
    Canceled,
}

/// Camera family
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CameraFamily {
    Tapo,
    Vigi,
    Nest,
    Axis,
    Hikvision,
    Dahua,
    Other,
    Unknown,
}

impl Default for CameraFamily {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Scan job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJob {
    pub job_id: Uuid,
    pub targets: Vec<String>,
    pub mode: ScanMode,
    pub ports: Vec<u16>,
    pub timeout_ms: u32,
    pub concurrency: u8,
    pub status: JobStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub summary: Option<JobSummary>,
    pub created_at: DateTime<Utc>,
}

/// Job summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub total_ips: u32,
    pub hosts_alive: u32,
    pub cameras_found: u32,
    pub cameras_verified: u32,
}

/// Scan job request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJobRequest {
    pub targets: Vec<String>,
    pub mode: Option<ScanMode>,
    pub ports: Option<Vec<u16>>,
    pub timeout_ms: Option<u32>,
    pub concurrency: Option<u8>,
}

/// Scanned device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedDevice {
    pub device_id: Uuid,
    pub ip: String,
    pub subnet: String,
    pub mac: Option<String>,
    pub oui_vendor: Option<String>,
    pub hostnames: Vec<String>,
    pub open_ports: Vec<PortStatus>,
    pub score: u32,
    pub verified: bool,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub family: CameraFamily,
    pub confidence: u8,
    pub rtsp_uri: Option<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

/// Port status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortStatus {
    pub port: u16,
    pub status: PortState,
}

/// Port state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortState {
    Open,
    Closed,
    Filtered,
}

/// Device filter
#[derive(Debug, Clone, Default)]
pub struct DeviceFilter {
    pub subnet: Option<String>,
    pub family: Option<CameraFamily>,
    pub verified: Option<bool>,
}
