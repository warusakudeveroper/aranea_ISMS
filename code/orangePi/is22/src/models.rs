//! Shared models and types for IS22
//!
//! This module contains types shared across multiple modules
//! to avoid circular dependencies.

use serde::{Deserialize, Serialize};

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_sec: u64,
    pub is21_connected: bool,
    pub go2rtc_connected: bool,
    pub db_connected: bool,
}

/// Processing time breakdown for performance analysis
///
/// ボトルネック分析用の処理時間内訳。
/// 各フェーズの処理時間を記録し、どこが遅いか特定可能にする。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingTimings {
    /// Total processing time (ms)
    pub total_ms: i32,
    /// Snapshot capture time - カメラからの画像取得時間 (ms)
    pub snapshot_ms: i32,
    /// IS21 request/response time - IS21への送信〜レスポンス受信 (ms)
    pub is21_roundtrip_ms: i32,
    /// IS21 internal inference time - IS21内の推論時間 (ms)
    pub is21_inference_ms: i32,
    /// IS21 YOLO detection time (ms)
    pub is21_yolo_ms: i32,
    /// IS21 PAR (attribute) analysis time (ms)
    pub is21_par_ms: i32,
    /// Database save time (ms)
    pub save_ms: i32,
    /// Snapshot capture source: "go2rtc", "ffmpeg", or "http"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_source: Option<String>,
}

impl ProcessingTimings {
    /// Create new ProcessingTimings
    pub fn new(
        total_ms: i32,
        snapshot_ms: i32,
        is21_roundtrip_ms: i32,
        is21_inference_ms: i32,
        is21_yolo_ms: i32,
        is21_par_ms: i32,
        save_ms: i32,
    ) -> Self {
        Self {
            total_ms,
            snapshot_ms,
            is21_roundtrip_ms,
            is21_inference_ms,
            is21_yolo_ms,
            is21_par_ms,
            save_ms,
            snapshot_source: None,
        }
    }

    /// Create new ProcessingTimings with snapshot source
    pub fn with_source(
        total_ms: i32,
        snapshot_ms: i32,
        is21_roundtrip_ms: i32,
        is21_inference_ms: i32,
        is21_yolo_ms: i32,
        is21_par_ms: i32,
        save_ms: i32,
        snapshot_source: String,
    ) -> Self {
        Self {
            total_ms,
            snapshot_ms,
            is21_roundtrip_ms,
            is21_inference_ms,
            is21_yolo_ms,
            is21_par_ms,
            save_ms,
            snapshot_source: Some(snapshot_source),
        }
    }

    /// Get network overhead (roundtrip - inference)
    pub fn network_overhead_ms(&self) -> i32 {
        self.is21_roundtrip_ms - self.is21_inference_ms
    }

    /// Get percentage breakdown
    pub fn percentage_breakdown(&self) -> TimingPercentages {
        if self.total_ms == 0 {
            return TimingPercentages::default();
        }
        let total = self.total_ms as f32;
        TimingPercentages {
            snapshot_pct: (self.snapshot_ms as f32 / total * 100.0).round() as i32,
            is21_pct: (self.is21_roundtrip_ms as f32 / total * 100.0).round() as i32,
            save_pct: (self.save_ms as f32 / total * 100.0).round() as i32,
        }
    }
}

/// Percentage breakdown of processing time
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimingPercentages {
    pub snapshot_pct: i32,
    pub is21_pct: i32,
    pub save_pct: i32,
}
