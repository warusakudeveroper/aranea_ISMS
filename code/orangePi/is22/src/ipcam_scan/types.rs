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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
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

/// Device status for registration flow
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type, Default)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum DeviceStatus {
    #[default]
    Discovered,
    Verifying,
    Verified,
    Rejected,
    Approved,
}

/// Connection status - ユーザーに「なぜ繋がらないか」を伝える
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    #[default]
    NotTested,           // 未試行
    Success,             // 接続成功（ONVIF/RTSP応答あり）
    AuthRequired,        // 認証が必要（401/認証エラー）
    AuthFailed,          // 認証失敗（クレデンシャル不正）
    Timeout,             // タイムアウト
    Refused,             // 接続拒否
    PortOpenOnly,        // ポートは開いているが応答なし
    Unknown,             // 不明なエラー
}

/// Detection reason - なぜカメラと判断したか
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetectionReason {
    /// OUIベンダー一致（TP-Link, Axis等）
    pub oui_match: Option<String>,
    /// 開いているカメラ関連ポート
    pub camera_ports: Vec<u16>,
    /// ONVIFプローブ結果
    pub onvif_status: ConnectionStatus,
    /// RTSPプローブ結果
    pub rtsp_status: ConnectionStatus,
    /// 推定デバイスタイプ
    pub device_type: DeviceType,
    /// ユーザー向けメッセージ
    pub user_message: String,
    /// 推奨アクション
    pub suggested_action: SuggestedAction,
}

/// 検出されたデバイスタイプ
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    #[default]
    Unknown,
    CameraConfirmed,     // カメラ確定（ONVIF/RTSP応答あり）
    CameraLikely,        // カメラの可能性高（ポート+OUI一致）
    CameraPossible,      // カメラかもしれない（ポートのみ）
    NvrLikely,           // NVRの可能性（複数ストリームポート等）
    NetworkDevice,       // ネットワーク機器（ルーター/スイッチ）
    OtherDevice,         // その他のデバイス
}

/// 推奨アクション
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedAction {
    #[default]
    None,
    SetCredentials,      // クレデンシャル設定が必要
    RetryWithAuth,       // 認証情報で再試行
    CheckNetwork,        // ネットワーク確認
    ManualCheck,         // 手動確認推奨
    Ignore,              // 無視推奨（カメラではない）
}

/// Credential trial status - スキャン時のクレデンシャル試行結果
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum CredentialStatus {
    #[default]
    NotTried,    // 試行なし（クレデンシャル未設定）
    Success,     // 成功（モデル情報取得済み）
    Failed,      // 失敗（全クレデンシャル不一致）
}

/// Trial credential for subnet (stored as JSON in DB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialCredential {
    pub username: String,
    pub password: String,
    pub priority: u8,  // 1-10, 試行順序
}

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
    pub status: DeviceStatus,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub family: CameraFamily,
    pub confidence: u8,
    pub rtsp_uri: Option<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    /// 検出理由とユーザーフィードバック
    #[serde(default)]
    pub detection: DetectionReason,
    /// クレデンシャル試行結果
    #[serde(default)]
    pub credential_status: CredentialStatus,
    /// 成功したクレデンシャルのユーザー名
    pub credential_username: Option<String>,
    /// 成功したクレデンシャルのパスワード
    pub credential_password: Option<String>,
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
    pub status: Option<DeviceStatus>,
}

/// Approve device request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveDeviceRequest {
    pub display_name: String,
    pub location: String,
    pub fid: String,
    /// RTSP credentials (required for go2rtc registration)
    pub credentials: Option<ApproveCredentials>,
}

/// Credentials for camera approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveCredentials {
    pub username: String,
    pub password: String,
}

/// Approve device response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveDeviceResponse {
    pub camera_id: String,
    pub lacis_id: String,
    pub ip_address: String,
    /// RTSP URL with credentials for go2rtc registration
    pub rtsp_url: Option<String>,
}

/// Batch verify request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVerifyRequest {
    pub fid: String,
    pub device_ids: Vec<Uuid>,
    pub use_facility_credentials: bool,
}

/// Batch verify result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVerifyResult {
    pub device_id: Uuid,
    pub verified: bool,
    pub rtsp_uri: Option<String>,
    pub error: Option<String>,
}

/// Facility credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacilityCredentials {
    pub fid: String,
    pub facility_name: String,
    pub username: String,
    // Note: password is not serialized for security
}

/// Create facility credentials request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFacilityCredentialsRequest {
    pub fid: String,
    pub facility_name: String,
    pub username: String,
    pub password: String,
}
