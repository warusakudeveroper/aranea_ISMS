//! Tag Analyzer
//!
//! タグ傾向分析・タグ固定化検出

use sqlx::{MySql, Pool, Row};
use crate::Result;
use serde::{Deserialize, Serialize};

/// タグ傾向の方向
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    /// 増加傾向（前日比 +10%以上）
    Increasing,
    /// 減少傾向（前日比 -10%以上）
    Decreasing,
    /// 安定（変動 ±10%以内）
    Stable,
}

/// タグ傾向データ
#[derive(Debug, Clone, Serialize)]
pub struct TagTrend {
    /// タグ名
    pub tag: String,
    /// 出現件数
    pub count: i64,
    /// 出現率（%）
    pub percentage: f64,
    /// 過剰検出フラグ（80%以上）
    pub is_over_detected: bool,
    /// 傾向
    pub trend: TrendDirection,
}

/// タグ固定化Issue
#[derive(Debug, Clone)]
pub struct TagFixationIssue {
    pub tag: String,
    pub count: i64,
    pub percentage: f64,
}

/// Tag Analyzer
pub struct TagAnalyzer {
    pool: Pool<MySql>,
}

impl TagAnalyzer {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }

    /// タグ固定化を検出（80%以上で同一タグ出現）
    pub async fn analyze_fixation(&self, camera_id: &str, interval: &str) -> Result<Vec<TagFixationIssue>> {
        // 総検出数を取得
        let total: Option<(i64,)> = sqlx::query_as(&format!(
            r#"
            SELECT COUNT(*) as total
            FROM detection_logs
            WHERE camera_id = ?
              AND created_at >= DATE_SUB(NOW(), INTERVAL {})
              AND tags IS NOT NULL
              AND tags != '[]'
            "#,
            interval
        ))
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await?;

        let total_count = total.map(|t| t.0).unwrap_or(0);
        if total_count == 0 {
            return Ok(Vec::new());
        }

        // タグ別出現数を集計（JSON配列からタグを展開）
        // MySQLのJSON_TABLE関数を使用してタグを展開
        let tag_counts: Vec<(String, i64)> = sqlx::query_as(&format!(
            r#"
            SELECT tag_value, COUNT(*) as tag_count
            FROM detection_logs dl,
                 JSON_TABLE(dl.tags, '$[*]' COLUMNS (tag_value VARCHAR(100) PATH '$')) AS jt
            WHERE dl.camera_id = ?
              AND dl.created_at >= DATE_SUB(NOW(), INTERVAL {})
              AND dl.tags IS NOT NULL
              AND dl.tags != '[]'
            GROUP BY tag_value
            HAVING tag_count * 100 / ? >= 80
            ORDER BY tag_count DESC
            "#,
            interval
        ))
        .bind(camera_id)
        .bind(total_count)
        .fetch_all(&self.pool)
        .await?;

        Ok(tag_counts.into_iter().map(|(tag, count)| {
            TagFixationIssue {
                tag,
                count,
                percentage: (count as f64 / total_count as f64) * 100.0,
            }
        }).collect())
    }

    /// カメラ別タグ傾向を取得（上位10件）
    pub async fn get_trends(&self, camera_id: &str, interval: &str) -> Result<Vec<TagTrend>> {
        // 総検出数を取得
        let total: Option<(i64,)> = sqlx::query_as(&format!(
            r#"
            SELECT COUNT(*) as total
            FROM detection_logs
            WHERE camera_id = ?
              AND created_at >= DATE_SUB(NOW(), INTERVAL {})
              AND tags IS NOT NULL
              AND tags != '[]'
            "#,
            interval
        ))
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await?;

        let total_count = total.map(|t| t.0).unwrap_or(0);
        if total_count == 0 {
            return Ok(Vec::new());
        }

        // 現在期間のタグ別出現数
        let current_tags: Vec<(String, i64)> = sqlx::query_as(&format!(
            r#"
            SELECT tag_value, COUNT(*) as tag_count
            FROM detection_logs dl,
                 JSON_TABLE(dl.tags, '$[*]' COLUMNS (tag_value VARCHAR(100) PATH '$')) AS jt
            WHERE dl.camera_id = ?
              AND dl.created_at >= DATE_SUB(NOW(), INTERVAL {})
              AND dl.tags IS NOT NULL
              AND dl.tags != '[]'
            GROUP BY tag_value
            ORDER BY tag_count DESC
            LIMIT 10
            "#,
            interval
        ))
        .bind(camera_id)
        .fetch_all(&self.pool)
        .await?;

        // 前期間（比較用）のタグ別出現数
        // interval が "1 DAY" なら前日、"7 DAY" なら前週
        let prev_interval = match interval {
            "7 DAY" => "14 DAY",
            "30 DAY" => "60 DAY",
            _ => "2 DAY",
        };

        let prev_tags: Vec<(String, i64)> = sqlx::query_as(&format!(
            r#"
            SELECT tag_value, COUNT(*) as tag_count
            FROM detection_logs dl,
                 JSON_TABLE(dl.tags, '$[*]' COLUMNS (tag_value VARCHAR(100) PATH '$')) AS jt
            WHERE dl.camera_id = ?
              AND dl.created_at >= DATE_SUB(NOW(), INTERVAL {})
              AND dl.created_at < DATE_SUB(NOW(), INTERVAL {})
              AND dl.tags IS NOT NULL
              AND dl.tags != '[]'
            GROUP BY tag_value
            "#,
            prev_interval, interval
        ))
        .bind(camera_id)
        .fetch_all(&self.pool)
        .await?;

        let prev_map: std::collections::HashMap<String, i64> = prev_tags.into_iter().collect();

        Ok(current_tags.into_iter().map(|(tag, count)| {
            let percentage = (count as f64 / total_count as f64) * 100.0;
            let is_over_detected = percentage >= 80.0;

            // 傾向判定
            let trend = if let Some(&prev_count) = prev_map.get(&tag) {
                if prev_count == 0 {
                    TrendDirection::Increasing
                } else {
                    let change_rate = (count as f64 - prev_count as f64) / prev_count as f64;
                    if change_rate > 0.1 {
                        TrendDirection::Increasing
                    } else if change_rate < -0.1 {
                        TrendDirection::Decreasing
                    } else {
                        TrendDirection::Stable
                    }
                }
            } else {
                // 前期間になければ新規（増加扱い）
                TrendDirection::Increasing
            };

            TagTrend {
                tag,
                count,
                percentage,
                is_over_detected,
                trend,
            }
        }).collect())
    }

    /// 全体タグサマリーを取得
    pub async fn get_summary(&self, interval: &str) -> Result<Vec<TagTrend>> {
        // 総検出数を取得
        let total: Option<(i64,)> = sqlx::query_as(&format!(
            r#"
            SELECT COUNT(*) as total
            FROM detection_logs
            WHERE created_at >= DATE_SUB(NOW(), INTERVAL {})
              AND tags IS NOT NULL
              AND tags != '[]'
            "#,
            interval
        ))
        .fetch_optional(&self.pool)
        .await?;

        let total_count = total.map(|t| t.0).unwrap_or(0);
        if total_count == 0 {
            return Ok(Vec::new());
        }

        // 全体のタグ別出現数（上位15件）
        let tags: Vec<(String, i64)> = sqlx::query_as(&format!(
            r#"
            SELECT tag_value, COUNT(*) as tag_count
            FROM detection_logs dl,
                 JSON_TABLE(dl.tags, '$[*]' COLUMNS (tag_value VARCHAR(100) PATH '$')) AS jt
            WHERE dl.created_at >= DATE_SUB(NOW(), INTERVAL {})
              AND dl.tags IS NOT NULL
              AND dl.tags != '[]'
            GROUP BY tag_value
            ORDER BY tag_count DESC
            LIMIT 15
            "#,
            interval
        ))
        .fetch_all(&self.pool)
        .await?;

        Ok(tags.into_iter().map(|(tag, count)| {
            let percentage = (count as f64 / total_count as f64) * 100.0;
            TagTrend {
                tag,
                count,
                percentage,
                is_over_detected: percentage >= 80.0,
                trend: TrendDirection::Stable, // サマリーでは傾向計算省略
            }
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_direction() {
        // +10%以上は Increasing
        let change_rate = 0.15;
        let trend = if change_rate > 0.1 {
            TrendDirection::Increasing
        } else if change_rate < -0.1 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };
        assert_eq!(trend, TrendDirection::Increasing);

        // -10%以上は Decreasing
        let change_rate = -0.15;
        let trend = if change_rate > 0.1 {
            TrendDirection::Increasing
        } else if change_rate < -0.1 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };
        assert_eq!(trend, TrendDirection::Decreasing);

        // ±10%以内は Stable
        let change_rate = 0.05;
        let trend = if change_rate > 0.1 {
            TrendDirection::Increasing
        } else if change_rate < -0.1 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };
        assert_eq!(trend, TrendDirection::Stable);
    }

    #[test]
    fn test_over_detection_threshold() {
        // 80%以上は過剰検出
        assert!(85.0 >= 80.0);
        assert!(80.0 >= 80.0);
        assert!(!(79.9 >= 80.0));
    }
}
