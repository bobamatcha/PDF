//! Email service module using Resend API directly
//!
//! All emails are sent directly through Resend API with:
//! - Global quota tracking (100/day, 3000/month)
//! - Automatic quota reset at midnight UTC (daily) and 1st of month (monthly)

pub mod resend;

use chrono::Datelike;
use resend::{send_via_resend, ResendConfig};
use serde::{Deserialize, Serialize};
use worker::{console_log, kv::KvStore, Env, Result};

/// Default from address for emails
/// NOTE: Using getsignatures.org (not mail.getsignatures.org) - DNS verified for Resend
pub const DEFAULT_FROM_ADDRESS: &str = "GetSignatures <noreply@getsignatures.org>";

/// Daily email limit (Resend free tier)
pub const DAILY_EMAIL_LIMIT: u32 = 100;

/// Monthly email limit (Resend free tier)
pub const MONTHLY_EMAIL_LIMIT: u32 = 3000;

/// Key for global email quota in RATE_LIMITS namespace
const EMAIL_QUOTA_KEY: &str = "email_quota:global";

/// Request to send an email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSendRequest {
    /// Recipient email addresses
    pub to: Vec<String>,
    /// Email subject line
    pub subject: String,
    /// HTML body content
    pub html: String,
    /// Optional plain text body (for clients that don't support HTML)
    #[serde(default)]
    pub text: Option<String>,
    /// Optional reply-to address
    #[serde(default)]
    pub reply_to: Option<String>,
    /// Optional tags for tracking (name, value pairs)
    #[serde(default)]
    pub tags: Vec<(String, String)>,
}

/// Result of sending an email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSendResult {
    /// Provider-assigned message ID
    pub id: String,
    /// Whether the send was successful
    pub success: bool,
    /// Error message if send failed
    #[serde(default)]
    pub error: Option<String>,
}

impl EmailSendResult {
    /// Create a successful result
    pub fn success(id: String) -> Self {
        Self {
            id,
            success: true,
            error: None,
        }
    }

    /// Create a failed result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            id: String::new(),
            success: false,
            error: Some(message.into()),
        }
    }
}

/// Global email quota tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailQuota {
    /// Number of emails sent today
    pub daily_count: u32,
    /// When the daily counter resets (ISO 8601)
    pub daily_reset_at: String,
    /// Number of emails sent this month
    pub monthly_count: u32,
    /// When the monthly counter resets (ISO 8601)
    pub monthly_reset_at: String,
}

