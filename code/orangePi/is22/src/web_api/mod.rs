//! WebAPI - REST API Endpoints
//!
//! ## Responsibilities
//!
//! - HTTP API routes
//! - Request validation
//! - Response formatting

mod routes;
mod sdm_routes;

pub use routes::create_router;
pub use sdm_routes::sdm_routes;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use crate::state::AppState;
use crate::models::HealthResponse;

/// Health check endpoint
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let is21_ok = state.ai_client.health_check().await.unwrap_or(false);
    let go2rtc_ok = state.stream.health_check().await.unwrap_or(false);

    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_sec: 0, // TODO: Track uptime
        is21_connected: is21_ok,
        go2rtc_connected: go2rtc_ok,
        db_connected: true, // If we got here, DB is connected
    };

    Json(response)
}

/// Status endpoint (araneaDevices common)
pub async fn device_status(State(_state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "device_type": "ar-is22",
        "firmware_version": env!("CARGO_PKG_VERSION"),
        "status": "running"
    }))
}
