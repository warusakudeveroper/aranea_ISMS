//! ParaclateClient Type Definitions
//!
//! Phase 4: ParaclateClient (Issue #117)
//! DD03_ParaclateClient.md準拠
//!
//! ## 概要
//! Paraclate APPとの通信に必要な型定義
//! - 設定同期構造体
//! - 送信キュー構造体
//! - リクエスト/レスポンス型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================
// Connection Status
// ============================================================

/// 接続状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Error,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connected => write!(f, "connected"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Error => write!(f, "error"),
        }
    }
}

impl From<&str> for ConnectionStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "connected" => Self::Connected,
            "error" => Self::Error,
            _ => Self::Disconnected,
        }
    }
}

// ============================================================
// Payload Type
// ============================================================

/// ペイロード種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadType {
    Summary,
    GrandSummary,
    Event,
    Emergency,
}

impl std::fmt::Display for PayloadType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Summary => write!(f, "summary"),
            Self::GrandSummary => write!(f, "grand_summary"),
            Self::Event => write!(f, "event"),
            Self::Emergency => write!(f, "emergency"),
        }
    }
}

impl From<&str> for PayloadType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "grand_summary" => Self::GrandSummary,
            "event" => Self::Event,
            "emergency" => Self::Emergency,
            _ => Self::Summary,
        }
    }
}

// ============================================================
// Queue Status
// ============================================================

/// 送信キュー状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    Pending,
    Sending,
    Sent,
    Failed,
    Skipped,
}

impl Default for QueueStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Sending => write!(f, "sending"),
            Self::Sent => write!(f, "sent"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

impl From<&str> for QueueStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sending" => Self::Sending,
            "sent" => Self::Sent,
            "failed" => Self::Failed,
            "skipped" => Self::Skipped,
            _ => Self::Pending,
        }
    }
}

// ============================================================
// Connection Log Event Type
// ============================================================

/// 接続ログイベント種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionEventType {
    Connect,
    Disconnect,
    Sync,
    Error,
    Retry,
}

impl std::fmt::Display for ConnectionEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect => write!(f, "connect"),
            Self::Disconnect => write!(f, "disconnect"),
            Self::Sync => write!(f, "sync"),
            Self::Error => write!(f, "error"),
            Self::Retry => write!(f, "retry"),
        }
    }
}

// ============================================================
// Config Structures
// ============================================================

/// Paraclate設定（DB構造体）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParaclateConfig {
    pub config_id: u32,
    pub tid: String,
    pub fid: String,
    pub endpoint: String,
    pub report_interval_minutes: i32,
    pub grand_summary_times: Vec<String>,
    pub retention_days: i32,
    pub attunement: serde_json::Value,
    pub sync_source_timestamp: Option<DateTime<Utc>>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub connection_status: ConnectionStatus,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Paraclate設定挿入用
#[derive(Debug, Clone)]
pub struct ParaclateConfigInsert {
    pub tid: String,
    pub fid: String,
    pub endpoint: String,
    pub report_interval_minutes: Option<i32>,
    pub grand_summary_times: Option<Vec<String>>,
    pub retention_days: Option<i32>,
    pub attunement: Option<serde_json::Value>,
}

/// Paraclate設定更新用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParaclateConfigUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_interval_minutes: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grand_summary_times: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_days: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attunement: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_source_timestamp: Option<DateTime<Utc>>,
}

// ============================================================
// Send Queue Structures
// ============================================================

/// 送信キュー項目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendQueueItem {
    pub queue_id: u64,
    pub tid: String,
    pub fid: String,
    pub payload_type: PayloadType,
    pub payload: serde_json::Value,
    pub reference_id: Option<u64>,
    pub status: QueueStatus,
    pub retry_count: i32,
    pub max_retries: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub http_status_code: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

/// 送信キュー挿入用
#[derive(Debug, Clone)]
pub struct SendQueueInsert {
    pub tid: String,
    pub fid: String,
    pub payload_type: PayloadType,
    pub payload: serde_json::Value,
    pub reference_id: Option<u64>,
    pub max_retries: Option<i32>,
}

/// 送信キュー更新用（リトライ時）
#[derive(Debug, Clone)]
pub struct SendQueueUpdate {
    pub status: QueueStatus,
    pub retry_count: Option<i32>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub http_status_code: Option<i32>,
    pub sent_at: Option<DateTime<Utc>>,
}

// ============================================================
// Connection Log Structures
// ============================================================

