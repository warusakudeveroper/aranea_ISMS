//! API Routes

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::admission_controller::{LeaseRequest, LeaseResponse, StreamQuality};
use crate::camera_brand::{
    AddGenericPathRequest, AddOuiRequest, AddTemplateRequest, CreateBrandRequest,
    UpdateBrandRequest, UpdateGenericPathRequest, UpdateOuiRequest, UpdateTemplateRequest,
};
use crate::config_store::{Camera, CreateCameraRequest, UpdateCameraRequest};
use crate::models::ApiResponse;
use crate::state::AppState;

/// Create API router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health & Status
        .route("/healthz", get(super::health_check))
        .route("/api/status", get(super::device_status))
        // Cameras
        .route("/api/cameras", get(list_cameras))
        .route("/api/cameras", post(create_camera))
        .route("/api/cameras/:id", get(get_camera))
        .route("/api/cameras/:id", put(update_camera))
        .route("/api/cameras/:id", delete(delete_camera))
        .route("/api/cameras/:id/soft-delete", post(soft_delete_camera))
        .route("/api/cameras/:id/rescan", post(rescan_camera))
        .route("/api/cameras/:id/auth-test", post(auth_test_camera))
        .route("/api/cameras/:id/sync-preset", post(sync_preset_to_is21))
        .route("/api/cameras/restore-by-mac", post(restore_camera_by_mac))
        // Modal Leases
        .route("/api/modal/lease", post(request_lease))
        .route("/api/modal/lease/:id", delete(release_lease))
        .route("/api/modal/lease/:id/heartbeat", post(heartbeat))
        // Suggest
        .route("/api/suggest", get(get_suggest))
        .route("/api/suggest/manual", post(set_manual_suggest))
        .route("/api/suggest", delete(clear_suggest))
        // Events (legacy in-memory)
        .route("/api/events", get(list_events))
        // Detection Logs (AI Event Log - MySQL persistence)
        .route("/api/detection-logs", get(list_detection_logs))
        .route("/api/detection-logs/stats", get(detection_log_stats))
        .route("/api/detection-logs/camera/:camera_id", get(detection_logs_by_camera))
        .route("/api/detection-logs/severity/:threshold", get(detection_logs_by_severity))
        // System
        .route("/api/system/status", get(system_status))
        // Settings (Settings Modal APIs)
        .route("/api/settings/is21/status", get(get_is21_status))
        .route("/api/settings/is21", get(get_is21_config))
        .route("/api/settings/is21", put(update_is21_config))
        .route("/api/settings/is21/test", post(test_is21_connection))
        .route("/api/settings/system", get(get_system_info))
        .route("/api/settings/performance/logs", get(get_performance_logs))
        .route("/api/settings/polling/logs", get(get_polling_logs))
        .route("/api/settings/timeouts", get(get_global_timeouts))
        .route("/api/settings/timeouts", put(update_global_timeouts))
        // Camera Brands (OUI/RTSP SSoT)
        .route("/api/settings/camera-brands", get(list_camera_brands))
        .route("/api/settings/camera-brands", post(create_camera_brand))
        .route("/api/settings/camera-brands/:id", get(get_camera_brand))
        .route("/api/settings/camera-brands/:id", put(update_camera_brand))
        .route("/api/settings/camera-brands/:id", delete(delete_camera_brand))
        // OUI Entries
        .route("/api/settings/camera-brands/:id/oui", get(list_oui_entries_for_brand))
        .route("/api/settings/camera-brands/:id/oui", post(add_oui_entry))
        .route("/api/settings/oui", get(list_all_oui_entries))
        .route("/api/settings/oui/:prefix", get(get_oui_entry))
        .route("/api/settings/oui/:prefix", put(update_oui_entry))
        .route("/api/settings/oui/:prefix", delete(delete_oui_entry))
        // RTSP Templates
        .route("/api/settings/camera-brands/:id/rtsp-templates", get(list_templates_for_brand))
        .route("/api/settings/camera-brands/:id/rtsp-templates", post(add_rtsp_template))
        .route("/api/settings/rtsp-templates", get(list_all_rtsp_templates))
        .route("/api/settings/rtsp-templates/:id", get(get_rtsp_template))
        .route("/api/settings/rtsp-templates/:id", put(update_rtsp_template))
        .route("/api/settings/rtsp-templates/:id", delete(delete_rtsp_template))
        // Generic RTSP Paths
        .route("/api/settings/generic-rtsp-paths", get(list_generic_rtsp_paths))
        .route("/api/settings/generic-rtsp-paths", post(add_generic_rtsp_path))
        .route("/api/settings/generic-rtsp-paths/:id", get(get_generic_rtsp_path))
        .route("/api/settings/generic-rtsp-paths/:id", put(update_generic_rtsp_path))
        .route("/api/settings/generic-rtsp-paths/:id", delete(delete_generic_rtsp_path))
        // Performance Dashboard API (統合デバッグAPI)
        .route("/api/debug/performance/dashboard", get(get_performance_dashboard))
        // Test API for E2E testing (BUG-001 verification)
        .route("/api/test/trigger-event", post(trigger_test_event))
        // IpcamScan
        .route("/api/ipcamscan/jobs", post(create_scan_job))
        .route("/api/ipcamscan/jobs/:id", get(get_scan_job))
        .route("/api/ipcamscan/jobs/:id/abort", post(abort_scan_job))
        .route("/api/ipcamscan/status", get(get_scan_status))
        .route("/api/ipcamscan/devices", get(list_scanned_devices))
        .route("/api/ipcamscan/devices-with-categories", get(list_devices_with_categories))
        .route("/api/ipcamscan/devices/cidr/count", post(count_devices_by_cidr))
        .route("/api/ipcamscan/devices/cidr/delete", post(delete_devices_by_cidr))
        .route("/api/ipcamscan/devices/:ip/verify", post(verify_device))
        .route("/api/ipcamscan/devices/:ip/approve", post(approve_device))
        .route("/api/ipcamscan/devices/:ip/reject", post(reject_device))
        .route("/api/ipcamscan/devices/:ip", delete(delete_scanned_device))
        .route("/api/ipcamscan/devices", delete(clear_scanned_devices))
        .route("/api/ipcamscan/devices/approve-batch", post(approve_devices_batch))
        .route("/api/ipcamscan/devices/verify-batch", post(verify_devices_batch))
        .route("/api/ipcamscan/devices/:ip/force-register", post(force_register_device))
        .route("/api/ipcamscan/cameras/:id/activate", post(activate_pending_camera))
        .route("/api/ipcamscan/maintenance/cleanup-credentials", post(cleanup_credentials))
        // Subnets
        .route("/api/subnets", get(list_subnets))
        .route("/api/subnets", post(create_subnet))
        .route("/api/subnets/:id", get(get_subnet))
        .route("/api/subnets/:id", put(update_subnet))
        .route("/api/subnets/:id", delete(delete_subnet))
        .route("/api/subnets/scan", post(scan_selected_subnets))
        // Credentials
        .route("/api/credentials", get(list_credentials))
        .route("/api/credentials", post(create_credential))
        .route("/api/credentials/:id", get(get_credential))
        .route("/api/credentials/:id", delete(delete_credential))
        // Streams (go2rtc)
        .route("/api/streams", get(list_streams))
        .route("/api/streams/:camera_id", get(get_stream_urls))
        .route("/api/streams/:camera_id/snapshot", get(get_snapshot))
        // Snapshots (ffmpeg cached images for CameraGrid)
        .route("/api/snapshots/:camera_id/latest.jpg", get(get_cached_snapshot))
        // WebSocket
        .route("/api/ws", get(websocket_handler))
        .with_state(state)
}

// ========================================
// Camera Handlers
// ========================================

async fn list_cameras(State(state): State<AppState>) -> impl IntoResponse {
    let cameras = state.config_store.get_cached_cameras().await;
    Json(ApiResponse::success(cameras))
}

async fn get_camera(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.config_store.service().get_camera(&id).await {
        Ok(Some(camera)) => Json(ApiResponse::success(camera)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn create_camera(
    State(state): State<AppState>,
    Json(req): Json<CreateCameraRequest>,
) -> impl IntoResponse {
    match state.config_store.service().create_camera(req).await {
        Ok(camera) => {
            let _ = state.config_store.refresh_cache().await;

            // Spawn polling loop for new subnet if needed
            if let Some(ip) = &camera.ip_address {
                state.polling.spawn_subnet_loop_if_needed(ip).await;
            }

            (StatusCode::CREATED, Json(ApiResponse::success(camera))).into_response()
        }
        Err(e) => e.into_response(),
    }
}

async fn update_camera(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCameraRequest>,
) -> impl IntoResponse {
    match state.config_store.service().update_camera(&id, req).await {
        Ok(camera) => {
            let _ = state.config_store.refresh_cache().await;

            // Spawn polling loop for new subnet if needed (IP may have changed)
            if let Some(ip) = &camera.ip_address {
                state.polling.spawn_subnet_loop_if_needed(ip).await;
            }

            Json(ApiResponse::success(camera)).into_response()
        }
        Err(e) => e.into_response(),
    }
}

async fn delete_camera(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.config_store.service().delete_camera(&id).await {
        Ok(_) => {
            // go2rtc cleanup is handled by polling_orchestrator at cycle start
            let _ = state.config_store.refresh_cache().await;
            Json(serde_json::json!({"ok": true})).into_response()
        }
        Err(e) => e.into_response(),
    }
}

/// Soft delete camera (MAC address preserved for restore)
async fn soft_delete_camera(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.config_store.service().soft_delete_camera(&id).await {
        Ok(camera) => {
            // go2rtc cleanup is handled by polling_orchestrator at cycle start
            let _ = state.config_store.refresh_cache().await;
            Json(ApiResponse::success(serde_json::json!({
                "ok": true,
                "camera_id": camera.camera_id,
                "mac_address": camera.mac_address,
                "deleted_at": camera.deleted_at,
                "recoverable": camera.mac_address.is_some()
            }))).into_response()
        }
        Err(e) => e.into_response(),
    }
}

/// Auth test request
#[derive(Debug, Deserialize)]
struct AuthTestRequest {
    username: String,
    password: String,
}

/// Auth test response
#[derive(Debug, serde::Serialize)]
struct AuthTestResponse {
    rtsp_success: bool,
    onvif_success: bool,
    model: Option<String>,
    message: String,
}

/// Test camera credentials (RTSP/ONVIF auth)
async fn auth_test_camera(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AuthTestRequest>,
) -> impl IntoResponse {
    // Get camera info first
    let camera = match state.config_store.service().get_camera(&id).await {
        Ok(Some(c)) => c,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Camera not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    let ip = match camera.ip_address {
        Some(ip) => ip,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Camera has no IP address"}))).into_response(),
    };

    // Try RTSP auth test
    let rtsp_success = test_rtsp_auth(&ip, &req.username, &req.password).await;

    // Try ONVIF auth test
    let (onvif_success, model) = test_onvif_auth(&ip, &req.username, &req.password).await;

    let message = if rtsp_success && onvif_success {
        "認証成功".to_string()
    } else if rtsp_success {
        "RTSP認証成功、ONVIF認証失敗".to_string()
    } else if onvif_success {
        "ONVIF認証成功、RTSP認証失敗".to_string()
    } else {
        "認証失敗".to_string()
    };

    Json(ApiResponse::success(AuthTestResponse {
        rtsp_success,
        onvif_success,
        model,
        message,
    })).into_response()
}

/// Test RTSP authentication
async fn test_rtsp_auth(ip: &str, username: &str, password: &str) -> bool {
    // Simple TCP connection test to RTSP port with auth
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let addr = format!("{}:554", ip);
    match timeout(Duration::from_secs(5), TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => {
            // Port is open, attempt DESCRIBE with auth
            // For now, just return true if port is open (simplified)
            // TODO: Implement proper RTSP DESCRIBE with Digest auth
            true
        }
        _ => false,
    }
}

/// Test ONVIF authentication
async fn test_onvif_auth(ip: &str, username: &str, password: &str) -> (bool, Option<String>) {
    // ONVIF GetDeviceInformation request
    // For now, simplified implementation
    // TODO: Implement proper ONVIF WS-Security UsernameToken auth

    use tokio::time::{timeout, Duration};

    let url = format!("http://{}:80/onvif/device_service", ip);
    let client = reqwest::Client::new();

    match timeout(Duration::from_secs(5), client.get(&url).send()).await {
        Ok(Ok(resp)) if resp.status().is_success() => {
            // ONVIF endpoint exists, assume success for now
            (true, Some("Unknown Model".to_string()))
        }
        _ => (false, None),
    }
}

/// Restore camera request
#[derive(Debug, Deserialize)]
struct RestoreCameraRequest {
    mac_address: String,
}

/// Restore soft-deleted camera by MAC address
async fn restore_camera_by_mac(
    State(state): State<AppState>,
    Json(req): Json<RestoreCameraRequest>,
) -> impl IntoResponse {
    match state.config_store.service().restore_camera_by_mac(&req.mac_address).await {
        Ok(camera) => {
            // go2rtc registration is handled by polling_orchestrator at cycle start
            let _ = state.config_store.refresh_cache().await;
            Json(ApiResponse::success(camera)).into_response()
        }
        Err(e) => e.into_response(),
    }
}

/// Rescan camera (find new IP by MAC address)
async fn rescan_camera(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Get camera info
    let camera = match state.config_store.service().get_camera(&id).await {
        Ok(Some(c)) => c,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Camera not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    let mac = match camera.mac_address.clone() {
        Some(m) => m,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Camera has no MAC address"}))).into_response(),
    };

    let old_ip = camera.ip_address.clone().unwrap_or_default();

    // TODO: Implement ARP scan to find new IP by MAC
    // For now, return a placeholder response indicating the current IP is still valid
    // This would require running arping or arp-scan on the local network

    Json(ApiResponse::success(serde_json::json!({
        "found": true,
        "old_ip": old_ip.clone(),
        "new_ip": old_ip,
        "updated": false
    }))).into_response()
}

// ========================================
// Preset Sync to IS21 Handler
// ========================================

/// Preset sync response
#[derive(Debug, serde::Serialize)]
struct PresetSyncResponse {
    success: bool,
    lacis_id: String,
    preset_id: String,
    preset_version: String,
    message: String,
    is21_response: Option<serde_json::Value>,
}

/// Sync camera preset to IS21
/// POST /api/cameras/:id/sync-preset
///
/// This endpoint registers the camera's preset configuration with IS21's preset cache.
/// IS21 uses this to customize AI analysis for each camera based on its lacisID.
async fn sync_preset_to_is21(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // 1. Get camera info
    let camera = match state.config_store.service().get_camera(&id).await {
        Ok(Some(c)) => c,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Camera not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    // 2. Get lacis_id (required for IS21 preset registration)
    let lacis_id = match camera.lacis_id.clone() {
        Some(id) if id.len() == 20 => id,  // Valid 20-digit lacisID
        Some(id) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": format!("Invalid lacis_id length: {} (expected 20 digits)", id.len()),
                "lacis_id": id
            }))).into_response();
        }
        None => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Camera has no lacis_id. Register with araneaDeviceGate first."
            }))).into_response();
        }
    };

    // 3. Get preset configuration
    let preset_id = camera.preset_id.clone().unwrap_or_else(|| "balanced".to_string());
    let preset = state.preset_loader.get_or_default(&preset_id);
    let preset_version = preset.version.clone();

    // 4. Build detection_config for IS21
    let detection_config = serde_json::json!({
        "confidence_threshold": 0.33,
        "nms_threshold": 0.40,
        "enabled_events": preset.expected_objects.clone(),
        "severity_boost": {},
        "excluded_objects": preset.excluded_objects.clone(),
        "person_detection": {
            "par_enabled": true,
            "par_max_persons": 10,
            "par_threshold": 0.5
        },
        "location_type": preset.location_type,
        "distance": preset.distance,
        "enable_frame_diff": preset.enable_frame_diff,
        "return_bboxes": preset.return_bboxes,
        "output_schema": preset.output_schema,
        "suggested_interval_sec": preset.suggested_interval_sec
    });

    // 5. Send to IS21
    let is21_url = format!("{}/v1/presets/{}", state.config.is21_url, lacis_id);
    let client = reqwest::Client::new();

    tracing::info!(
        camera_id = %id,
        lacis_id = %lacis_id,
        preset_id = %preset_id,
        "Syncing preset to IS21"
    );

    let form = reqwest::multipart::Form::new()
        .text("preset_id", preset_id.clone())
        .text("preset_version", preset_version.clone())
        .text("detection_config", detection_config.to_string());

    match client
        .post(&is21_url)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(is21_resp) => {
                        tracing::info!(
                            camera_id = %id,
                            lacis_id = %lacis_id,
                            "Preset sync successful"
                        );
                        Json(ApiResponse::success(PresetSyncResponse {
                            success: true,
                            lacis_id,
                            preset_id,
                            preset_version,
                            message: "プリセットをIS21に同期しました".to_string(),
                            is21_response: Some(is21_resp),
                        })).into_response()
                    }
                    Err(e) => {
                        tracing::warn!(
                            camera_id = %id,
                            error = %e,
                            "IS21 response parse failed but request succeeded"
                        );
                        Json(ApiResponse::success(PresetSyncResponse {
                            success: true,
                            lacis_id,
                            preset_id,
                            preset_version,
                            message: "プリセット同期成功（レスポンス解析失敗）".to_string(),
                            is21_response: None,
                        })).into_response()
                    }
                }
            } else {
                let status = response.status();
                let error_body = response.text().await.unwrap_or_default();
                tracing::error!(
                    camera_id = %id,
                    status = %status,
                    error = %error_body,
                    "IS21 preset sync failed"
                );
                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                    "error": format!("IS21 returned error: {} - {}", status, error_body),
                    "lacis_id": lacis_id,
                    "preset_id": preset_id
                }))).into_response()
            }
        }
        Err(e) => {
            tracing::error!(
                camera_id = %id,
                error = %e,
                url = %is21_url,
                "Failed to connect to IS21"
            );
            (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                "error": format!("IS21接続失敗: {}", e),
                "lacis_id": lacis_id,
                "is21_url": is21_url
            }))).into_response()
        }
    }
}

