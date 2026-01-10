//! Payload Builder
//!
//! Phase 3: Summary/GrandSummary (Issue #116)
//! Paraclate送信用JSONペイロードの構築
//!
//! ## SSoT: Paraclate_DesignOverview.md

use super::types::{
    CameraContextItem, CameraDetectionItem, PayloadLacisOath, SummaryOverview, SummaryPayload,
};
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
                        timestamp: log.detected_at.to_rfc3339(),
                        camera_lacis_id: lacis_id.clone(),
                        detection_detail: self.format_detection_detail(log),
                    }
                })
            })
            .collect();

        debug!(
            detection_count = detection_logs.len(),
            camera_count = camera_context.len(),
            "Built summary payload"
        );

        SummaryPayload {
            lacis_oath,
            summary_overview,
            camera_context,
            camera_detection,
        }
    }

    /// 検出詳細をJSON文字列にフォーマット
    fn format_detection_detail(&self, log: &DetectionLog) -> String {
        serde_json::json!({
            "detection_id": log.id,
            "camera_id": log.camera_id,
            "detection_reason": log.detection_reason,
            "suspicious_score": log.suspicious_score,
            "object_count": log.object_count
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

    let mut first = logs[0].detected_at;
    let mut last = logs[0].detected_at;

    for log in logs {
        if log.detected_at < first {
            first = log.detected_at;
        }
        if log.detected_at > last {
            last = log.detected_at;
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
