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
            "Daily email quota exceeded: {}/{}",
            quota.daily_count,
            DAILY_EMAIL_LIMIT
        );
        return Ok(EmailSendResult::error(
            "Daily email limit reached. Please try again tomorrow.",
        ));
    }

    if quota.monthly_count >= MONTHLY_EMAIL_LIMIT {
        console_log!(
            "Monthly email quota exceeded: {}/{}",
            quota.monthly_count,
            MONTHLY_EMAIL_LIMIT
        );
        return Ok(EmailSendResult::error(
            "Monthly email limit reached. Please try again next month.",
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