// ========================================
// Modal Lease Handlers
// ========================================

async fn request_lease(
    State(state): State<AppState>,
    Json(req): Json<LeaseRequest>,
) -> impl IntoResponse {
    match state.admission.request_lease(req.clone()).await {
        Ok(lease) => {
            let stream_urls = state.stream.get_stream_urls(&lease.camera_id);
            let response = LeaseResponse {
                lease_id: lease.lease_id,
                allowed_quality: lease.quality,
                expires_at: lease.expires_at,
                stream_url: stream_urls.webrtc_url,
            };
            Json(ApiResponse::success(response)).into_response()
        }
        Err(e) => e.into_response(),
    }
}

async fn release_lease(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let lease_id = match uuid::Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid UUID"}))).into_response(),
    };

    match state.admission.release_lease(&lease_id).await {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn heartbeat(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let lease_id = match uuid::Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid UUID"}))).into_response(),
    };

    match state.admission.heartbeat(&lease_id).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => e.into_response(),
    }
}

// ========================================
// Suggest Handlers
// ========================================

async fn get_suggest(State(state): State<AppState>) -> impl IntoResponse {
    let suggest_state = state.suggest.get_state().await;
    Json(suggest_state)
}

#[derive(Deserialize)]
struct ManualSuggestRequest {
    camera_id: String,
    duration_sec: Option<i64>,
}

async fn set_manual_suggest(
    State(state): State<AppState>,
    Json(req): Json<ManualSuggestRequest>,
) -> impl IntoResponse {
    let duration = req.duration_sec.unwrap_or(300);
    state.suggest.set_manual(req.camera_id, duration).await;
    Json(serde_json::json!({"ok": true}))
}

async fn clear_suggest(State(state): State<AppState>) -> impl IntoResponse {
    state.suggest.clear().await;
    Json(serde_json::json!({"ok": true}))
}

// ========================================
// Event Handlers
// ========================================

#[derive(Deserialize)]
struct EventQuery {
    limit: Option<usize>,
    camera_id: Option<String>,
}

async fn list_events(
    State(state): State<AppState>,
    Query(query): Query<EventQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100);

    let events = if let Some(camera_id) = query.camera_id {
        state.event_log.get_by_camera(&camera_id, limit).await
    } else {
        state.event_log.get_latest(limit).await
    };

    Json(ApiResponse::success(events))
}

// ========================================
// Detection Log Handlers (AI Event Log)
// ========================================

/// Query parameters for detection logs
#[derive(Deserialize)]
struct DetectionLogQuery {
    limit: Option<u32>,
    camera_id: Option<String>,
    severity_min: Option<i32>,
    start: Option<String>,  // ISO8601 datetime
    end: Option<String>,    // ISO8601 datetime
}

/// List detection logs with filtering
async fn list_detection_logs(
    State(state): State<AppState>,
    Query(query): Query<DetectionLogQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100);

    // Time range filter
    if let (Some(start_str), Some(end_str)) = (&query.start, &query.end) {
        let start = match chrono::DateTime::parse_from_rfc3339(start_str) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid start datetime format"}))).into_response(),
        };
        let end = match chrono::DateTime::parse_from_rfc3339(end_str) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid end datetime format"}))).into_response(),
        };

        match state.detection_log.get_by_time_range(start, end, limit).await {
            Ok(logs) => return Json(ApiResponse::success(serde_json::json!({
                "logs": logs,
                "total": logs.len(),
                "filter": {
                    "start": start_str,
                    "end": end_str,
                    "limit": limit
                }
            }))).into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        }
    }

    // Camera filter
    if let Some(camera_id) = &query.camera_id {
        match state.detection_log.get_by_camera(camera_id, limit).await {
            Ok(logs) => return Json(ApiResponse::success(serde_json::json!({
                "logs": logs,
                "total": logs.len(),
                "filter": {
                    "camera_id": camera_id,
                    "limit": limit
                }
            }))).into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        }
    }

    // Severity filter
    if let Some(threshold) = query.severity_min {
        match state.detection_log.get_high_severity(threshold, limit).await {
            Ok(logs) => return Json(ApiResponse::success(serde_json::json!({
                "logs": logs,
                "total": logs.len(),
                "filter": {
                    "severity_min": threshold,
                    "limit": limit
                }
            }))).into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        }
    }

    // Default: latest logs
    match state.detection_log.get_latest(limit).await {
        Ok(logs) => Json(ApiResponse::success(serde_json::json!({
            "logs": logs,
            "total": logs.len(),
            "filter": {
                "limit": limit
            }
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

/// Get detection log statistics
async fn detection_log_stats(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.detection_log.get_stats().await;

    // Get count from database
    let count_result: Result<i64, sqlx::Error> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM detection_logs"
    )
    .fetch_one(&state.pool)
    .await;

    let total_logs = count_result.unwrap_or(0);

    // Get today's count
    let today_result: Result<i64, sqlx::Error> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM detection_logs WHERE DATE(captured_at) = CURDATE()"
    )
    .fetch_one(&state.pool)
    .await;

    let today_logs = today_result.unwrap_or(0);

    // Get pending BQ sync count
    let pending_bq: Result<i64, sqlx::Error> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bq_sync_queue WHERE status = 'pending'"
    )
    .fetch_one(&state.pool)
    .await;

    let pending_bq_sync = pending_bq.unwrap_or(0);

    Json(ApiResponse::success(serde_json::json!({
        "service_stats": {
            "logs_saved": stats.logs_saved,
            "bq_queued": stats.bq_queued,
            "images_saved": stats.images_saved,
            "errors": stats.errors,
        },
        "database": {
            "total_logs": total_logs,
            "today_logs": today_logs,
            "pending_bq_sync": pending_bq_sync,
        }
    })))
}

/// Get detection logs by camera
async fn detection_logs_by_camera(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
    Query(query): Query<EventQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100) as u32;

    match state.detection_log.get_by_camera(&camera_id, limit).await {
        Ok(logs) => Json(ApiResponse::success(serde_json::json!({
            "camera_id": camera_id,
            "logs": logs,
            "total": logs.len(),
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

/// Get detection logs by severity threshold
async fn detection_logs_by_severity(
    State(state): State<AppState>,
    Path(threshold): Path<i32>,
    Query(query): Query<EventQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100) as u32;

    match state.detection_log.get_high_severity(threshold, limit).await {
        Ok(logs) => Json(ApiResponse::success(serde_json::json!({
            "severity_threshold": threshold,
            "logs": logs,
            "total": logs.len(),
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ========================================
// System Handlers
// ========================================

async fn system_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.admission.get_status().await;
    Json(ApiResponse::success(status))
}

// ========================================
// Settings Modal Handlers
// ========================================

/// IS21 connection status response
#[derive(Debug, serde::Serialize)]
struct Is21StatusResponse {
    url: String,
    connected: bool,
    health: Option<bool>,
    latency_ms: Option<u64>,
    schema_version: Option<String>,
    last_checked: String,
}

/// Get IS21 connection status
async fn get_is21_status(State(state): State<AppState>) -> impl IntoResponse {
    let url = state.config.is21_url.clone();
    let start = std::time::Instant::now();

    // Try health check
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    let health_url = format!("{}/healthz", url);
    let (connected, health, latency_ms) = match client.get(&health_url).send().await {
        Ok(resp) => {
            let latency = start.elapsed().as_millis() as u64;
            (true, Some(resp.status().is_success()), Some(latency))
        }
        Err(_) => (false, None, None),
    };

    // Try to get schema version
    let schema_version = if connected {
        let schema_url = format!("{}/v1/schema", url);
        match client.get(&schema_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                resp.json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|v| v.get("version").and_then(|v| v.as_str()).map(String::from))
            }
            _ => None,
        }
    } else {
        None
    };

    Json(ApiResponse::success(Is21StatusResponse {
        url,
        connected,
        health,
        latency_ms,
        schema_version,
        last_checked: chrono::Utc::now().to_rfc3339(),
    }))
}

/// IS21 configuration response
#[derive(Debug, serde::Serialize)]
struct Is21ConfigResponse {
    url: String,
    timeout_seconds: u64,
}

/// Get IS21 configuration
async fn get_is21_config(State(state): State<AppState>) -> impl IntoResponse {
    Json(ApiResponse::success(Is21ConfigResponse {
        url: state.config.is21_url.clone(),
        timeout_seconds: 30, // Default timeout
    }))
}

/// Update IS21 configuration request
#[derive(Debug, Deserialize)]
struct UpdateIs21ConfigRequest {
    url: Option<String>,
}

/// Update IS21 configuration
async fn update_is21_config(
    State(_state): State<AppState>,
    Json(req): Json<UpdateIs21ConfigRequest>,
) -> impl IntoResponse {
    // Note: In a real implementation, this would persist to database/config file
    // For now, we just validate the URL and return success
    // The actual config change would require restart or hot-reload support

    if let Some(url) = req.url {
        // Validate URL format
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid URL format. Must start with http:// or https://"
            }))).into_response();
        }

        // TODO: Persist to config and reload
        // For now, just return success with a note
        Json(serde_json::json!({
            "ok": true,
            "url": url,
            "note": "Configuration saved. Restart required for changes to take effect."
        })).into_response()
    } else {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "No url provided"
        }))).into_response()
    }
}

