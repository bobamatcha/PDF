//! Authentication types and data structures

use chrono::Datelike;
use serde::{Deserialize, Serialize};

/// User tier for rate limiting and feature access
/// Bug #6: Expanded from Free/Pro to 4-tier pricing model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserTier {
    Free,
    Personal,
    Professional,
    Business,
    /// Legacy tier - maps to Free for backward compatibility
    #[serde(alias = "pro")]
    Pro,
}

impl Default for UserTier {
    fn default() -> Self {
        Self::Free
    }
}

impl UserTier {
    /// Monthly document limit for this tier
    pub fn monthly_limit(&self) -> u32 {
        match self {
            Self::Free => 3,
            Self::Personal => 25,
            Self::Professional => 100,
            Self::Business => 300,
            Self::Pro => 25, // Legacy Pro maps to Personal limits
        }
    }

    /// Maximum documents allowed with overage (paid tiers only)
    pub fn max_with_overage(&self) -> u32 {
        match self {
            Self::Free => 3, // Hard limit, no overage
            Self::Personal => 50,
            Self::Professional => 200,
            Self::Business => 600,
            Self::Pro => 50, // Legacy Pro maps to Personal limits
        }
    }

    /// Whether this tier allows overage charges
    pub fn allows_overage(&self) -> bool {
        !matches!(self, Self::Free)
    }

    /// Get display name for the tier
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Personal => "Personal",
            Self::Professional => "Professional",
            Self::Business => "Business",
            Self::Pro => "Personal", // Legacy Pro displays as Personal
        }
    }

    /// Monthly price in cents (0 for free)
    pub fn monthly_price_cents(&self) -> u32 {
        match self {
            Self::Free => 0,
            Self::Personal | Self::Pro => 1000, // $10
            Self::Professional => 2500,         // $25
            Self::Business => 6000,             // $60
        }
    }

    /// Annual price in cents (2 months free)
    pub fn annual_price_cents(&self) -> u32 {
        match self {
            Self::Free => 0,
            Self::Personal | Self::Pro => 10000, // $100 (save $20)
            Self::Professional => 25000,         // $250 (save $50)
            Self::Business => 60000,             // $600 (save $120)
        }
    }
}

/// Billing cycle for paid subscriptions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BillingCycle {
    #[default]
    Monthly,
    Annual,
}

// ============================================
// Beta Access Grant System
// ============================================

/// Source of a user's tier (how they got their current tier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GrantSource {
    /// Paid via Stripe subscription
    #[default]
    Subscription,
    /// Admin approved beta access request
    BetaGrant,
    /// Pre-granted before account creation
    PreGrant,
}

// ============================================
// Bug #22: OAuth Providers
// ============================================

/// OAuth provider for third-party authentication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthProvider {
    /// Google Sign-In
    Google,
    /// Apple Sign-In (future)
    Apple,
}

/// Pre-grant record stored in BETA_GRANTS KV
/// Allows admin to grant tier access to emails before they register
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaGrant {
    /// Email address (lowercase normalized)
    pub email: String,
    /// Tier to grant (defaults to Professional)
    pub tier: UserTier,
    /// Admin email who created this grant
    pub granted_by: String,
    /// ISO timestamp when grant was created
    pub granted_at: String,
    /// Optional admin notes
    #[serde(default)]
    pub notes: Option<String>,
    /// Whether grant has been revoked
    #[serde(default)]
    pub revoked: bool,
    /// ISO timestamp when revoked (if applicable)
    #[serde(default)]
    pub revoked_at: Option<String>,
    /// Bug #19: Whether welcome email was sent
    #[serde(default)]
    pub welcome_email_sent: bool,
}

/// Bug #20: Do Not Email entry stored in DO_NOT_EMAIL KV
/// Stores unsubscribed emails and tracks account status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoNotEmailEntry {
    /// Email address (lowercase normalized)
    pub email: String,
    /// ISO timestamp when unsubscribed
    pub unsubscribed_at: String,
    /// Whether the email has an associated account
    pub has_account: bool,
    /// Optional reason for unsubscription
    #[serde(default)]
    pub reason: Option<String>,
}

/// Bug #20: Email preferences for users (stored in User struct)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EmailPreferences {
    /// Receive marketing/promotional emails (welcome gifts, etc.)
    #[serde(default = "default_true")]
    pub marketing: bool,
    /// Receive product updates and feature announcements
    #[serde(default = "default_true")]
    pub product_updates: bool,
    // Note: Transactional emails (signature requests, verifications) always sent
}

fn default_true() -> bool {
    true
}

