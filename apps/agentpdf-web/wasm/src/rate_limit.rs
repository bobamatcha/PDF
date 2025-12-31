//! Rate Limiting Module for AgentPDF
//!
//! Provides in-memory rate limiting for WASM and server-side use.
//! Designed for PDF processing, email dispatch, and session management endpoints.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Rate limit configuration for an endpoint
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests allowed in the window
    pub limit: u32,
    /// Time window for rate limiting
    pub window: Duration,
    /// Block duration after exceeding limit
    pub block_duration: Duration,
}

impl RateLimitConfig {
    /// Create a new rate limit config
    pub fn new(limit: u32, window_secs: u64, block_secs: u64) -> Self {
        Self {
            limit,
            window: Duration::from_secs(window_secs),
            block_duration: Duration::from_secs(block_secs),
        }
    }

    /// Preset for PDF rendering (10 req/min, 60s block)
    pub fn pdf_render() -> Self {
        Self::new(10, 60, 60)
    }

    /// Preset for session creation (5 req/min, 60s block)
    pub fn session_create() -> Self {
        Self::new(5, 60, 60)
    }

    /// Preset for email dispatch (10 req/hour, 300s block)
    pub fn email_dispatch() -> Self {
        Self::new(10, 3600, 300)
    }

    /// Preset for file upload (5 uploads/10min, 120s block)
    pub fn file_upload() -> Self {
        Self::new(5, 600, 120)
    }
}

/// Entry tracking requests for a single key
#[derive(Debug, Clone)]
struct RateLimitEntry {
    /// Number of requests in current window
    count: u32,
    /// When the current window started
    window_start: Instant,
    /// When the block expires (if blocked)
    blocked_until: Option<Instant>,
}

impl RateLimitEntry {
    fn new() -> Self {
        Self {
            count: 1,
            window_start: Instant::now(),
            blocked_until: None,
        }
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed,
    /// Request is rate limited, retry after duration
    Limited { retry_after: Duration },
    /// Client is blocked, blocked for duration
    Blocked { blocked_for: Duration },
}

impl RateLimitResult {
    /// Check if the request is allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed)
    }

    /// Get retry/block duration if limited
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            RateLimitResult::Allowed => None,
            RateLimitResult::Limited { retry_after } => Some(*retry_after),
            RateLimitResult::Blocked { blocked_for } => Some(*blocked_for),
        }
    }
}

/// In-memory rate limiter
///
/// Thread-safe for single-threaded WASM, requires Arc<Mutex> for multi-threaded use.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    entries: HashMap<String, RateLimitEntry>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            entries: HashMap::new(),
        }
    }

    /// Check if a request from the given key is allowed
    ///
    /// If allowed, increments the counter. If not, returns when to retry.
    pub fn check(&mut self, key: &str) -> RateLimitResult {
        let now = Instant::now();

        if let Some(entry) = self.entries.get_mut(key) {
            // Check if blocked
            if let Some(blocked_until) = entry.blocked_until {
                if now < blocked_until {
                    return RateLimitResult::Blocked {
                        blocked_for: blocked_until - now,
                    };
                }
                // Block expired, reset
                entry.blocked_until = None;
                entry.count = 0;
                entry.window_start = now;
            }

            // Check if window expired
            if now.duration_since(entry.window_start) > self.config.window {
                // Reset window
                entry.count = 1;
                entry.window_start = now;
                return RateLimitResult::Allowed;
            }

            // Check limit
            if entry.count >= self.config.limit {
                // Rate exceeded, block the client
                let blocked_until = now + self.config.block_duration;
                entry.blocked_until = Some(blocked_until);
                return RateLimitResult::Blocked {
                    blocked_for: self.config.block_duration,
                };
            }

            // Increment and allow
            entry.count += 1;
            RateLimitResult::Allowed
        } else {
            // First request from this key
            self.entries.insert(key.to_string(), RateLimitEntry::new());
            RateLimitResult::Allowed
        }
    }

    /// Check without incrementing (peek)
    pub fn peek(&self, key: &str) -> RateLimitResult {
        let now = Instant::now();

        if let Some(entry) = self.entries.get(key) {
            if let Some(blocked_until) = entry.blocked_until {
                if now < blocked_until {
                    return RateLimitResult::Blocked {
                        blocked_for: blocked_until - now,
                    };
                }
            }

            if now.duration_since(entry.window_start) > self.config.window {
                return RateLimitResult::Allowed;
            }

            if entry.count >= self.config.limit {
                let remaining = self.config.window - now.duration_since(entry.window_start);
                return RateLimitResult::Limited {
                    retry_after: remaining,
                };
            }

            RateLimitResult::Allowed
        } else {
            RateLimitResult::Allowed
        }
    }

    /// Get current request count for a key
    pub fn count(&self, key: &str) -> u32 {
        self.entries.get(key).map_or(0, |e| e.count)
    }

    /// Reset rate limit for a key (admin override)
    pub fn reset(&mut self, key: &str) {
        self.entries.remove(key);
    }

    /// Clean up expired entries (call periodically in long-running processes)
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        let window = self.config.window;

        self.entries.retain(|_, entry| {
            // Keep if blocked and block not expired
            if let Some(blocked_until) = entry.blocked_until {
                if now < blocked_until {
                    return true;
                }
            }
            // Keep if window not expired
            now.duration_since(entry.window_start) <= window
        });
    }

    /// Get number of tracked keys
    pub fn tracked_count(&self) -> usize {
        self.entries.len()
    }
}

