//! Summary Service Type Definitions
//!
//! Phase 3: Summary/GrandSummary (Issue #116)
//! DD02_Summary_GrandSummary.md準拠
//!
//! ## Summary種別
//! - Hourly: 時間間隔経過（デフォルト60分）
//! - Daily: 指定時刻到達（GrandSummary）
//! - Emergency: 異常検出時（severity >= 閾値）

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================
// Summary Type Definitions
// ============================================================

/// Summary種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryType {
    Hourly,
    Daily,
    Emergency,
}

impl std::fmt::Display for SummaryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hourly => write!(f, "hourly"),
            Self::Daily => write!(f, "daily"),
            Self::Emergency => write!(f, "emergency"),
        }
    }
}

impl From<&str> for SummaryType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "daily" => Self::Daily,
            "emergency" => Self::Emergency,
            _ => Self::Hourly,
        }
    }
}

/// レポート種別（スケジュール用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    Summary,
    GrandSummary,
}

impl std::fmt::Display for ReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Summary => write!(f, "summary"),
            Self::GrandSummary => write!(f, "grand_summary"),
        }
    }
}

impl From<&str> for ReportType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "grand_summary" => Self::GrandSummary,
            _ => Self::Summary,
        }
    }
}

// ============================================================
// Summary Result & Insert Structures
// ============================================================

/// Summary生成結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResult {
    pub summary_id: u64,
    pub tid: String,
    pub fid: String,
    pub summary_type: SummaryType,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary_text: String,
    pub summary_json: Option<serde_json::Value>,
    pub detection_count: i32,
    pub severity_max: i32,
    pub camera_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Summary挿入用構造体
#[derive(Debug, Clone)]
pub struct SummaryInsert {
    pub tid: String,
    pub fid: String,
    pub summary_type: SummaryType,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary_text: String,
    pub summary_json: Option<serde_json::Value>,
    pub detection_count: i32,
    pub severity_max: i32,
    pub camera_ids: serde_json::Value,
    pub expires_at: DateTime<Utc>,
}

// ============================================================
// Schedule Structures
// ============================================================

/// スケジュール設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSchedule {
    pub schedule_id: u32,
    pub tid: String,
    pub fid: String,
    pub report_type: ReportType,
    pub interval_minutes: Option<i32>,
    pub scheduled_times: Option<Vec<String>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub enabled: bool,
}

/// スケジュール更新リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScheduleRequest {
    pub report_type: ReportType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_minutes: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_times: Option<Vec<String>>,
    pub enabled: bool,
}

// ============================================================
// Paraclate Payload Structures (SSoT: Paraclate_DesignOverview.md)
// ============================================================

/// Summaryペイロード（Paraclate送信用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryPayload {
    pub lacis_oath: PayloadLacisOath,
    pub summary_overview: SummaryOverview,
    pub camera_context: HashMap<String, CameraContextItem>,
    pub camera_detection: Vec<CameraDetectionItem>,
    /// カメラステータスサマリー（接続状態の集計）
    pub camera_status_summary: CameraStatusSummary,
}

/// オフラインカメラ詳細情報
/// mobes CameraStatusSummary_Implementation_Guide.md 準拠
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineCameraDetail {
    /// カメラのLacisID
    pub lacis_id: String,
    /// カメラ名（表示名） - 必須
    pub camera_name: String,
    /// IPアドレス - 必須
    pub ip_address: String,
    /// 最終オンライン時刻（ISO 8601形式）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_online_at: Option<String>,
    /// オフライン理由（わかる場合）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// カメラステータスサマリー
/// LLMが「すべて稼働中」か「接続問題あり」を正確に判定するための情報
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraStatusSummary {
    /// 総カメラ数
    pub total_cameras: i32,
    /// オンラインカメラ数
    pub online_cameras: i32,
    /// オフライン（接続ロスト）カメラ数
    pub offline_cameras: i32,
    /// オフラインカメラ詳細（カメラ名・IP含む）
    pub offline_camera_details: Vec<OfflineCameraDetail>,
    /// 接続ロスト発生カメラのLacisIDリスト（deprecated: 後方互換用）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub offline_camera_ids: Vec<String>,
    /// 接続ロストイベント総数（期間内）
    pub connection_lost_events: i32,
    /// システム健全性: "healthy" | "degraded" | "critical"
    pub system_health: String,
}

/// LacisOath（ペイロード用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadLacisOath {
    /// is22のLacisID
    #[serde(rename = "lacisID")]
    pub lacis_id: String,
    /// テナントID
    pub tid: String,
    /// Client Identification Code
    pub cic: String,
    /// 越境アクセス用Blessing（通常はnull）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blessing: Option<String>,
}

/// SummaryOverview（サマリー概要）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryOverview {
    /// Summary ID（DBのsummary_idを正本、"SUM-{id}"形式）
    #[serde(rename = "summaryID")]
    pub summary_id: String,
    /// 最初の検出日時（ISO8601）
    pub first_detect_at: String,
    /// 最後の検出日時（ISO8601）
    pub last_detect_at: String,
    /// 検出イベント数
    pub detected_events: String,
}

impl SummaryOverview {
    /// DBのsummary_idからSUM-{id}形式を生成
    pub fn format_summary_id(db_id: u64) -> String {
        format!("SUM-{}", db_id)
    }

    /// プレースホルダーを作成（insert前）
    pub fn placeholder() -> Self {
        Self {
            summary_id: "SUM-PLACEHOLDER".to_string(),
            first_detect_at: String::new(),
            last_detect_at: String::new(),
            detected_events: "0".to_string(),
        }
    }
}

/// カメラコンテキスト項目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraContextItem {
    pub camera_name: String,
    pub camera_context: String,
    pub fid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
}

