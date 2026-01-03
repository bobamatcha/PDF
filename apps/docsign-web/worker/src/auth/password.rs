//! Password hashing using Argon2id
//!
//! This module provides secure password hashing following the same patterns
//! as florida-top-contractors and housecleaning projects.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Hash a password using Argon2id
///
/// # Arguments
/// * `password` - The plaintext password to hash
///
/// # Returns
/// * `Ok(String)` - The hashed password in PHC string format
/// * `Err(String)` - Error message if hashing fails
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("Failed to hash password: {}", e))
}

/// Verify a password against a hash
///
/// # Arguments
/// * `password` - The plaintext password to verify
/// * `hash` - The stored password hash in PHC string format
///
/// # Returns
/// * `true` if the password matches
/// * `false` if it doesn't match or if verification fails
pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed_hash) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok(),
        Err(_) => false,
    }
}

/// Validate password strength
///
/// Requirements:
/// - At least 8 characters
/// - At least one uppercase letter
/// - At least one lowercase letter
/// - At least one digit
///
/// # Returns
/// * `Ok(())` if password meets requirements
/// * `Err(String)` with explanation if it doesn't
pub fn validate_password_strength(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters long".to_string());
    }

    if !password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain at least one uppercase letter".to_string());
    }

    if !password.chars().any(|c| c.is_lowercase()) {
        return Err("Password must contain at least one lowercase letter".to_string());
    }

    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err("Password must contain at least one number".to_string());
    }

    Ok(())
}

/// Validate email format (basic validation)
pub fn validate_email(email: &str) -> Result<(), String> {
    let email = email.trim().to_lowercase();

    if email.len() < 5 {
        return Err("Email is too short".to_string());
    }

    if !email.contains('@') {
        return Err("Email must contain @".to_string());
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err("Invalid email format".to_string());
    }

    let (local, domain) = (parts[0], parts[1]);

    if local.is_empty() {
        return Err("Email local part cannot be empty".to_string());
    }

    if !domain.contains('.') {
        return Err("Email domain must contain a dot".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let password = "SecurePass123!";
        let hash = hash_password(password).expect("Hash should succeed");

        // Hash should be in PHC format
        assert!(hash.starts_with("$argon2"));

        // Verification should work
        assert!(verify_password(password, &hash));

        // Wrong password should fail
        assert!(!verify_password("WrongPassword", &hash));
    }

    #[test]
    fn test_password_strength_validation() {
        // Valid passwords
        assert!(validate_password_strength("SecurePass1").is_ok());
        assert!(validate_password_strength("MyP@ssw0rd").is_ok());

        // Too short
        assert!(validate_password_strength("Pass1").is_err());

        // No uppercase
        assert!(validate_password_strength("password123").is_err());

        // No lowercase
        assert!(validate_password_strength("PASSWORD123").is_err());

        // No digit
        assert!(validate_password_strength("SecurePassword").is_err());
    }

    #[test]
    fn test_email_validation() {
        // Valid emails
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("user.name@example.co.uk").is_ok());

        // Invalid emails
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user@domain").is_err());
    }

    #[test]
    fn test_different_passwords_different_hashes() {
        let hash1 = hash_password("Password1").unwrap();
        let hash2 = hash_password("Password1").unwrap();

        // Same password should produce different hashes (different salts)
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(verify_password("Password1", &hash1));
        assert!(verify_password("Password1", &hash2));
    }
}
