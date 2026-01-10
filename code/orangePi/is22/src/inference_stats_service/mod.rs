//! InferenceStatsService - 推論統計・分析サービス
//!
//! ## Responsibilities
//!
//! - カメラ別検出分布の集計
//! - 時系列傾向分析
//! - プリセット効果分析
//! - 異常カメラ検出
//! - ストレージ使用量モニタリング
//!
//! ## Design Reference
//!
//! - InferenceStatistics_Design.md (Issue #106)
//! - AIEventlog.md 41-47行目要件

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPool;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::Result;

/// 統計期間
#[derive(Debug, Clone, Copy, Deserialize)]
pub enum StatsPeriod {
    #[serde(rename = "24h")]
    Hours24,
    #[serde(rename = "7d")]
    Days7,
    #[serde(rename = "30d")]
    Days30,
}

impl StatsPeriod {
    pub fn to_hours(&self) -> i64 {
        match self {
            StatsPeriod::Hours24 => 24,
            StatsPeriod::Days7 => 24 * 7,
            StatsPeriod::Days30 => 24 * 30,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            StatsPeriod::Hours24 => "24h",
            StatsPeriod::Days7 => "7d",
            StatsPeriod::Days30 => "30d",
        }
    }
}

impl Default for StatsPeriod {
    fn default() -> Self {
        StatsPeriod::Hours24
    }
}

/// カメラ別統計
#[derive(Debug, Clone, Serialize)]
pub struct CameraStats {
    pub camera_id: String,
    pub camera_name: Option<String>,
    pub preset_id: String,
    pub total: i64,
    pub by_event: HashMap<String, i64>,
    pub avg_confidence: f64,
    /// 異常スコア（標準偏差ベース）
    pub anomaly_score: f64,
    /// 異常理由
    pub anomaly_reason: Option<String>,
}

/// カメラ別分布レスポンス
#[derive(Debug, Clone, Serialize)]
pub struct CameraDistributionResponse {
    pub period: String,
    pub total_inferences: i64,
    pub cameras: Vec<CameraStats>,
}

/// イベント傾向データポイント
#[derive(Debug, Clone, Serialize)]
pub struct TrendPoint {
    pub hour: String,  // ISO 8601 datetime
    pub human: i64,
    pub vehicle: i64,
    pub unknown: i64,
    pub none: i64,
    pub other: i64,
}

/// イベント傾向レスポンス
#[derive(Debug, Clone, Serialize)]
pub struct EventTrendsResponse {
    pub period: String,
    pub data: Vec<TrendPoint>,
}

/// プリセット効果統計
#[derive(Debug, Clone, Serialize)]
pub struct PresetStats {
    pub preset_id: String,
    pub camera_count: i64,
    pub total_inferences: i64,
    /// 検出率 (none以外の割合)
    pub detection_rate: f64,
    /// 誤検知率 (フィードバックベース)
    pub false_positive_rate: f64,
    pub avg_confidence: f64,
}

/// プリセット効果レスポンス
#[derive(Debug, Clone, Serialize)]
pub struct PresetEffectivenessResponse {
    pub period: String,
    pub presets: Vec<PresetStats>,
}

/// ストレージ統計
#[derive(Debug, Clone, Serialize)]
pub struct StorageStats {
    /// 使用中バイト数
    pub used_bytes: u64,
    /// 最大容量バイト数
    pub quota_bytes: u64,
    /// 使用率 (0.0-1.0)
    pub usage_ratio: f64,
    /// カメラ別使用量
    pub by_camera: Vec<CameraStorageStats>,
    /// ログ総数
    pub total_logs: i64,
    /// 画像総数
    pub total_images: i64,
}

/// カメラ別ストレージ統計
#[derive(Debug, Clone, Serialize)]
pub struct CameraStorageStats {
    pub camera_id: String,
    pub camera_name: Option<String>,
    pub image_count: i64,
    pub estimated_bytes: u64,
}

/// 異常カメラ情報
#[derive(Debug, Clone, Serialize)]
pub struct AnomalyCamera {
    pub camera_id: String,
    pub camera_name: Option<String>,
    pub anomaly_type: String,
    pub anomaly_score: f64,
    pub details: String,
    /// 推奨アクション
    pub recommendation: String,
}

