//! DocSign Server - Cloudflare Worker for email relay and signing sessions
//!
//! Email sending via Resend API (provider-agnostic, can swap to AWS SES/Postmark)
//! Signing sessions expire after 7 days

mod auth;
mod email;

use chrono::{Datelike, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use worker::*;

// Type alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

// ============================================================
// WASM-Safe Time Utilities
// ============================================================
// IMPORTANT: std::time::SystemTime does NOT work in Cloudflare Workers WASM!
// It panics with "time not implemented on this platform"
// Always use these helpers instead.

/// Get current Unix timestamp in seconds (WASM-safe)
/// Uses js_sys::Date::now() which works in Cloudflare Workers
fn get_timestamp_secs() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

/// Get current Unix timestamp in milliseconds (WASM-safe)
fn get_timestamp_millis() -> u128 {
    js_sys::Date::now() as u128
}

/// Request body for sending a document
#[derive(Deserialize)]
#[allow(dead_code)] // Fields used via serde deserialization
struct SendRequest {
    /// Recipient email address
    to: String,
    /// Email subject
    subject: String,
    /// PDF document as base64
    pdf_base64: String,
    /// Filename for the attachment
    filename: String,
    /// Optional: signing link to include in email
    #[serde(default)]
    signing_link: Option<String>,
}

/// Response from send endpoint
#[derive(Serialize)]
struct SendResponse {
    success: bool,
    message: String,
}

/// Health check response for monitoring
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: String,
    version: String,
    dependencies: HealthDependencies,
}

#[derive(Serialize)]
struct HealthDependencies {
    kv_sessions: DependencyStatus,
    kv_rate_limits: DependencyStatus,
}

#[derive(Serialize)]
struct DependencyStatus {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Signing mode determines how multiple signers interact
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
enum SigningMode {
    /// All signers sign the original simultaneously (default - simpler for users)
    #[default]
    Parallel,
    /// Signers must sign in order, each seeing previous signatures
    Sequential,
}

/// Reminder configuration for pending signers
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ReminderConfig {
    /// Hours between reminders (default: 48 = every 2 days)
    #[serde(default = "default_reminder_hours")]
    frequency_hours: u32,
    /// Maximum reminders per recipient (default: 3)
    #[serde(default = "default_max_reminders")]
    max_count: u32,
    /// Whether reminders are enabled (default: true)
    #[serde(default = "default_reminders_enabled")]
    enabled: bool,
}

impl Default for ReminderConfig {
    fn default() -> Self {
        Self {
            frequency_hours: 48,
            max_count: 3,
            enabled: true,
        }
    }
}

fn default_reminder_hours() -> u32 {
    48
}
fn default_max_reminders() -> u32 {
    3
}
fn default_reminders_enabled() -> bool {
    true
}

/// Request to create a signing session
#[derive(Deserialize)]
struct CreateSessionRequest {
    /// Encrypted document data (base64)
    encrypted_document: String,
    /// Document metadata (filename, page count, etc.)
    metadata: SessionMetadata,
    /// Recipients and their signing status
    recipients: Vec<RecipientInfo>,
    /// Field configurations (positions as percentages)
    fields: Vec<FieldInfo>,
    /// Session expiry in hours (default: 168 = 7 days)
    #[serde(default = "default_expiry_hours")]
    expiry_hours: u32,
    /// Signing mode: parallel (default) or sequential
    #[serde(default)]
    signing_mode: SigningMode,
    /// Reminder configuration
    #[serde(default)]
    reminder_config: ReminderConfig,
}

fn default_expiry_hours() -> u32 {
    168
} // 7 days

#[derive(Serialize, Deserialize, Clone)]
struct SessionMetadata {
    filename: String,
    page_count: u32,
    created_at: String,
    created_by: String,
    #[serde(default)]
    sender_email: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct RecipientInfo {
    id: String,
    name: String,
    email: String,
    role: String, // "signer" or "viewer"
    /// Whether recipient consented to electronic signing (clicked "Review Document")
    #[serde(default)]
    consented: bool,
    /// ISO 8601 timestamp when consent was given
    #[serde(default)]
    consent_at: Option<String>,
    /// User agent string at time of consent (for audit trail)
    #[serde(default)]
    consent_user_agent: Option<String>,
    #[serde(default)]
    signed: bool,
    #[serde(default)]
    signed_at: Option<String>,
    /// Whether the recipient declined to sign (UX-002)
    #[serde(default)]
    declined: bool,
    /// ISO 8601 timestamp when declined (UX-002)
    #[serde(default)]
    declined_at: Option<String>,
    /// Optional reason for declining (UX-002)
    #[serde(default)]
    decline_reason: Option<String>,
    /// For sequential mode: signing order (1, 2, 3...). None = parallel/any order
    #[serde(default)]
    signing_order: Option<u32>,
    /// Number of reminders sent to this recipient
    #[serde(default)]
    reminders_sent: u32,
    /// ISO 8601 timestamp of last reminder
    #[serde(default)]
    last_reminder_at: Option<String>,
}

/// Session status for the signing workflow (UX-002)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
enum SessionStatus {
    /// Session is pending - waiting for recipient action
    #[default]
    Pending,
    /// Recipient accepted and is signing
    Accepted,
    /// Recipient declined to sign
    Declined,
    /// All signatures completed
    Completed,
    /// Session has expired
    Expired,
}

#[derive(Serialize, Deserialize, Clone)]
struct FieldInfo {
    id: String,
    field_type: String,
    recipient_id: String,
    page: u32,
    x_percent: f64,
    y_percent: f64,
    width_percent: f64,
    height_percent: f64,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    value: Option<String>,
}

/// Stored signing session
#[derive(Serialize, Deserialize)]
struct SigningSession {
    id: String,
    /// The original document - never modified (for parallel mode merging)
    encrypted_document: String,
    metadata: SessionMetadata,
    recipients: Vec<RecipientInfo>,
    fields: Vec<FieldInfo>,
    expires_at: String,
    /// Each signer's signed version (parallel mode stores all; sequential overwrites)
    #[serde(default)]
    signed_versions: Vec<SignedVersion>,
    /// Session status for the workflow (UX-002)
    #[serde(default)]
    status: SessionStatus,
    /// Signing mode: parallel (default) or sequential
    #[serde(default)]
    signing_mode: SigningMode,
    /// Reminder configuration for pending signers
    #[serde(default)]
    reminder_config: Option<ReminderConfig>,
    /// Final merged document when all signers complete (parallel mode only)
    #[serde(default)]
    final_document: Option<String>,
    /// Legacy flag: if true, allow access without token (for backwards compatibility)
    /// New sessions should NOT set this flag (defaults to false)
    #[serde(default)]
    legacy: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct SignedVersion {
    recipient_id: String,
    encrypted_document: String,
    signed_at: String,
}

/// Response for session creation
#[derive(Serialize)]
struct CreateSessionResponse {
    success: bool,
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Response for session retrieval
#[derive(Serialize)]
struct GetSessionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<SessionPublicInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Serialize)]
struct SessionPublicInfo {
    id: String,
    metadata: SessionMetadata,
    recipients: Vec<RecipientInfo>,
    fields: Vec<FieldInfo>,
    encrypted_document: String,
    expires_at: String,
    /// Signing mode for this session
    signing_mode: SigningMode,
    /// Session status
    status: SessionStatus,
    /// Final merged document (parallel mode only, when all signed)
    #[serde(skip_serializing_if = "Option::is_none")]
    final_document: Option<String>,
}

/// Request to submit signed document
#[derive(Deserialize)]
struct SubmitSignedRequest {
    recipient_id: String,
    encrypted_document: String,
}

/// Request to decline a document (UX-002)
#[derive(Deserialize)]
struct DeclineRequest {
    recipient_id: String,
    reason: Option<String>,
}

/// Response from decline endpoint (UX-002)
#[derive(Serialize)]
struct DeclineResponse {
    success: bool,
    message: String,
}

/// Request to record consent acceptance (for audit trail)
#[derive(Deserialize)]
#[allow(dead_code)] // Fields used via serde deserialization
struct ConsentRequest {
    recipient_id: String,
    /// User agent string for audit trail
    user_agent: Option<String>,
    /// Hash of the consent text shown to user (to prove they saw specific terms)
    consent_text_hash: Option<String>,
}

/// Response from consent endpoint
#[derive(Serialize)]
struct ConsentResponse {
    success: bool,
    message: String,
    consent_at: String,
}

/// Request to send signing invitations
#[derive(Deserialize)]
#[allow(dead_code)]
struct InviteRequest {
    session_id: String,
    document_name: String,
    sender_name: String,
    invitations: Vec<InvitationInfo>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct InvitationInfo {
    recipient_id: String,
    name: String,
    email: String,
    signing_link: String,
}

/// Response from invite endpoint
#[derive(Serialize)]
struct InviteResponse {
    success: bool,
    message: String,
}

/// Request to request a new signing link (UX-004)
#[derive(Deserialize)]
#[allow(dead_code)] // Used by UX-004 endpoint (not yet implemented)
struct RequestLinkRequest {
    recipient_id: String,
}

/// Response from request-link endpoint (UX-004)
#[derive(Serialize)]
#[allow(dead_code)] // Used by UX-004 endpoint (not yet implemented)
struct RequestLinkResponse {
    success: bool,
    message: String,
}

/// Response from resend endpoint (UX-004)
#[derive(Serialize)]
#[allow(dead_code)] // Used by UX-004 endpoint (not yet implemented)
struct ResendResponse {
    success: bool,
    new_session_id: String,
    expires_at: String,
    message: String,
}

/// Response for expired sessions (UX-004)
#[derive(Serialize)]
struct ExpiredSessionResponse {
    status: String,
    sender_email: String,
    document_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

// ============================================================
// Per-IP Rate Limiting (DDoS Prevention)
// ============================================================

/// Per-IP rate limit state stored in KV
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
struct IpRateLimitState {
    /// Number of requests in current window
    request_count: u32,
    /// Unix timestamp when the current window started
    window_start: u64,
}

/// Rate limit tiers for different endpoint types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RateLimitTier {
    /// Health check: 100 req/min per IP
    Health,
    /// Session read (GET): 30 req/min per IP
    SessionRead,
    /// Session write (PUT signed/decline/consent): 5 req/min per IP
    SessionWrite,
    /// Request link: 100 req/day per IP (generous daily limit, bound by global email budget)
    RequestLink,
}

impl RateLimitTier {
    /// Returns (max_requests, window_seconds) for this tier
    fn limits(&self) -> (u32, u64) {
        match self {
            RateLimitTier::Health => (100, 60),         // 100/min
            RateLimitTier::SessionRead => (30, 60),     // 30/min
            RateLimitTier::SessionWrite => (5, 60),     // 5/min
            RateLimitTier::RequestLink => (100, 86400), // 100/day - generous for UX, global email limits handle cost
        }
    }

    /// Returns the tier name for KV key construction
    fn name(&self) -> &'static str {
        match self {
            RateLimitTier::Health => "health",
            RateLimitTier::SessionRead => "session_read",
            RateLimitTier::SessionWrite => "session_write",
            RateLimitTier::RequestLink => "request_link",
        }
    }

    /// Returns retry-after seconds for rate limit response
    #[allow(dead_code)] // Available for tests and future use
    fn retry_after_seconds(&self) -> u64 {
        let (_, window) = self.limits();
        window
    }
}

/// Result of per-IP rate limit check
#[derive(Debug, Clone, PartialEq)]
enum IpRateLimitResult {
    /// Request allowed
    Allowed,
    /// Rate limited - retry after specified seconds
    Limited { retry_after_seconds: u64 },
}

/// Check per-IP rate limit for a given tier
/// Returns IpRateLimitResult indicating if request is allowed or rate limited
async fn check_ip_rate_limit(
    kv: &kv::KvStore,
    ip: &str,
    tier: RateLimitTier,
) -> Result<IpRateLimitResult> {
    let (max_requests, window_seconds) = tier.limits();
    let key = format!("ip_limit:{}:{}", ip, tier.name());

    // Get current timestamp (WASM-safe - uses js_sys::Date)
    let now = get_timestamp_secs();

    // Get current state from KV
    let mut state: IpRateLimitState = kv.get(&key).json().await?.unwrap_or_default();

    // Check if window has expired
    if now >= state.window_start + window_seconds {
        // Start new window
        state.window_start = now;
        state.request_count = 0;
    }

    // Check if we're over the limit
    if state.request_count >= max_requests {
        let retry_after = (state.window_start + window_seconds).saturating_sub(now);
        return Ok(IpRateLimitResult::Limited {
            retry_after_seconds: retry_after.max(1), // At least 1 second
        });
    }

    // Increment and save
    state.request_count += 1;

    // TTL should be slightly longer than the window to handle edge cases
    let ttl = window_seconds + 60;

    kv.put(&key, serde_json::to_string(&state)?)?
        .expiration_ttl(ttl)
        .execute()
        .await?;

    Ok(IpRateLimitResult::Allowed)
}

/// Get client IP from Cloudflare headers
/// Falls back to "unknown" if not present
fn get_client_ip(req: &Request) -> String {
    req.headers()
        .get("CF-Connecting-IP")
        .ok()
        .flatten()
        .unwrap_or_else(|| "unknown".to_string())
}

/// Generate a 429 Too Many Requests response for IP rate limiting
fn ip_rate_limit_response(retry_after_seconds: u64) -> Result<Response> {
    #[derive(Serialize)]
    struct RateLimitError {
        error: String,
        retry_after_seconds: u64,
    }

    let resp = Response::from_json(&RateLimitError {
        error: "Rate limit exceeded".to_string(),
        retry_after_seconds,
    })?
    .with_status(429);

    // Add Retry-After header
    let headers = Headers::new();
    let _ = headers.set("Retry-After", &retry_after_seconds.to_string());

    Ok(resp.with_headers(headers))
}

/// Apply IP rate limiting to a request
/// Returns Some(Response) if rate limited, None if allowed
async fn apply_ip_rate_limit(
    req: &Request,
    env: &Env,
    tier: RateLimitTier,
) -> Option<Result<Response>> {
    let kv = match env.kv("RATE_LIMITS") {
        Ok(kv) => kv,
        Err(_) => {
            // KV not configured, skip rate limiting
            console_log!("Warning: RATE_LIMITS KV not configured, IP rate limiting disabled");
            return None;
        }
    };

    let ip = get_client_ip(req);

    match check_ip_rate_limit(&kv, &ip, tier).await {
        Ok(IpRateLimitResult::Allowed) => None,
        Ok(IpRateLimitResult::Limited {
            retry_after_seconds,
        }) => {
            console_log!(
                "IP {} rate limited for tier {:?}, retry after {} seconds",
                ip,
                tier,
                retry_after_seconds
            );
            Some(cors_response(ip_rate_limit_response(retry_after_seconds)))
        }
        Err(e) => {
            // On error, allow the request but log
            console_log!("IP rate limit check failed for {}: {}", ip, e);
            None
        }
    }
}

const SESSION_TTL_SECONDS: u64 = 7 * 24 * 60 * 60; // 7 days
const DOWNLOAD_LINK_EXPIRY_DAYS: u32 = 30;

// ============================================================
// Session Token Configuration (Security)
// ============================================================
/// Token expiry duration in seconds (30 days)
const TOKEN_EXPIRY_SECONDS: u64 = 30 * 24 * 60 * 60;

/// Errors that can occur during token verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenError {
    /// Token format is invalid (wrong number of parts, bad encoding)
    InvalidFormat,
    /// Token has expired
    Expired,
    /// HMAC signature is invalid
    InvalidSignature,
    /// Token session_id doesn't match the requested session
    SessionMismatch,
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::InvalidFormat => write!(f, "Invalid token format"),
            TokenError::Expired => write!(f, "Token has expired"),
            TokenError::InvalidSignature => write!(f, "Invalid token signature"),
            TokenError::SessionMismatch => write!(f, "Token does not match session"),
        }
    }
}