/// User record stored in KV
/// Bug #6: Expanded with 4-tier pricing and billing fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub email_verified: bool,
    pub password_hash: String,
    #[serde(default)]
    pub tier: UserTier,
    pub created_at: String,
    pub updated_at: String,
    /// First name (for personalized greetings, signatures)
    #[serde(default)]
    pub first_name: String,
    /// Middle initial (optional)
    #[serde(default)]
    pub middle_initial: Option<String>,
    /// Last name
    #[serde(default)]
    pub last_name: String,
    /// Number of documents created this month
    /// Bug #6: Limits vary by tier (Free: 3, Personal: 25, Professional: 100, Business: 300)
    #[serde(
        default,
        alias = "weekly_document_count",
        alias = "daily_document_count"
    )]
    pub monthly_document_count: u32,
    /// Current quota month in "YYYY-MM" format (e.g., "2025-01")
    #[serde(default, alias = "weekly_reset_at", alias = "daily_reset_at")]
    pub current_quota_month: String,
    /// Last login timestamp
    #[serde(default)]
    pub last_login_at: Option<String>,
    /// Total login count for analytics
    #[serde(default)]
    pub login_count: u32,
    // ============================================
    // Bug #6: New billing fields for paid tiers
    // ============================================
    /// Stripe customer ID (set when user upgrades to paid tier)
    #[serde(default)]
    pub stripe_customer_id: Option<String>,
    /// Stripe subscription ID (active subscription)
    #[serde(default)]
    pub stripe_subscription_id: Option<String>,
    /// Billing cycle (monthly or annual)
    #[serde(default)]
    pub billing_cycle: BillingCycle,
    /// Overage documents this month (documents beyond tier limit)
    /// Only applies to paid tiers; Free tier has hard limit
    #[serde(default)]
    pub overage_count: u32,
    /// Whether user was notified about hitting their limit this month
    /// Resets when current_quota_month changes
    #[serde(default)]
    pub limit_email_sent: bool,
    // ============================================
    // Bug #7: Name change approval
    // ============================================
    /// Whether user has ever set their name (first time is free, subsequent changes require approval)
    #[serde(default)]
    pub name_set: bool,
    /// Pending name change request ID (if any)
    #[serde(default)]
    pub pending_name_change_request_id: Option<String>,
    // ============================================
    // Beta Access Grant System
    // ============================================
    /// Source of user's current tier (subscription, beta grant, pre-grant)
    #[serde(default)]
    pub grant_source: Option<GrantSource>,
    /// Original granted tier (for display when tier was granted, not paid)
    #[serde(default)]
    pub granted_tier: Option<UserTier>,
    // ============================================
    // Bug #20: Email Preferences
    // ============================================
    /// User's email preferences for non-transactional emails
    #[serde(default)]
    pub email_preferences: EmailPreferences,
    // ============================================
    // Bug #22: OAuth Login
    // ============================================
    /// OAuth provider used for authentication (if any)
    #[serde(default)]
    pub oauth_provider: Option<OAuthProvider>,
    /// OAuth provider's user ID (Google sub, Apple sub, etc.)
    #[serde(default)]
    pub oauth_provider_id: Option<String>,
    /// Profile picture URL from OAuth provider
    #[serde(default)]
    pub profile_picture_url: Option<String>,
}

impl User {
    /// Create a new user with default values
    /// Bug #6: 4-tier pricing with new limits
    pub fn new(
        id: String,
        email: String,
        password_hash: String,
        first_name: String,
        middle_initial: Option<String>,
        last_name: String,
    ) -> Self {
        let now = chrono::Utc::now();
        let now_str = now.to_rfc3339();

        // Use current month in "YYYY-MM" format
        let current_month = format!("{}-{:02}", now.year(), now.month());

        // Bug #7: Check if name is set before moving
        let name_set = !first_name.is_empty() && !last_name.is_empty();

        Self {
            id,
            email,
            email_verified: false,
            password_hash,
            tier: UserTier::Free,
            created_at: now_str.clone(),
            updated_at: now_str,
            first_name,
            middle_initial,
            last_name,
            monthly_document_count: 0,
            current_quota_month: current_month,
            last_login_at: None,
            login_count: 0,
            // Bug #6: New billing fields
            stripe_customer_id: None,
            stripe_subscription_id: None,
            billing_cycle: BillingCycle::Monthly,
            overage_count: 0,
            limit_email_sent: false,
            // Bug #7: Name change approval
            name_set,
            pending_name_change_request_id: None,
            // Beta Access Grant System
            grant_source: None,
            granted_tier: None,
            // Bug #20: Email preferences
            email_preferences: EmailPreferences::default(),
            // Bug #22: OAuth
            oauth_provider: None,
            oauth_provider_id: None,
            profile_picture_url: None,
        }
    }

    /// Get full display name (combines parts)
    pub fn display_name(&self) -> String {
        match &self.middle_initial {
            Some(mi) if !mi.is_empty() => format!("{} {}. {}", self.first_name, mi, self.last_name),
            _ => format!("{} {}", self.first_name, self.last_name),
        }
    }

