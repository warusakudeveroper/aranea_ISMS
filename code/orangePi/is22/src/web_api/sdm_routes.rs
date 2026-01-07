//! SDM (Smart Device Management) API Routes
//!
//! Google Nest Doorbell integration endpoints.
//!
//! ## Endpoints
//! - GET /api/sdm/config - Get SDM configuration (secrets masked)
//! - PUT /api/sdm/config - Update SDM configuration
//! - GET /api/sdm/auth-url - Generate OAuth authorization URL
//! - POST /api/sdm/exchange-code - Exchange auth code for refresh token
//! - GET /api/sdm/devices - List SDM devices
//! - POST /api/sdm/devices/:id/register - Register SDM device as camera
//! - GET /api/sdm/test/go2rtc-version - Check go2rtc version compatibility
//! - GET /api/sdm/test/token - Test access token validity
//! - GET /api/sdm/test/device/:id - Test device access
//! - POST /api/sdm/reconcile - Sync SDM devices with cameras table

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::config_store::{CameraFamily, CreateCameraRequest};
use crate::sdm_integration::{
    ExchangeCodeRequest, RegisterSdmDeviceRequest, SdmConfigStatus, SdmService,
    UpdateSdmConfigRequest, MIN_GO2RTC_VERSION,
};
use crate::state::AppState;

/// Create SDM routes
pub fn sdm_routes() -> Router<AppState> {
    Router::new()
        // Configuration
        .route("/config", get(get_sdm_config))
        .route("/config", put(update_sdm_config))
        // OAuth
        .route("/auth-url", get(get_auth_url))
        .route("/exchange-code", post(exchange_code))
        // Devices
        .route("/devices", get(list_sdm_devices))
        .route("/devices/:id/register", post(register_sdm_device))
        // Tests
        .route("/test/go2rtc-version", get(test_go2rtc_version))
        .route("/test/token", get(test_token))
        .route("/test/device/:id", get(test_device))
        // Reconcile & Sync
        .route("/reconcile", post(reconcile_devices))
        .route("/sync-go2rtc", post(sync_go2rtc))
        .route("/stream-status/:camera_id", get(get_stream_status))
}

// ========================================
// Helper: Standard JSON Response
// ========================================

fn ok_response<T: serde::Serialize>(data: T) -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "data": data
    }))
}

fn err_response(msg: &str) -> impl IntoResponse {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "ok": false,
            "error": msg
        })),
    )
}

fn internal_err_response(msg: &str) -> impl IntoResponse {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "ok": false,
            "error": msg
        })),
    )
}

fn conflict_err_response(msg: &str) -> impl IntoResponse {
    (
        StatusCode::CONFLICT,
        Json(json!({
            "ok": false,
            "error": msg
        })),
    )
}

fn unauthorized_err_response(msg: &str) -> impl IntoResponse {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "ok": false,
            "error": msg
        })),
    )
}

/// Convert crate::Error to appropriate HTTP response
fn error_to_response(e: crate::Error) -> axum::response::Response {
    match e {
        crate::Error::Conflict(msg) => conflict_err_response(&msg).into_response(),
        crate::Error::Unauthorized(msg) => unauthorized_err_response(&msg).into_response(),
        crate::Error::Validation(msg) => err_response(&msg).into_response(),
        _ => internal_err_response(&e.to_string()).into_response(),
    }
}

// ========================================
// T3-1: GET /api/sdm/config
// ========================================

/// Get SDM configuration (secrets masked)
async fn get_sdm_config(State(state): State<AppState>) -> impl IntoResponse {
    let sdm_service = SdmService::new(state.config_store.clone());

    match sdm_service.get_config_response().await {
        Ok(config) => ok_response(config).into_response(),
        Err(e) => internal_err_response(&e.to_string()).into_response(),
    }
}

// ========================================
// T3-2: PUT /api/sdm/config
// ========================================

/// Update SDM configuration
async fn update_sdm_config(
    State(state): State<AppState>,
    Json(req): Json<UpdateSdmConfigRequest>,
) -> impl IntoResponse {
    let sdm_service = SdmService::new(state.config_store.clone());

    match sdm_service.save_config(&req).await {
        Ok(()) => {
            // Return updated config
            match sdm_service.get_config_response().await {
                Ok(config) => ok_response(config).into_response(),
                Err(e) => internal_err_response(&e.to_string()).into_response(),
            }
        }
        Err(e) => err_response(&e.to_string()).into_response(),
    }
}

