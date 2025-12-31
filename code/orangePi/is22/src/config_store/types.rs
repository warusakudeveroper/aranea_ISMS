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
    pub rtsp_main: Option<String>,
    pub rtsp_sub: Option<String>,
    pub snapshot_url: Option<String>,
    pub family: CameraFamily,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub lacis_id: Option<String>,
    pub enabled: bool,
    pub polling_enabled: bool,
    pub polling_interval_sec: i32,
    pub suggest_policy_weight: i32,
    pub camera_context: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Camera family enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "VARCHAR")]
#[sqlx(rename_all = "lowercase")]
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
    pub name: Option<String>,
    pub location: Option<String>,
    pub floor: Option<String>,
    pub rtsp_main: Option<String>,
    pub rtsp_sub: Option<String>,
    pub snapshot_url: Option<String>,
    pub family: Option<CameraFamily>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub lacis_id: Option<String>,
    pub enabled: Option<bool>,
    pub polling_enabled: Option<bool>,
    pub polling_interval_sec: Option<i32>,
    pub suggest_policy_weight: Option<i32>,
    pub camera_context: Option<serde_json::Value>,
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