/// Validate PDF processing limits
pub fn validate_pdf_limits(
    bytes_len: usize,
    page_count: usize,
    object_count: usize,
) -> Result<(), PdfLimitError> {
    const MAX_PDF_SIZE: usize = 50 * 1024 * 1024; // 50 MB
    const MAX_PAGE_COUNT: usize = 500;
    const MAX_OBJECT_COUNT: usize = 100_000;

    if bytes_len > MAX_PDF_SIZE {
        return Err(PdfLimitError::TooLarge {
            size: bytes_len,
            max: MAX_PDF_SIZE,
        });
    }

    if page_count > MAX_PAGE_COUNT {
        return Err(PdfLimitError::TooManyPages {
            count: page_count,
            max: MAX_PAGE_COUNT,
        });
    }

    if object_count > MAX_OBJECT_COUNT {
        return Err(PdfLimitError::TooManyObjects {
            count: object_count,
            max: MAX_OBJECT_COUNT,
        });
    }

    Ok(())
}

/// PDF processing limit errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfLimitError {
    TooLarge { size: usize, max: usize },
    TooManyPages { count: usize, max: usize },
    TooManyObjects { count: usize, max: usize },
}

impl std::fmt::Display for PdfLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge { size, max } => {
                write!(f, "PDF too large: {} bytes (max: {} bytes)", size, max)
            }
            Self::TooManyPages { count, max } => {
                write!(f, "Too many pages: {} (max: {})", count, max)
            }
            Self::TooManyObjects { count, max } => {
                write!(f, "Too many objects: {} (max: {})", count, max)
            }
        }
    }
}

impl std::error::Error for PdfLimitError {}

/// Validate email dispatch limits
pub fn validate_email_limits(
    recipient_count: usize,
    body_size: usize,
) -> Result<(), EmailLimitError> {
    const MAX_RECIPIENTS_PER_REQUEST: usize = 10;
    const MAX_EMAIL_SIZE: usize = 500 * 1024; // 500 KB

    if recipient_count > MAX_RECIPIENTS_PER_REQUEST {
        return Err(EmailLimitError::TooManyRecipients {
            count: recipient_count,
            max: MAX_RECIPIENTS_PER_REQUEST,
        });
    }

    if body_size > MAX_EMAIL_SIZE {
        return Err(EmailLimitError::BodyTooLarge {
            size: body_size,
            max: MAX_EMAIL_SIZE,
        });
    }

    Ok(())
}

/// Email dispatch limit errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailLimitError {
    TooManyRecipients { count: usize, max: usize },
    BodyTooLarge { size: usize, max: usize },
}

impl std::fmt::Display for EmailLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyRecipients { count, max } => {
                write!(f, "Too many recipients: {} (max: {})", count, max)
            }
            Self::BodyTooLarge { size, max } => {
                write!(
                    f,
                    "Email body too large: {} bytes (max: {} bytes)",
                    size, max
                )
            }
        }
    }
}