/// 接続ログ項目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionLog {
    pub log_id: u64,
    pub tid: String,
    pub fid: String,
    pub event_type: ConnectionEventType,
    pub event_detail: Option<String>,
    pub error_code: Option<String>,
    pub http_status_code: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// 接続ログ挿入用
#[derive(Debug, Clone)]
pub struct ConnectionLogInsert {
    pub tid: String,
    pub fid: String,
    pub event_type: ConnectionEventType,
    pub event_detail: Option<String>,
    pub error_code: Option<String>,
    pub http_status_code: Option<i32>,
}

// ============================================================
// LacisOath Authentication Header
// ============================================================

/// LacisOath認証ヘッダ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LacisOath {
    /// is22のLacisID
    pub lacis_id: String,
    /// テナントID
    pub tid: String,
    /// Client Identification Code
    pub cic: String,
    /// 越境アクセス用Blessing（通常はNone）
    pub blessing: Option<String>,
}

impl LacisOath {
    /// HTTPヘッダを生成
    ///
    /// mobes2.0 Paraclate API認証形式:
    /// Authorization: LacisOath <base64-encoded-json>
    ///
    /// JSON構造:
    /// {
    ///   "lacisId": "...",
    ///   "tid": "...",
    ///   "cic": "...",
    ///   "timestamp": "<ISO8601>"
    /// }
    pub fn to_headers(&self) -> Vec<(String, String)> {
        use base64::Engine;

        // 認証ペイロードを構築
        let auth_payload = serde_json::json!({
            "lacisId": self.lacis_id,
            "tid": self.tid,
            "cic": self.cic,
            "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        });

        // Base64エンコード
        let json_str = serde_json::to_string(&auth_payload).unwrap_or_default();
        let encoded = base64::engine::general_purpose::STANDARD.encode(json_str.as_bytes());

        let mut headers = vec![
            ("Authorization".to_string(), format!("LacisOath {}", encoded)),
        ];

        // blessingがある場合は追加ヘッダとして付与（越境アクセス用）
        if let Some(blessing) = &self.blessing {
            headers.push(("X-Lacis-Blessing".to_string(), blessing.clone()));
        }

        headers
    }

    /// 旧形式ヘッダを生成（後方互換性用）
    #[allow(dead_code)]
    pub fn to_legacy_headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![
            ("X-Lacis-ID".to_string(), self.lacis_id.clone()),
            ("X-Lacis-TID".to_string(), self.tid.clone()),
            ("X-Lacis-CIC".to_string(), self.cic.clone()),
        ];
        if let Some(blessing) = &self.blessing {
            headers.push(("X-Lacis-Blessing".to_string(), blessing.clone()));
        }
        headers
    }
}

// ============================================================
// API Request/Response Structures
// ============================================================

/// 接続リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRequest {
    pub endpoint: String,
    pub fid: String,
}

/// 接続レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResponse {
    pub connected: bool,
    pub endpoint: String,
    pub config_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// ステータスレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub connected: bool,
    pub connection_status: ConnectionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub queue_stats: QueueStats,
}

/// キュー統計
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QueueStats {
    pub pending: u64,
    pub sending: u64,
    pub failed: u64,
    pub sent_today: u64,
}

/// 設定レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResponse {
    pub config_id: u32,
    pub tid: String,
    pub fid: String,
    pub endpoint: String,
    pub report_interval_minutes: i32,
    pub grand_summary_times: Vec<String>,
    pub retention_days: i32,
    pub attunement: serde_json::Value,
    pub sync_source_timestamp: Option<DateTime<Utc>>,
    pub last_sync_at: Option<DateTime<Utc>>,
}

impl From<ParaclateConfig> for ConfigResponse {
    fn from(config: ParaclateConfig) -> Self {
        Self {
            config_id: config.config_id,
            tid: config.tid,
            fid: config.fid,
            endpoint: config.endpoint,
            report_interval_minutes: config.report_interval_minutes,
            grand_summary_times: config.grand_summary_times,
            retention_days: config.retention_days,
            attunement: config.attunement,
            sync_source_timestamp: config.sync_source_timestamp,
            last_sync_at: config.last_sync_at,
        }
    }
}

/// 設定更新リクエスト（API用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfigRequest {
    pub fid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_interval_minutes: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grand_summary_times: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_days: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attunement: Option<serde_json::Value>,
}

/// キュー一覧レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueListResponse {
    pub items: Vec<QueueItemResponse>,
    pub total: u64,
    pub stats: QueueStats,
}

/// キュー項目レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItemResponse {
    pub queue_id: u64,
    pub payload_type: PayloadType,
    pub reference_id: Option<u64>,
    pub status: QueueStatus,
    pub retry_count: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

