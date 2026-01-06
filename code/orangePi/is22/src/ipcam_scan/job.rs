//! Scan job and progress types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::types::{JobStatus, ScanMode};

/// Scan log event type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanLogEventType {
    ArpResponse,
    PortOpen,
    OuiMatch,
    OnvifProbe,
    RtspProbe,
    DeviceClassified,
    CredentialTrial,
    Info,
    Warning,  // ネットワーク到達性などの警告
    Error,
}

/// Scan log entry - ESP32シリアルプリントレベルの詳細ログ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanLogEntry {
    pub timestamp: DateTime<Utc>,
    pub ip_address: String,
    pub event_type: ScanLogEventType,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oui_vendor: Option<String>,
}

impl ScanLogEntry {
    pub fn new(ip: &str, event_type: ScanLogEventType, message: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            ip_address: ip.to_string(),
            event_type,
            message: message.to_string(),
            port: None,
            oui_vendor: None,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_oui(mut self, vendor: &str) -> Self {
        self.oui_vendor = Some(vendor.to_string());
        self
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
    /// リアルタイムスキャンログ
    #[serde(default)]
    pub logs: Vec<ScanLogEntry>,
    /// 現在のスキャンフェーズ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<String>,
    /// 進捗パーセント (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_percent: Option<u8>,
}

/// Job summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub total_ips: u32,
    pub hosts_alive: u32,
    pub cameras_found: u32,
    pub cameras_verified: u32,
}

/// Scan progress tracker
/// Total = subnets × 253 × PHASE_COUNT
/// Progress% = completed_units / total_units × 100
#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    /// Total work units (subnets × 253 × phases)
    pub total_units: usize,
    /// Completed work units
    pub completed_units: usize,
    /// Number of subnets
    pub subnet_count: usize,
    /// IPs per subnet (253 for /24)
    pub ips_per_subnet: usize,
    /// Number of phases
    pub phase_count: usize,
}

impl ScanProgress {
    /// Phase count: Discovery, PortScan, Protocol, Score, Auth
    pub const PHASE_COUNT: usize = 5;
    pub const IPS_PER_SUBNET: usize = 253;

    /// Create new progress tracker
    pub fn new(subnet_count: usize) -> Self {
        let ips_per_subnet = Self::IPS_PER_SUBNET;
        let phase_count = Self::PHASE_COUNT;
        let total_units = subnet_count * ips_per_subnet * phase_count;
        Self {
            total_units,
            completed_units: 0,
            subnet_count,
            ips_per_subnet,
            phase_count,
        }
    }

    /// Complete units for a phase (e.g., all IPs in a subnet for one phase)
    pub fn complete_phase(&mut self, ip_count: usize) {
        self.completed_units += ip_count;
    }

    /// Complete a single unit
    pub fn complete_unit(&mut self) {
        self.completed_units += 1;
    }

    /// Complete N units
    pub fn complete_units(&mut self, count: usize) {
        self.completed_units += count;
    }

    /// Get progress percentage (0-100)
    pub fn percent(&self) -> u8 {
        if self.total_units == 0 {
            return 0;
        }
        let pct = (self.completed_units as f64 / self.total_units as f64) * 100.0;
        (pct.min(100.0)) as u8
    }
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