/// カメラ検出項目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraDetectionItem {
    pub timestamp: String,
    pub camera_lacis_id: String,
    pub detection_detail: String,
}

// ============================================================
// API Request/Response Structures
// ============================================================

/// Summary生成リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSummaryRequest {
    pub fid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_start: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_end: Option<DateTime<Utc>>,
}

/// Summary生成レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSummaryResponse {
    pub summary_id: u64,
    pub summary_type: SummaryType,
    pub detection_count: i32,
    pub severity_max: i32,
    pub camera_ids: Vec<String>,
}

impl From<SummaryResult> for GenerateSummaryResponse {
    fn from(result: SummaryResult) -> Self {
        Self {
            summary_id: result.summary_id,
            summary_type: result.summary_type,
            detection_count: result.detection_count,
            severity_max: result.severity_max,
            camera_ids: result.camera_ids,
        }
    }
}

/// Summary詳細レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryDetailResponse {
    pub summary_id: u64,
    pub tid: String,
    pub fid: String,
    pub summary_type: SummaryType,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary_text: String,
    pub detection_count: i32,
    pub severity_max: i32,
    pub camera_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl From<SummaryResult> for SummaryDetailResponse {
    fn from(result: SummaryResult) -> Self {
        Self {
            summary_id: result.summary_id,
            tid: result.tid,
            fid: result.fid,
            summary_type: result.summary_type,
            period_start: result.period_start,
            period_end: result.period_end,
            summary_text: result.summary_text,
            detection_count: result.detection_count,
            severity_max: result.severity_max,
            camera_ids: result.camera_ids,
            created_at: result.created_at,
        }
    }
}

/// Summary一覧レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryListResponse {
    pub summaries: Vec<SummaryDetailResponse>,
    pub total: usize,
    pub has_more: bool,
}

/// GrandSummaryレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrandSummaryResponse {
    pub summary_id: u64,
    pub summary_type: SummaryType,
    pub date: NaiveDate,
    pub summary_text: String,
    pub included_summary_ids: Vec<u64>,
    pub detection_count: i32,
    pub severity_max: i32,
    pub camera_ids: Vec<String>,
}

/// スケジュール一覧レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleListResponse {
    pub schedules: Vec<ReportSchedule>,
}

// ============================================================
// Internal Helper Structures
// ============================================================

/// カメラ統計（Summary生成時の内部構造）
#[derive(Debug, Clone, Default)]
pub struct CameraStats {
    pub detection_count: i32,
    pub severity_max: i32,
    pub detection_ids: Vec<u64>,
}

/// Summary生成エラー
#[derive(Debug, Clone)]
pub enum SummaryError {
    Database(String),
    InvalidSchedule(String),
    InvalidPeriod(String),
    GenerationFailed(String),
    NotFound(String),
}

impl std::fmt::Display for SummaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "Database error: {}", e),
            Self::InvalidSchedule(e) => write!(f, "Invalid schedule: {}", e),
            Self::InvalidPeriod(e) => write!(f, "Invalid period: {}", e),
            Self::GenerationFailed(e) => write!(f, "Generation failed: {}", e),
            Self::NotFound(e) => write!(f, "Not found: {}", e),
        }
    }
}

impl std::error::Error for SummaryError {}

impl From<SummaryError> for crate::Error {
    fn from(e: SummaryError) -> Self {
        crate::Error::Summary(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_type_display() {
        assert_eq!(SummaryType::Hourly.to_string(), "hourly");
        assert_eq!(SummaryType::Daily.to_string(), "daily");
        assert_eq!(SummaryType::Emergency.to_string(), "emergency");
    }

    #[test]
    fn test_summary_type_from_str() {
        assert_eq!(SummaryType::from("hourly"), SummaryType::Hourly);
        assert_eq!(SummaryType::from("DAILY"), SummaryType::Daily);
        assert_eq!(SummaryType::from("unknown"), SummaryType::Hourly);
    }

    #[test]
    fn test_report_type_display() {
        assert_eq!(ReportType::Summary.to_string(), "summary");
        assert_eq!(ReportType::GrandSummary.to_string(), "grand_summary");
    }

    #[test]
    fn test_summary_overview_format_id() {
        assert_eq!(SummaryOverview::format_summary_id(12345), "SUM-12345");
    }

    #[test]
    fn test_summary_payload_serialization() {
        let payload = SummaryPayload {
            lacis_oath: PayloadLacisOath {
                lacis_id: "3022AABBCCDDEEFF0000".to_string(),
                tid: "T123".to_string(),
                cic: "123456".to_string(),
                blessing: None,
            },
            summary_overview: SummaryOverview {
                summary_id: "SUM-1".to_string(),
                first_detect_at: "2026-01-10T00:00:00Z".to_string(),
                last_detect_at: "2026-01-10T01:00:00Z".to_string(),
                detected_events: "42".to_string(),
            },
            camera_context: HashMap::new(),
            camera_detection: vec![],
            camera_status_summary: CameraStatusSummary {
                total_cameras: 8,
                online_cameras: 5,
                offline_cameras: 3,
                offline_camera_details: vec![
                    OfflineCameraDetail {
                        lacis_id: "3080AABBCCDD00010001".to_string(),
                        camera_name: "エントランス北".to_string(),
                        ip_address: "192.168.1.101".to_string(),
                        last_online_at: Some("2026-01-13T08:45:00Z".to_string()),
                        reason: Some("connection_lost".to_string()),
                    },
                ],
                offline_camera_ids: vec![],
                connection_lost_events: 5,
                system_health: "degraded".to_string(),
            },
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("lacisID"));
        assert!(json.contains("summaryID"));
        assert!(json.contains("cameraStatusSummary"));
        assert!(json.contains("offlineCameraDetails"));
        assert!(json.contains("エントランス北"));
    }
}
