//! Error types for DocSign API

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Invalid signing key")]
    InvalidSigningKey,

    #[error("Session expired")]
    SessionExpired,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::SessionNotFound(id) => {
                (StatusCode::NOT_FOUND, format!("Session not found: {}", id))
            }
            ApiError::InvalidSigningKey => {
                (StatusCode::UNAUTHORIZED, "Invalid signing key".to_string())
            }
            ApiError::SessionExpired => (StatusCode::GONE, "Session has expired".to_string()),
            ApiError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Database(e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
            ApiError::Internal(e) => {
                tracing::error!("Internal error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}