/// 異常検出レスポンス
#[derive(Debug, Clone, Serialize)]
pub struct AnomaliesResponse {
    pub period: String,
    pub anomalies: Vec<AnomalyCamera>,
}

/// InferenceStatsService
pub struct InferenceStatsService {
    pool: MySqlPool,
}

impl InferenceStatsService {
    /// Create new InferenceStatsService
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Get pool reference for direct queries
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    // ========================================
    // T1-1: カメラ別分布統計
    // ========================================

    /// GET /api/stats/cameras - カメラ別分布統計
    pub async fn get_camera_distribution(
        &self,
        period: StatsPeriod,
        camera_id: Option<&str>,
    ) -> Result<CameraDistributionResponse> {
        let hours = period.to_hours();
        let since = Utc::now() - Duration::hours(hours);

        // 総推論回数を取得
        let total_inferences: i64 = if let Some(cam_id) = camera_id {
            sqlx::query_scalar(
                r#"SELECT COUNT(*) FROM detection_logs
                   WHERE captured_at >= ? AND camera_id = ?"#
            )
            .bind(since)
            .bind(cam_id)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM detection_logs WHERE captured_at >= ?"
            )
            .bind(since)
            .fetch_one(&self.pool)
            .await?
        };

        // カメラ別統計を取得
        let base_query = if camera_id.is_some() {
            r#"SELECT
                dl.camera_id,
                c.name as camera_name,
                COALESCE(dl.preset_id, 'balanced') as preset_id,
                COUNT(*) as total,
                CAST(SUM(CASE WHEN dl.primary_event = 'none' THEN 1 ELSE 0 END) AS SIGNED) as none_count,
                CAST(SUM(CASE WHEN dl.primary_event = 'human' THEN 1 ELSE 0 END) AS SIGNED) as human_count,
                CAST(SUM(CASE WHEN dl.primary_event = 'vehicle' THEN 1 ELSE 0 END) AS SIGNED) as vehicle_count,
                CAST(SUM(CASE WHEN dl.primary_event = 'unknown' THEN 1 ELSE 0 END) AS SIGNED) as unknown_count,
                CAST(SUM(CASE WHEN dl.primary_event = 'animal' THEN 1 ELSE 0 END) AS SIGNED) as animal_count,
                CAST(SUM(CASE WHEN dl.primary_event NOT IN ('none', 'human', 'vehicle', 'unknown', 'animal') THEN 1 ELSE 0 END) AS SIGNED) as other_count,
                CAST(COALESCE(AVG(dl.confidence), 0) AS DOUBLE) as avg_confidence
            FROM detection_logs dl
            LEFT JOIN cameras c ON dl.camera_id = c.camera_id
            WHERE dl.captured_at >= ? AND dl.camera_id = ?
            GROUP BY dl.camera_id, c.name, dl.preset_id
            ORDER BY total DESC"#
        } else {
            r#"SELECT
                dl.camera_id,
                c.name as camera_name,
                COALESCE(dl.preset_id, 'balanced') as preset_id,
                COUNT(*) as total,
                CAST(SUM(CASE WHEN dl.primary_event = 'none' THEN 1 ELSE 0 END) AS SIGNED) as none_count,
                CAST(SUM(CASE WHEN dl.primary_event = 'human' THEN 1 ELSE 0 END) AS SIGNED) as human_count,
                CAST(SUM(CASE WHEN dl.primary_event = 'vehicle' THEN 1 ELSE 0 END) AS SIGNED) as vehicle_count,
                CAST(SUM(CASE WHEN dl.primary_event = 'unknown' THEN 1 ELSE 0 END) AS SIGNED) as unknown_count,
                CAST(SUM(CASE WHEN dl.primary_event = 'animal' THEN 1 ELSE 0 END) AS SIGNED) as animal_count,
                CAST(SUM(CASE WHEN dl.primary_event NOT IN ('none', 'human', 'vehicle', 'unknown', 'animal') THEN 1 ELSE 0 END) AS SIGNED) as other_count,
                CAST(COALESCE(AVG(dl.confidence), 0) AS DOUBLE) as avg_confidence
            FROM detection_logs dl
            LEFT JOIN cameras c ON dl.camera_id = c.camera_id
            WHERE dl.captured_at >= ?
            GROUP BY dl.camera_id, c.name, dl.preset_id
            ORDER BY total DESC
            LIMIT 50"#
        };

