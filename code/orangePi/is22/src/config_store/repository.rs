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

    /// Camera SELECT columns (all fields for SSoT)
    const CAMERA_COLUMNS: &'static str = r#"
        camera_id, name, location, floor, rid,
        rtsp_main, rtsp_sub, rtsp_username, rtsp_password, snapshot_url,
        family, manufacturer, model, ip_address, mac_address, lacis_id, cic,
        enabled, polling_enabled, polling_interval_sec, suggest_policy_weight,
        camera_context, rotation, fid, tid, sort_order,
        serial_number, hardware_id, firmware_version, onvif_endpoint,
        rtsp_port, http_port, onvif_port,
        resolution_main, codec_main, fps_main, bitrate_main,
        resolution_sub, codec_sub, fps_sub, bitrate_sub,
        ptz_supported, ptz_continuous, ptz_absolute, ptz_relative,
        ptz_pan_range, ptz_tilt_range, ptz_zoom_range, ptz_presets, ptz_home_supported,
        audio_input_supported, audio_output_supported, audio_codec,
        onvif_profiles, discovery_method, last_verified_at, last_rescan_at, deleted_at,
        created_at, updated_at
    "#;

    /// Get all cameras (excluding soft-deleted)
    pub async fn get_all_cameras(&self) -> Result<Vec<Camera>> {
        let query = format!(
            "SELECT {} FROM cameras WHERE deleted_at IS NULL ORDER BY sort_order, location, name",
            Self::CAMERA_COLUMNS
        );
        let cameras = sqlx::query_as::<_, Camera>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(cameras)
    }

    /// Get enabled cameras for polling (excluding soft-deleted)
    pub async fn get_enabled_cameras(&self) -> Result<Vec<Camera>> {
        let query = format!(
            "SELECT {} FROM cameras WHERE enabled = TRUE AND polling_enabled = TRUE AND deleted_at IS NULL ORDER BY suggest_policy_weight DESC, sort_order, location, name",
            Self::CAMERA_COLUMNS
        );
        let cameras = sqlx::query_as::<_, Camera>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(cameras)
    }

    /// Get camera by ID (including soft-deleted for restore)
    pub async fn get_camera(&self, camera_id: &str) -> Result<Option<Camera>> {
        let query = format!(
            "SELECT {} FROM cameras WHERE camera_id = ?",
            Self::CAMERA_COLUMNS
        );
        let camera = sqlx::query_as::<_, Camera>(&query)
            .bind(camera_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(camera)
    }

    /// Get camera by MAC address (for restore from soft-delete)
    pub async fn get_camera_by_mac(&self, mac_address: &str) -> Result<Option<Camera>> {
        let query = format!(
            "SELECT {} FROM cameras WHERE mac_address = ?",
            Self::CAMERA_COLUMNS
        );
        let camera = sqlx::query_as::<_, Camera>(&query)
            .bind(mac_address)
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

    /// Update camera - handles all fields dynamically
    pub async fn update_camera(&self, camera_id: &str, req: &UpdateCameraRequest) -> Result<Camera> {
        let now = chrono::Utc::now();

        // Build the update query with all possible fields
        let mut set_clauses = vec!["updated_at = ?".to_string()];

        // String fields
        if req.name.is_some() { set_clauses.push("name = ?".to_string()); }
        if req.location.is_some() { set_clauses.push("location = ?".to_string()); }
        if req.floor.is_some() { set_clauses.push("floor = ?".to_string()); }
        if req.rid.is_some() { set_clauses.push("rid = ?".to_string()); }
        if req.rtsp_main.is_some() { set_clauses.push("rtsp_main = ?".to_string()); }
        if req.rtsp_sub.is_some() { set_clauses.push("rtsp_sub = ?".to_string()); }
        if req.rtsp_username.is_some() { set_clauses.push("rtsp_username = ?".to_string()); }
        if req.rtsp_password.is_some() { set_clauses.push("rtsp_password = ?".to_string()); }
        if req.snapshot_url.is_some() { set_clauses.push("snapshot_url = ?".to_string()); }
        if req.family.is_some() { set_clauses.push("family = ?".to_string()); }
        if req.manufacturer.is_some() { set_clauses.push("manufacturer = ?".to_string()); }
        if req.model.is_some() { set_clauses.push("model = ?".to_string()); }
        if req.ip_address.is_some() { set_clauses.push("ip_address = ?".to_string()); }
        if req.mac_address.is_some() { set_clauses.push("mac_address = ?".to_string()); }
        if req.lacis_id.is_some() { set_clauses.push("lacis_id = ?".to_string()); }
        if req.cic.is_some() { set_clauses.push("cic = ?".to_string()); }
        if req.fid.is_some() { set_clauses.push("fid = ?".to_string()); }
        if req.tid.is_some() { set_clauses.push("tid = ?".to_string()); }
        if req.serial_number.is_some() { set_clauses.push("serial_number = ?".to_string()); }
        if req.hardware_id.is_some() { set_clauses.push("hardware_id = ?".to_string()); }
        if req.firmware_version.is_some() { set_clauses.push("firmware_version = ?".to_string()); }
        if req.onvif_endpoint.is_some() { set_clauses.push("onvif_endpoint = ?".to_string()); }
        if req.resolution_main.is_some() { set_clauses.push("resolution_main = ?".to_string()); }
        if req.codec_main.is_some() { set_clauses.push("codec_main = ?".to_string()); }
        if req.resolution_sub.is_some() { set_clauses.push("resolution_sub = ?".to_string()); }
        if req.codec_sub.is_some() { set_clauses.push("codec_sub = ?".to_string()); }
        if req.audio_codec.is_some() { set_clauses.push("audio_codec = ?".to_string()); }
        if req.discovery_method.is_some() { set_clauses.push("discovery_method = ?".to_string()); }

        // Boolean fields
        if req.enabled.is_some() { set_clauses.push("enabled = ?".to_string()); }
        if req.polling_enabled.is_some() { set_clauses.push("polling_enabled = ?".to_string()); }
        if req.ptz_supported.is_some() { set_clauses.push("ptz_supported = ?".to_string()); }
        if req.ptz_continuous.is_some() { set_clauses.push("ptz_continuous = ?".to_string()); }
        if req.ptz_absolute.is_some() { set_clauses.push("ptz_absolute = ?".to_string()); }
        if req.ptz_relative.is_some() { set_clauses.push("ptz_relative = ?".to_string()); }
        if req.ptz_home_supported.is_some() { set_clauses.push("ptz_home_supported = ?".to_string()); }
        if req.audio_input_supported.is_some() { set_clauses.push("audio_input_supported = ?".to_string()); }
        if req.audio_output_supported.is_some() { set_clauses.push("audio_output_supported = ?".to_string()); }

        // Integer fields
        if req.polling_interval_sec.is_some() { set_clauses.push("polling_interval_sec = ?".to_string()); }
        if req.suggest_policy_weight.is_some() { set_clauses.push("suggest_policy_weight = ?".to_string()); }
        if req.rotation.is_some() { set_clauses.push("rotation = ?".to_string()); }
        if req.sort_order.is_some() { set_clauses.push("sort_order = ?".to_string()); }
        if req.rtsp_port.is_some() { set_clauses.push("rtsp_port = ?".to_string()); }
        if req.http_port.is_some() { set_clauses.push("http_port = ?".to_string()); }
        if req.onvif_port.is_some() { set_clauses.push("onvif_port = ?".to_string()); }
        if req.fps_main.is_some() { set_clauses.push("fps_main = ?".to_string()); }
        if req.bitrate_main.is_some() { set_clauses.push("bitrate_main = ?".to_string()); }
        if req.fps_sub.is_some() { set_clauses.push("fps_sub = ?".to_string()); }
        if req.bitrate_sub.is_some() { set_clauses.push("bitrate_sub = ?".to_string()); }

        // JSON fields
        if req.camera_context.is_some() { set_clauses.push("camera_context = ?".to_string()); }
        if req.ptz_pan_range.is_some() { set_clauses.push("ptz_pan_range = ?".to_string()); }
        if req.ptz_tilt_range.is_some() { set_clauses.push("ptz_tilt_range = ?".to_string()); }
        if req.ptz_zoom_range.is_some() { set_clauses.push("ptz_zoom_range = ?".to_string()); }
        if req.ptz_presets.is_some() { set_clauses.push("ptz_presets = ?".to_string()); }
        if req.onvif_profiles.is_some() { set_clauses.push("onvif_profiles = ?".to_string()); }

        if set_clauses.len() <= 1 {
            // Only updated_at, no actual changes
            return self
                .get_camera(camera_id)
                .await?
                .ok_or(Error::NotFound("Camera not found".to_string()));
        }

        let query = format!(
            "UPDATE cameras SET {} WHERE camera_id = ?",
            set_clauses.join(", ")
        );

        // Build the query with bindings
        let mut q = sqlx::query(&query).bind(now);

        // Bind in same order as set_clauses
        // String fields
        if let Some(ref v) = req.name { q = q.bind(v); }
        if let Some(ref v) = req.location { q = q.bind(v); }
        if let Some(ref v) = req.floor { q = q.bind(v); }
        if let Some(ref v) = req.rid { q = q.bind(v); }
        if let Some(ref v) = req.rtsp_main { q = q.bind(v); }
        if let Some(ref v) = req.rtsp_sub { q = q.bind(v); }
        if let Some(ref v) = req.rtsp_username { q = q.bind(v); }
        if let Some(ref v) = req.rtsp_password { q = q.bind(v); }
        if let Some(ref v) = req.snapshot_url { q = q.bind(v); }
        if let Some(ref v) = req.family { q = q.bind(String::from(*v)); }
        if let Some(ref v) = req.manufacturer { q = q.bind(v); }
        if let Some(ref v) = req.model { q = q.bind(v); }
        if let Some(ref v) = req.ip_address { q = q.bind(v); }
        if let Some(ref v) = req.mac_address { q = q.bind(v); }
        if let Some(ref v) = req.lacis_id { q = q.bind(v); }
        if let Some(ref v) = req.cic { q = q.bind(v); }
        if let Some(ref v) = req.fid { q = q.bind(v); }
        if let Some(ref v) = req.tid { q = q.bind(v); }
        if let Some(ref v) = req.serial_number { q = q.bind(v); }
        if let Some(ref v) = req.hardware_id { q = q.bind(v); }
        if let Some(ref v) = req.firmware_version { q = q.bind(v); }
        if let Some(ref v) = req.onvif_endpoint { q = q.bind(v); }
        if let Some(ref v) = req.resolution_main { q = q.bind(v); }
        if let Some(ref v) = req.codec_main { q = q.bind(v); }
        if let Some(ref v) = req.resolution_sub { q = q.bind(v); }
        if let Some(ref v) = req.codec_sub { q = q.bind(v); }
        if let Some(ref v) = req.audio_codec { q = q.bind(v); }
        if let Some(ref v) = req.discovery_method { q = q.bind(v); }

        // Boolean fields
        if let Some(v) = req.enabled { q = q.bind(v); }
        if let Some(v) = req.polling_enabled { q = q.bind(v); }
        if let Some(v) = req.ptz_supported { q = q.bind(v); }
        if let Some(v) = req.ptz_continuous { q = q.bind(v); }
        if let Some(v) = req.ptz_absolute { q = q.bind(v); }
        if let Some(v) = req.ptz_relative { q = q.bind(v); }
        if let Some(v) = req.ptz_home_supported { q = q.bind(v); }
        if let Some(v) = req.audio_input_supported { q = q.bind(v); }
        if let Some(v) = req.audio_output_supported { q = q.bind(v); }

        // Integer fields
        if let Some(v) = req.polling_interval_sec { q = q.bind(v); }
        if let Some(v) = req.suggest_policy_weight { q = q.bind(v); }
        if let Some(v) = req.rotation { q = q.bind(v); }
        if let Some(v) = req.sort_order { q = q.bind(v); }
        if let Some(v) = req.rtsp_port { q = q.bind(v); }
        if let Some(v) = req.http_port { q = q.bind(v); }
        if let Some(v) = req.onvif_port { q = q.bind(v); }
        if let Some(v) = req.fps_main { q = q.bind(v); }
        if let Some(v) = req.bitrate_main { q = q.bind(v); }
        if let Some(v) = req.fps_sub { q = q.bind(v); }
        if let Some(v) = req.bitrate_sub { q = q.bind(v); }

        // JSON fields
        if let Some(ref v) = req.camera_context { q = q.bind(v); }
        if let Some(ref v) = req.ptz_pan_range { q = q.bind(v); }
        if let Some(ref v) = req.ptz_tilt_range { q = q.bind(v); }
        if let Some(ref v) = req.ptz_zoom_range { q = q.bind(v); }
        if let Some(ref v) = req.ptz_presets { q = q.bind(v); }
        if let Some(ref v) = req.onvif_profiles { q = q.bind(v); }

        // Bind camera_id last
        q = q.bind(camera_id);
        q.execute(&self.pool).await?;

        self.get_camera(camera_id)
            .await?
            .ok_or(Error::NotFound("Camera not found after update".to_string()))
    }

    /// Soft delete camera (set deleted_at)
    pub async fn soft_delete_camera(&self, camera_id: &str) -> Result<Camera> {
        let now = chrono::Utc::now();
        sqlx::query("UPDATE cameras SET deleted_at = ?, updated_at = ? WHERE camera_id = ?")
            .bind(&now)
            .bind(&now)
            .bind(camera_id)
            .execute(&self.pool)
            .await?;

        self.get_camera(camera_id)
            .await?
            .ok_or(Error::NotFound("Camera not found after soft delete".to_string()))
    }

    /// Restore soft-deleted camera by MAC address
    pub async fn restore_camera_by_mac(&self, mac_address: &str) -> Result<Camera> {
        let now = chrono::Utc::now();
        sqlx::query("UPDATE cameras SET deleted_at = NULL, updated_at = ? WHERE mac_address = ?")
            .bind(&now)
            .bind(mac_address)
            .execute(&self.pool)
            .await?;

        self.get_camera_by_mac(mac_address)
            .await?
            .ok_or(Error::NotFound("Camera not found after restore".to_string()))
    }

    /// Update camera IP address (for rescan)
    pub async fn update_camera_ip(&self, camera_id: &str, new_ip: &str) -> Result<Camera> {
        let now = chrono::Utc::now();
        sqlx::query("UPDATE cameras SET ip_address = ?, last_rescan_at = ?, updated_at = ? WHERE camera_id = ?")
            .bind(new_ip)
            .bind(&now)
            .bind(&now)
            .bind(camera_id)
            .execute(&self.pool)
            .await?;

        self.get_camera(camera_id)
            .await?
            .ok_or(Error::NotFound("Camera not found after IP update".to_string()))
    }

    /// Update last_verified_at
    pub async fn update_camera_verified(&self, camera_id: &str) -> Result<()> {
        let now = chrono::Utc::now();
        sqlx::query("UPDATE cameras SET last_verified_at = ?, updated_at = ? WHERE camera_id = ?")
            .bind(&now)
            .bind(&now)
            .bind(camera_id)
            .execute(&self.pool)
            .await?;
        Ok(())
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
