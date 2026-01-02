//! ConfigStore data types
//!
//! SSoT data structures for cameras, policies, and settings

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Camera entity (SSoT)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Camera {
    pub camera_id: String,
    pub name: String,
    pub location: String,
    pub floor: Option<String>,
    // === 管理用フィールド ===
    pub rid: Option<String>,
    pub rtsp_main: Option<String>,
    pub rtsp_sub: Option<String>,
    pub rtsp_username: Option<String>,
    pub rtsp_password: Option<String>,
    pub snapshot_url: Option<String>,
    /// Stored as VARCHAR in MySQL, converted to/from CameraFamily
    pub family: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub lacis_id: Option<String>,
    pub cic: Option<String>,
    pub enabled: bool,
    pub polling_enabled: bool,
    pub polling_interval_sec: i32,
    pub suggest_policy_weight: i32,
    pub camera_context: Option<serde_json::Value>,
    pub rotation: i32,
    pub fid: Option<String>,
    pub tid: Option<String>,
    pub sort_order: i32,
    // === AI分析設定（migration 009） ===
    pub preset_id: Option<String>,
    pub preset_version: Option<String>,
    pub ai_enabled: bool,
    pub ai_interval_sec: i32,
    // === ONVIF デバイス情報 ===
    pub serial_number: Option<String>,
    pub hardware_id: Option<String>,
    pub firmware_version: Option<String>,
    pub onvif_endpoint: Option<String>,
    // === ネットワーク情報 ===
    pub rtsp_port: Option<i32>,
    pub http_port: Option<i32>,
    pub onvif_port: Option<i32>,
    // === ビデオ能力（メイン） ===
    pub resolution_main: Option<String>,
    pub codec_main: Option<String>,
    pub fps_main: Option<i32>,
    pub bitrate_main: Option<i32>,
    // === ビデオ能力（サブ） ===
    pub resolution_sub: Option<String>,
    pub codec_sub: Option<String>,
    pub fps_sub: Option<i32>,
    pub bitrate_sub: Option<i32>,
    // === PTZ能力 ===
    pub ptz_supported: bool,
    pub ptz_continuous: bool,
    pub ptz_absolute: bool,
    pub ptz_relative: bool,
    pub ptz_pan_range: Option<serde_json::Value>,
    pub ptz_tilt_range: Option<serde_json::Value>,
    pub ptz_zoom_range: Option<serde_json::Value>,
    pub ptz_presets: Option<serde_json::Value>,
    pub ptz_home_supported: bool,
    // === 音声能力 ===
    pub audio_input_supported: bool,
    pub audio_output_supported: bool,
    pub audio_codec: Option<String>,
    // === ONVIFプロファイル ===
    pub onvif_profiles: Option<serde_json::Value>,
    // === 検出メタ情報 ===
    pub discovery_method: Option<String>,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub last_rescan_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    // === タイムスタンプ ===
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Camera family enum (for API serialization only, not for sqlx)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

impl From<&str> for CameraFamily {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tapo" => Self::Tapo,
            "vigi" => Self::Vigi,
            "nest" => Self::Nest,
            "axis" => Self::Axis,
            "hikvision" => Self::Hikvision,
            "dahua" => Self::Dahua,
            "other" => Self::Other,
            _ => Self::Unknown,
        }
    }
}

impl From<String> for CameraFamily {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl From<CameraFamily> for String {
    fn from(f: CameraFamily) -> Self {
        match f {
            CameraFamily::Tapo => "tapo".to_string(),
            CameraFamily::Vigi => "vigi".to_string(),
            CameraFamily::Nest => "nest".to_string(),
            CameraFamily::Axis => "axis".to_string(),
            CameraFamily::Hikvision => "hikvision".to_string(),
            CameraFamily::Dahua => "dahua".to_string(),
            CameraFamily::Other => "other".to_string(),
            CameraFamily::Unknown => "unknown".to_string(),
        }
    }
}

/// Camera creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCameraRequest {
    pub camera_id: String,
    pub name: String,
    pub location: String,
    pub floor: Option<String>,
    pub rtsp_main: Option<String>,
    pub rtsp_sub: Option<String>,
    pub snapshot_url: Option<String>,
    pub family: Option<CameraFamily>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub camera_context: Option<serde_json::Value>,
}