// ========================================
// T3-3: GET /api/sdm/auth-url
// ========================================

/// Generate OAuth authorization URL
async fn get_auth_url(State(state): State<AppState>) -> impl IntoResponse {
    let sdm_service = SdmService::new(state.config_store.clone());

    match sdm_service.generate_auth_url().await {
        Ok(url) => ok_response(json!({ "url": url })).into_response(),
        Err(e) => err_response(&e.to_string()).into_response(),
    }
}

// ========================================
// T3-4: POST /api/sdm/exchange-code
// ========================================

/// Exchange authorization code for refresh token
async fn exchange_code(
    State(state): State<AppState>,
    Json(req): Json<ExchangeCodeRequest>,
) -> impl IntoResponse {
    let sdm_service = SdmService::new(state.config_store.clone());

    match sdm_service.exchange_code(&req.auth_code).await {
        Ok(response) => ok_response(response).into_response(),
        Err(e) => internal_err_response(&e.to_string()).into_response(),
    }
}

// ========================================
// T3-5: GET /api/sdm/devices
// ========================================

/// List SDM devices (cameras/doorbells)
async fn list_sdm_devices(State(state): State<AppState>) -> impl IntoResponse {
    let sdm_service = SdmService::new(state.config_store.clone());

    match sdm_service.get_devices_info().await {
        Ok(devices) => ok_response(devices).into_response(),
        Err(e) => error_to_response(e),
    }
}

// ========================================
// T3-6: POST /api/sdm/devices/:id/register
// ========================================

/// Register SDM device as camera
async fn register_sdm_device(
    State(state): State<AppState>,
    Path(sdm_device_id): Path<String>,
    Json(req): Json<RegisterSdmDeviceRequest>,
) -> impl IntoResponse {
    // Validate camera_id format
    if let Err(e) = req.validate_camera_id() {
        return err_response(&e).into_response();
    }

    let sdm_service = SdmService::new(state.config_store.clone());

    // Verify device exists and get info
    let devices = match sdm_service.list_devices().await {
        Ok(d) => d,
        Err(e) => return internal_err_response(&e.to_string()).into_response(),
    };

    let device = match devices.iter().find(|d| d.device_id() == Some(&sdm_device_id)) {
        Some(d) => d,
        None => return err_response(&format!("SDM device {} not found", sdm_device_id)).into_response(),
    };

    // Get SDM config for nest source generation
    let sdm_config = match sdm_service.get_config().await {
        Ok(Some(c)) => c,
        Ok(None) => return err_response("SDM not configured").into_response(),
        Err(e) => return internal_err_response(&e.to_string()).into_response(),
    };

    // Generate nest:// source URL
    let nest_source = match sdm_config.build_nest_source(&sdm_device_id) {
        Some(s) => s,
        None => return err_response("refresh_token not available").into_response(),
    };

    // Create camera with SDM info
    let create_req = CreateCameraRequest {
        camera_id: req.camera_id.clone(),
        name: req.name.clone(),
        location: req.location.clone(),
        floor: req.floor.clone(),
        rtsp_main: Some(nest_source), // nest:// URL as rtsp_main
        rtsp_sub: None,
        snapshot_url: None,
        family: Some(CameraFamily::Nest),
        manufacturer: Some("Google".to_string()),
        model: Some(if device.is_doorbell() { "Nest Doorbell".to_string() } else { "Nest Camera".to_string() }),
        ip_address: None, // SDM devices don't have local IP
        mac_address: None,
        camera_context: req.camera_context.clone(),
    };

    // Create camera
    match state.config_store.service().create_camera(create_req).await {
        Ok(camera) => {
            // Update with SDM-specific fields
            let mut update_req = crate::config_store::UpdateCameraRequest::default();
            update_req.sdm_device_id = Some(sdm_device_id.clone());
            update_req.sdm_structure = device.parent_relations.first().map(|r| r.display_name.clone());
            update_req.sdm_traits = Some(device.traits.clone());
            update_req.fid = Some(req.fid.clone());
            update_req.tid = Some(req.tid.clone());
            update_req.discovery_method = Some("sdm".to_string());

            match state.config_store.service().update_camera(&camera.camera_id, update_req).await {
                Ok(updated_camera) => {
                    // Refresh cache
                    let _ = state.config_store.refresh_cache().await;
                    ok_response(updated_camera).into_response()
                }
                Err(e) => error_to_response(e),
            }
        }
        Err(e) => error_to_response(e), // Returns 409 for Conflict (duplicate camera_id)
    }
}

