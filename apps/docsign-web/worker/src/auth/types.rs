//! Authentication types and data structures

use chrono::Datelike;
use serde::{Deserialize, Serialize};

/// User tier for rate limiting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserTier {
    Free,
    Pro,
}

impl Default for UserTier {
    fn default() -> Self {
        Self::Free
    }
}

/// User record stored in KV
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
    /// Name for display purposes
    #[serde(default)]
    pub name: String,
    /// Number of documents created this week (free tier limit: 1/week)
    #[serde(default, alias = "daily_document_count")]
    pub weekly_document_count: u32,
    /// When the weekly counter resets (ISO 8601, Monday 00:00 UTC)
    #[serde(alias = "daily_reset_at")]
    pub weekly_reset_at: String,
    /// Last login timestamp
    #[serde(default)]
    pub last_login_at: Option<String>,
    /// Total login count for analytics
    #[serde(default)]
    pub login_count: u32,
}

impl User {
    /// Create a new user with default values
    pub fn new(id: String, email: String, password_hash: String, name: String) -> Self {
        let now = chrono::Utc::now();
        let now_str = now.to_rfc3339();

        // Calculate next Monday at 00:00 UTC for weekly reset
        let days_until_monday = (8 - now.weekday().num_days_from_monday()) % 7;
        let days_until_monday = if days_until_monday == 0 {
            7
        } else {
            days_until_monday
        };
        let next_monday = (now + chrono::Duration::days(days_until_monday as i64))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .to_rfc3339();

        Self {
            id,
            email,
            email_verified: false,
            password_hash,
            tier: UserTier::Free,
            created_at: now_str.clone(),
            updated_at: now_str,
            name,
            weekly_document_count: 0,
            weekly_reset_at: next_monday,
            last_login_at: None,
            login_count: 0,
        }
    }

    /// Check if user can create another document (based on tier limits)
    pub fn can_create_document(&self) -> bool {
        match self.tier {
            UserTier::Pro => true,
            UserTier::Free => self.weekly_document_count < 1,
        }
    }

    /// Get remaining documents for this week
    pub fn documents_remaining(&self) -> u32 {
        match self.tier {
            UserTier::Pro => u32::MAX,
            UserTier::Free => {
                if self.weekly_document_count >= 1 {
                    0
                } else {
                    1
                }
            }
        }
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

// ============================================
// Request/Response types
// ============================================

/// Registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
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

/// Password reset with token
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
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
#[derive(Debug, Clone, Serialize)]
pub struct UserPublic {
    pub id: String,
    pub email: String,
    pub name: String,
    pub tier: UserTier,
    pub weekly_documents_remaining: u32,
    /// Backward compat: also include under old name
    #[serde(rename = "daily_documents_remaining")]
    pub _daily_documents_remaining: u32,
}

impl From<&User> for UserPublic {
    fn from(user: &User) -> Self {
        let remaining = user.documents_remaining();
        Self {
            id: user.id.clone(),
            email: user.email.clone(),
            name: user.name.clone(),
            tier: user.tier,
            weekly_documents_remaining: remaining,
            _daily_documents_remaining: remaining,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_document_limits() {
        let mut user = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "Test User".to_string(),
        );

        // Free tier, 0 documents used
        assert!(user.can_create_document());
        assert_eq!(user.documents_remaining(), 1);

        // After using 1 document this week
        user.weekly_document_count = 1;
        assert!(!user.can_create_document());
        assert_eq!(user.documents_remaining(), 0);

        // Pro tier has unlimited
        user.tier = UserTier::Pro;
        assert!(user.can_create_document());
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "Test".to_string(),
        );

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("free")); // tier serialized as lowercase
        assert!(json.contains("weekly_document_count"));
    }

    #[test]
    fn test_weekly_reset_calculation() {
        let user = User::new(
            "test-id".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
            "Test".to_string(),
        );

        // weekly_reset_at should be a valid RFC3339 timestamp
        let reset_time = chrono::DateTime::parse_from_rfc3339(&user.weekly_reset_at);
        assert!(
            reset_time.is_ok(),
            "weekly_reset_at should be valid RFC3339"
        );

        // It should be in the future
        let reset = reset_time.unwrap();
        assert!(
            reset > chrono::Utc::now(),
            "weekly_reset_at should be in the future"
        );
    }
}