    /// Check and reset monthly quota if month has changed
    /// Bug #6: Also resets overage_count and limit_email_sent
    pub fn check_monthly_reset(&mut self) {
        let now = chrono::Utc::now();
        let current_month = format!("{}-{:02}", now.year(), now.month());

        if self.current_quota_month != current_month {
            // New month - reset all counters
            self.monthly_document_count = 0;
            self.overage_count = 0;
            self.limit_email_sent = false;
            self.current_quota_month = current_month;
        }
    }

    /// Check if user can create another document (based on tier limits)
    /// Bug #6: Uses tier-specific limits, paid tiers can use overage
    pub fn can_create_document(&self) -> bool {
        let total_used = self.monthly_document_count + self.overage_count;
        total_used < self.tier.max_with_overage()
    }

    /// Check if user is at their base limit (not counting overage)
    /// Used to trigger limit notification email
    pub fn is_at_base_limit(&self) -> bool {
        self.monthly_document_count >= self.tier.monthly_limit()
    }

    /// Check if user is in overage territory
    pub fn is_in_overage(&self) -> bool {
        self.monthly_document_count >= self.tier.monthly_limit() && self.tier.allows_overage()
    }

    /// Get remaining documents for this month (base limit only)
    /// Bug #6: Uses tier-specific limits
    pub fn documents_remaining(&self) -> u32 {
        let limit = self.tier.monthly_limit();
        if self.monthly_document_count >= limit {
            0
        } else {
            limit - self.monthly_document_count
        }
    }

    /// Get remaining documents including overage allowance
    pub fn documents_remaining_with_overage(&self) -> u32 {
        let max = self.tier.max_with_overage();
        let total_used = self.monthly_document_count + self.overage_count;
        if total_used >= max {
            0
        } else {
            max - total_used
        }
    }

    /// Get usage percentage (0-100) based on base limit
    pub fn usage_percentage(&self) -> u32 {
        let limit = self.tier.monthly_limit();
        if limit == 0 {
            return 100;
        }
        let percentage = (self.monthly_document_count * 100) / limit;
        percentage.min(100)
    }

    /// Record a document send, returning whether this triggered the limit
    /// Returns true if this send reached the base limit (for email notification)
    pub fn record_document_send(&mut self) -> bool {
        let was_at_limit = self.is_at_base_limit();

        if self.monthly_document_count < self.tier.monthly_limit() {
            self.monthly_document_count += 1;
        } else if self.tier.allows_overage() {
            self.overage_count += 1;
        }

        // Return true if we just hit the limit
        !was_at_limit && self.is_at_base_limit()
    }
}

/// Auth session stored in KV
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub user_id: String,
    pub created_at: String,
    pub expires_at: String,
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
}

/// Refresh token stored in KV
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub user_id: String,
    pub session_id: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Email verification token stored in VERIFICATIONS namespace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailVerification {
    pub user_id: String,
    pub email: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Password reset token stored in VERIFICATIONS namespace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordReset {
    pub user_id: String,
    pub email: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Bug #21: Account deletion request stored in VERIFICATIONS namespace
/// TTL: 1 hour (3600 seconds)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDeletionRequest {
    pub user_id: String,
    pub email: String,
    pub created_at: String,
    pub expires_at: String,
}

// ============================================
// Request/Response types
// ============================================

/// Registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub first_name: String,
    #[serde(default)]
    pub middle_initial: Option<String>,
    pub last_name: String,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Token refresh request
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Password reset request
#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

/// Resend verification email request
#[derive(Debug, Deserialize)]
pub struct ResendVerificationRequest {
    pub email: String,
}

/// Check email request (email-first UX)
#[derive(Debug, Deserialize)]
pub struct CheckEmailRequest {
    pub email: String,
}

/// Check email response (email-first UX)
#[derive(Debug, Serialize)]
pub struct CheckEmailResponse {
    /// Whether an account exists with this email
    pub exists: bool,
    /// Whether the account is verified (only meaningful if exists=true)
    pub verified: bool,
}

/// Password reset with token
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

/// Profile update request
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub middle_initial: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
}

/// Profile update response
#[derive(Debug, Serialize)]
pub struct UpdateProfileResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserPublic>,
}

/// Registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub message: String,
    /// Whether verification email was sent successfully
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_sent: Option<bool>,
    /// True if account exists but needs email verification (for resend flow)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_verification: Option<bool>,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserPublic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// True if login failed due to unverified email (for resend flow)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_verification: Option<bool>,
    /// Email address for resend verification flow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// Refresh response
#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Generic auth response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
    /// Whether an email was sent (for forgot-password, resend-verification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_sent: Option<bool>,
}