// ========================================
// T3-7: GET /api/sdm/test/go2rtc-version
// ========================================

/// Test go2rtc version compatibility for Nest support
async fn test_go2rtc_version(State(state): State<AppState>) -> impl IntoResponse {
    match state.stream.get_version().await {
        Ok(version) => {
            let compatible = is_version_compatible(&version, MIN_GO2RTC_VERSION);
            ok_response(json!({
                "version": version,
                "min_required": MIN_GO2RTC_VERSION,
                "compatible": compatible
            })).into_response()
        }
        Err(e) => internal_err_response(&format!("Failed to get go2rtc version: {}", e)).into_response(),
    }
}

/// Compare versions (simple semver comparison)
fn is_version_compatible(current: &str, min_required: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let current_parts = parse_version(current);
    let required_parts = parse_version(min_required);

    for i in 0..3 {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let r = required_parts.get(i).copied().unwrap_or(0);
        if c > r {
            return true;
        }
        if c < r {
            return false;
        }
    }
    true // Equal versions
}

// ========================================
// T3-8: GET /api/sdm/test/token
// ========================================

/// Test access token validity
async fn test_token(State(state): State<AppState>) -> impl IntoResponse {
    let sdm_service = SdmService::new(state.config_store.clone());

    match sdm_service.get_access_token().await {
        Ok(_) => ok_response(json!({
            "ok": true,
            "message": "Access token is valid"
        })).into_response(),
        Err(e) => ok_response(json!({
            "ok": false,
            "error": e.to_string()
        })).into_response(),
    }
}

// ========================================
// T3-9: GET /api/sdm/test/device/:id
// ========================================

/// Test device access
async fn test_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> impl IntoResponse {
    let sdm_service = SdmService::new(state.config_store.clone());

    // List devices and find the specific one
    match sdm_service.list_devices().await {
        Ok(devices) => {
            match devices.iter().find(|d| d.device_id() == Some(&device_id)) {
                Some(device) => ok_response(json!({
                    "ok": true,
                    "device_type": device.device_type,
                    "display_name": device.display_name(),
                    "traits_summary": device.traits.clone()
                })).into_response(),
                None => ok_response(json!({
                    "ok": false,
                    "error": format!("Device {} not found", device_id)
                })).into_response(),
            }
        }
        Err(e) => ok_response(json!({
            "ok": false,
            "error": e.to_string()
        })).into_response(),
    }
}

// ========================================
// T3-10: POST /api/sdm/reconcile
// ========================================

#[derive(Deserialize)]
struct ReconcileRequest {
    /// Default fid for new cameras
    fid: String,
    /// Default tid for new cameras
    tid: String,
    /// Auto-register unregistered devices
    #[serde(default)]
    auto_register: bool,
}

