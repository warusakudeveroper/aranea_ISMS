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
    Database(#[from] sqlx::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
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
            Error::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                e.to_string(),
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
            Error::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
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
