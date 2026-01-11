//! Document signing core logic
//!
//! This crate provides the core document signing functionality,
//! including PDF signing, audit trails, and session management.
//!
//! Most functionality is currently in the docsign-wasm crate and
//! will be migrated here incrementally.

// Re-export types from shared crates
pub use shared_crypto::{cert, cms, keys, tsa, EphemeralIdentity, SigningIdentity};
pub use shared_pdf::{dom_to_pdf, parser, pdf_to_dom, signer, PdfDocument};

// ============================================================
// Validation Module (Bug #1 - UX for Size Limits)
// ============================================================

pub mod validation {
    //! Input validation for document signing operations.
    //!
    //! These functions are designed to be called BEFORE expensive operations
    //! (like loading a PDF into memory) to provide clear, early feedback to users.

    /// Maximum PDF file size in bytes (100 MB)
    pub const MAX_PDF_SIZE_BYTES: u64 = 100 * 1024 * 1024;

    /// Maximum number of recipients for a signing ceremony
    pub const MAX_RECIPIENTS: usize = 10;

    /// Validation error types with user-friendly messages
    #[derive(Debug, Clone, PartialEq)]
    pub enum ValidationError {
        /// PDF file exceeds size limit
        PdfTooLarge { size_bytes: u64, max_bytes: u64 },
        /// Too many recipients
        TooManyRecipients { count: usize, max: usize },
        /// Field extends beyond page boundary
        FieldOutOfBounds {
            field_id: String,
            direction: String,
            overflow_points: f64,
        },
    }

    impl std::fmt::Display for ValidationError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ValidationError::PdfTooLarge {
                    size_bytes,
                    max_bytes,
                } => {
                    let size_mb = *size_bytes as f64 / (1024.0 * 1024.0);
                    let max_mb = *max_bytes as f64 / (1024.0 * 1024.0);
                    write!(
                        f,
                        "PDF is {:.1}MB but maximum allowed is {:.0}MB",
                        size_mb, max_mb
                    )
                }
                ValidationError::TooManyRecipients { count, max } => {
                    write!(
                        f,
                        "Cannot add {} recipients. Maximum allowed is {}",
                        count, max
                    )
                }
                ValidationError::FieldOutOfBounds {
                    field_id,
                    direction,
                    overflow_points,
                } => {
                    write!(
                        f,
                        "Field '{}' extends {:.0} points beyond the {} edge of the page",
                        field_id, overflow_points, direction
                    )
                }
            }
        }
    }

    impl std::error::Error for ValidationError {}

    /// Validate PDF file size before loading.
    ///
    /// Call this BEFORE loading the PDF into memory to prevent OOM and
    /// provide clear feedback to users with large files.
    ///
    /// # Arguments
    /// * `size_bytes` - The size of the PDF file in bytes
    ///
    /// # Returns
    /// * `Ok(())` if size is within limit
    /// * `Err(ValidationError::PdfTooLarge)` if size exceeds 100MB
    pub fn validate_pdf_size(size_bytes: u64) -> Result<(), ValidationError> {
        if size_bytes > MAX_PDF_SIZE_BYTES {
            Err(ValidationError::PdfTooLarge {
                size_bytes,
                max_bytes: MAX_PDF_SIZE_BYTES,
            })
        } else {
            Ok(())
        }
    }

    /// Validate recipient count before adding to ceremony.
    ///
    /// # Arguments
    /// * `count` - The total number of recipients (existing + new)
    ///
    /// # Returns
    /// * `Ok(())` if count is within limit
    /// * `Err(ValidationError::TooManyRecipients)` if count exceeds 10
    pub fn validate_recipient_count(count: usize) -> Result<(), ValidationError> {
        if count > MAX_RECIPIENTS {
            Err(ValidationError::TooManyRecipients {
                count,
                max: MAX_RECIPIENTS,
            })
        } else {
            Ok(())
        }
    }

    /// Validate that a field is within page bounds.
    ///
    /// # Arguments
    /// * `field_id` - Identifier for the field (for error messages)
    /// * `x` - X coordinate of field's bottom-left corner (PDF points)
    /// * `y` - Y coordinate of field's bottom-left corner (PDF points)
    /// * `width` - Field width (PDF points)
    /// * `height` - Field height (PDF points)
    /// * `page_width` - Page width (PDF points)
    /// * `page_height` - Page height (PDF points)
    ///
    /// # Returns
    /// * `Ok(())` if field is entirely within page bounds
    /// * `Err(ValidationError::FieldOutOfBounds)` if field extends past any edge
    pub fn validate_field_bounds(
        field_id: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        page_width: f64,
        page_height: f64,
    ) -> Result<(), ValidationError> {
        // Check left edge
        if x < 0.0 {
            return Err(ValidationError::FieldOutOfBounds {
                field_id: field_id.to_string(),
                direction: "left".to_string(),
                overflow_points: -x,
            });
        }

        // Check bottom edge
        if y < 0.0 {
            return Err(ValidationError::FieldOutOfBounds {
                field_id: field_id.to_string(),
                direction: "bottom".to_string(),
                overflow_points: -y,
            });
        }

        // Check right edge
        let right_overflow = (x + width) - page_width;
        if right_overflow > 0.0 {
            return Err(ValidationError::FieldOutOfBounds {
                field_id: field_id.to_string(),
                direction: "right".to_string(),
                overflow_points: right_overflow,
            });
        }

        // Check top edge
        let top_overflow = (y + height) - page_height;
        if top_overflow > 0.0 {
            return Err(ValidationError::FieldOutOfBounds {
                field_id: field_id.to_string(),
                direction: "top".to_string(),
                overflow_points: top_overflow,
            });
        }

        Ok(())
    }
}

