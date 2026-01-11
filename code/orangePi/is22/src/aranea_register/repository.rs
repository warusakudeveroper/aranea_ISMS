//! AraneaRegister Repository
//!
//! Phase 1: AraneaRegister (Issue #114)
//! DD01_AraneaRegister.md準拠
//!
//! ## 責務
//! - aranea_registrationテーブルへのCRUD操作
//! - 登録情報の永続化・取得

use super::types::AraneaRegistration;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, MySqlPool};

/// DB行マッピング用構造体
#[derive(Debug, Clone, FromRow)]
struct AraneaRegistrationRow {
    id: u32,
    lacis_id: String,
    tid: String,
    fid: Option<String>,
    cic: String,
    device_type: String,
    state_endpoint: Option<String>,
    mqtt_endpoint: Option<String>,
    registered_at: DateTime<Utc>,
    last_sync_at: Option<DateTime<Utc>>,
}

impl From<AraneaRegistrationRow> for AraneaRegistration {
    fn from(row: AraneaRegistrationRow) -> Self {
        AraneaRegistration {
            id: Some(row.id),
            lacis_id: row.lacis_id,
            tid: row.tid,
            fid: row.fid,
            cic: row.cic,
            device_type: row.device_type,
            state_endpoint: row.state_endpoint,
            mqtt_endpoint: row.mqtt_endpoint,
            registered_at: row.registered_at,
            last_sync_at: row.last_sync_at,
        }
    }
}

/// AraneaRegister Repository
pub struct AraneaRegistrationRepository {
    pool: MySqlPool,
}

impl AraneaRegistrationRepository {
    /// Create new repository
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Insert new registration
    pub async fn insert(&self, registration: &AraneaRegistration) -> crate::Result<u64> {
        let result = sqlx::query(
            r#"
            INSERT INTO aranea_registration
                (lacis_id, tid, fid, cic, device_type, state_endpoint, mqtt_endpoint, registered_at)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&registration.lacis_id)
        .bind(&registration.tid)
        .bind(&registration.fid)
        .bind(&registration.cic)
        .bind(&registration.device_type)
        .bind(&registration.state_endpoint)
        .bind(&registration.mqtt_endpoint)
        .bind(&registration.registered_at)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            lacis_id = %registration.lacis_id,
            tid = %registration.tid,
            "AraneaRegister: Registration inserted"
        );

        Ok(result.last_insert_id())
    }

    /// Get registration by lacis_id
    pub async fn get_by_lacis_id(
        &self,
        lacis_id: &str,
    ) -> crate::Result<Option<AraneaRegistration>> {
        let row = sqlx::query_as::<_, AraneaRegistrationRow>(
            r#"
            SELECT id, lacis_id, tid, fid, cic, device_type, state_endpoint, mqtt_endpoint, registered_at, last_sync_at
            FROM aranea_registration
            WHERE lacis_id = ?
            "#,
        )
        .bind(lacis_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    /// Get the latest registration (for single-device mode)
    pub async fn get_latest(&self) -> crate::Result<Option<AraneaRegistration>> {
        let row = sqlx::query_as::<_, AraneaRegistrationRow>(
            r#"
            SELECT id, lacis_id, tid, fid, cic, device_type, state_endpoint, mqtt_endpoint, registered_at, last_sync_at
            FROM aranea_registration
            ORDER BY registered_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    /// Get registered_at timestamp
    pub async fn get_registered_at(&self) -> crate::Result<Option<DateTime<Utc>>> {
        let row: Option<(DateTime<Utc>,)> = sqlx::query_as(
            r#"
            SELECT registered_at
            FROM aranea_registration
            ORDER BY registered_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(ts,)| ts))
    }

    /// Get last_sync_at timestamp
    pub async fn get_last_sync_at(&self) -> crate::Result<Option<DateTime<Utc>>> {
        let row: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            r#"
            SELECT last_sync_at
            FROM aranea_registration
            ORDER BY registered_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|(ts,)| ts))
    }

    /// Update last_sync_at timestamp
    pub async fn update_last_sync_at(&self, lacis_id: &str) -> crate::Result<()> {
        sqlx::query(
            r#"
            UPDATE aranea_registration
            SET last_sync_at = NOW(3)
            WHERE lacis_id = ?
            "#,
        )
        .bind(lacis_id)
        .execute(&self.pool)
        .await?;

        tracing::debug!(lacis_id = %lacis_id, "AraneaRegister: last_sync_at updated");

        Ok(())
    }

    /// Delete all registrations (for clear_registration)
    pub async fn delete_all(&self) -> crate::Result<u64> {
        let result = sqlx::query("DELETE FROM aranea_registration")
            .execute(&self.pool)
            .await?;

        let count = result.rows_affected();
        tracing::info!(count = count, "AraneaRegister: All registrations deleted");

        Ok(count)
    }

    /// Delete registration by lacis_id
    pub async fn delete_by_lacis_id(&self, lacis_id: &str) -> crate::Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM aranea_registration
            WHERE lacis_id = ?
            "#,
        )
        .bind(lacis_id)
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            tracing::info!(lacis_id = %lacis_id, "AraneaRegister: Registration deleted");
        }

        Ok(deleted)
    }

    /// Check if any registration exists
    pub async fn exists(&self) -> crate::Result<bool> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM aranea_registration")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.0 > 0)
    }

    /// Count registrations
    pub async fn count(&self) -> crate::Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM aranea_registration")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.0)
    }
}