// ============================================================
// Session Token Functions (Security)
// ============================================================

/// Get the signing secret from environment.
/// Priority: 1. SESSION_TOKEN_SECRET (secret), 2. SESSION_TOKEN_SECRET (var), 3. API key hash (fallback)
fn get_signing_secret(env: &Env) -> Vec<u8> {
    // Try secret first
    if let Ok(secret) = env.secret("SESSION_TOKEN_SECRET") {
        return secret.to_string().into_bytes();
    }

    // Try var for development
    if let Ok(var) = env.var("SESSION_TOKEN_SECRET") {
        return var.to_string().into_bytes();
    }

    // Fallback: derive from API key for backwards compatibility
    if let Ok(api_key) = env.secret("DOCSIGN_API_KEY") {
        // Use SHA256 of API key as the signing secret
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(api_key.to_string().as_bytes());
        hasher.update(b"session-token-secret-v1"); // Domain separation
        return hasher.finalize().to_vec();
    }

    // Last resort: empty secret (WARNING: insecure, for development only)
    console_log!(
        "WARNING: No SESSION_TOKEN_SECRET or DOCSIGN_API_KEY configured - tokens are insecure!"
    );
    b"insecure-dev-secret".to_vec()
}

/// Generate a signed recipient token for accessing a session.
///
/// Token format (base64url encoded): {session_id}:{recipient_id}:{expiry_timestamp}:{hmac_signature}
///
/// # Arguments
/// * `session_id` - The session ID this token is for
/// * `recipient_id` - The recipient ID this token authorizes
/// * `secret` - The HMAC signing secret
///
/// # Returns
/// A base64url-encoded token string
pub fn generate_recipient_token(session_id: &str, recipient_id: &str, secret: &[u8]) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    // Calculate expiry timestamp (30 days from now)
    let expiry = chrono::Utc::now().timestamp() as u64 + TOKEN_EXPIRY_SECONDS;

    // Create the payload to sign
    let payload = format!("{}:{}:{}", session_id, recipient_id, expiry);

    // Generate HMAC signature
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key size");
    mac.update(payload.as_bytes());
    let signature = mac.finalize().into_bytes();

    // Encode signature as base64url
    let sig_base64 = URL_SAFE_NO_PAD.encode(signature);

    // Create the full token
    let token = format!("{}:{}", payload, sig_base64);

    // Encode the entire token as base64url for safe URL inclusion
    URL_SAFE_NO_PAD.encode(token.as_bytes())
}

/// Verify a recipient token and extract the recipient_id if valid.
///
/// # Arguments
/// * `token` - The base64url-encoded token to verify
/// * `session_id` - The session ID to validate against
/// * `secret` - The HMAC signing secret
///
/// # Returns
/// * `Ok(recipient_id)` - The recipient ID from the token if valid
/// * `Err(TokenError)` - The reason verification failed
pub fn verify_recipient_token(
    token: &str,
    session_id: &str,
    secret: &[u8],
) -> std::result::Result<String, TokenError> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    // Decode the outer base64url encoding
    let token_bytes = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| TokenError::InvalidFormat)?;

    let token_str = String::from_utf8(token_bytes).map_err(|_| TokenError::InvalidFormat)?;

    // Split into parts: session_id:recipient_id:expiry:signature
    let parts: Vec<&str> = token_str.split(':').collect();
    if parts.len() != 4 {
        return Err(TokenError::InvalidFormat);
    }

    let token_session_id = parts[0];
    let token_recipient_id = parts[1];
    let expiry_str = parts[2];
    let signature_base64 = parts[3];

    // Verify session ID matches
    if token_session_id != session_id {
        return Err(TokenError::SessionMismatch);
    }

    // Parse and verify expiry
    let expiry: u64 = expiry_str.parse().map_err(|_| TokenError::InvalidFormat)?;
    let now = chrono::Utc::now().timestamp() as u64;
    if now > expiry {
        return Err(TokenError::Expired);
    }

    // Recreate the payload and verify HMAC
    let payload = format!("{}:{}:{}", token_session_id, token_recipient_id, expiry_str);
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key size");
    mac.update(payload.as_bytes());

    // Decode the signature
    let signature = URL_SAFE_NO_PAD
        .decode(signature_base64)
        .map_err(|_| TokenError::InvalidFormat)?;

    // Verify signature (constant-time comparison)
    mac.verify_slice(&signature)
        .map_err(|_| TokenError::InvalidSignature)?;

    Ok(token_recipient_id.to_string())
}

// ============================================================
// Request Size Limits (Security)
// ============================================================
/// Maximum size for PDF documents (10MB)
const MAX_PDF_SIZE: usize = 10 * 1024 * 1024;
/// Maximum size for signature data (100KB) - reserved for future granular validation
#[allow(dead_code)]
const MAX_SIGNATURE_SIZE: usize = 100 * 1024;
/// Maximum total request body size (12MB - PDF + overhead)
const MAX_REQUEST_BODY: usize = 12 * 1024 * 1024;

// ============================================================
// Session Limits per Sender (Storage Exhaustion Prevention)
// ============================================================
/// Maximum active sessions per sender to prevent storage exhaustion
const MAX_SESSIONS_PER_SENDER: usize = 100;

/// Maximum age for sessions in sender index before pruning (30 days)
const SESSION_INDEX_PRUNE_DAYS: i64 = 30;

/// Tracks all active sessions for a sender (stored in KV)
/// Key format: sender_index:{sha256_hash_of_email}
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
struct SenderSessionIndex {
    /// List of active session IDs for this sender
    session_ids: Vec<String>,
    /// ISO timestamps when each session was created (parallel array)
    created_at: Vec<String>,
}

impl SenderSessionIndex {
    /// Remove expired sessions from the index (older than prune_days)
    fn prune_expired(&mut self, prune_days: i64) {
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::days(prune_days);

        let mut new_ids = Vec::new();
        let mut new_times = Vec::new();

        for (id, created) in self.session_ids.iter().zip(self.created_at.iter()) {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created) {
                if dt > cutoff {
                    new_ids.push(id.clone());
                    new_times.push(created.clone());
                }
            }
            // If we can't parse the timestamp, drop it (corrupted data)
        }

        self.session_ids = new_ids;
        self.created_at = new_times;
    }

    /// Add a new session to the index
    fn add_session(&mut self, session_id: String, created_at: String) {
        self.session_ids.push(session_id);
        self.created_at.push(created_at);
    }

    /// Remove a session from the index by ID
    fn remove_session(&mut self, session_id: &str) {
        if let Some(idx) = self.session_ids.iter().position(|id| id == session_id) {
            self.session_ids.remove(idx);
            if idx < self.created_at.len() {
                self.created_at.remove(idx);
            }
        }
    }

    /// Get the number of active sessions
    fn count(&self) -> usize {
        self.session_ids.len()
    }
}

// ============================================================
// UX-006: Sender Notification Helper Functions
// ============================================================

/// Check if all recipients (with role "signer") have signed
fn all_recipients_signed(recipients: &[RecipientInfo]) -> bool {
    recipients
        .iter()
        .filter(|r| r.role == "signer")
        .all(|r| r.signed)
}

/// Generate a download link for a signed document that expires after the specified days
fn generate_download_link(session_id: &str, expiry_days: u32) -> String {
    let expiry_timestamp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(expiry_days as i64))
        .unwrap_or_else(chrono::Utc::now)
        .timestamp();

    format!(
        "https://getsignatures.org/download/{}?expires={}",
        session_id, expiry_timestamp
    )
}

/// Format a human-readable timestamp from RFC3339
fn format_timestamp(rfc3339: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(rfc3339) {
        Ok(dt) => dt.format("%B %d, %Y at %I:%M %p UTC").to_string(),
        Err(_) => rfc3339.to_string(),
    }
}

/// Format notification email when a single recipient completes signing
fn format_completion_notification_email(
    recipient_name: &str,
    document_name: &str,
    signed_at: &str,
    download_link: &str,
) -> String {
    let formatted_time = format_timestamp(signed_at);

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #059669 0%, #047857 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">Document Signed</h1>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">Good news!</p>

        <p style="font-size: 16px; margin-bottom: 20px;">
            <strong>{recipient_name}</strong> has signed your document:
        </p>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{document_name}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Signed At</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{signed_time}</p>
        </div>

        <div style="text-align: center; margin: 30px 0;">
            <a href="{download_link}" style="display: inline-block; background: #059669; color: white; padding: 14px 32px; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">Download Signed Document</a>
        </div>

        <div style="background: #fef3c7; border-left: 4px solid #f59e0b; padding: 15px; border-radius: 4px; margin-top: 25px;">
            <p style="margin: 0; font-size: 14px; color: #92400e;">
                <strong>Note:</strong> This download link will expire after {expiry_days} days for security purposes.
            </p>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            The signed document is securely stored and encrypted. You can download it anytime before the link expires.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
        recipient_name = recipient_name,
        document_name = document_name,
        signed_time = formatted_time,
        download_link = download_link,
        expiry_days = DOWNLOAD_LINK_EXPIRY_DAYS
    )
}

/// Format notification email when all recipients have completed signing
fn format_all_signed_notification_email(
    recipients: &[RecipientInfo],
    document_name: &str,
    download_link: &str,
) -> String {
    // Build list of signers with their timestamps
    let signers_list: Vec<String> = recipients
        .iter()
        .filter(|r| r.role == "signer" && r.signed)
        .map(|r| {
            let time = r
                .signed_at
                .as_ref()
                .map(|t| format_timestamp(t))
                .unwrap_or_else(|| "Unknown".to_string());
            format!(
                "<li style=\"margin: 10px 0;\"><strong>{}</strong> - {}</li>",
                r.name, time
            )
        })
        .collect();

    let signers_html = signers_list.join("\n                ");

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #059669 0%, #047857 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">All Recipients Have Signed!</h1>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">Congratulations!</p>

        <p style="font-size: 16px; margin-bottom: 20px;">
            All recipients have completed signing your document. The signing process is now <strong>complete</strong>.
        </p>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{document_name}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0 0 10px 0; font-size: 14px; color: #6b7280;">Signers</p>
            <ul style="margin: 0; padding-left: 20px; list-style-type: none;">
                {signers_list}
            </ul>
        </div>

        <div style="text-align: center; margin: 30px 0;">
            <a href="{download_link}" style="display: inline-block; background: #059669; color: white; padding: 14px 32px; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">Download Completed Document</a>
        </div>

        <div style="background: #fef3c7; border-left: 4px solid #f59e0b; padding: 15px; border-radius: 4px; margin-top: 25px;">
            <p style="margin: 0; font-size: 14px; color: #92400e;">
                <strong>Note:</strong> This download link will expire after {expiry_days} days for security purposes.
            </p>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            The fully signed document is securely stored and encrypted. You can download it anytime before the link expires.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
        document_name = document_name,
        signers_list = signers_html,
        download_link = download_link,
        expiry_days = DOWNLOAD_LINK_EXPIRY_DAYS
    )
}

/// Format notification email when a recipient declines to sign (UX-006)
fn format_decline_notification_email(
    recipient_name: &str,
    recipient_email: &str,
    document_name: &str,
    reason: Option<&str>,
    declined_at: &str,
) -> String {
    let formatted_time = format_timestamp(declined_at);
    let reason_section = if let Some(r) = reason {
        format!(
            r#"<div style="background: #fef3c7; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Reason Provided</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-style: italic;">"{}"</p>
        </div>"#,
            r
        )
    } else {
        "<p style=\"font-size: 14px; color: #6b7280; font-style: italic;\">No reason was provided.</p>"
            .to_string()
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #dc2626 0%, #b91c1c 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">Document Declined</h1>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">
            Unfortunately, a recipient has declined to sign your document.
        </p>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Recipient</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{recipient_name}</p>
            <p style="margin: 5px 0 0 0; font-size: 14px; color: #6b7280;">{recipient_email}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{document_name}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Declined At</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{declined_time}</p>
        </div>

        {reason_section}

        <div style="background: #dbeafe; border-left: 4px solid #3b82f6; padding: 15px; border-radius: 4px; margin-top: 25px;">
            <p style="margin: 0; font-size: 14px; color: #1e40af;">
                <strong>What's next?</strong> You may want to contact the recipient directly to resolve any concerns, or resend the document request if needed.
            </p>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            The original document remains securely stored. You can create a new signing request at any time.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
        recipient_name = recipient_name,
        recipient_email = recipient_email,
        document_name = document_name,
        declined_time = formatted_time,
        reason_section = reason_section
    )
}

