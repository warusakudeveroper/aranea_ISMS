//! Payload Builder
//!
//! Phase 3: Summary/GrandSummary (Issue #116)
//! Paraclate送信用JSONペイロードの構築
//!
//! ## SSoT: Paraclate_DesignOverview.md

use super::types::{
    CameraContextItem, CameraDetectionItem, CameraStatusSummary, OfflineCameraDetail,
    PayloadLacisOath, SummaryOverview, SummaryPayload,
};
use std::collections::HashSet;
use crate::camera_registry::{CameraContext, ContextMapEntry};
use crate::detection_log_service::DetectionLog;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::debug;

/// Paraclate送信用ペイロードビルダー
pub struct PayloadBuilder {
    /// is22のLacisID
    is22_lacis_id: String,
    /// テナントID
    tid: String,
    /// is22のCIC
    cic: String,
}

impl PayloadBuilder {
    /// 新しいPayloadBuilderを作成
    pub fn new(is22_lacis_id: String, tid: String, cic: String) -> Self {
        Self {
            is22_lacis_id,
            tid,
            cic,
        }
    }

    /// Summaryペイロードを構築
    ///
    /// # Arguments
    /// * `detection_logs` - 検出ログ一覧
    /// * `camera_context_map` - カメラコンテキストマップ（lacisID -> context）
    /// * `first_detect` - 最初の検出日時
    /// * `last_detect` - 最後の検出日時
    ///
    /// # Note
    /// summary_idはプレースホルダー。insert後に正しい値で更新する。
    pub fn build_summary_payload(
        &self,
        detection_logs: &[DetectionLog],
        camera_context_map: &HashMap<String, ContextMapEntry>,
        first_detect: DateTime<Utc>,
        last_detect: DateTime<Utc>,
    ) -> SummaryPayload {
        // lacisOath構築
        let lacis_oath = PayloadLacisOath {
            lacis_id: self.is22_lacis_id.clone(),
            tid: self.tid.clone(),
            cic: self.cic.clone(),
            blessing: None,
        };

        // summaryOverview構築（summary_idはプレースホルダー）
        let summary_overview = SummaryOverview {
            summary_id: "SUM-PLACEHOLDER".to_string(),
            first_detect_at: first_detect.to_rfc3339(),
            last_detect_at: last_detect.to_rfc3339(),
            detected_events: detection_logs.len().to_string(),
        };

        // cameraContext構築
        let mut camera_context: HashMap<String, CameraContextItem> = HashMap::new();
        for (lacis_id, entry) in camera_context_map {
            camera_context.insert(
                lacis_id.clone(),
                CameraContextItem {
                    camera_name: entry.name.clone(),
                    camera_context: entry.context_summary.clone(),
                    fid: entry.location.clone().unwrap_or_default(),
                    rid: None,
                    preset: None,
                },
            );
        }

        // cameraDetection構築
        let camera_detection: Vec<CameraDetectionItem> = detection_logs
            .iter()
            .filter_map(|log| {
                log.camera_lacis_id.as_ref().map(|lacis_id| {
                    CameraDetectionItem {
                        timestamp: log.captured_at.to_rfc3339(),
                        camera_lacis_id: lacis_id.clone(),
                        detection_detail: self.format_detection_detail(log),
                    }
                })
            })
            .collect();

        // カメラステータスサマリー構築
        let camera_status_summary =
            self.build_camera_status_summary(detection_logs, camera_context_map);

        debug!(
            detection_count = detection_logs.len(),
            camera_count = camera_context.len(),
            offline_cameras = camera_status_summary.offline_cameras,
            system_health = %camera_status_summary.system_health,
            "Built summary payload"
        );

        SummaryPayload {
            lacis_oath,
            summary_overview,
            camera_context,
            camera_detection,
            camera_status_summary,
        }
    }

