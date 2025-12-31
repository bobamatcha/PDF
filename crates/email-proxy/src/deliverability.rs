//! Email deliverability best practices implementation
//!
//! Implements Resend's top 10 deliverability tips:
//! 1. Authenticate your domain (DKIM, SPF, DMARC) - via SES configuration
//! 2. Maintain a clean email list - suppression list management
//! 3. Warm up your sending IP - gradual volume increase
//! 4. Use double opt-in - handled by consent tracking
//! 5. Personalize your emails - template personalization
//! 6. Include an unsubscribe link - List-Unsubscribe header
//! 7. Monitor your sender reputation - bounce/complaint tracking
//! 8. Avoid spam trigger words - content scanning (optional)
//! 9. Send relevant content - handled by application logic
//! 10. Test before sending - validation and preview

use chrono::{DateTime, Utc};
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Suppression list entry reason
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SuppressionReason {
    /// Hard bounce - permanent delivery failure
    HardBounce,
    /// Complaint - recipient marked as spam
    Complaint,
    /// Manual unsubscribe
    Unsubscribed,
    /// Soft bounce threshold exceeded
    SoftBounceThreshold,
}

impl std::fmt::Display for SuppressionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HardBounce => write!(f, "Hard bounce"),
            Self::Complaint => write!(f, "Spam complaint"),
            Self::Unsubscribed => write!(f, "Unsubscribed"),
            Self::SoftBounceThreshold => write!(f, "Too many soft bounces"),
        }
    }
}

/// Entry in the suppression list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressionEntry {
    pub email: String,
    pub reason: SuppressionReason,
    pub added_at: DateTime<Utc>,
    pub source: String, // e.g., "SES-bounce", "user-unsubscribe"
    pub bounce_type: Option<String>,
    pub bounce_subtype: Option<String>,
}

/// SNS notification types from SES
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "notificationType", rename_all = "PascalCase")]
pub enum SesNotification {
    Bounce {
        bounce: BounceDetails,
        mail: MailDetails,
    },
    Complaint {
        complaint: ComplaintDetails,
        mail: MailDetails,
    },
    Delivery {
        delivery: DeliveryDetails,
        mail: MailDetails,
    },
}

/// Bounce notification details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BounceDetails {
    pub bounce_type: String,
    pub bounce_sub_type: String,
    pub bounced_recipients: Vec<BouncedRecipient>,
    pub timestamp: String,
    pub feedback_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BouncedRecipient {
    pub email_address: String,
    pub action: Option<String>,
    pub status: Option<String>,
    pub diagnostic_code: Option<String>,
}

/// Complaint notification details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplaintDetails {
    pub complained_recipients: Vec<ComplainedRecipient>,
    pub timestamp: String,
    pub feedback_id: String,
    pub complaint_sub_type: Option<String>,
    pub complaint_feedback_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplainedRecipient {
    pub email_address: String,
}

/// Delivery confirmation details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryDetails {
    pub timestamp: String,
    pub processing_time_millis: u64,
    pub recipients: Vec<String>,
    pub smtp_response: String,
    pub reporting_mta: String,
}

/// Original mail details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailDetails {
    pub timestamp: String,
    pub source: String,
    pub source_arn: Option<String>,
    pub source_ip: Option<String>,
    pub sending_account_id: Option<String>,
    pub message_id: String,
    pub destination: Vec<String>,
    pub headers_truncated: Option<bool>,
    pub headers: Option<Vec<MailHeader>>,
    pub common_headers: Option<CommonHeaders>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonHeaders {
    pub from: Option<Vec<String>>,
    pub to: Option<Vec<String>>,
    pub message_id: Option<String>,
    pub subject: Option<String>,
}

/// Email warm-up schedule
/// Gradually increases sending volume to build sender reputation
#[derive(Debug, Clone)]
pub struct WarmUpSchedule {
    /// Daily volume targets by day number
    daily_targets: Vec<u32>,
    /// Start date of warm-up
    start_date: DateTime<Utc>,
}