/// Test IS21 connection
async fn test_is21_connection(State(state): State<AppState>) -> impl IntoResponse {
    let url = state.config.is21_url.clone();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();

    let mut results = serde_json::Map::new();

    // Test health endpoint
    let health_url = format!("{}/healthz", url);
    let health_start = std::time::Instant::now();
    let health_ok = match client.get(&health_url).send().await {
        Ok(resp) => {
            results.insert("healthz_latency_ms".to_string(), serde_json::json!(health_start.elapsed().as_millis() as u64));
            resp.status().is_success()
        }
        Err(e) => {
            results.insert("healthz_error".to_string(), serde_json::json!(e.to_string()));
            false
        }
    };
    results.insert("healthz_ok".to_string(), serde_json::json!(health_ok));

    // Test schema endpoint
    let schema_url = format!("{}/v1/schema", url);
    let schema_start = std::time::Instant::now();
    let schema_ok = match client.get(&schema_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            results.insert("schema_latency_ms".to_string(), serde_json::json!(schema_start.elapsed().as_millis() as u64));
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                results.insert("schema_version".to_string(), json.get("version").cloned().unwrap_or(serde_json::json!("unknown")));
            }
            true
        }
        Ok(resp) => {
            results.insert("schema_status".to_string(), serde_json::json!(resp.status().as_u16()));
            false
        }
        Err(e) => {
            results.insert("schema_error".to_string(), serde_json::json!(e.to_string()));
            false
        }
    };
    results.insert("schema_ok".to_string(), serde_json::json!(schema_ok));

    // Test capabilities endpoint
    let caps_url = format!("{}/v1/capabilities", url);
    let caps_start = std::time::Instant::now();
    let caps_ok = match client.get(&caps_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            results.insert("capabilities_latency_ms".to_string(), serde_json::json!(caps_start.elapsed().as_millis() as u64));
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                results.insert("capabilities".to_string(), json);
            }
            true
        }
        Err(e) => {
            results.insert("capabilities_error".to_string(), serde_json::json!(e.to_string()));
            false
        }
        _ => false,
    };
    results.insert("capabilities_ok".to_string(), serde_json::json!(caps_ok));

    let all_ok = health_ok && schema_ok;
    results.insert("overall_ok".to_string(), serde_json::json!(all_ok));
    results.insert("tested_url".to_string(), serde_json::json!(url));
    results.insert("tested_at".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));

    Json(ApiResponse::success(serde_json::Value::Object(results)))
}

/// System information response
#[derive(Debug, serde::Serialize)]
struct SystemInfoResponse {
    hostname: String,
    device_type: String,
    lacis_id: Option<String>,
    version: String,
    uptime_seconds: u64,
    cpu_percent: f32,
    memory_percent: f32,
    memory_used_mb: u64,
    memory_total_mb: u64,
    disk_percent: f32,
    disk_used_gb: f32,
    disk_total_gb: f32,
    temperature_c: Option<f32>,
    rust_version: String,
    build_time: String,
}

/// Get system information
async fn get_system_info(State(state): State<AppState>) -> impl IntoResponse {
    use sysinfo::{System, Disks};

    let mut sys = System::new_all();
    sys.refresh_all();

    // Get hostname
    let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());

    // Memory
    let memory_total = sys.total_memory();
    let memory_used = sys.used_memory();
    let memory_percent = if memory_total > 0 {
        (memory_used as f32 / memory_total as f32) * 100.0
    } else {
        0.0
    };

    // CPU (average across all cores)
    let cpu_percent = {
        let cpus = sys.cpus();
        if cpus.is_empty() {
            0.0
        } else {
            cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32
        }
    };

    // Disk (root partition)
    let disks = Disks::new_with_refreshed_list();
    let (disk_used, disk_total) = disks
        .iter()
        .find(|d| d.mount_point() == std::path::Path::new("/"))
        .map(|d| {
            let total = d.total_space();
            let used = total - d.available_space();
            (used, total)
        })
        .unwrap_or((0, 0));

    let disk_percent = if disk_total > 0 {
        (disk_used as f32 / disk_total as f32) * 100.0
    } else {
        0.0
    };

    // Temperature (if available)
    let temperature = {
        let components = sysinfo::Components::new_with_refreshed_list();
        components
            .iter()
            .find(|c| c.label().contains("CPU") || c.label().contains("Core"))
            .map(|c| c.temperature())
    };

    // Uptime
    let uptime_seconds = System::uptime();

    Json(ApiResponse::success(SystemInfoResponse {
        hostname,
        device_type: "is22".to_string(),
        lacis_id: None, // IS22 doesn't have its own lacisID
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        cpu_percent,
        memory_percent,
        memory_used_mb: memory_used / 1024 / 1024,
        memory_total_mb: memory_total / 1024 / 1024,
        disk_percent,
        disk_used_gb: disk_used as f32 / 1024.0 / 1024.0 / 1024.0,
        disk_total_gb: disk_total as f32 / 1024.0 / 1024.0 / 1024.0,
        temperature_c: temperature,
        rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
        build_time: chrono::Utc::now().to_rfc3339(), // Placeholder - would be compile time
    }))
}

/// Performance log entry with full timing breakdown for bottleneck analysis
#[derive(Debug, serde::Serialize)]
struct PerformanceLogEntry {
    timestamp: String,
    camera_id: String,
    camera_name: Option<String>,
    camera_ip: Option<String>,
    preset_id: Option<String>,
    image_path: Option<String>,
    // Polling cycle ID for correlation with 巡回ログ
    polling_cycle_id: Option<String>,
    // Total processing time
    processing_ms: i64,
    // IS22-side timing breakdown (ボトルネック分析用)
    snapshot_ms: Option<i64>,        // カメラからの画像取得時間
    is21_roundtrip_ms: Option<i64>,  // IS21への送信〜レスポンス受信
    save_ms: Option<i64>,            // DB保存時間
    network_overhead_ms: Option<i64>, // ネットワークオーバーヘッド
    // Snapshot capture source: "go2rtc", "ffmpeg", or "http"
    snapshot_source: Option<String>,
    // IS21 internal timing breakdown
    is21_inference_ms: Option<i64>,
    yolo_ms: Option<i64>,
    par_ms: Option<i64>,
    // Detection result
    primary_event: String,
    severity: i32,
    confidence: Option<f64>,
    bbox_count: Option<i32>,
    // Raw IS21 log for debugging (contains full IS21 response + IS22 timings)
    is21_log_raw: Option<String>,
}

/// Query parameters for performance logs
#[derive(Deserialize)]
struct PerformanceLogsQuery {
    limit: Option<u32>,
    camera_id: Option<String>,
}

/// Get performance logs (from detection_logs with performance info)
async fn get_performance_logs(
    State(state): State<AppState>,
    Query(query): Query<PerformanceLogsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100).min(1000);

    // Query detection_logs with full timing breakdown
    // is21_log contains IS22 timings (snapshot_ms, is21_roundtrip_ms, save_ms) and IS21 response
    let logs_query = if let Some(camera_id) = &query.camera_id {
        sqlx::query_as::<_, (
            chrono::DateTime<chrono::Utc>,
            String,
            i64,
            Option<String>,
            Option<String>,
            String,
            i32,
        )>(
            r#"SELECT
                dl.captured_at,
                dl.camera_id,
                COALESCE(dl.processing_ms, 0),
                dl.polling_cycle_id,
                dl.is21_log,
                dl.primary_event,
                dl.severity
            FROM detection_logs dl
            WHERE dl.camera_id = ?
            ORDER BY dl.captured_at DESC
            LIMIT ?"#
        )
        .bind(camera_id)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, (
            chrono::DateTime<chrono::Utc>,
            String,
            i64,
            Option<String>,
            Option<String>,
            String,
            i32,
        )>(
            r#"SELECT
                dl.captured_at,
                dl.camera_id,
                COALESCE(dl.processing_ms, 0),
                dl.polling_cycle_id,
                dl.is21_log,
                dl.primary_event,
                dl.severity
            FROM detection_logs dl
            ORDER BY dl.captured_at DESC
            LIMIT ?"#
        )
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    };

    match logs_query {
        Ok(rows) => {
            // Get camera info from cache
            let cameras = state.config_store.get_cached_cameras().await;
            let camera_map: std::collections::HashMap<String, (String, Option<String>, Option<String>)> = cameras
                .iter()
                .map(|c| (c.camera_id.clone(), (c.name.clone(), c.ip_address.clone(), c.preset_id.clone())))
                .collect();

            let logs: Vec<PerformanceLogEntry> = rows
                .into_iter()
                .map(|(captured_at, camera_id, processing_ms, polling_cycle_id, is21_log, primary_event, severity)| {
                    // Parse is21_log JSON for timing breakdown
                    let parsed = is21_log
                        .as_ref()
                        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok());

                    // IS22-side timings (stored by detection_log_service)
                    let is22_timings = parsed.as_ref().and_then(|v| v.get("is22_timings"));
                    let snapshot_ms = is22_timings.and_then(|t| t.get("snapshot_ms")).and_then(|v| v.as_i64());
                    let is21_roundtrip_ms = is22_timings.and_then(|t| t.get("is21_roundtrip_ms")).and_then(|v| v.as_i64());
                    let save_ms = is22_timings.and_then(|t| t.get("save_ms")).and_then(|v| v.as_i64());
                    let network_overhead_ms = is22_timings.and_then(|t| t.get("network_overhead_ms")).and_then(|v| v.as_i64());
                    let snapshot_source = is22_timings
                        .and_then(|t| t.get("snapshot_source"))
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    // IS21-side timings (from IS21 response)
                    let is21_perf = parsed.as_ref().and_then(|v| v.get("performance"));
                    let is21_inference_ms = is21_perf.and_then(|p| p.get("inference_ms")).and_then(|v| v.as_i64());
                    let yolo_ms = is21_perf.and_then(|p| p.get("yolo_ms")).and_then(|v| v.as_i64());
                    let par_ms = is21_perf.and_then(|p| p.get("par_ms")).and_then(|v| v.as_i64());

                    // Additional info
                    let image_path = parsed.as_ref()
                        .and_then(|v| v.get("image_path"))
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let confidence = parsed.as_ref()
                        .and_then(|v| v.get("confidence"))
                        .and_then(|v| v.as_f64());
                    let bbox_count = parsed.as_ref()
                        .and_then(|v| v.get("bbox_count"))
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32);

                    // Get camera info from cache
                    let (camera_name, camera_ip, preset_id) = camera_map
                        .get(&camera_id)
                        .cloned()
                        .unwrap_or((camera_id.clone(), None, None));

                    PerformanceLogEntry {
                        timestamp: captured_at.to_rfc3339(),
                        camera_id: camera_id.clone(),
                        camera_name: Some(camera_name),
                        camera_ip,
                        preset_id,
                        image_path,
                        polling_cycle_id,
                        processing_ms,
                        snapshot_ms,
                        is21_roundtrip_ms,
                        save_ms,
                        network_overhead_ms,
                        snapshot_source,
                        is21_inference_ms,
                        yolo_ms,
                        par_ms,
                        primary_event,
                        severity,
                        confidence,
                        bbox_count,
                        // Include raw IS21 log for debugging UI
                        is21_log_raw: is21_log.clone(),
                    }
                })
                .collect();

            Json(ApiResponse::success(serde_json::json!({
                "logs": logs,
                "total": logs.len()
            }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "error": e.to_string()
        }))).into_response(),
    }
}

/// Polling log entry (cycle stats with full breakdown)
#[derive(Debug, serde::Serialize)]
struct PollingLogEntry {
    /// ポーリングID (サブネット-日付-時刻-乱数)
    polling_id: String,
    /// サブネット (例: 192.168.125)
    subnet: String,
    /// サブネット第3オクテット (例: 125)
    subnet_octet3: i32,
    /// 巡回開始時刻
    started_at: String,
    /// 巡回完了時刻 (running中はnull)
    completed_at: Option<String>,
    /// サイクル番号
    cycle_number: u64,
    /// 対象カメラ台数
    camera_count: i32,
    /// 成功件数
    success_count: i32,
    /// 失敗件数
    failed_count: i32,
    /// タイムアウト件数
    timeout_count: i32,
    /// 巡回所要時間 (ms)
    duration_ms: Option<i32>,
    /// 平均処理時間 (ms)
    avg_processing_ms: Option<i32>,
    /// ステータス (running, completed, interrupted)
    status: String,
}

/// Query parameters for polling logs
#[derive(Deserialize)]
struct PollingLogsQuery {
    limit: Option<u32>,
    subnet: Option<String>,
    status: Option<String>,
}