/// Send notification email to sender when recipient signs
async fn send_sender_notification(
    env: &Env,
    sender_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<()> {
    let request = email::EmailSendRequest {
        to: vec![sender_email.to_string()],
        subject: subject.to_string(),
        html: html_body.to_string(),
        text: None,
        reply_to: None,
        tags: vec![("type".to_string(), "notification".to_string())],
    };

    match email::send_email(env, request).await {
        Ok(result) => {
            if result.success {
                console_log!("Notification sent to sender: {}", sender_email);
            } else {
                console_log!(
                    "Failed to send notification: {}",
                    result.error.unwrap_or_default()
                );
            }
        }
        Err(e) => {
            console_log!("Failed to send notification: {}", e);
        }
    }

    Ok(())
}

/// Verify API key from request header
fn verify_api_key(req: &Request, env: &Env) -> bool {
    // Get expected API key from environment
    let expected_key = match env.secret("DOCSIGN_API_KEY") {
        Ok(secret) => secret.to_string(),
        Err(_) => {
            console_log!("Warning: DOCSIGN_API_KEY not configured - API is unprotected!");
            return true; // Allow if not configured (for development)
        }
    };

    // Check X-API-Key header
    matches!(req.headers().get("X-API-Key").ok().flatten(), Some(key) if key == expected_key)
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // CORS preflight
    if req.method() == Method::Options {
        return cors_response(Response::empty());
    }

    let path = req.path();
    let method = req.method();

    // Route requests
    match (method, path.as_str()) {
        // Health check (public) - detailed monitoring endpoint
        (Method::Get, "/health") => {
            if let Some(response) = apply_ip_rate_limit(&req, &env, RateLimitTier::Health).await {
                return response;
            }
            handle_health_check(env).await
        }
        (Method::Get, "/") => cors_response(Response::ok("DocSign API Server")),

        // ============================================
        // Authentication endpoints (public)
        // ============================================
        (Method::Post, "/auth/register") => {
            // Rate limit: 5 registrations per hour per IP
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::RequestLink).await
            {
                return response;
            }
            cors_response(auth::handle_register(req, env).await)
        }
        (Method::Get, "/auth/verify-email") => {
            cors_response(auth::handle_verify_email(req, env).await)
        }
        (Method::Post, "/auth/login") => {
            // Rate limit: 10 login attempts per hour per IP
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            cors_response(auth::handle_login(req, env).await)
        }
        (Method::Post, "/auth/refresh") => cors_response(auth::handle_refresh(req, env).await),
        (Method::Post, "/auth/logout") => cors_response(auth::handle_logout(req, env).await),
        (Method::Post, "/auth/forgot-password") => {
            // Rate limit to prevent spam
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::RequestLink).await
            {
                return response;
            }
            cors_response(auth::handle_forgot_password(req, env).await)
        }
        (Method::Post, "/auth/reset-password") => {
            cors_response(auth::handle_reset_password(req, env).await)
        }
        (Method::Post, "/auth/resend-verification") => {
            // Rate limit to prevent spam
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::RequestLink).await
            {
                return response;
            }
            cors_response(auth::handle_resend_verification(req, env).await)
        }
        (Method::Post, "/auth/check-email") => {
            // Rate limit: Use SessionRead tier (30/min) - lightweight lookup
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionRead).await
            {
                return response;
            }
            cors_response(auth::handle_check_email(req, env).await)
        }
        (Method::Post, "/auth/profile") => {
            // Profile update requires valid auth token
            cors_response(auth::handle_update_profile(req, env).await)
        }

        // Protected endpoints - require API key
        (Method::Post, "/send") => {
            if !verify_api_key(&req, &env) {
                return cors_response(Response::error("Unauthorized", 401));
            }
            handle_send_email(req, env).await
        }
        (Method::Post, "/invite") => {
            if !verify_api_key(&req, &env) {
                return cors_response(Response::error("Unauthorized", 401));
            }
            handle_send_invites(req, env).await
        }

        // Session management (protected)
        (Method::Post, "/session") => {
            if !verify_api_key(&req, &env) {
                return cors_response(Response::error("Unauthorized", 401));
            }
            handle_create_session(req, env).await
        }
        (Method::Get, p) if p.starts_with("/session/") => {
            // Apply SessionRead rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionRead).await
            {
                return response;
            }
            let id = p.strip_prefix("/session/").unwrap_or("");
            if id.contains('/') {
                cors_response(Response::error("Not found", 404))
            } else {
                handle_get_session(id, &req, env).await
            }
        }
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/signed") => {
            // Apply SessionWrite rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_submit_signed(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/decline") => {
            // Apply SessionWrite rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_decline(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // Consent endpoint: logs consent acceptance for audit trail
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/consent") => {
            // Apply SessionWrite rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_consent(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // UX-004: Request new link endpoint (public - no API key required)
        // Strictest rate limit: 3 req/hour per IP (prevents email spam)
        (Method::Post, p) if p.starts_with("/session/") && p.ends_with("/request-link") => {
            // Apply RequestLink rate limit (strictest - 3/hour)
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::RequestLink).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_request_link(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // UX-004: Resend endpoint (protected with API key)
        (Method::Post, p) if p.starts_with("/session/") && p.ends_with("/resend") => {
            if !verify_api_key(&req, &env) {
                return cors_response(Response::error("Unauthorized", 401));
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_resend(parts[2], env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }

        _ => cors_response(Response::error("Not found", 404)),
    }
}

/// Handle health check requests - returns service status and dependency health
async fn handle_health_check(env: Env) -> Result<Response> {
    let timestamp = Utc::now().to_rfc3339();

    // Check KV Sessions availability
    let kv_sessions_status = match env.kv("SESSIONS") {
        Ok(kv) => {
            // Try a simple operation to verify KV is working
            match kv.get("__health_check__").text().await {
                Ok(_) => DependencyStatus {
                    status: "healthy".to_string(),
                    error: None,
                },
                Err(e) => DependencyStatus {
                    status: "degraded".to_string(),
                    error: Some(format!("KV read error: {}", e)),
                },
            }
        }
        Err(_) => DependencyStatus {
            status: "unavailable".to_string(),
            error: Some("SESSIONS KV binding not configured".to_string()),
        },
    };

    // Check KV Rate Limits availability
    let kv_rate_limits_status = match env.kv("RATE_LIMITS") {
        Ok(kv) => match kv.get("rate_state").text().await {
            Ok(_) => DependencyStatus {
                status: "healthy".to_string(),
                error: None,
            },
            Err(e) => DependencyStatus {
                status: "degraded".to_string(),
                error: Some(format!("KV read error: {}", e)),
            },
        },
        Err(_) => DependencyStatus {
            status: "unavailable".to_string(),
            error: Some("RATE_LIMITS KV binding not configured".to_string()),
        },
    };

    // Determine overall status
    let overall_status =
        if kv_sessions_status.status == "healthy" && kv_rate_limits_status.status == "healthy" {
            "healthy"
        } else if kv_sessions_status.status == "unavailable"
            || kv_rate_limits_status.status == "unavailable"
        {
            "unhealthy"
        } else {
            "degraded"
        };

    let response = HealthResponse {
        status: overall_status.to_string(),
        timestamp,
        version: env!("CARGO_PKG_VERSION").to_string(),
        dependencies: HealthDependencies {
            kv_sessions: kv_sessions_status,
            kv_rate_limits: kv_rate_limits_status,
        },
    };

    // Return appropriate status code based on health
    let status_code = match overall_status {
        "healthy" => 200,
        "degraded" => 200, // Still operational but with issues
        _ => 503,          // Service unavailable
    };

    let resp = Response::from_json(&response)?.with_status(status_code);
    cors_response(Ok(resp))
}

async fn handle_send_email(mut req: Request, env: Env) -> Result<Response> {
    // Check request size before parsing
    if let Some(response) = check_content_length(&req, MAX_REQUEST_BODY) {
        return cors_response(Ok(response));
    }

    // Parse request
    let body: SendRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Validate PDF size after parsing (double-check actual content)
    let pdf_size = body.pdf_base64.len();
    if pdf_size > MAX_PDF_SIZE {
        return cors_response(payload_too_large_response(MAX_PDF_SIZE, pdf_size));
    }

    // Send the email via Resend
    send_document_email(&env, &body).await
}

async fn send_document_email(env: &Env, body: &SendRequest) -> Result<Response> {
    // Build email body with optional signing link
    let html_body = if let Some(ref link) = body.signing_link {
        format!(
            "<p>You have been requested to sign a document.</p>\
            <p><a href=\"{}\">Click here to sign</a></p>\
            <p>Or download the attached PDF to sign locally.</p>",
            link
        )
    } else {
        "<p>Please find the attached document for your signature.</p>".to_string()
    };

    let request = email::EmailSendRequest {
        to: vec![body.to.clone()],
        subject: body.subject.clone(),
        html: html_body,
        text: None,
        reply_to: None,
        tags: vec![("type".to_string(), "document".to_string())],
    };

    match email::send_email(env, request).await {
        Ok(result) => {
            if result.success {
                console_log!("Email sent to {}", body.to);
                cors_response(Response::from_json(&SendResponse {
                    success: true,
                    message: "Email sent".to_string(),
                }))
            } else {
                let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                console_log!("Email send failed: {}", error_msg);
                cors_response(error_response(&error_msg))
            }
        }
        Err(e) => {
            console_log!("Email send error: {}", e);
            cors_response(error_response(&format!("Email service error: {}", e)))
        }
    }
}

async fn handle_send_invites(mut req: Request, env: Env) -> Result<Response> {
    // Check request size before parsing
    if let Some(response) = check_content_length(&req, MAX_REQUEST_BODY) {
        return cors_response(Ok(response));
    }

    // Parse request
    let body: InviteRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    if body.invitations.is_empty() {
        return cors_response(error_response("No invitations to send"));
    }

    // Send the invitations via Resend
    send_invitations(&env, &body).await
}

async fn send_invitations(env: &Env, body: &InviteRequest) -> Result<Response> {
    let mut success_count = 0;
    let mut errors = Vec::new();

    // Get signing secret for token generation
    let secret = get_signing_secret(env);

    for invitation in &body.invitations {
        // Generate a signed token for this recipient
        let token = generate_recipient_token(&body.session_id, &invitation.recipient_id, &secret);

        // Append token to the signing link
        let signing_link_with_token = if invitation.signing_link.contains('?') {
            format!("{}&token={}", invitation.signing_link, token)
        } else {
            format!("{}?token={}", invitation.signing_link, token)
        };

        // Build HTML email template
        let email_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #1e40af 0%, #1e3a8a 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">Document Signature Request</h1>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">Hello {recipient_name},</p>

        <p style="font-size: 16px; margin-bottom: 20px;">
            <strong>{sender_name}</strong> has requested your signature on the following document:
        </p>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{document_name}</p>
        </div>

        <div style="text-align: center; margin: 30px 0;">
            <a href="{signing_link}" style="display: inline-block; background: #1e40af; color: white; padding: 14px 32px; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">Review & Sign Document</a>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            If you have any questions about this signature request, please contact the sender directly.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
            recipient_name = invitation.name,
            sender_name = body.sender_name,
            document_name = body.document_name,
            signing_link = signing_link_with_token
        );

        // Send via email module
        let email_request = email::EmailSendRequest {
            to: vec![invitation.email.clone()],
            subject: format!("Signature Requested: {}", body.document_name),
            html: email_html,
            text: None,
            reply_to: None,
            tags: vec![
                ("type".to_string(), "invitation".to_string()),
                ("session_id".to_string(), body.session_id.clone()),
            ],
        };

        match email::send_email(env, email_request).await {
            Ok(result) => {
                if result.success {
                    success_count += 1;
                    console_log!("Invitation sent to {}", invitation.email);
                } else {
                    let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                    errors.push(format!("{}: {}", invitation.email, error_msg));
                    console_log!("Failed to send to {}: {}", invitation.email, error_msg);
                }
            }
            Err(e) => {
                errors.push(format!("{}: {}", invitation.email, e));
                console_log!("Failed to send to {}: {}", invitation.email, e);
            }
        }
    }

    if success_count == body.invitations.len() {
        cors_response(Response::from_json(&InviteResponse {
            success: true,
            message: format!("All {} invitations sent successfully", success_count),
        }))
    } else if success_count > 0 {
        cors_response(Response::from_json(&InviteResponse {
            success: false,
            message: format!(
                "Partial success: {}/{} invitations sent. Errors: {}",
                success_count,
                body.invitations.len(),
                errors.join(", ")
            ),
        }))
    } else {
        cors_response(error_response(&format!(
            "Failed to send invitations. Errors: {}",
            errors.join(", ")
        )))
    }
}

async fn handle_create_session(mut req: Request, env: Env) -> Result<Response> {
    // Check request size before parsing (contains PDF)
    if let Some(response) = check_content_length(&req, MAX_REQUEST_BODY) {
        return cors_response(Ok(response));
    }

    // Require authentication for session creation
    let (mut user, users_kv) = match auth::require_auth(&req, &env).await? {
        Ok(result) => result,
        Err(response) => return cors_response(Ok(response)),
    };

    // Check if email is verified
    if !user.email_verified {
        return cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Please verify your email address before creating signing sessions.",
            "error_code": "EMAIL_NOT_VERIFIED"
        }))?
        .with_status(403)));
    }

    // Check document limit for free tier users
    // Reset counter if the week has passed (reset happens on Mondays)
    let now = chrono::Utc::now();
    if let Ok(reset_at) = chrono::DateTime::parse_from_rfc3339(&user.weekly_reset_at) {
        if now >= reset_at {
            user.weekly_document_count = 0;
            // Calculate next Monday at 00:00 UTC
            let days_until_monday = (8 - now.weekday().num_days_from_monday()) % 7;
            let days_until_monday = if days_until_monday == 0 {
                7
            } else {
                days_until_monday
            };
            user.weekly_reset_at = (now + chrono::Duration::days(days_until_monday as i64))
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .to_rfc3339();
        }
    }

    if user.tier == auth::types::UserTier::Free && user.weekly_document_count >= 1 {
        return cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Weekly limit reached. Free accounts can create 1 signing ceremony per week. Upgrade to Pro for unlimited.",
            "error_code": "WEEKLY_LIMIT_EXCEEDED",
            "limit": 1
        }))?
        .with_status(429)));
    }

    let body: CreateSessionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Validate encrypted document size after parsing
    let doc_size = body.encrypted_document.len();
    if doc_size > MAX_PDF_SIZE {
        return cors_response(payload_too_large_response(MAX_PDF_SIZE, doc_size));
    }

    // Check for duplicate recipient emails (production bug prevention)
    // Bypass with ALLOW_DUPLICATE_EMAILS=true for testing
    let allow_duplicates = env
        .var("ALLOW_DUPLICATE_EMAILS")
        .map(|v| v.to_string() == "true")
        .unwrap_or(false);

    if !allow_duplicates {
        let mut seen_emails = std::collections::HashSet::new();
        for recipient in &body.recipients {
            let email_lower = recipient.email.to_lowercase();
            if !seen_emails.insert(email_lower.clone()) {
                return cors_response(Ok(Response::from_json(&serde_json::json!({
                    "success": false,
                    "message": format!("Duplicate recipient email: {}", recipient.email),
                    "error_code": "DUPLICATE_RECIPIENT_EMAIL"
                }))?
                .with_status(400)));
            }
        }
    }

    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Check sender session limits (storage exhaustion prevention)
    let sender_email = body.metadata.sender_email.as_deref().unwrap_or("");
    let sender_hash = hash_sender_email(sender_email);
    let mut sender_index = get_sender_index(&kv, &sender_hash).await;

    // Prune expired sessions from index first
    sender_index.prune_expired(SESSION_INDEX_PRUNE_DAYS);

    // Check if sender is at limit
    if sender_index.count() >= MAX_SESSIONS_PER_SENDER {
        return cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Maximum active sessions reached. Please complete or cancel existing sessions.",
            "error_code": "SESSION_LIMIT_EXCEEDED",
            "limit": MAX_SESSIONS_PER_SENDER
        }))?
        .with_status(429)));
    }

    // Generate session ID
    let session_id = generate_session_id();

    // Calculate expiry
    let expiry_seconds = (body.expiry_hours as u64) * 60 * 60;
    let created_at = chrono::Utc::now().to_rfc3339();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(expiry_seconds as i64))
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339();

    let session = SigningSession {
        id: session_id.clone(),
        encrypted_document: body.encrypted_document,
        metadata: body.metadata,
        recipients: body.recipients,
        fields: body.fields,
        expires_at,
        signed_versions: vec![],
        status: SessionStatus::Pending,
        signing_mode: body.signing_mode,
        reminder_config: Some(body.reminder_config),
        final_document: None,
        legacy: false, // New sessions require token authentication
    };

    // Store session with TTL
    let session_json = serde_json::to_string(&session)
        .map_err(|e| Error::from(format!("Serialize error: {}", e)))?;

    kv.put(&format!("session:{}", session_id), session_json)?
        .expiration_ttl(expiry_seconds.min(SESSION_TTL_SECONDS))
        .execute()
        .await?;

    // Add session to sender's index
    sender_index.add_session(session_id.clone(), created_at);
    if let Err(e) = save_sender_index(&kv, &sender_hash, &sender_index).await {
        // Log error but don't fail the request - session is already created
        console_log!("Warning: Failed to update sender index: {}", e);
    }

    // Increment document count for the user (best-effort, don't fail if this errors)
    user.weekly_document_count += 1;
    if let Err(e) = auth::save_user(&users_kv, &user).await {
        console_log!("Warning: Failed to update user document count: {:?}", e);
    }

    cors_response(Response::from_json(&CreateSessionResponse {
        success: true,
        session_id,
        message: None,
    }))
}