/// Public user info (safe to send to client)
/// Bug #6: 4-tier pricing with usage details
#[derive(Debug, Clone, Serialize)]
pub struct UserPublic {
    pub id: String,
    pub email: String,
    pub first_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middle_initial: Option<String>,
    pub last_name: String,
    pub tier: UserTier,
    /// Display name for the tier (e.g., "Professional")
    pub tier_display_name: String,
    /// Monthly document limit for this tier
    pub monthly_limit: u32,
    /// Documents used this month (base, not including overage)
    pub monthly_documents_used: u32,
    /// Documents remaining this month (base limit)
    pub monthly_documents_remaining: u32,
    /// Usage percentage (0-100)
    pub usage_percentage: u32,
    /// Whether user is in overage territory (paid tiers only)
    pub is_in_overage: bool,
    /// Overage documents used this month
    pub overage_count: u32,
    /// Whether tier allows overage
    pub allows_overage: bool,
    /// Maximum documents with overage
    pub max_with_overage: u32,
    /// Billing cycle (monthly/annual)
    pub billing_cycle: BillingCycle,
    /// Source of tier (subscription, beta_grant, pre_grant)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_source: Option<GrantSource>,
    /// Whether user is on a beta/granted tier (not paying)
    pub is_beta_user: bool,
    /// Backward compat: also include under old names
    #[serde(rename = "weekly_documents_remaining")]
    pub _weekly_documents_remaining: u32,
    #[serde(rename = "daily_documents_remaining")]
    pub _daily_documents_remaining: u32,
}

impl From<&User> for UserPublic {
    fn from(user: &User) -> Self {
        let remaining = user.documents_remaining();
        let is_beta_user = matches!(
            user.grant_source,
            Some(GrantSource::BetaGrant) | Some(GrantSource::PreGrant)
        );
        Self {
            id: user.id.clone(),
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            middle_initial: user.middle_initial.clone(),
            last_name: user.last_name.clone(),
            tier: user.tier,
            tier_display_name: user.tier.display_name().to_string(),
            monthly_limit: user.tier.monthly_limit(),
            monthly_documents_used: user.monthly_document_count,
            monthly_documents_remaining: remaining,
            usage_percentage: user.usage_percentage(),
            is_in_overage: user.is_in_overage(),
            overage_count: user.overage_count,
            allows_overage: user.tier.allows_overage(),
            max_with_overage: user.tier.max_with_overage(),
            billing_cycle: user.billing_cycle,
            grant_source: user.grant_source,
            is_beta_user,
            _weekly_documents_remaining: remaining,
            _daily_documents_remaining: remaining,
        }
    }
}

// ============================================================================
// Bug #4: Feature Request / Feedback System
// ============================================================================

/// Type of feedback/request from user
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    /// Bug report
    Bug,
    /// Feature request
    Feature,
    /// Request for more document quota
    MoreDocuments,
    /// General feedback
    Feedback,
    /// Name change request (Bug #7)
    NameChange,
    /// Request for beta tester access
    BetaAccess,
}

impl std::fmt::Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bug => write!(f, "Bug Report"),
            Self::Feature => write!(f, "Feature Request"),
            Self::MoreDocuments => write!(f, "More Documents Request"),
            Self::Feedback => write!(f, "General Feedback"),
            Self::NameChange => write!(f, "Name Change Request"),
            Self::BetaAccess => write!(f, "Beta Access Request"),
        }
    }
}

/// Status of a user request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    #[default]
    Pending,
    InProgress,
    Resolved,
    Rejected,
}

/// A feature request or feedback submission from a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRequest {
    /// Unique request ID
    pub id: String,
    /// User ID who submitted the request
    pub user_id: String,
    /// User's email (for easy admin viewing)
    pub user_email: String,
    /// Type of request
    pub request_type: RequestType,
    /// User's description
    pub description: String,
    /// Requested additional documents (for MoreDocuments type)
    #[serde(default)]
    pub additional_documents: Option<u32>,
    /// Request status
    pub status: RequestStatus,
    /// When request was created
    pub created_at: String,
    /// When request was last updated
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Admin notes (internal)
    #[serde(default)]
    pub admin_notes: Option<String>,
    // ============================================
    // Bug #7: Name change request fields
    // ============================================
    /// Proposed new first name (for NameChange requests)
    #[serde(default)]
    pub new_first_name: Option<String>,
    /// Proposed new middle initial (for NameChange requests)
    #[serde(default)]
    pub new_middle_initial: Option<String>,
    /// Proposed new last name (for NameChange requests)
    #[serde(default)]
    pub new_last_name: Option<String>,
    /// Current first name (for admin reference)
    #[serde(default)]
    pub current_first_name: Option<String>,
    /// Current last name (for admin reference)
    #[serde(default)]
    pub current_last_name: Option<String>,
}

