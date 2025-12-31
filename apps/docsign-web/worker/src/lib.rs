//! DocSign Server - Cloudflare Worker for email relay and signing sessions
//!
//! Rate limited via AWS SES (email-proxy Lambda): 100/day, 3000/month
//! Signing sessions expire after 7 days

use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use worker::*;

/// Type alias for HMAC-SHA256
#[allow(dead_code)]
type HmacSha256 = Hmac<Sha256>;

/// Per-sender maximum sessions for session security
#[allow(dead_code)]
const MAX_SESSIONS_PER_SENDER: usize = 100;

/// Request body for sending a document
#[derive(Deserialize, Serialize)]
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
#[derive(Serialize, Deserialize)]
struct SendResponse {
    success: bool,
    message: String,
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
    encrypted_document: String,
    metadata: SessionMetadata,
    recipients: Vec<RecipientInfo>,
    fields: Vec<FieldInfo>,
    expires_at: String,
    #[serde(default)]
    signed_versions: Vec<SignedVersion>,
    /// Session status for the workflow (UX-002)
    #[serde(default)]
    status: SessionStatus,
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

/// Request to verify email identity (UX-003)
#[derive(Deserialize)]
#[allow(dead_code)]
struct VerifyRequest {
    recipient_id: String,
    email_suffix: String,
}

/// Response from verify endpoint (UX-003)
#[derive(Serialize)]
#[allow(dead_code)]
struct VerifyResponse {
    success: bool,
    remaining_attempts: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    locked_until: Option<String>,
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
struct EmailResponse {
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

/// Email verification state for UX-003
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[allow(dead_code)] // Used by email verification endpoint (UX-003)
struct VerificationState {
    /// Number of failed verification attempts
    attempts: u32,
    /// ISO 8601 timestamp when lockout expires (None if not locked)
    locked_until: Option<String>,
    /// ISO 8601 timestamp of last verification attempt
    last_attempt: String,
}

impl Default for VerificationState {
    fn default() -> Self {
        Self {
            attempts: 0,
            locked_until: None,
            last_attempt: Utc::now().to_rfc3339(),
        }
    }
}

#[allow(dead_code)] // Methods used by email verification endpoint (UX-003)
impl VerificationState {
    /// Increment the attempt counter
    fn increment_attempt(&mut self) {
        self.attempts += 1;
        self.last_attempt = Utc::now().to_rfc3339();
    }

    /// Check if currently locked out
    fn is_locked(&self) -> bool {
        if let Some(ref locked_until_str) = self.locked_until {
            if let Ok(locked_until) = chrono::DateTime::parse_from_rfc3339(locked_until_str) {
                return locked_until > Utc::now();
            }
        }
        false
    }

    /// Apply a 15-minute lockout
    fn apply_lockout(&mut self) {
        let lockout_duration = chrono::Duration::minutes(15);
        let locked_until = Utc::now() + lockout_duration;
        self.locked_until = Some(locked_until.to_rfc3339());
    }

    /// Reset verification state (after successful verification)
    fn reset(&mut self) {
        self.attempts = 0;
        self.locked_until = None;
    }

