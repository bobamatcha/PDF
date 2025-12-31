//! Page range parsing for PDF split/merge operations
//!
//! This module provides Rust implementations of page range parsing that mirror
//! the TypeScript logic in page-operations.ts. Having Rust versions enables
//! property testing with proptest.

use std::collections::BTreeSet;

/// Parse a page ranges string into a sorted, deduplicated list of page numbers.
///
/// # Arguments
/// * `range_str` - Range string like "1-3, 5, 8-10"
/// * `total_pages` - Total pages in the document (for bounds checking)
///
/// # Returns
/// Sorted Vec of page numbers (1-indexed)
///
/// # Examples
/// ```
/// use agentpdf_wasm::page_ranges::parse_page_ranges;
///
/// assert_eq!(parse_page_ranges("1-3", 10), vec![1, 2, 3]);
/// assert_eq!(parse_page_ranges("1, 3, 5", 10), vec![1, 3, 5]);
/// assert_eq!(parse_page_ranges("1-3, 5, 8-10", 10), vec![1, 2, 3, 5, 8, 9, 10]);
/// ```
pub fn parse_page_ranges(range_str: &str, total_pages: usize) -> Vec<usize> {
    let mut pages = BTreeSet::new();

    for part in range_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if part.contains('-') {
            // Range like "1-3"
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() == 2 {
                if let (Ok(start), Ok(end)) = (
                    parts[0].trim().parse::<usize>(),
                    parts[1].trim().parse::<usize>(),
                ) {
                    let start = start.max(1);
                    let end = end.min(total_pages);
                    for i in start..=end {
                        pages.insert(i);
                    }
                }
            }
        } else {
            // Single page like "5"
            if let Ok(num) = part.parse::<usize>() {
                if num >= 1 && num <= total_pages {
                    pages.insert(num);
                }
            }
        }
    }

    pages.into_iter().collect()
}

/// Validate a page range string without parsing it
///
/// Returns None if valid, Some(error_message) if invalid
pub fn validate_page_range(range_str: &str, total_pages: usize) -> Option<String> {
    if range_str.trim().is_empty() {
        return Some("Page range cannot be empty".to_string());
    }

    for part in range_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if part.contains('-') {
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() != 2 {
                return Some(format!("Invalid range format: {}", part));
            }

            let start = parts[0].trim().parse::<usize>();
            let end = parts[1].trim().parse::<usize>();

            match (start, end) {
                (Ok(s), Ok(e)) => {
                    if s > e {
                        return Some(format!("Invalid range: start {} > end {}", s, e));
                    }
                    if s < 1 {
                        return Some(format!("Page number must be >= 1, got {}", s));
                    }
                    if e > total_pages {
                        return Some(format!("Page {} exceeds total pages {}", e, total_pages));
                    }
                }
                _ => return Some(format!("Invalid numbers in range: {}", part)),
            }
        } else {
            match part.parse::<usize>() {
                Ok(n) => {
                    if n < 1 {
                        return Some(format!("Page number must be >= 1, got {}", n));
                    }
                    if n > total_pages {
                        return Some(format!("Page {} exceeds total pages {}", n, total_pages));
                    }
                }
                Err(_) => return Some(format!("Invalid page number: {}", part)),
            }
        }
    }

    None
}

/// Field placement constraints for WCAG accessibility
pub mod field_constraints {
    /// Minimum width for interactive fields (WCAG touch target)
    pub const MIN_FIELD_WIDTH: f64 = 100.0;

    /// Minimum height for interactive fields (WCAG touch target)
    pub const MIN_FIELD_HEIGHT: f64 = 44.0;

    /// Minimum font size for text fields
    pub const MIN_FONT_SIZE: f64 = 11.0;

    /// Maximum font size for text fields
    pub const MAX_FONT_SIZE: f64 = 72.0;

    /// Default signature field dimensions
    pub const SIGNATURE_WIDTH: f64 = 200.0;
    pub const SIGNATURE_HEIGHT: f64 = 50.0;

    /// Default initials field dimensions
    pub const INITIALS_WIDTH: f64 = 60.0;
    pub const INITIALS_HEIGHT: f64 = 44.0;

