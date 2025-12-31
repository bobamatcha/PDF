//! Error types for the agentPDF server

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// Server error types
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Compilation error: {0}")]
    CompileError(String),

    #[error("Render timeout after {0}ms")]
    Timeout(u64),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Rate limit exceeded")]
    #[allow(dead_code)] // Will be used when rate limiting is enforced
    RateLimitExceeded,
}

/// Error response body
#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
    code: String,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            ServerError::TemplateNotFound(name) => (
                StatusCode::NOT_FOUND,
                "TEMPLATE_NOT_FOUND",
                format!("Template '{}' not found", name),
            ),
            ServerError::CompileError(msg) => {
                (StatusCode::BAD_REQUEST, "COMPILE_ERROR", msg.clone())
            }
            ServerError::Timeout(ms) => (
                StatusCode::REQUEST_TIMEOUT,
                "TIMEOUT",
                format!("Render timeout after {}ms", ms),
            ),
            ServerError::InvalidRequest(msg) => {
                (StatusCode::BAD_REQUEST, "INVALID_REQUEST", msg.clone())
            }
            ServerError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
            ServerError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT_EXCEEDED",
                "Too many requests, please slow down".to_string(),
            ),
        };

        let body = ErrorResponse {
            success: false,
            error: message,
            code: code.to_string(),
        };

        (status, Json(body)).into_response()
    }
}

impl From<typst_engine::compiler::ServerError> for ServerError {
    fn from(err: typst_engine::compiler::ServerError) -> Self {
        use typst_engine::compiler::ServerError as TypstError;
        match err {
            TypstError::TemplateNotFound(name) => ServerError::TemplateNotFound(name),
            TypstError::Timeout(ms) => ServerError::Timeout(ms),
            TypstError::CompileError(errors) => {
                let msg = errors
                    .iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ");
                ServerError::CompileError(msg)
            }
            other => ServerError::Internal(other.to_string()),
        }
    }
}