/// Camera update request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateCameraRequest {
    // === 基本情報 ===
    pub name: Option<String>,
    pub location: Option<String>,
    pub floor: Option<String>,
    pub rid: Option<String>,
    // === ストリーム設定 ===
    pub rtsp_main: Option<String>,
    pub rtsp_sub: Option<String>,
    pub rtsp_username: Option<String>,
    pub rtsp_password: Option<String>,
    pub snapshot_url: Option<String>,
    // === デバイス情報 ===
    pub family: Option<CameraFamily>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub lacis_id: Option<String>,
    pub cic: Option<String>,
    // === ポリシー ===
    pub enabled: Option<bool>,
    pub polling_enabled: Option<bool>,
    pub polling_interval_sec: Option<i32>,
    pub suggest_policy_weight: Option<i32>,
    // === カメラコンテキスト・表示 ===
    pub camera_context: Option<serde_json::Value>,
    pub rotation: Option<i32>,
    pub sort_order: Option<i32>,
    // === 所属情報 ===
    pub fid: Option<String>,
    pub tid: Option<String>,
    // === AI分析設定 ===
    pub preset_id: Option<String>,
    pub preset_version: Option<String>,
    pub ai_enabled: Option<bool>,
    pub ai_interval_sec: Option<i32>,
    // === ONVIF デバイス情報 ===
    pub serial_number: Option<String>,
    pub hardware_id: Option<String>,
    pub firmware_version: Option<String>,
    pub onvif_endpoint: Option<String>,
    // === ネットワーク情報 ===
    pub rtsp_port: Option<i32>,
    pub http_port: Option<i32>,
    pub onvif_port: Option<i32>,
    // === ビデオ能力（メイン） ===
    pub resolution_main: Option<String>,
    pub codec_main: Option<String>,
    pub fps_main: Option<i32>,
    pub bitrate_main: Option<i32>,
    // === ビデオ能力（サブ） ===
    pub resolution_sub: Option<String>,
    pub codec_sub: Option<String>,
    pub fps_sub: Option<i32>,
    pub bitrate_sub: Option<i32>,
    // === PTZ能力 ===
    pub ptz_supported: Option<bool>,
    pub ptz_continuous: Option<bool>,
    pub ptz_absolute: Option<bool>,
    pub ptz_relative: Option<bool>,
    pub ptz_pan_range: Option<serde_json::Value>,
    pub ptz_tilt_range: Option<serde_json::Value>,
    pub ptz_zoom_range: Option<serde_json::Value>,
    pub ptz_presets: Option<serde_json::Value>,
    pub ptz_home_supported: Option<bool>,
    // === 音声能力 ===
    pub audio_input_supported: Option<bool>,
    pub audio_output_supported: Option<bool>,
    pub audio_codec: Option<String>,
    // === ONVIFプロファイル ===
    pub onvif_profiles: Option<serde_json::Value>,
    // === 検出メタ情報 ===
    pub discovery_method: Option<String>,
}

/// Schema version entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SchemaVersion {
    pub version_id: String,
    pub schema_json: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Setting entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Setting {
    pub setting_key: String,
    pub setting_json: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

/// Polling policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingPolicy {
    pub base_interval_sec: i32,
    pub priority_interval_sec: i32,
    pub max_concurrent: i32,
    pub timeout_ms: i32,
}

impl Default for PollingPolicy {
    fn default() -> Self {
        Self {
            base_interval_sec: 60,
            priority_interval_sec: 15,
            max_concurrent: 5,
            timeout_ms: 10000,
        }
    }
}

/// Suggest policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestPolicy {
    pub min_score_threshold: i32,
    pub ttl_sec: i32,
    pub hysteresis_ratio: f32,
    pub cooldown_after_clear_sec: i32,
    pub sticky_policies: Vec<String>,
}

impl Default for SuggestPolicy {
    fn default() -> Self {
        Self {
            min_score_threshold: 50,
            ttl_sec: 60,
            hysteresis_ratio: 1.2,
            cooldown_after_clear_sec: 10,
            sticky_policies: vec!["hazard_detected".to_string()],
        }
    }
}

/// Admission policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionPolicy {
    pub total_stream_units: i32,
    pub max_ui_users: i32,
    pub reserved_baseline_units: i32,
    pub sub_stream_cost: i32,
    pub main_stream_cost: i32,
    pub modal_ttl_sec: i32,
    pub heartbeat_interval_sec: i32,
    pub heartbeat_grace_sec: i32,
}

impl Default for AdmissionPolicy {
    fn default() -> Self {
        Self {
            total_stream_units: 50,
            max_ui_users: 10,
            reserved_baseline_units: 5,
            sub_stream_cost: 1,
            main_stream_cost: 2,
            modal_ttl_sec: 300,
            heartbeat_interval_sec: 30,
            heartbeat_grace_sec: 45,
        }
    }
}

/// Notification policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPolicy {
    pub cooldown_sec: i32,
    pub daily_summary_enabled: bool,
    pub daily_summary_time: String,
    pub webhook_url: Option<String>,
}

impl Default for NotificationPolicy {
    fn default() -> Self {
        Self {
            cooldown_sec: 300,
            daily_summary_enabled: true,
            daily_summary_time: "08:00".to_string(),
            webhook_url: None,
        }
    }
}