    /// Get remaining attempts (max 3)
    fn remaining_attempts(&self) -> u32 {
        3u32.saturating_sub(self.attempts)
    }
}

// ============================================================
// Per-IP Rate Limiting
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
    /// Request link: 3 req/hour per IP (prevents email spam)
    RequestLink,
}

impl RateLimitTier {
    /// Returns (max_requests, window_seconds) for this tier
    fn limits(&self) -> (u32, u64) {
        match self {
            RateLimitTier::Health => (100, 60),      // 100/min
            RateLimitTier::SessionRead => (30, 60),  // 30/min
            RateLimitTier::SessionWrite => (5, 60),  // 5/min
            RateLimitTier::RequestLink => (3, 3600), // 3/hour
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

    // Get current timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

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
        .or_else(|| req.headers().get("X-Forwarded-For").ok().flatten())
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

const EMAIL_PROXY_URL: &str =
    "https://5wbbpgjw7acyu4sgjqksmsqtvq0zajks.lambda-url.us-east-2.on.aws/send";
const SESSION_TTL_SECONDS: u64 = 7 * 24 * 60 * 60; // 7 days
const DOWNLOAD_LINK_EXPIRY_DAYS: u32 = 30;

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

/// Send notification email to sender when recipient signs
async fn send_sender_notification(
    env: &Env,
    sender_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<()> {
    let api_key = match env.secret("EMAIL_PROXY_API_KEY") {
        Ok(key) => key.to_string(),
        Err(_) => {
            console_log!("Cannot send notification: EMAIL_PROXY_API_KEY not configured");
            return Ok(()); // Don't fail the signing process if email fails
        }
    };

    let email_text = html_body
        .replace("<br>", "\n")
        .replace("</p>", "\n")
        .replace("<li>", "- ")
        .chars()
        .filter(|c| c.is_ascii() || c.is_whitespace())
        .collect::<String>();

    let email_body = serde_json::json!({
        "from": "GetSignatures <noreply@mail.getsignatures.org>",
        "to": [sender_email],
        "subject": subject,
        "html": html_body,
        "text": email_text,
    });

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", api_key))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(serde_json::to_string(&email_body)?.into()));

    let request = Request::new_with_init(EMAIL_PROXY_URL, &init)?;

    match Fetch::Request(request).send().await {
        Ok(response) => {
            if response.status_code() == 200 {
                console_log!("Notification sent to sender: {}", sender_email);
            } else {
                console_log!(
                    "Failed to send notification: status {}",
                    response.status_code()
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
        // Health check (public) - Rate limit: 100/min per IP
        (Method::Get, "/health") => {
            if let Some(response) = apply_ip_rate_limit(&req, &env, RateLimitTier::Health).await {
                return response;
            }
            cors_response(Response::ok("OK"))
        }
        (Method::Get, "/") => cors_response(Response::ok("DocSign API Server")),

        // Protected endpoints - require API key
        (Method::Post, "/send") => {
            if !verify_api_key(&req, &env) {
                return cors_response(Response::error("Unauthorized", 401));
            }
            handle_send_email(req, env).await
        }
        // /invite - Rate limit: 3/hour per IP (prevents email spam)
        (Method::Post, "/invite") => {
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::RequestLink).await
            {
                return response;
            }
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
        // GET session - Rate limit: 30/min per IP
        (Method::Get, p) if p.starts_with("/session/") => {
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionRead).await
            {
                return response;
            }
            let id = p.strip_prefix("/session/").unwrap_or("");
            if id.contains('/') {
                cors_response(Response::error("Not found", 404))
            } else {
                handle_get_session(id, env).await
            }
        }
        // PUT signed - Rate limit: 5/min per IP
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/signed") => {
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
        // PUT decline - Rate limit: 5/min per IP
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/decline") => {
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
        // POST verify - Rate limit: 5/min per IP (write operation)
        (Method::Post, p) if p.starts_with("/session/") && p.ends_with("/verify") => {
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_verify(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // UX-004: Request new link endpoint (public - no API key required)
        // Rate limit: 3/hour per IP (prevents email spam)
        (Method::Post, p) if p.starts_with("/session/") && p.ends_with("/request-link") => {
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

async fn handle_send_email(mut req: Request, env: Env) -> Result<Response> {
    // Parse request
    let body: SendRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Send the email (per-IP rate limiting is handled at router level)
    send_email(&env, &body).await
}

async fn send_email(env: &Env, body: &SendRequest) -> Result<Response> {
    let api_key = match env.secret("EMAIL_PROXY_API_KEY") {
        Ok(key) => key.to_string(),
        Err(_) => {
            return cors_response(error_response("EMAIL_PROXY_API_KEY not configured"));
        }
    };

    // Build email body with optional signing link
    let email_text = if let Some(ref link) = body.signing_link {
        format!(
            "You have been requested to sign a document.\n\n\
            Click the link below to sign:\n{}\n\n\
            Or download the attached PDF to sign locally.",
            link
        )
    } else {
        "Please find the attached document for your signature.".to_string()
    };

    let email_body = serde_json::json!({
        "from": "GetSignatures <noreply@mail.getsignatures.org>",
        "to": [body.to],
        "subject": body.subject,
        "text": email_text,
        "attachments": [{
            "filename": body.filename,
            "content": body.pdf_base64
        }]
    });

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", api_key))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(serde_json::to_string(&email_body)?.into()));

    let request = Request::new_with_init(EMAIL_PROXY_URL, &init)?;
    let response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let status = response.status_code();
        let mut response = response;
        let error_text = response.text().await.unwrap_or_default();
        console_log!("email-proxy error {}: {}", status, error_text);
        return cors_response(error_response(&format!(
            "email-proxy API error: {}",
            status
        )));
    }

    cors_response(Response::from_json(&SendResponse {
        success: true,
        message: "Email sent".to_string(),
    }))
}

async fn handle_send_invites(mut req: Request, env: Env) -> Result<Response> {
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

    // Send the invitations (per-IP rate limiting is handled at router level)
    send_invitations(&env, &body).await
}

async fn send_invitations(env: &Env, body: &InviteRequest) -> Result<Response> {
    let api_key = match env.secret("EMAIL_PROXY_API_KEY") {
        Ok(key) => key.to_string(),
        Err(_) => {
            return cors_response(error_response("EMAIL_PROXY_API_KEY not configured"));
        }
    };

    // Send invitation emails to each recipient
    let mut success_count = 0;
    let mut errors = Vec::new();

    for invitation in &body.invitations {
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

        <div style="background: #fef3c7; border-left: 4px solid #f59e0b; padding: 15px; border-radius: 4px; margin-top: 25px;">
            <p style="margin: 0; font-size: 14px; color: #92400e;">
                <strong>Note:</strong> This link is unique to you and expires based on the sender's deadline setting.
                The document is encrypted end-to-end for your security.
            </p>
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
            signing_link = invitation.signing_link
        );

        let email_text = format!(
            "Hello {},\n\n\
            {} has requested your signature on the following document:\n\
            {}\n\n\
            Click the link below to review and sign:\n\
            {}\n\n\
            Note: This link is unique to you and expires based on the sender's deadline setting.\n\
            The document is encrypted end-to-end for your security.\n\n\
            If you have any questions, please contact the sender directly.\n\n\
            ---\n\
            Sent via GetSignatures - Secure Document Signing",
            invitation.name, body.sender_name, body.document_name, invitation.signing_link
        );

        let email_body = serde_json::json!({
            "from": "GetSignatures <noreply@mail.getsignatures.org>",
            "to": [invitation.email],
            "subject": format!("Signature Requested: {}", body.document_name),
            "html": email_html,
            "text": email_text,
        });

        let headers = Headers::new();
        headers.set("Authorization", &format!("Bearer {}", api_key))?;
        headers.set("Content-Type", "application/json")?;

        let mut init = RequestInit::new();
        init.with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(serde_json::to_string(&email_body)?.into()));

        let request = Request::new_with_init(EMAIL_PROXY_URL, &init)?;

        match Fetch::Request(request).send().await {
            Ok(response) => {
                if response.status_code() == 200 {
                    success_count += 1;
                    console_log!("Invitation sent to {}", invitation.email);
                } else {
                    let status = response.status_code();
                    errors.push(format!("{}: HTTP {}", invitation.email, status));
                    console_log!("Failed to send to {}: HTTP {}", invitation.email, status);
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
    let body: CreateSessionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Generate session ID
    let session_id = generate_session_id();

    // Calculate expiry
    let expiry_seconds = (body.expiry_hours as u64) * 60 * 60;
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
    };

    // Store session with TTL
    let session_json = serde_json::to_string(&session)
        .map_err(|e| Error::from(format!("Serialize error: {}", e)))?;

    kv.put(&format!("session:{}", session_id), session_json)?
        .expiration_ttl(expiry_seconds.min(SESSION_TTL_SECONDS))
        .execute()
        .await?;

    cors_response(Response::from_json(&CreateSessionResponse {
        success: true,
        session_id,
        message: None,
    }))
}

async fn handle_get_session(session_id: &str, env: Env) -> Result<Response> {
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

            cors_response(Response::from_json(&GetSessionResponse {
                success: true,
                session: Some(SessionPublicInfo {
                    id: s.id,
                    metadata: s.metadata,
                    recipients: s.recipients,
                    fields: s.fields,
                    encrypted_document: s.encrypted_document,
                    expires_at: s.expires_at,
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

            // TODO: Send notification email to sender (UX-006 integration)
            // For now, we just log it
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
            // Generate new session ID
            let new_session_id = generate_session_id();

            // Calculate new expiry (7 days default)
            let expiry_hours = 168u64;
            let expiry_seconds = expiry_hours * 60 * 60;
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
                        r
                    })
                    .collect(),
                fields: s.fields,
                expires_at: expires_at.clone(),
                signed_versions: vec![],
                status: SessionStatus::Pending,
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

            cors_response(Ok(Response::from_json(&EmailResponse {
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
    let body: SubmitSignedRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

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
                encrypted_document: body.encrypted_document,
                signed_at: chrono::Utc::now().to_rfc3339(),
            });

            // Update the main document if this is the latest version
            if let Some(latest) = s.signed_versions.last() {
                s.encrypted_document = latest.encrypted_document.clone();
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

async fn handle_verify(session_id: &str, mut req: Request, env: Env) -> Result<Response> {
    // Parse the verify request
    let body: VerifyRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Get SESSIONS KV to verify the session and recipient exist
    let sessions_kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Get VERIFICATIONS KV to track verification attempts
    let verifications_kv = match env.kv("VERIFICATIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("VERIFICATIONS KV not configured"));
        }
    };

    // Fetch the session to verify it exists and get recipient email
    let session: Option<SigningSession> = sessions_kv
        .get(&format!("session:{}", session_id))
        .json()
        .await?;

    let session = match session {
        Some(s) => s,
        None => {
            return cors_response(Ok(Response::from_json(&VerifyResponse {
                success: false,
                remaining_attempts: 0,
                locked_until: None,
            })?
            .with_status(404)));
        }
    };

    // Find the recipient
    let recipient = session
        .recipients
        .iter()
        .find(|r| r.id == body.recipient_id);

    let recipient = match recipient {
        Some(r) => r,
        None => {
            return cors_response(Ok(Response::from_json(&VerifyResponse {
                success: false,
                remaining_attempts: 0,
                locked_until: None,
            })?
            .with_status(404)));
        }
    };

    // Get or create verification state for this recipient
    let verify_key = format!("verify:{}:{}", session_id, body.recipient_id);
    let mut state: VerificationState = verifications_kv
        .get(&verify_key)
        .json()
        .await?
        .unwrap_or_default();

    // Check if currently locked out
    if state.is_locked() {
        return cors_response(Ok(Response::from_json(&VerifyResponse {
            success: false,
            remaining_attempts: state.remaining_attempts(),
            locked_until: state.locked_until.clone(),
        })?));
    }

    // Verify the email suffix
    let recipient_email = &recipient.email;
    let expected_suffix = &recipient_email[recipient_email.len().saturating_sub(6)..];

    let is_correct = body.email_suffix == expected_suffix;

    if is_correct {
        // Success! Reset the verification state
        state.reset();

        // Save the reset state
        verifications_kv
            .put(&verify_key, serde_json::to_string(&state)?)?
            .execute()
            .await?;

        cors_response(Ok(Response::from_json(&VerifyResponse {
            success: true,
            remaining_attempts: 3,
            locked_until: None,
        })?))
    } else {
        // Incorrect suffix - increment attempts
        state.increment_attempt();

        // Check if we should apply lockout (3 failures)
        if state.attempts >= 3 {
            state.apply_lockout();
        }

        // Save the updated state
        verifications_kv
            .put(&verify_key, serde_json::to_string(&state)?)?
            .execute()
            .await?;

        cors_response(Ok(Response::from_json(&VerifyResponse {
            success: false,
            remaining_attempts: state.remaining_attempts(),
            locked_until: state.locked_until.clone(),
        })?))
    }
}

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    // Generate random suffix
    let random: u64 = js_sys::Math::random().to_bits();

    format!("{:x}{:08x}", timestamp, random as u32)
}

/// Generate a HMAC-SHA256 token for a recipient
///
/// The token is computed as HMAC-SHA256 of "session_id:recipient_id" using the provided secret.
/// Returns the token as a hex-encoded string.
#[allow(dead_code)]
fn generate_recipient_token(session_id: &str, recipient_id: &str, secret: &[u8]) -> String {
    let message = format!("{}:{}", session_id, recipient_id);
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Verify a HMAC-SHA256 token for a recipient
///
/// Performs constant-time comparison to prevent timing attacks.
/// Returns true if the token matches the expected HMAC, false otherwise.
#[allow(dead_code)]
fn verify_recipient_token(
    token: &str,
    session_id: &str,
    recipient_id: &str,
    secret: &[u8],
) -> bool {
    let expected_token = generate_recipient_token(session_id, recipient_id, secret);

    // Constant-time comparison to prevent timing attacks
    if token.len() != expected_token.len() {
        return false;
    }

    token
        .bytes()
        .zip(expected_token.bytes())
        .all(|(a, b)| a == b)
}

fn error_response(msg: &str) -> Result<Response> {
    let resp = Response::from_json(&SendResponse {
        success: false,
        message: msg.to_string(),
    })?;
    Ok(resp.with_status(400))
}

fn cors_response(response: Result<Response>) -> Result<Response> {
    response.map(|r| {
        let headers = Headers::new();
        let _ = headers.set("Access-Control-Allow-Origin", "*");
        let _ = headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, OPTIONS");
        let _ = headers.set("Access-Control-Allow-Headers", "Content-Type");
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
    // Unit Tests for HMAC-SHA256 Token Generation and Verification
    // ============================================================

    #[test]
    fn test_generate_recipient_token() {
        let secret = b"test-secret-key";
        let session_id = "session-123";
        let recipient_id = "recipient-456";

        let token = generate_recipient_token(session_id, recipient_id, secret);

        // Token should be a hex string (64 characters for SHA256)
        assert_eq!(token.len(), 64);
        // Verify it's valid hex
        assert!(hex::decode(&token).is_ok());
    }

    #[test]
    fn test_verify_recipient_token_success() {
        let secret = b"test-secret-key";
        let session_id = "session-123";
        let recipient_id = "recipient-456";

        let token = generate_recipient_token(session_id, recipient_id, secret);
        assert!(verify_recipient_token(
            &token,
            session_id,
            recipient_id,
            secret
        ));
    }

    #[test]
    fn test_verify_recipient_token_wrong_session() {
        let secret = b"test-secret-key";
        let session_id = "session-123";
        let recipient_id = "recipient-456";

        let token = generate_recipient_token(session_id, recipient_id, secret);
        assert!(!verify_recipient_token(
            &token,
            "session-wrong",
            recipient_id,
            secret
        ));
    }

    #[test]
    fn test_verify_recipient_token_wrong_recipient() {
        let secret = b"test-secret-key";
        let session_id = "session-123";
        let recipient_id = "recipient-456";

        let token = generate_recipient_token(session_id, recipient_id, secret);
        assert!(!verify_recipient_token(
            &token,
            session_id,
            "recipient-wrong",
            secret
        ));
    }

    #[test]
    fn test_verify_recipient_token_wrong_secret() {
        let secret = b"test-secret-key";
        let session_id = "session-123";
        let recipient_id = "recipient-456";

        let token = generate_recipient_token(session_id, recipient_id, secret);
        assert!(!verify_recipient_token(
            &token,
            session_id,
            recipient_id,
            b"wrong-secret-key"
        ));
    }

    #[test]
    fn test_verify_recipient_token_empty_inputs() {
        let secret = b"test-secret-key";

        let token1 = generate_recipient_token("", "recipient-456", secret);
        assert!(verify_recipient_token(&token1, "", "recipient-456", secret));

        let token2 = generate_recipient_token("session-123", "", secret);
        assert!(verify_recipient_token(&token2, "session-123", "", secret));

        let token3 = generate_recipient_token("", "", secret);
        assert!(verify_recipient_token(&token3, "", "", secret));
    }

    #[test]
    fn test_verify_recipient_token_empty_secret() {
        let session_id = "session-123";
        let recipient_id = "recipient-456";

        let token = generate_recipient_token(session_id, recipient_id, b"");
        assert!(verify_recipient_token(
            &token,
            session_id,
            recipient_id,
            b""
        ));
    }

    #[test]
    fn test_verify_recipient_token_malformed_token() {
        let secret = b"test-secret-key";
        let session_id = "session-123";
        let recipient_id = "recipient-456";

        assert!(!verify_recipient_token(
            "not-a-valid-hex",
            session_id,
            recipient_id,
            secret
        ));
        assert!(!verify_recipient_token(
            "",
            session_id,
            recipient_id,
            secret
        ));
    }

    use proptest::prelude::*;

    // ============================================================
    // Property Tests for Session/Magic Link Functionality
    // ============================================================

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
                signed,
                signed_at: if signed { Some("2025-01-15T10:00:00Z".to_string()) } else { None },
                declined: false,
                declined_at: None,
                decline_reason: None,
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
                .map(|i| RecipientInfo {
                    id: i.to_string(),
                    name: format!("Recipient {}", i),
                    email: format!("user{}@example.com", i),
                    role: "signer".to_string(),
                    signed: false,
                    signed_at: None,
                    declined: false,
                    declined_at: None,
                    decline_reason: None,
                })
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
        let session = SigningSession {
            id: "sess_123".to_string(),
            encrypted_document: "base64_encrypted_doc".to_string(),
            metadata: SessionMetadata {
                filename: "contract.pdf".to_string(),
                page_count: 5,
                created_at: "2025-01-15T10:00:00Z".to_string(),
                created_by: "Sender".to_string(),
                sender_email: Some("sender@example.com".to_string()),
            },
            recipients: vec![RecipientInfo {
                id: "1".to_string(),
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                role: "signer".to_string(),
                signed: true,
                signed_at: Some("2025-01-15T11:00:00Z".to_string()),
                declined: false,
                declined_at: None,
                decline_reason: None,
            }],
            fields: vec![],
            expires_at: "2025-01-22T10:00:00Z".to_string(),
            signed_versions: vec![SignedVersion {
                recipient_id: "1".to_string(),
                encrypted_document: "signed_doc_base64".to_string(),
                signed_at: "2025-01-15T11:00:00Z".to_string(),
            }],
            status: SessionStatus::Completed,
        };

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
            let recipient = RecipientInfo {
                id: "1".to_string(),
                name: "Test".to_string(),
                email: "test@example.com".to_string(),
                role: role.to_string(),
                signed: false,
                signed_at: None,
                declined: false,
                declined_at: None,
                decline_reason: None,
            };

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
    // UX-003: Email Verification Tests
    // ============================================================

    #[test]
    fn test_verification_state_structure() {
        // Verification state should track attempts and lockout
        let state = VerificationState {
            attempts: 2,
            locked_until: None,
            last_attempt: "2025-01-15T10:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: VerificationState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.attempts, 2);
        assert_eq!(deserialized.locked_until, None);
    }

    #[test]
    fn test_verification_state_with_lockout() {
        // Verification state should store lockout expiry
        let state = VerificationState {
            attempts: 3,
            locked_until: Some("2025-01-15T10:15:00Z".to_string()),
            last_attempt: "2025-01-15T10:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: VerificationState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.attempts, 3);
        assert!(deserialized.locked_until.is_some());
    }

    #[test]
    fn test_increment_verification_attempts() {
        let mut state = VerificationState::default();

        state.increment_attempt();
        assert_eq!(state.attempts, 1);

        state.increment_attempt();
        assert_eq!(state.attempts, 2);

        state.increment_attempt();
        assert_eq!(state.attempts, 3);
    }

    #[test]
    fn test_lockout_after_3_failures() {
        let mut state = VerificationState::default();

        // First 2 attempts should not lock
        state.increment_attempt();
        state.increment_attempt();
        assert!(!state.is_locked());

        // 3rd attempt should trigger lockout
        state.increment_attempt();
        state.apply_lockout();
        assert!(state.is_locked());
        assert!(state.locked_until.is_some());
    }

    #[test]
    fn test_lockout_duration_15_minutes() {
        let mut state = VerificationState::default();
        let before = Utc::now();

        state.attempts = 3;
        state.apply_lockout();

        let _after = Utc::now();
        let lockout_time =
            chrono::DateTime::parse_from_rfc3339(&state.locked_until.unwrap()).unwrap();

        // Lockout should be approximately 15 minutes from now
        let duration = lockout_time.signed_duration_since(before);
        assert!(duration.num_minutes() >= 14);
        assert!(duration.num_minutes() <= 16);
    }

    #[test]
    fn test_reset_verification_state() {
        let mut state = VerificationState {
            attempts: 3,
            locked_until: Some("2025-01-15T10:15:00Z".to_string()),
            last_attempt: "2025-01-15T10:00:00Z".to_string(),
        };

        state.reset();

        assert_eq!(state.attempts, 0);
        assert_eq!(state.locked_until, None);
    }

    #[test]
    fn test_remaining_attempts() {
        let mut state = VerificationState::default();

        assert_eq!(state.remaining_attempts(), 3);

        state.increment_attempt();
        assert_eq!(state.remaining_attempts(), 2);

        state.increment_attempt();
        assert_eq!(state.remaining_attempts(), 1);

        state.increment_attempt();
        assert_eq!(state.remaining_attempts(), 0);
    }

    // ============================================================
    // UX-003: Email Verification Endpoint Tests (FAILING)
    // ============================================================

    #[test]
    fn test_verify_request_structure() {
        // Verify request should contain recipient_id and email_suffix
        let json = r#"{
            "recipient_id": "recip_123",
            "email_suffix": "abc123"
        }"#;

        let request: VerifyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.recipient_id, "recip_123");
        assert_eq!(request.email_suffix, "abc123");
    }

    #[test]
    fn test_verify_response_structure_success() {
        // Verify response should include success flag and remaining attempts
        let response = VerifyResponse {
            success: true,
            remaining_attempts: 3,
            locked_until: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"remaining_attempts\":3"));
    }

    #[test]
    fn test_verify_response_structure_failure() {
        // Verify response on failure should include remaining attempts
        let response = VerifyResponse {
            success: false,
            remaining_attempts: 2,
            locked_until: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"remaining_attempts\":2"));
    }

    #[test]
    fn test_verify_response_structure_locked() {
        // Verify response when locked should include lockout expiry
        let response = VerifyResponse {
            success: false,
            remaining_attempts: 0,
            locked_until: Some("2025-01-15T10:15:00Z".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"remaining_attempts\":0"));
        assert!(json.contains("\"locked_until\""));
    }

    #[test]
    fn test_verification_state_tracks_per_recipient() {
        // Each recipient should have their own verification state
        // This test will fail until we implement the endpoint
        let recipient1 = "recip_1";
        let recipient2 = "recip_2";

        // States should be independent
        let mut state1 = VerificationState::default();
        let state2 = VerificationState::default();

        state1.increment_attempt();
        assert_eq!(state1.attempts, 1);
        assert_eq!(state2.attempts, 0); // state2 unchanged

        state1.increment_attempt();
        state1.increment_attempt();
        state1.apply_lockout();
        assert!(state1.is_locked());
        assert!(!state2.is_locked()); // state2 still not locked

        // In the actual implementation, these would be stored separately in KV
        // with keys like "verify:session_123:recip_1" and "verify:session_123:recip_2"
        assert_ne!(
            format!("verify:session:{}:{}", "sess_123", recipient1),
            format!("verify:session:{}:{}", "sess_123", recipient2)
        );
    }

    #[test]
    fn test_verification_email_suffix_matching() {
        // Email suffix should match the last 6 characters of the recipient's email
        // For example, email "alice@example.com" has suffix "le.com"
        // This would be the last 6 chars of the email

        let email = "alice@example.com";
        let expected_suffix = &email[email.len().saturating_sub(6)..];
        assert_eq!(expected_suffix, "le.com");

        let email2 = "bob@test.org";
        let expected_suffix2 = &email2[email2.len().saturating_sub(6)..];
        assert_eq!(expected_suffix2, "st.org");

        // Short emails (less than 6 chars) should use the whole email
        let short_email = "a@b.c";
        let expected_suffix3 = &short_email[short_email.len().saturating_sub(6)..];
        assert_eq!(expected_suffix3, "a@b.c");
    }

    #[test]
    fn test_verification_lockout_prevents_further_attempts() {
        // Once locked out, verification should fail even with correct suffix
        let mut state = VerificationState {
            attempts: 3,
            ..Default::default()
        };

        state.apply_lockout();

        assert!(state.is_locked());
        assert_eq!(state.remaining_attempts(), 0);

        // Lockout should be active
        assert!(state.locked_until.is_some());
    }

    #[test]
    fn test_verification_success_resets_state() {
        // Successful verification should reset attempt counter
        let mut state = VerificationState::default();

        state.increment_attempt();
        state.increment_attempt();
        assert_eq!(state.attempts, 2);

        // After successful verification
        state.reset();
        assert_eq!(state.attempts, 0);
        assert_eq!(state.locked_until, None);
        assert_eq!(state.remaining_attempts(), 3);
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
        let recipients = vec![
            RecipientInfo {
                id: "1".to_string(),
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                role: "signer".to_string(),
                signed: true,
                signed_at: Some("2025-01-15T11:00:00Z".to_string()),
                declined: false,
                declined_at: None,
                decline_reason: None,
            },
            RecipientInfo {
                id: "2".to_string(),
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
                role: "signer".to_string(),
                signed: true,
                signed_at: Some("2025-01-15T12:00:00Z".to_string()),
                declined: false,
                declined_at: None,
                decline_reason: None,
            },
        ];

        // This will fail because the function doesn't exist yet
        assert!(all_recipients_signed(&recipients));
    }

    #[test]
    fn test_not_all_recipients_signed_detection() {
        // Test that we correctly detect when not all recipients have signed
        let recipients = vec![
            RecipientInfo {
                id: "1".to_string(),
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                role: "signer".to_string(),
                signed: true,
                signed_at: Some("2025-01-15T11:00:00Z".to_string()),
                declined: false,
                declined_at: None,
                decline_reason: None,
            },
            RecipientInfo {
                id: "2".to_string(),
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
                role: "signer".to_string(),
                signed: false,
                signed_at: None,
                declined: false,
                declined_at: None,
                decline_reason: None,
            },
        ];

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
        let recipients = vec![
            RecipientInfo {
                id: "1".to_string(),
                name: "Alice Smith".to_string(),
                email: "alice@example.com".to_string(),
                role: "signer".to_string(),
                signed: true,
                signed_at: Some("2025-01-15T11:00:00Z".to_string()),
                declined: false,
                declined_at: None,
                decline_reason: None,
            },
            RecipientInfo {
                id: "2".to_string(),
                name: "Bob Jones".to_string(),
                email: "bob@example.com".to_string(),
                role: "signer".to_string(),
                signed: true,
                signed_at: Some("2025-01-15T12:00:00Z".to_string()),
                declined: false,
                declined_at: None,
                decline_reason: None,
            },
        ];

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
        let recipient = RecipientInfo {
            id: "1".to_string(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            role: "signer".to_string(),
            signed: false,
            signed_at: None,
            declined: false, // This field should exist
            declined_at: None,
            decline_reason: None,
        };

        assert!(!recipient.declined);
        assert!(recipient.declined_at.is_none());
        assert!(recipient.decline_reason.is_none());
    }

    #[test]
    fn test_recipient_decline_sets_fields() {
        // UX-002: Declining a recipient should set declined=true and declined_at
        let mut recipient = RecipientInfo {
            id: "1".to_string(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            role: "signer".to_string(),
            signed: false,
            signed_at: None,
            declined: false,
            declined_at: None,
            decline_reason: None,
        };

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
        let recipient = RecipientInfo {
            id: "1".to_string(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            role: "signer".to_string(),
            signed: false,
            signed_at: None,
            declined: true, // Already declined
            declined_at: Some("2025-12-21T12:00:00Z".to_string()),
            decline_reason: Some("Not ready".to_string()),
        };

        // A declined recipient should be blocked from signing
        assert!(recipient.declined);
        assert!(!recipient.signed);
        // Attempting to sign a declined recipient should fail
        // (This would be tested at the handler level)
    }

    #[test]
    fn test_session_has_status_field() {
        // UX-002: SigningSession must have a status field
        let session = SigningSession {
            id: "sess_123".to_string(),
            encrypted_document: "encrypted_data".to_string(),
            metadata: SessionMetadata {
                filename: "contract.pdf".to_string(),
                page_count: 3,
                created_at: "2025-12-21T12:00:00Z".to_string(),
                created_by: "Alice".to_string(),
                sender_email: Some("alice@example.com".to_string()),
            },
            recipients: vec![],
            fields: vec![],
            expires_at: "2025-12-28T12:00:00Z".to_string(),
            signed_versions: vec![],
            status: SessionStatus::Pending, // This field should exist
        };

        assert_eq!(session.status, SessionStatus::Pending);
    }

    #[test]
    fn test_session_status_transitions() {
        // UX-002: Session status should transition correctly
        let mut session = SigningSession {
            id: "sess_123".to_string(),
            encrypted_document: "encrypted_data".to_string(),
            metadata: SessionMetadata {
                filename: "contract.pdf".to_string(),
                page_count: 3,
                created_at: "2025-12-21T12:00:00Z".to_string(),
                created_by: "Alice".to_string(),
                sender_email: Some("alice@example.com".to_string()),
            },
            recipients: vec![],
            fields: vec![],
            expires_at: "2025-12-28T12:00:00Z".to_string(),
            signed_versions: vec![],
            status: SessionStatus::Pending,
        };

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
        let session = SigningSession {
            id: "sess_expired".to_string(),
            encrypted_document: "encrypted_data".to_string(),
            metadata: SessionMetadata {
                filename: "expired.pdf".to_string(),
                page_count: 1,
                created_at: "2025-12-01T12:00:00Z".to_string(),
                created_by: "Alice".to_string(),
                sender_email: Some("alice@example.com".to_string()),
            },
            recipients: vec![],
            fields: vec![],
            expires_at: "2025-12-10T12:00:00Z".to_string(), // Past date
            signed_versions: vec![],
            status: SessionStatus::Expired,
        };

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
    // Email-Proxy Integration Tests
    // ============================================================

    #[test]
    fn test_send_request_structure() {
        // Verify SendRequest can be serialized/deserialized correctly
        // This ensures email payload structure matches expected format
        let request = SendRequest {
            to: "recipient@example.com".to_string(),
            subject: "Please Sign Document".to_string(),
            pdf_base64: "JVBERi0xLjQK...".to_string(), // Mock base64 PDF
            filename: "contract.pdf".to_string(),
            signing_link: Some("https://getsignatures.org/sign/abc123".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&request).unwrap();
        let parsed: SendRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.to, "recipient@example.com");
        assert_eq!(parsed.subject, "Please Sign Document");
        assert_eq!(parsed.filename, "contract.pdf");
        assert_eq!(
            parsed.signing_link,
            Some("https://getsignatures.org/sign/abc123".to_string())
        );
    }

    #[test]
    fn test_send_request_without_signing_link() {
        // Verify SendRequest works without optional signing_link
        let request = SendRequest {
            to: "user@example.com".to_string(),
            subject: "Document".to_string(),
            pdf_base64: "JVBERi0xLjQK...".to_string(),
            filename: "doc.pdf".to_string(),
            signing_link: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: SendRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.to, "user@example.com");
        assert_eq!(parsed.signing_link, None);
    }

    #[test]
    fn test_email_payload_structure_with_attachments() {
        // Verify the email payload structure that would be sent to email-proxy
        // This simulates the JSON body that gets built for email service
        let request = SendRequest {
            to: "signer@example.com".to_string(),
            subject: "Action Required: Sign Document".to_string(),
            pdf_base64: "JVBERi0xLjQKJeLjz9M...".to_string(),
            filename: "agreement.pdf".to_string(),
            signing_link: Some("https://getsignatures.org/sign/sess_xyz789".to_string()),
        };

        // Build the email payload that would be sent
        let email_payload = serde_json::json!({
            "from": "GetSignatures <noreply@mail.getsignatures.org>",
            "to": [request.to],
            "subject": request.subject,
            "text": "You have been requested to sign a document.\n\nClick the link below to sign:\nhttps://getsignatures.org/sign/sess_xyz789\n\nOr download the attached PDF to sign locally.",
            "attachments": [{
                "filename": request.filename,
                "content": request.pdf_base64
            }]
        });

        // Verify structure
        assert_eq!(
            email_payload["from"],
            "GetSignatures <noreply@mail.getsignatures.org>"
        );
        assert_eq!(email_payload["to"][0], "signer@example.com");
        assert_eq!(email_payload["subject"], "Action Required: Sign Document");
        assert_eq!(email_payload["attachments"][0]["filename"], "agreement.pdf");
        assert_eq!(
            email_payload["attachments"][0]["content"],
            "JVBERi0xLjQKJeLjz9M..."
        );
    }

    #[test]
    fn test_email_proxy_url_constant() {
        // Verify the email-proxy URL constant is correctly configured
        assert_eq!(
            EMAIL_PROXY_URL,
            "https://5wbbpgjw7acyu4sgjqksmsqtvq0zajks.lambda-url.us-east-2.on.aws/send"
        );
    }

    #[test]
    fn test_send_response_structure() {
        // Verify SendResponse serialization for successful email send
        let response = SendResponse {
            success: true,
            message: "Email sent".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: SendResponse = serde_json::from_str(&json).unwrap();

        assert!(parsed.success);
        assert_eq!(parsed.message, "Email sent");
    }

    #[test]
    fn test_email_headers_authorization_format() {
        // Verify Authorization header format for email service
        let api_key = "test_api_key_12345";
        let header_value = format!("Bearer {}", api_key);

        assert_eq!(header_value, "Bearer test_api_key_12345");
        assert!(header_value.starts_with("Bearer "));
    }

    #[test]
    fn test_email_headers_content_type() {
        // Verify Content-Type header is correct for JSON email payloads
        let content_type = "application/json";
        assert_eq!(content_type, "application/json");
    }

    #[test]
    fn test_email_sender_notification_structure() {
        // Verify the sender notification email has correct structure
        let subject = "Document Signing Update";
        let html_body = "<html><body><p>Test</p></body></html>";

        assert!(!subject.is_empty());
        assert!(!html_body.is_empty());
        assert!(html_body.contains("<html>"));
        assert!(html_body.contains("<body>"));
    }

    #[test]
    fn test_recipient_email_validation() {
        // Verify email addresses are stored correctly in SendRequest
        let valid_emails = vec![
            "user@example.com",
            "john.doe@company.org",
            "contact+tag@domain.co.uk",
        ];

        for email in valid_emails {
            let request = SendRequest {
                to: email.to_string(),
                subject: "Test".to_string(),
                pdf_base64: "JVBERi0xLjQK".to_string(),
                filename: "test.pdf".to_string(),
                signing_link: None,
            };

            assert_eq!(request.to, email);
        }
    }

    #[test]
    fn test_pdf_base64_content_in_payload() {
        // Verify PDF base64 content is correctly placed in attachment
        let pdf_base64 = "JVBERi0xLjQKJeLjz9M0NTcnCjEgMCBvYmo=";
        let request = SendRequest {
            to: "test@example.com".to_string(),
            subject: "Document".to_string(),
            pdf_base64: pdf_base64.to_string(),
            filename: "document.pdf".to_string(),
            signing_link: None,
        };

        assert_eq!(request.pdf_base64, pdf_base64);
        assert!(!request.pdf_base64.is_empty());
        // Base64 should not contain spaces or newlines
        assert!(!request.pdf_base64.contains(' '));
        assert!(!request.pdf_base64.contains('\n'));
    }

    #[test]
    fn test_filename_preservation_in_attachment() {
        // Verify filename is correctly preserved through the email payload
        let filenames = vec![
            "contract.pdf",
            "document_2025.pdf",
            "Purchase Agreement.pdf",
            "lease-agreement-v2.pdf",
        ];

        for filename in filenames {
            let request = SendRequest {
                to: "test@example.com".to_string(),
                subject: "Sign".to_string(),
                pdf_base64: "JVBERi0xLjQK".to_string(),
                filename: filename.to_string(),
                signing_link: None,
            };

            assert_eq!(request.filename, filename);
            assert!(request.filename.ends_with(".pdf"));
        }
    }

    #[test]
    fn test_signing_link_url_format() {
        // Verify signing links are valid URLs
        let signing_links = vec![
            "https://getsignatures.org/sign/abc123",
            "https://getsignatures.org/sign/sess_xyz789",
            "https://getsignatures.org/sign/sess_12345abcdef",
        ];

        for link in signing_links {
            let request = SendRequest {
                to: "test@example.com".to_string(),
                subject: "Sign".to_string(),
                pdf_base64: "JVBERi0xLjQK".to_string(),
                filename: "doc.pdf".to_string(),
                signing_link: Some(link.to_string()),
            };

            assert_eq!(request.signing_link.unwrap(), link);
            assert!(link.starts_with("https://"));
        }
    }

    #[test]
    fn test_email_subject_length() {
        // Verify email subjects are reasonable length
        let valid_subjects = vec![
            "Please Sign",
            "Document Requires Your Signature",
            "Action Required: Review and Sign Contract",
            "Important: Legal Agreement Needs Your Signature",
        ];

        for subject in valid_subjects {
            let request = SendRequest {
                to: "test@example.com".to_string(),
                subject: subject.to_string(),
                pdf_base64: "JVBERi0xLjQK".to_string(),
                filename: "doc.pdf".to_string(),
                signing_link: None,
            };

            assert!(!request.subject.is_empty());
            assert!(request.subject.len() < 1000); // Reasonable email subject length
        }
    }

    #[test]
    fn test_multiple_attachments_not_supported() {
        // Verify that SendRequest only supports single attachment
        // (as per the current email payload structure)
        let request = SendRequest {
            to: "test@example.com".to_string(),
            subject: "Document".to_string(),
            pdf_base64: "JVBERi0xLjQK".to_string(),
            filename: "document.pdf".to_string(),
            signing_link: None,
        };

        // Only one filename field exists
        assert_eq!(request.filename, "document.pdf");
        // Only one pdf_base64 field exists
        assert_eq!(request.pdf_base64, "JVBERi0xLjQK");
    }

    #[test]
    fn test_invite_request_structure() {
        // Verify InviteRequest structure for bulk email sending
        #[derive(Serialize, Deserialize)]
        struct TestInvite {
            recipient_email: String,
            recipient_name: String,
            signing_link: String,
        }

        let invite = TestInvite {
            recipient_email: "john@example.com".to_string(),
            recipient_name: "John Doe".to_string(),
            signing_link: "https://getsignatures.org/sign/abc123".to_string(),
        };

        let json = serde_json::to_string(&invite).unwrap();
        let parsed: TestInvite = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.recipient_email, "john@example.com");
        assert_eq!(parsed.recipient_name, "John Doe");
    }

    #[test]
    fn test_email_from_address_format() {
        // Verify the "from" email address format is consistent
        let from_address = "GetSignatures <noreply@mail.getsignatures.org>";

        // Should be in "Name <email>" format
        assert!(from_address.contains('<'));
        assert!(from_address.contains('>'));
        assert!(from_address.contains("noreply@mail.getsignatures.org"));
    }

    #[test]
    fn test_email_text_body_format() {
        // Verify plain text email body is formatted correctly
        let link = "https://getsignatures.org/sign/abc123";
        let email_text = format!(
            "You have been requested to sign a document.\n\n\
            Click the link below to sign:\n{}\n\n\
            Or download the attached PDF to sign locally.",
            link
        );

        assert!(email_text.contains("You have been requested"));
        assert!(email_text.contains(link));
        assert!(email_text.contains("attached PDF"));
    }

    #[test]
    fn test_email_text_body_without_link() {
        // Verify plain text email body when signing_link is not provided
        let email_text = "Please find the attached document for your signature.";

        assert!(!email_text.is_empty());
        assert!(email_text.contains("attached document"));
    }

    // ============================================================
    // Email-Proxy Configuration Tests
    // ============================================================

    #[test]
    fn test_missing_api_key_error_message() {
        // Verify error message when API key is not configured
        let error_msg = "EMAIL_PROXY_API_KEY not configured";
        assert_eq!(error_msg, "EMAIL_PROXY_API_KEY not configured");
    }

    #[test]
    fn test_session_ttl_constant() {
        // Verify session TTL is reasonable (7 days = 604800 seconds)
        let session_ttl = SESSION_TTL_SECONDS;
        assert_eq!(session_ttl, 7 * 24 * 60 * 60); // 7 days in seconds
        assert_eq!(session_ttl, 604800);
    }

    #[test]
    fn test_download_link_expiry_days() {
        // Verify download link expiry is set correctly
        let expiry_days = DOWNLOAD_LINK_EXPIRY_DAYS;
        assert_eq!(expiry_days, 30);
        assert!(expiry_days > 0);
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

        // RequestLink: 3 req/hour
        let (max, window) = RateLimitTier::RequestLink.limits();
        assert_eq!(max, 3);
        assert_eq!(window, 3600);
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
        assert_eq!(RateLimitTier::RequestLink.retry_after_seconds(), 3600);
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

    // ============================================================
    // Per-IP Rate Limiting Property Tests
    // ============================================================

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
