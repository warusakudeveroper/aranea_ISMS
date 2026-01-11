//! Camera Sync Repository
//!
//! Phase 8: カメラ双方向同期 (Issue #121)
//! T8-2: repository.rs

use super::types::*;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use tracing::{debug, error, info};

/// Camera Sync Repository
pub struct CameraSyncRepository {
    pool: MySqlPool,
}

impl CameraSyncRepository {
    /// 新しいCameraSyncRepositoryを作成
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// プールへの参照を取得
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    // ========================================
    // camera_sync_state テーブル操作
    // ========================================

    /// 同期状態を保存または更新
    pub async fn upsert_sync_state(&self, state: &CameraSyncState) -> crate::Result<()> {
        let status_str = match state.mobes_sync_status {
            MobesSyncStatus::Synced => "synced",
            MobesSyncStatus::Pending => "pending",
            MobesSyncStatus::Failed => "failed",
            MobesSyncStatus::Deleted => "deleted",
        };

        sqlx::query(
            r#"
            INSERT INTO camera_sync_state
                (camera_id, lacis_id, last_sync_to_mobes, last_sync_from_mobes,
                 mobes_sync_status, sync_error_message, retry_count)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                lacis_id = VALUES(lacis_id),
                last_sync_to_mobes = VALUES(last_sync_to_mobes),
                last_sync_from_mobes = VALUES(last_sync_from_mobes),
                mobes_sync_status = VALUES(mobes_sync_status),
                sync_error_message = VALUES(sync_error_message),
                retry_count = VALUES(retry_count),
                updated_at = NOW(3)
            "#,
        )
        .bind(&state.camera_id)
        .bind(&state.lacis_id)
        .bind(state.last_sync_to_mobes)
        .bind(state.last_sync_from_mobes)
        .bind(status_str)
        .bind(&state.sync_error_message)
        .bind(state.retry_count)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        debug!(camera_id = %state.camera_id, "Upserted camera sync state");
        Ok(())
    }

