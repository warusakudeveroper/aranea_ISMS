//! SDM Integration Types
//!
//! Type definitions for Google Smart Device Management (SDM) integration.
//! Used for Nest Doorbell and other Google Home devices.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ========================================
// SDM Configuration (SSoT: settings.sdm_config)
// ========================================

/// SDM configuration stored in settings table
///
/// SSoT: `settings.sdm_config` is the single source of truth.
/// `/etc/is22/secrets/sdm.env` is for initial input only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdmConfig {
    /// Google Cloud Project ID (string form, e.g., "my-sdm-project")
    pub project_id: String,
    /// Google Cloud Project Number (numeric form)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_number: Option<String>,
    /// Device Access enterprise project ID (enterprises/xxx)
    pub enterprise_project_id: String,
    /// OAuth 2.0 Client ID
    pub client_id: String,
    /// OAuth 2.0 Client Secret (NEVER return in API responses)
    #[serde(skip_serializing)]
    pub client_secret: String,
    /// OAuth 2.0 Refresh Token (NEVER return in API responses)
    #[serde(skip_serializing)]
    pub refresh_token: Option<String>,
    /// Last successful sync timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<DateTime<Utc>>,
    /// Current configuration status
    pub status: SdmConfigStatus,
    /// Error reason if status is Error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
}

impl Default for SdmConfig {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            project_number: None,
            enterprise_project_id: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            refresh_token: None,
            last_synced_at: None,
            status: SdmConfigStatus::NotConfigured,
            error_reason: None,
        }
    }
}

/// SDM configuration status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdmConfigStatus {
    /// No configuration saved
    NotConfigured,
    /// Client credentials saved, but no refresh_token yet
    ConfiguredPendingAuth,
    /// Fully authorized with valid refresh_token
    Authorized,
    /// Error state (token expired, API failure, etc.)
    Error,
}

impl Default for SdmConfigStatus {
    fn default() -> Self {
        Self::NotConfigured
    }
}

// ========================================
// API Request/Response Types
// ========================================

/// Response for GET /api/sdm/config
/// Note: client_secret and refresh_token are NEVER included
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdmConfigResponse {
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enterprise_project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    pub has_refresh_token: bool,
    pub status: SdmConfigStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<DateTime<Utc>>,
}

impl From<&SdmConfig> for SdmConfigResponse {
    fn from(config: &SdmConfig) -> Self {
        let configured = !config.project_id.is_empty() && !config.client_id.is_empty();
        Self {
            configured,
            project_id: if configured { Some(config.project_id.clone()) } else { None },
            enterprise_project_id: if configured { Some(config.enterprise_project_id.clone()) } else { None },
            client_id: if configured { Some(config.client_id.clone()) } else { None },
            has_refresh_token: config.refresh_token.is_some(),
            status: config.status,
            error_reason: config.error_reason.clone(),
            last_synced_at: config.last_synced_at,
        }
    }
}

/// Request for PUT /api/sdm/config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSdmConfigRequest {
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_number: Option<String>,
    pub enterprise_project_id: String,
    pub client_id: String,
    pub client_secret: String,
    /// If empty string, don't update (preserve existing)
    #[serde(default)]
    pub refresh_token: Option<String>,
}

/// Request for POST /api/sdm/exchange-code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeCodeRequest {
    pub auth_code: String,
}

/// Response for POST /api/sdm/exchange-code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeCodeResponse {
    pub ok: bool,
    pub has_refresh_token: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ========================================
// SDM Device Types
// ========================================

/// SDM device from devices.list API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdmDevice {
    /// Full device name (enterprises/.../devices/xxx)
    pub name: String,
    /// Device type (e.g., "sdm.devices.types.DOORBELL")
    #[serde(rename = "type")]
    pub device_type: String,
    /// Device traits
    #[serde(default)]
    pub traits: serde_json::Value,
    /// Parent relations (structures/rooms)
    #[serde(default)]
    pub parent_relations: Vec<SdmParentRelation>,
}

impl SdmDevice {
    /// Extract device_id from full name (enterprises/.../devices/xxx -> xxx)
    pub fn device_id(&self) -> Option<&str> {
        self.name.rsplit('/').next()
    }

