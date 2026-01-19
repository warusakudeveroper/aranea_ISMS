//! Summary Generator
//!
//! Phase 3: Summary/GrandSummary (Issue #116)
//! T3-2: summary_generator.rs 実装
//!
//! ## 処理フロー
//! 1. 期間内の検出ログを取得
//! 2. カメラごとに集計
//! 3. summary_text生成（テンプレートベース）
//! 4. summary_json構築（Paraclate送信用）
//! 5. DB保存
//! 6. summaryID更新（DB正本値）

use super::payload_builder::{calculate_detect_times, PayloadBuilder};
use super::repository::SummaryRepository;
use super::types::{CameraStats, SummaryInsert, SummaryResult, SummaryType};
use crate::camera_registry::CameraContextService;
use crate::config_store::ConfigStore;
use crate::detection_log_service::DetectionLogService;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Summary生成サービス
pub struct SummaryGenerator {
    /// 検出ログサービス
    detection_log_service: Arc<DetectionLogService>,
    /// カメラコンテキストサービス
    camera_context_service: CameraContextService,
    /// Summaryリポジトリ
    repository: SummaryRepository,
    /// 設定ストア
    config_store: Arc<ConfigStore>,
}

impl SummaryGenerator {
    /// 新しいSummaryGeneratorを作成
    pub fn new(
        detection_log_service: Arc<DetectionLogService>,
        camera_context_service: CameraContextService,
        repository: SummaryRepository,
        config_store: Arc<ConfigStore>,
    ) -> Self {
        Self {
            detection_log_service,
            camera_context_service,
            repository,
            config_store,
        }
    }

