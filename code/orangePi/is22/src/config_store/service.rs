//! ConfigStore Service
//!
//! Business logic layer for ConfigStore

use super::repository::ConfigRepository;
use super::types::*;
use crate::error::Result;

/// ConfigStore service for business logic
pub struct ConfigService {
    repo: ConfigRepository,
}

impl ConfigService {
    /// Create new service
    pub fn new(repo: ConfigRepository) -> Self {
        Self { repo }
    }

    // ========================================
    // Camera Operations
    // ========================================

    /// List all cameras
    pub async fn list_cameras(&self) -> Result<Vec<Camera>> {
        self.repo.get_all_cameras().await
    }

    /// List enabled cameras for polling
    pub async fn list_enabled_cameras(&self) -> Result<Vec<Camera>> {
        self.repo.get_enabled_cameras().await
    }

    /// Get camera by ID
    pub async fn get_camera(&self, camera_id: &str) -> Result<Option<Camera>> {
        self.repo.get_camera(camera_id).await
    }

    /// Create camera
    pub async fn create_camera(&self, req: CreateCameraRequest) -> Result<Camera> {
        // Validate camera_id format
        if req.camera_id.is_empty() || req.camera_id.len() > 64 {
            return Err(crate::Error::Validation(
                "camera_id must be 1-64 characters".to_string(),
            ));
        }

        // Check for duplicates
        if self.repo.get_camera(&req.camera_id).await?.is_some() {
            return Err(crate::Error::Conflict(format!(
                "Camera {} already exists",
                req.camera_id
            )));
        }

        self.repo.create_camera(&req).await
    }

    /// Update camera
    pub async fn update_camera(&self, camera_id: &str, req: UpdateCameraRequest) -> Result<Camera> {
        // Check existence
        if self.repo.get_camera(camera_id).await?.is_none() {
            return Err(crate::Error::NotFound(format!(
                "Camera {} not found",
                camera_id
            )));
        }

        self.repo.update_camera(camera_id, &req).await
    }

    /// Delete camera (hard delete)
    pub async fn delete_camera(&self, camera_id: &str) -> Result<()> {
        // Check existence
        if self.repo.get_camera(camera_id).await?.is_none() {
            return Err(crate::Error::NotFound(format!(
                "Camera {} not found",
                camera_id
            )));
        }

        self.repo.delete_camera(camera_id).await
    }

    /// Soft delete camera (set deleted_at, MAC preserved for restore)
    pub async fn soft_delete_camera(&self, camera_id: &str) -> Result<Camera> {
        // Check existence
        if self.repo.get_camera(camera_id).await?.is_none() {
            return Err(crate::Error::NotFound(format!(
                "Camera {} not found",
                camera_id
            )));
        }

        self.repo.soft_delete_camera(camera_id).await
    }

    /// Restore soft-deleted camera by MAC address
    pub async fn restore_camera_by_mac(&self, mac_address: &str) -> Result<Camera> {
        // Check if camera with this MAC exists
        match self.repo.get_camera_by_mac(mac_address).await? {
            Some(camera) if camera.deleted_at.is_some() => {
                // Camera exists and is soft-deleted, restore it
                self.repo.restore_camera_by_mac(mac_address).await
            }
            Some(_) => {
                // Camera exists but is not deleted
                Err(crate::Error::Conflict(format!(
                    "Camera with MAC {} is not deleted",
                    mac_address
                )))
            }
            None => {
                Err(crate::Error::NotFound(format!(
                    "No camera found with MAC {}",
                    mac_address
                )))
            }
        }
    }

    /// Update camera IP (for rescan)
    pub async fn update_camera_ip(&self, camera_id: &str, new_ip: &str) -> Result<Camera> {
        // Check existence
        if self.repo.get_camera(camera_id).await?.is_none() {
            return Err(crate::Error::NotFound(format!(
                "Camera {} not found",
                camera_id
            )));
        }

        self.repo.update_camera_ip(camera_id, new_ip).await
    }

    /// Update camera verified timestamp
    pub async fn update_camera_verified(&self, camera_id: &str) -> Result<()> {
        // Check existence
        if self.repo.get_camera(camera_id).await?.is_none() {
            return Err(crate::Error::NotFound(format!(
                "Camera {} not found",
                camera_id
            )));
        }

        self.repo.update_camera_verified(camera_id).await
    }

    // ========================================
    // Schema Operations
    // ========================================

    /// Get current schema version
    pub async fn get_current_schema_version(&self) -> Result<Option<String>> {
        let version = self.repo.get_active_schema_version().await?;
        Ok(version.map(|v| v.version_id))
    }

    /// Get schema JSON
    pub async fn get_schema_json(&self) -> Result<Option<serde_json::Value>> {
        let version = self.repo.get_active_schema_version().await?;
        Ok(version.map(|v| v.schema_json))
    }

    /// Update schema version
    pub async fn set_schema_version(&self, version_id: &str, schema_json: serde_json::Value) -> Result<()> {
        self.repo.set_schema_version(version_id, schema_json).await
    }

    // ========================================
    // Settings Operations
    // ========================================

    /// Get all settings
    pub async fn get_all_settings(&self) -> Result<std::collections::HashMap<String, serde_json::Value>> {
        self.repo.get_all_settings().await
    }

    /// Get polling policy
    pub async fn get_polling_policy(&self) -> Result<PollingPolicy> {
        let setting = self.repo.get_setting("polling").await?;
        match setting {
            Some(json) => Ok(serde_json::from_value(json)?),
            None => Ok(PollingPolicy::default()),
        }
    }

    /// Set polling policy
    pub async fn set_polling_policy(&self, policy: PollingPolicy) -> Result<()> {
        let json = serde_json::to_value(&policy)?;
        self.repo.set_setting("polling", json).await
    }

    /// Get suggest policy
    pub async fn get_suggest_policy(&self) -> Result<SuggestPolicy> {
        let setting = self.repo.get_setting("suggest").await?;
        match setting {
            Some(json) => Ok(serde_json::from_value(json)?),
            None => Ok(SuggestPolicy::default()),
        }
    }

    /// Set suggest policy
    pub async fn set_suggest_policy(&self, policy: SuggestPolicy) -> Result<()> {
        let json = serde_json::to_value(&policy)?;
        self.repo.set_setting("suggest", json).await
    }

    /// Get admission policy
    pub async fn get_admission_policy(&self) -> Result<AdmissionPolicy> {
        let setting = self.repo.get_setting("admission").await?;
        match setting {
            Some(json) => Ok(serde_json::from_value(json)?),
            None => Ok(AdmissionPolicy::default()),
        }
    }

    /// Set admission policy
    pub async fn set_admission_policy(&self, policy: AdmissionPolicy) -> Result<()> {
        let json = serde_json::to_value(&policy)?;
        self.repo.set_setting("admission", json).await
    }

    /// Get notification policy
    pub async fn get_notification_policy(&self) -> Result<NotificationPolicy> {
        let setting = self.repo.get_setting("notification").await?;
        match setting {
            Some(json) => Ok(serde_json::from_value(json)?),
            None => Ok(NotificationPolicy::default()),
        }
    }

    /// Set notification policy
    pub async fn set_notification_policy(&self, policy: NotificationPolicy) -> Result<()> {
        let json = serde_json::to_value(&policy)?;
        self.repo.set_setting("notification", json).await
    }
}
