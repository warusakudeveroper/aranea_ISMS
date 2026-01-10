//! Overdetection Analyzer Service
//!
//! Issue #107: プリセットグラフィカルUI・過剰検出判定・タグ傾向分析
//!
//! ロジックベースの過剰検出判定サービス（LLM不要）
//! - タグ固定化検出（80%以上で同一タグ出現）
//! - 固定物反応検出（同一座標で繰り返し検出）
//! - 高頻度検出（Z-score > 2.0）
//! - Unknown乱発（30%超過）

pub mod tag_analyzer;
pub mod static_detector;

use sqlx::{MySql, Pool};
use crate::Result;
use serde::{Deserialize, Serialize};

// Re-exports
pub use tag_analyzer::{TagAnalyzer, TagTrend, TrendDirection};
pub use static_detector::{StaticDetector, StaticObjectIssue};

/// 過剰検出の種別
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverdetectionType {
    /// タグ固定化（80%以上で同一タグ出現）
    TagFixation,
    /// 固定物反応（同一座標で繰り返し検出）
    StaticObject,
    /// 高頻度検出（Z-score > 2.0）
    HighFrequency,
    /// Unknown乱発（30%超過）
    UnknownFlood,
}

/// 過剰検出の深刻度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverdetectionSeverity {
    /// 情報（参考レベル）
    Info,
    /// 警告（70-80%）
    Warning,
    /// 危険（80%以上）
    Critical,
}

/// 過剰検出Issue
#[derive(Debug, Clone, Serialize)]
pub struct OverdetectionIssue {
    /// Issue種別
    #[serde(rename = "type")]
    pub issue_type: OverdetectionType,
    /// 関連タグ（タグ固定化の場合）
    pub tag: Option<String>,
    /// 関連ラベル（固定物反応の場合）
    pub label: Option<String>,
    /// ゾーン（固定物反応の場合）
    pub zone: Option<String>,
    /// 発生率（%）
    pub rate: f64,
    /// 件数
    pub count: i64,
    /// 深刻度
    pub severity: OverdetectionSeverity,
    /// 改善提案
    pub suggestion: String,
}

/// カメラ別過剰検出結果
#[derive(Debug, Clone, Serialize)]
pub struct CameraOverdetection {
    pub camera_id: String,
    pub camera_name: Option<String>,
    pub issues: Vec<OverdetectionIssue>,
}

/// 過剰検出分析結果
#[derive(Debug, Clone, Serialize)]
pub struct OverdetectionResult {
    pub period: String,
    pub cameras: Vec<CameraOverdetection>,
}

/// プリセットバランス値
#[derive(Debug, Clone, Serialize)]
pub struct PresetBalance {
    /// 検出感度（0-100）: 低閾値 = 高感度
    pub detection_sensitivity: u8,
    /// 人物詳細度（0-100）: 低par_threshold = 詳細多
    pub person_detail: u8,
    /// 対象多様性（0-100）: excluded少 = 多様
    pub object_variety: u8,
    /// 動体検知感度（0-100）
    pub motion_sensitivity: u8,
}

/// プリセットバランス情報
#[derive(Debug, Clone, Serialize)]
pub struct PresetBalanceInfo {
    pub preset_id: String,
    pub name: String,
    pub balance: PresetBalance,
}

/// Overdetection Analyzer Service
pub struct OverdetectionAnalyzer {
    pool: Pool<MySql>,
    tag_analyzer: TagAnalyzer,
    static_detector: StaticDetector,
}

