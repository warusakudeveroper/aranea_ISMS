//! CameraRegistry Repository
//!
//! Phase 2: CameraRegistry (Issue #115)
//! DD05_CameraRegistry.md準拠
//!
//! カメラ登録情報のDB永続化

use super::types::{CameraRegistration, RegistrationState};
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;

/// CameraRegistry Repository
pub struct CameraRegistrationRepository {
    pool: MySqlPool,
}

impl CameraRegistrationRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// プールへの参照を取得（内部用）
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    /// カメラ登録情報を保存
    pub async fn save_registration(&self, reg: &CameraRegistration) -> crate::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO camera_registration
                (camera_id, lacis_id, tid, fid, rid, cic, state, registered_at, error_message)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                lacis_id = VALUES(lacis_id),
                tid = VALUES(tid),
                fid = VALUES(fid),
                rid = VALUES(rid),
                cic = VALUES(cic),
                state = VALUES(state),
                registered_at = VALUES(registered_at),
                error_message = VALUES(error_message),
                updated_at = CURRENT_TIMESTAMP(3)
            "#,
        )
        .bind(&reg.camera_id)
        .bind(&reg.lacis_id)
        .bind(&reg.tid)
        .bind(&reg.fid)
        .bind(&reg.rid)
        .bind(&reg.cic)
        .bind(reg.state.to_string())
        .bind(reg.registered_at)
        .bind(&reg.error_message)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        // camerasテーブルも更新
        sqlx::query(
            r#"
            UPDATE cameras SET
                lacis_id = ?,
                tid = ?,
                fid = ?,
                rid = ?,
                cic = ?,
                registration_state = ?,
                registered_at = ?
            WHERE camera_id = ?
            "#,
        )
        .bind(&reg.lacis_id)
        .bind(&reg.tid)
        .bind(&reg.fid)
        .bind(&reg.rid)
        .bind(&reg.cic)
        .bind(reg.state.to_string())
        .bind(reg.registered_at)
        .bind(&reg.camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(())
    }

    /// カメラ登録情報を取得
    pub async fn get_registration(
        &self,
        camera_id: &str,
    ) -> crate::Result<Option<CameraRegistration>> {
        let row = sqlx::query_as::<_, RegistrationRow>(
            r#"
            SELECT
                camera_id, lacis_id, tid, fid, rid, cic, state, registered_at, error_message
            FROM camera_registration
            WHERE camera_id = ?
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    /// LacisIDでカメラ登録情報を取得
    pub async fn get_by_lacis_id(
        &self,
        lacis_id: &str,
    ) -> crate::Result<Option<CameraRegistration>> {
        let row = sqlx::query_as::<_, RegistrationRow>(
            r#"
            SELECT
                camera_id, lacis_id, tid, fid, rid, cic, state, registered_at, error_message
            FROM camera_registration
            WHERE lacis_id = ? AND state = 'registered'
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .bind(lacis_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    /// 登録済みカメラ一覧を取得
    pub async fn get_registered_cameras(&self, tid: &str) -> crate::Result<Vec<CameraRegistration>> {
        let rows = sqlx::query_as::<_, RegistrationRow>(
            r#"
            SELECT
                camera_id, lacis_id, tid, fid, rid, cic, state, registered_at, error_message
            FROM camera_registration
            WHERE tid = ? AND state = 'registered'
            ORDER BY registered_at DESC
            "#,
        )
        .bind(tid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 登録をクリア
    pub async fn clear_registration(&self, camera_id: &str) -> crate::Result<()> {
        sqlx::query(
            r#"
            UPDATE camera_registration SET
                state = 'cleared',
                updated_at = CURRENT_TIMESTAMP(3)
            WHERE camera_id = ? AND state = 'registered'
            "#,
        )
        .bind(camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        // camerasテーブルも更新
        sqlx::query(
            r#"
            UPDATE cameras SET
                registration_state = 'pending',
                cic = NULL
            WHERE camera_id = ?
            "#,
        )
        .bind(camera_id)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(())
    }

    /// カメラのMACアドレスを取得
    pub async fn get_camera_mac(&self, camera_id: &str) -> crate::Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            r#"
            SELECT mac_address FROM cameras WHERE camera_id = ?
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(row.and_then(|r| r.0))
    }

    /// カメラのブランドProductCodeを取得
    pub async fn get_camera_product_code(&self, camera_id: &str) -> crate::Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            r#"
            SELECT cb.product_code
            FROM cameras c
            LEFT JOIN oui_entries oe ON UPPER(SUBSTRING(c.mac_address, 1, 8)) = oe.oui_prefix
            LEFT JOIN camera_brands cb ON oe.brand_id = cb.id
            WHERE c.camera_id = ?
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(row.and_then(|r| r.0))
    }
}

/// DB行マッピング用構造体
#[derive(sqlx::FromRow)]
struct RegistrationRow {
    camera_id: String,
    lacis_id: String,
    tid: String,
    fid: Option<String>,
    rid: Option<String>,
    cic: String,
    state: String,
    registered_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
}

impl From<RegistrationRow> for CameraRegistration {
    fn from(row: RegistrationRow) -> Self {
        Self {
            camera_id: row.camera_id,
            lacis_id: row.lacis_id,
            tid: row.tid,
            fid: row.fid,
            rid: row.rid,
            cic: row.cic,
            state: RegistrationState::from(row.state.as_str()),
            registered_at: row.registered_at,
            error_message: row.error_message,
        }
    }
}