impl Default for WarmUpSchedule {
    fn default() -> Self {
        // Conservative 4-week warm-up schedule
        // Week 1: 50, 100, 200, 300, 400, 500, 600
        // Week 2: 700, 800, 900, 1000, 1200, 1400, 1600
        // Week 3: 1800, 2000, 2500, 3000, 4000, 5000, 6000
        // Week 4: 8000, 10000, 15000, 20000, 30000, 40000, 50000+
        Self {
            daily_targets: vec![
                // Week 1
                50, 100, 200, 300, 400, 500, 600, // Week 2
                700, 800, 900, 1000, 1200, 1400, 1600, // Week 3
                1800, 2000, 2500, 3000, 4000, 5000, 6000, // Week 4
                8000, 10000, 15000, 20000, 30000, 40000, 50000,
            ],
            start_date: Utc::now(),
        }
    }
}

impl WarmUpSchedule {
    /// Create a new warm-up schedule starting from now
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a schedule that started on a specific date
    pub fn started_on(start_date: DateTime<Utc>) -> Self {
        Self {
            start_date,
            ..Default::default()
        }
    }

    /// Get the daily limit based on current warm-up progress
    pub fn daily_limit(&self) -> u32 {
        let days_since_start = (Utc::now() - self.start_date).num_days() as usize;

        if days_since_start >= self.daily_targets.len() {
            // Past warm-up period, no limit
            u32::MAX
        } else {
            self.daily_targets[days_since_start]
        }
    }

    /// Check if warm-up is complete
    pub fn is_complete(&self) -> bool {
        let days = (Utc::now() - self.start_date).num_days() as usize;
        days >= self.daily_targets.len()
    }

    /// Get warm-up progress as percentage
    pub fn progress_percent(&self) -> f32 {
        let days = (Utc::now() - self.start_date).num_days() as f32;
        let total = self.daily_targets.len() as f32;
        ((days / total) * 100.0).min(100.0)
    }
}

/// Sender reputation metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReputationMetrics {
    /// Total emails sent
    pub total_sent: u64,
    /// Successful deliveries
    pub delivered: u64,
    /// Hard bounces
    pub hard_bounces: u64,
    /// Soft bounces
    pub soft_bounces: u64,
    /// Complaints (marked as spam)
    pub complaints: u64,
    /// Opens (if tracking enabled)
    pub opens: u64,
    /// Clicks (if tracking enabled)
    pub clicks: u64,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

impl ReputationMetrics {
    /// Calculate bounce rate (should be < 2%)
    pub fn bounce_rate(&self) -> f64 {
        if self.total_sent == 0 {
            return 0.0;
        }
        (self.hard_bounces as f64 / self.total_sent as f64) * 100.0
    }

    /// Calculate complaint rate (should be < 0.1%)
    pub fn complaint_rate(&self) -> f64 {
        if self.total_sent == 0 {
            return 0.0;
        }
        (self.complaints as f64 / self.total_sent as f64) * 100.0
    }

    /// Check if metrics are healthy
    /// Industry standards: bounce rate < 5%, complaint rate <= 0.1%
    pub fn is_healthy(&self) -> bool {
        self.bounce_rate() < 5.0 && self.complaint_rate() <= 0.1
    }

    /// Get a health score (0-100)
    pub fn health_score(&self) -> u8 {
        let mut score = 100u8;

        // Penalize for bounces (target < 2%)
        let bounce_penalty = (self.bounce_rate() * 10.0) as u8;
        score = score.saturating_sub(bounce_penalty);

        // Penalize heavily for complaints (target < 0.1%)
        let complaint_penalty = (self.complaint_rate() * 500.0) as u8;
        score = score.saturating_sub(complaint_penalty);

        score
    }
}

/// Deliverability manager
/// Handles suppression lists, rate limiting, and metrics
pub struct DeliverabilityManager {
    /// Suppression list (in-memory, would be backed by DynamoDB in production)
    suppression_list: Arc<RwLock<HashMap<String, SuppressionEntry>>>,

    /// Rate limiter (per second)
    rate_limiter: RateLimiter<
        governor::state::NotKeyed,
        governor::state::InMemoryState,
        governor::clock::DefaultClock,
    >,

    /// Warm-up schedule
    warm_up: WarmUpSchedule,