impl UserRequest {
    /// Create a new user request
    pub fn new(
        user_id: String,
        user_email: String,
        request_type: RequestType,
        description: String,
        additional_documents: Option<u32>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: format!("req_{}", uuid::Uuid::new_v4().to_string().replace("-", "")),
            user_id,
            user_email,
            request_type,
            description,
            additional_documents,
            status: RequestStatus::Pending,
            created_at: now,
            updated_at: None,
            admin_notes: None,
            // Bug #7: Name change fields default to None
            new_first_name: None,
            new_middle_initial: None,
            new_last_name: None,
            current_first_name: None,
            current_last_name: None,
        }
    }

    /// Create a name change request (Bug #7)
    pub fn new_name_change(
        user_id: String,
        user_email: String,
        current_first_name: String,
        current_last_name: String,
        new_first_name: String,
        new_middle_initial: Option<String>,
        new_last_name: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let description = format!(
            "Name change: {} {} → {} {}",
            current_first_name, current_last_name, new_first_name, new_last_name
        );
        Self {
            id: format!("req_{}", uuid::Uuid::new_v4().to_string().replace("-", "")),
            user_id,
            user_email,
            request_type: RequestType::NameChange,
            description,
            additional_documents: None,
            status: RequestStatus::Pending,
            created_at: now,
            updated_at: None,
            admin_notes: None,
            new_first_name: Some(new_first_name),
            new_middle_initial,
            new_last_name: Some(new_last_name),
            current_first_name: Some(current_first_name),
            current_last_name: Some(current_last_name),
        }
    }
}

/// Request to submit feedback (from frontend)
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitRequestBody {
    pub request_type: RequestType,
    pub description: String,
    #[serde(default)]
    pub additional_documents: Option<u32>,
}

/// Response to feedback submission
#[derive(Debug, Clone, Serialize)]
pub struct SubmitRequestResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ============================================================================
// Bug #8: Admin Dashboard Types
// ============================================================================

/// Admin email - the only email allowed to access admin features
pub const ADMIN_EMAIL: &str = "orlandodowntownhome@gmail.com";

/// Check if a user is an admin
pub fn is_admin(email: &str) -> bool {
    email.eq_ignore_ascii_case(ADMIN_EMAIL)
}

/// Admin action on a user request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminRequestAction {
    Approve,
    Deny,
    MarkInProgress,
}

/// Request to update a user request (admin action)
#[derive(Debug, Clone, Deserialize)]
pub struct AdminUpdateRequestBody {
    pub action: AdminRequestAction,
    #[serde(default)]
    pub admin_notes: Option<String>,
    /// For MoreDocuments requests: how many documents to grant
    #[serde(default)]
    pub granted_documents: Option<u32>,
}

/// Response to admin updating a request
#[derive(Debug, Clone, Serialize)]
pub struct AdminUpdateRequestResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<UserRequest>,
}

/// Request to adjust user quota (admin action)
#[derive(Debug, Clone, Deserialize)]
pub struct AdminAdjustQuotaBody {
    /// New tier for the user (optional)
    #[serde(default)]
    pub new_tier: Option<UserTier>,
    /// Bonus documents to add this month (one-time grant)
    #[serde(default)]
    pub bonus_documents: Option<u32>,
    /// Admin notes for the change
    #[serde(default)]
    pub admin_notes: Option<String>,
}

/// Response to admin adjusting user quota
#[derive(Debug, Clone, Serialize)]
pub struct AdminAdjustQuotaResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// User summary for admin listing
#[derive(Debug, Clone, Serialize)]
pub struct AdminUserSummary {
    pub id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub tier: UserTier,
    pub email_verified: bool,
    pub monthly_document_count: u32,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
    /// Source of tier (subscription, beta_grant, pre_grant)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_source: Option<GrantSource>,
    /// Whether user is on a beta/granted tier
    pub is_beta_user: bool,
}

impl From<&User> for AdminUserSummary {
    fn from(user: &User) -> Self {
        let is_beta_user = matches!(
            user.grant_source,
            Some(GrantSource::BetaGrant) | Some(GrantSource::PreGrant)
        );
        Self {
            id: user.id.clone(),
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            tier: user.tier,
            email_verified: user.email_verified,
            monthly_document_count: user.monthly_document_count,
            created_at: user.created_at.clone(),
            last_login_at: user.last_login_at.clone(),
            grant_source: user.grant_source,
            is_beta_user,
        }
    }
}

/// Response listing users for admin
#[derive(Debug, Clone, Serialize)]
pub struct AdminUsersListResponse {
    pub success: bool,
    pub users: Vec<AdminUserSummary>,
    pub total: usize,
}