/// Reconcile SDM devices with cameras table
async fn reconcile_devices(
    State(state): State<AppState>,
    Json(req): Json<ReconcileRequest>,
) -> impl IntoResponse {
    let sdm_service = SdmService::new(state.config_store.clone());

    // Get SDM devices
    let devices = match sdm_service.get_devices_info().await {
        Ok(d) => d,
        Err(e) => return internal_err_response(&e.to_string()).into_response(),
    };

    let mut added = 0;
    let mut skipped = 0;
    let mut errors: Vec<String> = Vec::new();

    if req.auto_register {
        let sdm_config = match sdm_service.get_config().await {
            Ok(Some(c)) if c.status == SdmConfigStatus::Authorized => c,
            Ok(Some(_)) => {
                return err_response("SDM not authorized").into_response();
            }
            Ok(None) => {
                return err_response("SDM not configured").into_response();
            }
            Err(e) => return internal_err_response(&e.to_string()).into_response(),
        };

        for device in &devices.devices {
            if device.is_registered {
                skipped += 1;
                continue;
            }

            // Generate camera_id from SDM device ID
            let camera_id = format!("nest-{}", &device.sdm_device_id[..8.min(device.sdm_device_id.len())]);

            // Generate nest source
            let nest_source = match sdm_config.build_nest_source(&device.sdm_device_id) {
                Some(s) => s,
                None => {
                    errors.push(format!("No refresh_token for {}", device.sdm_device_id));
                    continue;
                }
            };

            // Create camera
            let create_req = CreateCameraRequest {
                camera_id: camera_id.clone(),
                name: device.name.clone(),
                location: device.structure.clone().unwrap_or_default(),
                floor: None,
                rtsp_main: Some(nest_source),
                rtsp_sub: None,
                snapshot_url: None,
                family: Some(CameraFamily::Nest),
                manufacturer: Some("Google".to_string()),
                model: Some(device.device_type.clone()),
                ip_address: None,
                mac_address: None,
                camera_context: None,
            };

            match state.config_store.service().create_camera(create_req).await {
                Ok(camera) => {
                    // Update with SDM-specific fields
                    let mut update_req = crate::config_store::UpdateCameraRequest::default();
                    update_req.sdm_device_id = Some(device.sdm_device_id.clone());
                    update_req.sdm_structure = device.structure.clone();
                    update_req.sdm_traits = Some(device.traits_summary.clone());
                    update_req.fid = Some(req.fid.clone());
                    update_req.tid = Some(req.tid.clone());
                    update_req.discovery_method = Some("sdm".to_string());

                    if let Err(e) = state.config_store.service().update_camera(&camera.camera_id, update_req).await {
                        errors.push(format!("Failed to update {}: {}", camera_id, e));
                    } else {
                        added += 1;
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to create {}: {}", camera_id, e));
                }
            }
        }

        // Refresh cache after all changes
        let _ = state.config_store.refresh_cache().await;
    } else {
        // Just count registered/unregistered
        for device in &devices.devices {
            if device.is_registered {
                skipped += 1;
            }
        }
    }

    ok_response(json!({
        "added": added,
        "skipped": skipped,
        "unregistered": devices.devices.iter().filter(|d| !d.is_registered).count(),
        "errors": errors
    })).into_response()
}

// ========================================
// Phase 4: go2rtc Sync Endpoints
// ========================================

/// Sync Nest cameras with go2rtc
async fn sync_go2rtc(State(state): State<AppState>) -> impl IntoResponse {
    // Get all Nest cameras from config store
    let cameras = state.config_store.get_cached_cameras().await;
    let nest_cameras: Vec<_> = cameras
        .iter()
        .filter(|c| c.family == "nest" && c.rtsp_main.is_some())
        .map(|c| crate::stream_gateway::NestCameraInfo {
            camera_id: c.camera_id.clone(),
            nest_url: c.rtsp_main.clone().unwrap_or_default(),
        })
        .collect();

    if nest_cameras.is_empty() {
        return ok_response(json!({
            "added": 0,
            "skipped": 0,
            "errors": [],
            "message": "No Nest cameras found"
        })).into_response();
    }

    // Sync with go2rtc
    let result = state.stream.sync_nest_cameras(&nest_cameras).await;

    ok_response(json!({
        "added": result.added,
        "skipped": result.skipped,
        "errors": result.errors,
        "total_nest_cameras": nest_cameras.len()
    })).into_response()
}

/// Get stream status for a camera
async fn get_stream_status(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
) -> impl IntoResponse {
    match state.stream.get_stream_status(&camera_id).await {
        Ok(status) => ok_response(status).into_response(),
        Err(e) => internal_err_response(&e.to_string()).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        assert!(is_version_compatible("1.9.9", "1.9.9"));
        assert!(is_version_compatible("1.9.10", "1.9.9"));
        assert!(is_version_compatible("1.10.0", "1.9.9"));
        assert!(is_version_compatible("2.0.0", "1.9.9"));
        assert!(!is_version_compatible("1.9.8", "1.9.9"));
        assert!(!is_version_compatible("1.8.10", "1.9.9"));
        assert!(!is_version_compatible("0.9.9", "1.9.9"));
    }
}
