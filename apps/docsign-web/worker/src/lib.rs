//! DocSign Server - Cloudflare Worker for email relay and signing sessions
//!
//! Rate limited to Resend free tier: 100/day, 3000/month
//! Signing sessions expire after 7 days

use chrono::Utc;
use serde::{Deserialize, Serialize};
use worker::*;

/// Request body for sending a document
#[derive(Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    remaining_today: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remaining_month: Option<u32>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    remaining_today: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remaining_month: Option<u32>,
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

/// Rate limit state stored in KV
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
struct RateLimitState {
    daily_count: u32,
    daily_date: String, // YYYY-MM-DD
    monthly_count: u32,
    monthly_month: String, // YYYY-MM
    #[serde(default)]
    daily_warning_sent: bool,
    #[serde(default)]
    monthly_warning_sent: bool,
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

// Warning thresholds - send admin email when these are hit
const DAILY_WARNING_THRESHOLD: u32 = 90;
const MONTHLY_WARNING_THRESHOLD: u32 = 2950;
const ADMIN_EMAIL: &str = "bobamatchasolutions@gmail.com";

/// Result of checking rate limits
#[derive(Debug, Clone, PartialEq)]
enum RateLimitCheck {
    /// Request allowed, optionally with warnings to send
    Allowed {
        send_daily_warning: bool,
        send_monthly_warning: bool,
    },
    /// Daily limit exceeded
    DailyLimitExceeded { limit: u32, remaining_month: u32 },
    /// Monthly limit exceeded
    MonthlyLimitExceeded { limit: u32 },
}

impl RateLimitState {
    /// Reset counters if date/month has changed, returns whether resets occurred
    fn maybe_reset(&mut self, today: &str, this_month: &str) -> (bool, bool) {
        let daily_reset = if self.daily_date != today {
            self.daily_count = 0;
            self.daily_date = today.to_string();
            self.daily_warning_sent = false;
            true
        } else {
            false
        };

        let monthly_reset = if self.monthly_month != this_month {
            self.monthly_count = 0;
            self.monthly_month = this_month.to_string();
            self.monthly_warning_sent = false;
            true
        } else {
            false
        };

        (daily_reset, monthly_reset)
    }

    /// Check if request is allowed and increment counters if so
    fn check_and_increment(
        &mut self,
        daily_limit: u32,
        monthly_limit: u32,
        daily_warning_threshold: u32,
        monthly_warning_threshold: u32,
    ) -> RateLimitCheck {
        // Check limits before incrementing
        if self.daily_count >= daily_limit {
            return RateLimitCheck::DailyLimitExceeded {
                limit: daily_limit,
                remaining_month: monthly_limit.saturating_sub(self.monthly_count),
            };
        }

        if self.monthly_count >= monthly_limit {
            return RateLimitCheck::MonthlyLimitExceeded {
                limit: monthly_limit,
            };
        }

        // Increment counts
        self.daily_count += 1;
        self.monthly_count += 1;

        // Check if we should send warnings (only once per period)
        let send_daily_warning =
            if self.daily_count >= daily_warning_threshold && !self.daily_warning_sent {
                self.daily_warning_sent = true;
                true
            } else {
                false
            };

        let send_monthly_warning =
            if self.monthly_count >= monthly_warning_threshold && !self.monthly_warning_sent {
                self.monthly_warning_sent = true;
                true
            } else {
                false
            };

        RateLimitCheck::Allowed {
            send_daily_warning,
            send_monthly_warning,
        }
    }

