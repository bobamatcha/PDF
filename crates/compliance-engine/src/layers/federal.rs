//! Federal Layer - Baseline compliance requirements
//!
//! Applies to ALL residential leases nationwide:
//! - Lead-Based Paint Disclosure (pre-1978 properties)
//! - Fair Housing Act compliance (no discrimination)

use lazy_static::lazy_static;
use regex::Regex;
use shared_types::{Severity, TextPosition, Violation};

lazy_static! {
    /// Lead paint disclosure patterns
    static ref LEAD_PAINT_DISCLOSURE_PATTERN: Regex =
        Regex::new(r"(?i)(lead.based\s+paint|lead\s+paint|lead.hazard|EPA\s+pamphlet|protect\s+your\s+family)").unwrap();

    /// Year built extraction pattern
    static ref YEAR_BUILT_PATTERN: Regex =
        Regex::new(r"(?i)(?:built|constructed|year\s+built)(?:\s+in)?[:\s]+(\d{4})").unwrap();

    /// Fair Housing Act protected classes
    static ref FAIR_HOUSING_PATTERNS: Vec<(Regex, &'static str)> = vec![
        (
            Regex::new(r"(?i)\b(no\s+children|no\s+kids|adults\s+only|child.?free)\b").unwrap(),
            "familial status",
        ),
        (
            Regex::new(r"(?i)\b(christian\s+only|no\s+muslims|no\s+jews|religious\s+preference)\b")
                .unwrap(),
            "religion",
        ),
        (
            Regex::new(r"(?i)\b(whites?\s+only|no\s+blacks?|caucasian\s+only|no\s+hispanics?)\b")
                .unwrap(),
            "race/national origin",
        ),
        (
            Regex::new(r"(?i)\b(no\s+disabled|no\s+wheelchairs?|must\s+be\s+able.bodied)\b")
                .unwrap(),
            "disability",
        ),
        (
            Regex::new(r"(?i)\b(female\s+only|male\s+only|no\s+single\s+(?:men|women))\b").unwrap(),
            "sex",
        ),
    ];
}

/// Check all federal compliance requirements
pub fn check_federal_compliance(text: &str, year_built: Option<u32>) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_lead_paint_disclosure(text, year_built));
    violations.extend(check_fair_housing(text));

    violations
}

/// Check lead-based paint disclosure requirements
///
/// Per 42 U.S.C. § 4852d and 24 CFR Part 35:
/// - Required for housing built before 1978
/// - Must disclose known lead-based paint hazards
/// - Must provide EPA pamphlet "Protect Your Family From Lead in Your Home"
pub fn check_lead_paint_disclosure(text: &str, year_built: Option<u32>) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Try to extract year built from text if not provided
    let effective_year = year_built.or_else(|| {
        YEAR_BUILT_PATTERN
            .captures(text)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    });

    // Check if property is pre-1978
    let is_pre_1978 = match effective_year {
        Some(year) if year < 1978 => true,
        Some(_) => false,
        None => {
            // Can't determine year - check if any lead paint language exists
            // If mentioned but no disclosure, that's a problem
            text.to_lowercase().contains("1978") || text.to_lowercase().contains("lead")
        }
    };

    if is_pre_1978 {
        let has_disclosure = LEAD_PAINT_DISCLOSURE_PATTERN.is_match(text);
        let has_pamphlet_reference = text.to_lowercase().contains("pamphlet")
            || text.to_lowercase().contains("protect your family");

        if !has_disclosure {
            violations.push(Violation {
                statute: "42 U.S.C. § 4852d".to_string(),
                severity: Severity::Critical,
                message: "Lead-based paint disclosure required for pre-1978 housing. \
                         Must include disclosure of known hazards and provide EPA pamphlet."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        } else if !has_pamphlet_reference {
            violations.push(Violation {
                statute: "24 CFR 35.92".to_string(),
                severity: Severity::Warning,
                message: "Lead paint disclosure present but EPA pamphlet reference not found. \
                         Landlord must provide 'Protect Your Family From Lead in Your Home'."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

/// Check Fair Housing Act compliance
///
/// Per 42 U.S.C. § 3604 (Fair Housing Act):
/// Protected classes: race, color, religion, sex, familial status, national origin, disability
pub fn check_fair_housing(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    for (pattern, protected_class) in FAIR_HOUSING_PATTERNS.iter() {
        if let Some(m) = pattern.find(text) {
            let snippet = extract_context(text, m.start(), m.end());
            violations.push(Violation {
                statute: "42 U.S.C. § 3604".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Fair Housing Act violation: Discriminatory language based on {}. \
                     This clause is void and subjects landlord to civil liability.",
                    protected_class
                ),
                page: None,
                text_snippet: Some(snippet),
                text_position: Some(TextPosition {
                    start_offset: m.start(),
                    end_offset: m.end(),
                }),
            });
        }
    }

    violations
}

/// Extract context around a match for display
fn extract_context(text: &str, start: usize, end: usize) -> String {
    let context_size = 50;
    let ctx_start = start.saturating_sub(context_size);
    let ctx_end = (end + context_size).min(text.len());

    let mut result = String::new();
    if ctx_start > 0 {
        result.push_str("...");
    }
    result.push_str(&text[ctx_start..ctx_end]);
    if ctx_end < text.len() {
        result.push_str("...");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lead_paint_pre_1978_no_disclosure() {
        let text = "This property was built in 1965. Monthly rent is $1500.";
        let violations = check_lead_paint_disclosure(text, None);

        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.statute.contains("4852d")));
    }

    #[test]
    fn test_lead_paint_pre_1978_with_disclosure() {
        let text = "This property was built in 1965. \
                   Lead-Based Paint Disclosure: Landlord has no knowledge of lead-based paint hazards. \
                   Tenant has received the EPA pamphlet 'Protect Your Family From Lead in Your Home'.";
        let violations = check_lead_paint_disclosure(text, None);

        assert!(violations.is_empty());
    }

    #[test]
    fn test_lead_paint_post_1978() {
        let text = "This property was built in 1985.";
        let violations = check_lead_paint_disclosure(text, Some(1985));

        assert!(violations.is_empty());
    }

    #[test]
    fn test_fair_housing_familial_status() {
        let text = "This is an adults only community. No children allowed.";
        let violations = check_fair_housing(text);

        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.message.contains("familial")));
    }

    #[test]
    fn test_fair_housing_religion() {
        let text = "Christian only household preferred.";
        let violations = check_fair_housing(text);

        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.message.contains("religion")));
    }

    #[test]
    fn test_fair_housing_disability() {
        let text = "Tenant must be able-bodied and capable of using stairs.";
        let violations = check_fair_housing(text);

        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.message.contains("disability")));
    }

    #[test]
    fn test_fair_housing_compliant() {
        let text = "All applicants will be considered equally without regard to race, \
                   religion, sex, familial status, or disability.";
        let violations = check_fair_housing(text);

        assert!(violations.is_empty());
    }

    #[test]
    fn test_federal_combined() {
        let text = "Built 1960. No children. Adults only community.";
        let violations = check_federal_compliance(text, None);

        // Should have both lead paint and fair housing violations
        assert!(violations.len() >= 2);
    }
}