impl std::error::Error for EmailLimitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let config = RateLimitConfig::new(5, 60, 60);
        let mut limiter = RateLimiter::new(config);

        for _ in 0..5 {
            assert!(limiter.check("client-1").is_allowed());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let config = RateLimitConfig::new(3, 60, 60);
        let mut limiter = RateLimiter::new(config);

        assert!(limiter.check("client-1").is_allowed()); // 1
        assert!(limiter.check("client-1").is_allowed()); // 2
        assert!(limiter.check("client-1").is_allowed()); // 3
        assert!(!limiter.check("client-1").is_allowed()); // blocked
    }

    #[test]
    fn test_rate_limiter_separate_keys() {
        let config = RateLimitConfig::new(2, 60, 60);
        let mut limiter = RateLimiter::new(config);

        assert!(limiter.check("client-1").is_allowed());
        assert!(limiter.check("client-1").is_allowed());
        assert!(!limiter.check("client-1").is_allowed()); // blocked

        // Different client should be allowed
        assert!(limiter.check("client-2").is_allowed());
        assert!(limiter.check("client-2").is_allowed());
    }

    #[test]
    fn test_rate_limiter_count() {
        let config = RateLimitConfig::new(10, 60, 60);
        let mut limiter = RateLimiter::new(config);

        assert_eq!(limiter.count("client-1"), 0);

        limiter.check("client-1");
        assert_eq!(limiter.count("client-1"), 1);

        limiter.check("client-1");
        limiter.check("client-1");
        assert_eq!(limiter.count("client-1"), 3);
    }

    #[test]
    fn test_rate_limiter_reset() {
        let config = RateLimitConfig::new(3, 60, 60);
        let mut limiter = RateLimiter::new(config);

        limiter.check("client-1");
        limiter.check("client-1");
        limiter.check("client-1");
        assert!(!limiter.check("client-1").is_allowed()); // blocked

        limiter.reset("client-1");
        assert!(limiter.check("client-1").is_allowed()); // allowed after reset
    }

    #[test]
    fn test_rate_limiter_peek_does_not_increment() {
        let config = RateLimitConfig::new(5, 60, 60);
        let mut limiter = RateLimiter::new(config);

        limiter.check("client-1"); // count = 1
        assert!(limiter.peek("client-1").is_allowed());
        assert_eq!(limiter.count("client-1"), 1); // unchanged
    }

    #[test]
    fn test_pdf_limits_valid() {
        assert!(validate_pdf_limits(1024, 10, 100).is_ok());
        assert!(validate_pdf_limits(50 * 1024 * 1024, 500, 100_000).is_ok());
    }

    #[test]
    fn test_pdf_limits_too_large() {
        let result = validate_pdf_limits(51 * 1024 * 1024, 10, 100);
        assert!(matches!(result, Err(PdfLimitError::TooLarge { .. })));
    }

    #[test]
    fn test_pdf_limits_too_many_pages() {
        let result = validate_pdf_limits(1024, 501, 100);
        assert!(matches!(result, Err(PdfLimitError::TooManyPages { .. })));
    }

    #[test]
    fn test_pdf_limits_too_many_objects() {
        let result = validate_pdf_limits(1024, 10, 100_001);
        assert!(matches!(result, Err(PdfLimitError::TooManyObjects { .. })));
    }

    #[test]
    fn test_email_limits_valid() {
        assert!(validate_email_limits(5, 1024).is_ok());
        assert!(validate_email_limits(10, 500 * 1024).is_ok());
    }

    #[test]
    fn test_email_limits_too_many_recipients() {
        let result = validate_email_limits(11, 1024);
        assert!(matches!(
            result,
            Err(EmailLimitError::TooManyRecipients { .. })
        ));
    }

    #[test]
    fn test_email_limits_body_too_large() {
        let result = validate_email_limits(5, 501 * 1024);
        assert!(matches!(result, Err(EmailLimitError::BodyTooLarge { .. })));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: First N requests are always allowed (where N = limit)
        #[test]
        fn first_n_requests_allowed(limit in 1u32..100, key in "[a-z]{5,10}") {
            let config = RateLimitConfig::new(limit, 60, 60);
            let mut limiter = RateLimiter::new(config);

            for i in 0..limit {
                let result = limiter.check(&key);
                prop_assert!(result.is_allowed(), "Request {} should be allowed", i + 1);
            }
        }

        /// Property: Request N+1 is always blocked (where N = limit)
        #[test]
        fn request_after_limit_blocked(limit in 1u32..50, key in "[a-z]{5,10}") {
            let config = RateLimitConfig::new(limit, 60, 60);
            let mut limiter = RateLimiter::new(config);

            // Exhaust the limit
            for _ in 0..limit {
                limiter.check(&key);
            }

            // Next request should be blocked
            let result = limiter.check(&key);
            prop_assert!(!result.is_allowed(), "Request after limit should be blocked");
        }

        /// Property: Different keys are independent
        #[test]
        fn keys_are_independent(
            limit in 1u32..20,
            key1 in "[a-z]{5}",
            key2 in "[A-Z]{5}"
        ) {
            let config = RateLimitConfig::new(limit, 60, 60);
            let mut limiter = RateLimiter::new(config);

            // Exhaust key1
            for _ in 0..=limit {
                limiter.check(&key1);
            }
            prop_assert!(!limiter.check(&key1).is_allowed());

            // key2 should still be allowed
            prop_assert!(limiter.check(&key2).is_allowed());
        }

        /// Property: Count equals number of successful requests
        #[test]
        fn count_tracks_requests(requests in 1u32..50) {
            let limit = 100; // High limit so nothing gets blocked
            let config = RateLimitConfig::new(limit, 60, 60);
            let mut limiter = RateLimiter::new(config);

            for _ in 0..requests {
                limiter.check("test-key");
            }

            prop_assert_eq!(limiter.count("test-key"), requests);
        }

        /// Property: Peek never changes state
        #[test]
        fn peek_is_readonly(operations in 0usize..20) {
            let config = RateLimitConfig::new(10, 60, 60);
            let mut limiter = RateLimiter::new(config);

            // Do some operations
            for _ in 0..operations {
                limiter.check("key");
            }

            let count_before = limiter.count("key");

            // Peek multiple times
            for _ in 0..10 {
                limiter.peek("key");
            }

            let count_after = limiter.count("key");
            prop_assert_eq!(count_before, count_after, "Peek should not change count");
        }

        /// Property: Reset returns key to initial state
        #[test]
        fn reset_clears_state(requests in 1u32..50) {
            let config = RateLimitConfig::new(100, 60, 60);
            let mut limiter = RateLimiter::new(config);

            for _ in 0..requests {
                limiter.check("key");
            }

            limiter.reset("key");

            prop_assert_eq!(limiter.count("key"), 0);
            prop_assert!(limiter.check("key").is_allowed());
        }

        /// Property: PDF validation rejects exactly when limits exceeded
        #[test]
        fn pdf_limits_boundary(
            size in 0usize..60_000_000,
            pages in 0usize..600,
            objects in 0usize..110_000
        ) {
            let result = validate_pdf_limits(size, pages, objects);

            let expected_ok = size <= 50 * 1024 * 1024
                && pages <= 500
                && objects <= 100_000;

            prop_assert_eq!(result.is_ok(), expected_ok);
        }

        /// Property: Email validation rejects exactly when limits exceeded
        #[test]
        fn email_limits_boundary(
            recipients in 0usize..15,
            body_size in 0usize..600_000
        ) {
            let result = validate_email_limits(recipients, body_size);

            let expected_ok = recipients <= 10 && body_size <= 500 * 1024;

            prop_assert_eq!(result.is_ok(), expected_ok);
        }

        /// Property: Retry-after duration is always positive when blocked
        #[test]
        fn blocked_has_positive_duration(limit in 1u32..10, block_secs in 1u64..3600) {
            let config = RateLimitConfig::new(limit, 60, block_secs);
            let mut limiter = RateLimiter::new(config);

            // Exhaust limit
            for _ in 0..=limit {
                limiter.check("key");
            }

            let result = limiter.check("key");
            if let Some(duration) = result.retry_after() {
                prop_assert!(duration.as_secs() > 0 || duration.subsec_nanos() > 0);
            }
        }

        /// Property: RateLimitConfig presets have sensible values
        #[test]
        fn presets_are_valid(_dummy in 0u8..1) {
            let pdf = RateLimitConfig::pdf_render();
            prop_assert!(pdf.limit > 0);
            prop_assert!(pdf.window.as_secs() > 0);

            let session = RateLimitConfig::session_create();
            prop_assert!(session.limit > 0);
            prop_assert!(session.window.as_secs() > 0);

            let email = RateLimitConfig::email_dispatch();
            prop_assert!(email.limit > 0);
            prop_assert!(email.window.as_secs() > 0);

            let upload = RateLimitConfig::file_upload();
            prop_assert!(upload.limit > 0);
            prop_assert!(upload.window.as_secs() > 0);
        }
    }
}