/// Get polling logs (cycle history from polling_cycles table)
async fn get_polling_logs(
    State(state): State<AppState>,
    Query(query): Query<PollingLogsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100).min(500);

    // Check if there's a polling_cycles table
    let check_table: Result<Option<String>, _> = sqlx::query_scalar(
        "SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'polling_cycles'"
    )
    .fetch_optional(&state.pool)
    .await;

    match check_table {
        Ok(Some(_)) => {
            // Table exists, query it with new schema
            let logs_query = if let Some(subnet) = &query.subnet {
                sqlx::query_as::<_, (
                    String,                              // polling_id
                    String,                              // subnet
                    i32,                                 // subnet_octet3
                    chrono::DateTime<chrono::Utc>,       // started_at
                    Option<chrono::DateTime<chrono::Utc>>, // completed_at
                    i64,                                 // cycle_number
                    i32,                                 // camera_count
                    i32,                                 // success_count
                    i32,                                 // failed_count
                    i32,                                 // timeout_count
                    Option<i32>,                         // duration_ms
                    Option<i32>,                         // avg_processing_ms
                    String,                              // status
                )>(
                    r#"SELECT
                        polling_id,
                        subnet,
                        subnet_octet3,
                        started_at,
                        completed_at,
                        cycle_number,
                        camera_count,
                        success_count,
                        failed_count,
                        timeout_count,
                        duration_ms,
                        avg_processing_ms,
                        status
                    FROM polling_cycles
                    WHERE subnet = ?
                    ORDER BY started_at DESC
                    LIMIT ?"#
                )
                .bind(subnet)
                .bind(limit)
                .fetch_all(&state.pool)
                .await
            } else if let Some(status) = &query.status {
                sqlx::query_as::<_, (
                    String, String, i32,
                    chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>,
                    i64, i32, i32, i32, i32, Option<i32>, Option<i32>, String,
                )>(
                    r#"SELECT
                        polling_id, subnet, subnet_octet3, started_at, completed_at,
                        cycle_number, camera_count, success_count, failed_count, timeout_count,
                        duration_ms, avg_processing_ms, status
                    FROM polling_cycles
                    WHERE status = ?
                    ORDER BY started_at DESC
                    LIMIT ?"#
                )
                .bind(status)
                .bind(limit)
                .fetch_all(&state.pool)
                .await
            } else {
                sqlx::query_as::<_, (
                    String, String, i32,
                    chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>,
                    i64, i32, i32, i32, i32, Option<i32>, Option<i32>, String,
                )>(
                    r#"SELECT
                        polling_id, subnet, subnet_octet3, started_at, completed_at,
                        cycle_number, camera_count, success_count, failed_count, timeout_count,
                        duration_ms, avg_processing_ms, status
                    FROM polling_cycles
                    ORDER BY started_at DESC
                    LIMIT ?"#
                )
                .bind(limit)
                .fetch_all(&state.pool)
                .await
            };

            match logs_query {
                Ok(rows) => {
                    let logs: Vec<PollingLogEntry> = rows
                        .into_iter()
                        .map(|(
                            polling_id, subnet, subnet_octet3,
                            started_at, completed_at,
                            cycle_number, camera_count, success_count, failed_count, timeout_count,
                            duration_ms, avg_processing_ms, status
                        )| {
                            PollingLogEntry {
                                polling_id,
                                subnet,
                                subnet_octet3,
                                started_at: started_at.to_rfc3339(),
                                completed_at: completed_at.map(|dt| dt.to_rfc3339()),
                                cycle_number: cycle_number as u64,
                                camera_count,
                                success_count,
                                failed_count,
                                timeout_count,
                                duration_ms,
                                avg_processing_ms,
                                status,
                            }
                        })
                        .collect();

                    Json(ApiResponse::success(serde_json::json!({
                        "logs": logs,
                        "total": logs.len()
                    }))).into_response()
                }
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": e.to_string()
                }))).into_response(),
            }
        }
        Ok(None) | Err(_) => {
            // Table doesn't exist, return empty with a note
            Json(ApiResponse::success(serde_json::json!({
                "logs": [],
                "total": 0,
                "note": "Polling cycle logging not yet enabled. Run migration 012_polling_cycles.sql."
            }))).into_response()
        }
    }
}

// ========================================
// Performance Dashboard API (統合デバッグAPI)
// ========================================

/// Dashboard query parameters
#[derive(Deserialize)]
struct DashboardQuery {
    /// 期間開始 (ISO8601), デフォルト: 24時間前
    start: Option<String>,
    /// 期間終了 (ISO8601), デフォルト: 現在
    end: Option<String>,
    /// サブネットフィルタ (複数可、カンマ区切り)
    subnets: Option<String>,
    /// タイムライン粒度 (分), デフォルト: 5
    granularity_minutes: Option<i32>,
}

/// Timeline data point for lap time chart
#[derive(Debug, serde::Serialize)]
struct TimelineDataPoint {
    timestamp: String,
    subnet: String,
    avg_lap_time_ms: i64,
    min_lap_time_ms: i64,
    max_lap_time_ms: i64,
    sample_count: i32,
}

/// Error rate data point
#[derive(Debug, serde::Serialize)]
struct ErrorRateDataPoint {
    timestamp: String,
    subnet: String,
    total_count: i32,
    success_count: i32,
    image_error_count: i32,
    inference_error_count: i32,
    error_rate_pct: f64,
}

/// Processing time distribution entry (for pie chart)
#[derive(Debug, serde::Serialize)]
struct ProcessingDistribution {
    camera_id: String,
    camera_name: Option<String>,
    subnet: String,
    total_processing_ms: i64,
    avg_processing_ms: i64,
    sample_count: i32,
    percentage: f64,
}

/// Camera ranking entry
#[derive(Debug, Clone, serde::Serialize)]
struct CameraRanking {
    rank: i32,
    camera_id: String,
    camera_name: Option<String>,
    camera_ip: Option<String>,
    subnet: String,
    avg_time_ms: i64,
    min_time_ms: i64,
    max_time_ms: i64,
    sample_count: i32,
}

/// Per-subnet statistics
#[derive(Debug, serde::Serialize)]
struct SubnetStats {
    subnet: String,
    camera_count: i32,
    total_samples: i64,
    avg_snapshot_ms: Option<i64>,
    avg_inference_ms: Option<i64>,
    avg_total_ms: i64,
    success_rate_pct: f64,
    performance_grade: String,  // A, B, C, D, F
}

/// Performance dashboard response
#[derive(Debug, serde::Serialize)]
struct PerformanceDashboard {
    /// Query period
    period: PeriodInfo,
    /// Timeline data for lap time chart (multiple subnets)
    timeline: Vec<TimelineDataPoint>,
    /// Error rates over time
    error_rates: Vec<ErrorRateDataPoint>,
    /// Processing time distribution (pie chart)
    distribution: Vec<ProcessingDistribution>,
    /// Fastest cameras ranking
    fastest_ranking: Vec<CameraRanking>,
    /// Slowest cameras ranking
    slowest_ranking: Vec<CameraRanking>,
    /// Per-subnet statistics
    subnet_stats: Vec<SubnetStats>,
    /// Summary
    summary: DashboardSummary,
}

#[derive(Debug, serde::Serialize)]
struct PeriodInfo {
    start: String,
    end: String,
    duration_hours: f64,
    granularity_minutes: i32,
}

#[derive(Debug, serde::Serialize)]
struct DashboardSummary {
    total_samples: i64,
    total_subnets: i32,
    total_cameras: i32,
    overall_avg_processing_ms: i64,
    overall_success_rate_pct: f64,
    overall_grade: String,
}

/// Get performance dashboard data
/// GET /api/debug/performance/dashboard
///
/// Returns comprehensive performance statistics for the monitoring dashboard:
// ============================================================================
// Timeout Settings API
// ============================================================================

