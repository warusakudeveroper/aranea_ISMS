//! CameraContext Management
//!
//! Phase 2: CameraRegistry (Issue #115)
//! T2-7: カメラコンテキスト管理
//!
//! ## 概要
//! カメラ毎のコンテキスト情報を管理し、ParaclateSummary用の
//! コンテキストマップを構築する。
//!
//! ## コンテキスト情報
//! - 設置場所・用途
//! - 監視対象・重点項目
//! - 運用上の注意事項
//! - カスタムメタデータ

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::collections::HashMap;
use tracing::{debug, info};

/// カメラコンテキスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraContext {
    pub camera_id: String,
    pub lacis_id: Option<String>,

    /// 設置場所の説明
    pub location_description: Option<String>,

    /// 用途・目的
    pub purpose: Option<String>,

    /// 監視対象（例: "入口", "駐車場", "倉庫"）
    pub monitoring_target: Option<String>,

    /// 重点項目（例: "人物検出重視", "車両監視"）
    pub priority_focus: Option<String>,

    /// 運用上の注意事項
    pub operational_notes: Option<String>,

    /// カスタムメタデータ (JSON)
    pub custom_metadata: Option<serde_json::Value>,

    /// 最終更新日時
    pub updated_at: Option<DateTime<Utc>>,
}

impl Default for CameraContext {
    fn default() -> Self {
        Self {
            camera_id: String::new(),
            lacis_id: None,
            location_description: None,
            purpose: None,
            monitoring_target: None,
            priority_focus: None,
            operational_notes: None,
            custom_metadata: None,
            updated_at: None,
        }
    }
}

/// コンテキスト更新リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContextRequest {
    pub location_description: Option<String>,
    pub purpose: Option<String>,
    pub monitoring_target: Option<String>,
    pub priority_focus: Option<String>,
    pub operational_notes: Option<String>,
    pub custom_metadata: Option<serde_json::Value>,
}

/// Summary用コンテキストマップエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMapEntry {
    pub camera_id: String,
    pub lacis_id: String,
    pub name: String,
    pub location: Option<String>,
    pub context_summary: String,
}

/// CameraContext Service
pub struct CameraContextService {
    pool: MySqlPool,
}

