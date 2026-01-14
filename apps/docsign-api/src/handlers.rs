//! HTTP handlers for DocSign API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::state::AppState;

/// Health check endpoint
pub async fn health() -> &'static str {
    "OK"
}

/// Create a new signing session
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    // Decode PDF
    let pdf_data = BASE64
        .decode(&req.pdf_base64)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid PDF base64: {}", e)))?;

    // Generate document hash
    let document_hash = hex::encode(Sha256::digest(&pdf_data));

    // Generate session ID
    let session_id = Uuid::new_v4().to_string();

    // Calculate expiry
    let expires_at = req
        .expires_in_hours
        .map(|h| Utc::now() + chrono::Duration::hours(h));

    let now = Utc::now();

    // Serialize JSON fields
    let recipients_json = serde_json::to_string(&req.recipients)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid recipients: {}", e)))?;
    let fields_json = serde_json::to_string(&req.fields)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid fields: {}", e)))?;

    // Insert into database
    sqlx::query(
        r#"
        INSERT INTO sessions (id, document_name, document_hash, pdf_data, recipients_json, fields_json, status, created_at, updated_at, expires_at)
        VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?)
        "#,
    )
    .bind(&session_id)
    .bind(&req.document_name)
    .bind(&document_hash)
    .bind(&pdf_data)
    .bind(&recipients_json)
    .bind(&fields_json)
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(expires_at.map(|e| e.to_rfc3339()))
    .execute(&state.db)
    .await?;

    tracing::info!("Created session: {}", session_id);

    Ok(Json(SessionResponse {
        id: session_id,
        document_name: req.document_name,
        document_hash,
        recipients: req.recipients,
        fields: req.fields,
        status: SessionStatus::Pending,
        created_at: now,
        updated_at: now,
        expires_at,
    }))
}

/// Get session by ID
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionResponse>, ApiError> {
    let session: Option<DbSession> = sqlx::query_as(
        r#"
        SELECT id, document_name, document_hash, pdf_data, recipients_json, fields_json,
               signatures_json, status, created_at, updated_at, expires_at
        FROM sessions
        WHERE id = ?
        "#,
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    let session = session.ok_or_else(|| ApiError::SessionNotFound(id.clone()))?;

    // Check expiry
    if let Some(expires) = &session.expires_at {
        if expires < &Utc::now() {
            return Err(ApiError::SessionExpired);
        }
    }

    // Parse JSON fields
    let recipients: Vec<Recipient> =
        serde_json::from_str(&session.recipients_json).map_err(|e| ApiError::Internal(e.into()))?;
    let fields: Vec<SignatureField> =
        serde_json::from_str(&session.fields_json).map_err(|e| ApiError::Internal(e.into()))?;

    let status = match session.status.as_str() {
        "pending" => SessionStatus::Pending,
        "in_progress" => SessionStatus::InProgress,
        "completed" => SessionStatus::Completed,
        "declined" => SessionStatus::Declined,
        "expired" => SessionStatus::Expired,
        _ => SessionStatus::Pending,
    };

    Ok(Json(SessionResponse {
        id: session.id,
        document_name: session.document_name,
        document_hash: session.document_hash,
        recipients,
        fields,
        status,
        created_at: session.created_at,
        updated_at: session.updated_at,
        expires_at: session.expires_at,
    }))
}

/// Get document PDF for a session
pub async fn get_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<(StatusCode, [(String, String); 2], Vec<u8>), ApiError> {
    let session: Option<DbSession> = sqlx::query_as(
        r#"
        SELECT id, document_name, document_hash, pdf_data, recipients_json, fields_json,
               signatures_json, status, created_at, updated_at, expires_at
        FROM sessions
        WHERE id = ?
        "#,
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    let session = session.ok_or_else(|| ApiError::SessionNotFound(id.clone()))?;

    // Check expiry
    if let Some(expires) = &session.expires_at {
        if expires < &Utc::now() {
            return Err(ApiError::SessionExpired);
        }
    }

    Ok((
        StatusCode::OK,
        [
            ("Content-Type".to_string(), "application/pdf".to_string()),
            (
                "Content-Disposition".to_string(),
                format!("inline; filename=\"{}\"", session.document_name),
            ),
        ],
        session.pdf_data,
    ))
}

/// Sync signatures from client
pub async fn sync_signatures(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SyncSignaturesRequest>,
) -> Result<Json<SyncResponse>, ApiError> {
    // Get session
    let session: Option<DbSession> = sqlx::query_as(
        r#"
        SELECT id, document_name, document_hash, pdf_data, recipients_json, fields_json,
               signatures_json, status, created_at, updated_at, expires_at
        FROM sessions
        WHERE id = ?
        "#,
    )
    .bind(&req.session_id)
    .fetch_optional(&state.db)
    .await?;

    let session = session.ok_or_else(|| ApiError::SessionNotFound(req.session_id.clone()))?;

    // Check expiry
    if let Some(expires) = &session.expires_at {
        if expires < &Utc::now() {
            return Err(ApiError::SessionExpired);
        }
    }

    // Verify signing key
    let recipients: Vec<Recipient> =
        serde_json::from_str(&session.recipients_json).map_err(|e| ApiError::Internal(e.into()))?;

    let recipient = recipients
        .iter()
        .find(|r| r.id == req.recipient_id)
        .ok_or(ApiError::InvalidRequest("Recipient not found".into()))?;

    // Validate signing key if set
    if let Some(expected_key) = &recipient.signing_key {
        if expected_key != &req.signing_key {
            return Err(ApiError::InvalidSigningKey);
        }
    }

    // Parse existing signatures
    let mut existing_signatures: HashMap<String, SignatureData> =
        serde_json::from_str(&session.signatures_json).unwrap_or_default();

    // Merge new signatures (client takes precedence if newer)
    let server_timestamp = Utc::now().timestamp_millis();
    let synced_count = req.signatures.len();

    for (field_id, sig_data) in req.signatures {
        // Client's signature takes precedence
        existing_signatures.insert(field_id, sig_data);
    }

    // Determine new status
    let fields: Vec<SignatureField> =
        serde_json::from_str(&session.fields_json).map_err(|e| ApiError::Internal(e.into()))?;

    let all_required_signed = fields
        .iter()
        .filter(|f| f.required)
        .all(|f| existing_signatures.contains_key(&f.id));

    let new_status = if req.completed_at.is_some() && all_required_signed {
        SessionStatus::Completed
    } else if !existing_signatures.is_empty() {
        SessionStatus::InProgress
    } else {
        SessionStatus::Pending
    };

    // Update database
    let signatures_json =
        serde_json::to_string(&existing_signatures).map_err(|e| ApiError::Internal(e.into()))?;

    sqlx::query(
        r#"
        UPDATE sessions
        SET signatures_json = ?, status = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&signatures_json)
    .bind(new_status.to_string())
    .bind(Utc::now().to_rfc3339())
    .bind(&req.session_id)
    .execute(&state.db)
    .await?;

    tracing::info!(
        "Synced signatures for session {}, recipient {}: {} signatures, status: {:?}",
        req.session_id,
        req.recipient_id,
        existing_signatures.len(),
        new_status
    );

    Ok(Json(SyncResponse {
        success: true,
        session_status: new_status,
        server_timestamp,
        message: Some(format!("Synced {} signatures", synced_count)),
    }))
}
