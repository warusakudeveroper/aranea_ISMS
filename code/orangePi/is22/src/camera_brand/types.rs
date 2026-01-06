//! Camera Brand Types
//!
//! Data structures for camera brands, OUI entries, and RTSP templates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Database Entities
// ============================================================================

/// Camera brand entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CameraBrand {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub is_builtin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// OUI entry entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OuiEntry {
    pub oui_prefix: String,
    pub brand_id: i32,
    pub description: Option<String>,
    pub score_bonus: i32,
    pub status: String,
    pub verification_source: Option<String>,
    pub is_builtin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// RTSP template entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RtspTemplate {
    pub id: i32,
    pub brand_id: i32,
    pub name: String,
    pub main_path: String,
    pub sub_path: Option<String>,
    pub default_port: i32,
    pub is_default: bool,
    pub priority: i32,
    pub notes: Option<String>,
    pub is_builtin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Generic RTSP path entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GenericRtspPath {
    pub id: i32,
    pub main_path: String,
    pub sub_path: Option<String>,
    pub description: Option<String>,
    pub priority: i32,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Cache Structures (for high-performance lookups)
// ============================================================================

/// OUI lookup result with brand information
#[derive(Debug, Clone, Serialize)]
pub struct OuiBrandInfo {
    pub oui_prefix: String,
    pub brand_id: i32,
    pub brand_name: String,
    pub brand_display_name: String,
    pub category: String,
    pub score_bonus: i32,
    pub status: String,
}

/// RTSP template info for URL generation
#[derive(Debug, Clone, Serialize)]
pub struct RtspTemplateInfo {
    pub id: i32,
    pub brand_id: i32,
    pub brand_name: String,
    pub name: String,
    pub main_path: String,
    pub sub_path: Option<String>,
    pub default_port: i32,
    pub is_default: bool,
    pub priority: i32,
}

// ============================================================================
// API Request/Response Types
// ============================================================================

/// Create brand request
#[derive(Debug, Deserialize)]
pub struct CreateBrandRequest {
    pub name: String,
    pub display_name: String,
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_category() -> String {
    "unknown".to_string()
}

/// Update brand request
#[derive(Debug, Deserialize)]
pub struct UpdateBrandRequest {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub category: Option<String>,
}

/// Add OUI entry request
#[derive(Debug, Deserialize)]
pub struct AddOuiRequest {
    pub oui_prefix: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_score_bonus")]
    pub score_bonus: i32,
    #[serde(default = "default_oui_status")]
    pub status: String,
    #[serde(default)]
    pub verification_source: Option<String>,
}

fn default_score_bonus() -> i32 {
    20
}

fn default_oui_status() -> String {
    "confirmed".to_string()
}

/// Update OUI entry request
#[derive(Debug, Deserialize)]
pub struct UpdateOuiRequest {
    pub description: Option<String>,
    pub score_bonus: Option<i32>,
    pub status: Option<String>,
    pub verification_source: Option<String>,
}

/// Add RTSP template request
#[derive(Debug, Deserialize)]
pub struct AddTemplateRequest {
    pub name: String,
    pub main_path: String,
    #[serde(default)]
    pub sub_path: Option<String>,
    #[serde(default = "default_port")]
    pub default_port: i32,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_port() -> i32 {
    554
}

fn default_priority() -> i32 {
    100
}

/// Update RTSP template request
#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub main_path: Option<String>,
    pub sub_path: Option<Option<String>>,
    pub default_port: Option<i32>,
    pub is_default: Option<bool>,
    pub priority: Option<i32>,
    pub notes: Option<Option<String>>,
}

/// Add generic path request
#[derive(Debug, Deserialize)]
pub struct AddGenericPathRequest {
    pub main_path: String,
    #[serde(default)]
    pub sub_path: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default = "default_enabled")]
    pub is_enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Update generic path request
#[derive(Debug, Deserialize)]
pub struct UpdateGenericPathRequest {
    pub main_path: Option<String>,
    pub sub_path: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub priority: Option<i32>,
    pub is_enabled: Option<bool>,
}

// ============================================================================
// Extended Response Types
// ============================================================================

/// Brand with OUI count and template count
#[derive(Debug, Clone, Serialize)]
pub struct CameraBrandWithCounts {
    #[serde(flatten)]
    pub brand: CameraBrand,
    pub oui_count: i64,
    pub template_count: i64,
}

/// OUI entry with brand name
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct OuiEntryWithBrand {
    pub oui_prefix: String,
    pub brand_id: i32,
    pub brand_name: String,
    pub brand_display_name: String,
    pub description: Option<String>,
    pub score_bonus: i32,
    pub status: String,
    pub verification_source: Option<String>,
    pub is_builtin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// RTSP template with brand name
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RtspTemplateWithBrand {
    pub id: i32,
    pub brand_id: i32,
    pub brand_name: String,
    pub brand_display_name: String,
    pub name: String,
    pub main_path: String,
    pub sub_path: Option<String>,
    pub default_port: i32,
    pub is_default: bool,
    pub priority: i32,
    pub notes: Option<String>,
    pub is_builtin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Validation
// ============================================================================

impl CreateBrandRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() || self.name.len() > 100 {
            return Err("name must be 1-100 characters".to_string());
        }
        if self.display_name.is_empty() || self.display_name.len() > 100 {
            return Err("display_name must be 1-100 characters".to_string());
        }
        let valid_categories = ["consumer", "professional", "enterprise", "unknown"];
        if !valid_categories.contains(&self.category.as_str()) {
            return Err(format!("category must be one of: {:?}", valid_categories));
        }
        Ok(())
    }
}

impl AddOuiRequest {
    pub fn validate(&self) -> Result<(), String> {
        // OUI format: XX:XX:XX (uppercase hex)
        let oui_regex = regex::Regex::new(r"^[0-9A-Fa-f]{2}:[0-9A-Fa-f]{2}:[0-9A-Fa-f]{2}$").unwrap();
        if !oui_regex.is_match(&self.oui_prefix) {
            return Err("oui_prefix must be in XX:XX:XX format".to_string());
        }
        let valid_statuses = ["confirmed", "candidate", "investigating"];
        if !valid_statuses.contains(&self.status.as_str()) {
            return Err(format!("status must be one of: {:?}", valid_statuses));
        }
        Ok(())
    }
}

impl AddTemplateRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() || self.name.len() > 100 {
            return Err("name must be 1-100 characters".to_string());
        }
        if self.main_path.is_empty() || !self.main_path.starts_with('/') {
            return Err("main_path must start with /".to_string());
        }
        if let Some(ref sub) = self.sub_path {
            if !sub.starts_with('/') {
                return Err("sub_path must start with /".to_string());
            }
        }
        if self.default_port < 1 || self.default_port > 65535 {
            return Err("default_port must be 1-65535".to_string());
        }
        Ok(())
    }
}

impl AddGenericPathRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.main_path.is_empty() || !self.main_path.starts_with('/') {
            return Err("main_path must start with /".to_string());
        }
        if let Some(ref sub) = self.sub_path {
            if !sub.starts_with('/') {
                return Err("sub_path must start with /".to_string());
            }
        }
        Ok(())
    }
}