// ============================================================
// Validation Tests (Bug #1)
// ============================================================

#[cfg(test)]
mod validation_tests {
    use super::validation::*;
    use proptest::prelude::*;

    // ============================================================
    // PDF Size Validation Tests
    // ============================================================

    proptest! {
        /// Property: PDFs under 100MB pass validation
        #[test]
        fn pdf_under_limit_passes(size_bytes in 0u64..MAX_PDF_SIZE_BYTES) {
            let result = validate_pdf_size(size_bytes);
            prop_assert!(result.is_ok(), "PDF under limit should pass: {:?}", result);
        }

        /// Property: PDFs over 100MB fail with clear error
        #[test]
        fn pdf_over_limit_fails(size_bytes in (MAX_PDF_SIZE_BYTES + 1)..500_000_000u64) {
            let result = validate_pdf_size(size_bytes);
            prop_assert!(result.is_err());

            match result.unwrap_err() {
                ValidationError::PdfTooLarge { size_bytes: s, max_bytes: m } => {
                    prop_assert_eq!(s, size_bytes);
                    prop_assert_eq!(m, MAX_PDF_SIZE_BYTES);
                }
                _ => prop_assert!(false, "Wrong error type"),
            }
        }
    }

    #[test]
    fn pdf_exactly_at_limit_passes() {
        let result = validate_pdf_size(MAX_PDF_SIZE_BYTES);
        assert!(result.is_ok(), "PDF exactly at limit should pass");
    }