impl From<SendQueueItem> for QueueItemResponse {
    fn from(item: SendQueueItem) -> Self {
        Self {
            queue_id: item.queue_id,
            payload_type: item.payload_type,
            reference_id: item.reference_id,
            status: item.status,
            retry_count: item.retry_count,
            next_retry_at: item.next_retry_at,
            last_error: item.last_error,
            created_at: item.created_at,
            sent_at: item.sent_at,
        }
    }
}

// ============================================================
// Snapshot / Event Structures (T4-3: LacisFiles連携)
// ============================================================

/// カメラ不調種別
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraMalfunctionType {
    /// カメラオフライン
    Offline,
    /// ストリームエラー
    StreamError,
    /// 高レイテンシ
    HighLatency,
    /// 低FPS
    LowFps,
    /// フレーム取得不可
    NoFrames,
}

impl std::fmt::Display for CameraMalfunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Offline => write!(f, "offline"),
            Self::StreamError => write!(f, "stream_error"),
            Self::HighLatency => write!(f, "high_latency"),
            Self::LowFps => write!(f, "low_fps"),
            Self::NoFrames => write!(f, "no_frames"),
        }
    }
}

/// Event送信ペイロード
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPayload {
    /// テナントID
    pub tid: String,
    /// 施設ID
    pub fid: String,
    /// 検出ログID
    pub log_id: u64,
    /// カメラのLacisID
    pub camera_lacis_id: String,
    /// 検出時刻
    pub captured_at: DateTime<Utc>,
    /// プライマリイベント (person, vehicle, etc.)
    pub primary_event: String,
    /// 重要度 (0-3)
    pub severity: i32,
    /// 信頼度 (0.0-1.0)
    pub confidence: f32,
    /// タグ一覧
    pub tags: Vec<String>,
    /// 検出詳細（JSONシリアライズ済み）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// スナップショット画像（Base64エンコード）
    /// mobes2.0 LacisFiles連携: JSON bodyに含めて送信
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_base64: Option<String>,
    /// スナップショットのMIMEタイプ（例: image/jpeg）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_mime_type: Option<String>,
    /// カメラ不調種別（不調報告時のみ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub malfunction_type: Option<CameraMalfunctionType>,
    /// 不調詳細情報
    #[serde(skip_serializing_if = "Option::is_none")]
    pub malfunction_details: Option<serde_json::Value>,
}

/// Event送信レスポンス (Paraclate APPからの応答)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventResponse {
    /// 成功フラグ
    pub ok: bool,
    /// mobes側で生成されたイベントID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// snapshotの保存結果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<SnapshotRef>,
    /// エラーメッセージ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Snapshot参照情報 (LacisFiles)
///
/// ddreview_2 P1-2確定:
/// - 署名URL禁止、storagePath（恒久参照子）を使用
/// - 形式: `lacisFiles/{tid}/{fileId}.{ext}`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRef {
    /// テナントID
    pub tid: String,
    /// ファイルID
    pub file_id: String,
    /// 保存パス (lacisFiles/{tid}/{fileId}.{ext})
    pub storage_path: String,
}

/// Event送信結果（クライアント用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSendResult {
    /// 成功フラグ
    pub success: bool,
    /// 送信キューID
    pub queue_id: u64,
    /// mobes側イベントID（送信成功時）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Snapshot storage path（送信成功時）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_url: Option<String>,
    /// エラーメッセージ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================
// Paraclate APP API Structures (mobes2.0側)
// ============================================================

/// mobes2.0からの設定同期レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobesSyncResponse {
    pub success: bool,
    pub config: Option<MobesConfig>,
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// mobes2.0設定構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobesConfig {
    pub report_interval_minutes: i32,
    pub grand_summary_times: Vec<String>,
    pub retention_days: i32,
    pub attunement: MobesAttunement,
}

/// mobes2.0アチューンメント設定
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MobesAttunement {
    #[serde(default)]
    pub auto_tuning_enabled: bool,
    #[serde(default = "default_tuning_frequency")]
    pub tuning_frequency: String,
    #[serde(default = "default_tuning_aggressiveness")]
    pub tuning_aggressiveness: i32,
}

fn default_tuning_frequency() -> String {
    "weekly".to_string()
}

fn default_tuning_aggressiveness() -> i32 {
    50
}

// ============================================================
// Error Types
// ============================================================