    /// Summary生成（期間指定）
    pub async fn generate(
        &self,
        tid: &str,
        fid: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> crate::Result<SummaryResult> {
        info!(
            tid = %tid,
            fid = %fid,
            period_start = %period_start,
            period_end = %period_end,
            "Generating summary"
        );

        // 1. 期間内の検出ログを取得
        let logs = self
            .detection_log_service
            .get_by_tid_fid(tid, Some(fid), 10000)
            .await?
            .into_iter()
            .filter(|log| log.captured_at >= period_start && log.captured_at <= period_end)
            .collect::<Vec<_>>();

        if logs.is_empty() {
            return self
                .create_empty_summary(tid, fid, period_start, period_end)
                .await;
        }

        // 2. カメラごとに集計
        let mut camera_stats: HashMap<String, CameraStats> = HashMap::new();
        let mut severity_max = 0i32;

        for log in &logs {
            let camera_id = log.camera_lacis_id.clone().unwrap_or_else(|| log.camera_id.clone());
            let stats = camera_stats.entry(camera_id).or_default();

            stats.detection_count += 1;
            let severity = log.severity;
            stats.severity_max = stats.severity_max.max(severity);
            stats.detection_ids.push(log.log_id.unwrap_or(0));

            severity_max = severity_max.max(severity);
        }

        let camera_ids: Vec<String> = camera_stats.keys().cloned().collect();

        // 3. カメラコンテキストを取得
        let context_map = self.camera_context_service.build_context_map(tid).await?;

        // 4. 最初/最後の検出時刻を算出
        let (first_detect, last_detect) = calculate_detect_times(&logs);

        // 5. summary_text生成
        let summary_text = self.generate_summary_text(
            &logs,
            &camera_stats,
            period_start,
            period_end,
        );

        // 6. PayloadBuilder取得とsummary_json構築
        let payload_builder = self.get_payload_builder().await?;
        let payload = payload_builder.build_summary_payload(
            &logs,
            &context_map,
            first_detect,
            last_detect,
        );
        let summary_json = serde_json::to_value(&payload)
            .map_err(|e| crate::Error::Parse(e.to_string()))?;

        // 7. DB保存
        let mut result = self
            .repository
            .insert(SummaryInsert {
                tid: tid.to_string(),
                fid: fid.to_string(),
                summary_type: SummaryType::Hourly,
                period_start,
                period_end,
                summary_text,
                summary_json: Some(summary_json.clone()),
                detection_count: logs.len() as i32,
                severity_max,
                camera_ids: serde_json::to_value(&camera_ids)
                    .map_err(|e| crate::Error::Parse(e.to_string()))?,
                expires_at: Utc::now() + Duration::days(30),
            })
            .await?;

        // 8. summaryIDを正しい値で更新
        let mut updated_json = summary_json;
        PayloadBuilder::update_summary_id(&mut updated_json, result.summary_id)?;
        self.repository
            .update_summary_json(result.summary_id, &updated_json)
            .await?;

        result.summary_json = Some(updated_json);

        info!(
            summary_id = result.summary_id,
            detection_count = result.detection_count,
            "Summary generated successfully"
        );

        Ok(result)
    }

    /// 空のSummaryを生成（検出なしの場合）
    async fn create_empty_summary(
        &self,
        tid: &str,
        fid: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> crate::Result<SummaryResult> {
        let summary_text = format!(
            "{}から{}の期間: 検出イベントなし",
            period_start.format("%Y-%m-%d %H:%M"),
            period_end.format("%Y-%m-%d %H:%M")
        );

        let result = self
            .repository
            .insert(SummaryInsert {
                tid: tid.to_string(),
                fid: fid.to_string(),
                summary_type: SummaryType::Hourly,
                period_start,
                period_end,
                summary_text,
                summary_json: None,
                detection_count: 0,
                severity_max: 0,
                camera_ids: serde_json::json!([]),
                expires_at: Utc::now() + Duration::days(30),
            })
            .await?;

        debug!(
            summary_id = result.summary_id,
            "Empty summary created"
        );

        Ok(result)
    }

    /// テンプレートベースのサマリーテキスト生成
    fn generate_summary_text(
        &self,
        logs: &[crate::detection_log_service::DetectionLog],
        camera_stats: &HashMap<String, CameraStats>,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> String {
        let duration_hours = (period_end - period_start).num_hours();
        let duration_str = if duration_hours >= 1 {
            format!("{}時間", duration_hours)
        } else {
            let minutes = (period_end - period_start).num_minutes();
            format!("{}分", minutes)
        };

        let total_count = logs.len();
        let camera_count = camera_stats.len();

        let camera_breakdown = self.format_camera_breakdown(camera_stats);

        format!(
            "{}の検出サマリー: 合計{}件の検出（{}台のカメラ）\n{}",
            duration_str,
            total_count,
            camera_count,
            camera_breakdown
        )
    }

    /// カメラ別の検出内訳をフォーマット
    fn format_camera_breakdown(&self, stats: &HashMap<String, CameraStats>) -> String {
        let mut lines: Vec<String> = stats
            .iter()
            .map(|(id, s)| {
                // LacisIDの場合は短縮表示
                let display_id = if id.len() > 10 {
                    format!("{}...", &id[..10])
                } else {
                    id.clone()
                };
                format!(
                    "- {}: {}件 (最大severity: {})",
                    display_id, s.detection_count, s.severity_max
                )
            })
            .collect();

        lines.sort();
        lines.join("\n")
    }

    /// PayloadBuilderを取得（設定値からis22の登録情報を取得）
    async fn get_payload_builder(&self) -> crate::Result<PayloadBuilder> {
        let is22_lacis_id = self
            .get_config_value("aranea.lacis_id")
            .await
            .ok_or_else(|| crate::Error::Config("IS22 LacisID not configured".into()))?;

        let tid = self
            .get_config_value("aranea.tid")
            .await
            .ok_or_else(|| crate::Error::Config("TID not configured".into()))?;

        let cic = self
            .get_config_value("aranea.cic")
            .await
            .ok_or_else(|| crate::Error::Config("CIC not configured".into()))?;

        Ok(PayloadBuilder::new(is22_lacis_id, tid, cic))
    }

    /// ConfigStoreから設定値を取得
    async fn get_config_value(&self, key: &str) -> Option<String> {
        self.config_store
            .service()
            .get_setting(key)
            .await
            .ok()
            .flatten()
            .and_then(|v| v.as_str().map(String::from))
    }

    /// Emergency Summary生成（異常検出時の即時報告）
    pub async fn generate_emergency(
        &self,
        tid: &str,
        fid: &str,
        detection_log: &crate::detection_log_service::DetectionLog,
    ) -> crate::Result<SummaryResult> {
        let now = Utc::now();
        let severity = detection_log.severity;

        let summary_text = format!(
            "緊急検出: {} - カメラ {} で severity {} の検出\n検出理由: {}",
            now.format("%Y-%m-%d %H:%M:%S"),
            detection_log.camera_id,
            severity,
            detection_log.primary_event
        );

        let camera_id = detection_log
            .camera_lacis_id
            .clone()
            .unwrap_or_else(|| detection_log.camera_id.clone());

        let result = self
            .repository
            .insert(SummaryInsert {
                tid: tid.to_string(),
                fid: fid.to_string(),
                summary_type: SummaryType::Emergency,
                period_start: detection_log.captured_at,
                period_end: detection_log.captured_at,
                summary_text,
                summary_json: None,
                detection_count: 1,
                severity_max: severity,
                camera_ids: serde_json::to_value(vec![camera_id])
                    .map_err(|e| crate::Error::Parse(e.to_string()))?,
                expires_at: Utc::now() + Duration::days(90),
            })
            .await?;

        warn!(
            summary_id = result.summary_id,
            detection_id = detection_log.log_id.unwrap_or(0),
            severity = severity,
            "Emergency summary generated"
        );

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_camera_breakdown_empty() {
        let generator = SummaryGenerator {
            detection_log_service: todo!(),
            camera_context_service: todo!(),
            repository: todo!(),
            config_store: todo!(),
        };

        // このテストはモック化が必要なため、実際のテストは統合テストで行う
    }
}
