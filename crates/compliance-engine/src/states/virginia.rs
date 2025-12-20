//! Virginia Residential Landlord-Tenant Law Compliance
//!
//! Virginia Residential Landlord and Tenant Act (VRLTA)
//! VA Code § 55.1-1200 et seq.
//! Key requirements based on LEASE_RESEARCH.md:
//! - HB 2430 Fee Transparency - all fees on Page 1
//! - Security Deposit: 2 months max
//! - Move-in/Move-out Inspection required
//! - Mold Disclosure
//! - Written lease required for 3+ month terms

use lazy_static::lazy_static;
use regex::Regex;
use shared_types::{Severity, Violation};

lazy_static! {
    /// Security deposit amount pattern
    static ref DEPOSIT_AMOUNT_PATTERN: Regex =
        Regex::new(r"(?i)(?:security\s+)?deposit[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    /// Monthly rent pattern
    static ref RENT_AMOUNT_PATTERN: Regex =
        Regex::new(r"(?i)(?:monthly\s+)?rent[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    /// Fee pattern for fee transparency check
    static ref FEE_PATTERN: Regex =
        Regex::new(r"(?i)(application\s+fee|admin\s+fee|pet\s+fee|parking\s+fee|amenity\s+fee)[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();
}

/// Check all Virginia-specific compliance requirements
pub fn check_virginia_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_fee_transparency(text));
    violations.extend(check_security_deposit(text));
    violations.extend(check_mold_disclosure(text));
    violations.extend(check_move_inspection(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// HB 2430 Fee Transparency
// ============================================================================

/// Check fee transparency requirements
///
/// Per VA Code § 55.1-1204:
/// - All mandatory fees must be clearly disclosed
/// - Fees should be itemized, not bundled
pub fn check_fee_transparency(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if there are fees mentioned
    if FEE_PATTERN.is_match(text) {
        // Check for itemization
        let has_itemization = text_lower.contains("itemized")
            || text_lower.contains("fee schedule")
            || text_lower.contains("breakdown");

        if !has_itemization {
            violations.push(Violation {
                statute: "VA Code § 55.1-1204".to_string(),
                severity: Severity::Warning,
                message: "Fees should be clearly itemized. Virginia HB 2430 requires fee \
                         transparency with all charges disclosed."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for non-refundable fee disclosure
    if text_lower.contains("non-refundable") || text_lower.contains("nonrefundable") {
        let has_clear_disclosure = text_lower.contains("clearly stated")
            || text_lower.contains("non-refundable fee")
            || text_lower.contains("not refundable");

        if !has_clear_disclosure {
            violations.push(Violation {
                statute: "VA Code § 55.1-1226".to_string(),
                severity: Severity::Info,
                message: "Non-refundable fees must be clearly identified as such.".to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

// ============================================================================
// Security Deposit Requirements
// ============================================================================

/// Check security deposit limits
///
/// Per VA Code § 55.1-1226:
/// - Maximum 2 months rent
/// - Must return within 45 days
/// - Itemized deduction statement required
pub fn check_security_deposit(text: &str) -> Vec<Violation> {
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

    // Check deposit cap (2 months max)
    if let (Some(deposit_amt), Some(rent_amt)) = (deposit, rent) {
        if deposit_amt > rent_amt * 2.0 {
            violations.push(Violation {
                statute: "VA Code § 55.1-1226".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit (${:.2}) exceeds Virginia limit of 2 months' rent (${:.2}).",
                    deposit_amt,
                    rent_amt * 2.0
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for 45-day return requirement
    let text_lower = text.to_lowercase();
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_return_timeline = text_lower.contains("45 days")
            || text_lower.contains("forty-five days")
            || text_lower.contains("45-day");

        if !has_return_timeline {
            violations.push(Violation {
                statute: "VA Code § 55.1-1226".to_string(),
                severity: Severity::Info,
                message: "Consider specifying 45-day deposit return timeline as required by \
                         Virginia law."
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
// Mold Disclosure
// ============================================================================

/// Check mold disclosure requirements
///
/// Per VA Code § 55.1-1215:
/// - Landlord must disclose known mold
pub fn check_mold_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for mold mention without proper disclosure
    if text_lower.contains("mold") || text_lower.contains("mildew") {
        let has_proper_disclosure = text_lower.contains("mold disclosure")
            || text_lower.contains("55.1-1215")
            || (text_lower.contains("mold") && text_lower.contains("disclosure"));

        if !has_proper_disclosure {
            violations.push(Violation {
                statute: "VA Code § 55.1-1215".to_string(),
                severity: Severity::Warning,
                message: "Mold disclosure should include specific information about known mold \
                         conditions as required by Virginia law."
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
/// Per VA Code § 55.1-1214:
/// - Landlord must offer move-in inspection
/// - Must provide written report
pub fn check_move_inspection(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions deposit but not inspection
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_inspection = text_lower.contains("move-in inspection")
            || text_lower.contains("move in inspection")
            || text_lower.contains("inspection report")
            || text_lower.contains("condition checklist")
            || text_lower.contains("walk-through");

        if !has_inspection {
            violations.push(Violation {
                statute: "VA Code § 55.1-1214".to_string(),
                severity: Severity::Info,
                message: "Consider including move-in inspection provision. Virginia requires \
                         landlords to offer move-in inspection within 5 days of occupancy."
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

/// Check for void clauses under Virginia law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of landlord's duty to maintain
    if text_lower.contains("waive")
        && (text_lower.contains("habitability") || text_lower.contains("repair"))
    {
        violations.push(Violation {
            statute: "VA Code § 55.1-1228".to_string(),
            severity: Severity::Critical,
            message: "Waiver of landlord's duty to maintain premises is void under Virginia law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for waiver of tenant rights
    if text_lower.contains("waive") && text_lower.contains("tenant rights") {
        violations.push(Violation {
            statute: "VA Code § 55.1-1208".to_string(),
            severity: Severity::Critical,
            message: "Waiver of tenant rights under VRLTA is void.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "VA Code § 55.1-1208".to_string(),
            severity: Severity::Critical,
            message:
                "Confession of judgment clauses are prohibited in Virginia residential leases."
                    .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Fee Transparency Tests
    // ========================================================================

    #[test]
    fn test_warns_non_itemized_fees() {
        let text = "Application fee: $50. Admin fee: $100. Pet fee: $200.";
        let violations = check_virginia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("55.1-1204") && v.message.contains("itemized")),
            "Should warn about non-itemized fees"
        );
    }

    #[test]
    fn test_accepts_itemized_fees() {
        let text = "Fee Schedule (itemized): Application fee $50, Credit check $25.";
        let violations = check_virginia_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("55.1-1204") && v.severity == Severity::Critical),
            "Should accept itemized fees"
        );
    }

    // ========================================================================
    // Security Deposit Tests
    // ========================================================================

    #[test]
    fn test_detects_excessive_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $5,000.";
        let violations = check_virginia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("55.1-1226") && v.severity == Severity::Critical),
            "Should detect deposit exceeding 2 months rent"
        );
    }

    #[test]
    fn test_accepts_two_month_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $3,000.";
        let violations = check_virginia_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("55.1-1226") && v.severity == Severity::Critical),
            "Should accept deposit equal to 2 months rent"
        );
    }

    // ========================================================================
    // Mold Disclosure Tests
    // ========================================================================

    #[test]
    fn test_warns_incomplete_mold_disclosure() {
        let text = "There may be mold in the basement. Tenant accepts risk.";
        let violations = check_virginia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("55.1-1215") && v.message.contains("Mold")),
            "Should warn about incomplete mold disclosure"
        );
    }

    #[test]
    fn test_accepts_proper_mold_disclosure() {
        let text = "Mold Disclosure: No known mold conditions exist. \
                    See attached mold disclosure form.";
        let violations = check_virginia_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("55.1-1215") && v.severity == Severity::Critical),
            "Should accept proper mold disclosure"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_habitability_waiver() {
        let text = "Tenant agrees to waive warranty of habitability.";
        let violations = check_virginia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("55.1-1228") && v.severity == Severity::Critical),
            "Should detect habitability waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_virginia_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("55.1-1208") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_virginia_lease() {
        let text = "Monthly rent: $2,000. Security deposit: $2,000. \
                    Fee Schedule (itemized): Application $50, Credit check $25. \
                    Deposit returned within 45 days. Move-in inspection offered. \
                    Landlord maintains property in habitable condition.";
        let violations = check_virginia_compliance(text);

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
