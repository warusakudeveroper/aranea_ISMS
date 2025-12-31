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
    Json(status)
}

// ========================================
// IpcamScan Handlers
// ========================================

async fn create_scan_job(
    State(_state): State<AppState>,
    Json(_req): Json<crate::ipcam_scan::ScanJobRequest>,
) -> impl IntoResponse {
    // TODO: Implement
    Json(serde_json::json!({"error": "Not implemented"}))
}

async fn get_scan_job(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // TODO: Implement
    Json(serde_json::json!({"error": "Not implemented"}))
}

async fn list_scanned_devices(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    // TODO: Implement
    Json(serde_json::json!({"devices": [], "total": 0}))
}