    /// 同期状態を取得
    pub async fn get_sync_state(&self, camera_id: &str) -> crate::Result<Option<CameraSyncState>> {
        let row = sqlx::query_as::<_, SyncStateRow>(
            r#"
            SELECT id, camera_id, lacis_id, last_sync_to_mobes, last_sync_from_mobes,
                   mobes_sync_status, sync_error_message, retry_count, created_at, updated_at
            FROM camera_sync_state
            WHERE camera_id = ?
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    /// LacisIDで同期状態を取得
    pub async fn get_sync_state_by_lacis_id(
        &self,
        lacis_id: &str,
    ) -> crate::Result<Option<CameraSyncState>> {
        let row = sqlx::query_as::<_, SyncStateRow>(
            r#"
            SELECT id, camera_id, lacis_id, last_sync_to_mobes, last_sync_from_mobes,
                   mobes_sync_status, sync_error_message, retry_count, created_at, updated_at
            FROM camera_sync_state
            WHERE lacis_id = ?
            "#,
        )
        .bind(lacis_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    /// 同期保留中のカメラを取得
    pub async fn get_pending_cameras(&self) -> crate::Result<Vec<CameraSyncState>> {
        let rows = sqlx::query_as::<_, SyncStateRow>(
            r#"
            SELECT id, camera_id, lacis_id, last_sync_to_mobes, last_sync_from_mobes,
                   mobes_sync_status, sync_error_message, retry_count, created_at, updated_at
            FROM camera_sync_state
            WHERE mobes_sync_status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 同期失敗でリトライ可能なカメラを取得
    pub async fn get_retryable_cameras(&self, max_retry: i32) -> crate::Result<Vec<CameraSyncState>> {
        let rows = sqlx::query_as::<_, SyncStateRow>(
            r#"
            SELECT id, camera_id, lacis_id, last_sync_to_mobes, last_sync_from_mobes,
                   mobes_sync_status, sync_error_message, retry_count, created_at, updated_at
            FROM camera_sync_state
            WHERE mobes_sync_status = 'failed' AND retry_count < ?
            ORDER BY updated_at ASC
            "#,
        )
        .bind(max_retry)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 同期状態を「同期済み」にマーク
    pub async fn mark_synced_to_mobes(&self, camera_id: &str) -> crate::Result<()> {
        sqlx::query(
            r#"
            UPDATE camera_sync_state
            SET mobes_sync_status = 'synced',
                last_sync_to_mobes = NOW(3),
                sync_error_message = NULL,
                retry_count = 0,
                updated_at = NOW(3)
            WHERE camera_id = ?
            "#,
        )
        .bind(camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(())
    }

    /// 同期状態を「失敗」にマーク
    pub async fn mark_sync_failed(
        &self,
        camera_id: &str,
        error_message: &str,
    ) -> crate::Result<()> {
        sqlx::query(
            r#"
            UPDATE camera_sync_state
            SET mobes_sync_status = 'failed',
                sync_error_message = ?,
                retry_count = retry_count + 1,
                updated_at = NOW(3)
            WHERE camera_id = ?
            "#,
        )
        .bind(error_message)
        .bind(camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(())
    }

    /// 同期状態カウントを取得
    pub async fn get_sync_counts(&self) -> crate::Result<SyncCounts> {
        let row = sqlx::query_as::<_, SyncCountsRow>(
            r#"
            SELECT
                SUM(CASE WHEN mobes_sync_status = 'synced' THEN 1 ELSE 0 END) as synced_count,
                SUM(CASE WHEN mobes_sync_status = 'pending' THEN 1 ELSE 0 END) as pending_count,
                SUM(CASE WHEN mobes_sync_status = 'failed' THEN 1 ELSE 0 END) as failed_count
            FROM camera_sync_state
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(SyncCounts {
            synced: row.synced_count.unwrap_or(0) as usize,
            pending: row.pending_count.unwrap_or(0) as usize,
            failed: row.failed_count.unwrap_or(0) as usize,
        })
    }

    // ========================================
    // camera_paraclate_settings テーブル操作
    // ========================================

    /// カメラ個別設定を保存または更新
    pub async fn upsert_paraclate_settings(
        &self,
        camera_id: &str,
        lacis_id: &str,
        settings: &CameraParaclateSettings,
    ) -> crate::Result<()> {
        let settings_json = serde_json::json!({
            "sensitivity": settings.sensitivity,
            "detection_zone": settings.detection_zone,
            "alert_threshold": settings.alert_threshold,
            "custom_preset": settings.custom_preset
        });

        sqlx::query(
            r#"
            INSERT INTO camera_paraclate_settings
                (camera_id, lacis_id, sensitivity, detection_zone, alert_threshold,
                 custom_preset, settings_json, synced_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, NOW(3))
            ON DUPLICATE KEY UPDATE
                lacis_id = VALUES(lacis_id),
                sensitivity = VALUES(sensitivity),
                detection_zone = VALUES(detection_zone),
                alert_threshold = VALUES(alert_threshold),
                custom_preset = VALUES(custom_preset),
                settings_json = VALUES(settings_json),
                synced_at = NOW(3),
                updated_at = NOW(3)
            "#,
        )
        .bind(camera_id)
        .bind(lacis_id)
        .bind(settings.sensitivity)
        .bind(&settings.detection_zone)
        .bind(settings.alert_threshold)
        .bind(&settings.custom_preset)
        .bind(settings_json.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        info!(camera_id = %camera_id, "Updated paraclate settings from mobes2.0");
        Ok(())
    }

    /// カメラ個別設定を取得
    pub async fn get_paraclate_settings(
        &self,
        camera_id: &str,
    ) -> crate::Result<Option<CameraParaclateSettings>> {
        let row = sqlx::query_as::<_, ParaclateSettingsRow>(
            r#"
            SELECT sensitivity, detection_zone, alert_threshold, custom_preset
            FROM camera_paraclate_settings
            WHERE camera_id = ?
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(row.map(|r| CameraParaclateSettings {
            sensitivity: r.sensitivity,
            detection_zone: r.detection_zone,
            alert_threshold: r.alert_threshold,
            custom_preset: r.custom_preset,
        }))
    }

    // ========================================
    // camera_sync_logs テーブル操作
    // ========================================

    /// 同期ログを記録
    pub async fn log_sync(
        &self,
        camera_id: &str,
        lacis_id: Option<&str>,
        direction: SyncDirection,
        sync_type: &str,
        status: &str,
        changed_fields: Option<&serde_json::Value>,
        error_message: Option<&str>,
        request_payload: Option<&serde_json::Value>,
        response_payload: Option<&serde_json::Value>,
    ) -> crate::Result<()> {
        let direction_str = match direction {
            SyncDirection::ToMobes => "to_mobes",
            SyncDirection::FromMobes => "from_mobes",
        };

        sqlx::query(
            r#"
            INSERT INTO camera_sync_logs
                (camera_id, lacis_id, sync_direction, sync_type, status,
                 changed_fields, error_message, request_payload, response_payload)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(camera_id)
        .bind(lacis_id)
        .bind(direction_str)
        .bind(sync_type)
        .bind(status)
        .bind(changed_fields.map(|v| v.to_string()))
        .bind(error_message)
        .bind(request_payload.map(|v| v.to_string()))
        .bind(response_payload.map(|v| v.to_string()))
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(())
    }

    /// 最近の同期ログを取得
    pub async fn get_recent_logs(&self, limit: i32) -> crate::Result<Vec<CameraSyncLog>> {
        let rows = sqlx::query_as::<_, SyncLogRow>(
            r#"
            SELECT id, camera_id, lacis_id, sync_direction, sync_type, status,
                   changed_fields, error_message, request_payload, response_payload, created_at
            FROM camera_sync_logs
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    // ========================================
    // cameras テーブル操作（同期関連）
    // ========================================

    /// camerasテーブルの同期日時を更新
    pub async fn update_camera_mobes_sync(&self, camera_id: &str) -> crate::Result<()> {
        sqlx::query(
            r#"
            UPDATE cameras
            SET mobes_synced_at = NOW(3),
                mobes_sync_version = mobes_sync_version + 1
            WHERE camera_id = ?
            "#,
        )
        .bind(camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(())
    }
}

// ========================================
// Row Types (sqlx)
// ========================================

#[derive(sqlx::FromRow)]
struct SyncStateRow {
    id: i64,
    camera_id: String,
    lacis_id: String,
    last_sync_to_mobes: Option<chrono::NaiveDateTime>,
    last_sync_from_mobes: Option<chrono::NaiveDateTime>,
    mobes_sync_status: String,
    sync_error_message: Option<String>,
    retry_count: i32,
    created_at: Option<chrono::NaiveDateTime>,
    updated_at: Option<chrono::NaiveDateTime>,
}

impl From<SyncStateRow> for CameraSyncState {
    fn from(row: SyncStateRow) -> Self {
        let status = match row.mobes_sync_status.as_str() {
            "synced" => MobesSyncStatus::Synced,
            "pending" => MobesSyncStatus::Pending,
            "failed" => MobesSyncStatus::Failed,
            "deleted" => MobesSyncStatus::Deleted,
            _ => MobesSyncStatus::Pending,
        };

        Self {
            id: Some(row.id),
            camera_id: row.camera_id,
            lacis_id: row.lacis_id,
            last_sync_to_mobes: row.last_sync_to_mobes.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            last_sync_from_mobes: row.last_sync_from_mobes.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            mobes_sync_status: status,
            sync_error_message: row.sync_error_message,
            retry_count: row.retry_count,
            created_at: row.created_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            updated_at: row.updated_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        }
    }
}

#[derive(sqlx::FromRow)]
struct SyncCountsRow {
    synced_count: Option<i64>,
    pending_count: Option<i64>,
    failed_count: Option<i64>,
}

pub struct SyncCounts {
    pub synced: usize,
    pub pending: usize,
    pub failed: usize,
}

#[derive(sqlx::FromRow)]
struct ParaclateSettingsRow {
    sensitivity: f32,
    detection_zone: String,
    alert_threshold: i32,
    custom_preset: Option<String>,
}

#[derive(sqlx::FromRow)]
struct SyncLogRow {
    id: i64,
    camera_id: String,
    lacis_id: Option<String>,
    sync_direction: String,
    sync_type: String,
    status: String,
    changed_fields: Option<String>,
    error_message: Option<String>,
    request_payload: Option<String>,
    response_payload: Option<String>,
    created_at: Option<chrono::NaiveDateTime>,
}

impl From<SyncLogRow> for CameraSyncLog {
    fn from(row: SyncLogRow) -> Self {
        let direction = if row.sync_direction == "to_mobes" {
            SyncDirection::ToMobes
        } else {
            SyncDirection::FromMobes
        };

        Self {
            id: Some(row.id),
            camera_id: row.camera_id,
            lacis_id: row.lacis_id,
            sync_direction: direction,
            sync_type: row.sync_type,
            status: row.status,
            changed_fields: row.changed_fields.and_then(|s| serde_json::from_str(&s).ok()),
            error_message: row.error_message,
            request_payload: row.request_payload.and_then(|s| serde_json::from_str(&s).ok()),
            response_payload: row.response_payload.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: row.created_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        }
    }
}
