//! Auto Attunement Service
//!
//! Issue #106: AIEventlog.md 46-47行目要件
//! - autoAttunement: LLMコンテキスト連携による自動閾値調整
//!
//! 統計ベースの信頼度閾値自動調整サービス
//! - 誤検知率に基づく閾値調整
//! - カメラ別の最適閾値計算
//! - 閾値変更履歴の記録

use sqlx::{MySql, Pool};
use std::sync::Arc;
use crate::inference_stats_service::InferenceStatsService;
use crate::Result;

/// 閾値調整計算結果
#[derive(Debug, Clone, serde::Serialize)]
pub struct AttunementResult {
    pub camera_id: String,
    pub current_threshold: f32,
    pub recommended_threshold: f32,
    pub adjustment: f32,
    pub reason: String,
    pub false_positive_rate: f32,
    pub total_detections: i64,
    pub total_feedbacks: i64,
}

/// カメラ別統計データ
#[derive(Debug, Clone)]
pub struct CameraAttunementStats {
    pub camera_id: String,
    pub total_inferences: i64,
    pub detection_count: i64,
    pub unknown_count: i64,
    pub avg_confidence: f64,
    pub current_threshold: f32,
}

/// 誤検知報告データ
#[derive(Debug, Clone)]
pub struct FeedbackStats {
    pub camera_id: String,
    pub total_feedbacks: i64,
    pub false_positive_count: i64,
    pub false_negative_count: i64,
}

/// Auto Attunement Service
pub struct AutoAttunementService {
    pool: Pool<MySql>,
    inference_stats: Arc<InferenceStatsService>,
}

impl AutoAttunementService {
    pub fn new(pool: Pool<MySql>, inference_stats: Arc<InferenceStatsService>) -> Self {
        Self {
            pool,
            inference_stats,
        }
    }

    /// カメラ別統計を取得（過去7日）
    pub async fn get_camera_stats(&self, camera_id: &str) -> Result<CameraAttunementStats> {
        // detection_logsから統計を取得
        let stats: Option<(i64, i64, i64, f64)> = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total_inferences,
                CAST(COALESCE(SUM(CASE WHEN primary_event != 'none' THEN 1 ELSE 0 END), 0) AS SIGNED) as detection_count,
                CAST(COALESCE(SUM(CASE WHEN primary_event = 'unknown' THEN 1 ELSE 0 END), 0) AS SIGNED) as unknown_count,
                CAST(COALESCE(AVG(confidence), 0) AS DOUBLE) as avg_confidence
            FROM detection_logs
            WHERE camera_id = ?
              AND created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY)
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await?;

        let (total, detection, unknown, avg_conf) = stats.unwrap_or((0, 0, 0, 0.0));

        // camera_thresholdsから現在の閾値を取得
        let threshold: Option<(f32,)> = sqlx::query_as(
            "SELECT conf_threshold FROM camera_thresholds WHERE camera_id = ?",
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await?;

        let current_threshold = threshold.map(|t| t.0).unwrap_or(0.5);

        Ok(CameraAttunementStats {
            camera_id: camera_id.to_string(),
            total_inferences: total,
            detection_count: detection,
            unknown_count: unknown,
            avg_confidence: avg_conf,
            current_threshold,
        })
    }

    /// 誤検知報告統計を取得（過去7日）
    pub async fn get_feedback_stats(&self, camera_id: &str) -> Result<FeedbackStats> {
        let stats: Option<(i64, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total_feedbacks,
                CAST(COALESCE(SUM(CASE WHEN correct_label = 'none' AND reported_label != 'none' THEN 1 ELSE 0 END), 0) AS SIGNED) as false_positive,
                CAST(COALESCE(SUM(CASE WHEN correct_label != 'none' AND reported_label = 'none' THEN 1 ELSE 0 END), 0) AS SIGNED) as false_negative
            FROM misdetection_feedbacks
            WHERE camera_id = ?
              AND created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY)
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await?;

        let (total, fp, fn_count) = stats.unwrap_or((0, 0, 0));

        Ok(FeedbackStats {
            camera_id: camera_id.to_string(),
            total_feedbacks: total,
            false_positive_count: fp,
            false_negative_count: fn_count,
        })
    }

