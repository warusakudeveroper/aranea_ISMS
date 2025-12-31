//! ConfigStore Repository
//!
//! Database access layer for ConfigStore

use super::types::*;
use crate::error::{Error, Result};
use sqlx::MySqlPool;

/// ConfigStore repository for database operations
#[derive(Clone)]
pub struct ConfigRepository {
    pool: MySqlPool,
}

impl ConfigRepository {
    /// Create new repository
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    // ========================================
    // Camera CRUD
    // ========================================

    /// Get all cameras
    pub async fn get_all_cameras(&self) -> Result<Vec<Camera>> {
        let cameras = sqlx::query_as::<_, Camera>(
            r#"
            SELECT
                camera_id, name, location, floor,
                rtsp_main, rtsp_sub, snapshot_url,
                family, manufacturer, model,
                ip_address, mac_address, lacis_id,
                enabled, polling_enabled, polling_interval_sec,
                suggest_policy_weight, camera_context,
                created_at, updated_at
            FROM cameras
            ORDER BY location, name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(cameras)
    }

    /// Get enabled cameras for polling
    pub async fn get_enabled_cameras(&self) -> Result<Vec<Camera>> {
        let cameras = sqlx::query_as::<_, Camera>(
            r#"
            SELECT
                camera_id, name, location, floor,
                rtsp_main, rtsp_sub, snapshot_url,
                family, manufacturer, model,
                ip_address, mac_address, lacis_id,
                enabled, polling_enabled, polling_interval_sec,
                suggest_policy_weight, camera_context,
                created_at, updated_at
            FROM cameras
            WHERE enabled = TRUE AND polling_enabled = TRUE
            ORDER BY suggest_policy_weight DESC, location, name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(cameras)
    }

    /// Get camera by ID
    pub async fn get_camera(&self, camera_id: &str) -> Result<Option<Camera>> {
        let camera = sqlx::query_as::<_, Camera>(
            r#"
            SELECT
                camera_id, name, location, floor,
                rtsp_main, rtsp_sub, snapshot_url,
                family, manufacturer, model,
                ip_address, mac_address, lacis_id,
                enabled, polling_enabled, polling_interval_sec,
                suggest_policy_weight, camera_context,
                created_at, updated_at
            FROM cameras
            WHERE camera_id = ?
            "#,
        )
        .bind(camera_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(camera)
    }

    /// Create camera
    pub async fn create_camera(&self, req: &CreateCameraRequest) -> Result<Camera> {
        let now = chrono::Utc::now();
        let family = req.family.unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO cameras (
                camera_id, name, location, floor,
                rtsp_main, rtsp_sub, snapshot_url,
                family, manufacturer, model,
                ip_address, mac_address,
                camera_context,
                enabled, polling_enabled, polling_interval_sec,
                suggest_policy_weight,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, TRUE, TRUE, 60, 5, ?, ?)
            "#,
        )
        .bind(&req.camera_id)
        .bind(&req.name)
        .bind(&req.location)
        .bind(&req.floor)
        .bind(&req.rtsp_main)
        .bind(&req.rtsp_sub)
        .bind(&req.snapshot_url)
        .bind(format!("{:?}", family).to_lowercase())
        .bind(&req.manufacturer)
        .bind(&req.model)
        .bind(&req.ip_address)
        .bind(&req.mac_address)
        .bind(&req.camera_context)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_camera(&req.camera_id)
            .await?
            .ok_or(Error::NotFound("Camera not found after insert".to_string()))
    }

    /// Update camera
    pub async fn update_camera(&self, camera_id: &str, req: &UpdateCameraRequest) -> Result<Camera> {
        let now = chrono::Utc::now();

        // Build dynamic update query
        let mut updates = vec!["updated_at = ?".to_string()];
        let mut has_update = false;

        if req.name.is_some() {
            updates.push("name = ?".to_string());
            has_update = true;
        }
        if req.location.is_some() {
            updates.push("location = ?".to_string());
            has_update = true;
        }
        if req.floor.is_some() {
            updates.push("floor = ?".to_string());
            has_update = true;
        }
        if req.rtsp_main.is_some() {
            updates.push("rtsp_main = ?".to_string());
            has_update = true;
        }
        if req.rtsp_sub.is_some() {
            updates.push("rtsp_sub = ?".to_string());
            has_update = true;
        }
        if req.snapshot_url.is_some() {
            updates.push("snapshot_url = ?".to_string());
            has_update = true;
        }
        if req.enabled.is_some() {
            updates.push("enabled = ?".to_string());
            has_update = true;
        }
        if req.polling_enabled.is_some() {
            updates.push("polling_enabled = ?".to_string());
            has_update = true;
        }
        if req.polling_interval_sec.is_some() {
            updates.push("polling_interval_sec = ?".to_string());
            has_update = true;
        }
        if req.suggest_policy_weight.is_some() {
            updates.push("suggest_policy_weight = ?".to_string());
            has_update = true;
        }
        if req.camera_context.is_some() {
            updates.push("camera_context = ?".to_string());
            has_update = true;
        }

        if !has_update {
            return self
                .get_camera(camera_id)
                .await?
                .ok_or(Error::NotFound("Camera not found".to_string()));
        }

        let query = format!(
            "UPDATE cameras SET {} WHERE camera_id = ?",
            updates.join(", ")
        );

        let mut q = sqlx::query(&query).bind(&now);

        if let Some(ref v) = req.name {
            q = q.bind(v);
        }
        if let Some(ref v) = req.location {
            q = q.bind(v);
        }
        if let Some(ref v) = req.floor {
            q = q.bind(v);
        }
        if let Some(ref v) = req.rtsp_main {
            q = q.bind(v);
        }
        if let Some(ref v) = req.rtsp_sub {
            q = q.bind(v);
        }
        if let Some(ref v) = req.snapshot_url {
            q = q.bind(v);
        }
        if let Some(v) = req.enabled {
            q = q.bind(v);
        }
        if let Some(v) = req.polling_enabled {
            q = q.bind(v);
        }
        if let Some(v) = req.polling_interval_sec {
            q = q.bind(v);
        }
        if let Some(v) = req.suggest_policy_weight {
            q = q.bind(v);
        }
        if let Some(ref v) = req.camera_context {
            q = q.bind(v);
        }

        q = q.bind(camera_id);
        q.execute(&self.pool).await?;

        self.get_camera(camera_id)
            .await?
            .ok_or(Error::NotFound("Camera not found after update".to_string()))
    }

    /// Delete camera
    pub async fn delete_camera(&self, camera_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM cameras WHERE camera_id = ?")
            .bind(camera_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================
    // Schema Version
    // ========================================

    /// Get active schema version
    pub async fn get_active_schema_version(&self) -> Result<Option<SchemaVersion>> {
        let version = sqlx::query_as::<_, SchemaVersion>(
            r#"
            SELECT version_id, schema_json, is_active, created_at
            FROM schema_versions
            WHERE is_active = TRUE
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(version)
    }

    /// Set schema version
    pub async fn set_schema_version(&self, version_id: &str, schema_json: serde_json::Value) -> Result<()> {
        let now = chrono::Utc::now();

        // Deactivate all
        sqlx::query("UPDATE schema_versions SET is_active = FALSE")
            .execute(&self.pool)
            .await?;

        // Insert or update
        sqlx::query(
            r#"
            INSERT INTO schema_versions (version_id, schema_json, is_active, created_at)
            VALUES (?, ?, TRUE, ?)
            ON DUPLICATE KEY UPDATE schema_json = ?, is_active = TRUE
            "#,
        )
        .bind(version_id)
        .bind(&schema_json)
        .bind(&now)
        .bind(&schema_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========================================
    // Settings
    // ========================================

    /// Get setting by key
    pub async fn get_setting(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let setting = sqlx::query_as::<_, Setting>(
            "SELECT setting_key, setting_json, updated_at FROM settings WHERE setting_key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(setting.map(|s| s.setting_json))
    }

    /// Get all settings
    pub async fn get_all_settings(&self) -> Result<std::collections::HashMap<String, serde_json::Value>> {
        let settings = sqlx::query_as::<_, Setting>(
            "SELECT setting_key, setting_json, updated_at FROM settings",
        )
        .fetch_all(&self.pool)
        .await?;

        let map = settings
            .into_iter()
            .map(|s| (s.setting_key, s.setting_json))
            .collect();

        Ok(map)
    }

    /// Set setting
    pub async fn set_setting(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO settings (setting_key, setting_json, updated_at)
            VALUES (?, ?, ?)
            ON DUPLICATE KEY UPDATE setting_json = ?, updated_at = ?
            "#,
        )
        .bind(key)
        .bind(&value)
        .bind(&now)
        .bind(&value)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
