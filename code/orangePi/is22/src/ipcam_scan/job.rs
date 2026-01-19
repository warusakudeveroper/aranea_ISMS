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
    /// Brute Force Mode: OFFの場合、カメラの可能性が低いデバイスはクレデンシャル試行をスキップ
    #[serde(default)]
    pub brute_force: bool,
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

/// Scan progress tracker (legacy)
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

/// Dynamic Progress Calculator (#82 T2-7)
///
/// Adapts progress calculation based on actual discovery results:
/// - Phase 1 (Discovery): Fixed 20% of total (subnet-based)
/// - Phase 2-5: Dynamically weighted based on discovered hosts
///
/// This provides more accurate progress for scans where:
/// - Few hosts are found (phases complete faster)
/// - Many hosts are found (phases take longer)
#[derive(Debug, Clone)]
pub struct DynamicProgressCalculator {
    /// Number of target subnets
    subnet_count: usize,
    /// Phase 1 progress (0.0 - 1.0)
    phase1_progress: f64,
    /// Discovered hosts after Phase 1
    discovered_hosts: usize,
    /// Phase 2+ progress (0.0 - 1.0 each)
    phase2_progress: f64,  // Port scan
    phase3_progress: f64,  // Protocol probe
    phase4_progress: f64,  // Scoring
    phase5_progress: f64,  // Auth trial
    /// Camera candidates after Phase 4
    camera_candidates: usize,
}

impl DynamicProgressCalculator {
    /// Phase weights (total = 100%)
    /// Phase 1 (Discovery) = 20% - Fixed, subnet-based
    /// Phase 2 (Port Scan) = 20% - Per discovered host
    /// Phase 3 (Protocol) = 25% - Per host with open ports
    /// Phase 4 (Scoring) = 10% - Quick
    /// Phase 5 (Auth) = 25% - Per camera candidate
    const PHASE1_WEIGHT: f64 = 0.20;
    const PHASE2_WEIGHT: f64 = 0.20;
    const PHASE3_WEIGHT: f64 = 0.25;
    const PHASE4_WEIGHT: f64 = 0.10;
    const PHASE5_WEIGHT: f64 = 0.25;

    /// Create new dynamic progress calculator
    pub fn new(subnet_count: usize) -> Self {
        Self {
            subnet_count,
            phase1_progress: 0.0,
            discovered_hosts: 0,
            phase2_progress: 0.0,
            phase3_progress: 0.0,
            phase4_progress: 0.0,
            phase5_progress: 0.0,
            camera_candidates: 0,
        }
    }

    /// Update Phase 1 progress (Discovery)
    /// Called per-subnet as ARP/TCP discovery completes
    pub fn update_phase1(&mut self, completed_subnets: usize) {
        if self.subnet_count > 0 {
            self.phase1_progress = (completed_subnets as f64 / self.subnet_count as f64).min(1.0);
        }
    }

    /// Complete Phase 1 and set discovered host count
    /// This adjusts the weighting for subsequent phases
    pub fn complete_phase1(&mut self, discovered_hosts: usize) {
        self.phase1_progress = 1.0;
        self.discovered_hosts = discovered_hosts;
    }

    /// Update Phase 2 progress (Port Scan)
    pub fn update_phase2(&mut self, completed_hosts: usize) {
        if self.discovered_hosts > 0 {
            self.phase2_progress = (completed_hosts as f64 / self.discovered_hosts as f64).min(1.0);
        } else {
            self.phase2_progress = 1.0; // No hosts = phase complete
        }
    }

    /// Complete Phase 2
    pub fn complete_phase2(&mut self) {
        self.phase2_progress = 1.0;
    }

    /// Update Phase 3 progress (Protocol Probe)
    pub fn update_phase3(&mut self, completed_hosts: usize, hosts_with_ports: usize) {
        if hosts_with_ports > 0 {
            self.phase3_progress = (completed_hosts as f64 / hosts_with_ports as f64).min(1.0);
        } else {
            self.phase3_progress = 1.0;
        }
    }

    /// Complete Phase 3
    pub fn complete_phase3(&mut self) {
        self.phase3_progress = 1.0;
    }

    /// Complete Phase 4 and set camera candidate count
    pub fn complete_phase4(&mut self, camera_candidates: usize) {
        self.phase4_progress = 1.0;
        self.camera_candidates = camera_candidates;
    }

    /// Update Phase 5 progress (Auth Trial)
    pub fn update_phase5(&mut self, completed_devices: usize) {
        if self.camera_candidates > 0 {
            self.phase5_progress = (completed_devices as f64 / self.camera_candidates as f64).min(1.0);
        } else {
            self.phase5_progress = 1.0;
        }
    }

    /// Complete Phase 5
    pub fn complete_phase5(&mut self) {
        self.phase5_progress = 1.0;
    }

    /// Get current progress percentage (0-100)
    pub fn percent(&self) -> u8 {
        let total =
            (self.phase1_progress * Self::PHASE1_WEIGHT) +
            (self.phase2_progress * Self::PHASE2_WEIGHT) +
            (self.phase3_progress * Self::PHASE3_WEIGHT) +
            (self.phase4_progress * Self::PHASE4_WEIGHT) +
            (self.phase5_progress * Self::PHASE5_WEIGHT);

        ((total * 100.0).round().min(100.0)) as u8
    }

    /// Get current phase name
    pub fn current_phase(&self) -> &'static str {
        if self.phase1_progress < 1.0 {
            "Stage 1-2: ホスト検出"
        } else if self.phase2_progress < 1.0 {
            "Stage 3: ポートスキャン"
        } else if self.phase3_progress < 1.0 {
            "Stage 4: プロトコル検査"
        } else if self.phase4_progress < 1.0 {
            "Stage 5: スコアリング"
        } else if self.phase5_progress < 1.0 {
            "Stage 6-7: 認証試行"
        } else {
            "完了"
        }
    }

    /// Get detailed progress info for logging
    pub fn debug_info(&self) -> String {
        format!(
            "P1:{:.0}% P2:{:.0}% P3:{:.0}% P4:{:.0}% P5:{:.0}% | hosts:{} cams:{}",
            self.phase1_progress * 100.0,
            self.phase2_progress * 100.0,
            self.phase3_progress * 100.0,
            self.phase4_progress * 100.0,
            self.phase5_progress * 100.0,
            self.discovered_hosts,
            self.camera_candidates,
        )
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
    /// Brute Force Mode: OFFの場合、カメラの可能性が低いデバイスはクレデンシャル試行をスキップ
    #[serde(default)]
    pub brute_force: bool,
}