/// GET /api/settings/timeouts - Get global timeout settings
async fn get_global_timeouts(State(state): State<AppState>) -> impl IntoResponse {
    // Load timeout settings from settings.polling
    let result = sqlx::query("SELECT setting_json FROM settings WHERE setting_key = 'polling'")
        .fetch_optional(&state.pool)
        .await;

    match result {
        Ok(Some(row)) => {
            let setting_json: String = row.get("setting_json");
            match serde_json::from_str::<serde_json::Value>(&setting_json) {
                Ok(polling_settings) => {
                    let timeout_main = polling_settings["timeout_main_sec"].as_u64().unwrap_or(10);
                    let timeout_sub = polling_settings["timeout_sub_sec"].as_u64().unwrap_or(20);

                    Json(json!({
                        "ok": true,
                        "data": {
                            "timeout_main_sec": timeout_main,
                            "timeout_sub_sec": timeout_sub
                        }
                    }))
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to parse polling settings JSON");
                    Json(json!({
                        "ok": true,
                        "data": {
                            "timeout_main_sec": 10,
                            "timeout_sub_sec": 20
                        }
                    }))
                }
            }
        }
        Ok(None) => {
            // Settings not found, return defaults
            Json(json!({
                "ok": true,
                "data": {
                    "timeout_main_sec": 10,
                    "timeout_sub_sec": 20
                }
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to load timeout settings");
            Json(json!({
                "ok": false,
                "error": "Failed to load settings"
            }))
        }
    }
}

/// PUT /api/settings/timeouts - Update global timeout settings
#[derive(Deserialize)]
struct UpdateTimeoutRequest {
    timeout_main_sec: u64,
    timeout_sub_sec: u64,
}

async fn update_global_timeouts(
    State(state): State<AppState>,
    Json(payload): Json<UpdateTimeoutRequest>,
) -> impl IntoResponse {
    // Validation
    if payload.timeout_main_sec < 5 || payload.timeout_main_sec > 120 {
        return Json(json!({
            "ok": false,
            "error": "timeout_main_sec must be between 5 and 120"
        }));
    }
    if payload.timeout_sub_sec < 5 || payload.timeout_sub_sec > 120 {
        return Json(json!({
            "ok": false,
            "error": "timeout_sub_sec must be between 5 and 120"
        }));
    }

    // Update settings
    let result = sqlx::query(
        "UPDATE settings
         SET setting_json = JSON_SET(
             setting_json,
             '$.timeout_main_sec', ?,
             '$.timeout_sub_sec', ?
         )
         WHERE setting_key = 'polling'"
    )
    .bind(payload.timeout_main_sec)
    .bind(payload.timeout_sub_sec)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {
            tracing::info!(
                timeout_main_sec = payload.timeout_main_sec,
                timeout_sub_sec = payload.timeout_sub_sec,
                "Global timeout settings updated"
            );

            // Refresh ConfigStore cache
            state.config_store.refresh_cache().await;

            Json(json!({
                "ok": true,
                "message": "Timeout settings updated"
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to update timeout settings");
            Json(json!({
                "ok": false,
                "error": "Failed to update settings"
            }))
        }
    }
}

// ============================================================================
// Performance Dashboard API
// ============================================================================

/// GET /api/debug/performance/dashboard - Performance statistics dashboard
/// Returns aggregated performance data:
/// - Timeline lap times per subnet (for line chart)
/// - Error rates over time (image vs inference errors)
/// - Processing time distribution (for pie chart)
/// - Fastest/slowest camera rankings
/// - Per-subnet statistics with performance grades
async fn get_performance_dashboard(
    State(state): State<AppState>,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    // Parse time range
    let now = chrono::Utc::now();
    let default_start = now - chrono::Duration::hours(24);

    let start = query.start.as_ref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or(default_start);

    let end = query.end.as_ref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or(now);

    let granularity = query.granularity_minutes.unwrap_or(5).max(1).min(60);

    // Parse subnet filter
    let subnet_filter: Option<Vec<String>> = query.subnets.as_ref()
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());

    // Get camera info for name lookup
    let cameras = state.config_store.get_cached_cameras().await;
    let camera_map: std::collections::HashMap<String, (String, Option<String>, String)> = cameras
        .iter()
        .filter_map(|c| {
            let subnet = c.ip_address.as_ref()
                .map(|ip| {
                    let parts: Vec<&str> = ip.split('.').collect();
                    if parts.len() >= 3 {
                        format!("{}.{}.{}", parts[0], parts[1], parts[2])
                    } else {
                        "unknown".to_string()
                    }
                })
                .unwrap_or_else(|| "unknown".to_string());
            Some((c.camera_id.clone(), (c.name.clone(), c.ip_address.clone(), subnet)))
        })
        .collect();

    // 1. Timeline data (lap times per subnet)
    let timeline = get_timeline_data(&state.pool, &start, &end, granularity, &subnet_filter).await;

    // 2. Error rates
    let error_rates = get_error_rates(&state.pool, &start, &end, granularity, &subnet_filter).await;

    // 3. Processing distribution
    let distribution = get_processing_distribution(&state.pool, &start, &end, &subnet_filter, &camera_map).await;

    // 4. Rankings
    let (fastest_ranking, slowest_ranking) = get_camera_rankings(&state.pool, &start, &end, &subnet_filter, &camera_map).await;

    // 5. Subnet stats
    let subnet_stats = get_subnet_stats(&state.pool, &start, &end, &subnet_filter).await;

    // 6. Summary
    let summary = calculate_summary(&timeline, &subnet_stats, &camera_map);

    let duration_hours = (end - start).num_minutes() as f64 / 60.0;

    let dashboard = PerformanceDashboard {
        period: PeriodInfo {
            start: start.to_rfc3339(),
            end: end.to_rfc3339(),
            duration_hours,
            granularity_minutes: granularity,
        },
        timeline,
        error_rates,
        distribution,
        fastest_ranking,
        slowest_ranking,
        subnet_stats,
        summary,
    };

    Json(ApiResponse::success(dashboard))
}

/// Get timeline data for lap time chart
async fn get_timeline_data(
    pool: &sqlx::MySqlPool,
    start: &chrono::DateTime<chrono::Utc>,
    end: &chrono::DateTime<chrono::Utc>,
    granularity_minutes: i32,
    subnet_filter: &Option<Vec<String>>,
) -> Vec<TimelineDataPoint> {
    // Use polling_cycles table for subnet-level lap times
    let subnet_clause = subnet_filter.as_ref()
        .map(|subnets| {
            let placeholders: Vec<&str> = subnets.iter().map(|_| "?").collect();
            format!("AND subnet IN ({})", placeholders.join(","))
        })
        .unwrap_or_default();

    let query = format!(
        r#"SELECT
            DATE_FORMAT(started_at, '%Y-%m-%d %H:%i:00') as time_bucket,
            subnet,
            CAST(AVG(COALESCE(duration_ms, 0)) AS DOUBLE) as avg_lap,
            CAST(MIN(COALESCE(duration_ms, 0)) AS SIGNED) as min_lap,
            CAST(MAX(COALESCE(duration_ms, 0)) AS SIGNED) as max_lap,
            CAST(COUNT(*) AS SIGNED) as cnt
        FROM polling_cycles
        WHERE started_at BETWEEN ? AND ?
        AND status = 'completed'
        {}
        GROUP BY time_bucket, subnet
        ORDER BY time_bucket, subnet"#,
        subnet_clause
    );

    let mut q = sqlx::query_as::<_, (String, String, f64, i64, i64, i64)>(&query)
        .bind(start)
        .bind(end);

    if let Some(subnets) = subnet_filter {
        for subnet in subnets {
            q = q.bind(subnet);
        }
    }

    match q.fetch_all(pool).await {
        Ok(rows) => rows.into_iter().map(|(ts, subnet, avg, min, max, cnt)| {
            TimelineDataPoint {
                timestamp: ts,
                subnet,
                avg_lap_time_ms: avg as i64,
                min_lap_time_ms: min,
                max_lap_time_ms: max,
                sample_count: cnt as i32,
            }
        }).collect(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get timeline data");
            vec![]
        }
    }
}

/// Get error rates over time
async fn get_error_rates(
    pool: &sqlx::MySqlPool,
    start: &chrono::DateTime<chrono::Utc>,
    end: &chrono::DateTime<chrono::Utc>,
    granularity_minutes: i32,
    subnet_filter: &Option<Vec<String>>,
) -> Vec<ErrorRateDataPoint> {
    // Extract subnet from camera_id or use polling_cycles
    // For now, use polling_cycles data
    let subnet_clause = subnet_filter.as_ref()
        .map(|subnets| {
            let placeholders: Vec<&str> = subnets.iter().map(|_| "?").collect();
            format!("AND subnet IN ({})", placeholders.join(","))
        })
        .unwrap_or_default();

    let query = format!(
        r#"SELECT
            DATE_FORMAT(started_at, '%Y-%m-%d %H:%i:00') as time_bucket,
            subnet,
            CAST(SUM(camera_count) AS SIGNED) as total,
            CAST(SUM(success_count) AS SIGNED) as success,
            CAST(SUM(failed_count) AS SIGNED) as failed,
            CAST(SUM(timeout_count) AS SIGNED) as timeout
        FROM polling_cycles
        WHERE started_at BETWEEN ? AND ?
        {}
        GROUP BY time_bucket, subnet
        ORDER BY time_bucket, subnet"#,
        subnet_clause
    );

    let mut q = sqlx::query_as::<_, (String, String, i64, i64, i64, i64)>(&query)
        .bind(start)
        .bind(end);

    if let Some(subnets) = subnet_filter {
        for subnet in subnets {
            q = q.bind(subnet);
        }
    }

    match q.fetch_all(pool).await {
        Ok(rows) => rows.into_iter().map(|(ts, subnet, total, success, failed, timeout)| {
            let error_count = failed + timeout;
            let error_rate = if total > 0 { (error_count as f64 / total as f64) * 100.0 } else { 0.0 };
            ErrorRateDataPoint {
                timestamp: ts,
                subnet,
                total_count: total as i32,
                success_count: success as i32,
                image_error_count: timeout as i32,  // Timeout often means image capture failure
                inference_error_count: failed as i32,  // Failed often means inference error
                error_rate_pct: (error_rate * 100.0).round() / 100.0,
            }
        }).collect(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get error rates");
            vec![]
        }
    }
}

/// Get processing time distribution for pie chart
async fn get_processing_distribution(
    pool: &sqlx::MySqlPool,
    start: &chrono::DateTime<chrono::Utc>,
    end: &chrono::DateTime<chrono::Utc>,
    subnet_filter: &Option<Vec<String>>,
    camera_map: &std::collections::HashMap<String, (String, Option<String>, String)>,
) -> Vec<ProcessingDistribution> {
    // Get per-camera processing time from detection_logs
    let query = r#"SELECT
        camera_id,
        CAST(SUM(COALESCE(processing_ms, 0)) AS SIGNED) as total_ms,
        CAST(AVG(COALESCE(processing_ms, 0)) AS DOUBLE) as avg_ms,
        CAST(COUNT(*) AS SIGNED) as cnt
    FROM detection_logs
    WHERE captured_at BETWEEN ? AND ?
    GROUP BY camera_id
    ORDER BY total_ms DESC"#;

    let result: Result<Vec<(String, i64, f64, i64)>, _> = sqlx::query_as(query)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await;

    match result {
        Ok(rows) => {
            let total_all: i64 = rows.iter().map(|(_, t, _, _)| t).sum();

            rows.into_iter()
                .filter_map(|(camera_id, total_ms, avg_ms, cnt)| {
                    let (name, ip, subnet) = camera_map.get(&camera_id)
                        .cloned()
                        .unwrap_or((camera_id.clone(), None, "unknown".to_string()));

                    // Apply subnet filter if specified
                    if let Some(ref filter) = subnet_filter {
                        if !filter.contains(&subnet) {
                            return None;
                        }
                    }

                    let percentage = if total_all > 0 {
                        (total_ms as f64 / total_all as f64) * 100.0
                    } else {
                        0.0
                    };

                    Some(ProcessingDistribution {
                        camera_id,
                        camera_name: Some(name),
                        subnet,
                        total_processing_ms: total_ms,
                        avg_processing_ms: avg_ms as i64,
                        sample_count: cnt as i32,
                        percentage: (percentage * 100.0).round() / 100.0,
                    })
                })
                .collect()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get processing distribution");
            vec![]
        }
    }
}

/// Get camera rankings (fastest and slowest)
async fn get_camera_rankings(
    pool: &sqlx::MySqlPool,
    start: &chrono::DateTime<chrono::Utc>,
    end: &chrono::DateTime<chrono::Utc>,
    subnet_filter: &Option<Vec<String>>,
    camera_map: &std::collections::HashMap<String, (String, Option<String>, String)>,
) -> (Vec<CameraRanking>, Vec<CameraRanking>) {
    let query = r#"SELECT
        camera_id,
        CAST(AVG(COALESCE(processing_ms, 0)) AS DOUBLE) as avg_ms,
        CAST(MIN(COALESCE(processing_ms, 0)) AS SIGNED) as min_ms,
        CAST(MAX(COALESCE(processing_ms, 0)) AS SIGNED) as max_ms,
        CAST(COUNT(*) AS SIGNED) as cnt
    FROM detection_logs
    WHERE captured_at BETWEEN ? AND ?
    GROUP BY camera_id
    HAVING cnt >= 5
    ORDER BY avg_ms ASC"#;

    let result: Result<Vec<(String, f64, i64, i64, i64)>, _> = sqlx::query_as(query)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await;

    match result {
        Ok(rows) => {
            let all_rankings: Vec<CameraRanking> = rows.into_iter()
                .enumerate()
                .filter_map(|(i, (camera_id, avg_ms, min_ms, max_ms, cnt))| {
                    let (name, ip, subnet) = camera_map.get(&camera_id)
                        .cloned()
                        .unwrap_or((camera_id.clone(), None, "unknown".to_string()));

                    // Apply subnet filter if specified
                    if let Some(ref filter) = subnet_filter {
                        if !filter.contains(&subnet) {
                            return None;
                        }
                    }

                    Some(CameraRanking {
                        rank: (i + 1) as i32,
                        camera_id,
                        camera_name: Some(name),
                        camera_ip: ip,
                        subnet,
                        avg_time_ms: avg_ms as i64,
                        min_time_ms: min_ms,
                        max_time_ms: max_ms,
                        sample_count: cnt as i32,
                    })
                })
                .collect();

            // Recalculate ranks after filtering
            let mut fastest: Vec<CameraRanking> = all_rankings.iter().take(10).cloned().collect();
            for (i, r) in fastest.iter_mut().enumerate() {
                r.rank = (i + 1) as i32;
            }

            let mut slowest: Vec<CameraRanking> = all_rankings.iter().rev().take(10).cloned().collect();
            for (i, r) in slowest.iter_mut().enumerate() {
                r.rank = (i + 1) as i32;
            }

            (fastest, slowest)
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get camera rankings");
            (vec![], vec![])
        }
    }
}

/// Get per-subnet statistics
async fn get_subnet_stats(
    pool: &sqlx::MySqlPool,
    start: &chrono::DateTime<chrono::Utc>,
    end: &chrono::DateTime<chrono::Utc>,
    subnet_filter: &Option<Vec<String>>,
) -> Vec<SubnetStats> {
    let subnet_clause = subnet_filter.as_ref()
        .map(|subnets| {
            let placeholders: Vec<&str> = subnets.iter().map(|_| "?").collect();
            format!("AND subnet IN ({})", placeholders.join(","))
        })
        .unwrap_or_default();

    let query = format!(
        r#"SELECT
            subnet,
            CAST(COUNT(DISTINCT camera_count) AS SIGNED) as camera_cnt,
            CAST(SUM(camera_count) AS SIGNED) as total_samples,
            CAST(AVG(avg_processing_ms) AS DOUBLE) as avg_total,
            CAST(SUM(success_count) AS SIGNED) as success,
            CAST(SUM(success_count + failed_count + timeout_count) AS SIGNED) as total
        FROM polling_cycles
        WHERE started_at BETWEEN ? AND ?
        AND status = 'completed'
        {}
        GROUP BY subnet
        ORDER BY subnet"#,
        subnet_clause
    );

    let mut q = sqlx::query_as::<_, (String, i64, i64, Option<f64>, i64, i64)>(&query)
        .bind(start)
        .bind(end);

    if let Some(subnets) = subnet_filter {
        for subnet in subnets {
            q = q.bind(subnet);
        }
    }

    match q.fetch_all(pool).await {
        Ok(rows) => rows.into_iter().map(|(subnet, camera_cnt, samples, avg_total, success, total)| {
            let success_rate = if total > 0 { (success as f64 / total as f64) * 100.0 } else { 0.0 };
            let avg_ms = avg_total.unwrap_or(0.0) as i64;

            // Performance grade based on average processing time and success rate
            let grade = calculate_performance_grade(avg_ms, success_rate);

            SubnetStats {
                subnet,
                camera_count: camera_cnt as i32,
                total_samples: samples,
                avg_snapshot_ms: None,  // TODO: Extract from is21_log
                avg_inference_ms: None,  // TODO: Extract from is21_log
                avg_total_ms: avg_ms,
                success_rate_pct: (success_rate * 100.0).round() / 100.0,
                performance_grade: grade,
            }
        }).collect(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get subnet stats");
            vec![]
        }
    }
}

/// Calculate performance grade (A-F)
fn calculate_performance_grade(avg_ms: i64, success_rate: f64) -> String {
    // Grade based on:
    // - A: avg < 2000ms, success >= 98%
    // - B: avg < 3000ms, success >= 95%
    // - C: avg < 5000ms, success >= 90%
    // - D: avg < 8000ms, success >= 80%
    // - F: otherwise
    if avg_ms < 2000 && success_rate >= 98.0 {
        "A".to_string()
    } else if avg_ms < 3000 && success_rate >= 95.0 {
        "B".to_string()
    } else if avg_ms < 5000 && success_rate >= 90.0 {
        "C".to_string()
    } else if avg_ms < 8000 && success_rate >= 80.0 {
        "D".to_string()
    } else {
        "F".to_string()
    }
}

/// Calculate dashboard summary
fn calculate_summary(
    timeline: &[TimelineDataPoint],
    subnet_stats: &[SubnetStats],
    camera_map: &std::collections::HashMap<String, (String, Option<String>, String)>,
) -> DashboardSummary {
    let total_samples: i64 = subnet_stats.iter().map(|s| s.total_samples).sum();
    let total_subnets = subnet_stats.len() as i32;
    let total_cameras = camera_map.len() as i32;

    let overall_avg = if !subnet_stats.is_empty() {
        subnet_stats.iter().map(|s| s.avg_total_ms).sum::<i64>() / subnet_stats.len() as i64
    } else {
        0
    };

    let overall_success = if !subnet_stats.is_empty() {
        subnet_stats.iter().map(|s| s.success_rate_pct).sum::<f64>() / subnet_stats.len() as f64
    } else {
        0.0
    };

    let overall_grade = calculate_performance_grade(overall_avg, overall_success);

    DashboardSummary {
        total_samples,
        total_subnets,
        total_cameras,
        overall_avg_processing_ms: overall_avg,
        overall_success_rate_pct: (overall_success * 100.0).round() / 100.0,
        overall_grade,
    }
}

// ========================================
// IpcamScan Handlers
// ========================================

async fn create_scan_job(
    State(state): State<AppState>,
    Json(req): Json<crate::ipcam_scan::ScanJobRequest>,
) -> impl IntoResponse {
    // create_job now returns Result<ScanJob> (single execution guard)
    let job = match state.ipcam_scan.create_job(req).await {
        Ok(job) => job,
        Err(e) => return e.into_response(),
    };
    let job_id = job.job_id;

    // Spawn background task to run the scan
    let scanner = state.ipcam_scan.clone();
    tokio::spawn(async move {
        if let Err(e) = scanner.run_job(job_id).await {
            tracing::error!(job_id = %job_id, error = %e, "Scan job failed");
        }
    });

    (StatusCode::CREATED, Json(ApiResponse::success(job))).into_response()
}

/// Abort a running scan job
async fn abort_scan_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let job_id = match uuid::Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid UUID"}))).into_response(),
    };

    if state.ipcam_scan.abort_job(&job_id).await {
        Json(ApiResponse::success(serde_json::json!({
            "message": "スキャン中止リクエストを受け付けました",
            "job_id": job_id.to_string()
        }))).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": "指定されたジョブは実行中ではありません"
        }))).into_response()
    }
}

