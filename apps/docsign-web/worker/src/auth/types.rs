//! Authentication types and data structures

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
    /// Number of documents created today (free tier limit)
    #[serde(default)]
    pub daily_document_count: u32,
    /// When the daily counter resets (ISO 8601)
    pub daily_reset_at: String,
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
        let now = chrono::Utc::now().to_rfc3339();
        let tomorrow = (chrono::Utc::now() + chrono::Duration::days(1))
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
            created_at: now.clone(),
            updated_at: now,
            name,
            daily_document_count: 0,
            daily_reset_at: tomorrow,
            last_login_at: None,
            login_count: 0,
        }
    }

    /// Check if user can create another document (based on tier limits)
    pub fn can_create_document(&self) -> bool {
        match self.tier {
            UserTier::Pro => true,
            UserTier::Free => self.daily_document_count < 1,
        }
    }

    /// Get remaining documents for today
    pub fn documents_remaining(&self) -> u32 {
        match self.tier {
            UserTier::Pro => u32::MAX,
            UserTier::Free => {
                if self.daily_document_count >= 1 {
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
}

/// Public user info (safe to send to client)
#[derive(Debug, Clone, Serialize)]
pub struct UserPublic {
    pub id: String,
    pub email: String,
    pub name: String,
    pub tier: UserTier,
    pub daily_documents_remaining: u32,
}

impl From<&User> for UserPublic {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.clone(),
            email: user.email.clone(),
            name: user.name.clone(),
            tier: user.tier,
            daily_documents_remaining: user.documents_remaining(),
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

        // After using 1 document
        user.daily_document_count = 1;
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
    }
}