        let rows = if let Some(cam_id) = camera_id {
            sqlx::query(base_query)
                .bind(since)
                .bind(cam_id)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(base_query)
                .bind(since)
                .fetch_all(&self.pool)
                .await?
        };

        // 異常スコア計算用: 全カメラのunknown率の平均と標準偏差を計算
        let mut unknown_rates: Vec<f64> = Vec::new();
        let mut camera_stats_raw: Vec<(String, Option<String>, String, i64, HashMap<String, i64>, f64, f64)> = Vec::new();

        for row in &rows {
            let total: i64 = row.get("total");
            let unknown_count: i64 = row.get("unknown_count");
            let unknown_rate = if total > 0 { unknown_count as f64 / total as f64 } else { 0.0 };
            unknown_rates.push(unknown_rate);

            let mut by_event = HashMap::new();
            by_event.insert("none".to_string(), row.get("none_count"));
            by_event.insert("human".to_string(), row.get("human_count"));
            by_event.insert("vehicle".to_string(), row.get("vehicle_count"));
            by_event.insert("unknown".to_string(), row.get("unknown_count"));
            by_event.insert("animal".to_string(), row.get("animal_count"));
            by_event.insert("other".to_string(), row.get("other_count"));

            camera_stats_raw.push((
                row.get("camera_id"),
                row.try_get("camera_name").ok(),
                row.get("preset_id"),
                total,
                by_event,
                row.get("avg_confidence"),
                unknown_rate,
            ));
        }

        // 平均と標準偏差を計算
        let mean_unknown_rate = if !unknown_rates.is_empty() {
            unknown_rates.iter().sum::<f64>() / unknown_rates.len() as f64
        } else {
            0.0
        };

        let std_dev = if unknown_rates.len() > 1 {
            let variance: f64 = unknown_rates.iter()
                .map(|r| (r - mean_unknown_rate).powi(2))
                .sum::<f64>() / unknown_rates.len() as f64;
            variance.sqrt()
        } else {
            0.0
        };

        // CameraStatsを構築
        let cameras: Vec<CameraStats> = camera_stats_raw
            .into_iter()
            .map(|(camera_id, camera_name, preset_id, total, by_event, avg_confidence, unknown_rate)| {
                // 異常スコア = (unknown率 - 平均) / 標準偏差
                let anomaly_score = if std_dev > 0.0 {
                    (unknown_rate - mean_unknown_rate) / std_dev
                } else {
                    0.0
                };

                // 異常判定 (2σ以上で異常)
                let anomaly_reason = if anomaly_score > 2.0 {
                    Some(format!(
                        "unknown発生率が平均の{:.1}倍",
                        if mean_unknown_rate > 0.0 { unknown_rate / mean_unknown_rate } else { 0.0 }
                    ))
                } else {
                    None
                };

                CameraStats {
                    camera_id,
                    camera_name,
                    preset_id,
                    total,
                    by_event,
                    avg_confidence,
                    anomaly_score: (anomaly_score * 10.0).round() / 10.0, // 小数点1桁
                    anomaly_reason,
                }
            })
            .collect();

