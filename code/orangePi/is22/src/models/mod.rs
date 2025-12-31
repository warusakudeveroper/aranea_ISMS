//! Shared data models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: ApiError) -> ApiResponse<T> {
        ApiResponse {
            ok: false,
            data: None,
            error: Some(error),
        }
    }
}

/// API error
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_sec: u64,
    pub is21_connected: bool,
    pub go2rtc_connected: bool,
    pub db_connected: bool,
}

/// Device status response (araneaDevices common)
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceStatusResponse {
    pub device_type: String,
    pub lacis_id: Option<String>,
    pub firmware_version: String,
    pub uptime_sec: u64,
    pub ip_address: String,
    pub wifi_rssi: Option<i32>,
    pub free_heap: Option<u64>,
    pub mqtt_connected: bool,
}

/// Camera summary
#[derive(Debug, Serialize, Deserialize)]
pub struct CameraSummary {
    pub camera_id: String,
    pub name: String,
    pub location: String,
    pub online: bool,
    pub last_detection_at: Option<DateTime<Utc>>,
    pub last_detection_type: Option<String>,
}

/// Event summary
#[derive(Debug, Serialize, Deserialize)]
pub struct EventSummary {
    pub total_events: u64,
    pub events_today: u64,
    pub events_by_severity: Vec<SeverityCount>,
    pub top_cameras: Vec<CameraEventCount>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SeverityCount {
    pub severity: i32,
    pub count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CameraEventCount {
    pub camera_id: String,
    pub camera_name: String,
    pub count: u64,
}