/// ParaclateClient エラー
#[derive(Debug, Clone)]
pub enum ParaclateError {
    Database(String),
    Http(String),
    Config(String),
    Queue(String),
    Offline(String),
    Auth(String),
    /// Sync failed (T4-7)
    Sync(String),
    /// Parse/decode error (T4-7)
    Parse(String),
    /// FID validation error (Issue #119)
    FidValidation(String),
}

impl std::fmt::Display for ParaclateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "Database error: {}", e),
            Self::Http(e) => write!(f, "HTTP error: {}", e),
            Self::Config(e) => write!(f, "Config error: {}", e),
            Self::Queue(e) => write!(f, "Queue error: {}", e),
            Self::Offline(e) => write!(f, "Offline: {}", e),
            Self::Auth(e) => write!(f, "Auth error: {}", e),
            Self::Sync(e) => write!(f, "Sync error: {}", e),
            Self::Parse(e) => write!(f, "Parse error: {}", e),
            Self::FidValidation(e) => write!(f, "FID validation error: {}", e),
        }
    }
}

impl std::error::Error for ParaclateError {}

impl From<ParaclateError> for crate::Error {
    fn from(e: ParaclateError) -> Self {
        crate::Error::Paraclate(e.to_string())
    }
}

// ============================================================
// Constants
// ============================================================

/// デフォルト設定値
pub mod defaults {
    pub const REPORT_INTERVAL_MINUTES: i32 = 60;
    pub const RETENTION_DAYS: i32 = 60;
    pub const MAX_RETRIES: i32 = 5;
    pub const BASE_RETRY_DELAY_SECS: u64 = 30;
    pub const MAX_RETRY_DELAY_SECS: u64 = 3600;

    pub fn grand_summary_times() -> Vec<String> {
        vec!["09:00".to_string(), "17:00".to_string(), "21:00".to_string()]
    }
}

/// 指数バックオフでリトライ遅延を計算
pub fn calculate_retry_delay(retry_count: i32) -> std::time::Duration {
    let base = defaults::BASE_RETRY_DELAY_SECS;
    let max = defaults::MAX_RETRY_DELAY_SECS;
    let delay = base * 2u64.pow(retry_count as u32);
    std::time::Duration::from_secs(delay.min(max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_status_display() {
        assert_eq!(ConnectionStatus::Connected.to_string(), "connected");
        assert_eq!(ConnectionStatus::Disconnected.to_string(), "disconnected");
        assert_eq!(ConnectionStatus::Error.to_string(), "error");
    }

    #[test]
    fn test_payload_type_display() {
        assert_eq!(PayloadType::Summary.to_string(), "summary");
        assert_eq!(PayloadType::GrandSummary.to_string(), "grand_summary");
        assert_eq!(PayloadType::Event.to_string(), "event");
        assert_eq!(PayloadType::Emergency.to_string(), "emergency");
    }

    #[test]
    fn test_queue_status_from_str() {
        assert_eq!(QueueStatus::from("pending"), QueueStatus::Pending);
        assert_eq!(QueueStatus::from("SENT"), QueueStatus::Sent);
        assert_eq!(QueueStatus::from("unknown"), QueueStatus::Pending);
    }

    #[test]
    fn test_lacis_oath_to_headers() {
        let oath = LacisOath {
            lacis_id: "3022AABBCCDDEEFF0000".to_string(),
            tid: "T123".to_string(),
            cic: "123456".to_string(),
            blessing: None,
        };
        let headers = oath.to_headers();
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0], ("X-Lacis-ID".to_string(), "3022AABBCCDDEEFF0000".to_string()));
    }

    #[test]
    fn test_lacis_oath_with_blessing() {
        let oath = LacisOath {
            lacis_id: "3022AABBCCDDEEFF0000".to_string(),
            tid: "T123".to_string(),
            cic: "123456".to_string(),
            blessing: Some("BLESSING_TOKEN".to_string()),
        };
        let headers = oath.to_headers();
        assert_eq!(headers.len(), 4);
    }

    #[test]
    fn test_calculate_retry_delay() {
        // 0回目: 30s
        assert_eq!(calculate_retry_delay(0), std::time::Duration::from_secs(30));
        // 1回目: 60s
        assert_eq!(calculate_retry_delay(1), std::time::Duration::from_secs(60));
        // 2回目: 120s
        assert_eq!(calculate_retry_delay(2), std::time::Duration::from_secs(120));
        // 10回目: max 3600s
        assert_eq!(calculate_retry_delay(10), std::time::Duration::from_secs(3600));
    }

    #[test]
    fn test_default_values() {
        assert_eq!(defaults::REPORT_INTERVAL_MINUTES, 60);
        assert_eq!(defaults::grand_summary_times().len(), 3);
    }
}
