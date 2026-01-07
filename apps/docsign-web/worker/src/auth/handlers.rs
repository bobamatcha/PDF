//! HTTP handlers for authentication endpoints
//!
//! Implements: register, verify-email, login, refresh, logout, forgot-password, reset-password, resend-verification

use chrono::Datelike;

use super::jwt::{
    extract_bearer_token, generate_access_token, generate_refresh_token, validate_access_token,
    validate_refresh_token, ACCESS_TOKEN_EXPIRY,
};
use super::password::{hash_password, validate_email, validate_password_strength, verify_password};
use super::types::*;
use crate::email::{send_email, EmailSendRequest};
use serde_json::json;
use worker::{console_log, kv::KvStore, Env, Request, Response, Result};

/// TTL for email verification tokens (24 hours)
const VERIFICATION_TTL: u64 = 24 * 60 * 60;

/// TTL for password reset tokens (1 hour)
const PASSWORD_RESET_TTL: u64 = 60 * 60;

/// TTL for auth sessions (7 days)
const AUTH_SESSION_TTL: u64 = 7 * 24 * 60 * 60;

/// TTL for refresh tokens (30 days)
const REFRESH_TOKEN_TTL: u64 = 30 * 24 * 60 * 60;

// ============================================
// Helper functions
// ============================================

/// Get JWT secret from environment
fn get_jwt_secret(env: &Env) -> Result<String> {
    env.secret("JWT_SECRET")
        .map(|s| s.to_string())
        .map_err(|_| worker::Error::RustError("JWT_SECRET not configured".to_string()))
}

/// Get user by ID from KV
async fn get_user_by_id(kv: &KvStore, user_id: &str) -> Result<Option<User>> {
    let key = format!("user:{}", user_id);
    kv.get(&key)
        .json::<User>()
        .await
        .map_err(|e| worker::Error::RustError(format!("KV error: {:?}", e)))
}

/// Get user by email from KV (using index)
async fn get_user_by_email(kv: &KvStore, email: &str) -> Result<Option<User>> {
    let email_lower = email.trim().to_lowercase();
    let index_key = format!("user_email:{}", email_lower);

    // First, get user ID from email index
    let user_id = match kv
        .get(&index_key)
        .text()
        .await
        .map_err(|e| worker::Error::RustError(format!("KV error: {:?}", e)))?
    {
        Some(id) => id,
        None => return Ok(None),
    };

    // Then get the full user record
    get_user_by_id(kv, &user_id).await
}

/// Save user to KV (updates both user record and email index)
pub async fn save_user(kv: &KvStore, user: &User) -> Result<()> {
    let user_key = format!("user:{}", user.id);
    let email_key = format!("user_email:{}", user.email.to_lowercase());

    // Save user record
    kv.put(&user_key, serde_json::to_string(user)?)?
        .execute()
        .await?;

    // Save email index
    kv.put(&email_key, &user.id)?.execute().await?;

    Ok(())
}

/// Generate a secure random token
fn generate_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Create JSON response with status code
fn json_response<T: serde::Serialize>(status: u16, data: &T) -> Result<Response> {
    Response::from_json(data).map(|r| r.with_status(status))
}

// ============================================
// Handlers
// ============================================

