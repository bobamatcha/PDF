//! California Residential Landlord-Tenant Law Compliance
//!
//! CA Civil Code 1940-1954, Tenant Protection Act (AB 1482)
//! Key requirements based on LEASE_RESEARCH.md:
//! - Security Deposit Cap (AB 12) - 1 month max (effective July 1, 2024)
//! - Junk Fees Ban (SB 611) - fees must be itemized (effective July 1, 2025)
//! - Just Cause Exemption (AB 1482)
//! - Void Clauses (Civil Code 1953) - waiver of jury/notice/habitability

use lazy_static::lazy_static;
use regex::Regex;
use shared_types::{Severity, TextPosition, Violation};

lazy_static! {
    /// Security deposit amount pattern
    static ref DEPOSIT_AMOUNT_PATTERN: Regex =
        Regex::new(r"(?i)(?:security\s+)?deposit[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    /// Monthly rent pattern
    static ref RENT_AMOUNT_PATTERN: Regex =
        Regex::new(r"(?i)(?:monthly\s+)?rent[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    /// Void clause patterns (Civil Code 1953)
    static ref VOID_CLAUSE_PATTERNS: Vec<(Regex, &'static str)> = vec![
        (
            Regex::new(r"(?i)\b(waive|waiver|waives).*?\b(jury\s+trial|right\s+to\s+jury)\b").unwrap(),
            "jury trial rights",
        ),
        (
            Regex::new(r"(?i)\b(waive|waiver|waives).*?\b(notice|notification)\b").unwrap(),
            "statutory notice requirements",
        ),
        (
            Regex::new(r"(?i)\b(waive|waiver|waives).*?\b(habitability|habitable)\b").unwrap(),
            "implied warranty of habitability",
        ),
        (
            Regex::new(r"(?i)\b(waive|waiver|waives).*?\b(repair|maintenance)\b").unwrap(),
            "landlord repair obligations",
        ),
        (
            Regex::new(r"(?i)\b(waive|waiver|waives).*?\b(rights?\s+under|statutory)\b").unwrap(),
            "statutory tenant rights",
        ),
    ];

    /// Junk fees pattern - non-itemized bundled fees
    static ref BUNDLED_FEES_PATTERN: Regex =
        Regex::new(r"(?i)(administrative\s+fee|processing\s+fee|move.in\s+fee|amenity\s+fee)").unwrap();

    /// AB 1482 Just Cause patterns
    static ref JUST_CAUSE_EXEMPT_PATTERN: Regex =
        Regex::new(r"(?i)(exempt\s+from\s+(?:AB\s*1482|just\s+cause)|single.family|owner.occupied)").unwrap();
}

/// Check all California-specific compliance requirements
pub fn check_california_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_security_deposit_cap(text));
    violations.extend(check_void_clauses(text));
    violations.extend(check_junk_fees(text));
    violations.extend(check_just_cause_disclosure(text));
    violations.extend(check_rent_increase_notice(text));

    violations
}

// ============================================================================
// AB 12 - Security Deposit Cap (Effective July 1, 2024)
// ============================================================================