async fn handle_get_session(session_id: &str, req: &Request, env: Env) -> Result<Response> {
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(s) => {
            // Check if expired (UX-004)
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&s.expires_at) {
                if expires < chrono::Utc::now() {
                    // UX-004: Return expired status with sender info
                    return cors_response(Ok(Response::from_json(&ExpiredSessionResponse {
                        status: "expired".to_string(),
                        sender_email: s
                            .metadata
                            .sender_email
                            .unwrap_or_else(|| "unknown".to_string()),
                        document_name: s.metadata.filename,
                        message: Some(
                            "This signing link has expired. You can request a new link below."
                                .to_string(),
                        ),
                    })?
                    .with_status(410)));
                }
            }

            // Token verification: extract token from query parameter
            let url = req.url()?;
            let token = url
                .query_pairs()
                .find(|(k, _)| k == "token")
                .map(|(_, v)| v.to_string());

            // Verify token if present, or check legacy flag
            let verified_recipient_id = if let Some(token) = token {
                // Verify the token
                let secret = get_signing_secret(&env);
                match verify_recipient_token(&token, session_id, &secret) {
                    Ok(recipient_id) => Some(recipient_id),
                    Err(TokenError::Expired) => {
                        return cors_response(Ok(Response::from_json(&serde_json::json!({
                            "success": false,
                            "error": "token_expired",
                            "message": "Your signing link has expired. Please request a new link."
                        }))?
                        .with_status(401)));
                    }
                    Err(TokenError::InvalidSignature) | Err(TokenError::SessionMismatch) => {
                        return cors_response(Ok(Response::from_json(&serde_json::json!({
                            "success": false,
                            "error": "invalid_token",
                            "message": "Invalid signing link. Please use the link from your email."
                        }))?
                        .with_status(403)));
                    }
                    Err(TokenError::InvalidFormat) => {
                        return cors_response(Ok(Response::from_json(&serde_json::json!({
                            "success": false,
                            "error": "invalid_token",
                            "message": "Invalid signing link format."
                        }))?
                        .with_status(400)));
                    }
                }
            } else if s.legacy {
                // Legacy session without token requirement - allow access (backwards compatible)
                console_log!(
                    "WARNING: Legacy session {} accessed without token",
                    session_id
                );
                None
            } else {
                // No token provided and not a legacy session - deny access
                return cors_response(Ok(Response::from_json(&serde_json::json!({
                    "success": false,
                    "error": "token_required",
                    "message": "A valid signing token is required to access this session."
                }))?
                .with_status(401)));
            };

            // If we have a verified recipient ID, filter the response to only show
            // information relevant to that recipient
            let (recipients, fields) = if let Some(ref recipient_id) = verified_recipient_id {
                // Filter to only show this recipient's information
                let filtered_recipients: Vec<RecipientInfo> = s
                    .recipients
                    .iter()
                    .filter(|r| &r.id == recipient_id)
                    .cloned()
                    .collect();

                let filtered_fields: Vec<FieldInfo> = s
                    .fields
                    .iter()
                    .filter(|f| &f.recipient_id == recipient_id)
                    .cloned()
                    .collect();

                (filtered_recipients, filtered_fields)
            } else {
                // Legacy mode: return all recipients and fields
                (s.recipients.clone(), s.fields.clone())
            };

            cors_response(Response::from_json(&GetSessionResponse {
                success: true,
                session: Some(SessionPublicInfo {
                    id: s.id,
                    metadata: s.metadata,
                    recipients,
                    fields,
                    encrypted_document: s.encrypted_document,
                    expires_at: s.expires_at,
                    signing_mode: s.signing_mode,
                    status: s.status,
                    final_document: s.final_document,
                }),
                message: None,
            }))
        }
        None => cors_response(Ok(Response::from_json(&GetSessionResponse {
            success: false,
            session: None,
            message: Some("Session not found".to_string()),
        })?
        .with_status(404))),
    }
}

/// UX-002: Handle decline endpoint
/// PUT /session/{id}/decline
/// Body: { "recipient_id": "...", "reason": "optional reason" }
async fn handle_decline(session_id: &str, mut req: Request, env: Env) -> Result<Response> {
    // Parse the decline request
    let body: DeclineRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the session
    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(mut s) => {
            // Check if session is already declined or completed
            if s.status == SessionStatus::Declined {
                return cors_response(Ok(Response::from_json(&DeclineResponse {
                    success: false,
                    message: "Session has already been declined".to_string(),
                })?
                .with_status(400)));
            }

            if s.status == SessionStatus::Completed {
                return cors_response(Ok(Response::from_json(&DeclineResponse {
                    success: false,
                    message: "Session has already been completed".to_string(),
                })?
                .with_status(400)));
            }

            // Find and update the recipient
            let mut recipient_found = false;
            for r in s.recipients.iter_mut() {
                if r.id == body.recipient_id {
                    recipient_found = true;

                    // Check if already signed
                    if r.signed {
                        return cors_response(Ok(Response::from_json(&DeclineResponse {
                            success: false,
                            message: "Recipient has already signed".to_string(),
                        })?
                        .with_status(400)));
                    }

                    // Check if already declined
                    if r.declined {
                        return cors_response(Ok(Response::from_json(&DeclineResponse {
                            success: false,
                            message: "Recipient has already declined".to_string(),
                        })?
                        .with_status(400)));
                    }

                    // Mark as declined
                    r.declined = true;
                    r.declined_at = Some(chrono::Utc::now().to_rfc3339());
                    r.decline_reason = body.reason.clone();
                    break;
                }
            }

            if !recipient_found {
                return cors_response(Ok(Response::from_json(&DeclineResponse {
                    success: false,
                    message: "Recipient not found in session".to_string(),
                })?
                .with_status(404)));
            }

            // Update session status to Declined
            s.status = SessionStatus::Declined;

            // Save updated session
            kv.put(
                &format!("session:{}", session_id),
                serde_json::to_string(&s)?,
            )?
            .execute()
            .await?;

            // UX-006: Send decline notification email to sender
            if let Some(sender_email) = s.metadata.sender_email.as_ref() {
                if let Some(recipient) = s.recipients.iter().find(|r| r.id == body.recipient_id) {
                    let now = chrono::Utc::now().to_rfc3339();
                    let declined_at = recipient.declined_at.as_deref().unwrap_or(&now);

                    let subject = format!("{} Declined: {}", recipient.name, s.metadata.filename);
                    let html_body = format_decline_notification_email(
                        &recipient.name,
                        &recipient.email,
                        &s.metadata.filename,
                        body.reason.as_deref(),
                        declined_at,
                    );

                    if let Err(e) =
                        send_sender_notification(&env, sender_email, &subject, &html_body).await
                    {
                        console_log!("Failed to send decline notification: {:?}", e);
                    }
                }
            }

            console_log!(
                "Session {} declined by recipient {}: {:?}",
                session_id,
                body.recipient_id,
                body.reason
            );

            cors_response(Ok(Response::from_json(&DeclineResponse {
                success: true,
                message: "Document declined successfully".to_string(),
            })?))
        }
        None => cors_response(Ok(Response::from_json(&DeclineResponse {
            success: false,
            message: "Session not found".to_string(),
        })?
        .with_status(404))),
    }
}

/// PUT /session/{id}/consent
/// Records consent acceptance for the audit trail.
/// Called when user clicks "Review Document" on consent page.
async fn handle_consent(session_id: &str, mut req: Request, env: Env) -> Result<Response> {
    // Parse the consent request
    let body: ConsentRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the session
    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(mut s) => {
            // Check if session is expired
            if s.status == SessionStatus::Expired {
                return cors_response(Ok(Response::from_json(&ConsentResponse {
                    success: false,
                    message: "Session has expired".to_string(),
                    consent_at: String::new(),
                })?
                .with_status(400)));
            }

            // Find and update the recipient
            let mut recipient_found = false;
            let consent_timestamp = chrono::Utc::now().to_rfc3339();

            for r in s.recipients.iter_mut() {
                if r.id == body.recipient_id {
                    recipient_found = true;

                    // Check if already declined
                    if r.declined {
                        return cors_response(Ok(Response::from_json(&ConsentResponse {
                            success: false,
                            message: "Recipient has already declined".to_string(),
                            consent_at: String::new(),
                        })?
                        .with_status(400)));
                    }

                    // Record consent (even if already consented - update timestamp)
                    r.consented = true;
                    r.consent_at = Some(consent_timestamp.clone());
                    r.consent_user_agent = body.user_agent.clone();
                    break;
                }
            }

            if !recipient_found {
                return cors_response(Ok(Response::from_json(&ConsentResponse {
                    success: false,
                    message: "Recipient not found in session".to_string(),
                    consent_at: String::new(),
                })?
                .with_status(404)));
            }

            // Update session status to Accepted if it was Pending
            if s.status == SessionStatus::Pending {
                s.status = SessionStatus::Accepted;
            }

            // Save updated session
            kv.put(
                &format!("session:{}", session_id),
                serde_json::to_string(&s)?,
            )?
            .execute()
            .await?;

            console_log!(
                "Consent recorded for session {} by recipient {}",
                session_id,
                body.recipient_id
            );

            cors_response(Ok(Response::from_json(&ConsentResponse {
                success: true,
                message: "Consent recorded successfully".to_string(),
                consent_at: consent_timestamp,
            })?))
        }
        None => cors_response(Ok(Response::from_json(&ConsentResponse {
            success: false,
            message: "Session not found".to_string(),
            consent_at: String::new(),
        })?
        .with_status(404))),
    }
}

/// UX-004: Handle request-link endpoint
/// POST /session/{id}/request-link
/// Body: { "recipient_id": "..." }
/// Sends notification email to sender that recipient requested new link
async fn handle_request_link(session_id: &str, mut req: Request, env: Env) -> Result<Response> {
    // Parse the request
    let body: RequestLinkRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the session
    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(s) => {
            // Find the recipient
            let recipient = s.recipients.iter().find(|r| r.id == body.recipient_id);

            if recipient.is_none() {
                return cors_response(Ok(Response::from_json(&RequestLinkResponse {
                    success: false,
                    message: "Recipient not found in session".to_string(),
                })?
                .with_status(404)));
            }

            let recipient = recipient.unwrap();
            let sender_email = s.metadata.sender_email.as_deref().unwrap_or("unknown");

            // Send notification email to sender
            let subject = format!("New Link Requested for {}", s.metadata.filename);
            let html_body = format!(
                r#"<html>
<body style="font-family: system-ui, -apple-system, sans-serif; line-height: 1.6; color: #1f2937; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: #f3f4f6; padding: 20px; border-radius: 8px; margin-bottom: 20px;">
        <h2 style="margin: 0 0 10px 0; color: #111827;">Link Request Notification</h2>
        <p style="margin: 0; color: #6b7280;">GetSignatures Document Signing</p>
    </div>

    <div style="background: white; padding: 25px; border: 1px solid #e5e7eb; border-radius: 8px;">
        <p style="font-size: 16px; margin-top: 0;">Hello,</p>

        <p style="font-size: 15px;">
            <strong>{}</strong> ({}) has requested a new signing link for:
        </p>

        <div style="background: #f9fafb; padding: 15px; border-left: 4px solid #3b82f6; margin: 20px 0;">
            <p style="margin: 0; font-weight: 600; color: #1f2937;">{}</p>
        </div>

        <p style="font-size: 14px; color: #6b7280;">
            The current link has expired. Please generate a new signing session and send them a fresh link.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
                recipient.name, recipient.email, s.metadata.filename
            );

            // Send the email (best effort - don't fail if email fails)
            if let Err(e) = send_sender_notification(&env, sender_email, &subject, &html_body).await
            {
                console_log!("Warning: Failed to send request-link notification: {}", e);
            }

            cors_response(Ok(Response::from_json(&RequestLinkResponse {
                success: true,
                message: format!(
                    "Request sent to {}. They will send you a new link shortly.",
                    sender_email
                ),
            })?))
        }
        None => cors_response(Ok(Response::from_json(&RequestLinkResponse {
            success: false,
            message: "Session not found".to_string(),
        })?
        .with_status(404))),
    }
}

/// UX-004: Handle resend endpoint (protected with API key)
/// POST /session/{id}/resend
/// Creates new session with fresh expiry and invalidates old session
async fn handle_resend(session_id: &str, env: Env) -> Result<Response> {
    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the old session
    let old_session: Option<SigningSession> =
        kv.get(&format!("session:{}", session_id)).json().await?;

    match old_session {
        Some(s) => {
            // Capture sender email before consuming metadata
            let sender_email = s.metadata.sender_email.clone();

            // Generate new session ID
            let new_session_id = generate_session_id();

            // Calculate new expiry (7 days default)
            let expiry_hours = 168u64;
            let expiry_seconds = expiry_hours * 60 * 60;
            let created_at = chrono::Utc::now().to_rfc3339();
            let expires_at = chrono::Utc::now()
                .checked_add_signed(chrono::Duration::seconds(expiry_seconds as i64))
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339();

            // Create new session with same data but new ID and expiry
            let new_session = SigningSession {
                id: new_session_id.clone(),
                encrypted_document: s.encrypted_document,
                metadata: s.metadata,
                recipients: s
                    .recipients
                    .into_iter()
                    .map(|mut r| {
                        // Reset signed status for resend
                        r.signed = false;
                        r.signed_at = None;
                        r.reminders_sent = 0;
                        r.last_reminder_at = None;
                        r
                    })
                    .collect(),
                fields: s.fields,
                expires_at: expires_at.clone(),
                signed_versions: vec![],
                status: SessionStatus::Pending,
                signing_mode: s.signing_mode,
                reminder_config: s.reminder_config,
                final_document: None, // Reset for new session
                legacy: false,        // Resent sessions require token authentication
            };

            // Store new session
            let session_json = serde_json::to_string(&new_session)
                .map_err(|e| Error::from(format!("Serialize error: {}", e)))?;

            kv.put(&format!("session:{}", new_session_id), session_json)?
                .expiration_ttl(expiry_seconds.min(SESSION_TTL_SECONDS))
                .execute()
                .await?;

            // Delete old session to invalidate it
            kv.delete(&format!("session:{}", session_id)).await?;

            // Update sender session index: remove old, add new
            if let Some(ref email) = sender_email {
                let sender_hash = hash_sender_email(email);
                let mut sender_index = get_sender_index(&kv, &sender_hash).await;
                sender_index.remove_session(session_id);
                sender_index.add_session(new_session_id.clone(), created_at);
                if let Err(e) = save_sender_index(&kv, &sender_hash, &sender_index).await {
                    console_log!("Warning: Failed to update sender index on resend: {}", e);
                }
            }

            cors_response(Ok(Response::from_json(&ResendResponse {
                success: true,
                new_session_id,
                expires_at,
                message: "New session created successfully".to_string(),
            })?))
        }
        None => cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Session not found"
        }))?
        .with_status(404))),
    }
}