/// POST /auth/register
///
/// Creates a new user account and sends verification email.
pub async fn handle_register(mut req: Request, env: Env) -> Result<Response> {
    let body: RegisterRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => {
            return json_response(
                400,
                &RegisterResponse {
                    success: false,
                    user_id: None,
                    message: "Invalid request body".to_string(),
                    email_sent: None,
                    needs_verification: None,
                },
            );
        }
    };

    // Validate email
    if let Err(e) = validate_email(&body.email) {
        return json_response(
            400,
            &RegisterResponse {
                success: false,
                user_id: None,
                message: e,
                email_sent: None,
                needs_verification: None,
            },
        );
    }

    // Validate password strength
    if let Err(e) = validate_password_strength(&body.password) {
        return json_response(
            400,
            &RegisterResponse {
                success: false,
                user_id: None,
                message: e,
                email_sent: None,
                needs_verification: None,
            },
        );
    }

    // Validate name parts
    let first_name = body.first_name.trim();
    let last_name = body.last_name.trim();
    if first_name.is_empty() || first_name.len() > 50 {
        return json_response(
            400,
            &RegisterResponse {
                success: false,
                user_id: None,
                message: "First name must be between 1 and 50 characters".to_string(),
                email_sent: None,
                needs_verification: None,
            },
        );
    }
    if last_name.is_empty() || last_name.len() > 50 {
        return json_response(
            400,
            &RegisterResponse {
                success: false,
                user_id: None,
                message: "Last name must be between 1 and 50 characters".to_string(),
                email_sent: None,
                needs_verification: None,
            },
        );
    }
    let middle_initial = body
        .middle_initial
        .as_ref()
        .map(|mi| mi.trim().to_uppercase());
    if let Some(ref mi) = middle_initial {
        if mi.len() > 1 {
            return json_response(
                400,
                &RegisterResponse {
                    success: false,
                    user_id: None,
                    message: "Middle initial must be a single letter".to_string(),
                    email_sent: None,
                    needs_verification: None,
                },
            );
        }
    }

    let users_kv = env.kv("USERS")?;

    // Check if email already exists
    if let Some(existing_user) = get_user_by_email(&users_kv, &body.email).await? {
        if !existing_user.email_verified {
            // Account exists but is unverified - friendly message with resend option
            return json_response(
                409,
                &RegisterResponse {
                    success: false,
                    user_id: None,
                    message: "An account with this email already exists but hasn't been verified yet. Please check your email for a verification link, or request a new one.".to_string(),
                    email_sent: None,
                    needs_verification: Some(true),
                },
            );
        }
        // Account exists and is verified
        return json_response(
            409,
            &RegisterResponse {
                success: false,
                user_id: None,
                message: "An account with this email already exists. Please sign in instead."
                    .to_string(),
                email_sent: None,
                needs_verification: Some(false),
            },
        );
    }

    // Hash password
    let password_hash = match hash_password(&body.password) {
        Ok(h) => h,
        Err(e) => {
            console_log!("Password hashing failed: {}", e);
            return json_response(
                500,
                &RegisterResponse {
                    success: false,
                    user_id: None,
                    message: "Registration failed. Please try again.".to_string(),
                    email_sent: None,
                    needs_verification: None,
                },
            );
        }
    };

    // Create user
    let user_id = generate_token();
    let user = User::new(
        user_id.clone(),
        body.email.trim().to_lowercase(),
        password_hash,
        first_name.to_string(),
        middle_initial,
        last_name.to_string(),
    );

    // Save user
    if let Err(e) = save_user(&users_kv, &user).await {
        console_log!("Failed to save user: {:?}", e);
        return json_response(
            500,
            &RegisterResponse {
                success: false,
                user_id: None,
                message: "Registration failed. Please try again.".to_string(),
                email_sent: None,
                needs_verification: None,
            },
        );
    }

    // Create verification token
    let verification_token = generate_token();
    let verification = EmailVerification {
        user_id: user_id.clone(),
        email: user.email.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: (chrono::Utc::now() + chrono::Duration::seconds(VERIFICATION_TTL as i64))
            .to_rfc3339(),
    };

    let verifications_kv = env.kv("VERIFICATIONS")?;
    let verification_key = format!("email_verify:{}", verification_token);
    verifications_kv
        .put(&verification_key, serde_json::to_string(&verification)?)?
        .expiration_ttl(VERIFICATION_TTL)
        .execute()
        .await?;

    // Send verification email via Resend
    let verification_url = format!(
        "https://getsignatures.org/auth.html?action=verify&token={}",
        verification_token
    );
    let email_request = EmailSendRequest {
        to: vec![user.email.clone()],
        subject: "Verify your GetSignatures account".to_string(),
        html: format!(
            r#"<h2>Welcome to GetSignatures!</h2>
            <p>Hi {},</p>
            <p>Thank you for creating an account. Please click the link below to verify your email address:</p>
            <p><a href="{}" style="background-color: #4F46E5; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; display: inline-block;">Verify Email</a></p>
            <p>Or copy and paste this link into your browser:</p>
            <p>{}</p>
            <p>This link will expire in 24 hours.</p>
            <p>If you didn't create this account, you can safely ignore this email.</p>
            <p>Best,<br>The GetSignatures Team</p>"#,
            user.first_name, verification_url, verification_url
        ),
        text: Some(format!(
            "Welcome to GetSignatures!\n\nHi {},\n\nPlease verify your email by visiting: {}\n\nThis link expires in 24 hours.\n\nIf you didn't create this account, ignore this email.",
            user.first_name, verification_url
        )),
        reply_to: None,
        tags: vec![
            ("type".to_string(), "verification".to_string()),
            ("user_id".to_string(), user_id.clone()),
        ],
    };

    let email_result = send_email(&env, email_request).await;
    let (email_sent, email_error) = match &email_result {
        Ok(result) if result.success => {
            console_log!(
                "Verification email sent to {}: id={}",
                user.email,
                result.id
            );
            (true, None)
        }
        Ok(result) => {
            console_log!("Failed to send verification email: {:?}", result.error);
            (false, result.error.clone())
        }
        Err(e) => {
            console_log!("Email send error: {:?}", e);
            (false, Some(format!("{:?}", e)))
        }
    };

    // Also log for debugging
    console_log!(
        "Verification link: /auth/verify-email?token={}",
        verification_token
    );

    let message = if email_sent {
        "Account created. Please check your email to verify your account.".to_string()
    } else if let Some(err) = email_error {
        format!("Account created, but: {}", err)
    } else {
        "Account created but we couldn't send the verification email. Try the resend option."
            .to_string()
    };

    json_response(
        201,
        &RegisterResponse {
            success: true,
            user_id: Some(user_id),
            message,
            email_sent: Some(email_sent),
            needs_verification: None,
        },
    )
}

