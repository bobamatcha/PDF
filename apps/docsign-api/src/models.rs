//! Data models for DocSign API

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Session status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Pending,
    InProgress,
    Completed,
    Declined,
    Expired,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Pending => write!(f, "pending"),
            SessionStatus::InProgress => write!(f, "in_progress"),
            SessionStatus::Completed => write!(f, "completed"),
            SessionStatus::Declined => write!(f, "declined"),
            SessionStatus::Expired => write!(f, "expired"),
        }
    }
}

/// Recipient in a signing session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub status: String,
    pub signing_key: Option<String>,
}

/// Signature field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureField {
    pub id: String,
    pub field_type: String,
    pub page: i32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub recipient_id: String,
    pub required: bool,
}

/// Captured signature data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureData {
    pub field_id: String,
    pub data_url: String,
    pub timestamp: DateTime<Utc>,
}

/// Signing session stored in database
#[derive(Debug, Clone, FromRow)]
pub struct DbSession {
    pub id: String,
    pub document_name: String,
    pub document_hash: String,
    pub pdf_data: Vec<u8>,
    pub recipients_json: String,
    pub fields_json: String,
    pub signatures_json: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Session response for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub id: String,
    pub document_name: String,
    pub document_hash: String,
    pub recipients: Vec<Recipient>,
    pub fields: Vec<SignatureField>,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request to create a new session
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSessionRequest {
    pub document_name: String,
    pub pdf_base64: String,
    pub recipients: Vec<Recipient>,
    pub fields: Vec<SignatureField>,
    #[serde(default)]
    pub expires_in_hours: Option<i64>,
}

/// Request to sync signatures from client
#[derive(Debug, Clone, Deserialize)]
pub struct SyncSignaturesRequest {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "recipientId")]
    pub recipient_id: String,
    #[serde(rename = "signingKey")]
    pub signing_key: String,
    pub signatures: std::collections::HashMap<String, SignatureData>,
    #[serde(rename = "completedAt")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(rename = "clientTimestamp")]
    pub client_timestamp: i64,
}

/// Response from sync operation
#[derive(Debug, Clone, Serialize)]
pub struct SyncResponse {
    pub success: bool,
    pub session_status: SessionStatus,
    #[serde(rename = "serverTimestamp")]
    pub server_timestamp: i64,
    pub message: Option<String>,
}

/// Conflict response for sync
#[derive(Debug, Clone, Serialize)]
pub struct ConflictResponse {
    #[serde(rename = "serverTimestamp")]
    pub server_timestamp: i64,
    pub signatures: std::collections::HashMap<String, SignatureData>,
}