impl CameraContextService {
    /// 新しいCameraContextServiceを作成
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// カメラコンテキストを取得
    pub async fn get_context(&self, camera_id: &str) -> crate::Result<Option<CameraContext>> {
        let row = sqlx::query_as::<_, ContextRow>(
            r#"
            SELECT
                camera_id, lacis_id, camera_context, updated_at
            FROM cameras
            WHERE camera_id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        match row {
            Some(r) => {
                let context = self.parse_context(&r)?;
                Ok(Some(context))
            }
            None => Ok(None),
        }
    }

    /// カメラコンテキストを更新
    pub async fn update_context(
        &self,
        camera_id: &str,
        request: UpdateContextRequest,
    ) -> crate::Result<()> {
        // 既存コンテキスト取得
        let existing = self.get_context(camera_id).await?;
        let mut context = existing.unwrap_or_else(|| CameraContext {
            camera_id: camera_id.to_string(),
            ..Default::default()
        });

        // 更新
        if request.location_description.is_some() {
            context.location_description = request.location_description;
        }
        if request.purpose.is_some() {
            context.purpose = request.purpose;
        }
        if request.monitoring_target.is_some() {
            context.monitoring_target = request.monitoring_target;
        }
        if request.priority_focus.is_some() {
            context.priority_focus = request.priority_focus;
        }
        if request.operational_notes.is_some() {
            context.operational_notes = request.operational_notes;
        }
        if request.custom_metadata.is_some() {
            context.custom_metadata = request.custom_metadata;
        }
        context.updated_at = Some(Utc::now());

        // JSON化して保存
        let context_json = serde_json::to_string(&context)
            .map_err(|e| crate::Error::Parse(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE cameras
            SET camera_context = ?, updated_at = NOW(3)
            WHERE camera_id = ?
            "#,
        )
        .bind(&context_json)
        .bind(camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        info!(camera_id = %camera_id, "Camera context updated");
        Ok(())
    }

    /// Summary用コンテキストマップを構築
    ///
    /// ## 返却内容
    /// lacisID → コンテキスト要約のマップ
    pub async fn build_context_map(&self, tid: &str) -> crate::Result<HashMap<String, ContextMapEntry>> {
        let rows = sqlx::query_as::<_, ContextMapRow>(
            r#"
            SELECT
                c.camera_id,
                c.lacis_id,
                c.name,
                c.location,
                c.camera_context
            FROM cameras c
            WHERE c.tid = ?
            AND c.registration_state = 'registered'
            AND c.lacis_id IS NOT NULL
            AND c.deleted_at IS NULL
            "#,
        )
        .bind(tid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        let mut map = HashMap::new();
        for row in rows {
            let lacis_id = match row.lacis_id {
                Some(id) => id,
                None => continue,
            };

            let context_summary = self.build_context_summary(&row);

            let entry = ContextMapEntry {
                camera_id: row.camera_id.clone(),
                lacis_id: lacis_id.clone(),
                name: row.name,
                location: row.location,
                context_summary,
            };

            map.insert(lacis_id, entry);
        }

        debug!(
            tid = %tid,
            count = map.len(),
            "Built context map for Summary"
        );

        Ok(map)
    }

    /// fid単位でコンテキストマップを構築
    pub async fn build_context_map_by_fid(
        &self,
        tid: &str,
        fid: &str,
    ) -> crate::Result<HashMap<String, ContextMapEntry>> {
        let rows = sqlx::query_as::<_, ContextMapRow>(
            r#"
            SELECT
                c.camera_id,
                c.lacis_id,
                c.name,
                c.location,
                c.camera_context
            FROM cameras c
            WHERE c.tid = ? AND c.fid = ?
            AND c.registration_state = 'registered'
            AND c.lacis_id IS NOT NULL
            AND c.deleted_at IS NULL
            "#,
        )
        .bind(tid)
        .bind(fid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        let mut map = HashMap::new();
        for row in rows {
            let lacis_id = match row.lacis_id {
                Some(id) => id,
                None => continue,
            };

            let context_summary = self.build_context_summary(&row);

            let entry = ContextMapEntry {
                camera_id: row.camera_id.clone(),
                lacis_id: lacis_id.clone(),
                name: row.name,
                location: row.location,
                context_summary,
            };

            map.insert(lacis_id, entry);
        }

        debug!(
            tid = %tid,
            fid = %fid,
            count = map.len(),
            "Built context map for fid"
        );

        Ok(map)
    }

    /// コンテキスト要約を構築
    fn build_context_summary(&self, row: &ContextMapRow) -> String {
        let mut parts = Vec::new();

        // カメラ名と場所
        parts.push(row.name.clone());
        if let Some(ref loc) = row.location {
            parts.push(format!("({})", loc));
        }

        // camera_contextからの情報を追加
        if let Some(ref context_json) = row.camera_context {
            if let Ok(ctx) = serde_json::from_str::<CameraContext>(context_json) {
                if let Some(ref target) = ctx.monitoring_target {
                    parts.push(format!("監視: {}", target));
                }
                if let Some(ref focus) = ctx.priority_focus {
                    parts.push(format!("重点: {}", focus));
                }
                if let Some(ref purpose) = ctx.purpose {
                    parts.push(format!("用途: {}", purpose));
                }
            }
        }

        parts.join(" / ")
    }

    /// コンテキストJSON→構造体変換
    fn parse_context(&self, row: &ContextRow) -> crate::Result<CameraContext> {
        let mut context = if let Some(ref json_str) = row.camera_context {
            serde_json::from_str(json_str).unwrap_or_else(|_| CameraContext {
                camera_id: row.camera_id.clone(),
                lacis_id: row.lacis_id.clone(),
                ..Default::default()
            })
        } else {
            CameraContext {
                camera_id: row.camera_id.clone(),
                lacis_id: row.lacis_id.clone(),
                ..Default::default()
            }
        };

        context.camera_id = row.camera_id.clone();
        context.lacis_id = row.lacis_id.clone();

        Ok(context)
    }
}

#[derive(sqlx::FromRow)]
struct ContextRow {
    camera_id: String,
    lacis_id: Option<String>,
    camera_context: Option<String>,
    updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(sqlx::FromRow)]
struct ContextMapRow {
    camera_id: String,
    lacis_id: Option<String>,
    name: String,
    location: Option<String>,
    camera_context: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_context_default() {
        let ctx = CameraContext::default();
        assert!(ctx.camera_id.is_empty());
        assert!(ctx.lacis_id.is_none());
        assert!(ctx.location_description.is_none());
    }

    #[test]
    fn test_context_serialization() {
        let ctx = CameraContext {
            camera_id: "cam_001".to_string(),
            lacis_id: Some("3801AABBCCDDEEFF0001".to_string()),
            location_description: Some("入口".to_string()),
            purpose: Some("来客監視".to_string()),
            monitoring_target: Some("人物".to_string()),
            priority_focus: Some("不審者検出".to_string()),
            operational_notes: Some("深夜は感度を上げる".to_string()),
            custom_metadata: Some(serde_json::json!({"floor": 1})),
            updated_at: None,
        };

        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("cam_001"));
        assert!(json.contains("入口"));

        let parsed: CameraContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.camera_id, "cam_001");
        assert_eq!(parsed.location_description, Some("入口".to_string()));
    }

    #[test]
    fn test_update_context_request() {
        let req = UpdateContextRequest {
            location_description: Some("倉庫A".to_string()),
            purpose: None,
            monitoring_target: Some("荷物".to_string()),
            priority_focus: None,
            operational_notes: None,
            custom_metadata: None,
        };

        assert_eq!(req.location_description, Some("倉庫A".to_string()));
        assert!(req.purpose.is_none());
    }
}
