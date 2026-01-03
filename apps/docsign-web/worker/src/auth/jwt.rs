//! JWT token generation and validation
//!
//! Uses HS256 for signing with a secret key stored in Cloudflare secrets.

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Access token expiry in seconds (7 days)
pub const ACCESS_TOKEN_EXPIRY: u64 = 7 * 24 * 60 * 60;

/// Refresh token expiry in seconds (30 days)
pub const REFRESH_TOKEN_EXPIRY: u64 = 30 * 24 * 60 * 60;

/// JWT claims for access token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Email address
    pub email: String,
    /// User tier
    pub tier: String,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Expiration (Unix timestamp)
    pub exp: u64,
}

/// JWT claims for refresh token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Session ID
    pub session_id: String,
    /// Token type identifier
    pub token_type: String,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Expiration (Unix timestamp)
    pub exp: u64,
}

/// Generate an access token
///
/// # Arguments
/// * `user_id` - The user's unique identifier
/// * `email` - The user's email address
/// * `tier` - The user's tier (free/pro)
/// * `secret` - The JWT signing secret
///
/// # Returns
/// * `Ok(String)` - The encoded JWT
/// * `Err(String)` - Error message if encoding fails
pub fn generate_access_token(
    user_id: &str,
    email: &str,
    tier: &str,
    secret: &str,
) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp() as u64;

    let claims = AccessTokenClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        tier: tier.to_string(),
        iat: now,
        exp: now + ACCESS_TOKEN_EXPIRY,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| format!("Failed to generate access token: {}", e))
}

/// Generate a refresh token
///
/// # Arguments
/// * `user_id` - The user's unique identifier
/// * `session_id` - The session identifier
/// * `secret` - The JWT signing secret
///
/// # Returns
/// * `Ok(String)` - The encoded JWT
/// * `Err(String)` - Error message if encoding fails
pub fn generate_refresh_token(
    user_id: &str,
    session_id: &str,
    secret: &str,
) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp() as u64;

    let claims = RefreshTokenClaims {
        sub: user_id.to_string(),
        session_id: session_id.to_string(),
        token_type: "refresh".to_string(),
        iat: now,
        exp: now + REFRESH_TOKEN_EXPIRY,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| format!("Failed to generate refresh token: {}", e))
}

/// Validate an access token and extract claims
///
/// # Arguments
/// * `token` - The JWT to validate
/// * `secret` - The JWT signing secret
///
/// # Returns
/// * `Ok(AccessTokenClaims)` - The decoded claims
/// * `Err(String)` - Error message if validation fails
pub fn validate_access_token(token: &str, secret: &str) -> Result<AccessTokenClaims, String> {
    decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| format!("Invalid access token: {}", e))
}

/// Validate a refresh token and extract claims
///
/// # Arguments
/// * `token` - The JWT to validate
/// * `secret` - The JWT signing secret
///
/// # Returns
/// * `Ok(RefreshTokenClaims)` - The decoded claims
/// * `Err(String)` - Error message if validation fails
pub fn validate_refresh_token(token: &str, secret: &str) -> Result<RefreshTokenClaims, String> {
    let claims = decode::<RefreshTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| format!("Invalid refresh token: {}", e))?;

    // Additional check for token type
    if claims.token_type != "refresh" {
        return Err("Invalid token type".to_string());
    }

    Ok(claims)
}

/// Extract Bearer token from Authorization header
///
/// # Arguments
/// * `auth_header` - The Authorization header value (e.g., "Bearer xxx")
///
/// # Returns
/// * `Some(String)` - The extracted token
/// * `None` - If the header is missing or malformed
pub fn extract_bearer_token(auth_header: Option<&str>) -> Option<String> {
    auth_header
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-at-least-32-bytes-long";

    #[test]
    fn test_access_token_generation_and_validation() {
        let token =
            generate_access_token("user-123", "test@example.com", "free", TEST_SECRET).unwrap();

        let claims = validate_access_token(&token, TEST_SECRET).unwrap();

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.tier, "free");
    }

    #[test]
    fn test_refresh_token_generation_and_validation() {
        let token = generate_refresh_token("user-123", "session-456", TEST_SECRET).unwrap();

        let claims = validate_refresh_token(&token, TEST_SECRET).unwrap();

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.session_id, "session-456");
        assert_eq!(claims.token_type, "refresh");
    }

    #[test]
    fn test_invalid_token() {
        let result = validate_access_token("invalid-token", TEST_SECRET);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let token =
            generate_access_token("user-123", "test@example.com", "free", TEST_SECRET).unwrap();

        let result = validate_access_token(&token, "wrong-secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_bearer_token() {
        assert_eq!(
            extract_bearer_token(Some("Bearer abc123")),
            Some("abc123".to_string())
        );

        assert_eq!(extract_bearer_token(Some("abc123")), None);
        assert_eq!(extract_bearer_token(None), None);
    }
}