impl Default for EmailQuota {
    fn default() -> Self {
        let now = chrono::Utc::now();
        let tomorrow = (now + chrono::Duration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let next_month = now
            .date_naive()
            .with_day(1)
            .unwrap()
            .checked_add_months(chrono::Months::new(1))
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        Self {
            daily_count: 0,
            daily_reset_at: tomorrow.to_rfc3339(),
            monthly_count: 0,
            monthly_reset_at: next_month.to_rfc3339(),
        }
    }
}

/// Email usage statistics
#[derive(Debug, Clone, Serialize)]
pub struct EmailUsage {
    pub daily_count: u32,
    pub daily_limit: u32,
    pub daily_remaining: u32,
    pub monthly_count: u32,
    pub monthly_limit: u32,
    pub monthly_remaining: u32,
    pub can_send: bool,
}

/// Get current email quota from KV
async fn get_email_quota(kv: &KvStore) -> Result<EmailQuota> {
    match kv.get(EMAIL_QUOTA_KEY).json::<EmailQuota>().await? {
        Some(quota) => Ok(quota),
        None => Ok(EmailQuota::default()),
    }
}

/// Save email quota to KV
async fn save_email_quota(kv: &KvStore, quota: &EmailQuota) -> Result<()> {
    kv.put(EMAIL_QUOTA_KEY, serde_json::to_string(quota)?)?
        .execute()
        .await?;
    Ok(())
}

/// Check if quota allows sending and reset counters if needed
async fn check_and_reset_quota(kv: &KvStore) -> Result<EmailQuota> {
    let mut quota = get_email_quota(kv).await?;
    let now = chrono::Utc::now();
    let mut needs_save = false;

    // Check and reset daily counter
    if let Ok(reset_at) = chrono::DateTime::parse_from_rfc3339(&quota.daily_reset_at) {
        if now > reset_at {
            console_log!("Resetting daily email counter");
            quota.daily_count = 0;
            quota.daily_reset_at = (now + chrono::Duration::days(1))
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .to_rfc3339();
            needs_save = true;
        }
    }

    // Check and reset monthly counter
    if let Ok(reset_at) = chrono::DateTime::parse_from_rfc3339(&quota.monthly_reset_at) {
        if now > reset_at {
            console_log!("Resetting monthly email counter");
            quota.monthly_count = 0;
            quota.monthly_reset_at = now
                .date_naive()
                .with_day(1)
                .unwrap()
                .checked_add_months(chrono::Months::new(1))
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .to_rfc3339();
            needs_save = true;
        }
    }

    if needs_save {
        save_email_quota(kv, &quota).await?;
    }

    Ok(quota)
}

/// Check if we can send an email (under quota)
pub async fn can_send_email(env: &Env) -> Result<bool> {
    let kv = env.kv("RATE_LIMITS")?;
    let quota = check_and_reset_quota(&kv).await?;

    Ok(quota.daily_count < DAILY_EMAIL_LIMIT && quota.monthly_count < MONTHLY_EMAIL_LIMIT)
}

/// Get current email usage statistics
pub async fn get_email_usage(env: &Env) -> Result<EmailUsage> {
    let kv = env.kv("RATE_LIMITS")?;
    let quota = check_and_reset_quota(&kv).await?;

    let daily_remaining = DAILY_EMAIL_LIMIT.saturating_sub(quota.daily_count);
    let monthly_remaining = MONTHLY_EMAIL_LIMIT.saturating_sub(quota.monthly_count);

    Ok(EmailUsage {
        daily_count: quota.daily_count,
        daily_limit: DAILY_EMAIL_LIMIT,
        daily_remaining,
        monthly_count: quota.monthly_count,
        monthly_limit: MONTHLY_EMAIL_LIMIT,
        monthly_remaining,
        can_send: daily_remaining > 0 && monthly_remaining > 0,
    })
}

/// Increment email quota after successful send
async fn increment_quota(kv: &KvStore, count: u32) -> Result<()> {
    let mut quota = get_email_quota(kv).await?;
    quota.daily_count += count;
    quota.monthly_count += count;
    save_email_quota(kv, &quota).await
}

/// Send an email via Resend
///
/// Checks quota before sending and increments counters after success.
///
/// # Arguments
/// * `env` - Cloudflare Worker environment (for RESEND_API_KEY)
/// * `request` - The email to send
///
/// # Returns
/// * `Ok(EmailSendResult)` - Result with success/failure and message ID
pub async fn send_email(env: &Env, request: EmailSendRequest) -> Result<EmailSendResult> {
    let kv = env.kv("RATE_LIMITS")?;

    // Check and reset quota
    let quota = check_and_reset_quota(&kv).await?;

    // Check if under quota
    if quota.daily_count >= DAILY_EMAIL_LIMIT {
        console_log!(
            "ALERT: Daily email quota exceeded: {}/{}",
            quota.daily_count,
            DAILY_EMAIL_LIMIT
        );
        return Ok(EmailSendResult::error(
            "We ran out of emails for today. Try again tomorrow, or hit us up at bobamatchsolutions @ gmail dot com if it's urgent.",
        ));
    }

    if quota.monthly_count >= MONTHLY_EMAIL_LIMIT {
        console_log!(
            "ALERT: Monthly email quota exceeded: {}/{}",
            quota.monthly_count,
            MONTHLY_EMAIL_LIMIT
        );
        return Ok(EmailSendResult::error(
            "We ran out of emails for the month. Try again next month, or email bobamatchsolutions @ gmail dot com and we'll sort it out.",
        ));
    }

    // Get from address
    let from_address = env
        .var("EMAIL_FROM")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| DEFAULT_FROM_ADDRESS.to_string());

    // Get Resend API key
    let resend_api_key = match env.secret("RESEND_API_KEY") {
        Ok(key) => key.to_string(),
        Err(_) => {
            console_log!("Error: RESEND_API_KEY not configured");
            return Ok(EmailSendResult::error(
                "Email service not configured. Please contact support.",
            ));
        }
    };

    let config = ResendConfig {
        api_key: resend_api_key,
        from_address,
    };

    // Count recipients
    let recipient_count = request.to.len() as u32;

    // Send via Resend
    let result = send_via_resend(&config, &request).await?;

