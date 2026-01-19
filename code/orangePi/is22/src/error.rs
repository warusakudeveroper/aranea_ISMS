//! Error handling for IS22 Camserver

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// Error types
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Conflict (duplicate)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Unauthorized
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Forbidden
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Over capacity (admission control)
    #[error("Over capacity: {0}")]
    OverCapacity(String),

    /// System overload
    #[error("System overload: {0}")]
    SystemOverload(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Network error (AraneaRegister etc.)
    #[error("Network error: {0}")]
    Network(String),

    /// Config error
    #[error("Config error: {0}")]
    Config(String),

    /// API error
    #[error("API error: {0}")]
    Api(String),

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Summary service error
    #[error("Summary error: {0}")]
    Summary(String),

    /// Paraclate client error
    #[error("Paraclate error: {0}")]
    Paraclate(String),

    /// Access Absorber error (stream limit/connection control)
    #[error("Access denied for camera {camera_id}: {message}")]
    AccessAbsorber { camera_id: String, message: String },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// SQLx database error
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            Error::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone()),
            Error::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            Error::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            Error::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            Error::OverCapacity(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "OVER_CAPACITY",
                msg.clone(),
            ),
            Error::SystemOverload(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "SYSTEM_OVERLOAD",
                msg.clone(),
            ),
            Error::Database(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                msg.clone(),
            ),
            Error::Serialization(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERIALIZATION_ERROR",
                e.to_string(),
            ),
            Error::Http(e) => (
                StatusCode::BAD_GATEWAY,
                "HTTP_ERROR",
                e.to_string(),
            ),
            Error::Io(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "IO_ERROR",
                e.to_string(),
            ),
            Error::Network(msg) => (
                StatusCode::BAD_GATEWAY,
                "NETWORK_ERROR",
                msg.clone(),
            ),
            Error::Config(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_ERROR",
                msg.clone(),
            ),
            Error::Api(msg) => (
                StatusCode::BAD_GATEWAY,
                "API_ERROR",
                msg.clone(),
            ),
            Error::Parse(msg) => (
                StatusCode::BAD_REQUEST,
                "PARSE_ERROR",
                msg.clone(),
            ),
            Error::Summary(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SUMMARY_ERROR",
                msg.clone(),
            ),
            Error::Paraclate(msg) => (
                StatusCode::BAD_GATEWAY,
                "PARACLATE_ERROR",
                msg.clone(),
            ),
            Error::AccessAbsorber { camera_id, message } => (
                StatusCode::SERVICE_UNAVAILABLE,
                "ACCESS_DENIED",
                format!("Camera {}: {}", camera_id, message),
            ),
            Error::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
            Error::Sqlx(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                e.to_string(),
            ),
        };

        tracing::error!(
            status = %status,
            error_code = %error_code,
            message = %message,
            "Request error"
        );

        let body = Json(json!({
            "error_code": error_code,
            "message": message
        }));

        (status, body).into_response()
    }
}