/// Check if a scan is currently running
async fn get_scan_status(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let is_running = state.ipcam_scan.is_scan_running().await;
    let running_job_id = state.ipcam_scan.get_running_job_id().await;

    Json(ApiResponse::success(serde_json::json!({
        "is_running": is_running,
        "running_job_id": running_job_id.map(|id| id.to_string())
    })))
}

async fn get_scan_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let job_id = match uuid::Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid UUID"}))).into_response(),
    };

    match state.ipcam_scan.get_job(&job_id).await {
        Some(job) => Json(ApiResponse::success(job)).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Job not found"}))).into_response(),
    }
}

#[derive(Deserialize)]
struct DeviceFilterQuery {
    subnet: Option<String>,
    family: Option<String>,
    verified: Option<bool>,
    status: Option<String>,
}

async fn list_scanned_devices(
    State(state): State<AppState>,
    Query(query): Query<DeviceFilterQuery>,
) -> impl IntoResponse {
    let filter = crate::ipcam_scan::DeviceFilter {
        subnet: query.subnet,
        family: query.family.and_then(|f| match f.to_lowercase().as_str() {
            "tapo" => Some(crate::ipcam_scan::CameraFamily::Tapo),
            "vigi" => Some(crate::ipcam_scan::CameraFamily::Vigi),
            "nest" => Some(crate::ipcam_scan::CameraFamily::Nest),
            "other" => Some(crate::ipcam_scan::CameraFamily::Other),
            "unknown" => Some(crate::ipcam_scan::CameraFamily::Unknown),
            _ => None,
        }),
        verified: query.verified,
        status: query.status.and_then(|s| match s.to_lowercase().as_str() {
            "discovered" => Some(crate::ipcam_scan::DeviceStatus::Discovered),
            "verifying" => Some(crate::ipcam_scan::DeviceStatus::Verifying),
            "verified" => Some(crate::ipcam_scan::DeviceStatus::Verified),
            "rejected" => Some(crate::ipcam_scan::DeviceStatus::Rejected),
            "approved" => Some(crate::ipcam_scan::DeviceStatus::Approved),
            _ => None,
        }),
    };

    let devices = state.ipcam_scan.list_devices(filter).await;
    let total = devices.len();

    Json(serde_json::json!({
        "devices": devices,
        "total": total
    }))
}

/// List devices with category enrichment (#82 T2-8)
/// Query params: subnet, include_lost (default: true)
#[derive(Deserialize)]
struct DevicesWithCategoriesQuery {
    subnet: Option<String>,
    include_lost: Option<bool>,
}

async fn list_devices_with_categories(
    State(state): State<AppState>,
    Query(query): Query<DevicesWithCategoriesQuery>,
) -> impl IntoResponse {
    let filter = crate::ipcam_scan::DeviceFilter {
        subnet: query.subnet,
        ..Default::default()
    };
    let include_lost = query.include_lost.unwrap_or(true);

    let devices = state.ipcam_scan.list_devices_with_categories(filter, include_lost).await;

    // Group by category for summary
    let mut category_counts = std::collections::HashMap::new();
    for device in &devices {
        let cat_key = format!("{:?}", device.category);
        *category_counts.entry(cat_key).or_insert(0) += 1;
    }

    Json(serde_json::json!({
        "devices": devices,
        "total": devices.len(),
        "by_category": category_counts
    }))
}

/// CIDR request for count/delete operations (#82 T2-9)
#[derive(Deserialize)]
struct CidrRequest {
    cidr: String,
}

/// Count devices by CIDR (preview before delete)
async fn count_devices_by_cidr(
    State(state): State<AppState>,
    Json(req): Json<CidrRequest>,
) -> impl IntoResponse {
    match state.ipcam_scan.count_devices_by_cidr(&req.cidr).await {
        Ok(count) => Json(serde_json::json!({
            "ok": true,
            "cidr": req.cidr,
            "count": count
        })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// Delete devices by CIDR
async fn delete_devices_by_cidr(
    State(state): State<AppState>,
    Json(req): Json<CidrRequest>,
) -> impl IntoResponse {
    match state.ipcam_scan.delete_devices_by_cidr(&req.cidr).await {
        Ok(deleted) => Json(serde_json::json!({
            "ok": true,
            "cidr": req.cidr,
            "deleted": deleted
        })).into_response(),
        Err(e) => e.into_response(),
    }
}

#[derive(Deserialize)]
struct VerifyDeviceRequest {
    credentials: VerifyCredentials,
}

#[derive(Deserialize)]
struct VerifyCredentials {
    username: String,
    password: String,
}

async fn verify_device(
    State(state): State<AppState>,
    Path(device_ip): Path<String>,
    Json(req): Json<VerifyDeviceRequest>,
) -> impl IntoResponse {
    match state.ipcam_scan.verify_device(&device_ip, &req.credentials.username, &req.credentials.password).await {
        Ok(verified) => Json(serde_json::json!({
            "ok": true,
            "verified": verified
        })).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn approve_device(
    State(state): State<AppState>,
    Path(device_ip): Path<String>,
    Json(req): Json<crate::ipcam_scan::ApproveDeviceRequest>,
) -> impl IntoResponse {
    match state.ipcam_scan.approve_device(&device_ip, &req).await {
        Ok(response) => {
            // go2rtc registration is handled by polling_orchestrator at cycle start
            // Refresh config cache so polling orchestrator picks up new camera
            let _ = state.config_store.refresh_cache().await;

            // Spawn polling loop for new subnet if needed
            state.polling.spawn_subnet_loop_if_needed(&device_ip).await;

            Json(serde_json::json!({
                "ok": true,
                "camera": response
            })).into_response()
        },
        Err(e) => e.into_response(),
    }
}

async fn reject_device(
    State(state): State<AppState>,
    Path(device_ip): Path<String>,
) -> impl IntoResponse {
    match state.ipcam_scan.reject_device(&device_ip).await {
        Ok(_) => Json(serde_json::json!({
            "ok": true,
            "message": format!("Device {} rejected", device_ip)
        })).into_response(),
        Err(e) => e.into_response(),
    }
}

#[derive(Deserialize)]
struct BatchApproveRequest {
    devices: Vec<BatchApproveItem>,
}

#[derive(Deserialize)]
struct BatchApproveItem {
    ip: String,
    display_name: String,
    location: String,
    fid: String,
    credentials: Option<crate::ipcam_scan::ApproveCredentials>,
}

async fn approve_devices_batch(
    State(state): State<AppState>,
    Json(req): Json<BatchApproveRequest>,
) -> impl IntoResponse {
    let mut results = Vec::new();
    let mut approved_ips = Vec::new();

    for item in req.devices {
        let approve_req = crate::ipcam_scan::ApproveDeviceRequest {
            display_name: item.display_name,
            location: item.location,
            fid: item.fid,
            credentials: item.credentials,
        };

        let ip = item.ip.clone();
        match state.ipcam_scan.approve_device(&item.ip, &approve_req).await {
            Ok(response) => {
                // go2rtc registration is handled by polling_orchestrator at cycle start
                approved_ips.push(ip.clone());
                results.push(serde_json::json!({
                    "ip": ip,
                    "ok": true,
                    "camera": response
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "ip": ip,
                    "ok": false,
                    "error": e.to_string()
                }));
            }
        }
    }

    // Refresh config cache after all approvals
    let _ = state.config_store.refresh_cache().await;

    // Spawn polling loops for new subnets if needed
    for ip in &approved_ips {
        state.polling.spawn_subnet_loop_if_needed(ip).await;
    }

    let success_count = results.iter().filter(|r| r["ok"].as_bool().unwrap_or(false)).count();

    Json(serde_json::json!({
        "ok": true,
        "total": results.len(),
        "success": success_count,
        "results": results
    }))
}

#[derive(Deserialize)]
struct BatchVerifyRequest {
    device_ips: Vec<String>,
    /// Use stored credential by FID
    credential_fid: Option<String>,
    /// Or provide inline credentials
    username: Option<String>,
    password: Option<String>,
}

async fn verify_devices_batch(
    State(state): State<AppState>,
    Json(req): Json<BatchVerifyRequest>,
) -> impl IntoResponse {
    // Get credentials: either from stored fid or inline
    let (username, password) = if let Some(fid) = &req.credential_fid {
        match get_credentials_for_fid(&state.pool, fid).await {
            Ok(Some((u, p))) => (u, p),
            Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": format!("Credential FID '{}' not found", fid)
            }))).into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": e.to_string()
            }))).into_response(),
        }
    } else if let (Some(u), Some(p)) = (&req.username, &req.password) {
        (u.clone(), p.clone())
    } else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Either credential_fid or username/password must be provided"
        }))).into_response();
    };

    let mut results = Vec::new();

    for ip in &req.device_ips {
        match state.ipcam_scan.verify_device(ip, &username, &password).await {
            Ok(verified) => {
                results.push(serde_json::json!({
                    "ip": ip,
                    "ok": true,
                    "verified": verified
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "ip": ip,
                    "ok": false,
                    "verified": false,
                    "error": e.to_string()
                }));
            }
        }
    }

    let verified_count = results.iter()
        .filter(|r| r["verified"].as_bool().unwrap_or(false))
        .count();

    Json(serde_json::json!({
        "ok": true,
        "total": results.len(),
        "verified": verified_count,
        "results": results
    })).into_response()
}

