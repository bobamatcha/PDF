//! JWT token generation and validation
//!
//! Uses HS256 for signing with a secret key stored in Cloudflare secrets.
//! Implemented manually to avoid `ring` dependency which doesn't compile to WASM.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Access token expiry in seconds (7 days)
pub const ACCESS_TOKEN_EXPIRY: u64 = 7 * 24 * 60 * 60;

/// Refresh token expiry in seconds (30 days)
pub const REFRESH_TOKEN_EXPIRY: u64 = 30 * 24 * 60 * 60;

/// JWT Header for HS256
#[derive(Debug, Serialize, Deserialize)]
struct JwtHeader {
    alg: String,
    typ: String,
}

impl Default for JwtHeader {
    fn default() -> Self {
        Self {
            alg: "HS256".to_string(),
            typ: "JWT".to_string(),
        }
    }
}

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

/// Encode claims to JWT using HS256
fn encode_jwt<T: Serialize>(claims: &T, secret: &str) -> Result<String, String> {
    // Encode header
    let header = JwtHeader::default();
    let header_json = serde_json::to_string(&header).map_err(|e| e.to_string())?;
    let header_b64 = URL_SAFE_NO_PAD.encode(header_json.as_bytes());

    // Encode payload
    let payload_json = serde_json::to_string(claims).map_err(|e| e.to_string())?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json.as_bytes());

    // Create signing input
    let signing_input = format!("{}.{}", header_b64, payload_b64);

    // Sign with HMAC-SHA256
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| format!("HMAC error: {}", e))?;
    mac.update(signing_input.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = URL_SAFE_NO_PAD.encode(&signature);

    Ok(format!("{}.{}", signing_input, signature_b64))
}

/// Decode and validate JWT using HS256
fn decode_jwt<T: DeserializeOwned>(token: &str, secret: &str) -> Result<T, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid token format".to_string());
    }

    let header_b64 = parts[0];
    let payload_b64 = parts[1];
    let signature_b64 = parts[2];

    // Verify signature
    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| format!("HMAC error: {}", e))?;
    mac.update(signing_input.as_bytes());

    let expected_signature = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| "Invalid signature encoding")?;

    mac.verify_slice(&expected_signature)
        .map_err(|_| "Invalid signature")?;

    // Verify header
    let header_bytes = URL_SAFE_NO_PAD
        .decode(header_b64)
        .map_err(|_| "Invalid header encoding")?;
    let header: JwtHeader =
        serde_json::from_slice(&header_bytes).map_err(|_| "Invalid header format")?;

    if header.alg != "HS256" {
        return Err("Unsupported algorithm".to_string());
    }

    // Decode payload
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| "Invalid payload encoding")?;
    let claims: T = serde_json::from_slice(&payload_bytes).map_err(|_| "Invalid payload format")?;

    Ok(claims)
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

    encode_jwt(&claims, secret).map_err(|e| format!("Failed to generate access token: {}", e))
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

    encode_jwt(&claims, secret).map_err(|e| format!("Failed to generate refresh token: {}", e))
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
    let claims: AccessTokenClaims =
        decode_jwt(token, secret).map_err(|e| format!("Invalid access token: {}", e))?;

    // Check expiration
    let now = chrono::Utc::now().timestamp() as u64;
    if claims.exp < now {
        return Err("Access token expired".to_string());
    }

    Ok(claims)
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
    let claims: RefreshTokenClaims =
        decode_jwt(token, secret).map_err(|e| format!("Invalid refresh token: {}", e))?;

    // Check expiration
    let now = chrono::Utc::now().timestamp() as u64;
    if claims.exp < now {
        return Err("Refresh token expired".to_string());
    }

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

    #[test]
    fn test_jwt_format() {
        let token =
            generate_access_token("user-123", "test@example.com", "free", TEST_SECRET).unwrap();

        // JWT should have 3 parts separated by dots
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);

        // Header should decode to valid JSON with HS256
        let header_bytes = URL_SAFE_NO_PAD.decode(parts[0]).unwrap();
        let header: JwtHeader = serde_json::from_slice(&header_bytes).unwrap();
        assert_eq!(header.alg, "HS256");
        assert_eq!(header.typ, "JWT");
    }
}
