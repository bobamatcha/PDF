//! Georgia Residential Landlord-Tenant Law Compliance
//!
//! Georgia Code Title 44, Chapter 7
//! Key requirements based on LEASE_RESEARCH.md:
//! - Safe at Home Act (HB 404) - As-Is clauses void, duty of habitability
//! - 3-Day Notice Requirement for nonpayment
//! - Security Deposit - no statutory cap (but recommend 2 months max)
//! - Flooding Disclosure - required if 3+ floods in 5 years
//! - Move-in/Move-out Inspection requirements

use lazy_static::lazy_static;
use regex::Regex;
use shared_types::{Severity, TextPosition, Violation};

lazy_static! {
    /// As-is clause patterns
    static ref AS_IS_PATTERN: Regex =
        Regex::new(r"(?i)\b(as.is|as-is|as\s+is)\b").unwrap();

    /// Habitability waiver patterns
    static ref HABITABILITY_WAIVER_PATTERN: Regex =
        Regex::new(r"(?i)\b(waive|waiver|waives).*?\b(habitability|habitable|warranty)\b").unwrap();

    /// Security deposit amount pattern
    static ref DEPOSIT_AMOUNT_PATTERN: Regex =
        Regex::new(r"(?i)(?:security\s+)?deposit[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    /// Monthly rent pattern
    static ref RENT_AMOUNT_PATTERN: Regex =
        Regex::new(r"(?i)(?:monthly\s+)?rent[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    /// Nonpayment notice period pattern
    static ref NOTICE_PERIOD_PATTERN: Regex =
        Regex::new(r"(?i)(\d+)\s*(?:day|days)\s*(?:notice|to\s+(?:pay|cure|vacate))").unwrap();

    /// Flooding disclosure patterns
    static ref FLOODING_PATTERN: Regex =
        Regex::new(r"(?i)(flood|flooding|flooded|water\s+damage)").unwrap();
}

/// Check all Georgia-specific compliance requirements
pub fn check_georgia_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_habitability_requirement(text));
    violations.extend(check_notice_period(text));
    violations.extend(check_security_deposit(text));
    violations.extend(check_flooding_disclosure(text));
    violations.extend(check_move_inspection(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// HB 404 - Safe at Home Act (Habitability)
// ============================================================================

/// Check habitability requirements under Safe at Home Act
///
/// Per Georgia HB 404:
/// As-Is clauses are void; landlord has duty of habitability
pub fn check_habitability_requirement(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Check for as-is clauses
    if let Some(m) = AS_IS_PATTERN.find(text) {
        let snippet = extract_context(text, m.start(), m.end());
        violations.push(Violation {
            statute: "GA Code § 44-7-13 (HB 404)".to_string(),
            severity: Severity::Critical,
            message: "As-Is clauses are void under Georgia's Safe at Home Act. \
                     Landlord has non-waivable duty to maintain habitable premises."
                .to_string(),
            page: None,
            text_snippet: Some(snippet),
            text_position: Some(TextPosition {
                start_offset: m.start(),
                end_offset: m.end(),
            }),
        });
    }

    // Check for habitability waivers
    if HABITABILITY_WAIVER_PATTERN.is_match(text) {
        violations.push(Violation {
            statute: "GA Code § 44-7-13 (HB 404)".to_string(),
            severity: Severity::Critical,
            message: "Waiver of warranty of habitability is void under Georgia law.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Notice Period Requirements
// ============================================================================

/// Check nonpayment notice period
///
/// Per Georgia Code:
/// Landlord must provide at least 3 days notice before filing dispossessory
pub fn check_notice_period(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions nonpayment eviction
    if text_lower.contains("nonpayment") || text_lower.contains("non-payment") {
        // Check for notice period
        if let Some(caps) = NOTICE_PERIOD_PATTERN.captures(text) {
            if let Some(days_str) = caps.get(1) {
                if let Ok(days) = days_str.as_str().parse::<u32>() {
                    if days < 3 {
                        violations.push(Violation {
                            statute: "GA Code § 44-7-50".to_string(),
                            severity: Severity::Critical,
                            message: format!(
                                "Notice period ({} days) is insufficient. \
                                 Georgia requires minimum 3 days notice for nonpayment.",
                                days
                            ),
                            page: None,
                            text_snippet: None,
                            text_position: None,
                        });
                    }
                }
            }
        }
    }

    violations
}

// ============================================================================
// Security Deposit
// ============================================================================

/// Check security deposit requirements
///
/// Georgia has no statutory cap, but best practice is 2 months max
/// Per GA Code § 44-7-30-36: specific handling requirements
pub fn check_security_deposit(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

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

    // Warning for high deposits (more than 2 months)
    if let (Some(deposit_amt), Some(rent_amt)) = (deposit, rent) {
        if deposit_amt > rent_amt * 2.0 {
            violations.push(Violation {
                statute: "GA Code § 44-7-30".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Security deposit (${:.2}) exceeds 2 months' rent (${:.2}). \
                     While Georgia has no cap, excessive deposits may be challenged.",
                    deposit_amt,
                    rent_amt * 2.0
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for escrow account requirement (10+ units)
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");
    if mentions_deposit {
        let has_escrow_disclosure = text_lower.contains("escrow")
            || text_lower.contains("separate account")
            || text_lower.contains("trust account");

        if !has_escrow_disclosure {
            violations.push(Violation {
                statute: "GA Code § 44-7-31".to_string(),
                severity: Severity::Info,
                message: "Consider disclosing deposit location. Georgia requires deposits \
                         held in escrow for properties with 10+ units."
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
// Flooding Disclosure
// ============================================================================

/// Check flooding disclosure requirements
///
/// Georgia law requires disclosure if property has flooded 3+ times in 5 years
pub fn check_flooding_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions flooding
    if FLOODING_PATTERN.is_match(text) {
        // Check for proper disclosure language
        let has_proper_disclosure = text_lower.contains("flood disclosure")
            || text_lower.contains("flooding history")
            || text_lower.contains("flood zone")
            || (text_lower.contains("flood") && text_lower.contains("disclosure"));

        if !has_proper_disclosure {
            violations.push(Violation {
                statute: "GA Code § 44-7-20".to_string(),
                severity: Severity::Warning,
                message: "Flooding disclosure may be required. Georgia law requires disclosure \
                         if property has flooded 3 or more times in the past 5 years."
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
// Move-In/Move-Out Inspection
// ============================================================================

/// Check move-in/move-out inspection requirements
///
/// Per GA Code § 44-7-33:
/// Landlord should provide move-in inspection list
pub fn check_move_inspection(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions deposit but not inspection
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_inspection = text_lower.contains("move-in inspection")
            || text_lower.contains("move in inspection")
            || text_lower.contains("inspection list")
            || text_lower.contains("condition checklist")
            || text_lower.contains("walk-through");

        if !has_inspection {
            violations.push(Violation {
                statute: "GA Code § 44-7-33".to_string(),
                severity: Severity::Info,
                message: "Consider including move-in inspection provision. \
                         Georgia requires landlords to provide inspection list within 3 days of move-in."
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
// Void Clauses
// ============================================================================

/// Check for void clauses under Georgia law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for liability waiver for landlord's negligence
    if text_lower.contains("waive")
        && (text_lower.contains("negligence") || text_lower.contains("liability"))
        && text_lower.contains("landlord")
    {
        violations.push(Violation {
            statute: "GA Code § 44-7-2".to_string(),
            severity: Severity::Critical,
            message: "Waiver of landlord's liability for negligence may be void under Georgia law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "GA Code § 44-7-2".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses may be unenforceable in Georgia.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
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
    // Habitability Tests (HB 404)
    // ========================================================================

    #[test]
    fn test_detects_as_is_clause() {
        let text = "Tenant accepts the property as-is with no warranty.";
        let violations = check_georgia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("44-7-13") && v.severity == Severity::Critical),
            "Should detect as-is clause violation"
        );
    }

    #[test]
    fn test_detects_as_is_with_space() {
        let text = "Property is rented as is without any repairs.";
        let violations = check_georgia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("HB 404") && v.message.contains("As-Is")),
            "Should detect 'as is' with space"
        );
    }

    #[test]
    fn test_detects_habitability_waiver() {
        let text = "Tenant waives the warranty of habitability.";
        let violations = check_georgia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("44-7-13") && v.message.contains("habitability")),
            "Should detect habitability waiver"
        );
    }

    #[test]
    fn test_no_violation_for_normal_text() {
        let text = "Landlord will maintain the property in good condition.";
        let violations = check_georgia_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("44-7-13") && v.severity == Severity::Critical),
            "Should not flag normal maintenance language"
        );
    }

    // ========================================================================
    // Notice Period Tests
    // ========================================================================

    #[test]
    fn test_detects_insufficient_notice() {
        let text = "For nonpayment of rent, tenant has 1 day notice to pay or vacate.";
        let violations = check_georgia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("44-7-50") && v.message.contains("3 days")),
            "Should detect insufficient notice period"
        );
    }

    #[test]
    fn test_accepts_3_day_notice() {
        let text = "For nonpayment of rent, tenant has 3 days notice to cure.";
        let violations = check_georgia_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("44-7-50") && v.severity == Severity::Critical),
            "Should accept 3-day notice"
        );
    }

    // ========================================================================
    // Security Deposit Tests
    // ========================================================================

    #[test]
    fn test_warns_excessive_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $5,000.";
        let violations = check_georgia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("44-7-30") && v.severity == Severity::Warning),
            "Should warn about deposit exceeding 2 months"
        );
    }

    #[test]
    fn test_accepts_reasonable_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $3,000.";
        let violations = check_georgia_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("44-7-30") && v.severity == Severity::Warning),
            "Should accept deposit equal to 2 months rent"
        );
    }

    // ========================================================================
    // Flooding Disclosure Tests
    // ========================================================================

    #[test]
    fn test_warns_flood_without_disclosure() {
        let text = "The basement has flooded in the past. Tenant accepts risk.";
        let violations = check_georgia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("44-7-20") && v.message.contains("flood")),
            "Should warn about flooding disclosure"
        );
    }

    #[test]
    fn test_accepts_proper_flood_disclosure() {
        let text = "Flood Disclosure: Property is in flood zone X. \
                    Flooding history attached.";
        let violations = check_georgia_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("44-7-20") && v.severity == Severity::Critical),
            "Should accept proper flood disclosure"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_negligence_waiver() {
        let text = "Tenant waives any claims for landlord negligence.";
        let violations = check_georgia_compliance(text);

        assert!(
            violations.iter().any(|v| v.statute.contains("44-7-2")),
            "Should detect negligence waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_georgia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("44-7-2") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_georgia_lease() {
        let text = "Monthly rent: $1,800. Security deposit: $1,800 held in escrow account. \
                    Move-in inspection will be conducted within 3 days of occupancy. \
                    Landlord maintains property in habitable condition. \
                    For nonpayment, 3 days notice required before legal action.";
        let violations = check_georgia_compliance(text);

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
