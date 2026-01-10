//! AraneaRegister API Routes
//!
//! Phase 1: AraneaRegister (Issue #114)
//! DD01_AraneaRegister.md準拠
//!
//! ## エンドポイント
//! - POST /api/register/device - デバイス登録実行
//! - GET /api/register/status - 登録状態取得
//! - DELETE /api/register - 登録情報クリア

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;

use crate::aranea_register::{ClearResult, RegisterRequest, RegisterResult, RegistrationStatus};
use crate::state::AppState;

/// Create register routes
pub fn register_routes() -> Router<AppState> {
    Router::new()
        .route("/device", post(register_device))
        .route("/status", get(get_registration_status))
        .route("/", delete(clear_registration))
}

/// POST /api/register/device
///
/// デバイス登録を実行
///
/// ## Request Body
/// ```json
/// {
///   "tenantPrimaryAuth": {
///     "lacisId": "18217487937895888001",
///     "userId": "soejim@mijeos.com",
///     "cic": "204965"
///   },
///   "tid": "T2025120621041161827"
/// }
/// ```
///
/// ## Response (201 Created)
/// ```json
/// {
///   "ok": true,
///   "lacisId": "3022AABBCCDDEEFF0000",
///   "cic": "123456",
///   "stateEndpoint": "https://..."
/// }
/// ```
///
/// ## Response (409 Conflict)
/// ```json
/// {
///   "ok": false,
///   "error": "Device already registered"
/// }
/// ```
async fn register_device(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    tracing::info!(
        tid = %req.tid,
        "AraneaRegister API: Register device requested"
    );

    // aranea_register_serviceが存在するか確認
    let service = match &state.aranea_register {
        Some(s) => s,
        None => {
            tracing::error!("AraneaRegister API: Service not initialized");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "ok": false,
                    "error": "AraneaRegister service not initialized"
                })),
            )
                .into_response();
        }
    };

    match service.register_device(req).await {
        Ok(result) => {
            let status = if result.ok {
                StatusCode::CREATED
            } else {
                StatusCode::BAD_REQUEST
            };
            (status, Json(result)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "AraneaRegister API: Registration failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RegisterResult {
                    ok: false,
                    lacis_id: None,
                    cic: None,
                    state_endpoint: None,
                    mqtt_endpoint: None,
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}

/// GET /api/register/status
///
/// 登録状態を取得
///
/// ## Response
/// ```json
/// {
///   "registered": true,
///   "lacisId": "3022AABBCCDDEEFF0000",
///   "tid": "T2025120621041161827",
///   "cic": "123456",
///   "registeredAt": "2026-01-10T10:00:00.000Z",
///   "lastSyncAt": "2026-01-10T12:00:00.000Z"
/// }
/// ```
async fn get_registration_status(State(state): State<AppState>) -> impl IntoResponse {
    let service = match &state.aranea_register {
        Some(s) => s,
        None => {
            // サービス未初期化の場合は未登録として返す
            return Json(RegistrationStatus::default()).into_response();
        }
    };

    match service.get_registration_status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "AraneaRegister API: Failed to get status");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// DELETE /api/register
///
/// 登録情報をクリア（再登録用）
///
/// ## Response (200 OK)
/// ```json
/// {
///   "ok": true,
///   "message": "Registration cleared"
/// }
/// ```
async fn clear_registration(State(state): State<AppState>) -> impl IntoResponse {
    tracing::warn!("AraneaRegister API: Clear registration requested");

    let service = match &state.aranea_register {
        Some(s) => s,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "ok": false,
                    "message": "AraneaRegister service not initialized"
                })),
            )
                .into_response();
        }
    };

    match service.clear_registration().await {
        Ok(result) => Json(result).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "AraneaRegister API: Failed to clear registration");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ClearResult {
                    ok: false,
                    message: e.to_string(),
                }),
            )
                .into_response()
        }
    }
}
