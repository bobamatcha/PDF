//! Authentication module for docsign-web
//!
//! Provides user registration, login, and session management.
//!
//! ## Features
//! - Argon2id password hashing
//! - JWT access and refresh tokens
//! - Email verification
//! - Password reset
//! - Free tier document limits (1/day)
//!
//! ## Usage
//!
//! ```rust
//! use auth::handlers::{handle_register, handle_login, require_auth};
//!
//! // In your router:
//! // POST /auth/register -> handle_register
//! // POST /auth/login -> handle_login
//! // Protected endpoint:
//! // match require_auth(&req, &env).await? {
//! //     Ok(user) => { /* authenticated */ }
//! //     Err(response) => return Ok(response),
//! // }
//! ```

pub mod handlers;
pub mod jwt;
pub mod password;
pub mod types;

// Re-export commonly used items
pub use handlers::{
    get_authenticated_user, handle_forgot_password, handle_login, handle_logout, handle_refresh,
    handle_register, handle_resend_verification, handle_reset_password, handle_verify_email,
    require_auth, save_user,
};
pub use jwt::{extract_bearer_token, validate_access_token, AccessTokenClaims};
pub use types::{User, UserPublic, UserTier};