    #[test]
    fn pdf_one_byte_over_limit_fails() {
        let result = validate_pdf_size(MAX_PDF_SIZE_BYTES + 1);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("100MB"),
            "Error should mention 100MB limit: {}",
            msg
        );
    }

    // ============================================================
    // Recipient Count Validation Tests
    // ============================================================

    proptest! {
        /// Property: 1-10 recipients pass validation
        #[test]
        fn valid_recipient_count_passes(count in 1usize..=MAX_RECIPIENTS) {
            let result = validate_recipient_count(count);
            prop_assert!(result.is_ok(), "Valid recipient count should pass: {:?}", result);
        }

        /// Property: >10 recipients fail with clear error
        #[test]
        fn too_many_recipients_fails(count in (MAX_RECIPIENTS + 1)..100usize) {
            let result = validate_recipient_count(count);
            prop_assert!(result.is_err());

            match result.unwrap_err() {
                ValidationError::TooManyRecipients { count: c, max: m } => {
                    prop_assert_eq!(c, count);
                    prop_assert_eq!(m, MAX_RECIPIENTS);
                }
                _ => prop_assert!(false, "Wrong error type"),
            }
        }
    }

    #[test]
    fn zero_recipients_passes() {
        // Zero recipients is valid (e.g., just uploaded PDF, no recipients yet)
        let result = validate_recipient_count(0);
        assert!(result.is_ok());
    }

    #[test]
    fn ten_recipients_passes() {
        let result = validate_recipient_count(10);
        assert!(result.is_ok(), "Exactly 10 recipients should pass");
    }

    #[test]
    fn eleven_recipients_fails() {
        let result = validate_recipient_count(11);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("10"),
            "Error should mention max of 10: {}",
            msg
        );
    }

    // ============================================================
    // Field Bounds Validation Tests
    // ============================================================

    const LETTER_WIDTH: f64 = 612.0;
    const LETTER_HEIGHT: f64 = 792.0;

    proptest! {
        /// Property: Fields fully within page pass validation
        #[test]
        fn field_within_bounds_passes(
            x in 0.0f64..500.0,
            y in 0.0f64..700.0,
            width in 10.0f64..100.0,
            height in 10.0f64..50.0,
        ) {
            // Ensure field fits within letter page
            prop_assume!(x + width <= LETTER_WIDTH);
            prop_assume!(y + height <= LETTER_HEIGHT);

            let result = validate_field_bounds("test-field", x, y, width, height, LETTER_WIDTH, LETTER_HEIGHT);
            prop_assert!(result.is_ok(), "Field within bounds should pass: {:?}", result);
        }

        /// Property: Fields extending past right edge fail
        #[test]
        fn field_past_right_edge_fails(
            x in 500.0f64..600.0,
            y in 0.0f64..700.0,
            width in 50.0f64..200.0,
            height in 10.0f64..50.0,
        ) {
            // Ensure field extends past right edge
            prop_assume!(x + width > LETTER_WIDTH);
            prop_assume!(y + height <= LETTER_HEIGHT);

            let result = validate_field_bounds("sig-1", x, y, width, height, LETTER_WIDTH, LETTER_HEIGHT);
            prop_assert!(result.is_err());

            match result.unwrap_err() {
                ValidationError::FieldOutOfBounds { direction, .. } => {
                    prop_assert_eq!(direction, "right");
                }
                _ => prop_assert!(false, "Wrong error type"),
            }
        }

        /// Property: Fields extending past top edge fail
        #[test]
        fn field_past_top_edge_fails(
            x in 0.0f64..500.0,
            y in 700.0f64..780.0,
            width in 10.0f64..100.0,
            height in 50.0f64..200.0,
        ) {
            // Ensure field extends past top edge
            prop_assume!(x + width <= LETTER_WIDTH);
            prop_assume!(y + height > LETTER_HEIGHT);

            let result = validate_field_bounds("sig-2", x, y, width, height, LETTER_WIDTH, LETTER_HEIGHT);
            prop_assert!(result.is_err());

            match result.unwrap_err() {
                ValidationError::FieldOutOfBounds { direction, .. } => {
                    prop_assert_eq!(direction, "top");
                }
                _ => prop_assert!(false, "Wrong error type"),
            }
        }
    }

    #[test]
    fn field_with_negative_x_fails() {
        let result = validate_field_bounds(
            "neg-x",
            -10.0,
            100.0,
            100.0,
            50.0,
            LETTER_WIDTH,
            LETTER_HEIGHT,
        );
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            ValidationError::FieldOutOfBounds {
                direction,
                overflow_points,
                ..
            } => {
                assert_eq!(direction, "left");
                assert!((overflow_points - 10.0).abs() < 0.001);
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn field_with_negative_y_fails() {
        let result = validate_field_bounds(
            "neg-y",
            100.0,
            -20.0,
            100.0,
            50.0,
            LETTER_WIDTH,
            LETTER_HEIGHT,
        );
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            ValidationError::FieldOutOfBounds {
                direction,
                overflow_points,
                ..
            } => {
                assert_eq!(direction, "bottom");
                assert!((overflow_points - 20.0).abs() < 0.001);
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn error_message_is_user_friendly() {
        // Test that error messages are clear for elderly users
        let pdf_err = ValidationError::PdfTooLarge {
            size_bytes: 150 * 1024 * 1024, // 150 MB
            max_bytes: MAX_PDF_SIZE_BYTES, // 100 MB
        };
        let msg = pdf_err.to_string();
        assert!(msg.contains("MB"), "Should use MB not bytes");
        assert!(msg.contains("100"), "Should mention 100MB limit: {}", msg);

        let recipient_err = ValidationError::TooManyRecipients { count: 15, max: 10 };
        let msg = recipient_err.to_string();
        assert!(msg.contains("15"), "Should show requested count");
        assert!(msg.contains("10"), "Should show maximum");

        let bounds_err = ValidationError::FieldOutOfBounds {
            field_id: "signature".to_string(),
            direction: "right".to_string(),
            overflow_points: 25.0,
        };
        let msg = bounds_err.to_string();
        assert!(msg.contains("right"), "Should mention direction");
        assert!(msg.contains("25"), "Should show overflow amount");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // ============================================================
    // Signature Capture Types and Validation
    // ============================================================

    /// Represents a signature captured from the user
    #[derive(Debug, Clone)]
    pub enum SignatureData {
        /// A drawn signature with raw image data (PNG/SVG bytes)
        Drawn {
            image_data: Vec<u8>,
            format: ImageFormat,
        },
        /// A typed signature with text
        Typed { text: String, font_family: String },
    }

    /// Supported image formats for drawn signatures
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ImageFormat {
        Png,
        Svg,
    }

    /// A signature field with position and dimensions
    #[derive(Debug, Clone)]
    pub struct SignatureField {
        pub id: String,
        pub page: u32,
        pub x: f64,
        pub y: f64,
        pub width: f64,
        pub height: f64,
        pub signature: Option<SignatureData>,
    }

    /// Page dimensions for bounds checking
    #[derive(Debug, Clone, Copy)]
    pub struct PageBounds {
        pub width: f64,
        pub height: f64,
    }

    impl PageBounds {
        pub fn letter() -> Self {
            Self {
                width: 612.0,
                height: 792.0,
            }
        }

        pub fn a4() -> Self {
            Self {
                width: 595.0,
                height: 842.0,
            }
        }
    }

    // ============================================================
    // Validation Functions
    // ============================================================

    /// Minimum signature dimensions (in PDF points)
    const MIN_SIGNATURE_WIDTH: f64 = 50.0;
    const MIN_SIGNATURE_HEIGHT: f64 = 20.0;

    /// Maximum signature dimensions (in PDF points)
    const MAX_SIGNATURE_WIDTH: f64 = 500.0;
    const MAX_SIGNATURE_HEIGHT: f64 = 200.0;

    /// Validate a typed signature
    pub fn validate_typed_signature(text: &str, font_family: &str) -> Result<(), &'static str> {
        if text.is_empty() {
            return Err("Typed signature text must not be empty");
        }
        if text.trim().is_empty() {
            return Err("Typed signature text must not be only whitespace");
        }
        if font_family.is_empty() {
            return Err("Font family must not be empty");
        }
        Ok(())
    }

    /// Validate a drawn signature
    pub fn validate_drawn_signature(
        image_data: &[u8],
        format: ImageFormat,
    ) -> Result<(), &'static str> {
        if image_data.is_empty() {
            return Err("Drawn signature image data must not be empty");
        }
        // Check for valid magic bytes based on format
        match format {
            ImageFormat::Png => {
                // PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A
                if image_data.len() < 8 {
                    return Err("PNG data too short");
                }
                let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
                if !image_data.starts_with(&png_magic) {
                    return Err("Invalid PNG magic bytes");
                }
            }
            ImageFormat::Svg => {
                // SVG should start with <?xml or <svg
                let data_str =
                    std::str::from_utf8(image_data).map_err(|_| "Invalid UTF-8 in SVG")?;
                let trimmed = data_str.trim();
                if !trimmed.starts_with("<?xml") && !trimmed.starts_with("<svg") {
                    return Err("Invalid SVG format");
                }
            }
        }
        Ok(())
    }

    /// Validate signature dimensions
    pub fn validate_signature_dimensions(width: f64, height: f64) -> Result<(), &'static str> {
        if width <= 0.0 || height <= 0.0 {
            return Err("Signature dimensions must be positive");
        }
        if width < MIN_SIGNATURE_WIDTH {
            return Err("Signature width too small");
        }
        if height < MIN_SIGNATURE_HEIGHT {
            return Err("Signature height too small");
        }
        if width > MAX_SIGNATURE_WIDTH {
            return Err("Signature width too large");
        }
        if height > MAX_SIGNATURE_HEIGHT {
            return Err("Signature height too large");
        }
        Ok(())
    }

    /// Validate signature bounds within page
    pub fn validate_signature_bounds(
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        page: PageBounds,
    ) -> Result<(), &'static str> {
        if x < 0.0 {
            return Err("Signature X coordinate must be non-negative");
        }
        if y < 0.0 {
            return Err("Signature Y coordinate must be non-negative");
        }
        if x + width > page.width {
            return Err("Signature extends beyond page right edge");
        }
        if y + height > page.height {
            return Err("Signature extends beyond page top edge");
        }
        Ok(())
    }

    /// Check if two signature fields overlap
    pub fn signatures_overlap(a: &SignatureField, b: &SignatureField) -> bool {
        // Different pages cannot overlap
        if a.page != b.page {
            return false;
        }

        // Check for rectangle intersection
        let a_right = a.x + a.width;
        let a_top = a.y + a.height;
        let b_right = b.x + b.width;
        let b_top = b.y + b.height;

        // No overlap if one is completely to the left, right, above, or below the other
        !(a_right <= b.x || b_right <= a.x || a_top <= b.y || b_top <= a.y)
    }

    /// Validate that no signatures in a list overlap
    pub fn validate_no_overlapping_signatures(
        signatures: &[SignatureField],
    ) -> Result<(), (&SignatureField, &SignatureField)> {
        for i in 0..signatures.len() {
            for j in (i + 1)..signatures.len() {
                if signatures_overlap(&signatures[i], &signatures[j]) {
                    return Err((&signatures[i], &signatures[j]));
                }
            }
        }
        Ok(())
    }

    // ============================================================
    // Proptest Strategies
    // ============================================================

    /// Strategy for valid signature dimensions
    fn valid_dimensions() -> impl Strategy<Value = (f64, f64)> {
        (
            MIN_SIGNATURE_WIDTH..=MAX_SIGNATURE_WIDTH,
            MIN_SIGNATURE_HEIGHT..=MAX_SIGNATURE_HEIGHT,
        )
    }

    /// Strategy for invalid (too small) signature dimensions
    fn too_small_dimensions() -> impl Strategy<Value = (f64, f64)> {
        prop_oneof![
            (
                0.1f64..MIN_SIGNATURE_WIDTH,
                MIN_SIGNATURE_HEIGHT..=MAX_SIGNATURE_HEIGHT
            ),
            (
                MIN_SIGNATURE_WIDTH..=MAX_SIGNATURE_WIDTH,
                0.1f64..MIN_SIGNATURE_HEIGHT
            ),
        ]
    }

    /// Strategy for valid page bounds
    fn valid_page_bounds() -> impl Strategy<Value = PageBounds> {
        prop_oneof![Just(PageBounds::letter()), Just(PageBounds::a4()),]
    }

    /// Strategy for non-empty strings (for typed signatures)
    fn non_empty_string() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9 ]{0,49}".prop_map(|s| s.to_string())
    }

    /// Strategy for valid PNG header (minimal valid PNG-like data)
    fn valid_png_data() -> impl Strategy<Value = Vec<u8>> {
        Just(vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG magic
            0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
            0x49, 0x48, 0x44, 0x52, // IHDR
        ])
    }

    /// Strategy for valid SVG data
    fn valid_svg_data() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            Just(b"<svg></svg>".to_vec()),
            Just(b"<?xml version=\"1.0\"?><svg></svg>".to_vec()),
        ]
    }

    /// Strategy for a signature field ID
    fn field_id() -> impl Strategy<Value = String> {
        "[a-f0-9]{8}-[a-f0-9]{4}".prop_map(|s| s.to_string())
    }

    // ============================================================
    // Signature Capture Property Tests
    // ============================================================

    proptest! {
        /// Property: Typed signatures with non-empty text are valid
        #[test]
        fn typed_signature_non_empty_text_is_valid(
            text in non_empty_string(),
            font in non_empty_string(),
        ) {
            let result = validate_typed_signature(&text, &font);
            prop_assert!(result.is_ok(), "Non-empty typed signature should be valid: {:?}", result);
        }

        /// Property: Typed signatures with empty text are invalid
        #[test]
        fn typed_signature_empty_text_is_invalid(
            font in non_empty_string(),
        ) {
            let result = validate_typed_signature("", &font);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Typed signature text must not be empty");
        }

        /// Property: Typed signatures with whitespace-only text are invalid
        #[test]
        fn typed_signature_whitespace_only_is_invalid(
            spaces in " {1,10}",
            font in non_empty_string(),
        ) {
            let result = validate_typed_signature(&spaces, &font);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Typed signature text must not be only whitespace");
        }

        /// Property: Drawn signatures with valid PNG data are valid
        #[test]
        fn drawn_signature_valid_png_is_valid(
            data in valid_png_data(),
        ) {
            let result = validate_drawn_signature(&data, ImageFormat::Png);
            prop_assert!(result.is_ok(), "Valid PNG should pass validation: {:?}", result);
        }

        /// Property: Drawn signatures with valid SVG data are valid
        #[test]
        fn drawn_signature_valid_svg_is_valid(
            data in valid_svg_data(),
        ) {
            let result = validate_drawn_signature(&data, ImageFormat::Svg);
            prop_assert!(result.is_ok(), "Valid SVG should pass validation: {:?}", result);
        }

        /// Property: Drawn signatures with empty data are invalid
        #[test]
        fn drawn_signature_empty_data_is_invalid(
            format in prop_oneof![Just(ImageFormat::Png), Just(ImageFormat::Svg)],
        ) {
            let result = validate_drawn_signature(&[], format);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Drawn signature image data must not be empty");
        }

        /// Property: Drawn signatures with invalid PNG magic bytes are rejected
        #[test]
        fn drawn_signature_invalid_png_magic_rejected(
            garbage in prop::collection::vec(any::<u8>(), 8..100),
        ) {
            // Ensure it doesn't accidentally have valid PNG magic
            let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
            prop_assume!(!garbage.starts_with(&png_magic));

            let result = validate_drawn_signature(&garbage, ImageFormat::Png);
            prop_assert!(result.is_err());
        }

        /// Property: Drawn signatures with invalid SVG format are rejected
        #[test]
        fn drawn_signature_invalid_svg_rejected(
            garbage in "[a-zA-Z0-9]{10,50}",
        ) {
            prop_assume!(!garbage.starts_with("<?xml") && !garbage.starts_with("<svg"));
            let result = validate_drawn_signature(garbage.as_bytes(), ImageFormat::Svg);
            prop_assert!(result.is_err());
        }
    }

    // ============================================================
    // Signature Bounds Checking Property Tests
    // ============================================================

    proptest! {
        /// Property: Signatures within page bounds are valid
        #[test]
        fn signature_within_bounds_is_valid(
            page in valid_page_bounds(),
            (width, height) in valid_dimensions(),
        ) {
            // Generate position that fits within page
            let max_x = (page.width - width).max(0.0);
            let max_y = (page.height - height).max(0.0);
            prop_assume!(max_x > 0.0 && max_y > 0.0);

            // Use a position in the valid range
            let x = max_x / 2.0;
            let y = max_y / 2.0;

            let result = validate_signature_bounds(x, y, width, height, page);
            prop_assert!(result.is_ok(), "Signature within bounds should be valid: {:?}", result);
        }

        /// Property: Signatures extending beyond right edge are invalid
        #[test]
        fn signature_beyond_right_edge_is_invalid(
            page in valid_page_bounds(),
            (width, height) in valid_dimensions(),
            overflow in 1.0f64..100.0,
        ) {
            let x = page.width - width + overflow; // Starts beyond where it would fit
            let y = 0.0;

            let result = validate_signature_bounds(x, y, width, height, page);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Signature extends beyond page right edge");
        }

        /// Property: Signatures extending beyond top edge are invalid
        #[test]
        fn signature_beyond_top_edge_is_invalid(
            page in valid_page_bounds(),
            (width, height) in valid_dimensions(),
            overflow in 1.0f64..100.0,
        ) {
            let x = 0.0;
            let y = page.height - height + overflow;

            let result = validate_signature_bounds(x, y, width, height, page);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Signature extends beyond page top edge");
        }

        /// Property: Signatures with negative X coordinate are invalid
        #[test]
        fn signature_negative_x_is_invalid(
            page in valid_page_bounds(),
            (width, height) in valid_dimensions(),
            neg_x in -100.0f64..-0.1,
        ) {
            let result = validate_signature_bounds(neg_x, 0.0, width, height, page);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Signature X coordinate must be non-negative");
        }

        /// Property: Signatures with negative Y coordinate are invalid
        #[test]
        fn signature_negative_y_is_invalid(
            page in valid_page_bounds(),
            (width, height) in valid_dimensions(),
            neg_y in -100.0f64..-0.1,
        ) {
            let result = validate_signature_bounds(0.0, neg_y, width, height, page);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Signature Y coordinate must be non-negative");
        }
    }

    // ============================================================
    // Signature Size Validation Property Tests
    // ============================================================

    proptest! {
        /// Property: Valid signature dimensions pass validation
        #[test]
        fn valid_dimensions_pass_validation(
            (width, height) in valid_dimensions(),
        ) {
            let result = validate_signature_dimensions(width, height);
            prop_assert!(result.is_ok(), "Valid dimensions should pass: {:?}", result);
        }

        /// Property: Too small dimensions fail validation
        #[test]
        fn too_small_dimensions_fail_validation(
            (width, height) in too_small_dimensions(),
        ) {
            let result = validate_signature_dimensions(width, height);
            prop_assert!(result.is_err(), "Too small dimensions should fail validation");
        }

        /// Property: Zero or negative width fails validation
        #[test]
        fn zero_or_negative_width_fails(
            width in -100.0f64..=0.0,
            height in MIN_SIGNATURE_HEIGHT..=MAX_SIGNATURE_HEIGHT,
        ) {
            let result = validate_signature_dimensions(width, height);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Signature dimensions must be positive");
        }

        /// Property: Zero or negative height fails validation
        #[test]
        fn zero_or_negative_height_fails(
            width in MIN_SIGNATURE_WIDTH..=MAX_SIGNATURE_WIDTH,
            height in -100.0f64..=0.0,
        ) {
            let result = validate_signature_dimensions(width, height);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Signature dimensions must be positive");
        }

        /// Property: Too large width fails validation
        #[test]
        fn too_large_width_fails(
            width in (MAX_SIGNATURE_WIDTH + 1.0)..1000.0,
            height in MIN_SIGNATURE_HEIGHT..=MAX_SIGNATURE_HEIGHT,
        ) {
            let result = validate_signature_dimensions(width, height);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Signature width too large");
        }

        /// Property: Too large height fails validation
        #[test]
        fn too_large_height_fails(
            width in MIN_SIGNATURE_WIDTH..=MAX_SIGNATURE_WIDTH,
            height in (MAX_SIGNATURE_HEIGHT + 1.0)..500.0,
        ) {
            let result = validate_signature_dimensions(width, height);
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err(), "Signature height too large");
        }
    }

    // ============================================================
    // Multiple Signature Placement Property Tests
    // ============================================================

    proptest! {
        /// Property: Non-overlapping signatures on same page pass validation
        #[test]
        fn non_overlapping_same_page_is_valid(
            id1 in field_id(),
            id2 in field_id(),
            page in 1u32..10,
        ) {
            prop_assume!(id1 != id2);

            // Place signatures side by side
            let sig1 = SignatureField {
                id: id1,
                page,
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
                signature: None,
            };

            let sig2 = SignatureField {
                id: id2,
                page,
                x: 120.0, // Non-overlapping (110 + gap)
                y: 10.0,
                width: 100.0,
                height: 50.0,
                signature: None,
            };

            prop_assert!(!signatures_overlap(&sig1, &sig2));
            prop_assert!(validate_no_overlapping_signatures(&[sig1, sig2]).is_ok());
        }

        /// Property: Overlapping signatures on same page fail validation
        #[test]
        fn overlapping_same_page_is_invalid(
            id1 in field_id(),
            id2 in field_id(),
            page in 1u32..10,
            overlap_amount in 1.0f64..50.0,
        ) {
            prop_assume!(id1 != id2);

            let sig1 = SignatureField {
                id: id1,
                page,
                x: 50.0,
                y: 50.0,
                width: 100.0,
                height: 50.0,
                signature: None,
            };

            // Overlapping signature
            let sig2 = SignatureField {
                id: id2,
                page,
                x: 50.0 + 100.0 - overlap_amount, // Overlaps by overlap_amount
                y: 50.0,
                width: 100.0,
                height: 50.0,
                signature: None,
            };

            prop_assert!(signatures_overlap(&sig1, &sig2));
            prop_assert!(validate_no_overlapping_signatures(&[sig1, sig2]).is_err());
        }

        /// Property: Signatures on different pages never overlap
        #[test]
        fn different_pages_never_overlap(
            id1 in field_id(),
            id2 in field_id(),
            page1 in 1u32..10,
            page2 in 1u32..10,
            x in 0.0f64..500.0,
            y in 0.0f64..700.0,
        ) {
            prop_assume!(id1 != id2);
            prop_assume!(page1 != page2);

            // Same position but different pages
            let sig1 = SignatureField {
                id: id1,
                page: page1,
                x,
                y,
                width: 100.0,
                height: 50.0,
                signature: None,
            };

            let sig2 = SignatureField {
                id: id2,
                page: page2,
                x,
                y,
                width: 100.0,
                height: 50.0,
                signature: None,
            };

            prop_assert!(!signatures_overlap(&sig1, &sig2));
            prop_assert!(validate_no_overlapping_signatures(&[sig1, sig2]).is_ok());
        }

        /// Property: Adjacent (touching) signatures do not overlap
        #[test]
        fn adjacent_signatures_do_not_overlap(
            id1 in field_id(),
            id2 in field_id(),
            page in 1u32..10,
        ) {
            prop_assume!(id1 != id2);

            let sig1 = SignatureField {
                id: id1,
                page,
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
                signature: None,
            };

            // Exactly touching (not overlapping)
            let sig2 = SignatureField {
                id: id2,
                page,
                x: 110.0, // Starts exactly where sig1 ends
                y: 10.0,
                width: 100.0,
                height: 50.0,
                signature: None,
            };

            prop_assert!(!signatures_overlap(&sig1, &sig2));
        }

        /// Property: Completely contained signature overlaps
        #[test]
        fn contained_signature_overlaps(
            id1 in field_id(),
            id2 in field_id(),
            page in 1u32..10,
        ) {
            prop_assume!(id1 != id2);

            // Large outer signature
            let sig1 = SignatureField {
                id: id1,
                page,
                x: 10.0,
                y: 10.0,
                width: 200.0,
                height: 100.0,
                signature: None,
            };

            // Small inner signature (completely contained)
            let sig2 = SignatureField {
                id: id2,
                page,
                x: 50.0,
                y: 30.0,
                width: 50.0,
                height: 30.0,
                signature: None,
            };

            prop_assert!(signatures_overlap(&sig1, &sig2));
            prop_assert!(validate_no_overlapping_signatures(&[sig1, sig2]).is_err());
        }

        /// Property: Empty signature list is always valid
        #[test]
        fn empty_signature_list_is_valid(_unused in Just(())) {
            let result = validate_no_overlapping_signatures(&[]);
            prop_assert!(result.is_ok());
        }

        /// Property: Single signature is always valid (no overlap possible)
        #[test]
        fn single_signature_is_valid(
            id in field_id(),
            page in 1u32..10,
            x in 0.0f64..500.0,
            y in 0.0f64..700.0,
            (width, height) in valid_dimensions(),
        ) {
            let sig = SignatureField {
                id,
                page,
                x,
                y,
                width,
                height,
                signature: None,
            };

            let sigs = [sig];
            let result = validate_no_overlapping_signatures(&sigs);
            prop_assert!(result.is_ok());
        }
    }

    // ============================================================
    // Signature Placement Retrieval Property Tests
    // ============================================================

    proptest! {
        /// Property: A signature placed at (x, y) with (width, height) should be retrievable at those coordinates
        #[test]
        fn signature_placement_is_retrievable(
            id in field_id(),
            page in 1u32..20,
            x in 0.0f64..500.0,
            y in 0.0f64..700.0,
            (width, height) in valid_dimensions(),
        ) {
            let field = SignatureField {
                id: id.clone(),
                page,
                x,
                y,
                width,
                height,
                signature: None,
            };

            // Verify all properties are stored correctly
            prop_assert_eq!(field.id, id);
            prop_assert_eq!(field.page, page);
            prop_assert!((field.x - x).abs() < f64::EPSILON);
            prop_assert!((field.y - y).abs() < f64::EPSILON);
            prop_assert!((field.width - width).abs() < f64::EPSILON);
            prop_assert!((field.height - height).abs() < f64::EPSILON);
        }

        /// Property: Signature data is preserved when set
        #[test]
        fn signature_data_is_preserved_typed(
            id in field_id(),
            text in non_empty_string(),
            font in non_empty_string(),
        ) {
            let mut field = SignatureField {
                id,
                page: 1,
                x: 100.0,
                y: 100.0,
                width: 150.0,
                height: 50.0,
                signature: None,
            };

            // Set typed signature
            field.signature = Some(SignatureData::Typed {
                text: text.clone(),
                font_family: font.clone(),
            });

            // Verify it's retrievable
            match &field.signature {
                Some(SignatureData::Typed { text: t, font_family: f }) => {
                    prop_assert_eq!(t, &text);
                    prop_assert_eq!(f, &font);
                }
                _ => prop_assert!(false, "Expected Typed signature data"),
            }
        }

        /// Property: Signature data is preserved when set (drawn)
        #[test]
        fn signature_data_is_preserved_drawn(
            id in field_id(),
            data in valid_png_data(),
        ) {
            let mut field = SignatureField {
                id,
                page: 1,
                x: 100.0,
                y: 100.0,
                width: 150.0,
                height: 50.0,
                signature: None,
            };

            // Set drawn signature
            field.signature = Some(SignatureData::Drawn {
                image_data: data.clone(),
                format: ImageFormat::Png,
            });

            // Verify it's retrievable
            match &field.signature {
                Some(SignatureData::Drawn { image_data, format }) => {
                    prop_assert_eq!(image_data, &data);
                    prop_assert_eq!(*format, ImageFormat::Png);
                }
                _ => prop_assert!(false, "Expected Drawn signature data"),
            }
        }
    }

    // ============================================================
    // Sign-Verify Roundtrip Property Tests
    // ============================================================

    proptest! {
        /// Property 1: Sign-verify roundtrip - Any document signed with a key must verify
        /// with the corresponding public key
        #[test]
        fn sign_verify_roundtrip(document in prop::collection::vec(any::<u8>(), 0..2048)) {
            let identity = EphemeralIdentity::generate();
            let signature = identity.sign(&document);

            prop_assert!(
                identity.verify(&document, &signature),
                "Signature should verify with the same identity"
            );
        }

        /// Property 2: Cross-key verification failure - Signature from key A should NOT
        /// verify with key B
        #[test]
        fn cross_key_verification_fails(document in prop::collection::vec(any::<u8>(), 1..1024)) {
            let identity_a = EphemeralIdentity::generate();
            let identity_b = EphemeralIdentity::generate();

            let signature_a = identity_a.sign(&document);

            // Key B should not be able to verify signature from Key A
            prop_assert!(
                !identity_b.verify(&document, &signature_a),
                "Signature from identity A should not verify with identity B"
            );
        }

        /// Property 3: Tampering detection - Modifying a signed document must cause
        /// verification to fail
        #[test]
        fn tampering_detection(
            document in prop::collection::vec(any::<u8>(), 1..512),
            tamper_index in any::<prop::sample::Index>(),
            tamper_byte in any::<u8>(),
        ) {
            let identity = EphemeralIdentity::generate();
            let signature = identity.sign(&document);

            // Create tampered document
            let mut tampered = document.clone();
            let idx = tamper_index.index(tampered.len());
            let original_byte = tampered[idx];

            // Only tamper if the new byte is different
            prop_assume!(tamper_byte != original_byte);
            tampered[idx] = tamper_byte;

            prop_assert!(
                !identity.verify(&tampered, &signature),
                "Tampered document should fail verification"
            );
        }

        /// Property 4: Signature format consistency - Signatures are always valid DER
        #[test]
        fn signature_format_valid(document in prop::collection::vec(any::<u8>(), 0..256)) {
            let identity = EphemeralIdentity::generate();
            let signature = identity.sign(&document);

            // ECDSA DER signature starts with 0x30 (SEQUENCE tag)
            prop_assert_eq!(signature[0], 0x30, "Signature should be DER-encoded SEQUENCE");

            // Length should be valid
            let len = signature[1] as usize;
            prop_assert!(
                signature.len() >= len + 2,
                "Signature length should be consistent with DER encoding"
            );
        }

        /// Property 5: Public key consistency - Export/import preserves verification capability
        #[test]
        fn key_export_import_preserves_verification(
            document in prop::collection::vec(any::<u8>(), 1..512)
        ) {
            let original = EphemeralIdentity::generate();
            let signature = original.sign(&document);

            // Export and re-import
            let exported = original.export_private_key();
            let restored = EphemeralIdentity::from_private_key(&exported)
                .expect("Should successfully import valid private key");

            // Restored key should verify signatures from original
            prop_assert!(
                restored.verify(&document, &signature),
                "Restored identity should verify original signature"
            );

            // Original should verify signatures from restored
            let new_signature = restored.sign(&document);
            prop_assert!(
                original.verify(&document, &new_signature),
                "Original identity should verify restored signature"
            );
        }

        /// Property 6: SHA-256 prehash signing works correctly
        #[test]
        fn prehash_sign_verify(document in prop::collection::vec(any::<u8>(), 0..1024)) {
            use sha2::{Digest, Sha256};

            let identity = EphemeralIdentity::generate();

            // Compute hash manually
            let mut hasher = Sha256::new();
            hasher.update(&document);
            let hash: [u8; 32] = hasher.finalize().into();

            // Sign the prehashed data
            let signature = identity.sign_prehashed(&hash);

            // Signature should be valid DER
            prop_assert_eq!(signature[0], 0x30, "Prehash signature should be DER-encoded");
            prop_assert!(signature.len() >= 68 && signature.len() <= 72,
                "ECDSA P-256 signature should be 68-72 bytes in DER format");
        }

        /// Property 7: Different documents produce different signatures
        #[test]
        fn different_documents_different_signatures(
            doc1 in prop::collection::vec(any::<u8>(), 1..256),
            doc2 in prop::collection::vec(any::<u8>(), 1..256),
        ) {
            prop_assume!(doc1 != doc2);

            let identity = EphemeralIdentity::generate();
            let sig1 = identity.sign(&doc1);
            let sig2 = identity.sign(&doc2);

            // Signatures should be different for different documents
            // (ECDSA uses random nonce, so even same doc gives different sigs,
            // but different docs MUST give different sigs)
            prop_assert_ne!(sig1, sig2, "Different documents should produce different signatures");
        }

        /// Property 8: Empty document can be signed and verified
        #[test]
        fn empty_document_sign_verify(_unused in Just(())) {
            let identity = EphemeralIdentity::generate();
            let empty: Vec<u8> = vec![];

            let signature = identity.sign(&empty);
            prop_assert!(
                identity.verify(&empty, &signature),
                "Empty document should sign and verify correctly"
            );
        }

        /// Property 9: Invalid signatures fail verification
        #[test]
        fn invalid_signature_fails(
            document in prop::collection::vec(any::<u8>(), 1..256),
            garbage_sig in prop::collection::vec(any::<u8>(), 0..100),
        ) {
            let identity = EphemeralIdentity::generate();

            prop_assert!(
                !identity.verify(&document, &garbage_sig),
                "Random garbage should not verify as valid signature"
            );
        }

        /// Property 10: Signature with wrong length fails
        #[test]
        fn truncated_signature_fails(
            document in prop::collection::vec(any::<u8>(), 1..256),
            truncate_by in 1usize..30,
        ) {
            let identity = EphemeralIdentity::generate();
            let signature = identity.sign(&document);

            // Truncate the signature
            let truncate_amount = truncate_by.min(signature.len().saturating_sub(1));
            if truncate_amount > 0 {
                let truncated = &signature[..signature.len() - truncate_amount];

                prop_assert!(
                    !identity.verify(&document, truncated),
                    "Truncated signature should fail verification"
                );
            }
        }
    }
}