    /// Today's send count
    daily_count: Arc<RwLock<u32>>,

    /// Reputation metrics
    metrics: Arc<RwLock<ReputationMetrics>>,

    /// Soft bounce counts per email (for threshold tracking)
    soft_bounce_counts: Arc<RwLock<HashMap<String, u32>>>,
}

impl DeliverabilityManager {
    /// Create a new deliverability manager
    pub fn new() -> Self {
        // Default: 14 emails per second (SES default rate)
        Self::with_rate_limit(14)
    }

    /// Create with custom rate limit
    pub fn with_rate_limit(per_second: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(per_second).unwrap());

        Self {
            suppression_list: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: RateLimiter::direct(quota),
            warm_up: WarmUpSchedule::new(),
            daily_count: Arc::new(RwLock::new(0)),
            metrics: Arc::new(RwLock::new(ReputationMetrics::default())),
            soft_bounce_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if an email should be suppressed
    pub async fn is_suppressed(&self, email: &str) -> Option<SuppressionEntry> {
        let list = self.suppression_list.read().await;
        list.get(&email.to_lowercase()).cloned()
    }

    /// Add an email to the suppression list
    pub async fn suppress(&self, email: &str, reason: SuppressionReason, source: &str) {
        let mut list = self.suppression_list.write().await;
        list.insert(
            email.to_lowercase(),
            SuppressionEntry {
                email: email.to_lowercase(),
                reason,
                added_at: Utc::now(),
                source: source.to_string(),
                bounce_type: None,
                bounce_subtype: None,
            },
        );
    }

    /// Process an SES notification (bounce/complaint)
    pub async fn process_notification(&self, notification: SesNotification) {
        match notification {
            SesNotification::Bounce { bounce, .. } => {
                let is_hard_bounce = bounce.bounce_type == "Permanent";

                for recipient in bounce.bounced_recipients {
                    if is_hard_bounce {
                        // Hard bounce: immediately suppress
                        let mut list = self.suppression_list.write().await;
                        list.insert(
                            recipient.email_address.to_lowercase(),
                            SuppressionEntry {
                                email: recipient.email_address.to_lowercase(),
                                reason: SuppressionReason::HardBounce,
                                added_at: Utc::now(),
                                source: "SES-bounce".to_string(),
                                bounce_type: Some(bounce.bounce_type.clone()),
                                bounce_subtype: Some(bounce.bounce_sub_type.clone()),
                            },
                        );

                        let mut metrics = self.metrics.write().await;
                        metrics.hard_bounces += 1;
                        metrics.updated_at = Utc::now();
                    } else {
                        // Soft bounce: track count, suppress after 3
                        let mut counts = self.soft_bounce_counts.write().await;
                        let count = counts
                            .entry(recipient.email_address.to_lowercase())
                            .or_insert(0);
                        *count += 1;

                        if *count >= 3 {
                            let mut list = self.suppression_list.write().await;
                            list.insert(
                                recipient.email_address.to_lowercase(),
                                SuppressionEntry {
                                    email: recipient.email_address.to_lowercase(),
                                    reason: SuppressionReason::SoftBounceThreshold,
                                    added_at: Utc::now(),
                                    source: "SES-soft-bounce-threshold".to_string(),
                                    bounce_type: Some(bounce.bounce_type.clone()),
                                    bounce_subtype: Some(bounce.bounce_sub_type.clone()),
                                },
                            );
                        }

                        let mut metrics = self.metrics.write().await;
                        metrics.soft_bounces += 1;
                        metrics.updated_at = Utc::now();
                    }
                }
            }

            SesNotification::Complaint { complaint, .. } => {
                for recipient in complaint.complained_recipients {
                    let mut list = self.suppression_list.write().await;
                    list.insert(
                        recipient.email_address.to_lowercase(),
                        SuppressionEntry {
                            email: recipient.email_address.to_lowercase(),
                            reason: SuppressionReason::Complaint,
                            added_at: Utc::now(),
                            source: "SES-complaint".to_string(),
                            bounce_type: None,
                            bounce_subtype: None,
                        },
                    );
                }

                let mut metrics = self.metrics.write().await;
                metrics.complaints += 1;
                metrics.updated_at = Utc::now();
            }

            SesNotification::Delivery { delivery, .. } => {
                let mut metrics = self.metrics.write().await;
                metrics.delivered += delivery.recipients.len() as u64;
                metrics.updated_at = Utc::now();
            }
        }
    }

    /// Check if we can send (rate limit + warm-up)
    pub async fn can_send(&self) -> Result<(), SendBlockedReason> {
        // Check rate limit
        if self.rate_limiter.check().is_err() {
            return Err(SendBlockedReason::RateLimited);
        }

        // Check warm-up daily limit
        let daily_limit = self.warm_up.daily_limit();
        let count = *self.daily_count.read().await;

        if count >= daily_limit {
            return Err(SendBlockedReason::WarmUpLimitReached {
                limit: daily_limit,
                current: count,
            });
        }

        Ok(())
    }

    /// Record a send
    pub async fn record_send(&self) {
        let mut count = self.daily_count.write().await;
        *count += 1;

        let mut metrics = self.metrics.write().await;
        metrics.total_sent += 1;
        metrics.updated_at = Utc::now();
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> ReputationMetrics {
        self.metrics.read().await.clone()
    }

    /// Get warm-up status
    pub fn warm_up_status(&self) -> WarmUpStatus {
        WarmUpStatus {
            is_complete: self.warm_up.is_complete(),
            progress_percent: self.warm_up.progress_percent(),
            daily_limit: self.warm_up.daily_limit(),
        }
    }

    /// Reset daily count (call at midnight UTC)
    pub async fn reset_daily_count(&self) {
        let mut count = self.daily_count.write().await;
        *count = 0;
    }
}

impl Default for DeliverabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Reason why sending was blocked
#[derive(Debug, Clone, thiserror::Error)]
pub enum SendBlockedReason {
    #[error("Rate limited: too many requests per second")]
    RateLimited,

    #[error("Warm-up limit reached: {current}/{limit} emails today")]
    WarmUpLimitReached { limit: u32, current: u32 },

    #[error("Email suppressed: {0}")]
    Suppressed(SuppressionReason),
}

/// Warm-up status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmUpStatus {
    pub is_complete: bool,
    pub progress_percent: f32,
    pub daily_limit: u32,
}

/// Content scanner for spam trigger words
/// Returns a score where higher = more likely spam
pub fn scan_content_for_spam(subject: &str, body: &str) -> SpamScanResult {
    let mut score = 0.0;
    let mut triggers: Vec<String> = vec![];

    // Common spam trigger words
    let spam_words = [
        ("FREE", 2.0),
        ("WINNER", 3.0),
        ("URGENT", 1.5),
        ("ACT NOW", 2.0),
        ("LIMITED TIME", 1.5),
        ("CLICK HERE", 1.5),
        ("CONGRATULATIONS", 2.0),
        ("100%", 1.0),
        ("GUARANTEE", 1.0),
        ("NO OBLIGATION", 1.5),
        ("RISK FREE", 1.5),
        ("$$", 2.0),
        ("CASH BONUS", 2.5),
        ("DOUBLE YOUR", 2.5),
        ("EARN MONEY", 2.0),
    ];

    let combined = format!("{} {}", subject, body).to_uppercase();

    for (word, weight) in spam_words {
        if combined.contains(word) {
            score += weight;
            triggers.push(word.to_string());
        }
    }

    // Check for excessive caps in subject
    let caps_ratio =
        subject.chars().filter(|c| c.is_uppercase()).count() as f64 / subject.len().max(1) as f64;

    if caps_ratio > 0.5 {
        score += 1.5;
        triggers.push("EXCESSIVE_CAPS".to_string());
    }

    // Check for excessive exclamation marks
    let exclaim_count = combined.matches('!').count();
    if exclaim_count > 3 {
        score += 0.5 * (exclaim_count - 3) as f64;
        triggers.push("EXCESSIVE_EXCLAMATION".to_string());
    }

    SpamScanResult {
        score,
        is_likely_spam: score > 5.0,
        triggers,
    }
}

/// Result of spam content scan
#[derive(Debug, Clone)]
pub struct SpamScanResult {
    pub score: f64,
    pub is_likely_spam: bool,
    pub triggers: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_suppression_list() {
        let manager = DeliverabilityManager::new();

        // Initially not suppressed
        assert!(manager.is_suppressed("test@example.com").await.is_none());

        // Add to suppression list
        manager
            .suppress("test@example.com", SuppressionReason::HardBounce, "test")
            .await;

        // Now suppressed
        let entry = manager.is_suppressed("test@example.com").await;
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().reason, SuppressionReason::HardBounce);

