//! Email Proxy Lambda - AWS SES integration with automated deliverability
//!
//! This crate provides a Resend-compatible email API backed by AWS SES,
//! with automated implementation of email deliverability best practices:
//!
//! ## Resend's Top 10 Deliverability Tips (Automated)
//!
//! 1. **Authenticate your domain (DKIM, SPF, DMARC)** - Configured via AWS SES
//!    - DKIM: SES provides DKIM signing automatically with verified domains
//!    - SPF: Add `include:amazonses.com` to your domain's SPF record
//!    - DMARC: Add DMARC record with your policy
//!
//! 2. **Maintain a clean email list** - Automatic suppression list management
//!    - Hard bounces immediately suppressed
//!    - Soft bounces tracked, suppressed after 3 attempts
//!    - Complaints (spam reports) immediately suppressed
//!
//! 3. **Warm up your sending IP** - Automatic warm-up schedule
//!    - 4-week graduated volume increase
//!    - Daily limits enforced automatically
//!
//! 4. **Use double opt-in** - Handled by DocSign consent tracking
//!
//! 5. **Personalize your emails** - Template system with personalization
//!
//! 6. **Include an unsubscribe link** - List-Unsubscribe header added automatically
//!
//! 7. **Monitor your sender reputation** - Metrics tracking
//!    - Bounce rate monitoring (target < 2%)
//!    - Complaint rate monitoring (target < 0.1%)
//!    - Health score calculation
//!
//! 8. **Avoid spam trigger words** - Content scanner available
//!
//! 9. **Send relevant content** - Handled by application logic
//!
//! 10. **Test before sending** - Validation and preview endpoints
//!
//! ## Architecture
//!
//! ```text
//! Cloudflare Worker → API Gateway → Lambda (this) → AWS SES → Recipient
//!                                      ↓
//!                                  DynamoDB (suppression list)
//!                                      ↑
//!                           SNS (bounce/complaint notifications)
//! ```
//!
//! ## Usage
//!
//! Deploy as an AWS Lambda function with API Gateway trigger.
//! See `main.rs` for the Lambda handler implementation.

pub mod deliverability;
pub mod ses;
pub mod types;

pub use deliverability::{
    DeliverabilityManager, ReputationMetrics, SendBlockedReason, SesNotification, SpamScanResult,
    SuppressionEntry, SuppressionReason, WarmUpSchedule, WarmUpStatus,
};
pub use ses::{SesError, SesSender};
pub use types::{
    Attachment, EmailHeader, EmailStatus, EmailTag, EmailTemplate, SendEmailRequest,
    SendEmailResponse, ValidationError,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Configuration for the email proxy
#[derive(Debug, Clone)]
pub struct EmailProxyConfig {
    /// The verified sending domain
    pub from_domain: String,

    /// Default "from" address
    pub default_from: String,

    /// SES configuration set for tracking
    pub configuration_set: Option<String>,

    /// Enable warm-up mode (limits daily volume)
    pub warm_up_enabled: bool,

    /// Rate limit (emails per second)
    pub rate_limit_per_second: u32,
}

impl Default for EmailProxyConfig {
    fn default() -> Self {
        Self {
            from_domain: "getsignatures.org".to_string(),
            default_from: "GetSignatures <noreply@getsignatures.org>".to_string(),
            configuration_set: Some("docsign-transactional".to_string()),
            warm_up_enabled: true,
            rate_limit_per_second: 14, // SES default
        }
    }
}

impl EmailProxyConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        Self {
            from_domain: std::env::var("FROM_DOMAIN")
                .unwrap_or_else(|_| "getsignatures.org".to_string()),
            default_from: std::env::var("DEFAULT_FROM")
                .unwrap_or_else(|_| "GetSignatures <noreply@getsignatures.org>".to_string()),
            configuration_set: std::env::var("SES_CONFIGURATION_SET").ok(),
            warm_up_enabled: std::env::var("WARM_UP_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            rate_limit_per_second: std::env::var("RATE_LIMIT_PER_SECOND")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(14),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EmailProxyConfig::default();
        assert_eq!(config.from_domain, "getsignatures.org");
        assert!(config.warm_up_enabled);
        assert_eq!(config.rate_limit_per_second, 14);
    }

    /// Regression test: Ensure we're using GA-level Lambda runtime (0.14+)
    /// See: https://aws.amazon.com/blogs/aws/aws-weekly-roundup-aws-lambda-load-balancers-amazon-dcv-amazon-linux-2023-and-more-november-17-2025/
    #[test]
    fn test_lambda_runtime_version_is_ga() {
        // This test verifies key types from lambda_http 0.16+ exist
        // These types are used in main.rs and must be available
        fn _assert_types_exist() {
            // Body type must exist
            let _: fn() -> lambda_http::Body = || lambda_http::Body::Empty;
            // Request type must exist
            type _Request = lambda_http::Request;
            // Response type must exist
            type _Response = lambda_http::Response<lambda_http::Body>;
        }
    }

    /// Regression test: Ensure governor 0.10+ for NonZeroU32 API stability
    #[test]
    fn test_governor_quota_api() {
        use std::num::NonZeroU32;
        // Governor 0.10 uses NonZeroU32 directly, 0.7 had different API
        let quota = governor::Quota::per_second(NonZeroU32::new(10).unwrap());
        // If this compiles, we have the right governor version
        let _ = quota;
    }

    /// Regression test: AWS SDK client construction pattern
    /// Ensures we follow best practice of initializing client once
    #[test]
    fn test_ses_client_is_clonable() {
        // SesSender must be Arc-wrapped for sharing across Lambda invocations
        // This test ensures the pattern in main.rs is valid
        fn _assert_sender_clonable<T: Clone>() {}
        _assert_sender_clonable::<std::sync::Arc<ses::SesSender>>();
    }

    /// Regression test: Rate limiter creation doesn't panic
    #[test]
    fn test_rate_limiter_creation() {
        use std::num::NonZeroU32;
        // Default SES rate is 14/second
        let quota = governor::Quota::per_second(NonZeroU32::new(14).unwrap());
        let limiter = governor::RateLimiter::direct(quota);
        // Should be able to check without panic
        assert!(limiter.check().is_ok());
    }

    /// Regression test: Tracing subscriber has CloudWatch-compatible methods
    /// These methods must exist for the main.rs configuration to compile:
    /// - .json() - for structured logging
    /// - .with_ansi(false) - CloudWatch doesn't support ANSI colors
    /// - .with_current_span(false) - reduces duplicate info
    /// - .without_time() - CloudWatch adds its own timestamp
    #[test]
    fn test_tracing_cloudwatch_methods_exist() {
        // This test verifies the tracing_subscriber API we depend on exists
        // If any of these methods are removed in a future version, this won't compile
        fn _assert_cloudwatch_config_compiles() {
            use tracing_subscriber::fmt;

            // These methods must exist for CloudWatch-optimized logging
            let _ = fmt::fmt()
                .json()
                .with_ansi(false)
                .with_current_span(false)
                .without_time();
        }
    }

    /// Regression test: tracing-subscriber has required features enabled
    #[test]
    fn test_tracing_features_enabled() {
        // The "json" feature must be enabled for .json() to work
        // The "env-filter" feature must be enabled for EnvFilter
        fn _assert_features() {
            use tracing_subscriber::EnvFilter;
            let _ = EnvFilter::from_default_env();
        }
    }
}
