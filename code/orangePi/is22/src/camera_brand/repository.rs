//! Camera Brand Repository
//!
//! Database access layer for camera brands, OUI entries, and RTSP templates.

use super::types::*;
use crate::error::{Error, Result};
use sqlx::MySqlPool;

/// Camera brand repository for database operations
#[derive(Clone)]
pub struct CameraBrandRepository {
    pool: MySqlPool,
}

impl CameraBrandRepository {
    /// Create new repository
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Camera Brands
    // ========================================================================

    /// Get all brands
    pub async fn get_all_brands(&self) -> Result<Vec<CameraBrand>> {
        let brands = sqlx::query_as::<_, CameraBrand>(
            r#"SELECT id, name, display_name, category, is_builtin, created_at, updated_at
               FROM camera_brands
               ORDER BY is_builtin DESC, display_name"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(brands)
    }

    /// Get all brands with counts
    pub async fn get_all_brands_with_counts(&self) -> Result<Vec<CameraBrandWithCounts>> {
        let rows = sqlx::query_as::<_, CameraBrand>(
            r#"SELECT id, name, display_name, category, is_builtin, created_at, updated_at
               FROM camera_brands
               ORDER BY is_builtin DESC, display_name"#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for brand in rows {
            let oui_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM oui_entries WHERE brand_id = ?",
            )
            .bind(brand.id)
            .fetch_one(&self.pool)
            .await?;

            let template_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM rtsp_templates WHERE brand_id = ?",
            )
            .bind(brand.id)
            .fetch_one(&self.pool)
            .await?;

            result.push(CameraBrandWithCounts {
                brand,
                oui_count: oui_count.0,
                template_count: template_count.0,
            });
        }
        Ok(result)
    }

    /// Get brand by ID
    pub async fn get_brand(&self, id: i32) -> Result<Option<CameraBrand>> {
        let brand = sqlx::query_as::<_, CameraBrand>(
            r#"SELECT id, name, display_name, category, is_builtin, created_at, updated_at
               FROM camera_brands WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(brand)
    }

    /// Get brand by name
    pub async fn get_brand_by_name(&self, name: &str) -> Result<Option<CameraBrand>> {
        let brand = sqlx::query_as::<_, CameraBrand>(
            r#"SELECT id, name, display_name, category, is_builtin, created_at, updated_at
               FROM camera_brands WHERE name = ?"#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(brand)
    }

    /// Create brand
    pub async fn create_brand(&self, req: &CreateBrandRequest) -> Result<CameraBrand> {
        let result = sqlx::query(
            r#"INSERT INTO camera_brands (name, display_name, category, is_builtin)
               VALUES (?, ?, ?, FALSE)"#,
        )
        .bind(&req.name)
        .bind(&req.display_name)
        .bind(&req.category)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_id() as i32;
        self.get_brand(id)
            .await?
            .ok_or_else(|| Error::Internal("Failed to get created brand".to_string()))
    }

    /// Update brand
    pub async fn update_brand(&self, id: i32, req: &UpdateBrandRequest) -> Result<CameraBrand> {
        let current = self.get_brand(id).await?.ok_or_else(|| {
            Error::NotFound(format!("Brand {} not found", id))
        })?;

        let name = req.name.as_ref().unwrap_or(&current.name);
        let display_name = req.display_name.as_ref().unwrap_or(&current.display_name);
        let category = req.category.as_ref().unwrap_or(&current.category);

        sqlx::query(
            r#"UPDATE camera_brands
               SET name = ?, display_name = ?, category = ?
               WHERE id = ?"#,
        )
        .bind(name)
        .bind(display_name)
        .bind(category)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_brand(id)
            .await?
            .ok_or_else(|| Error::Internal("Failed to get updated brand".to_string()))
    }

    /// Delete brand
    pub async fn delete_brand(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM camera_brands WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================================================================
    // OUI Entries
    // ========================================================================

    /// Get all OUI entries
    pub async fn get_all_oui_entries(&self) -> Result<Vec<OuiEntryWithBrand>> {
        let entries = sqlx::query_as::<_, OuiEntryWithBrand>(
            r#"SELECT o.oui_prefix, o.brand_id, b.name as brand_name, b.display_name as brand_display_name,
                      o.description, o.score_bonus, o.status, o.verification_source,
                      o.is_builtin, o.created_at, o.updated_at
               FROM oui_entries o
               JOIN camera_brands b ON o.brand_id = b.id
               ORDER BY b.display_name, o.oui_prefix"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(entries)
    }

    /// Get OUI entries for a brand
    pub async fn get_oui_entries_for_brand(&self, brand_id: i32) -> Result<Vec<OuiEntry>> {
        let entries = sqlx::query_as::<_, OuiEntry>(
            r#"SELECT oui_prefix, brand_id, description, score_bonus, status,
                      verification_source, is_builtin, created_at, updated_at
               FROM oui_entries
               WHERE brand_id = ?
               ORDER BY oui_prefix"#,
        )
        .bind(brand_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(entries)
    }

    /// Get OUI entry by prefix
    pub async fn get_oui_entry(&self, oui_prefix: &str) -> Result<Option<OuiEntry>> {
        let entry = sqlx::query_as::<_, OuiEntry>(
            r#"SELECT oui_prefix, brand_id, description, score_bonus, status,
                      verification_source, is_builtin, created_at, updated_at
               FROM oui_entries
               WHERE oui_prefix = ?"#,
        )
        .bind(oui_prefix.to_uppercase())
        .fetch_optional(&self.pool)
        .await?;
        Ok(entry)
    }

    /// Add OUI entry
    pub async fn add_oui_entry(&self, brand_id: i32, req: &AddOuiRequest) -> Result<OuiEntry> {
        let oui_upper = req.oui_prefix.to_uppercase();
        sqlx::query(
            r#"INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
               VALUES (?, ?, ?, ?, ?, ?, FALSE)"#,
        )
        .bind(&oui_upper)
        .bind(brand_id)
        .bind(&req.description)
        .bind(req.score_bonus)
        .bind(&req.status)
        .bind(&req.verification_source)
        .execute(&self.pool)
        .await?;

        self.get_oui_entry(&oui_upper)
            .await?
            .ok_or_else(|| Error::Internal("Failed to get created OUI entry".to_string()))
    }

    /// Update OUI entry
    pub async fn update_oui_entry(&self, oui_prefix: &str, req: &UpdateOuiRequest) -> Result<OuiEntry> {
        let oui_upper = oui_prefix.to_uppercase();
        let current = self.get_oui_entry(&oui_upper).await?.ok_or_else(|| {
            Error::NotFound(format!("OUI {} not found", oui_upper))
        })?;

        let description = req.description.as_ref().or(current.description.as_ref());
        let score_bonus = req.score_bonus.unwrap_or(current.score_bonus);
        let status = req.status.as_ref().unwrap_or(&current.status);
        let verification_source = req.verification_source.as_ref().or(current.verification_source.as_ref());

        sqlx::query(
            r#"UPDATE oui_entries
               SET description = ?, score_bonus = ?, status = ?, verification_source = ?
               WHERE oui_prefix = ?"#,
        )
        .bind(description)
        .bind(score_bonus)
        .bind(status)
        .bind(verification_source)
        .bind(&oui_upper)
        .execute(&self.pool)
        .await?;

        self.get_oui_entry(&oui_upper)
            .await?
            .ok_or_else(|| Error::Internal("Failed to get updated OUI entry".to_string()))
    }

    /// Delete OUI entry
    pub async fn delete_oui_entry(&self, oui_prefix: &str) -> Result<()> {
        sqlx::query("DELETE FROM oui_entries WHERE oui_prefix = ?")
            .bind(oui_prefix.to_uppercase())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================================================================
    // RTSP Templates
    // ========================================================================

    /// Get all RTSP templates
    pub async fn get_all_templates(&self) -> Result<Vec<RtspTemplateWithBrand>> {
        let templates = sqlx::query_as::<_, RtspTemplateWithBrand>(
            r#"SELECT t.id, t.brand_id, b.name as brand_name, b.display_name as brand_display_name,
                      t.name, t.main_path, t.sub_path, t.default_port, t.is_default, t.priority,
                      t.notes, t.is_builtin, t.created_at, t.updated_at
               FROM rtsp_templates t
               JOIN camera_brands b ON t.brand_id = b.id
               ORDER BY b.display_name, t.priority, t.name"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(templates)
    }

    /// Get RTSP templates for a brand
    pub async fn get_templates_for_brand(&self, brand_id: i32) -> Result<Vec<RtspTemplate>> {
        let templates = sqlx::query_as::<_, RtspTemplate>(
            r#"SELECT id, brand_id, name, main_path, sub_path, default_port,
                      is_default, priority, notes, is_builtin, created_at, updated_at
               FROM rtsp_templates
               WHERE brand_id = ?
               ORDER BY priority, name"#,
        )
        .bind(brand_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(templates)
    }

    /// Get RTSP template by ID
    pub async fn get_template(&self, id: i32) -> Result<Option<RtspTemplate>> {
        let template = sqlx::query_as::<_, RtspTemplate>(
            r#"SELECT id, brand_id, name, main_path, sub_path, default_port,
                      is_default, priority, notes, is_builtin, created_at, updated_at
               FROM rtsp_templates
               WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(template)
    }

    /// Get default RTSP template for a brand
    pub async fn get_default_template(&self, brand_id: i32) -> Result<Option<RtspTemplate>> {
        let template = sqlx::query_as::<_, RtspTemplate>(
            r#"SELECT id, brand_id, name, main_path, sub_path, default_port,
                      is_default, priority, notes, is_builtin, created_at, updated_at
               FROM rtsp_templates
               WHERE brand_id = ? AND is_default = TRUE
               LIMIT 1"#,
        )
        .bind(brand_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(template)
    }

    /// Add RTSP template
    pub async fn add_template(&self, brand_id: i32, req: &AddTemplateRequest) -> Result<RtspTemplate> {
        // If this is marked as default, unset other defaults for this brand
        if req.is_default {
            sqlx::query("UPDATE rtsp_templates SET is_default = FALSE WHERE brand_id = ?")
                .bind(brand_id)
                .execute(&self.pool)
                .await?;
        }

        let result = sqlx::query(
            r#"INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, notes, is_builtin)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, FALSE)"#,
        )
        .bind(brand_id)
        .bind(&req.name)
        .bind(&req.main_path)
        .bind(&req.sub_path)
        .bind(req.default_port)
        .bind(req.is_default)
        .bind(req.priority)
        .bind(&req.notes)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_id() as i32;
        self.get_template(id)
            .await?
            .ok_or_else(|| Error::Internal("Failed to get created template".to_string()))
    }

    /// Update RTSP template
    pub async fn update_template(&self, id: i32, req: &UpdateTemplateRequest) -> Result<RtspTemplate> {
        let current = self.get_template(id).await?.ok_or_else(|| {
            Error::NotFound(format!("Template {} not found", id))
        })?;

        // If setting as default, unset other defaults
        if req.is_default == Some(true) {
            sqlx::query("UPDATE rtsp_templates SET is_default = FALSE WHERE brand_id = ? AND id != ?")
                .bind(current.brand_id)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        let name = req.name.as_ref().unwrap_or(&current.name);
        let main_path = req.main_path.as_ref().unwrap_or(&current.main_path);
        let sub_path = match &req.sub_path {
            Some(v) => v.clone(),
            None => current.sub_path.clone(),
        };
        let default_port = req.default_port.unwrap_or(current.default_port);
        let is_default = req.is_default.unwrap_or(current.is_default);
        let priority = req.priority.unwrap_or(current.priority);
        let notes = match &req.notes {
            Some(v) => v.clone(),
            None => current.notes.clone(),
        };

        sqlx::query(
            r#"UPDATE rtsp_templates
               SET name = ?, main_path = ?, sub_path = ?, default_port = ?,
                   is_default = ?, priority = ?, notes = ?
               WHERE id = ?"#,
        )
        .bind(name)
        .bind(main_path)
        .bind(&sub_path)
        .bind(default_port)
        .bind(is_default)
        .bind(priority)
        .bind(&notes)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_template(id)
            .await?
            .ok_or_else(|| Error::Internal("Failed to get updated template".to_string()))
    }

    /// Delete RTSP template
    pub async fn delete_template(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM rtsp_templates WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================================================================
    // Generic RTSP Paths
    // ========================================================================

    /// Get all generic paths
    pub async fn get_all_generic_paths(&self) -> Result<Vec<GenericRtspPath>> {
        let paths = sqlx::query_as::<_, GenericRtspPath>(
            r#"SELECT id, main_path, sub_path, description, priority, is_enabled, created_at, updated_at
               FROM generic_rtsp_paths
               ORDER BY priority"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(paths)
    }

    /// Get enabled generic paths (for scanning)
    pub async fn get_enabled_generic_paths(&self) -> Result<Vec<GenericRtspPath>> {
        let paths = sqlx::query_as::<_, GenericRtspPath>(
            r#"SELECT id, main_path, sub_path, description, priority, is_enabled, created_at, updated_at
               FROM generic_rtsp_paths
               WHERE is_enabled = TRUE
               ORDER BY priority"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(paths)
    }

    /// Get generic path by ID
    pub async fn get_generic_path(&self, id: i32) -> Result<Option<GenericRtspPath>> {
        let path = sqlx::query_as::<_, GenericRtspPath>(
            r#"SELECT id, main_path, sub_path, description, priority, is_enabled, created_at, updated_at
               FROM generic_rtsp_paths
               WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(path)
    }

    /// Add generic path
    pub async fn add_generic_path(&self, req: &AddGenericPathRequest) -> Result<GenericRtspPath> {
        let result = sqlx::query(
            r#"INSERT INTO generic_rtsp_paths (main_path, sub_path, description, priority, is_enabled)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(&req.main_path)
        .bind(&req.sub_path)
        .bind(&req.description)
        .bind(req.priority)
        .bind(req.is_enabled)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_id() as i32;
        self.get_generic_path(id)
            .await?
            .ok_or_else(|| Error::Internal("Failed to get created generic path".to_string()))
    }

    /// Update generic path
    pub async fn update_generic_path(&self, id: i32, req: &UpdateGenericPathRequest) -> Result<GenericRtspPath> {
        let current = self.get_generic_path(id).await?.ok_or_else(|| {
            Error::NotFound(format!("Generic path {} not found", id))
        })?;

        let main_path = req.main_path.as_ref().unwrap_or(&current.main_path);
        let sub_path = match &req.sub_path {
            Some(v) => v.clone(),
            None => current.sub_path.clone(),
        };
        let description = match &req.description {
            Some(v) => v.clone(),
            None => current.description.clone(),
        };
        let priority = req.priority.unwrap_or(current.priority);
        let is_enabled = req.is_enabled.unwrap_or(current.is_enabled);

        sqlx::query(
            r#"UPDATE generic_rtsp_paths
               SET main_path = ?, sub_path = ?, description = ?, priority = ?, is_enabled = ?
               WHERE id = ?"#,
        )
        .bind(main_path)
        .bind(&sub_path)
        .bind(&description)
        .bind(priority)
        .bind(is_enabled)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_generic_path(id)
            .await?
            .ok_or_else(|| Error::Internal("Failed to get updated generic path".to_string()))
    }

    /// Delete generic path
    pub async fn delete_generic_path(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM generic_rtsp_paths WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================================================================
    // Cache Loading (for high-performance lookups)
    // ========================================================================

    /// Load all OUI entries with brand info (for cache)
    pub async fn load_oui_brand_map(&self) -> Result<Vec<OuiBrandInfo>> {
        let entries = sqlx::query_as::<_, (String, i32, String, String, String, i32, String)>(
            r#"SELECT o.oui_prefix, o.brand_id, b.name, b.display_name, b.category, o.score_bonus, o.status
               FROM oui_entries o
               JOIN camera_brands b ON o.brand_id = b.id
               WHERE o.status IN ('confirmed', 'candidate')"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(entries
            .into_iter()
            .map(|(oui_prefix, brand_id, brand_name, brand_display_name, category, score_bonus, status)| {
                OuiBrandInfo {
                    oui_prefix,
                    brand_id,
                    brand_name,
                    brand_display_name,
                    category,
                    score_bonus,
                    status,
                }
            })
            .collect())
    }

    /// Load all RTSP templates with brand info (for cache)
    pub async fn load_rtsp_templates(&self) -> Result<Vec<RtspTemplateInfo>> {
        let templates = sqlx::query_as::<_, (i32, i32, String, String, String, Option<String>, i32, bool, i32)>(
            r#"SELECT t.id, t.brand_id, b.name, t.name, t.main_path, t.sub_path, t.default_port, t.is_default, t.priority
               FROM rtsp_templates t
               JOIN camera_brands b ON t.brand_id = b.id
               ORDER BY t.brand_id, t.priority"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(templates
            .into_iter()
            .map(|(id, brand_id, brand_name, name, main_path, sub_path, default_port, is_default, priority)| {
                RtspTemplateInfo {
                    id,
                    brand_id,
                    brand_name,
                    name,
                    main_path,
                    sub_path,
                    default_port,
                    is_default,
                    priority,
                }
            })
            .collect())
    }
}