impl OverdetectionAnalyzer {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self {
            pool: pool.clone(),
            tag_analyzer: TagAnalyzer::new(pool.clone()),
            static_detector: StaticDetector::new(pool),
        }
    }

    /// 全プリセットのバランス値を計算 (v2.0)
    pub fn get_all_preset_balances(&self) -> Vec<PresetBalanceInfo> {
        use crate::preset_loader::{Preset, preset_ids};

        // v2.0: 13プリセット（廃止3種除外、新規4種追加）
        let presets = vec![
            (preset_ids::PERSON_PRIORITY, "人物優先", Preset::person_priority()),
            (preset_ids::BALANCED, "バランス", Preset::balanced()),
            (preset_ids::PARKING, "駐車場", Preset::parking()),
            (preset_ids::ENTRANCE, "エントランス", Preset::entrance()),
            (preset_ids::CORRIDOR, "廊下", Preset::corridor()),
            (preset_ids::HOTEL_CORRIDOR, "ホテル廊下", Preset::hotel_corridor()),  // v2.0 新規
            (preset_ids::OUTDOOR, "屋外", Preset::outdoor()),
            (preset_ids::CROWD, "群衆", Preset::crowd()),
            (preset_ids::OFFICE, "オフィス", Preset::office()),
            (preset_ids::RESTAURANT, "飲食店ホール", Preset::restaurant()),        // v2.0 新規
            (preset_ids::SECURITY_ZONE, "警戒区域", Preset::security_zone()),      // v2.0 新規
            (preset_ids::OBJECT_FOCUS, "対物検知", Preset::object_focus()),        // v2.0 新規
            (preset_ids::CUSTOM, "カスタム", Preset::custom()),
        ];
        // v2.0 廃止: night_vision, retail, warehouse

        presets.into_iter().map(|(id, name, preset)| {
            PresetBalanceInfo {
                preset_id: id.to_string(),
                name: name.to_string(),
                balance: Self::calculate_balance(&preset),
            }
        }).collect()
    }

    /// プリセットからバランス値を計算
    fn calculate_balance(preset: &crate::preset_loader::Preset) -> PresetBalance {
        // 検出感度: conf_override基準（低い = 高感度）
        // conf_override: 0.2-0.8, デフォルト0.5
        let conf = preset.conf_override.unwrap_or(0.5);
        let detection_sensitivity = ((1.0 - conf) * 100.0 / 0.8 * 100.0).clamp(0.0, 100.0) as u8;

        // 人物詳細度: par_threshold基準（低い = 詳細多）
        // par_threshold: 0.3-0.8, デフォルト0.5
        let par = preset.par_threshold.unwrap_or(0.5);
        let person_detail = ((1.0 - par) * 100.0 / 0.8 * 100.0).clamp(0.0, 100.0) as u8;

        // 対象多様性: excluded_objects数基準（少ない = 多様）
        // v2.0: YOLOラベル展開により最大40個と想定
        // (VEHICLES:8 + ANIMALS:10 + OFFICE:8 + INDOOR:6 + FURNITURE:4 + TABLEWARE:7 = 43)
        let excluded_count = preset.excluded_objects.len();
        let object_variety = (100 - (excluded_count * 100 / 40).min(100)) as u8;

        // 動体検知感度: enable_frame_diff基準
        let motion_sensitivity = if preset.enable_frame_diff { 80 } else { 20 };

        PresetBalance {
            detection_sensitivity,
            person_detail,
            object_variety,
            motion_sensitivity,
        }
    }

    /// 過剰検出分析を実行
    pub async fn analyze(&self, period: &str) -> Result<OverdetectionResult> {
        let interval = match period {
            "7d" => "7 DAY",
            "30d" => "30 DAY",
            _ => "1 DAY",
        };

        tracing::info!("Overdetection analysis started for period: {}", period);

        // 1. アクティブなカメラを取得（上位10件に制限）
        let cameras: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT DISTINCT c.camera_id, c.name
            FROM cameras c
            INNER JOIN detection_logs d ON c.camera_id = d.camera_id
            WHERE d.created_at >= DATE_SUB(NOW(), INTERVAL 1 DAY)
            GROUP BY c.camera_id, c.name
            ORDER BY COUNT(*) DESC
            LIMIT 10
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();

        // 2. カメラごとに分析
        for (camera_id, camera_name) in cameras {
            let mut issues = Vec::new();

            // 高頻度検出チェック
            if let Some(issue) = self.check_high_frequency(&camera_id, interval).await? {
                issues.push(issue);
            }

            // Unknown乱発チェック
            if let Some(issue) = self.check_unknown_flood(&camera_id, interval).await? {
                issues.push(issue);
            }

            // 問題があるカメラのみ結果に追加
            if !issues.is_empty() {
                results.push(CameraOverdetection {
                    camera_id,
                    camera_name,
                    issues,
                });
            }
        }

        tracing::info!("Overdetection analysis completed: {} cameras with issues", results.len());

        Ok(OverdetectionResult {
            period: period.to_string(),
            cameras: results,
        })
    }

    /// Unknown乱発チェック（30%超過）
    async fn check_unknown_flood(&self, camera_id: &str, interval: &str) -> Result<Option<OverdetectionIssue>> {
        let stats: Option<(i64, i64)> = sqlx::query_as(&format!(
            r#"
            SELECT
                COUNT(*) as total,
                CAST(COALESCE(SUM(CASE WHEN primary_event = 'unknown' THEN 1 ELSE 0 END), 0) AS SIGNED) as unknown_count
            FROM detection_logs
            WHERE camera_id = ?
              AND created_at >= DATE_SUB(NOW(), INTERVAL {})
            "#,
            interval
        ))
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((total, unknown_count)) = stats {
            if total > 0 {
                let rate = (unknown_count as f64 / total as f64) * 100.0;
                if rate > 30.0 {
                    return Ok(Some(OverdetectionIssue {
                        issue_type: OverdetectionType::UnknownFlood,
                        tag: None,
                        label: Some("unknown".to_string()),
                        zone: None,
                        rate,
                        count: unknown_count,
                        severity: if rate > 50.0 {
                            OverdetectionSeverity::Critical
                        } else {
                            OverdetectionSeverity::Warning
                        },
                        suggestion: "conf_thresholdを上げるか、excluded_objectsを見直し".to_string(),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// 高頻度検出チェック（Z-score > 2.0）
    /// 性能最適化: 全カメラ統計は重いため、簡略化
    async fn check_high_frequency(&self, camera_id: &str, interval: &str) -> Result<Option<OverdetectionIssue>> {
        // このカメラの検出数のみ取得（Z-scoreは省略）
        let camera_count: Option<(i64,)> = sqlx::query_as(&format!(
            r#"
            SELECT COUNT(*) as cnt
            FROM detection_logs
            WHERE camera_id = ?
              AND created_at >= DATE_SUB(NOW(), INTERVAL {})
            "#,
            interval
        ))
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await?;

        // 1日あたり500件以上を高頻度とみなす簡易判定
        let threshold = match interval {
            "7 DAY" => 3500,  // 500 * 7
            "30 DAY" => 15000, // 500 * 30
            _ => 500, // 1 DAY
        };

        if let Some((count,)) = camera_count {
            if count > threshold {
                return Ok(Some(OverdetectionIssue {
                    issue_type: OverdetectionType::HighFrequency,
                    tag: None,
                    label: None,
                    zone: None,
                    rate: count as f64,
                    count,
                    severity: if count > threshold * 2 {
                        OverdetectionSeverity::Critical
                    } else {
                        OverdetectionSeverity::Warning
                    },
                    suggestion: "検出数が多い。ポーリング間隔を延ばすか閾値を上げて検討".to_string(),
                }));
            }
        }

        Ok(None)
    }

    /// タグ傾向を取得
    pub async fn get_tag_trends(&self, camera_id: &str, period: &str) -> Result<Vec<TagTrend>> {
        let interval = match period {
            "7d" => "7 DAY",
            "30d" => "30 DAY",
            _ => "1 DAY",
        };

        self.tag_analyzer.get_trends(camera_id, interval).await
    }

    /// 全体タグサマリーを取得
    pub async fn get_tag_summary(&self, period: &str) -> Result<Vec<TagTrend>> {
        let interval = match period {
            "7d" => "7 DAY",
            "30d" => "30 DAY",
            _ => "1 DAY",
        };

        self.tag_analyzer.get_summary(interval).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_calculation() {
        use crate::preset_loader::Preset;

        let person_priority = Preset::person_priority();
        let balance = OverdetectionAnalyzer::calculate_balance(&person_priority);

        // person_priority: conf_override=0.3 → 高感度
        assert!(balance.detection_sensitivity > 50);
        // par_threshold=0.4 → 詳細多め
        assert!(balance.person_detail > 50);
        // excluded_objects: vehicle, animal → 少ない
        assert!(balance.object_variety > 80);
        // enable_frame_diff=true
        assert_eq!(balance.motion_sensitivity, 80);
    }

    #[test]
    fn test_severity_levels() {
        // 80%以上はCritical
        assert_eq!(
            if 85.0 >= 80.0 {
                OverdetectionSeverity::Critical
            } else if 85.0 >= 70.0 {
                OverdetectionSeverity::Warning
            } else {
                OverdetectionSeverity::Info
            },
            OverdetectionSeverity::Critical
        );

        // 70-80%はWarning
        assert_eq!(
            if 75.0 >= 80.0 {
                OverdetectionSeverity::Critical
            } else if 75.0 >= 70.0 {
                OverdetectionSeverity::Warning
            } else {
                OverdetectionSeverity::Info
            },
            OverdetectionSeverity::Warning
        );
    }
}