        // Case insensitive
        assert!(manager.is_suppressed("TEST@EXAMPLE.COM").await.is_some());
    }

    #[tokio::test]
    async fn test_reputation_metrics() {
        // Good metrics scenario
        let good_metrics = ReputationMetrics {
            total_sent: 10000,
            delivered: 9980,
            hard_bounces: 10, // 0.1% bounce rate
            soft_bounces: 10,
            complaints: 1, // 0.01% complaint rate
            opens: 5000,
            clicks: 1000,
            updated_at: Utc::now(),
        };

        // 0.1% bounce rate - excellent
        assert!(
            good_metrics.bounce_rate() < 1.0,
            "bounce rate: {}",
            good_metrics.bounce_rate()
        );

        // 0.01% complaint rate - excellent
        assert!(
            good_metrics.complaint_rate() < 0.1,
            "complaint rate: {}",
            good_metrics.complaint_rate()
        );

        // Should be healthy
        assert!(good_metrics.is_healthy());

        // Health score should be very high with these good metrics
        assert!(
            good_metrics.health_score() > 90,
            "health score: {}",
            good_metrics.health_score()
        );

        // Bad metrics scenario
        let bad_metrics = ReputationMetrics {
            total_sent: 1000,
            delivered: 900,
            hard_bounces: 60, // 6% bounce rate - unhealthy
            soft_bounces: 40,
            complaints: 5, // 0.5% complaint rate - unhealthy
            opens: 100,
            clicks: 10,
            updated_at: Utc::now(),
        };

        // Should NOT be healthy (both thresholds exceeded)
        assert!(!bad_metrics.is_healthy());
    }