        Ok(CameraDistributionResponse {
            period: period.as_str().to_string(),
            total_inferences,
            cameras,
        })
    }

    // ========================================
    // T1-2: イベント傾向分析
    // ========================================

    /// GET /api/stats/events - 時系列傾向分析
    pub async fn get_event_trends(&self, period: StatsPeriod) -> Result<EventTrendsResponse> {
        let hours = period.to_hours();
        let since = Utc::now() - Duration::hours(hours);

        // 1時間ごとの集計
        let rows = sqlx::query(
            r#"SELECT
                DATE_FORMAT(captured_at, '%Y-%m-%d %H:00:00') as hour_bucket,
                CAST(SUM(CASE WHEN primary_event = 'human' THEN 1 ELSE 0 END) AS SIGNED) as human_count,
                CAST(SUM(CASE WHEN primary_event = 'vehicle' THEN 1 ELSE 0 END) AS SIGNED) as vehicle_count,
                CAST(SUM(CASE WHEN primary_event = 'unknown' THEN 1 ELSE 0 END) AS SIGNED) as unknown_count,
                CAST(SUM(CASE WHEN primary_event = 'none' THEN 1 ELSE 0 END) AS SIGNED) as none_count,
                CAST(SUM(CASE WHEN primary_event NOT IN ('human', 'vehicle', 'unknown', 'none') THEN 1 ELSE 0 END) AS SIGNED) as other_count
            FROM detection_logs
            WHERE captured_at >= ?
            GROUP BY hour_bucket
            ORDER BY hour_bucket ASC"#
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        let data: Vec<TrendPoint> = rows
            .into_iter()
            .map(|row| TrendPoint {
                hour: row.get("hour_bucket"),
                human: row.get("human_count"),
                vehicle: row.get("vehicle_count"),
                unknown: row.get("unknown_count"),
                none: row.get("none_count"),
                other: row.get("other_count"),
            })
            .collect();

        Ok(EventTrendsResponse {
            period: period.as_str().to_string(),
            data,
        })
    }

    // ========================================
    // T1-3: プリセット効果分析
    // ========================================

    /// GET /api/stats/presets - プリセット効果分析
    pub async fn get_preset_effectiveness(&self, period: StatsPeriod) -> Result<PresetEffectivenessResponse> {
        let hours = period.to_hours();
        let since = Utc::now() - Duration::hours(hours);

        // プリセット別統計
        let rows = sqlx::query(
            r#"SELECT
                COALESCE(dl.preset_id, 'balanced') as preset_id,
                COUNT(DISTINCT dl.camera_id) as camera_count,
                COUNT(*) as total_inferences,
                CAST(SUM(CASE WHEN dl.primary_event != 'none' THEN 1 ELSE 0 END) AS SIGNED) as detection_count,
                CAST(COALESCE(AVG(dl.confidence), 0) AS DOUBLE) as avg_confidence
            FROM detection_logs dl
            WHERE dl.captured_at >= ?
            GROUP BY preset_id
            ORDER BY total_inferences DESC"#
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        // 誤検知フィードバック数を取得
        let feedback_rows = sqlx::query(
            r#"SELECT
                COALESCE(dl.preset_id, 'balanced') as preset_id,
                COUNT(mf.feedback_id) as feedback_count
            FROM misdetection_feedbacks mf
            JOIN detection_logs dl ON mf.log_id = dl.log_id
            WHERE mf.created_at >= ?
            GROUP BY preset_id"#
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let mut feedback_map: HashMap<String, i64> = HashMap::new();
        for row in feedback_rows {
            let preset_id: String = row.get("preset_id");
            let count: i64 = row.get("feedback_count");
            feedback_map.insert(preset_id, count);
        }

        let presets: Vec<PresetStats> = rows
            .into_iter()
            .map(|row| {
                let preset_id: String = row.get("preset_id");
                let total_inferences: i64 = row.get("total_inferences");
                let detection_count: i64 = row.get("detection_count");
                let feedback_count = feedback_map.get(&preset_id).copied().unwrap_or(0);

                let detection_rate = if total_inferences > 0 {
                    detection_count as f64 / total_inferences as f64
                } else {
                    0.0
                };

                let false_positive_rate = if detection_count > 0 {
                    feedback_count as f64 / detection_count as f64
                } else {
                    0.0
                };

                PresetStats {
                    preset_id,
                    camera_count: row.get("camera_count"),
                    total_inferences,
                    detection_rate: (detection_rate * 1000.0).round() / 1000.0, // 小数点3桁
                    false_positive_rate: (false_positive_rate * 1000.0).round() / 1000.0,
                    avg_confidence: row.get("avg_confidence"),
                }
            })
            .collect();

        Ok(PresetEffectivenessResponse {
            period: period.as_str().to_string(),
            presets,
        })
    }

    // ========================================
    // T1-4: ストレージ統計
    // ========================================

    /// GET /api/stats/storage - ストレージ使用状況
    pub async fn get_storage_stats(&self, quota_bytes: u64) -> Result<StorageStats> {
        // ログ総数と画像数
        let (total_logs, total_images): (i64, i64) = sqlx::query_as(
            r#"SELECT
                COUNT(*) as total_logs,
                SUM(CASE WHEN image_path_local != '' THEN 1 ELSE 0 END) as total_images
            FROM detection_logs"#
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or((0, 0));

        // カメラ別画像数
        let camera_rows = sqlx::query(
            r#"SELECT
                dl.camera_id,
                c.name as camera_name,
                COUNT(*) as image_count
            FROM detection_logs dl
            LEFT JOIN cameras c ON dl.camera_id = c.camera_id
            WHERE dl.image_path_local != ''
            GROUP BY dl.camera_id, c.name
            ORDER BY image_count DESC"#
        )
        .fetch_all(&self.pool)
        .await?;

        let by_camera: Vec<CameraStorageStats> = camera_rows
            .into_iter()
            .map(|row| {
                let image_count: i64 = row.get("image_count");
                // 推定サイズ (平均100KB/画像)
                let estimated_bytes = (image_count as u64) * 100 * 1024;

                CameraStorageStats {
                    camera_id: row.get("camera_id"),
                    camera_name: row.try_get("camera_name").ok(),
                    image_count,
                    estimated_bytes,
                }
            })
            .collect();

        // 合計推定使用量
        let used_bytes: u64 = by_camera.iter().map(|c| c.estimated_bytes).sum();
        let usage_ratio = if quota_bytes > 0 {
            used_bytes as f64 / quota_bytes as f64
        } else {
            0.0
        };

        Ok(StorageStats {
            used_bytes,
            quota_bytes,
            usage_ratio: (usage_ratio * 100.0).round() / 100.0,
            by_camera,
            total_logs,
            total_images,
        })
    }

    // ========================================
    // T1-5: 異常カメラ検出
    // ========================================

    /// GET /api/stats/anomalies - 異常カメラ検出
    pub async fn get_anomaly_cameras(&self, period: StatsPeriod) -> Result<AnomaliesResponse> {
        let hours = period.to_hours();
        let since = Utc::now() - Duration::hours(hours);

        // カメラ別のunknown率を計算
        let rows = sqlx::query(
            r#"SELECT
                dl.camera_id,
                c.name as camera_name,
                COUNT(*) as total,
                CAST(SUM(CASE WHEN dl.primary_event = 'unknown' THEN 1 ELSE 0 END) AS SIGNED) as unknown_count,
                CAST(SUM(CASE WHEN dl.primary_event = 'human' THEN 1 ELSE 0 END) AS SIGNED) as human_count,
                CAST(SUM(CASE WHEN dl.primary_event != 'none' THEN 1 ELSE 0 END) AS SIGNED) as detection_count,
                CAST(COALESCE(AVG(dl.confidence), 0) AS DOUBLE) as avg_confidence
            FROM detection_logs dl
            LEFT JOIN cameras c ON dl.camera_id = c.camera_id
            WHERE dl.captured_at >= ?
            GROUP BY dl.camera_id, c.name
            HAVING total >= 10"#  // 10件以上のデータがあるカメラのみ
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        // 統計値計算
        let mut unknown_rates: Vec<f64> = Vec::new();
        let mut detection_rates: Vec<f64> = Vec::new();

        for row in &rows {
            let total: i64 = row.get("total");
            let unknown_count: i64 = row.get("unknown_count");
            let detection_count: i64 = row.get("detection_count");

            if total > 0 {
                unknown_rates.push(unknown_count as f64 / total as f64);
                detection_rates.push(detection_count as f64 / total as f64);
            }
        }

        let mean_unknown = if !unknown_rates.is_empty() {
            unknown_rates.iter().sum::<f64>() / unknown_rates.len() as f64
        } else { 0.0 };

        let mean_detection = if !detection_rates.is_empty() {
            detection_rates.iter().sum::<f64>() / detection_rates.len() as f64
        } else { 0.0 };

        let std_unknown = calculate_std_dev(&unknown_rates, mean_unknown);
        let std_detection = calculate_std_dev(&detection_rates, mean_detection);

        // 異常検出
        let mut anomalies: Vec<AnomalyCamera> = Vec::new();

        for row in rows {
            let camera_id: String = row.get("camera_id");
            let camera_name: Option<String> = row.try_get("camera_name").ok();
            let total: i64 = row.get("total");
            let unknown_count: i64 = row.get("unknown_count");
            let detection_count: i64 = row.get("detection_count");
            let avg_confidence: f64 = row.get("avg_confidence");

            let unknown_rate = unknown_count as f64 / total as f64;
            let detection_rate = detection_count as f64 / total as f64;

            // 異常判定ルール
            // 1. unknown率が平均 + 2σ以上
            let unknown_z_score = if std_unknown > 0.0 {
                (unknown_rate - mean_unknown) / std_unknown
            } else { 0.0 };

            if unknown_z_score > 2.0 {
                anomalies.push(AnomalyCamera {
                    camera_id: camera_id.clone(),
                    camera_name: camera_name.clone(),
                    anomaly_type: "high_unknown_rate".to_string(),
                    anomaly_score: (unknown_z_score * 10.0).round() / 10.0,
                    details: format!(
                        "unknown率: {:.1}% (平均: {:.1}%)",
                        unknown_rate * 100.0,
                        mean_unknown * 100.0
                    ),
                    recommendation: "excluded_objectsの設定追加を検討してください".to_string(),
                });
            }

            // 2. 検出率が異常に高い (平均 + 2σ以上で検出率50%以上)
            let detection_z_score = if std_detection > 0.0 {
                (detection_rate - mean_detection) / std_detection
            } else { 0.0 };

            if detection_z_score > 2.0 && detection_rate > 0.5 {
                anomalies.push(AnomalyCamera {
                    camera_id: camera_id.clone(),
                    camera_name: camera_name.clone(),
                    anomaly_type: "high_detection_rate".to_string(),
                    anomaly_score: (detection_z_score * 10.0).round() / 10.0,
                    details: format!(
                        "検出率: {:.1}% (平均: {:.1}%)",
                        detection_rate * 100.0,
                        mean_detection * 100.0
                    ),
                    recommendation: "conf_overrideを上げて誤検知を減らすことを検討してください".to_string(),
                });
            }

            // 3. 信頼度が異常に低い (0.4未満)
            if avg_confidence < 0.4 && total > 50 {
                anomalies.push(AnomalyCamera {
                    camera_id: camera_id.clone(),
                    camera_name: camera_name.clone(),
                    anomaly_type: "low_confidence".to_string(),
                    anomaly_score: (1.0 - avg_confidence) * 10.0,
                    details: format!("平均信頼度: {:.2}", avg_confidence),
                    recommendation: "カメラの画質・設置位置を確認してください".to_string(),
                });
            }
        }

        // 異常スコアでソート (高い順)
        anomalies.sort_by(|a, b| b.anomaly_score.partial_cmp(&a.anomaly_score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(AnomaliesResponse {
            period: period.as_str().to_string(),
            anomalies,
        })
    }
}

/// 標準偏差を計算
fn calculate_std_dev(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let variance: f64 = values.iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_period_to_hours() {
        assert_eq!(StatsPeriod::Hours24.to_hours(), 24);
        assert_eq!(StatsPeriod::Days7.to_hours(), 168);
        assert_eq!(StatsPeriod::Days30.to_hours(), 720);
    }

    #[test]
    fn test_stats_period_as_str() {
        assert_eq!(StatsPeriod::Hours24.as_str(), "24h");
        assert_eq!(StatsPeriod::Days7.as_str(), "7d");
        assert_eq!(StatsPeriod::Days30.as_str(), "30d");
    }

    #[test]
    fn test_calculate_std_dev() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mean = 3.0;
        let std = calculate_std_dev(&values, mean);
        assert!((std - 1.414).abs() < 0.01);
    }

    #[test]
    fn test_calculate_std_dev_single_value() {
        let values = vec![5.0];
        assert_eq!(calculate_std_dev(&values, 5.0), 0.0);
    }

    #[test]
    fn test_calculate_std_dev_empty() {
        let values: Vec<f64> = vec![];
        assert_eq!(calculate_std_dev(&values, 0.0), 0.0);
    }
}