    /// Checkbox dimensions
    pub const CHECKBOX_SIZE: f64 = 24.0;

    /// Validate field dimensions meet WCAG requirements
    pub fn validate_field_dimensions(width: f64, height: f64) -> Result<(), String> {
        if width < MIN_FIELD_WIDTH {
            return Err(format!(
                "Field width {} is below WCAG minimum of {}",
                width, MIN_FIELD_WIDTH
            ));
        }
        if height < MIN_FIELD_HEIGHT {
            return Err(format!(
                "Field height {} is below WCAG minimum of {}",
                height, MIN_FIELD_HEIGHT
            ));
        }
        Ok(())
    }

    /// Validate font size is within acceptable range
    pub fn validate_font_size(size: f64) -> Result<(), String> {
        if size < MIN_FONT_SIZE {
            return Err(format!(
                "Font size {} is below minimum of {}",
                size, MIN_FONT_SIZE
            ));
        }
        if size > MAX_FONT_SIZE {
            return Err(format!(
                "Font size {} exceeds maximum of {}",
                size, MAX_FONT_SIZE
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_page() {
        assert_eq!(parse_page_ranges("5", 10), vec![5]);
    }

    #[test]
    fn test_page_range() {
        assert_eq!(parse_page_ranges("2-4", 10), vec![2, 3, 4]);
    }

    #[test]
    fn test_multiple_pages() {
        assert_eq!(parse_page_ranges("1, 3, 5", 10), vec![1, 3, 5]);
    }

    #[test]
    fn test_mixed_ranges_and_pages() {
        assert_eq!(
            parse_page_ranges("1-3, 5, 8-10", 10),
            vec![1, 2, 3, 5, 8, 9, 10]
        );
    }

    #[test]
    fn test_out_of_bounds_clamped() {
        assert_eq!(
            parse_page_ranges("1-20", 10),
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        );
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(parse_page_ranges("", 10), Vec::<usize>::new());
    }

    #[test]
    fn test_invalid_input() {
        assert_eq!(parse_page_ranges("abc", 10), Vec::<usize>::new());
    }

    #[test]
    fn test_duplicates_removed() {
        assert_eq!(parse_page_ranges("1, 1, 2, 2", 10), vec![1, 2]);
    }

    #[test]
    fn test_zero_page_ignored() {
        assert_eq!(parse_page_ranges("0, 1, 2", 10), vec![1, 2]);
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(parse_page_ranges("  1 , 2 , 3  ", 10), vec![1, 2, 3]);
        assert_eq!(parse_page_ranges(" 1 - 3 ", 10), vec![1, 2, 3]);
    }

    #[test]
    fn test_validate_valid_range() {
        assert!(validate_page_range("1-5", 10).is_none());
        assert!(validate_page_range("1, 3, 5", 10).is_none());
    }

    #[test]
    fn test_validate_out_of_bounds() {
        assert!(validate_page_range("15", 10).is_some());
        assert!(validate_page_range("1-15", 10).is_some());
    }

    #[test]
    fn test_validate_invalid_range() {
        assert!(validate_page_range("5-3", 10).is_some()); // start > end
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: Result is always sorted
        #[test]
        fn result_is_sorted(range in "[0-9, -]+", total_pages in 1usize..100) {
            let result = parse_page_ranges(&range, total_pages);
            let mut sorted = result.clone();
            sorted.sort();
            prop_assert_eq!(result, sorted, "Result should be sorted");
        }

        /// Property: Result has no duplicates
        #[test]
        fn no_duplicates(range in "[0-9, -]+", total_pages in 1usize..100) {
            let result = parse_page_ranges(&range, total_pages);
            let unique: BTreeSet<_> = result.iter().collect();
            prop_assert_eq!(result.len(), unique.len(), "Should have no duplicates");
        }

        /// Property: All pages are within bounds
        #[test]
        fn all_pages_in_bounds(range in "[0-9, -]+", total_pages in 1usize..100) {
            let result = parse_page_ranges(&range, total_pages);
            for page in result {
                prop_assert!(page >= 1, "Page {} should be >= 1", page);
                prop_assert!(page <= total_pages, "Page {} should be <= {}", page, total_pages);
            }
        }

        /// Property: Empty input produces empty output
        #[test]
        fn empty_input_empty_output(total_pages in 1usize..100) {
            let result = parse_page_ranges("", total_pages);
            prop_assert!(result.is_empty(), "Empty input should produce empty output");
        }

        /// Property: Single valid page produces single result
        #[test]
        fn single_page_works(page in 1usize..=100, total_pages in 1usize..=100) {
            if page <= total_pages {
                let result = parse_page_ranges(&page.to_string(), total_pages);
                prop_assert_eq!(result, vec![page], "Single page {} should work", page);
            }
        }

        /// Property: Range 1-N with total N produces all pages
        #[test]
        fn full_range_produces_all(total_pages in 1usize..50) {
            let range = format!("1-{}", total_pages);
            let result = parse_page_ranges(&range, total_pages);
            let expected: Vec<usize> = (1..=total_pages).collect();
            prop_assert_eq!(result, expected, "Full range should produce all pages");
        }

        /// Property: Parsing is idempotent (formatting and reparsing gives same result)
        #[test]
        fn parsing_is_stable(range in "[0-9, -]+", total_pages in 1usize..100) {
            let result1 = parse_page_ranges(&range, total_pages);

            // Format result as a range string and reparse
            let formatted = result1.iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let result2 = parse_page_ranges(&formatted, total_pages);

            prop_assert_eq!(result1, result2, "Reparsing formatted result should be stable");
        }

        /// Property: Order of pages in input doesn't affect output
        #[test]
        fn order_independent(a in 1usize..=10, b in 1usize..=10, c in 1usize..=10) {
            let total_pages = 10;
            let range1 = format!("{}, {}, {}", a, b, c);
            let range2 = format!("{}, {}, {}", c, a, b);
            let range3 = format!("{}, {}, {}", b, c, a);

            let r1 = parse_page_ranges(&range1, total_pages);
            let r2 = parse_page_ranges(&range2, total_pages);
            let r3 = parse_page_ranges(&range3, total_pages);

            prop_assert_eq!(&r1, &r2, "Order should not matter");
            prop_assert_eq!(&r2, &r3, "Order should not matter");
        }
    }

    // Field constraint property tests
    proptest! {
        /// Property: Valid dimensions pass validation
        #[test]
        fn valid_dimensions_pass(
            width in 100.0f64..1000.0,
            height in 44.0f64..1000.0
        ) {
            let result = field_constraints::validate_field_dimensions(width, height);
            prop_assert!(result.is_ok(), "Valid dimensions should pass: {:?}", result);
        }

        /// Property: Invalid width fails validation
        #[test]
        fn invalid_width_fails(
            width in 0.0f64..100.0,
            height in 44.0f64..1000.0
        ) {
            let result = field_constraints::validate_field_dimensions(width, height);
            prop_assert!(result.is_err(), "Width {} should fail WCAG check", width);
        }

        /// Property: Invalid height fails validation
        #[test]
        fn invalid_height_fails(
            width in 100.0f64..1000.0,
            height in 0.0f64..44.0
        ) {
            let result = field_constraints::validate_field_dimensions(width, height);
            prop_assert!(result.is_err(), "Height {} should fail WCAG check", height);
        }

        /// Property: Valid font sizes pass
        #[test]
        fn valid_font_size_pass(size in 11.0f64..=72.0) {
            let result = field_constraints::validate_font_size(size);
            prop_assert!(result.is_ok(), "Font size {} should be valid", size);
        }

        /// Property: Font size below minimum fails
        #[test]
        fn small_font_fails(size in 0.0f64..11.0) {
            let result = field_constraints::validate_font_size(size);
            prop_assert!(result.is_err(), "Font size {} should fail", size);
        }

        /// Property: Font size above maximum fails
        #[test]
        fn large_font_fails(size in 72.1f64..200.0) {
            let result = field_constraints::validate_font_size(size);
            prop_assert!(result.is_err(), "Font size {} should fail", size);
        }
    }
}
