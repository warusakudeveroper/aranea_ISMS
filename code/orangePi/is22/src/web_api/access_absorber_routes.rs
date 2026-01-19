//! Access Absorber API Routes
//!
//! Provides REST API endpoints for camera stream connection management.
//! See: camBrand/accessAbsorber/SPECIFICATION.md

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::access_absorber::{
    AcquireStreamRequest, AcquireStreamResponse, ReleaseStreamRequest, StreamPurpose,
};
use crate::realtime_hub::{HubMessage, PreemptionUserMessage, StreamPreemptedMessage};
use crate::state::AppState;

/// Create Access Absorber API router
pub fn access_absorber_routes() -> Router<AppState> {
    Router::new()
        // Stream management
        .route("/streams/acquire", post(acquire_stream))
        .route("/streams/release", post(release_stream))
        .route("/streams/:session_id/heartbeat", post(heartbeat))
        // Camera state
        .route("/cameras/:camera_id/status", get(get_camera_status))
        .route("/cameras/:camera_id/state", get(get_connection_state))
        // Limits management
        .route("/limits", get(list_all_limits))
        .route("/limits/:family", get(get_limits_by_family))
        // Statistics
        .route("/stats/:camera_id", get(get_stats))
        // Maintenance
        .route("/cleanup", post(cleanup_expired))
}

/// Acquire a stream for a camera
async fn acquire_stream(
    State(state): State<AppState>,
    Json(req): Json<AcquireStreamRequest>,
) -> impl IntoResponse {
    let service = match &state.access_absorber {
        Some(s) => s,
        None => {
            return Json(AcquireStreamResponse {
                success: false,
                token: None,
                error: Some(crate::access_absorber::AbsorberError::Internal {
                    message: "Access Absorber not initialized".to_string(),
                }),
                preempted_session: None,
            })
        }
    };

    match service
        .acquire_stream(
            &req.camera_id,
            req.purpose,
            &req.client_id,
            req.stream_type,
            req.allow_preempt,
        )
        .await
    {
        Ok(result) => {
            let preempted_session = result.preemption.as_ref().map(|p| p.session_id.clone());

            // If preemption occurred, broadcast notification via WebSocket
            if let Some(preemption) = &result.preemption {
                // Generate user-friendly message based on purpose
                let (title, message, severity) = match preemption.preempted_purpose {
                    StreamPurpose::SuggestPlay => (
                        "ストリーム優先制御".to_string(),
                        format!(
                            "モーダル表示が優先されたため、サジェスト再生を終了します（{}秒後）",
                            preemption.exit_delay_sec
                        ),
                        "info".to_string(),
                    ),
                    StreamPurpose::HealthCheck => (
                        "ヘルスチェック中断".to_string(),
                        "より優先度の高い操作のため、ヘルスチェックを中断しました".to_string(),
                        "info".to_string(),
                    ),
                    _ => (
                        "ストリーム優先制御".to_string(),
                        "より優先度の高い操作のため、現在のストリームを終了します".to_string(),
                        "warning".to_string(),
                    ),
                };

                let preempt_msg = StreamPreemptedMessage {
                    camera_id: preemption.camera_id.clone(),
                    session_id: preemption.session_id.clone(),
                    preempted_purpose: format!("{:?}", preemption.preempted_purpose).to_lowercase(),
                    preempted_by: format!("{:?}", preemption.preempted_by_purpose).to_lowercase(),
                    user_message: PreemptionUserMessage {
                        title,
                        message,
                        severity,
                    },
                    exit_delay_sec: preemption.exit_delay_sec,
                };

                // Broadcast to all connected clients
                state
                    .realtime
                    .broadcast(HubMessage::StreamPreempted(preempt_msg))
                    .await;
            }

            Json(AcquireStreamResponse {
                success: true,
                token: Some(result.token),
                error: None,
                preempted_session,
            })
        }
        Err(error) => Json(AcquireStreamResponse {
            success: false,
            token: None,
            error: Some(error),
            preempted_session: None,
        }),
    }
}

/// Release a stream
async fn release_stream(
    State(state): State<AppState>,
    Json(req): Json<ReleaseStreamRequest>,
) -> impl IntoResponse {
    let service = match &state.access_absorber {
        Some(s) => s,
        None => {
            return Json(json!({
                "ok": false,
                "error": "Access Absorber not initialized"
            }))
        }
    };

    match service.release_stream(&req.session_id).await {
        Ok(_) => Json(json!({
            "ok": true,
            "data": {
                "released": true,
                "session_id": req.session_id
            }
        })),
        Err(e) => Json(json!({
            "ok": false,
            "error": e.to_string()
        })),
    }
}

