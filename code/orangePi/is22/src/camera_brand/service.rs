//! Camera Brand Service
//!
//! Business logic layer with in-memory caching for high-performance OUI lookups.

use super::repository::CameraBrandRepository;
use super::types::*;
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// In-memory cache for fast OUI/template lookups
#[derive(Debug, Default)]
struct BrandCache {
    /// OUI prefix (uppercase) -> brand info
    oui_map: HashMap<String, OuiBrandInfo>,
    /// Brand ID -> list of RTSP templates
    templates: HashMap<i32, Vec<RtspTemplateInfo>>,
    /// Generic RTSP paths (priority ordered)
    generic_paths: Vec<GenericRtspPath>,
    /// Cache last updated time
    last_updated: Option<DateTime<Utc>>,
}

/// Camera brand service with caching
pub struct CameraBrandService {
    repo: CameraBrandRepository,
    cache: Arc<RwLock<BrandCache>>,
}

impl CameraBrandService {
    /// Create new service
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            repo: CameraBrandRepository::new(pool),
            cache: Arc::new(RwLock::new(BrandCache::default())),
        }
    }

    /// Initialize cache on startup
    pub async fn init(&self) -> Result<()> {
        self.refresh_cache().await?;
        info!("CameraBrandService cache initialized");
        Ok(())
    }

    /// Refresh the in-memory cache from database
    pub async fn refresh_cache(&self) -> Result<()> {
        let oui_entries = self.repo.load_oui_brand_map().await?;
        let templates = self.repo.load_rtsp_templates().await?;
        let generic_paths = self.repo.get_enabled_generic_paths().await?;

        let mut cache = self.cache.write().await;

        // Build OUI map
        cache.oui_map.clear();
        for entry in oui_entries {
            cache.oui_map.insert(entry.oui_prefix.to_uppercase(), entry);
        }

        // Build templates map by brand
        cache.templates.clear();
        for template in templates {
            cache
                .templates
                .entry(template.brand_id)
                .or_insert_with(Vec::new)
                .push(template);
        }

        // Store generic paths
        cache.generic_paths = generic_paths;
        cache.last_updated = Some(Utc::now());

        info!(
            "Cache refreshed: {} OUIs, {} brands with templates, {} generic paths",
            cache.oui_map.len(),
            cache.templates.len(),
            cache.generic_paths.len()
        );

        Ok(())
    }

    // ========================================================================
    // Fast Lookup Methods (from cache)
    // ========================================================================

    /// Look up brand info by MAC address OUI prefix
    /// Returns None if OUI not found in cache
    pub async fn lookup_oui(&self, mac_address: &str) -> Option<OuiBrandInfo> {
        let oui_prefix = Self::extract_oui_prefix(mac_address)?;
        let cache = self.cache.read().await;
        cache.oui_map.get(&oui_prefix).cloned()
    }

    /// Get RTSP templates for a brand (from cache)
    pub async fn get_templates_for_brand(&self, brand_id: i32) -> Vec<RtspTemplateInfo> {
        let cache = self.cache.read().await;
        cache.templates.get(&brand_id).cloned().unwrap_or_default()
    }

    /// Get default RTSP template for a brand (from cache)
    pub async fn get_default_template(&self, brand_id: i32) -> Option<RtspTemplateInfo> {
        let cache = self.cache.read().await;
        cache
            .templates
            .get(&brand_id)?
            .iter()
            .find(|t| t.is_default)
            .cloned()
    }

    /// Get generic RTSP paths in priority order (from cache)
    pub async fn get_generic_paths(&self) -> Vec<GenericRtspPath> {
        let cache = self.cache.read().await;
        cache.generic_paths.clone()
    }

    /// Generate RTSP URL from template
    pub fn generate_rtsp_url(
        template: &RtspTemplateInfo,
        ip: &str,
        port: Option<u16>,
        username: &str,
        password: &str,
    ) -> (String, Option<String>) {
        let actual_port = port.unwrap_or(template.default_port as u16);

        let main_url = if username.is_empty() {
            format!("rtsp://{}:{}{}", ip, actual_port, template.main_path)
        } else {
            format!(
                "rtsp://{}:{}@{}:{}{}",
                username, password, ip, actual_port, template.main_path
            )
        };

        let sub_url = template.sub_path.as_ref().map(|sub_path| {
            if username.is_empty() {
                format!("rtsp://{}:{}{}", ip, actual_port, sub_path)
            } else {
                format!(
                    "rtsp://{}:{}@{}:{}{}",
                    username, password, ip, actual_port, sub_path
                )
            }
        });

        (main_url, sub_url)
    }

    /// Generate RTSP URL from generic path
    pub fn generate_generic_rtsp_url(
        path: &GenericRtspPath,
        ip: &str,
        port: Option<u16>,
        username: &str,
        password: &str,
    ) -> (String, Option<String>) {
        let actual_port = port.unwrap_or(554);

        let main_url = if username.is_empty() {
            format!("rtsp://{}:{}{}", ip, actual_port, path.main_path)
        } else {
            format!(
                "rtsp://{}:{}@{}:{}{}",
                username, password, ip, actual_port, path.main_path
            )
        };

        let sub_url = path.sub_path.as_ref().map(|sub_path| {
            if username.is_empty() {
                format!("rtsp://{}:{}{}", ip, actual_port, sub_path)
            } else {
                format!(
                    "rtsp://{}:{}@{}:{}{}",
                    username, password, ip, actual_port, sub_path
                )
            }
        });

        (main_url, sub_url)
    }

    // ========================================================================
    // Brand CRUD (with cache invalidation)
    // ========================================================================

    /// List all brands
    pub async fn list_brands(&self) -> Result<Vec<CameraBrand>> {
        self.repo.get_all_brands().await
    }

    /// List all brands with counts
    pub async fn list_brands_with_counts(&self) -> Result<Vec<CameraBrandWithCounts>> {
        self.repo.get_all_brands_with_counts().await
    }

    /// Get brand by ID
    pub async fn get_brand(&self, id: i32) -> Result<Option<CameraBrand>> {
        self.repo.get_brand(id).await
    }

    /// Create brand
    pub async fn create_brand(&self, req: CreateBrandRequest) -> Result<CameraBrand> {
        req.validate().map_err(|e| Error::Validation(e))?;

        // Check for duplicate name
        if self.repo.get_brand_by_name(&req.name).await?.is_some() {
            return Err(Error::Conflict(format!(
                "Brand with name '{}' already exists",
                req.name
            )));
        }

        let brand = self.repo.create_brand(&req).await?;
        // No cache refresh needed - new brand has no OUIs/templates yet
        Ok(brand)
    }

    /// Update brand
    pub async fn update_brand(&self, id: i32, req: UpdateBrandRequest) -> Result<CameraBrand> {
        let brand = self.repo.get_brand(id).await?.ok_or_else(|| {
            Error::NotFound(format!("Brand {} not found", id))
        })?;

        // Check is_builtin constraint
        if brand.is_builtin {
            return Err(Error::Forbidden(format!(
                "Brand '{}' is built-in and cannot be modified",
                brand.display_name
            )));
        }

        // Check for duplicate name if changing
        if let Some(ref new_name) = req.name {
            if new_name != &brand.name {
                if self.repo.get_brand_by_name(new_name).await?.is_some() {
                    return Err(Error::Conflict(format!(
                        "Brand with name '{}' already exists",
                        new_name
                    )));
                }
            }
        }

        let updated = self.repo.update_brand(id, &req).await?;
        self.refresh_cache().await?;
        Ok(updated)
    }

    /// Delete brand
    pub async fn delete_brand(&self, id: i32) -> Result<()> {
        let brand = self.repo.get_brand(id).await?.ok_or_else(|| {
            Error::NotFound(format!("Brand {} not found", id))
        })?;

        // Check is_builtin constraint
        if brand.is_builtin {
            return Err(Error::Forbidden(format!(
                "Brand '{}' is built-in and cannot be deleted",
                brand.display_name
            )));
        }

        self.repo.delete_brand(id).await?;
        self.refresh_cache().await?;
        Ok(())
    }

    // ========================================================================
    // OUI Entry CRUD (with cache invalidation)
    // ========================================================================

    /// List all OUI entries
    pub async fn list_oui_entries(&self) -> Result<Vec<OuiEntryWithBrand>> {
        self.repo.get_all_oui_entries().await
    }

    /// Get OUI entries for a brand
    pub async fn get_oui_entries_for_brand(&self, brand_id: i32) -> Result<Vec<OuiEntry>> {
        // Verify brand exists
        if self.repo.get_brand(brand_id).await?.is_none() {
            return Err(Error::NotFound(format!("Brand {} not found", brand_id)));
        }
        self.repo.get_oui_entries_for_brand(brand_id).await
    }

    /// Get OUI entry by prefix
    pub async fn get_oui_entry(&self, oui_prefix: &str) -> Result<Option<OuiEntry>> {
        self.repo.get_oui_entry(oui_prefix).await
    }

    /// Add OUI entry
    pub async fn add_oui_entry(&self, brand_id: i32, req: AddOuiRequest) -> Result<OuiEntry> {
        req.validate().map_err(|e| Error::Validation(e))?;

        // Verify brand exists
        if self.repo.get_brand(brand_id).await?.is_none() {
            return Err(Error::NotFound(format!("Brand {} not found", brand_id)));
        }

        // Check for duplicate OUI
        if self.repo.get_oui_entry(&req.oui_prefix).await?.is_some() {
            return Err(Error::Conflict(format!(
                "OUI '{}' already exists",
                req.oui_prefix.to_uppercase()
            )));
        }

        let entry = self.repo.add_oui_entry(brand_id, &req).await?;
        self.refresh_cache().await?;
        Ok(entry)
    }

    /// Update OUI entry
    pub async fn update_oui_entry(&self, oui_prefix: &str, req: UpdateOuiRequest) -> Result<OuiEntry> {
        let entry = self.repo.get_oui_entry(oui_prefix).await?.ok_or_else(|| {
            Error::NotFound(format!("OUI '{}' not found", oui_prefix))
        })?;

        // Check is_builtin constraint
        if entry.is_builtin {
            return Err(Error::Forbidden(format!(
                "OUI '{}' is built-in and cannot be modified",
                oui_prefix
            )));
        }

        let updated = self.repo.update_oui_entry(oui_prefix, &req).await?;
        self.refresh_cache().await?;
        Ok(updated)
    }

    /// Delete OUI entry
    pub async fn delete_oui_entry(&self, oui_prefix: &str) -> Result<()> {
        let entry = self.repo.get_oui_entry(oui_prefix).await?.ok_or_else(|| {
            Error::NotFound(format!("OUI '{}' not found", oui_prefix))
        })?;

        // Check is_builtin constraint
        if entry.is_builtin {
            return Err(Error::Forbidden(format!(
                "OUI '{}' is built-in and cannot be deleted",
                oui_prefix
            )));
        }

        self.repo.delete_oui_entry(oui_prefix).await?;
        self.refresh_cache().await?;
        Ok(())
    }

    // ========================================================================
    // RTSP Template CRUD (with cache invalidation)
    // ========================================================================

    /// List all RTSP templates
    pub async fn list_templates(&self) -> Result<Vec<RtspTemplateWithBrand>> {
        self.repo.get_all_templates().await
    }

    /// Get RTSP templates for a brand (from DB, not cache)
    pub async fn get_templates_for_brand_db(&self, brand_id: i32) -> Result<Vec<RtspTemplate>> {
        // Verify brand exists
        if self.repo.get_brand(brand_id).await?.is_none() {
            return Err(Error::NotFound(format!("Brand {} not found", brand_id)));
        }
        self.repo.get_templates_for_brand(brand_id).await
    }

    /// Get RTSP template by ID
    pub async fn get_template(&self, id: i32) -> Result<Option<RtspTemplate>> {
        self.repo.get_template(id).await
    }

    /// Add RTSP template
    pub async fn add_template(&self, brand_id: i32, req: AddTemplateRequest) -> Result<RtspTemplate> {
        req.validate().map_err(|e| Error::Validation(e))?;

        // Verify brand exists
        if self.repo.get_brand(brand_id).await?.is_none() {
            return Err(Error::NotFound(format!("Brand {} not found", brand_id)));
        }

        let template = self.repo.add_template(brand_id, &req).await?;
        self.refresh_cache().await?;
        Ok(template)
    }

    /// Update RTSP template
    pub async fn update_template(&self, id: i32, req: UpdateTemplateRequest) -> Result<RtspTemplate> {
        let template = self.repo.get_template(id).await?.ok_or_else(|| {
            Error::NotFound(format!("Template {} not found", id))
        })?;

        // Check is_builtin constraint
        if template.is_builtin {
            return Err(Error::Forbidden(format!(
                "Template '{}' is built-in and cannot be modified",
                template.name
            )));
        }

        let updated = self.repo.update_template(id, &req).await?;
        self.refresh_cache().await?;
        Ok(updated)
    }

    /// Delete RTSP template
    pub async fn delete_template(&self, id: i32) -> Result<()> {
        let template = self.repo.get_template(id).await?.ok_or_else(|| {
            Error::NotFound(format!("Template {} not found", id))
        })?;

        // Check is_builtin constraint
        if template.is_builtin {
            return Err(Error::Forbidden(format!(
                "Template '{}' is built-in and cannot be deleted",
                template.name
            )));
        }

        self.repo.delete_template(id).await?;
        self.refresh_cache().await?;
        Ok(())
    }

    // ========================================================================
    // Generic Path CRUD (with cache invalidation)
    // ========================================================================

    /// List all generic paths
    pub async fn list_generic_paths(&self) -> Result<Vec<GenericRtspPath>> {
        self.repo.get_all_generic_paths().await
    }

    /// Get generic path by ID
    pub async fn get_generic_path(&self, id: i32) -> Result<Option<GenericRtspPath>> {
        self.repo.get_generic_path(id).await
    }

    /// Add generic path
    pub async fn add_generic_path(&self, req: AddGenericPathRequest) -> Result<GenericRtspPath> {
        req.validate().map_err(|e| Error::Validation(e))?;
        let path = self.repo.add_generic_path(&req).await?;
        self.refresh_cache().await?;
        Ok(path)
    }

    /// Update generic path
    pub async fn update_generic_path(&self, id: i32, req: UpdateGenericPathRequest) -> Result<GenericRtspPath> {
        if self.repo.get_generic_path(id).await?.is_none() {
            return Err(Error::NotFound(format!("Generic path {} not found", id)));
        }

        let updated = self.repo.update_generic_path(id, &req).await?;
        self.refresh_cache().await?;
        Ok(updated)
    }

    /// Delete generic path
    pub async fn delete_generic_path(&self, id: i32) -> Result<()> {
        if self.repo.get_generic_path(id).await?.is_none() {
            return Err(Error::NotFound(format!("Generic path {} not found", id)));
        }

        self.repo.delete_generic_path(id).await?;
        self.refresh_cache().await?;
        Ok(())
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Extract OUI prefix from MAC address
    /// Supports formats: "XX:XX:XX:XX:XX:XX", "XX-XX-XX-XX-XX-XX", "XXXXXXXXXXXX"
    fn extract_oui_prefix(mac: &str) -> Option<String> {
        let cleaned: String = mac
            .to_uppercase()
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect();

        if cleaned.len() >= 6 {
            Some(format!(
                "{}:{}:{}",
                &cleaned[0..2],
                &cleaned[2..4],
                &cleaned[4..6]
            ))
        } else {
            None
        }
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            oui_count: cache.oui_map.len(),
            brand_count: cache.templates.len(),
            generic_path_count: cache.generic_paths.len(),
            last_updated: cache.last_updated,
        }
    }

    /// Export OUI map for scanner use
    /// Returns HashMap<OUI prefix, brand display name>
    /// This is used by the scanner module for synchronous OUI lookups
    pub async fn export_oui_map(&self) -> HashMap<String, String> {
        let cache = self.cache.read().await;
        cache.oui_map.iter()
            .map(|(k, v)| (k.clone(), v.brand_display_name.clone()))
            .collect()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub oui_count: usize,
    pub brand_count: usize,
    pub generic_path_count: usize,
    pub last_updated: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_oui_prefix() {
        // Standard colon format
        assert_eq!(
            CameraBrandService::extract_oui_prefix("70:5A:0F:12:34:56"),
            Some("70:5A:0F".to_string())
        );

        // Dash format
        assert_eq!(
            CameraBrandService::extract_oui_prefix("70-5A-0F-12-34-56"),
            Some("70:5A:0F".to_string())
        );

        // No separator
        assert_eq!(
            CameraBrandService::extract_oui_prefix("705A0F123456"),
            Some("70:5A:0F".to_string())
        );

        // Lowercase
        assert_eq!(
            CameraBrandService::extract_oui_prefix("70:5a:0f:12:34:56"),
            Some("70:5A:0F".to_string())
        );

        // Too short
        assert_eq!(CameraBrandService::extract_oui_prefix("70:5A"), None);

        // Empty
        assert_eq!(CameraBrandService::extract_oui_prefix(""), None);
    }
}