    /// カメラステータスサマリーを構築
    /// camera_lost イベントを集計してシステム健全性を判定
    /// mobes CameraStatusSummary_Implementation_Guide.md 準拠
    fn build_camera_status_summary(
        &self,
        detection_logs: &[DetectionLog],
        camera_context_map: &HashMap<String, ContextMapEntry>,
    ) -> CameraStatusSummary {
        let total_cameras = camera_context_map.len() as i32;

        // camera_lost イベントを集計（最終イベント時刻も記録）
        let mut offline_camera_data: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
        let mut connection_lost_events = 0i32;

        for log in detection_logs {
            if log.primary_event == "camera_lost" || log.primary_event == "connection_lost" {
                connection_lost_events += 1;
                if let Some(lacis_id) = &log.camera_lacis_id {
                    let reason = Some(log.primary_event.clone());
                    let last_online = Some(log.captured_at.to_rfc3339());
                    offline_camera_data.insert(lacis_id.clone(), (last_online, reason));
                }
            }
        }

        let offline_cameras = offline_camera_data.len() as i32;
        let online_cameras = total_cameras - offline_cameras;

        // オフラインカメラ詳細を構築（カメラ名・IP含む）
        let offline_camera_details: Vec<OfflineCameraDetail> = offline_camera_data
            .iter()
            .map(|(lacis_id, (last_online, reason))| {
                let entry = camera_context_map.get(lacis_id);
                OfflineCameraDetail {
                    lacis_id: lacis_id.clone(),
                    camera_name: entry.map(|e| e.name.clone()).unwrap_or_else(|| "Unknown".to_string()),
                    ip_address: entry
                        .and_then(|e| e.ip_address.clone())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    last_online_at: last_online.clone(),
                    reason: reason.clone(),
                }
            })
            .collect();

        // 後方互換用のLacisIDリスト
        let offline_camera_ids: Vec<String> = offline_camera_data.keys().cloned().collect();

        // システム健全性判定
        // - healthy: 接続ロストなし
        // - degraded: 一部カメラで接続ロスト（50%未満）
        // - critical: 半数以上のカメラで接続ロスト
        let system_health = if offline_cameras == 0 {
            "healthy".to_string()
        } else if total_cameras > 0 && (offline_cameras as f32 / total_cameras as f32) >= 0.5 {
            "critical".to_string()
        } else {
            "degraded".to_string()
        };

        CameraStatusSummary {
            total_cameras,
            online_cameras,
            offline_cameras,
            offline_camera_details,
            offline_camera_ids,
            connection_lost_events,
            system_health,
        }
    }

    /// 検出詳細をJSON文字列にフォーマット
    /// mobes summaryImageContext.ts 対応: confidence追加
    fn format_detection_detail(&self, log: &DetectionLog) -> String {
        serde_json::json!({
            "detection_id": log.log_id,
            "camera_id": log.camera_id,
            "primary_event": log.primary_event,
            "severity": log.severity,
            "confidence": log.confidence,
            "count_hint": log.count_hint
        })
        .to_string()
    }

    /// SummaryペイロードのsummaryIDを更新
    pub fn update_summary_id(
        payload: &mut serde_json::Value,
        db_summary_id: u64,
    ) -> crate::Result<()> {
        let summary_id_str = SummaryOverview::format_summary_id(db_summary_id);

        if let Some(overview) = payload.get_mut("summaryOverview") {
            if let Some(id_field) = overview.get_mut("summaryID") {
                *id_field = serde_json::Value::String(summary_id_str);
            }
        }

        Ok(())
    }
}

/// 検出ログから最初/最後の検出時刻を算出
pub fn calculate_detect_times(logs: &[DetectionLog]) -> (DateTime<Utc>, DateTime<Utc>) {
    if logs.is_empty() {
        let now = Utc::now();
        return (now, now);
    }

    let mut first = logs[0].captured_at;
    let mut last = logs[0].captured_at;

    for log in logs {
        if log.captured_at < first {
            first = log.captured_at;
        }
        if log.captured_at > last {
            last = log.captured_at;
        }
    }

    (first, last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_builder_new() {
        let builder = PayloadBuilder::new(
            "3022AABBCCDDEEFF0000".to_string(),
            "T123".to_string(),
            "123456".to_string(),
        );

        assert_eq!(builder.is22_lacis_id, "3022AABBCCDDEEFF0000");
        assert_eq!(builder.tid, "T123");
        assert_eq!(builder.cic, "123456");
    }

    #[test]
    fn test_format_summary_id() {
        assert_eq!(SummaryOverview::format_summary_id(12345), "SUM-12345");
        assert_eq!(SummaryOverview::format_summary_id(1), "SUM-1");
    }

    #[test]
    fn test_update_summary_id() {
        let mut payload = serde_json::json!({
            "summaryOverview": {
                "summaryID": "SUM-PLACEHOLDER",
                "firstDetectAt": "2026-01-10T00:00:00Z"
            }
        });

        PayloadBuilder::update_summary_id(&mut payload, 999).unwrap();

        assert_eq!(
            payload["summaryOverview"]["summaryID"],
            serde_json::Value::String("SUM-999".to_string())
        );
    }
}