    #[tokio::test]
    async fn test_warm_up_schedule() {
        let schedule = WarmUpSchedule::new();

        // Day 1 should be 50 emails
        assert_eq!(schedule.daily_limit(), 50);
        assert!(!schedule.is_complete());
        assert!(schedule.progress_percent() < 10.0);
    }

    #[test]
    fn test_spam_content_scanner() {
        // Clean email
        let result = scan_content_for_spam(
            "Document ready for signature",
            "Please review and sign the attached document.",
        );
        assert!(!result.is_likely_spam);
        assert!(result.score < 3.0);

        // Spammy email
        let result = scan_content_for_spam(
            "FREE MONEY!!! ACT NOW!!!",
            "CONGRATULATIONS!!! You're a WINNER!!! Click here for your CASH BONUS!!!",
        );
        assert!(result.is_likely_spam);
        assert!(result.triggers.len() > 3);
    }

    #[test]
    fn test_parse_bounce_notification() {
        let json = r#"{
            "notificationType": "Bounce",
            "bounce": {
                "bounceType": "Permanent",
                "bounceSubType": "General",
                "bouncedRecipients": [
                    {"emailAddress": "bounced@example.com"}
                ],
                "timestamp": "2025-01-01T00:00:00.000Z",
                "feedbackId": "feedback-123"
            },
            "mail": {
                "timestamp": "2025-01-01T00:00:00.000Z",
                "source": "sender@example.com",
                "messageId": "msg-123",
                "destination": ["bounced@example.com"]
            }
        }"#;

        let notification: SesNotification = serde_json::from_str(json).unwrap();

        match notification {
            SesNotification::Bounce { bounce, mail } => {
                assert_eq!(bounce.bounce_type, "Permanent");
                assert_eq!(bounce.bounced_recipients.len(), 1);
                assert_eq!(mail.source, "sender@example.com");
            }
            _ => panic!("Expected Bounce notification"),
        }
    }
}
