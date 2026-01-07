//! SDM Integration Module
//!
//! Google Smart Device Management (SDM) integration for Nest Doorbell support.
//!
//! ## Responsibilities (SOLID - Single Responsibility)
//! - SDM configuration management (load/save/validate)
//! - OAuth token exchange and refresh
//! - SDM API calls (devices.list, etc.)
//! - go2rtc source string generation
//! - Audit logging (with secrets masking)
//!
//! ## SSoT
//! - SDM config is stored in `settings.sdm_config` (database)
//! - `/etc/is22/secrets/sdm.env` is for initial input only

pub mod types;

use crate::config_store::ConfigStore;
use crate::error::{Error, Result};
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub use types::*;

/// SDM Integration Service
///
/// Manages Google SDM configuration, authentication, and device operations.
#[derive(Clone)]
pub struct SdmService {
    config_store: Arc<ConfigStore>,
    http_client: Client,
    /// Cached access token with expiry
    access_token_cache: Arc<RwLock<Option<CachedToken>>>,
}

/// Cached access token
struct CachedToken {
    token: String,
    expires_at: chrono::DateTime<Utc>,
}

impl SdmService {
    /// Create new SDM service
    pub fn new(config_store: Arc<ConfigStore>) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config_store,
            http_client,
            access_token_cache: Arc::new(RwLock::new(None)),
        }
    }

    // ========================================
    // T2-2: Config Load/Save/Validate
    // ========================================

    /// Get SDM configuration from settings
    pub async fn get_config(&self) -> Result<Option<SdmConfig>> {
        let settings = self.config_store.service().get_setting("sdm_config").await?;
        match settings {
            Some(json) => {
                let config: SdmConfig = serde_json::from_value(json)
                    .map_err(|e| Error::Internal(format!("Failed to parse sdm_config: {}", e)))?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// Get SDM configuration response (with secrets masked)
    pub async fn get_config_response(&self) -> Result<SdmConfigResponse> {
        match self.get_config().await? {
            Some(config) => Ok(SdmConfigResponse::from(&config)),
            None => Ok(SdmConfigResponse {
                configured: false,
                project_id: None,
                enterprise_project_id: None,
                client_id: None,
                has_refresh_token: false,
                status: SdmConfigStatus::NotConfigured,
                error_reason: None,
                last_synced_at: None,
            }),
        }
    }

    /// Save SDM configuration
    pub async fn save_config(&self, req: &UpdateSdmConfigRequest) -> Result<()> {
        // Validate required fields
        if req.project_id.is_empty() {
            return Err(Error::Validation("project_id is required".to_string()));
        }
        if req.enterprise_project_id.is_empty() {
            return Err(Error::Validation("enterprise_project_id is required".to_string()));
        }
        if req.client_id.is_empty() {
            return Err(Error::Validation("client_id is required".to_string()));
        }
        if req.client_secret.is_empty() {
            return Err(Error::Validation("client_secret is required".to_string()));
        }

        // Get existing config to preserve refresh_token if not updated
        let existing = self.get_config().await?;
        let existing_refresh_token = existing.as_ref().and_then(|c| c.refresh_token.clone());

        // Determine new refresh_token
        let new_refresh_token = match &req.refresh_token {
            Some(token) if !token.is_empty() => Some(token.clone()),
            _ => existing_refresh_token,
        };

        // Determine status
        let status = if new_refresh_token.is_some() {
            SdmConfigStatus::Authorized
        } else {
            SdmConfigStatus::ConfiguredPendingAuth
        };

        let config = SdmConfig {
            project_id: req.project_id.clone(),
            project_number: req.project_number.clone(),
            enterprise_project_id: req.enterprise_project_id.clone(),
            client_id: req.client_id.clone(),
            client_secret: req.client_secret.clone(),
            refresh_token: new_refresh_token,
            last_synced_at: existing.and_then(|c| c.last_synced_at),
            status,
            error_reason: None,
        };

        // Save to settings
        let json = serde_json::to_value(&config)
            .map_err(|e| Error::Internal(format!("Failed to serialize sdm_config: {}", e)))?;
        self.config_store.service().set_setting("sdm_config", json).await?;

        // Invalidate access token cache
        *self.access_token_cache.write().await = None;

        // Audit log (masked)
        self.audit_log("sdm_config_updated", &format!(
            "project_id={}, enterprise_project_id={}, has_refresh_token={}",
            req.project_id, req.enterprise_project_id, config.refresh_token.is_some()
        )).await;

        // Refresh config store cache
        self.config_store.refresh_cache().await?;

        Ok(())
    }

    /// Update config status and error
    async fn update_config_status(&self, status: SdmConfigStatus, error_reason: Option<String>) -> Result<()> {
        if let Some(mut config) = self.get_config().await? {
            config.status = status;
            config.error_reason = error_reason;

            let json = serde_json::to_value(&config)
                .map_err(|e| Error::Internal(format!("Failed to serialize sdm_config: {}", e)))?;
            self.config_store.service().set_setting("sdm_config", json).await?;
        }
        Ok(())
    }

    // ========================================
    // T2-3: Token Exchange (auth_code -> refresh_token)
    // ========================================

    /// Exchange authorization code for refresh token
    pub async fn exchange_code(&self, auth_code: &str) -> Result<ExchangeCodeResponse> {
        let config = self.get_config().await?
            .ok_or_else(|| Error::Validation("SDM not configured".to_string()))?;

        if config.client_id.is_empty() || config.client_secret.is_empty() {
            return Err(Error::Validation("SDM client credentials not configured".to_string()));
        }

        // Call Google token endpoint
        let params = [
            ("code", auth_code),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
            ("redirect_uri", REDIRECT_URI),
            ("grant_type", "authorization_code"),
        ];

        let response = self.http_client
            .post(GOOGLE_TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                error!(target: "sdm", "Token exchange HTTP error: {}", e);
                Error::Internal(format!("Failed to connect to Google: {}", e))
            })?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if !status.is_success() {
            // Parse error response
            let error_msg = if let Ok(err) = serde_json::from_str::<GoogleTokenError>(&body) {
                err.error_description.unwrap_or(err.error)
            } else {
                format!("HTTP {}: {}", status, body)
            };

            // Update status to error
            self.update_config_status(
                SdmConfigStatus::Error,
                Some(error_msg.clone())
            ).await?;

            // Audit log
            self.audit_log("token_exchange_failed", &format!("status={}, error={}", status, error_msg)).await;

            return Ok(ExchangeCodeResponse {
                ok: false,
                has_refresh_token: false,
                error: Some(error_msg),
            });
        }

        // Parse success response
        let token_response: GoogleTokenResponse = serde_json::from_str(&body)
            .map_err(|e| Error::Internal(format!("Failed to parse token response: {}", e)))?;

        // Update config with refresh_token
        if let Some(refresh_token) = &token_response.refresh_token {
            let mut updated_config = config;
            updated_config.refresh_token = Some(refresh_token.clone());
            updated_config.status = SdmConfigStatus::Authorized;
            updated_config.error_reason = None;
            updated_config.last_synced_at = Some(Utc::now());

            let json = serde_json::to_value(&updated_config)
                .map_err(|e| Error::Internal(format!("Failed to serialize sdm_config: {}", e)))?;
            self.config_store.service().set_setting("sdm_config", json).await?;
            self.config_store.refresh_cache().await?;

            // Audit log
            self.audit_log("token_exchange_success", "refresh_token obtained").await;

            Ok(ExchangeCodeResponse {
                ok: true,
                has_refresh_token: true,
                error: None,
            })
        } else {
            // No refresh_token in response (shouldn't happen with offline access)
            self.audit_log("token_exchange_warning", "no refresh_token in response").await;

            Ok(ExchangeCodeResponse {
                ok: true,
                has_refresh_token: false,
                error: Some("No refresh_token in response. Ensure access_type=offline in auth URL.".to_string()),
            })
        }
    }

    // ========================================
    // T2-4: Token Refresh (refresh_token -> access_token)
    // ========================================

    /// Get valid access token (from cache or refresh)
    pub async fn get_access_token(&self) -> Result<String> {
        // Check cache first
        {
            let cache = self.access_token_cache.read().await;
            if let Some(cached) = cache.as_ref() {
                if cached.expires_at > Utc::now() + chrono::Duration::seconds(60) {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Need to refresh
        self.refresh_access_token().await
    }

    /// Refresh access token with exponential backoff
    async fn refresh_access_token(&self) -> Result<String> {
        let config = self.get_config().await?
            .ok_or_else(|| Error::Validation("SDM not configured".to_string()))?;

        let refresh_token = config.refresh_token.as_ref()
            .ok_or_else(|| Error::Unauthorized("No refresh_token configured".to_string()))?;

        // Exponential backoff: 1s, 5s, 30s
        let delays = [1, 5, 30];
        let mut last_error = String::new();

        for (attempt, delay) in delays.iter().enumerate() {
            let params = [
                ("client_id", config.client_id.as_str()),
                ("client_secret", config.client_secret.as_str()),
                ("refresh_token", refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ];

            let response = self.http_client
                .post(GOOGLE_TOKEN_ENDPOINT)
                .form(&params)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();

                    if status.is_success() {
                        let token_response: GoogleTokenResponse = serde_json::from_str(&body)
                            .map_err(|e| Error::Internal(format!("Failed to parse token response: {}", e)))?;

                        // Update cache
                        let expires_at = Utc::now() + chrono::Duration::seconds(token_response.expires_in);
                        *self.access_token_cache.write().await = Some(CachedToken {
                            token: token_response.access_token.clone(),
                            expires_at,
                        });

                        return Ok(token_response.access_token);
                    }

                    // Handle specific errors
                    if status.as_u16() == 401 || status.as_u16() == 403 {
                        // Token revoked or invalid
                        self.update_config_status(
                            SdmConfigStatus::Error,
                            Some("refresh_token invalid or revoked".to_string())
                        ).await?;
                        self.audit_log("token_refresh_failed", &format!("status={}, body={}", status, &body[..body.len().min(100)])).await;
                        return Err(Error::Unauthorized("refresh_token invalid or revoked".to_string()));
                    }

                    if status.as_u16() == 429 {
                        // Rate limited - don't set error status, just wait
                        warn!(target: "sdm", "Rate limited on attempt {}, waiting {}s", attempt + 1, delay);
                        last_error = "Rate limited by Google API".to_string();
                    } else {
                        last_error = format!("HTTP {}: {}", status, &body[..body.len().min(100)]);
                    }
                }
                Err(e) => {
                    last_error = format!("Connection error: {}", e);
                }
            }

            // Wait before retry (except on last attempt)
            if attempt < delays.len() - 1 {
                tokio::time::sleep(Duration::from_secs(*delay as u64)).await;
            }
        }

        // All retries failed
        self.update_config_status(
            SdmConfigStatus::Error,
            Some(last_error.clone())
        ).await?;
        self.audit_log("token_refresh_failed", &last_error).await;

        Err(Error::Internal(format!("Failed to refresh token after 3 attempts: {}", last_error)))
    }

    // ========================================
    // T2-5: SDM Devices List
    // ========================================

    /// Get SDM devices list
    pub async fn list_devices(&self) -> Result<Vec<SdmDevice>> {
        let config = self.get_config().await?
            .ok_or_else(|| Error::Validation("SDM not configured".to_string()))?;

        let access_token = self.get_access_token().await?;

        let url = format!(
            "{}/enterprises/{}/devices",
            SDM_API_BASE, config.enterprise_project_id
        );

        let response = self.http_client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to connect to SDM API: {}", e)))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if !status.is_success() {
            if status.as_u16() == 401 || status.as_u16() == 403 {
                self.update_config_status(
                    SdmConfigStatus::Error,
                    Some("SDM API authorization failed".to_string())
                ).await?;
                return Err(Error::Unauthorized("SDM API authorization failed".to_string()));
            }

            return Err(Error::Internal(format!("SDM API error {}: {}", status, &body[..body.len().min(200)])));
        }

        // Parse response
        #[derive(Deserialize)]
        struct DevicesListResponse {
            #[serde(default)]
            devices: Vec<SdmDevice>,
        }

        let devices_response: DevicesListResponse = serde_json::from_str(&body)
            .map_err(|e| Error::Internal(format!("Failed to parse devices response: {}", e)))?;

        // Update last_synced_at
        if let Some(mut config) = self.get_config().await? {
            config.last_synced_at = Some(Utc::now());
            let json = serde_json::to_value(&config)
                .map_err(|e| Error::Internal(format!("Failed to serialize sdm_config: {}", e)))?;
            self.config_store.service().set_setting("sdm_config", json).await?;
        }

        Ok(devices_response.devices)
    }

    /// Get normalized device info list for API response
    pub async fn get_devices_info(&self) -> Result<SdmDevicesResponse> {
        let devices = self.list_devices().await?;

        // Get registered SDM cameras from config store
        let cameras = self.config_store.get_cached_cameras().await;
        let registered_sdm_ids: std::collections::HashMap<String, String> = cameras
            .iter()
            .filter_map(|c| {
                c.sdm_device_id.as_ref().map(|id| (id.clone(), c.camera_id.clone()))
            })
            .collect();

        let device_infos: Vec<SdmDeviceInfo> = devices
            .into_iter()
            .filter(|d| d.is_camera()) // Only cameras/doorbells
            .map(|d| {
                let device_id = d.device_id().unwrap_or("").to_string();
                let is_registered = registered_sdm_ids.contains_key(&device_id);
                let registered_camera_id = registered_sdm_ids.get(&device_id).cloned();
                let structure = d.parent_relations.first().map(|r| r.display_name.clone());

                SdmDeviceInfo {
                    sdm_device_id: device_id,
                    name: d.display_name(),
                    device_type: if d.is_doorbell() { "doorbell".to_string() } else { "camera".to_string() },
                    traits_summary: d.traits.clone(),
                    structure,
                    is_registered,
                    registered_camera_id,
                }
            })
            .collect();

        Ok(SdmDevicesResponse { devices: device_infos })
    }

    // ========================================
    // T2-6: Audit Logging
    // ========================================

    /// Log audit event (secrets are automatically masked in types.rs serialization)
    async fn audit_log(&self, action: &str, details: &str) {
        // Mask any potential secrets in details
        let masked_details = mask_secrets(details);
        info!(target: "sdm_audit", action = %action, details = %masked_details);
    }

    // ========================================
    // Helper: Generate Authorization URL
    // ========================================

    /// Generate OAuth authorization URL for user to visit
    pub async fn generate_auth_url(&self) -> Result<String> {
        let config = self.get_config().await?
            .ok_or_else(|| Error::Validation("SDM not configured".to_string()))?;

        if config.client_id.is_empty() || config.enterprise_project_id.is_empty() {
            return Err(Error::Validation("SDM client_id and enterprise_project_id required".to_string()));
        }

        Ok(format!(
            "https://nestservices.google.com/partnerconnections/{}/auth?redirect_uri={}&access_type=offline&prompt=consent&client_id={}&response_type=code&scope={}",
            config.enterprise_project_id,
            REDIRECT_URI,
            config.client_id,
            SDM_SCOPE
        ))
    }
}

/// Mask secrets in log messages
fn mask_secrets(s: &str) -> String {
    // Patterns to mask: refresh_token=xxx, client_secret=xxx, access_token=xxx
    let mut result = s.to_string();

    for pattern in &["refresh_token", "client_secret", "access_token"] {
        if let Some(start) = result.find(&format!("{}=", pattern)) {
            let value_start = start + pattern.len() + 1;
            if let Some(end) = result[value_start..].find(|c: char| c == '&' || c == ' ' || c == ',') {
                let value_end = value_start + end;
                result.replace_range(value_start..value_end, "***MASKED***");
            } else {
                result.replace_range(value_start.., "***MASKED***");
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_secrets() {
        let input = "refresh_token=abc123&client_id=xyz";
        let masked = mask_secrets(input);
        assert!(!masked.contains("abc123"));
        assert!(masked.contains("***MASKED***"));
        assert!(masked.contains("client_id=xyz"));
    }

    #[test]
    fn test_sdm_device_id_extraction() {
        let device = SdmDevice {
            name: "enterprises/abc/devices/xyz123".to_string(),
            device_type: "sdm.devices.types.DOORBELL".to_string(),
            traits: serde_json::json!({}),
            parent_relations: vec![],
        };
        assert_eq!(device.device_id(), Some("xyz123"));
        assert!(device.is_doorbell());
        assert!(device.is_camera());
    }

    #[test]
    fn test_camera_id_validation() {
        let valid = RegisterSdmDeviceRequest {
            camera_id: "nest-abc123".to_string(),
            name: "Test".to_string(),
            location: "Test".to_string(),
            fid: "0150".to_string(),
            tid: "T123".to_string(),
            floor: None,
            camera_context: None,
        };
        assert!(valid.validate_camera_id().is_ok());

        let invalid_uppercase = RegisterSdmDeviceRequest {
            camera_id: "Nest-ABC".to_string(),
            ..valid.clone()
        };
        assert!(invalid_uppercase.validate_camera_id().is_err());

        let invalid_space = RegisterSdmDeviceRequest {
            camera_id: "nest abc".to_string(),
            ..valid.clone()
        };
        assert!(invalid_space.validate_camera_id().is_err());
    }
}