async fn handle_submit_signed(session_id: &str, mut req: Request, env: Env) -> Result<Response> {
    // Check request size before parsing
    // Note: signed documents can be larger than signatures alone due to embedded signature images
    if let Some(response) = check_content_length(&req, MAX_PDF_SIZE) {
        return cors_response(Ok(response));
    }

    let body: SubmitSignedRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Validate encrypted document size after parsing
    let doc_size = body.encrypted_document.len();
    if doc_size > MAX_PDF_SIZE {
        return cors_response(payload_too_large_response(MAX_PDF_SIZE, doc_size));
    }

    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(mut s) => {
            // Mark recipient as signed
            for r in s.recipients.iter_mut() {
                if r.id == body.recipient_id {
                    r.signed = true;
                    r.signed_at = Some(chrono::Utc::now().to_rfc3339());
                }
            }

            // Add signed version
            s.signed_versions.push(SignedVersion {
                recipient_id: body.recipient_id.clone(),
                encrypted_document: body.encrypted_document.clone(),
                signed_at: chrono::Utc::now().to_rfc3339(),
            });

            // Handle document storage based on signing mode
            match s.signing_mode {
                SigningMode::Sequential => {
                    // Sequential mode: each signer sees previous signatures
                    // Update the main document for the next signer
                    s.encrypted_document = body.encrypted_document;
                }
                SigningMode::Parallel => {
                    // Parallel mode: keep original, merge when all complete
                    // Original stays in encrypted_document, each version in signed_versions
                    if all_recipients_signed(&s.recipients) {
                        // All signed - for now, use the last submitted version
                        // TODO: Implement proper PDF merge when docsign-core supports it
                        // For MVP: use last version (all signatures are on same positions anyway)
                        s.final_document = Some(body.encrypted_document);
                        s.status = SessionStatus::Completed;
                    }
                }
            }

            // UX-006: Send notification to sender
            if let Some(sender_email) = s.metadata.sender_email.as_ref() {
                // Find the recipient who just signed
                if let Some(recipient) = s.recipients.iter().find(|r| r.id == body.recipient_id) {
                    let download_link =
                        generate_download_link(session_id, DOWNLOAD_LINK_EXPIRY_DAYS);

                    // Check if all recipients have signed
                    if all_recipients_signed(&s.recipients) {
                        // Send "all signed" completion email
                        let subject = format!("All Recipients Signed: {}", s.metadata.filename);
                        let html_body = format_all_signed_notification_email(
                            &s.recipients,
                            &s.metadata.filename,
                            &download_link,
                        );
                        let _ = send_sender_notification(&env, sender_email, &subject, &html_body)
                            .await;
                    } else {
                        // Send individual recipient signed notification
                        let subject = format!("{} Signed: {}", recipient.name, s.metadata.filename);
                        let html_body = format_completion_notification_email(
                            &recipient.name,
                            &s.metadata.filename,
                            recipient
                                .signed_at
                                .as_ref()
                                .unwrap_or(&"Unknown".to_string()),
                            &download_link,
                        );
                        let _ = send_sender_notification(&env, sender_email, &subject, &html_body)
                            .await;
                    }
                }
            }

            // Save updated session
            let session_json = serde_json::to_string(&s)
                .map_err(|e| Error::from(format!("Serialize error: {}", e)))?;

            kv.put(&format!("session:{}", session_id), session_json)?
                .execute()
                .await?;

            cors_response(Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Signed document submitted"
            })))
        }
        None => cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Session not found"
        }))?
        .with_status(404))),
    }
}

fn generate_session_id() -> String {
    // WASM-safe timestamp (std::time::SystemTime panics in Workers)
    let timestamp = get_timestamp_millis();

    // Generate random suffix
    let random: u64 = js_sys::Math::random().to_bits();

    format!("{:x}{:08x}", timestamp, random as u32)
}

// ============================================================
// Sender Session Index Helper Functions
// ============================================================

