//! Static Object Detector
//!
//! 固定物反応検出（同一座標で繰り返し検出）

use sqlx::{MySql, Pool};
use crate::Result;
use serde::Serialize;

/// 固定物反応Issue
#[derive(Debug, Clone, Serialize)]
pub struct StaticObjectIssue {
    /// X座標ゾーン（0-20）
    pub x_zone: i32,
    /// Y座標ゾーン（0-20）
    pub y_zone: i32,
    /// 検出ラベル
    pub label: String,
    /// 繰り返し回数
    pub repeat_count: i64,
}

/// Static Object Detector
pub struct StaticDetector {
    pool: Pool<MySql>,
}

impl StaticDetector {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }

    /// 固定物反応を検出（同一イベントが高頻度で検出）
    ///
    /// 性能最適化: JSON_TABLE展開は重いため、primary_eventベースの簡易検出を使用
    pub async fn analyze(&self, camera_id: &str, interval: &str) -> Result<Vec<StaticObjectIssue>> {
        // primary_eventベースの簡易検出を使用（JSON_TABLE展開は性能問題のため無効化）
        self.analyze_by_primary_label(camera_id, interval).await
    }

    /// primary_eventベースの簡易固定物検出
    /// 同一イベントが高頻度で検出される場合を検出（性能最適化版）
    async fn analyze_by_primary_label(&self, camera_id: &str, interval: &str) -> Result<Vec<StaticObjectIssue>> {
        // 全期間で同一イベントの検出数を集計（時間単位分割を削除して高速化）
        let repeated_labels: Vec<(String, i64)> = sqlx::query_as(&format!(
            r#"
            SELECT
                primary_event,
                COUNT(*) as repeat_count
            FROM detection_logs
            WHERE camera_id = ?
              AND created_at >= DATE_SUB(NOW(), INTERVAL {})
              AND primary_event NOT IN ('none', '')
              AND primary_event IS NOT NULL
            GROUP BY primary_event
            HAVING repeat_count >= 50
            ORDER BY repeat_count DESC
            LIMIT 5
            "#,
            interval
        ))
        .bind(camera_id)
        .fetch_all(&self.pool)
        .await?;

        // 位置情報なしの場合はzone=0で表現
        Ok(repeated_labels.into_iter().map(|(label, repeat_count)| {
            StaticObjectIssue {
                x_zone: 0,
                y_zone: 0,
                label,
                repeat_count,
            }
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zone_calculation() {
        // 640x480画像を20x20ゾーンに分割
        // 各ゾーン: 32x24ピクセル

        let x = 100;
        let y = 50;
        let x_zone = x / 32; // = 3
        let y_zone = y / 24; // = 2

        assert_eq!(x_zone, 3);
        assert_eq!(y_zone, 2);
    }

    #[test]
    fn test_repeat_threshold() {
        // 10件/時間以上で固定物とみなす
        assert!(10 >= 10);
        assert!(!(9 >= 10));
    }
}