    // If successful, increment quota
    if result.success {
        if let Err(e) = increment_quota(&kv, recipient_count).await {
            console_log!("Warning: Failed to increment email quota: {:?}", e);
        }
    }

    Ok(result)
}

/// Send admin notification email when a user submits feedback/request (Bug #4)
///
/// # Arguments
/// * `env` - Cloudflare Worker environment
/// * `request` - The user request to notify about
pub async fn send_admin_notification_email(
    env: &Env,
    user_email: &str,
    request_type: &str,
    description: &str,
    additional_documents: Option<u32>,
) -> Result<EmailSendResult> {
    let admin_email = "bobamatchsolutions@gmail.com";
    let subject = format!("[GetSignatures] New {}", request_type);

    let docs_section = if let Some(docs) = additional_documents {
        format!(
            r#"<div style="background: #fef3c7; border: 1px solid #fcd34d; border-radius: 8px; padding: 12px; margin: 16px 0;">
                <strong>Requested additional documents:</strong> {}
            </div>"#,
            docs
        )
    } else {
        String::new()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="font-family: -apple-system, sans-serif; line-height: 1.6; color: #334155; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h2 style="color: #0056b3;">New {request_type}</h2>

    <p><strong>From:</strong> {user_email}</p>

    <div style="background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 16px; margin: 16px 0;">
        <strong>Description:</strong>
        <p style="white-space: pre-wrap;">{description}</p>
    </div>

    {docs_section}

    <p style="color: #6b7280; font-size: 14px; margin-top: 24px;">
        Respond to this request in the admin dashboard or reply to the user directly.
    </p>
</body>
</html>"#,
        request_type = request_type,
        user_email = user_email,
        description = description,
        docs_section = docs_section
    );

    let request = EmailSendRequest {
        to: vec![admin_email.to_string()],
        subject,
        html,
        text: Some(format!(
            "New {} from {}\n\nDescription:\n{}\n\n{}",
            request_type,
            user_email,
            description,
            if let Some(docs) = additional_documents {
                format!("Requested documents: {}", docs)
            } else {
                String::new()
            }
        )),
        reply_to: Some(user_email.to_string()),
        tags: vec![
            ("type".to_string(), "admin_notification".to_string()),
            ("request_type".to_string(), request_type.to_string()),
        ],
    };

    console_log!(
        "Sending admin notification for {} from {}",
        request_type,
        user_email
    );
    send_email(env, request).await
}