/// Session heartbeat
async fn heartbeat(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let service = match &state.access_absorber {
        Some(s) => s,
        None => {
            return Json(json!({
                "ok": false,
                "error": "Access Absorber not initialized"
            }))
        }
    };

    match service.heartbeat(&session_id).await {
        Ok(found) => {
            if found {
                Json(json!({
                    "ok": true,
                    "data": {
                        "session_id": session_id,
                        "status": "active"
                    }
                }))
            } else {
                Json(json!({
                    "ok": false,
                    "error": "Session not found or expired"
                }))
            }
        }
        Err(e) => Json(json!({
            "ok": false,
            "error": e.to_string()
        })),
    }
}

/// Get stream status for a camera
async fn get_camera_status(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
) -> impl IntoResponse {
    let service = match &state.access_absorber {
        Some(s) => s,
        None => {
            return Json(json!({
                "ok": false,
                "error": "Access Absorber not initialized"
            }))
        }
    };

    match service.get_stream_status(&camera_id).await {
        Ok(status) => Json(json!({
            "ok": true,
            "data": status
        })),
        Err(e) => Json(json!({
            "ok": false,
            "error": e.to_string()
        })),
    }
}

/// Get connection state for a camera
async fn get_connection_state(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
) -> impl IntoResponse {
    let service = match &state.access_absorber {
        Some(s) => s,
        None => {
            return Json(json!({
                "ok": false,
                "error": "Access Absorber not initialized"
            }))
        }
    };

    match service.get_connection_state(&camera_id).await {
        Ok(conn_state) => Json(json!({
            "ok": true,
            "data": conn_state
        })),
        Err(e) => Json(json!({
            "ok": false,
            "error": e.to_string()
        })),
    }
}

/// List all connection limits
async fn list_all_limits(State(state): State<AppState>) -> impl IntoResponse {
    let service = match &state.access_absorber {
        Some(s) => s,
        None => {
            return Json(json!({
                "ok": false,
                "error": "Access Absorber not initialized"
            }))
        }
    };

    match service.get_all_limits().await {
        Ok(limits) => Json(json!({
            "ok": true,
            "data": limits
        })),
        Err(e) => Json(json!({
            "ok": false,
            "error": e.to_string()
        })),
    }
}

/// Get limits for a specific family
async fn get_limits_by_family(
    State(state): State<AppState>,
    Path(family): Path<String>,
) -> impl IntoResponse {
    let service = match &state.access_absorber {
        Some(s) => s,
        None => {
            return Json(json!({
                "ok": false,
                "error": "Access Absorber not initialized"
            }))
        }
    };

    // Use get_all_limits and filter
    match service.get_all_limits().await {
        Ok(limits) => {
            if let Some(limit) = limits.into_iter().find(|l| l.family == family) {
                Json(json!({
                    "ok": true,
                    "data": limit
                }))
            } else {
                Json(json!({
                    "ok": false,
                    "error": format!("Family '{}' not found", family)
                }))
            }
        }
        Err(e) => Json(json!({
            "ok": false,
            "error": e.to_string()
        })),
    }
}

#[derive(Debug, Deserialize)]
struct StatsQuery {
    #[serde(default = "default_hours")]
    hours: i64,
}

fn default_hours() -> i64 {
    24
}

/// Get connection statistics for a camera
async fn get_stats(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
    Query(query): Query<StatsQuery>,
) -> impl IntoResponse {
    let service = match &state.access_absorber {
        Some(s) => s,
        None => {
            return Json(json!({
                "ok": false,
                "error": "Access Absorber not initialized"
            }))
        }
    };

    match service.get_stats(&camera_id, query.hours).await {
        Ok(stats) => Json(json!({
            "ok": true,
            "data": stats
        })),
        Err(e) => Json(json!({
            "ok": false,
            "error": e.to_string()
        })),
    }
}

/// Cleanup expired sessions
async fn cleanup_expired(State(state): State<AppState>) -> impl IntoResponse {
    let service = match &state.access_absorber {
        Some(s) => s,
        None => {
            return Json(json!({
                "ok": false,
                "error": "Access Absorber not initialized"
            }))
        }
    };

    match service.cleanup_expired().await {
        Ok(count) => Json(json!({
            "ok": true,
            "data": {
                "cleaned": count
            }
        })),
        Err(e) => Json(json!({
            "ok": false,
            "error": e.to_string()
        })),
    }
}