    /// Get display name from traits
    pub fn display_name(&self) -> String {
        self.traits
            .get("sdm.devices.traits.Info")
            .and_then(|v| v.get("customName"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed Device")
            .to_string()
    }

    /// Check if this is a doorbell
    pub fn is_doorbell(&self) -> bool {
        self.device_type.contains("DOORBELL")
    }

    /// Check if this is a camera
    pub fn is_camera(&self) -> bool {
        self.device_type.contains("CAMERA") || self.is_doorbell()
    }
}

/// SDM parent relation (structure/room assignment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdmParentRelation {
    pub parent: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

/// Normalized SDM device for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdmDeviceInfo {
    /// Device ID only (not full name)
    pub sdm_device_id: String,
    /// Display name
    pub name: String,
    /// Device type (doorbell, camera, etc.)
    pub device_type: String,
    /// Traits summary (key traits only)
    pub traits_summary: serde_json::Value,
    /// Structure/room name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structure: Option<String>,
    /// Already registered in cameras table
    pub is_registered: bool,
    /// If registered, the camera_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_camera_id: Option<String>,
}

/// Response for GET /api/sdm/devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdmDevicesResponse {
    pub devices: Vec<SdmDeviceInfo>,
}

// ========================================
// Camera Registration Types
// ========================================

/// Request for POST /api/sdm/devices/:id/register
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSdmDeviceRequest {
    /// Camera ID (will be used as go2rtc source name)
    pub camera_id: String,
    /// Display name
    pub name: String,
    /// Location description
    pub location: String,
    /// Facility ID (required for subnet organization)
    pub fid: String,
    /// Tenant ID (required for subnet organization)
    pub tid: String,
    /// Floor (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor: Option<String>,
    /// Camera context for AI hints (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_context: Option<serde_json::Value>,
}

/// Validation result for camera_id
impl RegisterSdmDeviceRequest {
    /// Validate camera_id format (lowercase alphanumeric and hyphens only, max 64 chars)
    pub fn validate_camera_id(&self) -> Result<(), String> {
        if self.camera_id.is_empty() {
            return Err("camera_id is required".to_string());
        }
        if self.camera_id.len() > 64 {
            return Err("camera_id must be 64 characters or less".to_string());
        }
        if !self.camera_id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err("camera_id must contain only lowercase letters, numbers, and hyphens".to_string());
        }
        Ok(())
    }
}

// ========================================
// Test/Health Types
// ========================================

/// Response for GET /api/sdm/test/go2rtc-version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Go2rtcVersionResponse {
    pub version: String,
    pub compatible: bool,
}

/// Response for GET /api/sdm/test/token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTestResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for GET /api/sdm/test/device/:id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTestResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traits_summary: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for POST /api/sdm/reconcile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileResponse {
    pub added: usize,
    pub skipped: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

// ========================================
// Internal Token Types
// ========================================

/// Google OAuth token response (internal use)
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleTokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub token_type: String,
    #[serde(default)]
    pub scope: Option<String>,
}

/// Google OAuth error response
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleTokenError {
    pub error: String,
    #[serde(default)]
    pub error_description: Option<String>,
}

// ========================================
// go2rtc Source String Generation
// ========================================

impl SdmConfig {
    /// Generate go2rtc nest source string
    ///
    /// Format: nest://{device_id}?project_id={project_id}&enterprise={enterprise}&client_id={client_id}&client_secret={secret}&refresh_token={token}
    pub fn build_nest_source(&self, device_id: &str) -> Option<String> {
        let refresh_token = self.refresh_token.as_ref()?;
        Some(format!(
            "nest://{}?project_id={}&enterprise={}&client_id={}&client_secret={}&refresh_token={}",
            device_id,
            self.project_id,
            self.enterprise_project_id,
            self.client_id,
            self.client_secret,
            refresh_token
        ))
    }
}

// ========================================
// Constants
// ========================================

/// Minimum go2rtc version for Nest support
pub const MIN_GO2RTC_VERSION: &str = "1.9.9";

/// SDM API base URL
pub const SDM_API_BASE: &str = "https://smartdevicemanagement.googleapis.com/v1";

/// Google OAuth token endpoint
pub const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";

/// SDM API scope
pub const SDM_SCOPE: &str = "https://www.googleapis.com/auth/sdm.service";

/// Redirect URI for manual code copy
pub const REDIRECT_URI: &str = "https://www.google.com";