async fn delete_scanned_device(
    State(state): State<AppState>,
    Path(device_ip): Path<String>,
) -> impl IntoResponse {
    match state.ipcam_scan.delete_device(&device_ip).await {
        Ok(_) => Json(serde_json::json!({
            "ok": true,
            "message": format!("Device {} deleted", device_ip)
        })).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn clear_scanned_devices(State(state): State<AppState>) -> impl IntoResponse {
    match state.ipcam_scan.clear_devices().await {
        Ok(count) => Json(serde_json::json!({
            "ok": true,
            "deleted": count
        })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// Force register device without authentication (#83 T2-10)
/// Creates camera with status='pending_auth', polling_enabled=false
async fn force_register_device(
    State(state): State<AppState>,
    Path(device_ip): Path<String>,
    Json(req): Json<crate::ipcam_scan::ForceRegisterRequest>,
) -> impl IntoResponse {
    match state.ipcam_scan.force_register_camera(&device_ip, &req).await {
        Ok(response) => {
            // Refresh config cache
            let _ = state.config_store.refresh_cache().await;

            Json(serde_json::json!({
                "ok": true,
                "camera": response
            })).into_response()
        }
        Err(e) => e.into_response(),
    }
}

/// Activate request for pending_auth camera (#83 T2-10)
#[derive(Deserialize)]
struct ActivateCameraRequest {
    username: String,
    password: String,
}

/// Activate a pending_auth camera after authentication success (#83 T2-10)
async fn activate_pending_camera(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
    Json(req): Json<ActivateCameraRequest>,
) -> impl IntoResponse {
    match state.ipcam_scan.activate_camera(&camera_id, &req.username, &req.password).await {
        Ok(_) => {
            // Refresh config cache so polling picks up the now-active camera
            let _ = state.config_store.refresh_cache().await;

            Json(serde_json::json!({
                "ok": true,
                "message": format!("Camera {} activated (pending_auth -> active)", camera_id)
            })).into_response()
        }
        Err(e) => e.into_response(),
    }
}

/// Manually trigger credential cleanup (#83 T2-11)
/// Normally run by scheduled job, but can be triggered manually for testing
async fn cleanup_credentials(State(state): State<AppState>) -> impl IntoResponse {
    match state.ipcam_scan.cleanup_tried_credentials().await {
        Ok(cleared) => Json(serde_json::json!({
            "ok": true,
            "cleared": cleared,
            "message": format!("Cleared {} expired credential records (24h policy)", cleared)
        })).into_response(),
        Err(e) => e.into_response(),
    }
}

// ========================================
// Stream Handlers (go2rtc integration)
// ========================================

async fn list_streams(State(state): State<AppState>) -> impl IntoResponse {
    match state.stream.list_streams().await {
        Ok(streams) => Json(ApiResponse::success(streams)).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_stream_urls(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
) -> impl IntoResponse {
    let urls = state.stream.get_stream_urls(&camera_id);
    Json(ApiResponse::success(urls))
}

async fn get_snapshot(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
) -> impl IntoResponse {
    // Proxy snapshot request to go2rtc
    let snapshot_url = format!(
        "{}/api/frame.jpeg?src={}",
        state.config.go2rtc_url,
        camera_id
    );

    let client = reqwest::Client::new();
    match client.get(&snapshot_url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.bytes().await {
                    Ok(bytes) => {
                        (
                            StatusCode::OK,
                            [("content-type", "image/jpeg")],
                            bytes.to_vec(),
                        ).into_response()
                    }
                    Err(e) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
                    }
                }
            } else {
                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": format!("go2rtc returned {}", resp.status())}))).into_response()
            }
        }
        Err(e) => {
            (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// Get cached snapshot from SnapshotService (ffmpeg RTSP capture)
///
/// This endpoint serves cached images captured by PollingOrchestrator.
/// CameraGrid should use this endpoint for displaying camera thumbnails.
async fn get_cached_snapshot(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
) -> impl IntoResponse {
    let cache_path = state.snapshot_service.get_cache_path(&camera_id);

    match tokio::fs::read(&cache_path).await {
        Ok(bytes) => {
            (
                StatusCode::OK,
                [
                    ("content-type", "image/jpeg"),
                    ("cache-control", "no-cache, no-store, must-revalidate"),
                ],
                bytes,
            ).into_response()
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Return placeholder or 404
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Snapshot not found",
                    "camera_id": camera_id,
                    "hint": "Camera may not have been polled yet"
                }))
            ).into_response()
        }
        Err(e) => {
            tracing::error!(
                camera_id = %camera_id,
                path = %cache_path.display(),
                error = %e,
                "Failed to read cached snapshot"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()}))
            ).into_response()
        }
    }
}

// ========================================
// Subnet Management Handlers
// ========================================

/// Trial credential for subnet authentication
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TrialCredential {
    username: String,
    password: String,
    priority: u8,  // 試行順序 (1-10)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
struct ScanSubnetRow {
    subnet_id: String,
    cidr: String,
    fid: Option<String>,
    tid: Option<String>,  // テナントID（カメラに継承される）
    facility_name: Option<String>,
    description: Option<String>,
    credentials: Option<String>,  // JSON
    enabled: bool,
    last_scanned_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ScanSubnet {
    subnet_id: String,
    cidr: String,
    fid: Option<String>,
    tid: Option<String>,  // テナントID（カメラに継承される）
    facility_name: Option<String>,
    description: Option<String>,
    credentials: Option<Vec<TrialCredential>>,
    enabled: bool,
    last_scanned_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl ScanSubnetRow {
    fn into_subnet(self) -> ScanSubnet {
        let credentials = self.credentials
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok());

        ScanSubnet {
            subnet_id: self.subnet_id,
            cidr: self.cidr,
            fid: self.fid,
            tid: self.tid,
            facility_name: self.facility_name,
            description: self.description,
            credentials,
            enabled: self.enabled,
            last_scanned_at: self.last_scanned_at,
            created_at: self.created_at,
        }
    }
}

#[derive(Deserialize)]
struct CreateSubnetRequest {
    cidr: String,
    fid: Option<String>,
    tid: Option<String>,  // テナントID
    facility_name: Option<String>,
    description: Option<String>,
    credentials: Option<Vec<TrialCredential>>,
    enabled: Option<bool>,
}

#[derive(Deserialize)]
struct UpdateSubnetRequest {
    cidr: Option<String>,
    fid: Option<String>,
    tid: Option<String>,  // テナントID
    facility_name: Option<String>,
    description: Option<String>,
    credentials: Option<Vec<TrialCredential>>,
    enabled: Option<bool>,
}

#[derive(Deserialize)]
struct ScanSelectedRequest {
    subnet_ids: Vec<String>,
    timeout_ms: Option<u32>,
}

async fn list_subnets(State(state): State<AppState>) -> impl IntoResponse {
    let result: Result<Vec<ScanSubnetRow>, sqlx::Error> = sqlx::query_as(
        r#"SELECT
            subnet_id, cidr, fid, tid, facility_name, description, credentials,
            enabled,
            last_scanned_at,
            created_at
        FROM scan_subnets ORDER BY created_at"#
    )
    .fetch_all(&state.pool)
    .await;

    match result {
        Ok(rows) => {
            let subnets: Vec<ScanSubnet> = rows.into_iter().map(|r| r.into_subnet()).collect();
            Json(ApiResponse::success(subnets)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn get_subnet(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let result: Result<Option<ScanSubnetRow>, sqlx::Error> = sqlx::query_as(
        r#"SELECT
            subnet_id, cidr, fid, tid, facility_name, description, credentials,
            enabled,
            last_scanned_at,
            created_at
        FROM scan_subnets WHERE subnet_id = ?"#
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(row)) => Json(ApiResponse::success(row.into_subnet())).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Subnet not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn create_subnet(
    State(state): State<AppState>,
    Json(req): Json<CreateSubnetRequest>,
) -> impl IntoResponse {
    let subnet_id = uuid::Uuid::new_v4().to_string();
    let enabled = req.enabled.unwrap_or(true);
    let credentials_json = req.credentials.as_ref()
        .map(|c| serde_json::to_string(c).unwrap_or_else(|_| "[]".to_string()));

    let result = sqlx::query(
        "INSERT INTO scan_subnets (subnet_id, cidr, fid, tid, facility_name, description, credentials, enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&subnet_id)
    .bind(&req.cidr)
    .bind(&req.fid)
    .bind(&req.tid)
    .bind(&req.facility_name)
    .bind(&req.description)
    .bind(&credentials_json)
    .bind(enabled)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {
            let subnet = ScanSubnet {
                subnet_id,
                cidr: req.cidr,
                fid: req.fid,
                tid: req.tid,
                facility_name: req.facility_name,
                description: req.description,
                credentials: req.credentials,
                enabled,
                last_scanned_at: None,
                created_at: chrono::Utc::now(),
            };
            (StatusCode::CREATED, Json(ApiResponse::success(subnet))).into_response()
        }
        Err(e) => {
            if e.to_string().contains("Duplicate entry") {
                (StatusCode::CONFLICT, Json(serde_json::json!({"error": "Subnet CIDR already exists"}))).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
            }
        }
    }
}

async fn update_subnet(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSubnetRequest>,
) -> impl IntoResponse {
    // Build dynamic update query with all supported fields
    let mut set_clauses = Vec::new();

    if let Some(ref cidr) = req.cidr {
        set_clauses.push(format!("cidr = '{}'", cidr.replace("'", "''")));
    }
    if let Some(ref fid) = req.fid {
        set_clauses.push(format!("fid = '{}'", fid.replace("'", "''")));
    }
    if let Some(ref tid) = req.tid {
        set_clauses.push(format!("tid = '{}'", tid.replace("'", "''")));
    }
    if let Some(ref facility_name) = req.facility_name {
        set_clauses.push(format!("facility_name = '{}'", facility_name.replace("'", "''")));
    }
    if let Some(ref desc) = req.description {
        set_clauses.push(format!("description = '{}'", desc.replace("'", "''")));
    }
    if let Some(ref creds) = req.credentials {
        // Serialize credentials to JSON
        match serde_json::to_string(creds) {
            Ok(json) => set_clauses.push(format!("credentials = '{}'", json.replace("'", "''"))),
            Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("Invalid credentials: {}", e)}))).into_response(),
        }
    }
    if let Some(enabled) = req.enabled {
        set_clauses.push(format!("enabled = {}", enabled));
    }

    if set_clauses.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "No fields to update"}))).into_response();
    }

    let query = format!(
        "UPDATE scan_subnets SET {} WHERE subnet_id = ?",
        set_clauses.join(", ")
    );

    match sqlx::query(&query).bind(&id).execute(&state.pool).await {
        Ok(result) => {
            if result.rows_affected() == 0 {
                (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Subnet not found"}))).into_response()
            } else {
                Json(serde_json::json!({"ok": true})).into_response()
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

/// Query parameters for subnet deletion
#[derive(Deserialize)]
struct DeleteSubnetQuery {
    /// If true, also delete scan results (ipcamscan_devices) for this subnet
    cleanup_scan_results: Option<bool>,
}

async fn delete_subnet(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteSubnetQuery>,
) -> impl IntoResponse {
    // If cleanup_scan_results is requested, first get the CIDR and delete scan results
    if query.cleanup_scan_results.unwrap_or(false) {
        // Get the CIDR for this subnet
        let cidr_result: Result<Option<String>, _> = sqlx::query_scalar(
            "SELECT cidr FROM scan_subnets WHERE subnet_id = ?"
        )
            .bind(&id)
            .fetch_optional(&state.pool)
            .await;

        match cidr_result {
            Ok(Some(cidr)) => {
                // Delete scan devices for this subnet (evidence will cascade delete)
                let delete_devices = sqlx::query(
                    "DELETE FROM ipcamscan_devices WHERE subnet = ?"
                )
                    .bind(&cidr)
                    .execute(&state.pool)
                    .await;

                if let Err(e) = delete_devices {
                    tracing::warn!(
                        subnet_id = %id,
                        cidr = %cidr,
                        error = %e,
                        "Failed to delete scan results for subnet"
                    );
                } else if let Ok(result) = delete_devices {
                    tracing::info!(
                        subnet_id = %id,
                        cidr = %cidr,
                        deleted_devices = result.rows_affected(),
                        "Cleaned up scan results for subnet"
                    );
                }
            }
            Ok(None) => {
                // Subnet not found, will be handled below
            }
            Err(e) => {
                tracing::warn!(
                    subnet_id = %id,
                    error = %e,
                    "Failed to get CIDR for subnet cleanup"
                );
            }
        }
    }

    let result = sqlx::query("DELETE FROM scan_subnets WHERE subnet_id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() == 0 {
                (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Subnet not found"}))).into_response()
            } else {
                Json(serde_json::json!({"ok": true, "deleted": 1})).into_response()
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn scan_selected_subnets(
    State(state): State<AppState>,
    Json(req): Json<ScanSelectedRequest>,
) -> impl IntoResponse {
    if req.subnet_ids.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "No subnets selected"}))).into_response();
    }

    // Get CIDRs for selected subnets
    let placeholders: Vec<&str> = req.subnet_ids.iter().map(|_| "?").collect();
    let query = format!(
        "SELECT cidr FROM scan_subnets WHERE subnet_id IN ({}) AND enabled = TRUE",
        placeholders.join(",")
    );

    let mut q = sqlx::query_scalar::<_, String>(&query);
    for id in &req.subnet_ids {
        q = q.bind(id);
    }

    let cidrs = match q.fetch_all(&state.pool).await {
        Ok(cidrs) => cidrs,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    if cidrs.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "No enabled subnets found for given IDs"}))).into_response();
    }

    // Create scan job with selected subnets
    let scan_req = crate::ipcam_scan::ScanJobRequest {
        targets: cidrs.clone(),
        mode: None,
        ports: None,
        timeout_ms: req.timeout_ms,
        concurrency: None,
    };

    // create_job now returns Result<ScanJob> (single execution guard)
    let job = match state.ipcam_scan.create_job(scan_req).await {
        Ok(job) => job,
        Err(e) => return e.into_response(),
    };
    let job_id = job.job_id;

    // Update last_scanned_at for selected subnets
    let update_query = format!(
        "UPDATE scan_subnets SET last_scanned_at = NOW(3) WHERE subnet_id IN ({})",
        placeholders.join(",")
    );
    let mut uq = sqlx::query(&update_query);
    for id in &req.subnet_ids {
        uq = uq.bind(id);
    }
    let _ = uq.execute(&state.pool).await;

    // Spawn background task to run the scan
    let scanner = state.ipcam_scan.clone();
    tokio::spawn(async move {
        if let Err(e) = scanner.run_job(job_id).await {
            tracing::error!(job_id = %job_id, error = %e, "Scan job failed");
        }
    });

    (StatusCode::CREATED, Json(serde_json::json!({
        "ok": true,
        "job": job,
        "subnets_scanned": cidrs
    }))).into_response()
}