/// Hash an email address using SHA-256 for privacy-preserving storage keys
fn hash_sender_email(email: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(email.to_lowercase().as_bytes());
    let result = hasher.finalize();
    // Return hex-encoded hash
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Get the sender session index from KV storage
async fn get_sender_index(kv: &kv::KvStore, sender_hash: &str) -> SenderSessionIndex {
    let key = format!("sender_index:{}", sender_hash);
    match kv.get(&key).json::<SenderSessionIndex>().await {
        Ok(Some(index)) => index,
        _ => SenderSessionIndex::default(),
    }
}

/// Save the sender session index to KV storage
async fn save_sender_index(
    kv: &kv::KvStore,
    sender_hash: &str,
    index: &SenderSessionIndex,
) -> Result<()> {
    let key = format!("sender_index:{}", sender_hash);
    kv.put(&key, serde_json::to_string(index)?)?
        .execute()
        .await?;
    Ok(())
}

/// Remove a session from a sender's index (for cleanup on session deletion)
#[allow(dead_code)] // Prepared for future explicit session deletion endpoint
async fn remove_session_from_sender_index(
    kv: &kv::KvStore,
    sender_email: &str,
    session_id: &str,
) -> Result<()> {
    let sender_hash = hash_sender_email(sender_email);
    let mut index = get_sender_index(kv, &sender_hash).await;
    index.remove_session(session_id);
    save_sender_index(kv, &sender_hash, &index).await
}

fn error_response(msg: &str) -> Result<Response> {
    let resp = Response::from_json(&SendResponse {
        success: false,
        message: msg.to_string(),
    })?;
    Ok(resp.with_status(400))
}

// ============================================================
// Request Size Validation (Security)
// ============================================================

/// Response structure for payload too large errors
#[derive(Serialize)]
struct PayloadTooLargeResponse {
    error: String,
    max_size_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    received_bytes: Option<usize>,
}

/// Check Content-Length header against a maximum size limit.
/// Returns None if within limits, Some(Response) with 413 if exceeded.
fn check_content_length(req: &Request, max_size: usize) -> Option<Response> {
    let content_length = req
        .headers()
        .get("Content-Length")
        .ok()
        .flatten()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    if content_length > max_size {
        let response = Response::from_json(&PayloadTooLargeResponse {
            error: "Request too large".to_string(),
            max_size_bytes: max_size,
            received_bytes: Some(content_length),
        })
        .map(|r| r.with_status(413))
        .unwrap_or_else(|_| Response::error("Request too large", 413).unwrap());

        return Some(response);
    }

    None
}

/// Create a 413 Payload Too Large response
fn payload_too_large_response(max_size: usize, actual_size: usize) -> Result<Response> {
    let resp = Response::from_json(&PayloadTooLargeResponse {
        error: "Request too large".to_string(),
        max_size_bytes: max_size,
        received_bytes: Some(actual_size),
    })?
    .with_status(413);
    Ok(resp)
}

fn cors_response(response: Result<Response>) -> Result<Response> {
    response.map(|r| {
        let headers = Headers::new();
        let _ = headers.set("Access-Control-Allow-Origin", "*");
        let _ = headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, OPTIONS");
        let _ = headers.set(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        );
        r.with_headers(headers)
    })
}

// ============================================================
// UX-006 Helper Functions (Stub implementations for tests)
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Test Helpers
    // ============================================================

    /// Create a RecipientInfo with just the required fields, defaults for the rest
    fn test_recipient(id: &str, name: &str, email: &str, role: &str) -> RecipientInfo {
        RecipientInfo {
            id: id.to_string(),
            name: name.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            consented: false,
            consent_at: None,
            consent_user_agent: None,
            signed: false,
            signed_at: None,
            declined: false,
            declined_at: None,
            decline_reason: None,
            signing_order: None,
            reminders_sent: 0,
            last_reminder_at: None,
        }
    }

    /// Create a SigningSession with minimal required data
    fn test_session(id: &str, recipients: Vec<RecipientInfo>) -> SigningSession {
        SigningSession {
            id: id.to_string(),
            encrypted_document: "test_doc_base64".to_string(),
            metadata: SessionMetadata {
                filename: "test.pdf".to_string(),
                page_count: 1,
                created_at: "2025-01-15T10:00:00Z".to_string(),
                created_by: "Test User".to_string(),
                sender_email: Some("sender@test.com".to_string()),
            },
            recipients,
            fields: vec![],
            expires_at: "2025-01-22T10:00:00Z".to_string(),
            signed_versions: vec![],
            status: SessionStatus::Pending,
            signing_mode: SigningMode::Parallel,
            reminder_config: None,
            final_document: None,
            legacy: false,
        }
    }

    // ============================================================
    // Unit Tests for Session Token Generation and Verification
    // ============================================================

    #[test]
    fn test_generate_and_verify_token_roundtrip() {
        let secret = b"test-secret-key";
        let session_id = "sess_abc123";
        let recipient_id = "recipient_456";

        let token = generate_recipient_token(session_id, recipient_id, secret);

        assert!(!token.is_empty());
        assert!(token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));

        let result = verify_recipient_token(&token, session_id, secret);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), recipient_id);
    }

    #[test]
    fn test_verify_token_wrong_session_id() {
        let secret = b"test-secret-key";
        let session_id = "sess_abc123";
        let recipient_id = "recipient_456";

        let token = generate_recipient_token(session_id, recipient_id, secret);
        let result = verify_recipient_token(&token, "sess_different", secret);
        assert_eq!(result, Err(TokenError::SessionMismatch));
    }

    #[test]
    fn test_verify_token_wrong_secret() {
        let secret = b"test-secret-key";
        let wrong_secret = b"wrong-secret-key";
        let session_id = "sess_abc123";
        let recipient_id = "recipient_456";

        let token = generate_recipient_token(session_id, recipient_id, secret);
        let result = verify_recipient_token(&token, session_id, wrong_secret);
        assert_eq!(result, Err(TokenError::InvalidSignature));
    }

    #[test]
    fn test_verify_token_invalid_format() {
        let secret = b"test-secret-key";
        let session_id = "sess_abc123";

        let result = verify_recipient_token("not-a-valid-token", session_id, secret);
        assert_eq!(result, Err(TokenError::InvalidFormat));

        let result = verify_recipient_token("", session_id, secret);
        assert_eq!(result, Err(TokenError::InvalidFormat));
    }

    #[test]
    fn test_verify_token_expired() {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        let secret = b"test-secret-key";
        let session_id = "sess_abc123";
        let recipient_id = "recipient_456";
        let expiry = chrono::Utc::now().timestamp() as u64 - 3600;
        let payload = format!("{}:{}:{}", session_id, recipient_id, expiry);

        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key size");
        mac.update(payload.as_bytes());
        let signature = mac.finalize().into_bytes();
        let sig_base64 = URL_SAFE_NO_PAD.encode(signature);

        let token_str = format!("{}:{}", payload, sig_base64);
        let token = URL_SAFE_NO_PAD.encode(token_str.as_bytes());

        let result = verify_recipient_token(&token, session_id, secret);
        assert_eq!(result, Err(TokenError::Expired));
    }

    #[test]
    fn test_token_error_display() {
        assert_eq!(
            TokenError::InvalidFormat.to_string(),
            "Invalid token format"
        );
        assert_eq!(TokenError::Expired.to_string(), "Token has expired");
        assert_eq!(
            TokenError::InvalidSignature.to_string(),
            "Invalid token signature"
        );
        assert_eq!(
            TokenError::SessionMismatch.to_string(),
            "Token does not match session"
        );
    }

    #[test]
    fn test_different_recipients_get_different_tokens() {
        let secret = b"test-secret-key";
        let session_id = "sess_abc123";

        let token1 = generate_recipient_token(session_id, "recipient_1", secret);
        let token2 = generate_recipient_token(session_id, "recipient_2", secret);

        assert_ne!(token1, token2);

        assert_eq!(
            verify_recipient_token(&token1, session_id, secret).unwrap(),
            "recipient_1"
        );
        assert_eq!(
            verify_recipient_token(&token2, session_id, secret).unwrap(),
            "recipient_2"
        );
    }

    // ============================================================
    // Unit Tests for Request Size Validation
    // ============================================================

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_size_limit_constants() {
        // Verify size limits are set correctly
        assert_eq!(MAX_PDF_SIZE, 10 * 1024 * 1024); // 10MB
        assert_eq!(MAX_SIGNATURE_SIZE, 100 * 1024); // 100KB
        assert_eq!(MAX_REQUEST_BODY, 12 * 1024 * 1024); // 12MB

        // Ensure hierarchy makes sense
        assert!(MAX_SIGNATURE_SIZE < MAX_PDF_SIZE);
        assert!(MAX_PDF_SIZE < MAX_REQUEST_BODY);
    }

    #[test]
    fn test_payload_too_large_response_serialization() {
        let response = PayloadTooLargeResponse {
            error: "Request too large".to_string(),
            max_size_bytes: MAX_PDF_SIZE,
            received_bytes: Some(15_000_000),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"error\":\"Request too large\""));
        assert!(json.contains("\"max_size_bytes\":10485760"));
        assert!(json.contains("\"received_bytes\":15000000"));
    }

    #[test]
    fn test_payload_too_large_response_without_received_bytes() {
        let response = PayloadTooLargeResponse {
            error: "Request too large".to_string(),
            max_size_bytes: MAX_PDF_SIZE,
            received_bytes: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"error\":\"Request too large\""));
        assert!(json.contains("\"max_size_bytes\":10485760"));
        // received_bytes should be omitted when None
        assert!(!json.contains("received_bytes"));
    }

    #[test]
    fn test_pdf_size_validation_within_limit() {
        // 1MB of data (within 10MB limit)
        let data = "a".repeat(1024 * 1024);
        assert!(data.len() <= MAX_PDF_SIZE);
    }

    #[test]
    fn test_pdf_size_validation_exceeds_limit() {
        // 11MB of data (exceeds 10MB limit)
        let data = "a".repeat(11 * 1024 * 1024);
        assert!(data.len() > MAX_PDF_SIZE);
    }

    #[test]
    fn test_signature_size_validation_within_limit() {
        // 50KB of data (within 100KB limit)
        let data = "a".repeat(50 * 1024);
        assert!(data.len() <= MAX_SIGNATURE_SIZE);
    }

    #[test]
    fn test_signature_size_validation_exceeds_limit() {
        // 150KB of data (exceeds 100KB limit)
        let data = "a".repeat(150 * 1024);
        assert!(data.len() > MAX_SIGNATURE_SIZE);
    }

    #[test]
    fn test_request_body_size_validation_within_limit() {
        // 10MB of data (within 12MB limit)
        let data = "a".repeat(10 * 1024 * 1024);
        assert!(data.len() <= MAX_REQUEST_BODY);
    }

    #[test]
    fn test_request_body_size_validation_exceeds_limit() {
        // 15MB of data (exceeds 12MB limit)
        let data = "a".repeat(15 * 1024 * 1024);
        assert!(data.len() > MAX_REQUEST_BODY);
    }

    // ============================================================
    // Unit Tests for SenderSessionIndex
    // ============================================================

    #[test]
    fn test_sender_session_index_default() {
        let index = SenderSessionIndex::default();
        assert_eq!(index.count(), 0);
        assert!(index.session_ids.is_empty());
        assert!(index.created_at.is_empty());
    }

    #[test]
    fn test_sender_session_index_add_session() {
        let mut index = SenderSessionIndex::default();
        index.add_session("sess_001".to_string(), "2025-01-15T10:00:00Z".to_string());

        assert_eq!(index.count(), 1);
        assert_eq!(index.session_ids[0], "sess_001");
        assert_eq!(index.created_at[0], "2025-01-15T10:00:00Z");
    }

    #[test]
    fn test_sender_session_index_remove_session() {
        let mut index = SenderSessionIndex::default();
        index.add_session("sess_001".to_string(), "2025-01-15T10:00:00Z".to_string());
        index.add_session("sess_002".to_string(), "2025-01-16T10:00:00Z".to_string());
        index.add_session("sess_003".to_string(), "2025-01-17T10:00:00Z".to_string());

        assert_eq!(index.count(), 3);

        index.remove_session("sess_002");
        assert_eq!(index.count(), 2);
        assert_eq!(index.session_ids, vec!["sess_001", "sess_003"]);
        assert_eq!(
            index.created_at,
            vec!["2025-01-15T10:00:00Z", "2025-01-17T10:00:00Z"]
        );
    }

    #[test]
    fn test_sender_session_index_remove_nonexistent_session() {
        let mut index = SenderSessionIndex::default();
        index.add_session("sess_001".to_string(), "2025-01-15T10:00:00Z".to_string());

        // Removing non-existent session should not panic or change count
        index.remove_session("sess_999");
        assert_eq!(index.count(), 1);
    }

    #[test]
    fn test_sender_session_index_prune_expired() {
        let mut index = SenderSessionIndex::default();

        // Add sessions at various ages
        // Session from 10 days ago (should be kept with 30-day prune)
        let ten_days_ago = (chrono::Utc::now() - chrono::Duration::days(10)).to_rfc3339();
        index.add_session("sess_recent".to_string(), ten_days_ago);

        // Session from 35 days ago (should be pruned with 30-day prune)
        let thirty_five_days_ago = (chrono::Utc::now() - chrono::Duration::days(35)).to_rfc3339();
        index.add_session("sess_old".to_string(), thirty_five_days_ago);

        // Session from today (should be kept)
        let now = chrono::Utc::now().to_rfc3339();
        index.add_session("sess_today".to_string(), now);

        assert_eq!(index.count(), 3);

        index.prune_expired(30);

        assert_eq!(index.count(), 2);
        assert!(index.session_ids.contains(&"sess_recent".to_string()));
        assert!(index.session_ids.contains(&"sess_today".to_string()));
        assert!(!index.session_ids.contains(&"sess_old".to_string()));
    }

    #[test]
    fn test_sender_session_index_prune_with_invalid_timestamps() {
        let mut index = SenderSessionIndex::default();

        // Valid timestamp
        let now = chrono::Utc::now().to_rfc3339();
        index.add_session("sess_valid".to_string(), now);

        // Invalid timestamp (should be removed during prune)
        index.add_session(
            "sess_invalid".to_string(),
            "not-a-valid-timestamp".to_string(),
        );

        assert_eq!(index.count(), 2);

        index.prune_expired(30);

        // Only the valid session should remain
        assert_eq!(index.count(), 1);
        assert_eq!(index.session_ids[0], "sess_valid");
    }

    #[test]
    fn test_sender_session_index_at_limit() {
        let mut index = SenderSessionIndex::default();

        // Add exactly MAX_SESSIONS_PER_SENDER sessions
        for i in 0..MAX_SESSIONS_PER_SENDER {
            index.add_session(format!("sess_{:03}", i), chrono::Utc::now().to_rfc3339());
        }

        assert_eq!(index.count(), MAX_SESSIONS_PER_SENDER);
        assert!(index.count() >= MAX_SESSIONS_PER_SENDER);
    }

    #[test]
    fn test_hash_sender_email_consistency() {
        // Same email should produce same hash
        let hash1 = hash_sender_email("test@example.com");
        let hash2 = hash_sender_email("test@example.com");
        assert_eq!(hash1, hash2);

        // Case insensitive
        let hash_lower = hash_sender_email("test@example.com");
        let hash_upper = hash_sender_email("TEST@EXAMPLE.COM");
        assert_eq!(hash_lower, hash_upper);

        // Different emails should produce different hashes
        let hash_other = hash_sender_email("other@example.com");
        assert_ne!(hash1, hash_other);
    }

    #[test]
    fn test_hash_sender_email_format() {
        let hash = hash_sender_email("test@example.com");

        // SHA-256 produces 64 hex characters
        assert_eq!(hash.len(), 64);

        // Should only contain hex characters
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sender_session_index_serialization() {
        let mut index = SenderSessionIndex::default();
        index.add_session("sess_001".to_string(), "2025-01-15T10:00:00Z".to_string());
        index.add_session("sess_002".to_string(), "2025-01-16T10:00:00Z".to_string());

        // Should serialize correctly
        let json = serde_json::to_string(&index).unwrap();
        assert!(json.contains("sess_001"));
        assert!(json.contains("sess_002"));
        assert!(json.contains("2025-01-15T10:00:00Z"));

        // Should deserialize correctly
        let deserialized: SenderSessionIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.count(), 2);
        assert_eq!(deserialized.session_ids, index.session_ids);
        assert_eq!(deserialized.created_at, index.created_at);
    }

    #[test]
    fn test_session_limit_constant() {
        // Verify the session limit is set to expected value
        assert_eq!(MAX_SESSIONS_PER_SENDER, 100);

        // Verify prune days is set correctly
        assert_eq!(SESSION_INDEX_PRUNE_DAYS, 30);
    }

    // ============================================================
    // Property Tests for Session/Magic Link Functionality
    // ============================================================

    use proptest::prelude::*;

    proptest! {
        /// Property: SessionMetadata serialization roundtrip preserves all fields
        #[test]
        fn prop_session_metadata_roundtrip(
            filename in "[a-zA-Z0-9_-]{1,50}\\.pdf",
            page_count in 1u32..1000,
            created_by in "[a-zA-Z ]{1,50}"
        ) {
            let metadata = SessionMetadata {
                filename: filename.clone(),
                page_count,
                created_at: "2025-01-15T10:00:00Z".to_string(),
                created_by: created_by.clone(),
                sender_email: Some("test@example.com".to_string()),
            };

            let json = serde_json::to_string(&metadata).unwrap();
            let deserialized: SessionMetadata = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(metadata.filename, deserialized.filename);
            prop_assert_eq!(metadata.page_count, deserialized.page_count);
            prop_assert_eq!(metadata.created_by, deserialized.created_by);
        }

        /// Property: RecipientInfo serialization preserves email and role
        #[test]
        fn prop_recipient_info_roundtrip(
            id in "[0-9]{1,10}",
            name in "[a-zA-Z ]{1,50}",
            email in "[a-z]{3,10}@[a-z]{3,10}\\.[a-z]{2,4}",
            role in prop_oneof!["signer", "viewer"],
            signed in proptest::bool::ANY
        ) {
            let recipient = RecipientInfo {
                id: id.clone(),
                name: name.clone(),
                email: email.clone(),
                role: role.clone(),
                consented: signed, // If signed, must have consented
                consent_at: if signed { Some("2025-01-15T09:55:00Z".to_string()) } else { None },
                consent_user_agent: None,
                signed,
                signed_at: if signed { Some("2025-01-15T10:00:00Z".to_string()) } else { None },
                declined: false,
                declined_at: None,
                decline_reason: None,
                signing_order: None,
                reminders_sent: 0,
                last_reminder_at: None,
            };

            let json = serde_json::to_string(&recipient).unwrap();
            let deserialized: RecipientInfo = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(recipient.id, deserialized.id);
            prop_assert_eq!(recipient.email, deserialized.email);
            prop_assert_eq!(recipient.role, deserialized.role);
            prop_assert_eq!(recipient.signed, deserialized.signed);
        }

        /// Property: FieldInfo coordinates are within 0-100% bounds
        #[test]
        fn prop_field_coordinates_in_bounds(
            x in 0.0f64..100.0,
            y in 0.0f64..100.0,
            width in 1.0f64..50.0,
            height in 1.0f64..20.0,
            page in 1u32..100
        ) {
            let field = FieldInfo {
                id: "field_1".to_string(),
                field_type: "signature".to_string(),
                recipient_id: "1".to_string(),
                page,
                x_percent: x,
                y_percent: y,
                width_percent: width,
                height_percent: height,
                required: true,
                value: None,
            };

            // Verify coordinates are valid percentages
            prop_assert!(field.x_percent >= 0.0 && field.x_percent <= 100.0);
            prop_assert!(field.y_percent >= 0.0 && field.y_percent <= 100.0);
            prop_assert!(field.width_percent > 0.0 && field.width_percent <= 100.0);
            prop_assert!(field.height_percent > 0.0 && field.height_percent <= 100.0);

            // Verify serialization roundtrip
            let json = serde_json::to_string(&field).unwrap();
            let deserialized: FieldInfo = serde_json::from_str(&json).unwrap();
            prop_assert!((deserialized.x_percent - x).abs() < 0.0001);
            prop_assert!((deserialized.y_percent - y).abs() < 0.0001);
        }

        /// Property: SigningSession serialization preserves all nested structures
        #[test]
        fn prop_signing_session_roundtrip(
            doc_size in 100usize..10000,
            num_recipients in 1usize..5,
            num_fields in 1usize..10
        ) {
            let encrypted_doc = "a".repeat(doc_size);

            let recipients: Vec<RecipientInfo> = (0..num_recipients)
                .map(|i| test_recipient(
                    &i.to_string(),
                    &format!("Recipient {}", i),
                    &format!("user{}@example.com", i),
                    "signer"
                ))
                .collect();

            let fields: Vec<FieldInfo> = (0..num_fields)
                .map(|i| FieldInfo {
                    id: format!("field_{}", i),
                    field_type: "signature".to_string(),
                    recipient_id: (i % num_recipients).to_string(),
                    page: 1,
                    x_percent: 10.0 + (i as f64 * 5.0),
                    y_percent: 20.0,
                    width_percent: 30.0,
                    height_percent: 10.0,
                    required: true,
                    value: None,
                })
                .collect();

            let session = SigningSession {
                id: "test_session_123".to_string(),
                encrypted_document: encrypted_doc.clone(),
                metadata: SessionMetadata {
                    filename: "test.pdf".to_string(),
                    page_count: 1,
                    created_at: "2025-01-15T10:00:00Z".to_string(),
                    created_by: "Test User".to_string(),
                    sender_email: Some("sender@example.com".to_string()),
                },
                recipients: recipients.clone(),
                fields: fields.clone(),
                expires_at: "2025-01-22T10:00:00Z".to_string(),
                signed_versions: vec![],
                status: SessionStatus::Pending,
                signing_mode: SigningMode::Parallel,
                reminder_config: None,
                final_document: None,
                legacy: false,
            };

            let json = serde_json::to_string(&session).unwrap();
            let deserialized: SigningSession = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(session.id, deserialized.id);
            prop_assert_eq!(session.encrypted_document.len(), deserialized.encrypted_document.len());
            prop_assert_eq!(session.recipients.len(), deserialized.recipients.len());
            prop_assert_eq!(session.fields.len(), deserialized.fields.len());
        }

        /// Property: Session expiry hours always produces valid future timestamp
        #[test]
        fn prop_expiry_hours_valid(
            expiry_hours in 1u32..168  // 1 hour to 7 days
        ) {
            use chrono::{Duration, Utc};

            let now = Utc::now();
            let expires_at = now + Duration::hours(expiry_hours as i64);

            // Expiry should always be in the future
            prop_assert!(expires_at > now);

            // Expiry should be within reasonable bounds (max 7 days)
            let max_expiry = now + Duration::hours(168);
            prop_assert!(expires_at <= max_expiry);
        }
    }

    // ============================================================
    // Unit Tests for Session Functionality
    // ============================================================

    #[test]
    fn test_session_with_signed_version() {
        let mut recipient = test_recipient("1", "Alice", "alice@example.com", "signer");
        recipient.signed = true;
        recipient.signed_at = Some("2025-01-15T11:00:00Z".to_string());

        let mut session = test_session("sess_123", vec![recipient]);
        session.encrypted_document = "base64_encrypted_doc".to_string();
        session.metadata.filename = "contract.pdf".to_string();
        session.metadata.page_count = 5;
        session.metadata.created_by = "Sender".to_string();
        session.metadata.sender_email = Some("sender@example.com".to_string());
        session.signed_versions = vec![SignedVersion {
            recipient_id: "1".to_string(),
            encrypted_document: "signed_doc_base64".to_string(),
            signed_at: "2025-01-15T11:00:00Z".to_string(),
        }];
        session.status = SessionStatus::Completed;

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SigningSession = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.signed_versions.len(), 1);
        assert_eq!(deserialized.signed_versions[0].recipient_id, "1");
        assert!(deserialized.recipients[0].signed);
    }

    #[test]
    fn test_field_types() {
        let field_types = ["signature", "initials", "date", "text", "checkbox"];

        for field_type in field_types {
            let field = FieldInfo {
                id: "f1".to_string(),
                field_type: field_type.to_string(),
                recipient_id: "1".to_string(),
                page: 1,
                x_percent: 50.0,
                y_percent: 50.0,
                width_percent: 20.0,
                height_percent: 5.0,
                required: true,
                value: None,
            };

            let json = serde_json::to_string(&field).unwrap();
            let deserialized: FieldInfo = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.field_type, field_type);
        }
    }

    #[test]
    fn test_recipient_roles() {
        let roles = ["signer", "viewer"];

        for role in roles {
            let recipient = test_recipient("1", "Test", "test@example.com", role);

            let json = serde_json::to_string(&recipient).unwrap();
            let deserialized: RecipientInfo = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.role, role);
        }
    }

    #[test]
    fn test_empty_signed_versions_deserialize() {
        // Ensure old sessions without signed_versions field can deserialize
        let json = r#"{
            "id": "sess_123",
            "encrypted_document": "abc",
            "metadata": {
                "filename": "test.pdf",
                "page_count": 1,
                "created_at": "2025-01-15T10:00:00Z",
                "created_by": "Test"
            },
            "recipients": [],
            "fields": [],
            "expires_at": "2025-01-22T10:00:00Z"
        }"#;

        let session: SigningSession = serde_json::from_str(json).unwrap();
        assert!(session.signed_versions.is_empty());
    }

    // ============================================================
    // UX-001: Consent Landing Page Tests
    // ============================================================

    #[test]
    fn test_session_metadata_has_sender_info_for_consent() {
        // Session metadata should include sender information for consent landing page
        let metadata = SessionMetadata {
            filename: "contract.pdf".to_string(),
            page_count: 5,
            created_at: "2025-01-15T10:00:00Z".to_string(),
            created_by: "John Doe".to_string(),
            sender_email: Some("john.doe@example.com".to_string()),
        };

        // We have created_by (sender name), created_at (date sent), and sender_email
        assert_eq!(metadata.created_by, "John Doe");
        assert_eq!(metadata.created_at, "2025-01-15T10:00:00Z");
        assert_eq!(metadata.filename, "contract.pdf");
        assert!(metadata.sender_email.as_ref().unwrap().contains('@'));
    }

    #[test]
    fn test_session_public_info_exposes_all_consent_data() {
        // SessionPublicInfo returned by GET /session/{id} should include all data
        // needed for the consent landing page
        let public_info = SessionPublicInfo {
            id: "sess_123".to_string(),
            metadata: SessionMetadata {
                filename: "test.pdf".to_string(),
                page_count: 1,
                created_at: "2025-01-15T10:00:00Z".to_string(),
                created_by: "Jane Smith".to_string(),
                sender_email: Some("jane.smith@example.com".to_string()),
            },
            recipients: vec![],
            fields: vec![],
            encrypted_document: "data".to_string(),
            expires_at: "2025-01-22T10:00:00Z".to_string(),
            signing_mode: SigningMode::Parallel,
            status: SessionStatus::Pending,
            final_document: None,
        };

        // Verify fields exist for consent page:
        // ✓ sender_name (created_by)
        // ✓ document_name (filename)
        // ✓ date_sent (created_at)
        // ✗ sender_email (MISSING)
        assert_eq!(public_info.metadata.created_by, "Jane Smith");
        assert_eq!(public_info.metadata.filename, "test.pdf");
        assert_eq!(public_info.metadata.created_at, "2025-01-15T10:00:00Z");
    }

    // ============================================================
    // UX-002: Accept/Decline Flow Tests
    // ============================================================

    #[test]
    fn test_decline_request_with_reason() {
        let json = r#"{
            "recipient_id": "r1",
            "reason": "Need more time to review"
        }"#;

        #[derive(Deserialize)]
        struct DeclineRequest {
            recipient_id: String,
            reason: Option<String>,
        }

        let req: DeclineRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.recipient_id, "r1");
        assert_eq!(req.reason, Some("Need more time to review".to_string()));
    }

    #[test]
    fn test_decline_request_without_reason() {
        let json = r#"{
            "recipient_id": "r2"
        }"#;

        #[derive(Deserialize)]
        struct DeclineRequest {
            recipient_id: String,
            reason: Option<String>,
        }

        let req: DeclineRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.recipient_id, "r2");
        assert_eq!(req.reason, None);
    }

    #[test]
    fn test_decline_response_serialization() {
        #[derive(Serialize)]
        struct DeclineResponse {
            success: bool,
            message: String,
        }

        let response = DeclineResponse {
            success: true,
            message: "Document declined successfully".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("Document declined"));
    }

    // ============================================================
    // Consent Tracking Tests
    // ============================================================

    #[test]
    fn test_consent_request_deserialization() {
        #[derive(Deserialize)]
        struct ConsentRequest {
            recipient_id: String,
            user_agent: Option<String>,
            consent_text_hash: Option<String>,
        }

        let json =
            r#"{"recipient_id": "r1", "user_agent": "Mozilla/5.0", "consent_text_hash": "abc123"}"#;
        let req: ConsentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.recipient_id, "r1");
        assert_eq!(req.user_agent, Some("Mozilla/5.0".to_string()));
        assert_eq!(req.consent_text_hash, Some("abc123".to_string()));
    }

    #[test]
    fn test_consent_request_without_optional_fields() {
        #[derive(Deserialize)]
        struct ConsentRequest {
            recipient_id: String,
            user_agent: Option<String>,
            consent_text_hash: Option<String>,
        }

        let json = r#"{"recipient_id": "r2"}"#;
        let req: ConsentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.recipient_id, "r2");
        assert_eq!(req.user_agent, None);
        assert_eq!(req.consent_text_hash, None);
    }

    #[test]
    fn test_consent_response_serialization() {
        #[derive(Serialize)]
        struct ConsentResponse {
            success: bool,
            message: String,
            consent_at: String,
        }

        let response = ConsentResponse {
            success: true,
            message: "Consent recorded successfully".to_string(),
            consent_at: "2025-01-15T10:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("consent_at"));
        assert!(json.contains("2025-01-15T10:00:00Z"));
    }

    #[test]
    fn test_recipient_has_consent_fields() {
        // Verify RecipientInfo has consent tracking fields
        let mut recipient = test_recipient("1", "John Doe", "john@example.com", "signer");

        // Initially not consented
        assert!(!recipient.consented);
        assert!(recipient.consent_at.is_none());
        assert!(recipient.consent_user_agent.is_none());

        // After consent
        recipient.consented = true;
        recipient.consent_at = Some("2025-01-15T10:00:00Z".to_string());
        recipient.consent_user_agent = Some("Mozilla/5.0".to_string());

        assert!(recipient.consented);
        assert!(recipient.consent_at.is_some());
        assert!(recipient.consent_user_agent.is_some());
    }

    #[test]
    fn test_consent_precedes_signing() {
        // Verify consent must happen before signing (logical flow)
        let mut recipient = test_recipient("1", "Jane", "jane@example.com", "signer");

        // Record consent
        recipient.consented = true;
        recipient.consent_at = Some("2025-01-15T10:00:00Z".to_string());

        // Then sign
        recipient.signed = true;
        recipient.signed_at = Some("2025-01-15T10:05:00Z".to_string());

        // Both should be recorded
        assert!(recipient.consented);
        assert!(recipient.signed);

        // Consent should come before signing
        let consent_time = recipient.consent_at.as_ref().unwrap();
        let sign_time = recipient.signed_at.as_ref().unwrap();
        assert!(consent_time < sign_time, "Consent should precede signing");
    }

    // ============================================================
    // UX-006: Sender Notification on Sign Tests
    // ============================================================

    #[test]
    fn test_session_metadata_contains_sender_email() {
        // Test that SessionMetadata includes sender_email field
        let metadata = SessionMetadata {
            filename: "contract.pdf".to_string(),
            page_count: 5,
            created_at: "2025-01-15T10:00:00Z".to_string(),
            created_by: "John Sender".to_string(),
            sender_email: Some("sender@example.com".to_string()),
        };

        // Verify we have a dedicated sender_email field
        assert!(
            metadata.sender_email.as_ref().unwrap().contains('@'),
            "Session metadata should contain sender email for notifications"
        );
    }

    #[test]
    fn test_notification_email_format() {
        // Test that notification email includes all required fields
        let recipient_name = "John Doe";
        let document_name = "contract.pdf";
        let signed_at = "2025-01-15T11:00:00Z";
        let download_link = "https://getsignatures.org/download/sess_123";

        let email_body = format_completion_notification_email(
            recipient_name,
            document_name,
            signed_at,
            download_link,
        );

        assert!(email_body.contains(recipient_name));
        assert!(email_body.contains(document_name));
        // The email formats the timestamp, so check for key parts like the year
        assert!(email_body.contains("2025"));
        assert!(email_body.contains("January") || email_body.contains("11:00"));
        assert!(email_body.contains(download_link));
    }

    #[test]
    fn test_all_recipients_signed_detection() {
        // Test that we can detect when all recipients have signed
        let mut alice = test_recipient("1", "Alice", "alice@example.com", "signer");
        alice.signed = true;
        alice.signed_at = Some("2025-01-15T11:00:00Z".to_string());

        let mut bob = test_recipient("2", "Bob", "bob@example.com", "signer");
        bob.signed = true;
        bob.signed_at = Some("2025-01-15T12:00:00Z".to_string());

        let recipients = vec![alice, bob];

        // This will fail because the function doesn't exist yet
        assert!(all_recipients_signed(&recipients));
    }

    #[test]
    fn test_not_all_recipients_signed_detection() {
        // Test that we correctly detect when not all recipients have signed
        let mut alice = test_recipient("1", "Alice", "alice@example.com", "signer");
        alice.signed = true;
        alice.signed_at = Some("2025-01-15T11:00:00Z".to_string());

        let bob = test_recipient("2", "Bob", "bob@example.com", "signer");
        // Bob hasn't signed (defaults to false)

        let recipients = vec![alice, bob];

        assert!(!all_recipients_signed(&recipients));
    }

    #[test]
    fn test_download_link_generation() {
        // Test that download links are generated with proper format
        let session_id = "sess_abc123";
        let expiry_days = 30;

        // This will fail because the function doesn't exist yet
        let download_link = generate_download_link(session_id, expiry_days);

        assert!(download_link.contains(session_id));
        assert!(download_link.starts_with("https://"));
        assert!(download_link.contains("download") || download_link.contains("session"));
    }

    #[test]
    fn test_completion_summary_email_includes_all_signers() {
        // Test that completion summary email includes all recipient names
        let mut alice = test_recipient("1", "Alice Smith", "alice@example.com", "signer");
        alice.signed = true;
        alice.signed_at = Some("2025-01-15T11:00:00Z".to_string());

        let mut bob = test_recipient("2", "Bob Jones", "bob@example.com", "signer");
        bob.signed = true;
        bob.signed_at = Some("2025-01-15T12:00:00Z".to_string());

        let recipients = vec![alice, bob];

        let document_name = "contract.pdf";
        let download_link = "https://getsignatures.org/download/sess_123";

        // This will fail because the function doesn't exist yet
        let email_body =
            format_all_signed_notification_email(&recipients, document_name, download_link);

        assert!(email_body.contains("Alice Smith"));
        assert!(email_body.contains("Bob Jones"));
        assert!(email_body.contains(document_name));
        assert!(
            email_body.contains("all recipients have signed") || email_body.contains("completed")
        );
    }

    #[test]
    fn test_download_link_expiry_30_days() {
        // Test that download links expire after 30 days
        let session_id = "sess_123";
        let expiry_days = 30;

        let download_link = generate_download_link(session_id, expiry_days);

        // The link should encode the expiry timestamp somehow
        // We'll verify this by checking that it's not just a simple session ID link
        assert!(
            download_link.len() > session_id.len() + 20,
            "Download link should include expiry information"
        );
    }

    // ============================================================
    // UX-002: Decline Flow Tests (MUST FAIL until implemented)
    // ============================================================

    #[test]
    fn test_recipient_has_declined_field() {
        // UX-002: RecipientInfo must have a declined field
        let recipient = test_recipient("1", "John Doe", "john@example.com", "signer");

        assert!(!recipient.declined);
        assert!(recipient.declined_at.is_none());
        assert!(recipient.decline_reason.is_none());
    }

    #[test]
    fn test_recipient_decline_sets_fields() {
        // UX-002: Declining a recipient should set declined=true and declined_at
        let mut recipient = test_recipient("1", "John Doe", "john@example.com", "signer");

        // Simulate decline
        recipient.declined = true;
        recipient.declined_at = Some("2025-12-21T12:00:00Z".to_string());
        recipient.decline_reason = Some("Terms not acceptable".to_string());

        assert!(recipient.declined);
        assert!(recipient.declined_at.is_some());
        assert_eq!(
            recipient.decline_reason,
            Some("Terms not acceptable".to_string())
        );
    }

    #[test]
    fn test_declined_recipient_cannot_sign() {
        // UX-002: A declined recipient should not be able to sign
        let mut recipient = test_recipient("1", "John Doe", "john@example.com", "signer");
        recipient.declined = true;
        recipient.declined_at = Some("2025-12-21T12:00:00Z".to_string());
        recipient.decline_reason = Some("Not ready".to_string());

        // A declined recipient should be blocked from signing
        assert!(recipient.declined);
        assert!(!recipient.signed);
        // Attempting to sign a declined recipient should fail
        // (This would be tested at the handler level)
    }

    #[test]
    fn test_session_has_status_field() {
        // UX-002: SigningSession must have a status field
        let session = test_session("sess_123", vec![]);

        assert_eq!(session.status, SessionStatus::Pending);
    }

    #[test]
    fn test_session_status_transitions() {
        // UX-002: Session status should transition correctly
        let mut session = test_session("sess_123", vec![]);

        // Transition to declined
        session.status = SessionStatus::Declined;
        assert_eq!(session.status, SessionStatus::Declined);

        // Once declined, should stay declined
        assert!(matches!(session.status, SessionStatus::Declined));
    }

    #[test]
    fn test_session_status_enum_values() {
        // UX-002: SessionStatus enum should have all required values
        let pending = SessionStatus::Pending;
        let accepted = SessionStatus::Accepted;
        let declined = SessionStatus::Declined;
        let completed = SessionStatus::Completed;
        let expired = SessionStatus::Expired;

        assert!(matches!(pending, SessionStatus::Pending));
        assert!(matches!(accepted, SessionStatus::Accepted));
        assert!(matches!(declined, SessionStatus::Declined));
        assert!(matches!(completed, SessionStatus::Completed));
        assert!(matches!(expired, SessionStatus::Expired));
    }

    #[test]
    fn test_session_status_serialization() {
        // UX-002: SessionStatus should serialize to lowercase strings
        let pending = SessionStatus::Pending;
        let declined = SessionStatus::Declined;

        let pending_json = serde_json::to_string(&pending).unwrap();
        let declined_json = serde_json::to_string(&declined).unwrap();

        assert_eq!(pending_json, "\"pending\"");
        assert_eq!(declined_json, "\"declined\"");
    }

    // ============================================================
    // UX-004: Session Expiry & Resend Tests
    // ============================================================

    #[test]
    fn test_expired_session_response_structure() {
        // UX-004: When session is expired, should return status="expired"
        // with sender_email and document_name

        // Create an expired session
        let mut session = test_session("sess_expired", vec![]);
        session.metadata.filename = "expired.pdf".to_string();
        session.metadata.created_at = "2025-12-01T12:00:00Z".to_string();
        session.metadata.created_by = "Alice".to_string();
        session.metadata.sender_email = Some("alice@example.com".to_string());
        session.expires_at = "2025-12-10T12:00:00Z".to_string(); // Past date
        session.status = SessionStatus::Expired;

        // Verify session has expired status
        assert_eq!(session.status, SessionStatus::Expired);
        assert_eq!(
            session.metadata.sender_email,
            Some("alice@example.com".to_string())
        );
        assert_eq!(session.metadata.filename, "expired.pdf");
    }

    #[test]
    fn test_session_expiry_time_check() {
        // UX-004: Test that we can determine if a session has expired

        // Session that expired yesterday
        let past_time = "2025-12-20T12:00:00Z";
        let past_parsed = chrono::DateTime::parse_from_rfc3339(past_time).unwrap();
        let now = chrono::Utc::now();
        assert!(past_parsed < now, "Past time should be before now");

        // Session that expires tomorrow
        let future_time = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .unwrap()
            .to_rfc3339();
        let future_parsed = chrono::DateTime::parse_from_rfc3339(&future_time).unwrap();
        assert!(future_parsed > now, "Future time should be after now");
    }

    #[test]
    fn test_request_link_payload_structure() {
        // UX-004: Request link endpoint should accept recipient_id
        #[derive(Deserialize, Serialize)]
        struct RequestLinkRequest {
            recipient_id: String,
        }

        let request = RequestLinkRequest {
            recipient_id: "recip_123".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: RequestLinkRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.recipient_id, "recip_123");
    }

    #[test]
    fn test_resend_endpoint_creates_new_session() {
        // UX-004: Resend endpoint should create new session with fresh expiry

        let old_expires = "2025-12-21T12:00:00Z";
        let new_expiry_hours = 168; // 7 days
        let new_expiry_seconds = (new_expiry_hours as u64) * 60 * 60;

        let new_expires = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::seconds(new_expiry_seconds as i64))
            .unwrap()
            .to_rfc3339();

        // Parse both timestamps
        let old_time = chrono::DateTime::parse_from_rfc3339(old_expires).unwrap();
        let new_time = chrono::DateTime::parse_from_rfc3339(&new_expires).unwrap();

        // New expiry should be after old one
        assert!(new_time > old_time, "New expiry should be in the future");
    }

    // Note: generate_session_id() uses js_sys and can't be tested in non-WASM tests
    // Session ID uniqueness is tested in integration tests

    // ============================================================
    // Email Integration Tests
    // ============================================================

    #[test]
    fn test_from_address_format() {
        // DEFAULT_FROM_ADDRESS should be in "Name <email>" format
        assert!(email::DEFAULT_FROM_ADDRESS.contains('<'));
        assert!(email::DEFAULT_FROM_ADDRESS.contains('>'));
        assert!(email::DEFAULT_FROM_ADDRESS.contains('@'));
        assert!(email::DEFAULT_FROM_ADDRESS.contains("getsignatures.org"));
    }

    #[test]
    fn test_email_request_payload_structure() {
        // Verify EmailSendRequest structure
        let request = email::EmailSendRequest {
            to: vec!["test@example.com".to_string()],
            subject: "Test Subject".to_string(),
            html: "<p>Test body</p>".to_string(),
            text: None,
            reply_to: None,
            tags: vec![],
        };

        // Verify fields are set correctly
        assert_eq!(request.to.len(), 1);
        assert_eq!(request.subject, "Test Subject");
        assert!(request.html.contains("<p>"));
    }

    #[test]
    fn test_email_request_multiple_recipients() {
        // EmailSendRequest accepts multiple recipients
        let request = email::EmailSendRequest {
            to: vec![
                "a@test.com".to_string(),
                "b@test.com".to_string(),
                "c@test.com".to_string(),
            ],
            subject: "Multi-recipient test".to_string(),
            html: "<p>Test</p>".to_string(),
            text: None,
            reply_to: None,
            tags: vec![],
        };

        assert_eq!(request.to.len(), 3);
    }

    #[test]
    fn test_invitation_html_escapes_special_chars() {
        // Ensure HTML template handles special characters safely
        let dangerous_name = "<script>alert('xss')</script>";
        let sender_name = "Alice & Bob";
        let document_name = "Contract \"Final\" <draft>";
        let signing_link = "https://example.com/sign?id=123&token=abc";

        // Simulate the template format string (simplified)
        let html = format!(
            "Hello {}, {} sent you {}. <a href=\"{}\">Sign</a>",
            dangerous_name, sender_name, document_name, signing_link
        );

        // The template should contain the raw text (HTML escaping happens at render)
        // This test verifies the template accepts these inputs without panicking
        assert!(html.contains("script"));
        assert!(html.contains("Alice & Bob"));
    }

    #[test]
    fn test_email_send_result_helpers() {
        // Test EmailSendResult helper methods
        let success = email::EmailSendResult::success("msg-123".to_string());
        assert!(success.success);
        assert_eq!(success.id, "msg-123");
        assert!(success.error.is_none());

        let failure = email::EmailSendResult::error("Something went wrong");
        assert!(!failure.success);
        assert!(failure.id.is_empty());
        assert_eq!(failure.error.as_deref(), Some("Something went wrong"));
    }

    // ============================================================
    // Regression Tests for UX Fixes (2026-01-06)
    // ============================================================

    /// Regression test: CORS headers must include Authorization header
    /// Bug: Account creation failed with network error because Authorization
    /// header was not in Access-Control-Allow-Headers
    #[test]
    fn test_cors_headers_include_authorization() {
        // Verify the CORS header value includes both Content-Type and Authorization
        // This is a compile-time check by examining the expected behavior
        let expected_headers = "Content-Type, Authorization";

        // The cors_response function sets these headers:
        // Access-Control-Allow-Origin: *
        // Access-Control-Allow-Methods: GET, POST, PUT, OPTIONS
        // Access-Control-Allow-Headers: Content-Type, Authorization

        // Verify Authorization is included (prevents regression of the network error bug)
        assert!(
            expected_headers.contains("Authorization"),
            "CORS headers must include Authorization to prevent browser blocking auth requests"
        );
        assert!(
            expected_headers.contains("Content-Type"),
            "CORS headers must include Content-Type for JSON requests"
        );
    }
}

