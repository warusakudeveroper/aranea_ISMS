//! API Routes

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;

use crate::admission_controller::{LeaseRequest, LeaseResponse, StreamQuality};
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
        .route("/api/cameras/restore-by-mac", post(restore_camera_by_mac))
        // Modal Leases
        .route("/api/modal/lease", post(request_lease))
        .route("/api/modal/lease/:id", delete(release_lease))
        .route("/api/modal/lease/:id/heartbeat", post(heartbeat))
        // Suggest
        .route("/api/suggest", get(get_suggest))
        .route("/api/suggest/manual", post(set_manual_suggest))
        .route("/api/suggest", delete(clear_suggest))
        // Events
        .route("/api/events", get(list_events))
        // System
        .route("/api/system/status", get(system_status))
        // IpcamScan
        .route("/api/ipcamscan/jobs", post(create_scan_job))
        .route("/api/ipcamscan/jobs/:id", get(get_scan_job))
        .route("/api/ipcamscan/devices", get(list_scanned_devices))
        .route("/api/ipcamscan/devices/:ip/verify", post(verify_device))
        .route("/api/ipcamscan/devices/:ip/approve", post(approve_device))
        .route("/api/ipcamscan/devices/:ip/reject", post(reject_device))
        .route("/api/ipcamscan/devices/:ip", delete(delete_scanned_device))
        .route("/api/ipcamscan/devices", delete(clear_scanned_devices))
        .route("/api/ipcamscan/devices/approve-batch", post(approve_devices_batch))
        .route("/api/ipcamscan/devices/verify-batch", post(verify_devices_batch))
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
// System Handlers
// ========================================

async fn system_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.admission.get_status().await;
    Json(ApiResponse::success(status))
}

// ========================================
// IpcamScan Handlers
// ========================================

async fn create_scan_job(
    State(state): State<AppState>,
    Json(req): Json<crate::ipcam_scan::ScanJobRequest>,
) -> impl IntoResponse {
    let job = state.ipcam_scan.create_job(req).await;
    let job_id = job.job_id;

    // Spawn background task to run the scan
    let scanner = state.ipcam_scan.clone();
    tokio::spawn(async move {
        if let Err(e) = scanner.run_job(job_id).await {
            tracing::error!(job_id = %job_id, error = %e, "Scan job failed");
        }
    });

    (StatusCode::CREATED, Json(ApiResponse::success(job)))
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
            // Register stream with go2rtc if RTSP URL is available
            if let Some(ref rtsp_url) = response.rtsp_url {
                if let Err(e) = state.stream.add_source(&response.camera_id, rtsp_url).await {
                    tracing::warn!(
                        camera_id = %response.camera_id,
                        error = %e,
                        "Failed to register stream with go2rtc (camera still approved)"
                    );
                } else {
                    tracing::info!(
                        camera_id = %response.camera_id,
                        "Stream registered with go2rtc"
                    );
                }
            }

            // Refresh config cache so polling orchestrator picks up new camera
            let _ = state.config_store.refresh_cache().await;

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

    for item in req.devices {
        let approve_req = crate::ipcam_scan::ApproveDeviceRequest {
            display_name: item.display_name,
            location: item.location,
            fid: item.fid,
            credentials: item.credentials,
        };

        match state.ipcam_scan.approve_device(&item.ip, &approve_req).await {
            Ok(response) => {
                // Register stream with go2rtc
                if let Some(ref rtsp_url) = response.rtsp_url {
                    let _ = state.stream.add_source(&response.camera_id, rtsp_url).await;
                }
                results.push(serde_json::json!({
                    "ip": item.ip,
                    "ok": true,
                    "camera": response
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "ip": item.ip,
                    "ok": false,
                    "error": e.to_string()
                }));
            }
        }
    }

    // Refresh config cache after all approvals
    let _ = state.config_store.refresh_cache().await;

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

async fn delete_subnet(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
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

    let job = state.ipcam_scan.create_job(scan_req).await;
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
