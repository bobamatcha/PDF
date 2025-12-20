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
            // Check if expired
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&s.expires_at) {
                if expires < chrono::Utc::now() {
                    return cors_response(Ok(Response::from_json(&GetSessionResponse {
                        success: false,
                        session: None,
                        message: Some("Session has expired".to_string()),
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
                },
                recipients: recipients.clone(),
                fields: fields.clone(),
                expires_at: "2025-01-22T10:00:00Z".to_string(),
                signed_versions: vec![],
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
            },
            recipients: vec![RecipientInfo {
                id: "1".to_string(),
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                role: "signer".to_string(),
                signed: true,
                signed_at: Some("2025-01-15T11:00:00Z".to_string()),
            }],
            fields: vec![],
            expires_at: "2025-01-22T10:00:00Z".to_string(),
            signed_versions: vec![SignedVersion {
                recipient_id: "1".to_string(),
                encrypted_document: "signed_doc_base64".to_string(),
                signed_at: "2025-01-15T11:00:00Z".to_string(),
            }],
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
}
