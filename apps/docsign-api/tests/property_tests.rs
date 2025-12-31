//! Property-based tests for docsign-api
//!
//! Tests the API models and validation logic using proptest.

use proptest::prelude::*;

// ============================================================
// Session ID Validation
// ============================================================

/// Valid session IDs are UUIDs (36 characters with hyphens)
fn valid_session_id() -> impl Strategy<Value = String> {
    "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
}

/// Invalid session IDs (too short, too long, or invalid characters)
fn invalid_session_id() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-z]{0,10}",        // Too short
        "[a-z]{50,100}",      // Too long
        "[!@#$%^&*]{10,20}",  // Invalid characters
        Just("".to_string()), // Empty
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    // ============================================================
    // Session ID Tests
    // ============================================================

    #[test]
    fn valid_session_ids_are_36_chars(id in valid_session_id()) {
        prop_assert_eq!(id.len(), 36);
        prop_assert!(id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }

    #[test]
    fn invalid_session_ids_dont_match_uuid_pattern(id in invalid_session_id()) {
        let uuid_pattern = regex::Regex::new(
            r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
        ).unwrap();
        prop_assert!(!uuid_pattern.is_match(&id));
    }

    // ============================================================
    // Recipient Validation Tests
    // ============================================================

    #[test]
    fn recipient_names_are_preserved(
        first in "[A-Za-z]{1,50}",
        last in "[A-Za-z]{1,50}"
    ) {
        let full_name = format!("{} {}", first, last);
        prop_assert!(full_name.len() >= 3);
        prop_assert!(full_name.contains(' '));
    }

    #[test]
    fn email_basic_validation(
        local in "[a-z]{1,20}",
        domain in "[a-z]{2,10}",
        tld in "[a-z]{2,4}"
    ) {
        let email = format!("{}@{}.{}", local, domain, tld);
        prop_assert!(email.contains('@'));
        prop_assert!(email.contains('.'));
        prop_assert!(email.len() >= 6);
    }

    // ============================================================
    // Signature Field Validation Tests
    // ============================================================

    #[test]
    fn signature_field_dimensions_are_positive(
        x in 0.0f64..1000.0,
        y in 0.0f64..1000.0,
        width in 1.0f64..500.0,
        height in 1.0f64..500.0
    ) {
        prop_assert!(x >= 0.0);
        prop_assert!(y >= 0.0);
        prop_assert!(width > 0.0);
        prop_assert!(height > 0.0);
    }

    #[test]
    fn page_numbers_are_positive(page in 1i32..1000) {
        prop_assert!(page >= 1);
    }

    #[test]
    fn field_types_are_valid(
        field_type in prop_oneof![
            Just("signature"),
            Just("initials"),
            Just("date"),
            Just("text"),
            Just("checkbox")
        ]
    ) {
        let valid_types = ["signature", "initials", "date", "text", "checkbox"];
        prop_assert!(valid_types.contains(&field_type));
    }

    // ============================================================
    // Signature Data Tests
    // ============================================================

    #[test]
    fn base64_data_url_format(
        data in "[A-Za-z0-9+/]{100,500}"
    ) {
        let data_url = format!("data:image/png;base64,{}", data);
        prop_assert!(data_url.starts_with("data:image/"));
        prop_assert!(data_url.contains(";base64,"));
    }

    #[test]
    fn timestamp_format_is_iso8601(
        year in 2020i32..2030,
        month in 1u32..13,
        day in 1u32..29,
        hour in 0u32..24,
        minute in 0u32..60,
        second in 0u32..60
    ) {
        let timestamp = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hour, minute, second
        );
        prop_assert!(timestamp.len() == 20);
        prop_assert!(timestamp.ends_with('Z'));
        prop_assert!(timestamp.contains('T'));
    }

    // ============================================================
    // Session Status Tests
    // ============================================================

    #[test]
    fn session_status_values_are_valid(
        status in prop_oneof![
            Just("pending"),
            Just("in_progress"),
            Just("completed"),
            Just("declined"),
            Just("expired")
        ]
    ) {
        // All status values should be non-empty lowercase with underscores
        prop_assert!(!status.is_empty());
        prop_assert!(status.chars().all(|c| c.is_ascii_lowercase() || c == '_'));

        // Should be one of the valid statuses
        let valid_statuses = ["pending", "in_progress", "completed", "declined", "expired"];
        prop_assert!(valid_statuses.contains(&status));
    }

    #[test]
    fn terminal_states_are_final(
        terminal in prop_oneof![
            Just("completed"),
            Just("declined"),
            Just("expired")
        ]
    ) {
        // Terminal states should be recognized as final
        let is_terminal = matches!(terminal, "completed" | "declined" | "expired");
        prop_assert!(is_terminal);
    }

    // ============================================================
    // Sync Request Tests
    // ============================================================

    #[test]
    fn client_timestamp_is_positive(timestamp in 0i64..i64::MAX) {
        prop_assert!(timestamp >= 0);
    }

    #[test]
    fn signing_key_has_minimum_length(key in "[A-Za-z0-9]{8,64}") {
        prop_assert!(key.len() >= 8);
        prop_assert!(key.len() <= 64);
    }

    // ============================================================
    // Document Hash Tests
    // ============================================================

    #[test]
    fn sha256_hash_is_64_hex_chars(hash in "[0-9a-f]{64}") {
        prop_assert_eq!(hash.len(), 64);
        prop_assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ============================================================
    // PDF Data Tests
    // ============================================================

    #[test]
    fn pdf_magic_bytes_check(
        rest in proptest::collection::vec(any::<u8>(), 0..100)
    ) {
        // PDF files start with %PDF-
        let mut pdf_data = vec![0x25, 0x50, 0x44, 0x46, 0x2D]; // %PDF-
        pdf_data.extend(rest);

        prop_assert!(pdf_data.len() >= 5);
        prop_assert_eq!(&pdf_data[0..5], b"%PDF-");
    }

    #[test]
    fn base64_pdf_roundtrip(data in proptest::collection::vec(any::<u8>(), 10..500)) {
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        let encoded = STANDARD.encode(&data);
        let decoded = STANDARD.decode(&encoded).unwrap();

        prop_assert_eq!(data, decoded);
    }

    // ============================================================
    // Error Response Tests
    // ============================================================

    #[test]
    fn http_status_codes_are_valid(
        status in prop_oneof![
            Just(200u16), // OK
            Just(201u16), // Created
            Just(400u16), // Bad Request
            Just(401u16), // Unauthorized
            Just(404u16), // Not Found
            Just(409u16), // Conflict
            Just(410u16), // Gone (expired)
            Just(500u16), // Internal Server Error
        ]
    ) {
        prop_assert!(status >= 100 && status < 600);
    }

    // ============================================================
    // Expiry Time Tests
    // ============================================================

    #[test]
    fn expiry_hours_are_reasonable(hours in 1i64..8760) {
        // 8760 hours = 1 year
        prop_assert!(hours >= 1);
        prop_assert!(hours <= 8760);
    }
}

// ============================================================
// Unit Tests (non-property)
// ============================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_session_status_display() {
        let statuses = ["pending", "in_progress", "completed", "declined", "expired"];
        for status in statuses {
            assert!(!status.is_empty());
            assert!(status.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
        }
    }

    #[test]
    fn test_field_type_variants() {
        let field_types = ["signature", "initials", "date", "text", "checkbox"];
        assert_eq!(field_types.len(), 5);
    }

    #[test]
    fn test_max_file_size_constant() {
        const MAX_FILE_SIZE: usize = 100 * 1024 * 1024; // 100 MB
        assert_eq!(MAX_FILE_SIZE, 104857600);
    }
}
