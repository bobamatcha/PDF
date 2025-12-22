//! Email identity verification module
//!
//! Provides email masking and verification for recipient identity checking

use wasm_bindgen::prelude::*;

/// Internal implementation of email masking (non-wasm)
/// Shows first character of local part and full domain
fn mask_email_impl(email: &str) -> Result<String, String> {
    if email.is_empty() {
        return Err("Email cannot be empty".to_string());
    }

    // Find the @ symbol
    let at_pos = email
        .find('@')
        .ok_or_else(|| "Invalid email: missing @ symbol".to_string())?;

    if at_pos == 0 {
        return Err("Invalid email: empty local part".to_string());
    }

    let local = &email[..at_pos];
    let domain = &email[at_pos + 1..];

    if domain.is_empty() {
        return Err("Invalid email: empty domain".to_string());
    }

    // Get first character of local part
    let first_char = local.chars().next().unwrap();

    Ok(format!("{}***@{}", first_char, domain))
}

/// Internal implementation of email suffix verification (non-wasm)
fn verify_email_suffix_impl(email: &str, suffix: &str) -> bool {
    // Empty suffix always matches
    if suffix.is_empty() {
        return true;
    }

    // Convert both to lowercase for case-insensitive comparison
    let email_lower = email.to_lowercase();
    let suffix_lower = suffix.to_lowercase();

    // Check if email ends with the suffix
    if !email_lower.ends_with(&suffix_lower) {
        return false;
    }

    // If suffix contains @, it should match from the @ position only
    // This prevents matching partial local parts like "ohn@gmail.com" in "john@gmail.com"
    if suffix_lower.contains('@') {
        // Find @ position in the email
        if let Some(at_pos) = email_lower.find('@') {
            // The suffix should match from @ onwards
            return email_lower[at_pos..] == suffix_lower;
        }
        return false;
    }

    true
}

/// Mask an email address for display during verification
/// Shows first character of local part and full domain
/// Examples:
/// - "john@gmail.com" -> "j***@gmail.com"
/// - "ab@x.co" -> "a***@x.co"
/// - "a@b.c" -> "a***@b.c"
#[wasm_bindgen]
pub fn mask_email(email: &str) -> Result<String, JsValue> {
    mask_email_impl(email).map_err(|e| JsValue::from_str(&e))
}

/// Verify that the provided suffix matches the last N characters of the email
/// Case-insensitive comparison
/// Examples:
/// - verify_email_suffix("john@gmail.com", "l.com") -> true
/// - verify_email_suffix("john@gmail.com", ".com") -> true
/// - verify_email_suffix("john@gmail.com", "mail") -> false
/// - verify_email_suffix("test@example.org", "e.org") -> true
#[wasm_bindgen]
pub fn verify_email_suffix(email: &str, suffix: &str) -> bool {
    verify_email_suffix_impl(email, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Email Masking Tests
    // ============================================================

    #[test]
    fn test_mask_email_standard() {
        let masked = mask_email_impl("john@gmail.com").unwrap();
        assert_eq!(masked, "j***@gmail.com");
    }

    #[test]
    fn test_mask_email_short_local() {
        let masked = mask_email_impl("ab@x.co").unwrap();
        assert_eq!(masked, "a***@x.co");
    }

    #[test]
    fn test_mask_email_single_char_local() {
        let masked = mask_email_impl("a@b.c").unwrap();
        assert_eq!(masked, "a***@b.c");
    }

    #[test]
    fn test_mask_email_long_local() {
        let masked = mask_email_impl("verylongemailaddress@example.com").unwrap();
        assert_eq!(masked, "v***@example.com");
    }

    #[test]
    fn test_mask_email_subdomain() {
        let masked = mask_email_impl("user@mail.company.org").unwrap();
        assert_eq!(masked, "u***@mail.company.org");
    }

    #[test]
    fn test_mask_email_with_plus() {
        let masked = mask_email_impl("user+tag@gmail.com").unwrap();
        assert_eq!(masked, "u***@gmail.com");
    }

    #[test]
    fn test_mask_email_invalid_no_at() {
        let result = mask_email_impl("notanemail");
        assert!(result.is_err());
    }

    #[test]
    fn test_mask_email_invalid_no_domain() {
        let result = mask_email_impl("user@");
        assert!(result.is_err());
    }

    #[test]
    fn test_mask_email_invalid_empty() {
        let result = mask_email_impl("");
        assert!(result.is_err());
    }

    // ============================================================
    // Email Verification Tests
    // ============================================================

    #[test]
    fn test_verify_email_suffix_exact_match() {
        assert!(verify_email_suffix_impl("john@gmail.com", "l.com"));
    }

    #[test]
    fn test_verify_email_suffix_domain_only() {
        assert!(verify_email_suffix_impl("john@gmail.com", ".com"));
    }

    #[test]
    fn test_verify_email_suffix_full_domain() {
        assert!(verify_email_suffix_impl("john@gmail.com", "gmail.com"));
    }

    #[test]
    fn test_verify_email_suffix_case_insensitive() {
        assert!(verify_email_suffix_impl("john@gmail.com", "L.COM"));
        assert!(verify_email_suffix_impl("john@gmail.com", "GMAIL.COM"));
    }

    #[test]
    fn test_verify_email_suffix_incorrect() {
        assert!(!verify_email_suffix_impl("john@gmail.com", "mail"));
        assert!(!verify_email_suffix_impl("john@gmail.com", "yahoo.com"));
    }

    #[test]
    fn test_verify_email_suffix_partial_local() {
        // Should not match partial local part
        assert!(!verify_email_suffix_impl("john@gmail.com", "ohn@gmail.com"));
    }

    #[test]
    fn test_verify_email_suffix_subdomain() {
        assert!(verify_email_suffix_impl("user@mail.company.org", "y.org"));
        assert!(verify_email_suffix_impl(
            "user@mail.company.org",
            "company.org"
        ));
    }

    #[test]
    fn test_verify_email_suffix_empty_suffix() {
        // Empty suffix should always match (edge case)
        assert!(verify_email_suffix_impl("test@example.com", ""));
    }

    #[test]
    fn test_verify_email_suffix_longer_than_email() {
        // Suffix longer than email should not match
        assert!(!verify_email_suffix_impl("a@b.c", "verylongsuffix@b.c"));
    }

    #[test]
    fn test_verify_email_suffix_with_spaces() {
        // Whitespace in suffix should not match
        assert!(!verify_email_suffix_impl("test@example.com", " .com"));
        assert!(!verify_email_suffix_impl("test@example.com", ".com "));
    }
}