    /// 最適閾値を計算
    pub async fn calculate_optimal_threshold(&self, camera_id: &str) -> Result<AttunementResult> {
        let stats = self.get_camera_stats(camera_id).await?;
        let feedbacks = self.get_feedback_stats(camera_id).await?;

        let current = stats.current_threshold;

        // 誤検知率を計算
        let false_positive_rate = if stats.detection_count > 0 {
            feedbacks.false_positive_count as f32 / stats.detection_count as f32
        } else {
            0.0
        };

        // unknown率を計算
        let unknown_rate = if stats.total_inferences > 0 {
            stats.unknown_count as f32 / stats.total_inferences as f32
        } else {
            0.0
        };

        // 閾値調整ロジック
        // - 誤検知率 > 10% → 閾値を+0.05上げる（検出を厳しく）
        // - 誤検知率 < 1% かつ unknown率 > 10% → 閾値を-0.02下げる（検出を緩く）
        // - それ以外 → 維持
        let (adjustment, reason) = if false_positive_rate > 0.1 {
            (0.05, format!("誤検知率が高い({:.1}%) - 閾値を上げて誤検知を抑制", false_positive_rate * 100.0))
        } else if false_positive_rate < 0.01 && unknown_rate > 0.1 {
            (-0.02, format!("誤検知率が低く({:.1}%)、unknown率が高い({:.1}%) - 閾値を下げて検出感度を上げる",
                false_positive_rate * 100.0, unknown_rate * 100.0))
        } else if feedbacks.total_feedbacks == 0 && stats.total_inferences > 100 {
            (0.0, "フィードバックがないため現状維持を推奨".to_string())
        } else {
            (0.0, format!("誤検知率({:.1}%)は適正範囲内 - 現状維持", false_positive_rate * 100.0))
        };

        // 閾値を0.2-0.8の範囲に制限
        let recommended = (current + adjustment).clamp(0.2, 0.8);

        Ok(AttunementResult {
            camera_id: camera_id.to_string(),
            current_threshold: current,
            recommended_threshold: recommended,
            adjustment,
            reason,
            false_positive_rate,
            total_detections: stats.detection_count,
            total_feedbacks: feedbacks.total_feedbacks,
        })
    }

    /// 閾値を適用
    pub async fn apply_threshold(&self, camera_id: &str, new_threshold: f32) -> Result<()> {
        // 範囲チェック
        let new_threshold = new_threshold.clamp(0.2, 0.8);

        // 現在の閾値を取得
        let old_threshold: Option<(f32,)> = sqlx::query_as(
            "SELECT conf_threshold FROM camera_thresholds WHERE camera_id = ?",
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await?;

        // camera_thresholdsを更新（UPSERT）
        sqlx::query(
            r#"
            INSERT INTO camera_thresholds (camera_id, conf_threshold, updated_at)
            VALUES (?, ?, NOW(3))
            ON DUPLICATE KEY UPDATE
                conf_threshold = VALUES(conf_threshold),
                updated_at = NOW(3)
            "#,
        )
        .bind(camera_id)
        .bind(new_threshold)
        .execute(&self.pool)
        .await?;

        // 変更履歴を記録
        sqlx::query(
            r#"
            INSERT INTO threshold_change_history (camera_id, old_threshold, new_threshold, change_reason, created_at)
            VALUES (?, ?, ?, 'auto_adjust', NOW(3))
            "#,
        )
        .bind(camera_id)
        .bind(old_threshold.map(|t| t.0))
        .bind(new_threshold)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            camera_id = %camera_id,
            old = ?old_threshold.map(|t| t.0),
            new = %new_threshold,
            "Threshold updated via auto-attunement"
        );

        Ok(())
    }

    /// 全カメラの調整状態を取得
    pub async fn get_all_attunement_status(&self) -> Result<Vec<AttunementResult>> {
        // アクティブなカメラIDを取得
        let camera_ids: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT camera_id
            FROM detection_logs
            WHERE created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY)
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for (camera_id,) in camera_ids {
            match self.calculate_optimal_threshold(&camera_id).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!(camera_id = %camera_id, error = %e, "Failed to calculate attunement");
                }
            }
        }

        Ok(results)
    }

    /// 自動調整が有効なカメラに対して閾値を自動適用
    pub async fn run_auto_adjustment(&self) -> Result<Vec<AttunementResult>> {
        // auto_adjust_enabled = true のカメラを取得
        let enabled_cameras: Vec<(String,)> = sqlx::query_as(
            "SELECT camera_id FROM camera_thresholds WHERE auto_adjust_enabled = TRUE",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for (camera_id,) in enabled_cameras {
            match self.calculate_optimal_threshold(&camera_id).await {
                Ok(result) => {
                    // 調整が必要な場合のみ適用
                    if result.adjustment.abs() > 0.001 {
                        if let Err(e) = self.apply_threshold(&camera_id, result.recommended_threshold).await {
                            tracing::error!(camera_id = %camera_id, error = %e, "Failed to apply threshold");
                        } else {
                            tracing::info!(
                                camera_id = %camera_id,
                                old = %result.current_threshold,
                                new = %result.recommended_threshold,
                                "Auto-adjusted threshold"
                            );
                        }
                    }
                    results.push(result);
                }
                Err(e) => {
                    tracing::warn!(camera_id = %camera_id, error = %e, "Failed to calculate attunement");
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_clamp() {
        // 閾値は0.2-0.8の範囲に制限される
        assert_eq!(0.5_f32.clamp(0.2, 0.8), 0.5);
        assert_eq!(0.1_f32.clamp(0.2, 0.8), 0.2);
        assert_eq!(0.9_f32.clamp(0.2, 0.8), 0.8);
    }

    #[test]
    fn test_false_positive_rate_calculation() {
        // 検出100件中、誤検知10件 → 10%
        let detection_count = 100;
        let false_positive_count = 10;
        let rate = false_positive_count as f32 / detection_count as f32;
        assert!((rate - 0.1).abs() < 0.001);
    }
}
