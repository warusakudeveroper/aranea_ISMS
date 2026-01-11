//! Camera Malfunction Reporter
//!
//! T5-3: カメラ不調報告（mobes2.0回答に基づく実装）
//!
//! ## 概要
//! カメラの不調状態をParaclate IngestEvent APIに報告
//!
//! ## mobes2.0仕様
//! - IngestEvent APIにmalfunction_type含めて送信
//! - 不調種別: offline, stream_error, high_latency, low_fps, no_frames

use crate::paraclate_client::{
    types::{CameraMalfunctionType, EventPayload},
    ParaclateClient,
};
use chrono::Utc;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// カメラ不調報告サービス
pub struct CameraMalfunctionReporter {
    paraclate_client: Arc<ParaclateClient>,
}

impl CameraMalfunctionReporter {
    /// 新規作成
    pub fn new(paraclate_client: Arc<ParaclateClient>) -> Self {
        Self { paraclate_client }
    }

    /// カメラ不調をParaclateに報告
    ///
    /// # Arguments
    /// * `tid` - テナントID
    /// * `fid` - 施設ID
    /// * `camera_lacis_id` - カメラのLacisID
    /// * `malfunction_type` - 不調種別
    /// * `details` - 不調詳細情報（オプション）
    ///
    /// # Returns
    /// 送信成功時はイベントID、失敗時はエラー
    pub async fn report_malfunction(
        &self,
        tid: &str,
        fid: &str,
        camera_lacis_id: &str,
        malfunction_type: CameraMalfunctionType,
        details: Option<serde_json::Value>,
    ) -> crate::Result<Option<String>> {
        info!(
            camera_lacis_id = %camera_lacis_id,
            malfunction_type = %malfunction_type,
            "Reporting camera malfunction"
        );

        // 不調イベント用ペイロードを構築
        let event_payload = EventPayload {
            tid: tid.to_string(),
            fid: fid.to_string(),
            log_id: 0, // 不調報告は検出ログIDなし
            camera_lacis_id: camera_lacis_id.to_string(),
            captured_at: Utc::now(),
            primary_event: "camera_malfunction".to_string(),
            severity: match malfunction_type {
                CameraMalfunctionType::Offline => 3, // 最高重要度
                CameraMalfunctionType::NoFrames => 3,
                CameraMalfunctionType::StreamError => 2,
                CameraMalfunctionType::HighLatency => 1,
                CameraMalfunctionType::LowFps => 1,
            },
            confidence: 1.0, // 不調は確実に検出されている
            tags: vec![
                "malfunction".to_string(),
                malfunction_type.to_string(),
            ],
            details: None,
            snapshot_base64: None,
            snapshot_mime_type: None,
            malfunction_type: Some(malfunction_type.clone()),
            malfunction_details: details,
        };

        // Paraclate IngestEvent APIに送信
        let result = self
            .paraclate_client
            .send_event_with_snapshot(tid, fid, event_payload, None, 0)
            .await;

        match result {
            Ok(send_result) => {
                if send_result.success {
                    info!(
                        camera_lacis_id = %camera_lacis_id,
                        event_id = ?send_result.event_id,
                        "Camera malfunction reported successfully"
                    );
                    Ok(send_result.event_id)
                } else {
                    warn!(
                        camera_lacis_id = %camera_lacis_id,
                        error = ?send_result.error,
                        queue_id = send_result.queue_id,
                        "Camera malfunction report queued (offline)"
                    );
                    Ok(None)
                }
            }
            Err(e) => {
                error!(
                    camera_lacis_id = %camera_lacis_id,
                    error = %e,
                    "Failed to report camera malfunction"
                );
                Err(crate::Error::Paraclate(e.to_string()))
            }
        }
    }

    /// カメラオフラインを報告
    pub async fn report_offline(
        &self,
        tid: &str,
        fid: &str,
        camera_lacis_id: &str,
        last_seen_at: Option<chrono::DateTime<Utc>>,
    ) -> crate::Result<Option<String>> {
        let details = last_seen_at.map(|ts| {
            serde_json::json!({
                "lastSeenAt": ts.to_rfc3339(),
                "offlineSince": Utc::now().to_rfc3339()
            })
        });

        self.report_malfunction(
            tid,
            fid,
            camera_lacis_id,
            CameraMalfunctionType::Offline,
            details,
        )
        .await
    }

    /// ストリームエラーを報告
    pub async fn report_stream_error(
        &self,
        tid: &str,
        fid: &str,
        camera_lacis_id: &str,
        error_message: &str,
        consecutive_errors: u32,
    ) -> crate::Result<Option<String>> {
        let details = Some(serde_json::json!({
            "errorMessage": error_message,
            "consecutiveErrors": consecutive_errors,
            "timestamp": Utc::now().to_rfc3339()
        }));

        self.report_malfunction(
            tid,
            fid,
            camera_lacis_id,
            CameraMalfunctionType::StreamError,
            details,
        )
        .await
    }

    /// 高レイテンシを報告
    pub async fn report_high_latency(
        &self,
        tid: &str,
        fid: &str,
        camera_lacis_id: &str,
        latency_ms: u64,
        threshold_ms: u64,
    ) -> crate::Result<Option<String>> {
        let details = Some(serde_json::json!({
            "latencyMs": latency_ms,
            "thresholdMs": threshold_ms,
            "timestamp": Utc::now().to_rfc3339()
        }));

        self.report_malfunction(
            tid,
            fid,
            camera_lacis_id,
            CameraMalfunctionType::HighLatency,
            details,
        )
        .await
    }

    /// 低FPSを報告
    pub async fn report_low_fps(
        &self,
        tid: &str,
        fid: &str,
        camera_lacis_id: &str,
        current_fps: f32,
        expected_fps: f32,
    ) -> crate::Result<Option<String>> {
        let details = Some(serde_json::json!({
            "currentFps": current_fps,
            "expectedFps": expected_fps,
            "timestamp": Utc::now().to_rfc3339()
        }));

        self.report_malfunction(
            tid,
            fid,
            camera_lacis_id,
            CameraMalfunctionType::LowFps,
            details,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_malfunction_type_display() {
        assert_eq!(CameraMalfunctionType::Offline.to_string(), "offline");
        assert_eq!(CameraMalfunctionType::StreamError.to_string(), "stream_error");
        assert_eq!(CameraMalfunctionType::HighLatency.to_string(), "high_latency");
        assert_eq!(CameraMalfunctionType::LowFps.to_string(), "low_fps");
        assert_eq!(CameraMalfunctionType::NoFrames.to_string(), "no_frames");
    }
}