/// Send limit notification email when user hits their monthly document limit
///
/// # Arguments
/// * `env` - Cloudflare Worker environment
/// * `user_email` - User's email address
/// * `user_first_name` - User's first name (for personalization)
/// * `tier_name` - User's tier display name (e.g., "Free", "Personal")
/// * `limit` - The monthly document limit they hit
///
/// # Returns
/// * `Ok(EmailSendResult)` - Result with success/failure
pub async fn send_limit_notification_email(
    env: &Env,
    user_email: &str,
    user_first_name: &str,
    tier_name: &str,
    limit: u32,
) -> Result<EmailSendResult> {
    // Calculate next month's 1st for the reset date
    let now = chrono::Utc::now();
    let next_month_1st = now
        .date_naive()
        .with_day(1)
        .unwrap()
        .checked_add_months(chrono::Months::new(1))
        .unwrap();
    let month_name = match next_month_1st.month() {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "next month",
    };

    let subject = "You've reached your document limit for this month";

    // HTML email template following UX best practices:
    // - Lead with what they CAN do
    // - Single focused CTA (upgrade)
    // - Clear reset date
    // - Mobile-friendly design
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Monthly Limit Reached</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #334155; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%); padding: 24px; border-radius: 12px 12px 0 0; text-align: center;">
        <h1 style="color: white; margin: 0; font-size: 24px;">📊 Monthly Limit Reached</h1>
    </div>

    <div style="background: #ffffff; border: 1px solid #e2e8f0; border-top: none; padding: 24px; border-radius: 0 0 12px 12px;">
        <p style="font-size: 16px; margin-bottom: 16px;">
            Hi {first_name},
        </p>

        <p style="font-size: 16px; margin-bottom: 16px;">
            You've sent all <strong>{limit} documents</strong> available on your <strong>{tier_name}</strong> plan this month.
        </p>

        <div style="background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 16px; margin-bottom: 20px;">
            <h3 style="margin: 0 0 12px 0; font-size: 14px; color: #475569;">What you can still do:</h3>
            <ul style="margin: 0; padding-left: 20px; color: #64748b; font-size: 14px;">
                <li>View your existing documents</li>
                <li>Download signed PDFs</li>
                <li>Check document status</li>
                <li>View your signing history</li>
            </ul>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-bottom: 20px; text-align: center;">
            New document sending will resume on <strong>{month_name} 1st</strong>.
        </p>

        <div style="text-align: center; margin-bottom: 20px;">
            <a href="https://getsignatures.org/pricing.html"
               style="display: inline-block; background: #0056b3; color: white; padding: 14px 32px; border-radius: 8px; text-decoration: none; font-weight: 600; font-size: 16px;">
                View Pricing &amp; Upgrade
            </a>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 24px; border-top: 1px solid #e2e8f0; padding-top: 16px;">
            Questions? Just reply to this email and we'll help you out.
        </p>

        <p style="font-size: 14px; color: #6b7280;">
            – The Get Signatures Team
        </p>
    </div>

    <div style="text-align: center; margin-top: 16px; font-size: 12px; color: #94a3b8;">
        <p>Get Signatures · Secure Document Signing</p>
    </div>
</body>
</html>"#,
        first_name = user_first_name,
        limit = limit,
        tier_name = tier_name,
        month_name = month_name
    );

    // Plain text version for email clients that don't support HTML
    let text = format!(
        r#"Hi {first_name},

You've sent all {limit} documents available on your {tier_name} plan this month.

What you can still do:
- View your existing documents
- Download signed PDFs
- Check document status
- View your signing history

New document sending will resume on {month_name} 1st.

Want to send more documents now? Upgrade your plan:
https://getsignatures.org/pricing.html

Questions? Just reply to this email.

– The Get Signatures Team"#,
        first_name = user_first_name,
        limit = limit,
        tier_name = tier_name,
        month_name = month_name
    );

    let request = EmailSendRequest {
        to: vec![user_email.to_string()],
        subject: subject.to_string(),
        html,
        text: Some(text),
        reply_to: Some("bobamatchsolutions@gmail.com".to_string()),
        tags: vec![
            ("type".to_string(), "limit_notification".to_string()),
            ("tier".to_string(), tier_name.to_string()),
        ],
    };

    console_log!(
        "Sending limit notification email to {} (tier: {}, limit: {})",
        user_email,
        tier_name,
        limit
    );

    send_email(env, request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_send_request_serialization() {
        let request = EmailSendRequest {
            to: vec!["test@example.com".to_string()],
            subject: "Test Subject".to_string(),
            html: "<p>Test</p>".to_string(),
            text: Some("Test".to_string()),
            reply_to: None,
            tags: vec![("type".to_string(), "test".to_string())],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("Test Subject"));
    }

    #[test]
    fn test_email_send_result_success() {
        let result = EmailSendResult::success("msg-123".to_string());
        assert!(result.success);
        assert_eq!(result.id, "msg-123");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_email_send_result_error() {
        let result = EmailSendResult::error("Something went wrong");
        assert!(!result.success);
        assert!(result.id.is_empty());
        assert_eq!(result.error.as_deref(), Some("Something went wrong"));
    }

    #[test]
    fn test_email_quota_default() {
        let quota = EmailQuota::default();
        assert_eq!(quota.daily_count, 0);
        assert_eq!(quota.monthly_count, 0);
        // Reset times should be in the future
        let now = chrono::Utc::now().to_rfc3339();
        assert!(quota.daily_reset_at > now);
        assert!(quota.monthly_reset_at > now);
    }

    #[test]
    fn test_email_usage_calculation() {
        // Simulate usage calculation
        let daily_count = 50;
        let monthly_count = 2500;

        let daily_remaining = DAILY_EMAIL_LIMIT.saturating_sub(daily_count);
        let monthly_remaining = MONTHLY_EMAIL_LIMIT.saturating_sub(monthly_count);

        assert_eq!(daily_remaining, 50); // 100 - 50
        assert_eq!(monthly_remaining, 500); // 3000 - 2500
    }

    #[test]
    fn test_quota_exceeded() {
        // Daily exceeded
        let daily_count = DAILY_EMAIL_LIMIT;
        assert!(daily_count >= DAILY_EMAIL_LIMIT);

        // Monthly exceeded
        let monthly_count = MONTHLY_EMAIL_LIMIT;
        assert!(monthly_count >= MONTHLY_EMAIL_LIMIT);
    }
}