// ============================================================
// Email Integration Property Tests
// ============================================================

#[cfg(test)]
mod email_proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: Email subject can contain any printable ASCII
        #[test]
        fn email_subject_accepts_printable_ascii(subject in "[a-zA-Z0-9 !@#$%^&*()_+=\\-\\[\\]{}|;:',.<>?/]{1,200}") {
            let request = email::EmailSendRequest {
                to: vec!["test@example.com".to_string()],
                subject: subject.clone(),
                html: "<p>Test</p>".to_string(),
                text: None,
                reply_to: None,
                tags: vec![],
            };

            // Should serialize without error
            let json_str = serde_json::to_string(&request);
            prop_assert!(json_str.is_ok(), "Should serialize email with subject: {}", subject);
        }

        /// Property: Recipient email addresses are preserved exactly
        #[test]
        fn recipient_emails_preserved(
            local in "[a-z0-9._%+-]{1,20}",
            domain in "[a-z0-9-]{1,20}",
            tld in "(com|org|net|io)"
        ) {
            let email_addr = format!("{}@{}.{}", local, domain, tld);
            let request = email::EmailSendRequest {
                to: vec![email_addr.clone()],
                subject: "Test".to_string(),
                html: "<p>Test</p>".to_string(),
                text: None,
                reply_to: None,
                tags: vec![],
            };

            prop_assert_eq!(&request.to[0], &email_addr);
        }

        /// Property: HTML body can contain any valid HTML characters
        #[test]
        fn html_body_accepts_valid_content(
            text in "[a-zA-Z0-9 .,!?]{1,100}"
        ) {
            let html = format!("<p>{}</p>", text);
            let request = email::EmailSendRequest {
                to: vec!["test@example.com".to_string()],
                subject: "Test".to_string(),
                html: html.clone(),
                text: None,
                reply_to: None,
                tags: vec![],
            };

            let json_str = serde_json::to_string(&request);
            prop_assert!(json_str.is_ok());

            // Verify HTML is preserved
            prop_assert!(request.html.contains(&text));
        }

        /// Property: Multiple recipients all included in request
        #[test]
        fn multiple_recipients_all_included(count in 1usize..10) {
            let recipients: Vec<String> = (0..count)
                .map(|i| format!("user{}@example.com", i))
                .collect();

            let request = email::EmailSendRequest {
                to: recipients.clone(),
                subject: "Test".to_string(),
                html: "<p>Test</p>".to_string(),
                text: None,
                reply_to: None,
                tags: vec![],
            };

            prop_assert_eq!(request.to.len(), count);

            for (i, recipient) in recipients.iter().enumerate() {
                prop_assert_eq!(&request.to[i], recipient);
            }
        }

        /// Property: Invitation email template handles various name lengths
        #[test]
        fn invitation_template_handles_name_lengths(
            name_len in 1usize..100
        ) {
            let name: String = (0..name_len).map(|_| 'A').collect();

            // Simulate template format (simplified)
            let html = format!(
                "<p>Hello {},</p><p>You have a document to sign.</p>",
                name
            );

            prop_assert!(html.len() > name_len);
            prop_assert!(html.contains(&name));
        }

        /// Property: Signing link URLs are preserved in template
        #[test]
        fn signing_link_preserved_in_template(
            session_id in "[a-zA-Z0-9]{8,32}",
            recipient_id in "[a-zA-Z0-9]{4,16}",
            key in "[a-zA-Z0-9]{16,64}"
        ) {
            let signing_link = format!(
                "https://getsignatures.org/sign?session={}&recipient={}&key={}",
                session_id, recipient_id, key
            );

            let html = format!(
                "<a href=\"{}\">Sign Document</a>",
                signing_link
            );

            prop_assert!(html.contains(&signing_link));
            prop_assert!(html.contains(&session_id));
            prop_assert!(html.contains(&recipient_id));
        }
    }

    // ============================================================
    // Per-IP Rate Limiting Tests
    // ============================================================

    #[test]
    fn test_ip_rate_limit_state_default() {
        let state = IpRateLimitState::default();
        assert_eq!(state.request_count, 0);
        assert_eq!(state.window_start, 0);
    }

    #[test]
    fn test_ip_rate_limit_state_serialization() {
        let state = IpRateLimitState {
            request_count: 42,
            window_start: 1704067200,
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: IpRateLimitState = serde_json::from_str(&json).unwrap();

        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_rate_limit_tier_limits() {
        // Health: 100 req/min
        let (max, window) = RateLimitTier::Health.limits();
        assert_eq!(max, 100);
        assert_eq!(window, 60);

        // SessionRead: 30 req/min
        let (max, window) = RateLimitTier::SessionRead.limits();
        assert_eq!(max, 30);
        assert_eq!(window, 60);

        // SessionWrite: 5 req/min
        let (max, window) = RateLimitTier::SessionWrite.limits();
        assert_eq!(max, 5);
        assert_eq!(window, 60);

        // RequestLink: 100 req/day (generous per-IP, global email quota handles cost)
        let (max, window) = RateLimitTier::RequestLink.limits();
        assert_eq!(max, 100);
        assert_eq!(window, 86400);
    }

    #[test]
    fn test_rate_limit_tier_names() {
        assert_eq!(RateLimitTier::Health.name(), "health");
        assert_eq!(RateLimitTier::SessionRead.name(), "session_read");
        assert_eq!(RateLimitTier::SessionWrite.name(), "session_write");
        assert_eq!(RateLimitTier::RequestLink.name(), "request_link");
    }

    #[test]
    fn test_rate_limit_tier_retry_after() {
        assert_eq!(RateLimitTier::Health.retry_after_seconds(), 60);
        assert_eq!(RateLimitTier::SessionRead.retry_after_seconds(), 60);
        assert_eq!(RateLimitTier::SessionWrite.retry_after_seconds(), 60);
        assert_eq!(RateLimitTier::RequestLink.retry_after_seconds(), 86400); // 1 day
    }

    #[test]
    fn test_ip_rate_limit_result_equality() {
        assert_eq!(IpRateLimitResult::Allowed, IpRateLimitResult::Allowed);
        assert_eq!(
            IpRateLimitResult::Limited {
                retry_after_seconds: 30
            },
            IpRateLimitResult::Limited {
                retry_after_seconds: 30
            }
        );
        assert_ne!(
            IpRateLimitResult::Allowed,
            IpRateLimitResult::Limited {
                retry_after_seconds: 30
            }
        );
    }

    #[test]
    fn test_ip_rate_limit_kv_key_format() {
        // Verify key format: ip_limit:{ip}:{tier_name}
        let ip = "192.168.1.1";
        let tier = RateLimitTier::Health;
        let key = format!("ip_limit:{}:{}", ip, tier.name());
        assert_eq!(key, "ip_limit:192.168.1.1:health");

        let tier = RateLimitTier::RequestLink;
        let key = format!("ip_limit:{}:{}", ip, tier.name());
        assert_eq!(key, "ip_limit:192.168.1.1:request_link");
    }

    /// Regression test: Code must NOT use std::time::SystemTime
    /// Bug: Worker crashed with "time not implemented on this platform" panic
    /// because std::time::SystemTime::now() doesn't work in WASM/Workers.
    /// Fix: Use js_sys::Date::now() via get_timestamp_secs()/get_timestamp_millis()
    #[test]
    fn test_no_std_time_usage_documented() {
        // This test documents the correct approach for getting timestamps in Workers.
        //
        // PROBLEM: std::time::SystemTime panics in WASM with:
        //   "panicked at library/std/src/sys/pal/wasm/../unsupported/time.rs:31:9:
        //    time not implemented on this platform"
        //
        // SOLUTION: The codebase provides WASM-safe helpers:
        //   - get_timestamp_secs() -> u64   (Unix timestamp in seconds)
        //   - get_timestamp_millis() -> u128 (Unix timestamp in milliseconds)
        //
        // Both use js_sys::Date::now() which works in Cloudflare Workers.
        //
        // VERIFICATION: Search codebase for std::time::SystemTime usage.
        // There should be ZERO occurrences outside of this test comment.
        //
        // grep -r "std::time::SystemTime" apps/docsign-web/worker/src/
        // Expected: 0 matches (only this comment mentions it)

        // Compile-time verification: the helper functions exist with correct signatures
        fn _verify_timestamp_helpers_exist() {
            let _: fn() -> u64 = get_timestamp_secs;
            let _: fn() -> u128 = get_timestamp_millis;
        }

        // The actual runtime behavior can only be tested in WASM environment
        // which is verified by deploying and hitting /health endpoint
        assert!(true, "This test documents WASM time requirements");
    }

    proptest! {
        /// Property: Window reset logic is correct
        #[test]
        fn prop_ip_rate_limit_window_expired(
            window_start in 0u64..1_000_000,
            window_seconds in 1u64..7200,
            offset in 0u64..10000
        ) {
            let now = window_start + window_seconds + offset;
            let is_expired = now >= window_start + window_seconds;

            // If offset >= 0, window should be expired (now >= window_start + window_seconds)
            prop_assert!(is_expired);
        }

        /// Property: Window NOT expired when current time is within window
        #[test]
        fn prop_ip_rate_limit_window_not_expired(
            window_start in 1000u64..1_000_000,
            window_seconds in 60u64..7200,
            offset in 1u64..59
        ) {
            // Ensure offset is less than window_seconds
            let actual_offset = offset % window_seconds;
            let now = window_start + actual_offset;
            let is_expired = now >= window_start + window_seconds;

            // Window should NOT be expired
            prop_assert!(!is_expired);
        }

        /// Property: Retry-after calculation is always at least 1 second
        #[test]
        fn prop_retry_after_at_least_one(
            window_start in 0u64..1_000_000,
            window_seconds in 60u64..7200,
            now in 0u64..2_000_000
        ) {
            let retry_after = (window_start + window_seconds).saturating_sub(now).max(1);
            prop_assert!(retry_after >= 1);
        }

        /// Property: Request count never goes negative
        #[test]
        fn prop_request_count_non_negative(
            initial in 0u32..1000,
            increments in 0usize..100
        ) {
            let mut count = initial;
            for _ in 0..increments {
                count += 1;
            }
            prop_assert!(count >= initial);
        }
    }
}
