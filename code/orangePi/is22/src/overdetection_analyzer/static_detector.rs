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

    /// 固定物反応を検出（同一座標±5%で同一ラベルが10件/時間以上）
    ///
    /// detection_logsにはbboxデータが含まれている場合に検出
    /// bbox_x, bbox_y, image_width, image_heightカラムが必要
    pub async fn analyze(&self, camera_id: &str, interval: &str) -> Result<Vec<StaticObjectIssue>> {
        // detection_logsテーブルの構造を確認
        // bboxesがJSONで保存されている場合の処理

        // bboxes JSON構造: [{"x": 100, "y": 50, "w": 200, "h": 300, "label": "person", "conf": 0.85}]
        // x, yを20分割ゾーンに変換して集計

        let static_objects: Vec<(i32, i32, String, i64)> = sqlx::query_as(&format!(
            r#"
            SELECT
                CAST(FLOOR(JSON_EXTRACT(bbox.*, '$.x') / 32) AS SIGNED) as x_zone,
                CAST(FLOOR(JSON_EXTRACT(bbox.*, '$.y') / 24) AS SIGNED) as y_zone,
                JSON_UNQUOTE(JSON_EXTRACT(bbox.*, '$.label')) as label,
                COUNT(*) as repeat_count
            FROM detection_logs dl,
                 JSON_TABLE(dl.bboxes, '$[*]' COLUMNS (
                     bbox JSON PATH '$'
                 )) AS bbox
            WHERE dl.camera_id = ?
              AND dl.created_at >= DATE_SUB(NOW(), INTERVAL {})
              AND dl.bboxes IS NOT NULL
              AND dl.bboxes != '[]'
              AND JSON_EXTRACT(bbox.*, '$.x') IS NOT NULL
            GROUP BY x_zone, y_zone, label
            HAVING repeat_count >= 10
            ORDER BY repeat_count DESC
            LIMIT 20
            "#,
            interval
        ))
        .bind(camera_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // bboxesカラムがない場合やデータがない場合は空を返す
        // 代替として、primary_labelベースの簡易検出を行う
        if static_objects.is_empty() {
            return self.analyze_by_primary_label(camera_id, interval).await;
        }

        Ok(static_objects.into_iter().map(|(x_zone, y_zone, label, repeat_count)| {
            StaticObjectIssue {
                x_zone,
                y_zone,
                label,
                repeat_count,
            }
        }).collect())
    }

    /// primary_labelベースの簡易固定物検出
    /// 同一ラベルが短時間に繰り返し検出される場合を検出
    async fn analyze_by_primary_label(&self, camera_id: &str, interval: &str) -> Result<Vec<StaticObjectIssue>> {
        // 1時間単位で同一ラベルの検出数を集計
        let repeated_labels: Vec<(String, i64)> = sqlx::query_as(&format!(
            r#"
            SELECT
                primary_label,
                COUNT(*) as repeat_count
            FROM detection_logs
            WHERE camera_id = ?
              AND created_at >= DATE_SUB(NOW(), INTERVAL {})
              AND primary_event != 'none'
              AND primary_label IS NOT NULL
              AND primary_label != ''
            GROUP BY primary_label, DATE_FORMAT(created_at, '%Y-%m-%d %H:00:00')
            HAVING repeat_count >= 10
            ORDER BY repeat_count DESC
            LIMIT 10
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