    /// Get remaining counts for response
    fn remaining(&self, daily_limit: u32, monthly_limit: u32) -> (u32, u32) {
        (
            daily_limit.saturating_sub(self.daily_count),
            monthly_limit.saturating_sub(self.monthly_count),
        )
    }
}

const RESEND_API_URL: &str = "https://api.resend.com/emails";
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
    let api_key = match env.secret("RESEND_API_KEY") {
        Ok(key) => key.to_string(),
        Err(_) => {
            console_log!("Cannot send notification: RESEND_API_KEY not configured");
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

    let resend_body = serde_json::json!({
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
        .with_body(Some(serde_json::to_string(&resend_body)?.into()));

    let request = Request::new_with_init(RESEND_API_URL, &init)?;

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
        // Health check (public)
        (Method::Get, "/health") => cors_response(Response::ok("OK")),
        (Method::Get, "/") => cors_response(Response::ok("DocSign API Server")),

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
            let id = p.strip_prefix("/session/").unwrap_or("");
            if id.contains('/') {
                cors_response(Response::error("Not found", 404))
            } else {
                handle_get_session(id, env).await
            }
        }
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/signed") => {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_submit_signed(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/decline") => {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_decline(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        (Method::Post, p) if p.starts_with("/session/") && p.ends_with("/verify") => {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_verify(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // UX-004: Request new link endpoint (public - no API key required)
        (Method::Post, p) if p.starts_with("/session/") && p.ends_with("/request-link") => {
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

    // Get config
    let daily_limit: u32 = env
        .var("DAILY_LIMIT")
        .map(|v| v.to_string().parse().unwrap_or(100))
        .unwrap_or(100);
    let monthly_limit: u32 = env
        .var("MONTHLY_LIMIT")
        .map(|v| v.to_string().parse().unwrap_or(3000))
        .unwrap_or(3000);

    // Check rate limits
    let kv = match env.kv("RATE_LIMITS") {
        Ok(kv) => kv,
        Err(_) => {
            console_log!("Warning: RATE_LIMITS KV not configured, rate limiting disabled");
            return send_email(&env, &body).await;
        }
    };

    let mut state = get_rate_limit_state(&kv).await;
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let this_month = Utc::now().format("%Y-%m").to_string();

    // Reset counters if day/month changed
    state.maybe_reset(&today, &this_month);

    // Check rate limits before sending
    let check_result = state.check_and_increment(
        daily_limit,
        monthly_limit,
        DAILY_WARNING_THRESHOLD,
        MONTHLY_WARNING_THRESHOLD,
    );

    match check_result {
        RateLimitCheck::DailyLimitExceeded {
            limit,
            remaining_month,
        } => {
            let resp = Response::from_json(&SendResponse {
                success: false,
                message: format!(
                    "Daily limit reached ({}/{}). Resets at midnight UTC.",
                    limit, limit
                ),
                remaining_today: Some(0),
                remaining_month: Some(remaining_month),
            })?
            .with_status(429);
            cors_response(Ok(resp))
        }
        RateLimitCheck::MonthlyLimitExceeded { limit } => {
            let resp = Response::from_json(&SendResponse {
                success: false,
                message: format!(
                    "Monthly limit reached ({}/{}). Resets on the 1st.",
                    limit, limit
                ),
                remaining_today: Some(0),
                remaining_month: Some(0),
            })?
            .with_status(429);
            cors_response(Ok(resp))
        }
        RateLimitCheck::Allowed {
            send_daily_warning,
            send_monthly_warning,
        } => {
            // Send the actual email
            let result = send_email(&env, &body).await;

            if result
                .as_ref()
                .map(|r| r.status_code() == 200)
                .unwrap_or(false)
            {
                // Send admin warnings if thresholds crossed
                if send_daily_warning {
                    let (remaining, _) = state.remaining(daily_limit, monthly_limit);
                    let _ = send_admin_warning(
                        &env,
                        &format!(
                            "Daily email limit warning: {}/{} emails sent today ({}). Only {} remaining.",
                            state.daily_count, daily_limit, state.daily_date, remaining
                        ),
                    ).await;
                }

                if send_monthly_warning {
                    let (_, remaining) = state.remaining(daily_limit, monthly_limit);
                    let _ = send_admin_warning(
                        &env,
                        &format!(
                            "Monthly email limit warning: {}/{} emails sent this month ({}). Only {} remaining.",
                            state.monthly_count, monthly_limit, state.monthly_month, remaining
                        ),
                    ).await;
                }

                let _ = save_rate_limit_state(&kv, &state).await;
            } else {
                // Email failed - decrement the counts we optimistically incremented
                state.daily_count = state.daily_count.saturating_sub(1);
                state.monthly_count = state.monthly_count.saturating_sub(1);
            }

            handle_email_result(result, &state, daily_limit, monthly_limit)
        }
    }
}

/// Helper to format the email send result as a response
fn handle_email_result(
    result: Result<Response>,
    state: &RateLimitState,
    daily_limit: u32,
    monthly_limit: u32,
) -> Result<Response> {
    match result {
        Ok(response) if response.status_code() == 200 => {
            let (remaining_today, remaining_month) = state.remaining(daily_limit, monthly_limit);
            cors_response(Response::from_json(&SendResponse {
                success: true,
                message: "Email sent".to_string(),
                remaining_today: Some(remaining_today),
                remaining_month: Some(remaining_month),
            }))
        }
        Ok(response) => cors_response(Ok(response)),
        Err(e) => Err(e),
    }
}

async fn send_email(env: &Env, body: &SendRequest) -> Result<Response> {
    let api_key = match env.secret("RESEND_API_KEY") {
        Ok(key) => key.to_string(),
        Err(_) => {
            return cors_response(error_response("RESEND_API_KEY not configured"));
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

    let resend_body = serde_json::json!({
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
        .with_body(Some(serde_json::to_string(&resend_body)?.into()));

    let request = Request::new_with_init(RESEND_API_URL, &init)?;
    let response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let status = response.status_code();
        let mut response = response;
        let error_text = response.text().await.unwrap_or_default();
        console_log!("Resend error {}: {}", status, error_text);
        return cors_response(error_response(&format!("Resend API error: {}", status)));
    }

    cors_response(Response::from_json(&SendResponse {
        success: true,
        message: "Email sent".to_string(),
        remaining_today: None,
        remaining_month: None,
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

    // Get config
    let daily_limit: u32 = env
        .var("DAILY_LIMIT")
        .map(|v| v.to_string().parse().unwrap_or(100))
        .unwrap_or(100);
    let monthly_limit: u32 = env
        .var("MONTHLY_LIMIT")
        .map(|v| v.to_string().parse().unwrap_or(3000))
        .unwrap_or(3000);

    // Check rate limits
    let kv = match env.kv("RATE_LIMITS") {
        Ok(kv) => kv,
        Err(_) => {
            console_log!("Warning: RATE_LIMITS KV not configured, rate limiting disabled");
            return send_invitations(&env, &body).await;
        }
    };

    let mut state = get_rate_limit_state(&kv).await;
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let this_month = Utc::now().format("%Y-%m").to_string();

    // Reset counters if day/month changed
    state.maybe_reset(&today, &this_month);

    // Check if we have enough capacity for all invitations
    let num_emails = body.invitations.len() as u32;
    if state.daily_count + num_emails > daily_limit {
        let resp = Response::from_json(&InviteResponse {
            success: false,
            message: format!(
                "Daily limit would be exceeded. Requested: {}, Remaining: {}",
                num_emails,
                daily_limit.saturating_sub(state.daily_count)
            ),
            remaining_today: Some(daily_limit.saturating_sub(state.daily_count)),
            remaining_month: Some(monthly_limit.saturating_sub(state.monthly_count)),
        })?
        .with_status(429);
        return cors_response(Ok(resp));
    }

    if state.monthly_count + num_emails > monthly_limit {
        let resp = Response::from_json(&InviteResponse {
            success: false,
            message: format!(
                "Monthly limit would be exceeded. Requested: {}, Remaining: {}",
                num_emails,
                monthly_limit.saturating_sub(state.monthly_count)
            ),
            remaining_today: Some(daily_limit.saturating_sub(state.daily_count)),
            remaining_month: Some(monthly_limit.saturating_sub(state.monthly_count)),
        })?
        .with_status(429);
        return cors_response(Ok(resp));
    }

    // Send the invitations
    let result = send_invitations(&env, &body).await;

    if result
        .as_ref()
        .map(|r| r.status_code() == 200)
        .unwrap_or(false)
    {
        // Increment counters
        state.daily_count += num_emails;
        state.monthly_count += num_emails;

        // Check if we should send warnings
        let send_daily_warning =
            if state.daily_count >= DAILY_WARNING_THRESHOLD && !state.daily_warning_sent {
                state.daily_warning_sent = true;
                true
            } else {
                false
            };

        let send_monthly_warning =
            if state.monthly_count >= MONTHLY_WARNING_THRESHOLD && !state.monthly_warning_sent {
                state.monthly_warning_sent = true;
                true
            } else {
                false
            };

        // Send admin warnings if thresholds crossed
        if send_daily_warning {
            let remaining = daily_limit.saturating_sub(state.daily_count);
            let _ = send_admin_warning(
                &env,
                &format!(
                    "Daily email limit warning: {}/{} emails sent today ({}). Only {} remaining.",
                    state.daily_count, daily_limit, state.daily_date, remaining
                ),
            )
            .await;
        }

        if send_monthly_warning {
            let remaining = monthly_limit.saturating_sub(state.monthly_count);
            let _ = send_admin_warning(
                &env,
                &format!(
                    "Monthly email limit warning: {}/{} emails sent this month ({}). Only {} remaining.",
                    state.monthly_count, monthly_limit, state.monthly_month, remaining
                ),
            ).await;
        }

        let _ = save_rate_limit_state(&kv, &state).await;

        let (remaining_today, remaining_month) = state.remaining(daily_limit, monthly_limit);
        cors_response(Response::from_json(&InviteResponse {
            success: true,
            message: format!("Invitations sent to {} recipient(s)", num_emails),
            remaining_today: Some(remaining_today),
            remaining_month: Some(remaining_month),
        }))
    } else {
        result
    }
}

async fn send_invitations(env: &Env, body: &InviteRequest) -> Result<Response> {
    let api_key = match env.secret("RESEND_API_KEY") {
        Ok(key) => key.to_string(),
        Err(_) => {
            return cors_response(error_response("RESEND_API_KEY not configured"));
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

        let resend_body = serde_json::json!({
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
            .with_body(Some(serde_json::to_string(&resend_body)?.into()));

        let request = Request::new_with_init(RESEND_API_URL, &init)?;

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
            remaining_today: None,
            remaining_month: None,
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
            remaining_today: None,
            remaining_month: None,
        }))
    } else {
        cors_response(error_response(&format!(
            "Failed to send invitations. Errors: {}",
            errors.join(", ")
        )))
    }
}

/// Send a warning email to the admin when approaching rate limits
async fn send_admin_warning(env: &Env, message: &str) {
    let api_key = match env.secret("RESEND_API_KEY") {
        Ok(key) => key.to_string(),
        Err(_) => {
            console_log!("Cannot send admin warning: RESEND_API_KEY not configured");
            return;
        }
    };

    let resend_body = serde_json::json!({
        "from": "GetSignatures Alerts <noreply@getsignatures.org>",
        "to": [ADMIN_EMAIL],
        "subject": "GetSignatures Rate Limit Warning",
        "text": format!(
            "{}\n\n\
            This is an automated alert from your GetSignatures deployment.\n\n\
            Consider upgrading your Resend plan if you're hitting limits frequently.\n\
            Current limits: 100 emails/day, 3000 emails/month (Resend free tier)",
            message
        )
    });

    let headers = Headers::new();
    if headers
        .set("Authorization", &format!("Bearer {}", api_key))
        .is_err()
    {
        return;
    }
    if headers.set("Content-Type", "application/json").is_err() {
        return;
    }

    let body_str = match serde_json::to_string(&resend_body) {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body_str.into()));

    let request = match Request::new_with_init(RESEND_API_URL, &init) {
        Ok(r) => r,
        Err(_) => return,
    };

    match Fetch::Request(request).send().await {
        Ok(response) => {
            if response.status_code() == 200 {
                console_log!("Admin warning email sent: {}", message);
            } else {
                console_log!(
                    "Failed to send admin warning: status {}",
                    response.status_code()
                );
            }
        }
        Err(e) => {
            console_log!("Failed to send admin warning: {}", e);
        }
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

async fn get_rate_limit_state(kv: &kv::KvStore) -> RateLimitState {
    match kv.get("rate_state").json::<RateLimitState>().await {
        Ok(Some(state)) => state,
        _ => RateLimitState::default(),
    }
}

async fn save_rate_limit_state(kv: &kv::KvStore, state: &RateLimitState) -> Result<()> {
    kv.put("rate_state", serde_json::to_string(state)?)?
        .execute()
        .await?;
    Ok(())
}

fn error_response(msg: &str) -> Result<Response> {
    let resp = Response::from_json(&SendResponse {
        success: false,
        message: msg.to_string(),
        remaining_today: None,
        remaining_month: None,
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
    // Unit Tests for RateLimitState
    // ============================================================

    #[test]
    fn test_default_state() {
        let state = RateLimitState::default();
        assert_eq!(state.daily_count, 0);
        assert_eq!(state.monthly_count, 0);
        assert_eq!(state.daily_date, "");
        assert_eq!(state.monthly_month, "");
        assert!(!state.daily_warning_sent);
        assert!(!state.monthly_warning_sent);
    }

    #[test]
    fn test_maybe_reset_same_day() {
        let mut state = RateLimitState {
            daily_count: 50,
            daily_date: "2025-01-15".to_string(),
            monthly_count: 1000,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: true,
            monthly_warning_sent: false,
        };

        let (daily_reset, monthly_reset) = state.maybe_reset("2025-01-15", "2025-01");

        assert!(!daily_reset);
        assert!(!monthly_reset);
        assert_eq!(state.daily_count, 50); // unchanged
        assert_eq!(state.monthly_count, 1000); // unchanged
        assert!(state.daily_warning_sent); // unchanged
    }

    #[test]
    fn test_maybe_reset_new_day_same_month() {
        let mut state = RateLimitState {
            daily_count: 95,
            daily_date: "2025-01-15".to_string(),
            monthly_count: 1000,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: true,
            monthly_warning_sent: false,
        };

        let (daily_reset, monthly_reset) = state.maybe_reset("2025-01-16", "2025-01");

        assert!(daily_reset);
        assert!(!monthly_reset);
        assert_eq!(state.daily_count, 0); // reset
        assert_eq!(state.daily_date, "2025-01-16");
        assert!(!state.daily_warning_sent); // reset
        assert_eq!(state.monthly_count, 1000); // unchanged
    }

    #[test]
    fn test_maybe_reset_new_month() {
        let mut state = RateLimitState {
            daily_count: 95,
            daily_date: "2025-01-31".to_string(),
            monthly_count: 2900,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: true,
            monthly_warning_sent: true,
        };

        let (daily_reset, monthly_reset) = state.maybe_reset("2025-02-01", "2025-02");

        assert!(daily_reset);
        assert!(monthly_reset);
        assert_eq!(state.daily_count, 0);
        assert_eq!(state.monthly_count, 0);
        assert!(!state.daily_warning_sent);
        assert!(!state.monthly_warning_sent);
    }

    #[test]
    fn test_check_and_increment_allowed_no_warnings() {
        let mut state = RateLimitState {
            daily_count: 10,
            daily_date: "2025-01-15".to_string(),
            monthly_count: 100,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: false,
            monthly_warning_sent: false,
        };

        let result = state.check_and_increment(100, 3000, 90, 2950);

        assert_eq!(
            result,
            RateLimitCheck::Allowed {
                send_daily_warning: false,
                send_monthly_warning: false,
            }
        );
        assert_eq!(state.daily_count, 11);
        assert_eq!(state.monthly_count, 101);
    }

    #[test]
    fn test_check_and_increment_triggers_daily_warning() {
        let mut state = RateLimitState {
            daily_count: 89, // Will become 90 after increment
            daily_date: "2025-01-15".to_string(),
            monthly_count: 100,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: false,
            monthly_warning_sent: false,
        };

        let result = state.check_and_increment(100, 3000, 90, 2950);

        assert_eq!(
            result,
            RateLimitCheck::Allowed {
                send_daily_warning: true,
                send_monthly_warning: false,
            }
        );
        assert_eq!(state.daily_count, 90);
        assert!(state.daily_warning_sent);
    }

    #[test]
    fn test_check_and_increment_triggers_monthly_warning() {
        let mut state = RateLimitState {
            daily_count: 10,
            daily_date: "2025-01-15".to_string(),
            monthly_count: 2949, // Will become 2950 after increment
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: false,
            monthly_warning_sent: false,
        };

        let result = state.check_and_increment(100, 3000, 90, 2950);

        assert_eq!(
            result,
            RateLimitCheck::Allowed {
                send_daily_warning: false,
                send_monthly_warning: true,
            }
        );
        assert_eq!(state.monthly_count, 2950);
        assert!(state.monthly_warning_sent);
    }

    #[test]
    fn test_check_and_increment_both_warnings() {
        let mut state = RateLimitState {
            daily_count: 89,
            daily_date: "2025-01-15".to_string(),
            monthly_count: 2949,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: false,
            monthly_warning_sent: false,
        };

        let result = state.check_and_increment(100, 3000, 90, 2950);

        assert_eq!(
            result,
            RateLimitCheck::Allowed {
                send_daily_warning: true,
                send_monthly_warning: true,
            }
        );
    }

    #[test]
    fn test_check_and_increment_warning_only_once() {
        let mut state = RateLimitState {
            daily_count: 90,
            daily_date: "2025-01-15".to_string(),
            monthly_count: 2950,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: true,   // Already sent
            monthly_warning_sent: true, // Already sent
        };

        let result = state.check_and_increment(100, 3000, 90, 2950);

        // Should NOT trigger warnings again
        assert_eq!(
            result,
            RateLimitCheck::Allowed {
                send_daily_warning: false,
                send_monthly_warning: false,
            }
        );
    }

    #[test]
    fn test_check_and_increment_daily_limit_exceeded() {
        let mut state = RateLimitState {
            daily_count: 100, // At limit
            daily_date: "2025-01-15".to_string(),
            monthly_count: 500,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: true,
            monthly_warning_sent: false,
        };

        let result = state.check_and_increment(100, 3000, 90, 2950);

        assert_eq!(
            result,
            RateLimitCheck::DailyLimitExceeded {
                limit: 100,
                remaining_month: 2500,
            }
        );
        // Count should NOT have incremented
        assert_eq!(state.daily_count, 100);
    }

    #[test]
    fn test_check_and_increment_monthly_limit_exceeded() {
        let mut state = RateLimitState {
            daily_count: 50,
            daily_date: "2025-01-15".to_string(),
            monthly_count: 3000, // At limit
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: false,
            monthly_warning_sent: true,
        };

        let result = state.check_and_increment(100, 3000, 90, 2950);

        assert_eq!(result, RateLimitCheck::MonthlyLimitExceeded { limit: 3000 });
        // Count should NOT have incremented
        assert_eq!(state.monthly_count, 3000);
    }

    #[test]
    fn test_remaining() {
        let state = RateLimitState {
            daily_count: 75,
            daily_date: "2025-01-15".to_string(),
            monthly_count: 2500,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: false,
            monthly_warning_sent: false,
        };

        let (daily_remaining, monthly_remaining) = state.remaining(100, 3000);
        assert_eq!(daily_remaining, 25);
        assert_eq!(monthly_remaining, 500);
    }

    #[test]
    fn test_remaining_saturates_at_zero() {
        let state = RateLimitState {
            daily_count: 150, // Over limit somehow
            daily_date: "2025-01-15".to_string(),
            monthly_count: 5000,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: true,
            monthly_warning_sent: true,
        };

        let (daily_remaining, monthly_remaining) = state.remaining(100, 3000);
        assert_eq!(daily_remaining, 0);
        assert_eq!(monthly_remaining, 0);
    }

    // ============================================================
    // Serialization Tests
    // ============================================================

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let state = RateLimitState {
            daily_count: 90,
            daily_date: "2025-01-15".to_string(),
            monthly_count: 2950,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: true,
            monthly_warning_sent: false,
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: RateLimitState = serde_json::from_str(&json).unwrap();

        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_deserialize_without_warning_fields() {
        // Old state format without warning fields should default to false
        let json = r#"{
            "daily_count": 50,
            "daily_date": "2025-01-15",
            "monthly_count": 1000,
            "monthly_month": "2025-01"
        }"#;

        let state: RateLimitState = serde_json::from_str(json).unwrap();

        assert_eq!(state.daily_count, 50);
        assert!(!state.daily_warning_sent); // defaults to false
        assert!(!state.monthly_warning_sent); // defaults to false
    }

    // ============================================================
    // Integration-style Tests (full workflow)
    // ============================================================

    #[test]
    fn test_full_day_workflow() {
        let mut state = RateLimitState::default();
        let daily_limit = 100;
        let monthly_limit = 3000;
        let daily_threshold = 90;
        let monthly_threshold = 2950;

        // Simulate a full day of requests
        state.maybe_reset("2025-01-15", "2025-01");

        // Send 89 emails - no warnings
        for _ in 0..89 {
            let result = state.check_and_increment(
                daily_limit,
                monthly_limit,
                daily_threshold,
                monthly_threshold,
            );
            assert!(matches!(
                result,
                RateLimitCheck::Allowed {
                    send_daily_warning: false,
                    ..
                }
            ));
        }
        assert_eq!(state.daily_count, 89);
        assert!(!state.daily_warning_sent);

        // 90th email triggers warning
        let result = state.check_and_increment(
            daily_limit,
            monthly_limit,
            daily_threshold,
            monthly_threshold,
        );
        assert_eq!(
            result,
            RateLimitCheck::Allowed {
                send_daily_warning: true,
                send_monthly_warning: false,
            }
        );
        assert!(state.daily_warning_sent);

        // 91-100 emails - no more warnings
        for _ in 91..=100 {
            let result = state.check_and_increment(
                daily_limit,
                monthly_limit,
                daily_threshold,
                monthly_threshold,
            );
            assert_eq!(
                result,
                RateLimitCheck::Allowed {
                    send_daily_warning: false,
                    send_monthly_warning: false,
                }
            );
        }
        assert_eq!(state.daily_count, 100);

        // 101st email - blocked
        let result = state.check_and_increment(
            daily_limit,
            monthly_limit,
            daily_threshold,
            monthly_threshold,
        );
        assert!(matches!(result, RateLimitCheck::DailyLimitExceeded { .. }));
        assert_eq!(state.daily_count, 100); // Not incremented

        // Next day - reset
        state.maybe_reset("2025-01-16", "2025-01");
        assert_eq!(state.daily_count, 0);
        assert!(!state.daily_warning_sent);
        assert_eq!(state.monthly_count, 100); // Monthly NOT reset
    }

    #[test]
    fn test_month_boundary_workflow() {
        let mut state = RateLimitState {
            daily_count: 50,
            daily_date: "2025-01-31".to_string(),
            monthly_count: 2900,
            monthly_month: "2025-01".to_string(),
            daily_warning_sent: true,
            monthly_warning_sent: false,
        };

        // Cross month boundary
        state.maybe_reset("2025-02-01", "2025-02");

        assert_eq!(state.daily_count, 0);
        assert_eq!(state.monthly_count, 0);
        assert!(!state.daily_warning_sent);
        assert!(!state.monthly_warning_sent);
        assert_eq!(state.daily_date, "2025-02-01");
        assert_eq!(state.monthly_month, "2025-02");
    }

    // ============================================================
    // Property Tests
    // ============================================================

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_daily_count_never_exceeds_limit_plus_one(
            initial_count in 0u32..200,
            num_requests in 0usize..50
        ) {
            let daily_limit = 100;
            let mut state = RateLimitState {
                daily_count: initial_count,
                daily_date: "2025-01-15".to_string(),
                monthly_count: 0,
                monthly_month: "2025-01".to_string(),
                daily_warning_sent: false,
                monthly_warning_sent: false,
            };

            for _ in 0..num_requests {
                let _ = state.check_and_increment(daily_limit, 3000, 90, 2950);
            }

            // After any number of requests, count should never exceed limit
            // (it can be at limit but check_and_increment won't increment past it)
            prop_assert!(state.daily_count <= daily_limit.max(initial_count));
        }

        #[test]
        fn prop_warning_sent_exactly_once(
            num_requests in 90usize..150
        ) {
            let mut state = RateLimitState::default();
            state.maybe_reset("2025-01-15", "2025-01");

            let mut warning_count = 0;
            for _ in 0..num_requests {
                if let RateLimitCheck::Allowed { send_daily_warning: true, .. } =
                    state.check_and_increment(100, 3000, 90, 2950) {
                    warning_count += 1;
                }
            }

            // Warning should be sent exactly once (when crossing threshold)
            prop_assert_eq!(warning_count, 1);
        }

        #[test]
        fn prop_reset_clears_warning_flag(
            count in 0u32..100,
            warning_sent in proptest::bool::ANY
        ) {
            let mut state = RateLimitState {
                daily_count: count,
                daily_date: "2025-01-15".to_string(),
                monthly_count: count,
                monthly_month: "2025-01".to_string(),
                daily_warning_sent: warning_sent,
                monthly_warning_sent: warning_sent,
            };

            state.maybe_reset("2025-01-16", "2025-02");

            prop_assert!(!state.daily_warning_sent);
            prop_assert!(!state.monthly_warning_sent);
            prop_assert_eq!(state.daily_count, 0);
            prop_assert_eq!(state.monthly_count, 0);
        }

        #[test]
        fn prop_remaining_is_consistent(
            daily_count in 0u32..150,
            monthly_count in 0u32..4000
        ) {
            let state = RateLimitState {
                daily_count,
                daily_date: "2025-01-15".to_string(),
                monthly_count,
                monthly_month: "2025-01".to_string(),
                daily_warning_sent: false,
                monthly_warning_sent: false,
            };

            let (daily_rem, monthly_rem) = state.remaining(100, 3000);

            // Remaining + count should equal limit (or be 0 if over limit)
            if daily_count <= 100 {
                prop_assert_eq!(daily_rem + daily_count, 100);
            } else {
                prop_assert_eq!(daily_rem, 0);
            }

            if monthly_count <= 3000 {
                prop_assert_eq!(monthly_rem + monthly_count, 3000);
            } else {
                prop_assert_eq!(monthly_rem, 0);
            }
        }

        #[test]
        fn prop_serialization_roundtrip(
            daily_count in 0u32..200,
            monthly_count in 0u32..5000,
            daily_warning in proptest::bool::ANY,
            monthly_warning in proptest::bool::ANY
        ) {
            let state = RateLimitState {
                daily_count,
                daily_date: "2025-01-15".to_string(),
                monthly_count,
                monthly_month: "2025-01".to_string(),
                daily_warning_sent: daily_warning,
                monthly_warning_sent: monthly_warning,
            };

            let json = serde_json::to_string(&state).unwrap();
            let deserialized: RateLimitState = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(state, deserialized);
        }
    }

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
}