/// Response listing requests for admin
#[derive(Debug, Clone, Serialize)]
pub struct AdminRequestsListResponse {
    pub success: bool,
    pub requests: Vec<UserRequest>,
    pub total: usize,
}

/// Response for deleting a user
#[derive(Debug, Clone, Serialize)]
pub struct AdminDeleteUserResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ============================================================================
// Beta Access Grant Admin Types
// ============================================================================

/// Request to create a beta grant (admin action)
#[derive(Debug, Clone, Deserialize)]
pub struct AdminCreateBetaGrantBody {
    /// Email address to grant access to
    pub email: String,
    /// Tier to grant (defaults to Professional if not specified)
    #[serde(default = "default_grant_tier")]
    pub tier: UserTier,
    /// Optional admin notes
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_grant_tier() -> UserTier {
    UserTier::Professional
}

/// Response to creating a beta grant
#[derive(Debug, Clone, Serialize)]
pub struct AdminCreateBetaGrantResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant: Option<BetaGrant>,
    /// Whether user account already exists
    pub user_exists: bool,
    /// Whether user was upgraded (only if user_exists)
    pub user_upgraded: bool,
}

/// Beta grant with status info for admin listing
#[derive(Debug, Clone, Serialize)]
pub struct BetaGrantWithStatus {
    /// The grant record
    #[serde(flatten)]
    pub grant: BetaGrant,
    /// Whether user account exists
    pub user_exists: bool,
    /// User ID if account exists
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// User's current tier (may differ from grant tier if they upgraded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_tier: Option<UserTier>,
}

/// Response listing beta grants for admin
#[derive(Debug, Clone, Serialize)]
pub struct AdminBetaGrantsListResponse {
    pub success: bool,
    pub grants: Vec<BetaGrantWithStatus>,
    pub total: usize,
    /// Count of active (non-revoked) grants
    pub active_count: usize,
}

/// Response to revoking a beta grant
#[derive(Debug, Clone, Serialize)]
pub struct AdminRevokeBetaGrantResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Whether user was downgraded (only if they had no subscription)
    pub user_downgraded: bool,
}