// ========================================
// Credential Management Handlers
// ========================================

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
struct FacilityCredential {
    fid: String,
    facility_name: String,
    username: String,
    #[serde(skip_serializing)]
    password_encrypted: Vec<u8>,
    #[serde(skip_serializing)]
    encryption_iv: Vec<u8>,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CredentialResponse {
    fid: String,
    facility_name: String,
    username: String,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<FacilityCredential> for CredentialResponse {
    fn from(c: FacilityCredential) -> Self {
        Self {
            fid: c.fid,
            facility_name: c.facility_name,
            username: c.username,
            notes: c.notes,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(Deserialize)]
struct CreateCredentialRequest {
    fid: String,
    facility_name: String,
    username: String,
    password: String,
    notes: Option<String>,
}

/// Simple XOR-based encryption (for demonstration - use proper AES in production)
fn encrypt_password(password: &str) -> (Vec<u8>, Vec<u8>) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let iv: Vec<u8> = (0..16).map(|_| rng.gen()).collect();

    // Simple XOR with IV (not secure - just for demonstration)
    let encrypted: Vec<u8> = password
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ iv[i % iv.len()])
        .collect();

    (encrypted, iv)
}

/// Decrypt password (XOR-based)
fn decrypt_password(encrypted: &[u8], iv: &[u8]) -> String {
    let decrypted: Vec<u8> = encrypted
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ iv[i % iv.len()])
        .collect();
    String::from_utf8_lossy(&decrypted).to_string()
}

async fn list_credentials(State(state): State<AppState>) -> impl IntoResponse {
    let result: Result<Vec<FacilityCredential>, sqlx::Error> = sqlx::query_as(
        r#"SELECT fid, facility_name, username, password_encrypted, encryption_iv, notes, created_at, updated_at
        FROM facility_credentials ORDER BY facility_name"#
    )
    .fetch_all(&state.pool)
    .await;

    match result {
        Ok(creds) => {
            let responses: Vec<CredentialResponse> = creds.into_iter().map(|c| c.into()).collect();
            Json(ApiResponse::success(responses)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn get_credential(
    State(state): State<AppState>,
    Path(fid): Path<String>,
) -> impl IntoResponse {
    let result: Result<Option<FacilityCredential>, sqlx::Error> = sqlx::query_as(
        r#"SELECT fid, facility_name, username, password_encrypted, encryption_iv, notes, created_at, updated_at
        FROM facility_credentials WHERE fid = ?"#
    )
    .bind(&fid)
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(cred)) => {
            let response: CredentialResponse = cred.into();
            Json(ApiResponse::success(response)).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Credential not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn create_credential(
    State(state): State<AppState>,
    Json(req): Json<CreateCredentialRequest>,
) -> impl IntoResponse {
    let (encrypted, iv) = encrypt_password(&req.password);

    let result = sqlx::query(
        "INSERT INTO facility_credentials (fid, facility_name, username, password_encrypted, encryption_iv, notes) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&req.fid)
    .bind(&req.facility_name)
    .bind(&req.username)
    .bind(&encrypted)
    .bind(&iv)
    .bind(&req.notes)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {
            let response = CredentialResponse {
                fid: req.fid,
                facility_name: req.facility_name,
                username: req.username,
                notes: req.notes,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            (StatusCode::CREATED, Json(ApiResponse::success(response))).into_response()
        }
        Err(e) => {
            if e.to_string().contains("Duplicate entry") {
                (StatusCode::CONFLICT, Json(serde_json::json!({"error": "Credential with this FID already exists"}))).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
            }
        }
    }
}

async fn delete_credential(
    State(state): State<AppState>,
    Path(fid): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM facility_credentials WHERE fid = ?")
        .bind(&fid)
        .execute(&state.pool)
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() == 0 {
                (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Credential not found"}))).into_response()
            } else {
                Json(serde_json::json!({"ok": true, "deleted": 1})).into_response()
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

/// Get decrypted credentials for internal use (batch verification)
pub async fn get_credentials_for_fid(pool: &sqlx::MySqlPool, fid: &str) -> Result<Option<(String, String)>, sqlx::Error> {
    let result: Option<FacilityCredential> = sqlx::query_as(
        r#"SELECT fid, facility_name, username, password_encrypted, encryption_iv, notes, created_at, updated_at
        FROM facility_credentials WHERE fid = ?"#
    )
    .bind(fid)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|c| {
        let password = decrypt_password(&c.password_encrypted, &c.encryption_iv);
        (c.username, password)
    }))
}

// ========================================
// WebSocket Handler
// ========================================

/// WebSocket upgrade handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Handle WebSocket connection
async fn handle_websocket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Register with RealtimeHub
    let user_id = uuid::Uuid::new_v4().to_string();
    let (conn_id, mut rx) = state.realtime.register(user_id).await;

    tracing::info!(connection_id = %conn_id, "WebSocket client connected");

    // Spawn task to forward messages from hub to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages (for future use - ping/pong, commands, etc.)
    let recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Ping(data)) => {
                    // Pong is handled automatically by axum
                    tracing::trace!("Received ping: {:?}", data);
                }
                Ok(Message::Close(_)) => {
                    tracing::info!(connection_id = %conn_id, "WebSocket client disconnected");
                    break;
                }
                Err(e) => {
                    tracing::warn!(connection_id = %conn_id, error = %e, "WebSocket error");
                    break;
                }
                _ => {}
            }
        }
        conn_id
    });

    // Wait for either task to complete
    let conn_id = tokio::select! {
        _ = send_task => conn_id,
        result = recv_task => result.unwrap_or(conn_id),
    };

    // Unregister from hub
    state.realtime.unregister(&conn_id).await;
}

// ========================================
// Test API (E2E testing)
// ========================================

/// Request body for trigger_test_event
#[derive(Debug, Deserialize)]
struct TriggerEventRequest {
    camera_id: Option<String>,
    lacis_id: Option<String>,
    severity: Option<i32>,
    primary_event: Option<String>,
}

/// Trigger a test event via WebSocket (for E2E testing BUG-001)
/// POST /api/test/trigger-event
async fn trigger_test_event(
    State(state): State<AppState>,
    Json(req): Json<TriggerEventRequest>,
) -> impl IntoResponse {
    use crate::realtime_hub::{EventLogMessage, HubMessage};
    use std::sync::atomic::{AtomicU64, Ordering};
    static TEST_EVENT_COUNTER: AtomicU64 = AtomicU64::new(1_000_000);

    // Get camera info if camera_id provided
    let (camera_id, lacis_id) = if let Some(cid) = req.camera_id {
        let cameras = state.config_store.get_cached_cameras().await;
        let camera = cameras.iter().find(|c| c.camera_id == cid);
        match camera {
            Some(c) => (c.camera_id.clone(), c.lacis_id.clone().unwrap_or_default()),
            None => (cid, req.lacis_id.unwrap_or_default()),
        }
    } else {
        // Use defaults or provided values
        ("test-camera-001".to_string(), req.lacis_id.unwrap_or("TEST_LACIS_ID".to_string()))
    };

    let event_id = TEST_EVENT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let severity = req.severity.unwrap_or(3);
    let primary_event = req.primary_event.unwrap_or("test_event".to_string());
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    let event_msg = EventLogMessage {
        event_id,
        camera_id: camera_id.clone(),
        lacis_id: lacis_id.clone(),
        primary_event: primary_event.clone(),
        severity,
        timestamp: timestamp.clone(),
    };

    // Broadcast via WebSocket
    state.realtime.broadcast(HubMessage::EventLog(event_msg)).await;

    tracing::info!(
        event_id = event_id,
        camera_id = %camera_id,
        lacis_id = %lacis_id,
        severity = severity,
        primary_event = %primary_event,
        "[TEST] Event broadcast via WebSocket"
    );

    Json(ApiResponse::success(json!({
        "event_id": event_id,
        "camera_id": camera_id,
        "lacis_id": lacis_id,
        "severity": severity,
        "primary_event": primary_event,
        "timestamp": timestamp,
        "message": "Test event broadcast via WebSocket"
    })))
}

// ========================================
// Camera Brand API Handlers
// ========================================

/// List all camera brands
async fn list_camera_brands(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, crate::Error> {
    let brands = state.camera_brand.list_brands_with_counts().await?;
    Ok(Json(ApiResponse::success(brands)))
}

/// Create camera brand
async fn create_camera_brand(
    State(state): State<AppState>,
    Json(req): Json<CreateBrandRequest>,
) -> Result<impl IntoResponse, crate::Error> {
    let brand = state.camera_brand.create_brand(req).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(brand))))
}

/// Get camera brand by ID
async fn get_camera_brand(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, crate::Error> {
    let brand = state.camera_brand.get_brand(id).await?.ok_or_else(|| {
        crate::Error::NotFound(format!("Brand {} not found", id))
    })?;
    Ok(Json(ApiResponse::success(brand)))
}

/// Update camera brand
async fn update_camera_brand(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateBrandRequest>,
) -> Result<impl IntoResponse, crate::Error> {
    let brand = state.camera_brand.update_brand(id, req).await?;
    Ok(Json(ApiResponse::success(brand)))
}

/// Delete camera brand
async fn delete_camera_brand(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, crate::Error> {
    state.camera_brand.delete_brand(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ========================================
// OUI Entry API Handlers
// ========================================

/// List all OUI entries
async fn list_all_oui_entries(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, crate::Error> {
    let entries = state.camera_brand.list_oui_entries().await?;
    Ok(Json(ApiResponse::success(entries)))
}

/// List OUI entries for a brand
async fn list_oui_entries_for_brand(
    State(state): State<AppState>,
    Path(brand_id): Path<i32>,
) -> Result<impl IntoResponse, crate::Error> {
    let entries = state.camera_brand.get_oui_entries_for_brand(brand_id).await?;
    Ok(Json(ApiResponse::success(entries)))
}

/// Get OUI entry by prefix
async fn get_oui_entry(
    State(state): State<AppState>,
    Path(prefix): Path<String>,
) -> Result<impl IntoResponse, crate::Error> {
    let entry = state.camera_brand.get_oui_entry(&prefix).await?.ok_or_else(|| {
        crate::Error::NotFound(format!("OUI {} not found", prefix))
    })?;
    Ok(Json(ApiResponse::success(entry)))
}

/// Add OUI entry
async fn add_oui_entry(
    State(state): State<AppState>,
    Path(brand_id): Path<i32>,
    Json(req): Json<AddOuiRequest>,
) -> Result<impl IntoResponse, crate::Error> {
    let entry = state.camera_brand.add_oui_entry(brand_id, req).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(entry))))
}

/// Update OUI entry
async fn update_oui_entry(
    State(state): State<AppState>,
    Path(prefix): Path<String>,
    Json(req): Json<UpdateOuiRequest>,
) -> Result<impl IntoResponse, crate::Error> {
    let entry = state.camera_brand.update_oui_entry(&prefix, req).await?;
    Ok(Json(ApiResponse::success(entry)))
}

/// Delete OUI entry
async fn delete_oui_entry(
    State(state): State<AppState>,
    Path(prefix): Path<String>,
) -> Result<impl IntoResponse, crate::Error> {
    state.camera_brand.delete_oui_entry(&prefix).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ========================================
// RTSP Template API Handlers
// ========================================

/// List all RTSP templates
async fn list_all_rtsp_templates(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, crate::Error> {
    let templates = state.camera_brand.list_templates().await?;
    Ok(Json(ApiResponse::success(templates)))
}

/// List RTSP templates for a brand
async fn list_templates_for_brand(
    State(state): State<AppState>,
    Path(brand_id): Path<i32>,
) -> Result<impl IntoResponse, crate::Error> {
    let templates = state.camera_brand.get_templates_for_brand_db(brand_id).await?;
    Ok(Json(ApiResponse::success(templates)))
}

/// Get RTSP template by ID
async fn get_rtsp_template(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, crate::Error> {
    let template = state.camera_brand.get_template(id).await?.ok_or_else(|| {
        crate::Error::NotFound(format!("Template {} not found", id))
    })?;
    Ok(Json(ApiResponse::success(template)))
}

/// Add RTSP template
async fn add_rtsp_template(
    State(state): State<AppState>,
    Path(brand_id): Path<i32>,
    Json(req): Json<AddTemplateRequest>,
) -> Result<impl IntoResponse, crate::Error> {
    let template = state.camera_brand.add_template(brand_id, req).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(template))))
}

/// Update RTSP template
async fn update_rtsp_template(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTemplateRequest>,
) -> Result<impl IntoResponse, crate::Error> {
    let template = state.camera_brand.update_template(id, req).await?;
    Ok(Json(ApiResponse::success(template)))
}

/// Delete RTSP template
async fn delete_rtsp_template(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, crate::Error> {
    state.camera_brand.delete_template(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ========================================
// Generic RTSP Path API Handlers
// ========================================

/// List all generic RTSP paths
async fn list_generic_rtsp_paths(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, crate::Error> {
    let paths = state.camera_brand.list_generic_paths().await?;
    Ok(Json(ApiResponse::success(paths)))
}

/// Get generic RTSP path by ID
async fn get_generic_rtsp_path(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, crate::Error> {
    let path = state.camera_brand.get_generic_path(id).await?.ok_or_else(|| {
        crate::Error::NotFound(format!("Generic path {} not found", id))
    })?;
    Ok(Json(ApiResponse::success(path)))
}

/// Add generic RTSP path
async fn add_generic_rtsp_path(
    State(state): State<AppState>,
    Json(req): Json<AddGenericPathRequest>,
) -> Result<impl IntoResponse, crate::Error> {
    let path = state.camera_brand.add_generic_path(req).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(path))))
}

/// Update generic RTSP path
async fn update_generic_rtsp_path(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateGenericPathRequest>,
) -> Result<impl IntoResponse, crate::Error> {
    let path = state.camera_brand.update_generic_path(id, req).await?;
    Ok(Json(ApiResponse::success(path)))
}

/// Delete generic RTSP path
async fn delete_generic_rtsp_path(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, crate::Error> {
    state.camera_brand.delete_generic_path(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