/// Check security deposit cap requirements
///
/// Per CA Civil Code 1950.5 as amended by AB 12:
/// Security deposit cannot exceed 1 month's rent (effective July 1, 2024)
pub fn check_security_deposit_cap(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Extract deposit amount
    let deposit = DEPOSIT_AMOUNT_PATTERN
        .captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().replace(",", "").parse::<f64>().ok());

    // Extract rent amount
    let rent = RENT_AMOUNT_PATTERN
        .captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().replace(",", "").parse::<f64>().ok());

    if let (Some(deposit_amt), Some(rent_amt)) = (deposit, rent) {
        if deposit_amt > rent_amt {
            violations.push(Violation {
                statute: "CA Civil Code § 1950.5 (AB 12)".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit (${:.2}) exceeds 1 month's rent (${:.2}). \
                     AB 12 caps deposits at 1 month's rent effective July 1, 2024.",
                    deposit_amt, rent_amt
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

// ============================================================================
// Civil Code 1953 - Void Clauses
// ============================================================================

/// Check for void clauses under Civil Code 1953
///
/// Per CA Civil Code 1953:
/// Certain waivers are void and unenforceable
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    for (pattern, waived_right) in VOID_CLAUSE_PATTERNS.iter() {
        if let Some(m) = pattern.find(text) {
            let snippet = extract_context(text, m.start(), m.end());
            violations.push(Violation {
                statute: "CA Civil Code § 1953".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Void clause: Waiver of {} is unenforceable under California law.",
                    waived_right
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

// ============================================================================
// SB 611 - Junk Fees Ban (Effective July 1, 2025)
// ============================================================================

/// Check for non-itemized fees (junk fees)
///
/// Per SB 611 (effective July 1, 2025):
/// All mandatory fees must be itemized and disclosed
pub fn check_junk_fees(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for bundled/administrative fees
    if BUNDLED_FEES_PATTERN.is_match(text) {
        // Check if fees are itemized
        let has_itemization = text_lower.contains("itemized")
            || text_lower.contains("breakdown")
            || text_lower.contains("fee schedule")
            || (text_lower.contains("$") && text_lower.contains("each"));

        if !has_itemization {
            violations.push(Violation {
                statute: "CA Civil Code § 1946.2 (SB 611)".to_string(),
                severity: Severity::Warning,
                message: "Administrative/processing fees must be itemized. \
                         SB 611 requires all mandatory fees to be disclosed individually."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

// ============================================================================
// AB 1482 - Just Cause Eviction
// ============================================================================

/// Check Just Cause disclosure requirements
///
/// Per AB 1482 (Tenant Protection Act):
/// Leases must include Just Cause disclosure or exemption notice
pub fn check_just_cause_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions eviction/termination
    let mentions_eviction = text_lower.contains("evict")
        || text_lower.contains("termination")
        || text_lower.contains("terminate");

    if mentions_eviction {
        // Check for Just Cause disclosure or exemption
        let has_just_cause_disclosure = text_lower.contains("just cause")
            || text_lower.contains("ab 1482")
            || text_lower.contains("tenant protection act")
            || JUST_CAUSE_EXEMPT_PATTERN.is_match(text);

        if !has_just_cause_disclosure {
            violations.push(Violation {
                statute: "CA Civil Code § 1946.2 (AB 1482)".to_string(),
                severity: Severity::Warning,
                message: "Just Cause disclosure recommended. Include AB 1482 \
                         Just Cause notice or exemption statement if applicable."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

// ============================================================================
// Rent Increase Notice Requirements
// ============================================================================

/// Check rent increase notice requirements
///
/// Per CA Civil Code 827:
/// - 30 days notice for increases ≤ 10%
/// - 90 days notice for increases > 10%
pub fn check_rent_increase_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions rent increases
    if text_lower.contains("rent increase") || text_lower.contains("increase rent") {
        // Check for proper notice period disclosure
        let has_notice_period = text_lower.contains("30 days")
            || text_lower.contains("30-day")
            || text_lower.contains("90 days")
            || text_lower.contains("90-day")
            || text_lower.contains("notice period");

        if !has_notice_period {
            violations.push(Violation {
                statute: "CA Civil Code § 827".to_string(),
                severity: Severity::Info,
                message: "Consider specifying rent increase notice periods. \
                         California requires 30 days for ≤10% increases, 90 days for >10%."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
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

    // ========================================================================
    // AB 12 - Security Deposit Cap Tests
    // ========================================================================

    #[test]
    fn test_detects_excessive_deposit() {
        let text = "Monthly rent: $2,000. Security deposit: $4,000.";
        let violations = check_california_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("1950.5") && v.severity == Severity::Critical),
            "Should detect deposit exceeding 1 month rent"
        );
    }

    #[test]
    fn test_accepts_compliant_deposit() {
        let text = "Monthly rent: $2,000. Security deposit: $2,000.";
        let violations = check_california_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("1950.5") && v.severity == Severity::Critical),
            "Should accept deposit equal to 1 month rent"
        );
    }

    #[test]
    fn test_accepts_lower_deposit() {
        let text = "Monthly rent: $2,500. Security deposit: $1,500.";
        let violations = check_california_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("1950.5") && v.severity == Severity::Critical),
            "Should accept deposit less than 1 month rent"
        );
    }

    // ========================================================================
    // Civil Code 1953 - Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_jury_trial_waiver() {
        let text = "Tenant waives the right to jury trial in any dispute.";
        let violations = check_california_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("1953") && v.message.contains("jury")),
            "Should detect jury trial waiver"
        );
    }

    #[test]
    fn test_detects_notice_waiver() {
        let text = "Tenant hereby waives all statutory notice requirements.";
        let violations = check_california_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("1953") && v.message.contains("notice")),
            "Should detect notice waiver"
        );
    }

    #[test]
    fn test_detects_habitability_waiver() {
        let text = "Tenant accepts unit as-is and waives implied warranty of habitability.";
        let violations = check_california_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("1953") && v.message.contains("habitability")),
            "Should detect habitability waiver"
        );
    }

    #[test]
    fn test_detects_repair_waiver() {
        let text = "Tenant waives landlord's repair obligations for the term of lease.";
        let violations = check_california_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("1953") && v.message.contains("repair")),
            "Should detect repair obligation waiver"
        );
    }

    #[test]
    fn test_no_waiver_detection_for_compliant_text() {
        let text = "Landlord agrees to maintain the property in habitable condition. \
                    Tenant will receive proper notice for all inspections.";
        let violations = check_california_compliance(text);

        assert!(
            !violations.iter().any(|v| v.statute.contains("1953")),
            "Should not flag compliant language"
        );
    }

    // ========================================================================
    // SB 611 - Junk Fees Tests
    // ========================================================================

    #[test]
    fn test_detects_non_itemized_admin_fee() {
        let text = "An administrative fee will be charged at move-in.";
        let violations = check_california_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("SB 611") || v.statute.contains("1946.2")),
            "Should detect non-itemized administrative fee"
        );
    }

    #[test]
    fn test_accepts_itemized_fees() {
        let text = "Administrative fee: $50 (itemized: $25 credit check, $25 processing). \
                    Fee schedule attached.";
        let violations = check_california_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("SB 611") && v.severity == Severity::Critical),
            "Should accept itemized fees"
        );
    }

    // ========================================================================
    // AB 1482 - Just Cause Tests
    // ========================================================================

    #[test]
    fn test_warns_missing_just_cause_disclosure() {
        let text = "Landlord may terminate this lease for any violation. \
                    Eviction proceedings will follow standard process.";
        let violations = check_california_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("AB 1482") || v.message.contains("Just Cause")),
            "Should warn about missing Just Cause disclosure"
        );
    }

    #[test]
    fn test_accepts_just_cause_disclosure() {
        let text = "This property is subject to AB 1482 Just Cause eviction requirements. \
                    Termination will only occur for Just Cause as defined by law.";
        let violations = check_california_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("AB 1482") && v.severity == Severity::Critical),
            "Should accept Just Cause disclosure"
        );
    }

    #[test]
    fn test_accepts_just_cause_exemption() {
        let text = "This single-family home is exempt from AB 1482 Just Cause requirements. \
                    Owner-occupied exemption applies.";
        let violations = check_california_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("AB 1482") && v.severity == Severity::Warning),
            "Should accept Just Cause exemption notice"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_california_lease() {
        let text = "Monthly rent: $2,000. Security deposit: $2,000. \
                    This property is subject to AB 1482 Just Cause requirements. \
                    Fee schedule: Application $50, Credit check $25. \
                    Rent increases require 30 days written notice for increases up to 10%, \
                    or 90 days for increases over 10%. \
                    Landlord maintains property in habitable condition.";
        let violations = check_california_compliance(text);

        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(
            critical.is_empty(),
            "Compliant lease should have no critical violations"
        );
    }
}