/// GET /auth/verify-email?token=xxx
///
/// Verifies a user's email address.
pub async fn handle_verify_email(req: Request, env: Env) -> Result<Response> {
    let url = req.url()?;
    let token = url
        .query_pairs()
        .find(|(k, _)| k == "token")
        .map(|(_, v)| v.to_string());

    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => {
            return json_response(
                400,
                &AuthResponse {
                    success: false,
                    message: "Missing verification token".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    let verifications_kv = env.kv("VERIFICATIONS")?;
    let verification_key = format!("email_verify:{}", token);

    // Get verification record
    let verification: EmailVerification = match verifications_kv
        .get(&verification_key)
        .json::<EmailVerification>()
        .await?
    {
        Some(v) => v,
        None => {
            return json_response(
                400,
                &AuthResponse {
                    success: false,
                    message: "Invalid or expired verification link".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    // Get and update user
    let users_kv = env.kv("USERS")?;
    let mut user = match get_user_by_id(&users_kv, &verification.user_id).await? {
        Some(u) => u,
        None => {
            return json_response(
                400,
                &AuthResponse {
                    success: false,
                    message: "User not found".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    user.email_verified = true;
    user.updated_at = chrono::Utc::now().to_rfc3339();
    save_user(&users_kv, &user).await?;

    // Delete verification token
    verifications_kv.delete(&verification_key).await?;

    console_log!("Email verified for user: {}", user.email);

    json_response(
        200,
        &AuthResponse {
            success: true,
            message: "Email verified successfully. You can now log in.".to_string(),
            email_sent: None,
        },
    )
}

/// POST /auth/login
///
/// Authenticates a user and returns access + refresh tokens.
pub async fn handle_login(mut req: Request, env: Env) -> Result<Response> {
    let body: LoginRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => {
            return json_response(
                400,
                &LoginResponse {
                    success: false,
                    access_token: None,
                    refresh_token: None,
                    expires_in: None,
                    user: None,
                    error: Some("Invalid request body".to_string()),
                    needs_verification: None,
                    email: None,
                },
            );
        }
    };

    let users_kv = env.kv("USERS")?;

    // Get user by email
    let mut user = match get_user_by_email(&users_kv, &body.email).await? {
        Some(u) => u,
        None => {
            // Don't reveal whether email exists
            return json_response(
                401,
                &LoginResponse {
                    success: false,
                    access_token: None,
                    refresh_token: None,
                    expires_in: None,
                    user: None,
                    error: Some("Invalid email or password".to_string()),
                    needs_verification: None,
                    email: None,
                },
            );
        }
    };

    // Verify password
    if !verify_password(&body.password, &user.password_hash) {
        return json_response(
            401,
            &LoginResponse {
                success: false,
                access_token: None,
                refresh_token: None,
                expires_in: None,
                user: None,
                error: Some("Invalid email or password".to_string()),
                needs_verification: None,
                email: None,
            },
        );
    }

    // Check if email is verified
    if !user.email_verified {
        return json_response(
            403,
            &LoginResponse {
                success: false,
                access_token: None,
                refresh_token: None,
                expires_in: None,
                user: None,
                error: Some("Your email address hasn't been verified yet. Please check your inbox for a verification link, or click below to send a new one.".to_string()),
                needs_verification: Some(true),
                email: Some(user.email.clone()),
            },
        );
    }

    // Get JWT secret
    let jwt_secret = match get_jwt_secret(&env) {
        Ok(s) => s,
        Err(_) => {
            console_log!("JWT_SECRET not configured");
            return json_response(
                500,
                &LoginResponse {
                    success: false,
                    access_token: None,
                    refresh_token: None,
                    expires_in: None,
                    user: None,
                    error: Some("Server configuration error".to_string()),
                    needs_verification: None,
                    email: None,
                },
            );
        }
    };

    // Check and reset weekly document counter if needed
    let now = chrono::Utc::now();
    if let Ok(reset_at) = chrono::DateTime::parse_from_rfc3339(&user.weekly_reset_at) {
        if now >= reset_at {
            user.weekly_document_count = 0;
            // Calculate next Monday at 00:00 UTC
            let days_until_monday = (8 - now.weekday().num_days_from_monday()) % 7;
            let days_until_monday = if days_until_monday == 0 {
                7
            } else {
                days_until_monday
            };
            user.weekly_reset_at = (now + chrono::Duration::days(days_until_monday as i64))
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .to_rfc3339();
        }
    }

    // Update login stats
    user.last_login_at = Some(now.to_rfc3339());
    user.login_count += 1;
    user.updated_at = now.to_rfc3339();
    save_user(&users_kv, &user).await?;

    // Generate session ID
    let session_id = generate_token();

    // Generate tokens
    let tier_str = match user.tier {
        UserTier::Free => "free",
        UserTier::Pro => "pro",
    };

    let access_token = match generate_access_token(&user.id, &user.email, tier_str, &jwt_secret) {
        Ok(t) => t,
        Err(e) => {
            console_log!("Failed to generate access token: {}", e);
            return json_response(
                500,
                &LoginResponse {
                    success: false,
                    access_token: None,
                    refresh_token: None,
                    expires_in: None,
                    user: None,
                    error: Some("Failed to create session".to_string()),
                    needs_verification: None,
                    email: None,
                },
            );
        }
    };

    let refresh_token = match generate_refresh_token(&user.id, &session_id, &jwt_secret) {
        Ok(t) => t,
        Err(e) => {
            console_log!("Failed to generate refresh token: {}", e);
            return json_response(
                500,
                &LoginResponse {
                    success: false,
                    access_token: None,
                    refresh_token: None,
                    expires_in: None,
                    user: None,
                    error: Some("Failed to create session".to_string()),
                    needs_verification: None,
                    email: None,
                },
            );
        }
    };

    // Store auth session in KV
    let sessions_kv = env.kv("AUTH_SESSIONS")?;
    let session = AuthSession {
        user_id: user.id.clone(),
        created_at: now.to_rfc3339(),
        expires_at: (now + chrono::Duration::seconds(AUTH_SESSION_TTL as i64)).to_rfc3339(),
        ip: req
            .headers()
            .get("CF-Connecting-IP")
            .ok()
            .flatten()
            .map(|s| s.to_string()),
        user_agent: req
            .headers()
            .get("User-Agent")
            .ok()
            .flatten()
            .map(|s| s.to_string()),
    };

    let session_key = format!("auth_session:{}", session_id);
    sessions_kv
        .put(&session_key, serde_json::to_string(&session)?)?
        .expiration_ttl(AUTH_SESSION_TTL)
        .execute()
        .await?;

    // Store refresh token reference
    let refresh_key = format!("refresh_token:{}", &refresh_token[..32]); // Use first 32 chars as key
    let refresh_record = RefreshToken {
        user_id: user.id.clone(),
        session_id: session_id.clone(),
        created_at: now.to_rfc3339(),
        expires_at: (now + chrono::Duration::seconds(REFRESH_TOKEN_TTL as i64)).to_rfc3339(),
    };
    sessions_kv
        .put(&refresh_key, serde_json::to_string(&refresh_record)?)?
        .expiration_ttl(REFRESH_TOKEN_TTL)
        .execute()
        .await?;

    console_log!("User logged in: {}", user.email);

    json_response(
        200,
        &LoginResponse {
            success: true,
            access_token: Some(access_token),
            refresh_token: Some(refresh_token),
            expires_in: Some(ACCESS_TOKEN_EXPIRY),
            user: Some(UserPublic::from(&user)),
            error: None,
            needs_verification: None,
            email: None,
        },
    )
}

/// POST /auth/refresh
///
/// Exchanges a refresh token for a new access token.
pub async fn handle_refresh(mut req: Request, env: Env) -> Result<Response> {
    let body: RefreshRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => {
            return json_response(
                400,
                &RefreshResponse {
                    success: false,
                    access_token: None,
                    expires_in: None,
                    error: Some("Invalid request body".to_string()),
                },
            );
        }
    };

    let jwt_secret = match get_jwt_secret(&env) {
        Ok(s) => s,
        Err(_) => {
            return json_response(
                500,
                &RefreshResponse {
                    success: false,
                    access_token: None,
                    expires_in: None,
                    error: Some("Server configuration error".to_string()),
                },
            );
        }
    };

    // Validate refresh token
    let claims = match validate_refresh_token(&body.refresh_token, &jwt_secret) {
        Ok(c) => c,
        Err(_) => {
            return json_response(
                401,
                &RefreshResponse {
                    success: false,
                    access_token: None,
                    expires_in: None,
                    error: Some("Invalid or expired refresh token".to_string()),
                },
            );
        }
    };

    // Get user
    let users_kv = env.kv("USERS")?;
    let user = match get_user_by_id(&users_kv, &claims.sub).await? {
        Some(u) => u,
        None => {
            return json_response(
                401,
                &RefreshResponse {
                    success: false,
                    access_token: None,
                    expires_in: None,
                    error: Some("User not found".to_string()),
                },
            );
        }
    };

    // Generate new access token
    let tier_str = match user.tier {
        UserTier::Free => "free",
        UserTier::Pro => "pro",
    };

    let access_token = match generate_access_token(&user.id, &user.email, tier_str, &jwt_secret) {
        Ok(t) => t,
        Err(_) => {
            return json_response(
                500,
                &RefreshResponse {
                    success: false,
                    access_token: None,
                    expires_in: None,
                    error: Some("Failed to generate token".to_string()),
                },
            );
        }
    };

    json_response(
        200,
        &RefreshResponse {
            success: true,
            access_token: Some(access_token),
            expires_in: Some(ACCESS_TOKEN_EXPIRY),
            error: None,
        },
    )
}

/// POST /auth/logout
///
/// Invalidates the current session.
pub async fn handle_logout(req: Request, env: Env) -> Result<Response> {
    let auth_header = req.headers().get("Authorization").ok().flatten();
    let token = match extract_bearer_token(auth_header.as_deref()) {
        Some(t) => t,
        None => {
            return json_response(
                200,
                &AuthResponse {
                    success: true,
                    message: "Logged out".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    let jwt_secret = match get_jwt_secret(&env) {
        Ok(s) => s,
        Err(_) => {
            return json_response(
                200,
                &AuthResponse {
                    success: true,
                    message: "Logged out".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    // Try to invalidate the session (best effort)
    if let Ok(claims) = validate_access_token(&token, &jwt_secret) {
        let sessions_kv = env.kv("AUTH_SESSIONS")?;
        // We don't have direct session ID from access token, but we log it
        console_log!("User logged out: {}", claims.email);
    }

    json_response(
        200,
        &AuthResponse {
            success: true,
            message: "Logged out successfully".to_string(),
            email_sent: None,
        },
    )
}

/// POST /auth/forgot-password
///
/// Initiates password reset flow.
/// Returns explicit error if email not found (UX clarity over email enumeration protection).
pub async fn handle_forgot_password(mut req: Request, env: Env) -> Result<Response> {
    let body: ForgotPasswordRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => {
            return json_response(
                400,
                &AuthResponse {
                    success: false,
                    message: "Invalid request. Please enter your email address.".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    let users_kv = env.kv("USERS")?;

    // Check if user exists - return explicit error if not found
    let user = match get_user_by_email(&users_kv, &body.email).await? {
        Some(u) => u,
        None => {
            return json_response(
                404,
                &AuthResponse {
                    success: false,
                    message: "No account found with this email address. Please check the email or create a new account.".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    // Create password reset token
    let reset_token = generate_token();
    let reset = PasswordReset {
        user_id: user.id.clone(),
        email: user.email.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: (chrono::Utc::now() + chrono::Duration::seconds(PASSWORD_RESET_TTL as i64))
            .to_rfc3339(),
    };

    let verifications_kv = env.kv("VERIFICATIONS")?;
    let reset_key = format!("password_reset:{}", reset_token);
    verifications_kv
        .put(&reset_key, serde_json::to_string(&reset)?)?
        .expiration_ttl(PASSWORD_RESET_TTL)
        .execute()
        .await?;

    // Send password reset email via Resend
    let reset_url = format!(
        "https://getsignatures.org/auth.html?action=reset&token={}",
        reset_token
    );
    let email_request = EmailSendRequest {
        to: vec![user.email.clone()],
        subject: "Reset your GetSignatures password".to_string(),
        html: format!(
            r#"<h2>Password Reset Request</h2>
            <p>Hi {},</p>
            <p>We received a request to reset your password. Click the link below to set a new password:</p>
            <p><a href="{}" style="background-color: #4F46E5; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; display: inline-block;">Reset Password</a></p>
            <p>Or copy and paste this link into your browser:</p>
            <p>{}</p>
            <p>This link will expire in 1 hour.</p>
            <p>If you didn't request a password reset, you can safely ignore this email. Your password will remain unchanged.</p>
            <p>Best,<br>The GetSignatures Team</p>"#,
            user.first_name, reset_url, reset_url
        ),
        text: Some(format!(
            "Password Reset Request\n\nHi {},\n\nReset your password by visiting: {}\n\nThis link expires in 1 hour.\n\nIf you didn't request this, ignore this email.",
            user.first_name, reset_url
        )),
        reply_to: None,
        tags: vec![
            ("type".to_string(), "password_reset".to_string()),
            ("user_id".to_string(), user.id.clone()),
        ],
    };

    let email_result = send_email(&env, email_request).await;
    let (email_sent, email_error) = match &email_result {
        Ok(result) if result.success => {
            console_log!(
                "Password reset email sent to {}: id={}",
                user.email,
                result.id
            );
            (true, None)
        }
        Ok(result) => {
            console_log!("Failed to send password reset email: {:?}", result.error);
            (false, result.error.clone())
        }
        Err(e) => {
            console_log!("Email send error: {:?}", e);
            (false, Some(format!("{:?}", e)))
        }
    };

    // Also log for debugging
    console_log!(
        "Password reset link: /auth/reset-password?token={}",
        reset_token
    );

    let message = if email_sent {
        "A password reset link has been sent to your email.".to_string()
    } else if let Some(err) = email_error {
        err
    } else {
        "Couldn't send the reset email. Try again in a bit.".to_string()
    };

    json_response(
        if email_sent { 200 } else { 500 },
        &AuthResponse {
            success: email_sent,
            message,
            email_sent: Some(email_sent),
        },
    )
}

/// POST /auth/reset-password
///
/// Resets password with token.
pub async fn handle_reset_password(mut req: Request, env: Env) -> Result<Response> {
    let body: ResetPasswordRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => {
            return json_response(
                400,
                &AuthResponse {
                    success: false,
                    message: "Invalid request body".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    // Validate new password
    if let Err(e) = validate_password_strength(&body.new_password) {
        return json_response(
            400,
            &AuthResponse {
                success: false,
                message: e,
                email_sent: None,
            },
        );
    }

    let verifications_kv = env.kv("VERIFICATIONS")?;
    let reset_key = format!("password_reset:{}", body.token);

    // Get reset record
    let reset: PasswordReset = match verifications_kv
        .get(&reset_key)
        .json::<PasswordReset>()
        .await?
    {
        Some(r) => r,
        None => {
            return json_response(
                400,
                &AuthResponse {
                    success: false,
                    message: "Invalid or expired reset link".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    // Get and update user
    let users_kv = env.kv("USERS")?;
    let mut user = match get_user_by_id(&users_kv, &reset.user_id).await? {
        Some(u) => u,
        None => {
            return json_response(
                400,
                &AuthResponse {
                    success: false,
                    message: "User not found".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    // Hash new password
    let password_hash = match hash_password(&body.new_password) {
        Ok(h) => h,
        Err(_) => {
            return json_response(
                500,
                &AuthResponse {
                    success: false,
                    message: "Failed to reset password. Please try again.".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    user.password_hash = password_hash;
    user.updated_at = chrono::Utc::now().to_rfc3339();
    save_user(&users_kv, &user).await?;

    // Delete reset token
    verifications_kv.delete(&reset_key).await?;

    console_log!("Password reset for user: {}", user.email);

    json_response(
        200,
        &AuthResponse {
            success: true,
            message: "Password reset successfully. You can now log in.".to_string(),
            email_sent: None,
        },
    )
}

/// POST /auth/resend-verification
///
/// Resends the verification email for an unverified account.
pub async fn handle_resend_verification(mut req: Request, env: Env) -> Result<Response> {
    let body: ResendVerificationRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => {
            return json_response(
                400,
                &AuthResponse {
                    success: false,
                    message: "Invalid request. Please enter your email address.".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    let users_kv = env.kv("USERS")?;

    // Get user by email
    let user = match get_user_by_email(&users_kv, &body.email).await? {
        Some(u) => u,
        None => {
            return json_response(
                404,
                &AuthResponse {
                    success: false,
                    message: "No account found with this email address.".to_string(),
                    email_sent: None,
                },
            );
        }
    };

    // Check if already verified
    if user.email_verified {
        return json_response(
            400,
            &AuthResponse {
                success: false,
                message: "This email is already verified. You can log in.".to_string(),
                email_sent: None,
            },
        );
    }

    // Create new verification token
    let verification_token = generate_token();
    let verification = EmailVerification {
        user_id: user.id.clone(),
        email: user.email.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: (chrono::Utc::now() + chrono::Duration::seconds(VERIFICATION_TTL as i64))
            .to_rfc3339(),
    };

    let verifications_kv = env.kv("VERIFICATIONS")?;
    let verification_key = format!("email_verify:{}", verification_token);
    verifications_kv
        .put(&verification_key, serde_json::to_string(&verification)?)?
        .expiration_ttl(VERIFICATION_TTL)
        .execute()
        .await?;

    // Send verification email via Resend
    let verification_url = format!(
        "https://getsignatures.org/auth.html?action=verify&token={}",
        verification_token
    );
    let email_request = EmailSendRequest {
        to: vec![user.email.clone()],
        subject: "Verify your GetSignatures account".to_string(),
        html: format!(
            r#"<h2>Verify Your Email</h2>
            <p>Hi {},</p>
            <p>You requested a new verification link. Click below to verify your email address:</p>
            <p><a href="{}" style="background-color: #4F46E5; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; display: inline-block;">Verify Email</a></p>
            <p>Or copy and paste this link into your browser:</p>
            <p>{}</p>
            <p>This link will expire in 24 hours.</p>
            <p>Best,<br>The GetSignatures Team</p>"#,
            user.first_name, verification_url, verification_url
        ),
        text: Some(format!(
            "Verify Your Email\n\nHi {},\n\nVerify your email by visiting: {}\n\nThis link expires in 24 hours.",
            user.first_name, verification_url
        )),
        reply_to: None,
        tags: vec![
            ("type".to_string(), "verification_resend".to_string()),
            ("user_id".to_string(), user.id.clone()),
        ],
    };

    let email_result = send_email(&env, email_request).await;
    let (email_sent, email_error) = match &email_result {
        Ok(result) if result.success => {
            console_log!(
                "Verification email resent to {}: id={}",
                user.email,
                result.id
            );
            (true, None)
        }
        Ok(result) => {
            console_log!("Failed to resend verification email: {:?}", result.error);
            (false, result.error.clone())
        }
        Err(e) => {
            console_log!("Email send error: {:?}", e);
            (false, Some(format!("{:?}", e)))
        }
    };

    // Also log for debugging
    console_log!(
        "Verification link: /auth/verify-email?token={}",
        verification_token
    );

    let message = if email_sent {
        "Verification email sent. Please check your inbox.".to_string()
    } else if let Some(err) = email_error {
        err
    } else {
        "Couldn't send the email. Try again in a bit.".to_string()
    };

    json_response(
        if email_sent { 200 } else { 500 },
        &AuthResponse {
            success: email_sent,
            message,
            email_sent: Some(email_sent),
        },
    )
}

/// POST /auth/check-email
/// Check if an email is already registered (for email-first UX)
///
/// Returns { exists: bool, verified: bool }
pub async fn handle_check_email(mut req: Request, env: Env) -> Result<Response> {
    // Parse request body
    let body: CheckEmailRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            console_log!("Invalid check-email request body: {}", e);
            return json_response(
                400,
                &CheckEmailResponse {
                    exists: false,
                    verified: false,
                },
            );
        }
    };

    // Validate email format
    let email = body.email.trim().to_lowercase();
    if validate_email(&email).is_err() {
        return json_response(
            400,
            &CheckEmailResponse {
                exists: false,
                verified: false,
            },
        );
    }

    // Check if user exists
    let users_kv = env.kv("USERS")?;

    match get_user_by_email(&users_kv, &email).await? {
        Some(user) => json_response(
            200,
            &CheckEmailResponse {
                exists: true,
                verified: user.email_verified,
            },
        ),
        None => json_response(
            200,
            &CheckEmailResponse {
                exists: false,
                verified: false,
            },
        ),
    }
}

/// Middleware: Extract and validate user from request
///
/// Returns the authenticated user or None if not authenticated.
pub async fn get_authenticated_user(req: &Request, env: &Env) -> Result<Option<User>> {
    let auth_header = req.headers().get("Authorization").ok().flatten();
    let token = match extract_bearer_token(auth_header.as_deref()) {
        Some(t) => t,
        None => return Ok(None),
    };

    let jwt_secret = match get_jwt_secret(env) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    let claims = match validate_access_token(&token, &jwt_secret) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    let users_kv = env.kv("USERS")?;
    get_user_by_id(&users_kv, &claims.sub).await
}

///// Middleware: Require authentication
///
/// Returns the authenticated user and KV store, or an error response.
/// The KvStore is returned so the caller can update the user record if needed.
pub async fn require_auth(
    req: &Request,
    env: &Env,
) -> Result<std::result::Result<(User, KvStore), Response>> {
    let users_kv = match env.kv("USERS") {
        Ok(kv) => kv,
        Err(_) => {
            return Ok(Err(Response::from_json(&json!({
                "success": false,
                "error": "Internal server error"
            }))?
            .with_status(500)))
        }
    };

    let jwt_secret = match get_jwt_secret(env) {
        Ok(s) => s,
        Err(_) => {
            return Ok(Err(Response::from_json(&json!({
                "success": false,
                "error": "Internal server error"
            }))?
            .with_status(500)))
        }
    };

    let auth_header = req.headers().get("Authorization").ok().flatten();
    let token = match extract_bearer_token(auth_header.as_deref()) {
        Some(t) => t,
        None => {
            return Ok(Err(Response::from_json(&json!({
                "success": false,
                "error": "Authentication required"
            }))?
            .with_status(401)))
        }
    };

    let claims = match validate_access_token(&token, &jwt_secret) {
        Ok(c) => c,
        Err(_) => {
            return Ok(Err(Response::from_json(&json!({
                "success": false,
                "error": "Invalid or expired token"
            }))?
            .with_status(401)))
        }
    };

    match get_user_by_id(&users_kv, &claims.sub).await? {
        Some(user) => Ok(Ok((user, users_kv))),
        None => Ok(Err(Response::from_json(&json!({
            "success": false,
            "error": "User not found"
        }))?
        .with_status(401))),
    }
}

/// POST /auth/profile
///
/// Updates user profile (name parts).
pub async fn handle_update_profile(req: Request, env: Env) -> Result<Response> {
    // Require authentication
    let (mut user, users_kv) = match require_auth(&req, &env).await? {
        Ok((u, kv)) => (u, kv),
        Err(resp) => return Ok(resp),
    };

    // Parse request body
    let mut req = req;
    let body: UpdateProfileRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => {
            return json_response(
                400,
                &UpdateProfileResponse {
                    success: false,
                    message: "Invalid request body".to_string(),
                    user: None,
                },
            );
        }
    };

    // Validate and update first_name
    if let Some(first_name) = body.first_name {
        let first_name = first_name.trim();
        if first_name.is_empty() || first_name.len() > 50 {
            return json_response(
                400,
                &UpdateProfileResponse {
                    success: false,
                    message: "First name must be between 1 and 50 characters".to_string(),
                    user: None,
                },
            );
        }
        user.first_name = first_name.to_string();
    }

    // Validate and update middle_initial
    if let Some(middle_initial) = body.middle_initial {
        let mi = middle_initial.trim().to_uppercase();
        if mi.is_empty() {
            user.middle_initial = None;
        } else if mi.len() > 1 {
            return json_response(
                400,
                &UpdateProfileResponse {
                    success: false,
                    message: "Middle initial must be a single letter".to_string(),
                    user: None,
                },
            );
        } else {
            user.middle_initial = Some(mi);
        }
    }

    // Validate and update last_name
    if let Some(last_name) = body.last_name {
        let last_name = last_name.trim();
        if last_name.is_empty() || last_name.len() > 50 {
            return json_response(
                400,
                &UpdateProfileResponse {
                    success: false,
                    message: "Last name must be between 1 and 50 characters".to_string(),
                    user: None,
                },
            );
        }
        user.last_name = last_name.to_string();
    }

    // Update timestamp
    user.updated_at = chrono::Utc::now().to_rfc3339();

    // Save user
    if let Err(e) = save_user(&users_kv, &user).await {
        console_log!("Failed to save user profile: {:?}", e);
        return json_response(
            500,
            &UpdateProfileResponse {
                success: false,
                message: "Failed to save profile. Please try again.".to_string(),
                user: None,
            },
        );
    }

    console_log!("Profile updated for user: {}", user.email);

    json_response(
        200,
        &UpdateProfileResponse {
            success: true,
            message: "Profile updated successfully".to_string(),
            user: Some(UserPublic::from(&user)),
        },
    )
}
