//! PTZ API Routes
//!
//! PTZ操作のHTTP APIエンドポイント

use crate::ptz_controller::{PtzHomeRequest, PtzMoveRequest, PtzResponse, PtzStopRequest};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

/// POST /api/cameras/:id/ptz/move
/// PTZ移動開始
pub async fn ptz_move(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
    Json(request): Json<PtzMoveRequest>,
) -> Result<Json<PtzResponse>, (StatusCode, Json<PtzResponse>)> {
    // Lease認証
    let lease_uuid = match Uuid::parse_str(&request.lease_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!(lease_id = %request.lease_id, "PTZ move rejected: invalid lease format");
            return Err((StatusCode::BAD_REQUEST, Json(PtzResponse::error("Invalid lease format"))));
        }
    };
    let lease = state.admission.get_lease_by_id(&lease_uuid).await;
    if lease.is_none() {
        tracing::warn!(lease_id = %request.lease_id, "PTZ move rejected: invalid lease");
        return Err((StatusCode::UNAUTHORIZED, Json(PtzResponse::error("Invalid lease - モーダルを再度開いてください"))));
    }

    match state.ptz_service.move_ptz(&camera_id, &request).await {
        Ok(response) => {
            if response.ok {
                Ok(Json(response))
            } else {
                Err((StatusCode::BAD_REQUEST, Json(response)))
            }
        }
        Err(e) => {
            tracing::error!("PTZ move error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PtzResponse::error(format!("Internal error: {}", e))),
            ))
        }
    }
}

/// POST /api/cameras/:id/ptz/stop
/// PTZ停止
pub async fn ptz_stop(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
    Json(request): Json<PtzStopRequest>,
) -> Result<Json<PtzResponse>, (StatusCode, Json<PtzResponse>)> {
    // Lease認証
    let lease_uuid = match Uuid::parse_str(&request.lease_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!(lease_id = %request.lease_id, "PTZ stop rejected: invalid lease format");
            return Err((StatusCode::BAD_REQUEST, Json(PtzResponse::error("Invalid lease format"))));
        }
    };
    let lease = state.admission.get_lease_by_id(&lease_uuid).await;
    if lease.is_none() {
        tracing::warn!(lease_id = %request.lease_id, "PTZ stop rejected: invalid lease");
        return Err((StatusCode::UNAUTHORIZED, Json(PtzResponse::error("Invalid lease"))));
    }

    match state.ptz_service.stop_ptz(&camera_id).await {
        Ok(response) => {
            if response.ok {
                Ok(Json(response))
            } else {
                Err((StatusCode::BAD_REQUEST, Json(response)))
            }
        }
        Err(e) => {
            tracing::error!("PTZ stop error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PtzResponse::error(format!("Internal error: {}", e))),
            ))
        }
    }
}

/// POST /api/cameras/:id/ptz/home
/// ホームポジション移動
pub async fn ptz_home(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
    Json(request): Json<PtzHomeRequest>,
) -> Result<Json<PtzResponse>, (StatusCode, Json<PtzResponse>)> {
    // Lease認証
    let lease_uuid = match Uuid::parse_str(&request.lease_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!(lease_id = %request.lease_id, "PTZ home rejected: invalid lease format");
            return Err((StatusCode::BAD_REQUEST, Json(PtzResponse::error("Invalid lease format"))));
        }
    };
    let lease = state.admission.get_lease_by_id(&lease_uuid).await;
    if lease.is_none() {
        tracing::warn!(lease_id = %request.lease_id, "PTZ home rejected: invalid lease");
        return Err((StatusCode::UNAUTHORIZED, Json(PtzResponse::error("Invalid lease"))));
    }

    match state.ptz_service.go_home(&camera_id).await {
        Ok(response) => {
            if response.ok {
                Ok(Json(response))
            } else {
                Err((StatusCode::BAD_REQUEST, Json(response)))
            }
        }
        Err(e) => {
            tracing::error!("PTZ home error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PtzResponse::error(format!("Internal error: {}", e))),
            ))
        }
    }
}

/// GET /api/cameras/:id/ptz/status
/// PTZステータス取得
pub async fn ptz_status(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
) -> Result<Json<crate::ptz_controller::PtzStatus>, (StatusCode, String)> {
    match state.ptz_service.get_status(&camera_id).await {
        Ok(status) => Ok(Json(status)),
        Err(e) => {
            tracing::error!("PTZ status error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}