/// Beta access request details (for BetaAccess request type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaAccessRequestDetails {
    /// Why the user wants beta access
    pub reason: String,
    /// Whether user agreed to help debug
    pub agreed_to_help: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_type_display() {
        assert_eq!(RequestType::Bug.to_string(), "Bug Report");
        assert_eq!(RequestType::Feature.to_string(), "Feature Request");
        assert_eq!(
            RequestType::MoreDocuments.to_string(),
            "More Documents Request"
        );
        assert_eq!(RequestType::Feedback.to_string(), "General Feedback");
        assert_eq!(RequestType::NameChange.to_string(), "Name Change Request");
        assert_eq!(RequestType::BetaAccess.to_string(), "Beta Access Request");
    }

    #[test]
    fn test_name_change_request_creation() {
        // Bug #7: Test name change request
        let req = UserRequest::new_name_change(
            "user_123".to_string(),
            "test@example.com".to_string(),
            "John".to_string(),
            "Doe".to_string(),
            "Jonathan".to_string(),
            Some("M".to_string()),
            "Doe".to_string(),
        );
        assert!(req.id.starts_with("req_"));
        assert_eq!(req.request_type, RequestType::NameChange);
        assert_eq!(req.new_first_name, Some("Jonathan".to_string()));
        assert_eq!(req.new_middle_initial, Some("M".to_string()));
        assert_eq!(req.new_last_name, Some("Doe".to_string()));
        assert_eq!(req.current_first_name, Some("John".to_string()));
        assert_eq!(req.current_last_name, Some("Doe".to_string()));
        assert!(req.description.contains("John Doe → Jonathan Doe"));
    }

    #[test]
    fn test_user_name_set_field() {
        // Bug #7: Test name_set field
        // New user with name: name_set should be true
        let user_with_name = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "John".to_string(),
            None,
            "Doe".to_string(),
        );
        assert!(user_with_name.name_set);

        // New user without name: name_set should be false
        let user_without_name = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "".to_string(),
            None,
            "".to_string(),
        );
        assert!(!user_without_name.name_set);
    }

    #[test]
    fn test_user_request_creation() {
        let req = UserRequest::new(
            "user_123".to_string(),
            "test@example.com".to_string(),
            RequestType::Bug,
            "Something is broken".to_string(),
            None,
        );
        assert!(req.id.starts_with("req_"));
        assert_eq!(req.user_id, "user_123");
        assert_eq!(req.status, RequestStatus::Pending);
        assert!(req.admin_notes.is_none());
    }

    #[test]
    fn test_request_serialization() {
        let req = UserRequest::new(
            "user_123".to_string(),
            "test@example.com".to_string(),
            RequestType::MoreDocuments,
            "Need more docs".to_string(),
            Some(10),
        );
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("more_documents"));
        assert!(json.contains("user_123"));
    }

    #[test]
    fn test_tier_limits() {
        // Bug #6: Test 4-tier pricing limits
        assert_eq!(UserTier::Free.monthly_limit(), 3);
        assert_eq!(UserTier::Personal.monthly_limit(), 25);
        assert_eq!(UserTier::Professional.monthly_limit(), 100);
        assert_eq!(UserTier::Business.monthly_limit(), 300);

        // Legacy Pro maps to Personal limits
        assert_eq!(UserTier::Pro.monthly_limit(), 25);

        // Overage limits
        assert_eq!(UserTier::Free.max_with_overage(), 3); // Hard limit
        assert_eq!(UserTier::Personal.max_with_overage(), 50);
        assert_eq!(UserTier::Professional.max_with_overage(), 200);
        assert_eq!(UserTier::Business.max_with_overage(), 600);

        // Only Free has no overage
        assert!(!UserTier::Free.allows_overage());
        assert!(UserTier::Personal.allows_overage());
        assert!(UserTier::Professional.allows_overage());
        assert!(UserTier::Business.allows_overage());
    }

    #[test]
    fn test_user_document_limits_free_tier() {
        let mut user = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "Test".to_string(),
            None,
            "User".to_string(),
        );

        // Free tier, 0 documents used - limit is 3
        assert!(user.can_create_document());
        assert_eq!(user.documents_remaining(), 3);
        assert_eq!(user.usage_percentage(), 0);

        // After using 2 documents
        user.monthly_document_count = 2;
        assert!(user.can_create_document());
        assert_eq!(user.documents_remaining(), 1);
        assert_eq!(user.usage_percentage(), 66); // 2/3 = 66%

        // After using 3 documents (at limit)
        user.monthly_document_count = 3;
        assert!(!user.can_create_document()); // Free has hard limit
        assert_eq!(user.documents_remaining(), 0);
        assert!(user.is_at_base_limit());
        assert!(!user.is_in_overage()); // Free tier can't have overage
    }

    #[test]
    fn test_user_document_limits_paid_tier() {
        let mut user = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "Test".to_string(),
            None,
            "User".to_string(),
        );
        user.tier = UserTier::Personal;

        // Personal tier: 25 base, 50 max with overage
        assert_eq!(user.documents_remaining(), 25);

        // At base limit
        user.monthly_document_count = 25;
        assert!(user.is_at_base_limit());
        assert!(user.can_create_document()); // Can still use overage
        assert!(user.is_in_overage());

        // Use some overage
        user.overage_count = 10;
        assert!(user.can_create_document());
        assert_eq!(user.documents_remaining_with_overage(), 15); // 50 - 25 - 10

        // At max with overage
        user.overage_count = 25;
        assert!(!user.can_create_document()); // At hard limit
    }

    #[test]
    fn test_record_document_send() {
        let mut user = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "Test".to_string(),
            None,
            "User".to_string(),
        );

        // First 2 sends don't trigger limit
        assert!(!user.record_document_send());
        assert_eq!(user.monthly_document_count, 1);
        assert!(!user.record_document_send());
        assert_eq!(user.monthly_document_count, 2);

        // Third send triggers limit notification
        assert!(user.record_document_send());
        assert_eq!(user.monthly_document_count, 3);
        assert!(user.is_at_base_limit());

        // Can't send more on Free tier
        assert!(!user.can_create_document());
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "Test".to_string(),
            Some("M".to_string()),
            "User".to_string(),
        );

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("free")); // tier serialized as lowercase
        assert!(json.contains("monthly_document_count"));
        assert!(json.contains("first_name"));
        assert!(json.contains("last_name"));
        assert!(json.contains("billing_cycle"));
        assert!(json.contains("overage_count"));
    }

    #[test]
    fn test_monthly_quota_format() {
        let user = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "Test".to_string(),
            None,
            "User".to_string(),
        );

        // current_quota_month should be in "YYYY-MM" format
        let parts: Vec<&str> = user.current_quota_month.split('-').collect();
        assert_eq!(
            parts.len(),
            2,
            "current_quota_month should be in YYYY-MM format"
        );

        let year: i32 = parts[0].parse().expect("Year should be a number");
        let month: u32 = parts[1].parse().expect("Month should be a number");

        assert!(year >= 2024 && year <= 2100, "Year should be reasonable");
        assert!((1..=12).contains(&month), "Month should be 1-12");
    }

    #[test]
    fn test_monthly_reset() {
        let mut user = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "Test".to_string(),
            None,
            "User".to_string(),
        );

        // Use all documents and some overage
        user.monthly_document_count = 3;
        user.overage_count = 2;
        user.limit_email_sent = true;
        assert!(!user.can_create_document());

        // Simulate a new month
        user.current_quota_month = "2020-01".to_string();
        user.check_monthly_reset();

        // All counters should be reset
        assert_eq!(user.monthly_document_count, 0);
        assert_eq!(user.overage_count, 0);
        assert!(!user.limit_email_sent);
        assert!(user.can_create_document());
    }

    #[test]
    fn test_tier_pricing() {
        // Monthly prices
        assert_eq!(UserTier::Free.monthly_price_cents(), 0);
        assert_eq!(UserTier::Personal.monthly_price_cents(), 1000);
        assert_eq!(UserTier::Professional.monthly_price_cents(), 2500);
        assert_eq!(UserTier::Business.monthly_price_cents(), 6000);

        // Annual prices (2 months free)
        assert_eq!(UserTier::Personal.annual_price_cents(), 10000); // $100
        assert_eq!(UserTier::Professional.annual_price_cents(), 25000); // $250
        assert_eq!(UserTier::Business.annual_price_cents(), 60000); // $600
    }

    #[test]
    fn test_legacy_pro_tier() {
        // Legacy Pro tier should deserialize and map to Personal
        let json = r#"{"tier":"pro"}"#;
        #[derive(Deserialize)]
        struct TierWrapper {
            tier: UserTier,
        }
        let wrapper: TierWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(wrapper.tier, UserTier::Pro);
        assert_eq!(wrapper.tier.monthly_limit(), 25); // Same as Personal
        assert_eq!(wrapper.tier.display_name(), "Personal");
    }

    // ============================================
    // Bug #8: Admin Dashboard Tests
    // ============================================

    #[test]
    fn test_is_admin() {
        // Admin email should be recognized
        assert!(is_admin("orlandodowntownhome@gmail.com"));
        assert!(is_admin("ORLANDODOWNTOWNHOME@GMAIL.COM")); // Case insensitive
        assert!(is_admin("OrlandoDowntownHome@Gmail.Com"));

        // Non-admin emails should not be admin
        assert!(!is_admin("other@gmail.com"));
        assert!(!is_admin("admin@example.com"));
        assert!(!is_admin(""));
    }

    #[test]
    fn test_admin_request_action_serialization() {
        // Test serialization to snake_case
        let approve = AdminRequestAction::Approve;
        let json = serde_json::to_string(&approve).unwrap();
        assert_eq!(json, r#""approve""#);

        let deny = AdminRequestAction::Deny;
        let json = serde_json::to_string(&deny).unwrap();
        assert_eq!(json, r#""deny""#);

        let in_progress = AdminRequestAction::MarkInProgress;
        let json = serde_json::to_string(&in_progress).unwrap();
        assert_eq!(json, r#""mark_in_progress""#);

        // Test deserialization
        let action: AdminRequestAction = serde_json::from_str(r#""approve""#).unwrap();
        assert_eq!(action, AdminRequestAction::Approve);
    }

    #[test]
    fn test_admin_update_request_body() {
        let json = r#"{"action":"approve","admin_notes":"Looks good","granted_documents":10}"#;
        let body: AdminUpdateRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.action, AdminRequestAction::Approve);
        assert_eq!(body.admin_notes, Some("Looks good".to_string()));
        assert_eq!(body.granted_documents, Some(10));

        // Test with minimal fields
        let json = r#"{"action":"deny"}"#;
        let body: AdminUpdateRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.action, AdminRequestAction::Deny);
        assert!(body.admin_notes.is_none());
        assert!(body.granted_documents.is_none());
    }

    #[test]
    fn test_admin_user_summary_from_user() {
        let user = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "John".to_string(),
            Some("M".to_string()),
            "Doe".to_string(),
        );

        let summary = AdminUserSummary::from(&user);
        assert_eq!(summary.id, "test-id");
        assert_eq!(summary.email, "test@example.com");
        assert_eq!(summary.first_name, "John");
        assert_eq!(summary.last_name, "Doe");
        assert_eq!(summary.tier, UserTier::Free);
        assert!(!summary.email_verified);
        assert_eq!(summary.monthly_document_count, 0);
    }

    #[test]
    fn test_admin_adjust_quota_body() {
        let json = r#"{"new_tier":"professional","bonus_documents":5,"admin_notes":"Upgraded for testing"}"#;
        let body: AdminAdjustQuotaBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.new_tier, Some(UserTier::Professional));
        assert_eq!(body.bonus_documents, Some(5));
        assert_eq!(body.admin_notes, Some("Upgraded for testing".to_string()));

        // Test with only bonus documents
        let json = r#"{"bonus_documents":10}"#;
        let body: AdminAdjustQuotaBody = serde_json::from_str(json).unwrap();
        assert!(body.new_tier.is_none());
        assert_eq!(body.bonus_documents, Some(10));
    }
}
