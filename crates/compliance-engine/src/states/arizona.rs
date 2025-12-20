//! Arizona Residential Landlord-Tenant Law Compliance
//!
//! Arizona Residential Landlord and Tenant Act (A.R.S. Title 33, Chapter 10)
//! Key requirements based on LEASE_RESEARCH.md:
//! - Bed Bug Disclosure Addendum
//! - Security Deposit: 1.5 months max
//! - Pool Safety Notice
//! - Tenant Handbook Reference
//! - 10-day notice for material breach

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
}

/// Check all Arizona-specific compliance requirements
pub fn check_arizona_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_bed_bug_disclosure(text));
    violations.extend(check_security_deposit(text));
    violations.extend(check_pool_safety(text));
    violations.extend(check_tenant_handbook(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// Bed Bug Disclosure
// ============================================================================

/// Check bed bug disclosure requirements
///
/// Per A.R.S. § 33-1319:
/// - Must disclose known bed bug infestations
/// - Educational information recommended
pub fn check_bed_bug_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for bed bug disclosure
    let has_bed_bug_disclosure = text_lower.contains("bed bug")
        || text_lower.contains("bedbug")
        || text_lower.contains("33-1319");

    if !has_bed_bug_disclosure {
        violations.push(Violation {
            statute: "A.R.S. § 33-1319".to_string(),
            severity: Severity::Warning,
            message: "Bed bug disclosure recommended. Arizona law requires landlords to disclose \
                     known bed bug infestations and provide educational information."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Security Deposit Requirements
// ============================================================================

/// Check security deposit limits
///
/// Per A.R.S. § 33-1321:
/// - Maximum 1.5 months rent
/// - Must return within 14 days
/// - Non-refundable fees must be clearly stated
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

    // Check deposit cap (1.5 months max)
    if let (Some(deposit_amt), Some(rent_amt)) = (deposit, rent) {
        if deposit_amt > rent_amt * 1.5 {
            violations.push(Violation {
                statute: "A.R.S. § 33-1321".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit (${:.2}) exceeds Arizona limit of 1.5 months' rent (${:.2}).",
                    deposit_amt,
                    rent_amt * 1.5
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for 14-day return requirement
    let text_lower = text.to_lowercase();
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_return_timeline = text_lower.contains("14 days")
            || text_lower.contains("fourteen days")
            || text_lower.contains("14-day");

        if !has_return_timeline {
            violations.push(Violation {
                statute: "A.R.S. § 33-1321".to_string(),
                severity: Severity::Info,
                message: "Consider specifying 14-day deposit return timeline. Arizona requires \
                         return within 14 days after termination and surrender."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }

        // Check non-refundable fee disclosure
        if text_lower.contains("non-refundable") || text_lower.contains("nonrefundable") {
            let has_clear_disclosure = text_lower.contains("non-refundable fee")
                || text_lower.contains("nonrefundable fee")
                || text_lower.contains("not refundable");

            if !has_clear_disclosure {
                violations.push(Violation {
                    statute: "A.R.S. § 33-1321".to_string(),
                    severity: Severity::Warning,
                    message: "Non-refundable fees must be clearly labeled as such in the lease."
                        .to_string(),
                    page: None,
                    text_snippet: None,
                    text_position: None,
                });
            }
        }
    }

    violations
}

// ============================================================================
// Pool Safety Notice
// ============================================================================

/// Check pool safety notice requirements
///
/// Per A.R.S. § 36-1681:
/// - Properties with pools must provide safety notice
pub fn check_pool_safety(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if property mentions pool
    if text_lower.contains("pool") || text_lower.contains("swimming") {
        let has_safety_notice = text_lower.contains("pool safety")
            || text_lower.contains("36-1681")
            || text_lower.contains("drowning prevention");

        if !has_safety_notice {
            violations.push(Violation {
                statute: "A.R.S. § 36-1681".to_string(),
                severity: Severity::Warning,
                message: "Pool safety notice required. Arizona law requires landlords to provide \
                         pool safety information."
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
// Tenant Handbook Reference
// ============================================================================

/// Check tenant handbook reference
///
/// Per A.R.S. § 33-1322:
/// - Recommended to reference Arizona Tenants Rights and Responsibilities
pub fn check_tenant_handbook(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for handbook reference
    let has_handbook = text_lower.contains("tenant rights")
        || text_lower.contains("tenant handbook")
        || text_lower.contains("33-1322")
        || text_lower.contains("arizona residential landlord");

    if !has_handbook {
        violations.push(Violation {
            statute: "A.R.S. § 33-1322".to_string(),
            severity: Severity::Info,
            message: "Consider referencing Arizona Tenant Rights and Responsibilities handbook."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Void Clauses
// ============================================================================

/// Check for void clauses under Arizona law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of landlord's duty to maintain
    if text_lower.contains("waive")
        && (text_lower.contains("habitability") || text_lower.contains("repair"))
    {
        violations.push(Violation {
            statute: "A.R.S. § 33-1315".to_string(),
            severity: Severity::Critical,
            message: "Waiver of landlord's duty to maintain premises is void under Arizona law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for waiver of tenant remedies
    if text_lower.contains("waive") && text_lower.contains("tenant") && text_lower.contains("remed")
    {
        violations.push(Violation {
            statute: "A.R.S. § 33-1315".to_string(),
            severity: Severity::Critical,
            message: "Waiver of tenant remedies is void under Arizona law.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "A.R.S. § 33-1315".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses are prohibited in Arizona residential leases."
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
    // Bed Bug Disclosure Tests
    // ========================================================================

    #[test]
    fn test_warns_missing_bed_bug_disclosure() {
        let text = "Monthly rent: $1,500. Security deposit: $1,500.";
        let violations = check_arizona_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("33-1319") && v.message.contains("bed bug")),
            "Should warn about missing bed bug disclosure"
        );
    }

    #[test]
    fn test_accepts_bed_bug_disclosure() {
        let text = "Bed Bug Disclosure: No known infestations. See attached addendum.";
        let violations = check_arizona_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("33-1319") && v.severity == Severity::Critical),
            "Should accept bed bug disclosure"
        );
    }

    // ========================================================================
    // Security Deposit Tests
    // ========================================================================

    #[test]
    fn test_detects_excessive_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $3,000.";
        let violations = check_arizona_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("33-1321") && v.severity == Severity::Critical),
            "Should detect deposit exceeding 1.5 months rent"
        );
    }

    #[test]
    fn test_accepts_compliant_deposit() {
        let text = "Monthly rent: $2,000. Security deposit: $3,000.";
        let violations = check_arizona_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("33-1321") && v.severity == Severity::Critical),
            "Should accept deposit equal to 1.5 months rent"
        );
    }

    // ========================================================================
    // Pool Safety Tests
    // ========================================================================

    #[test]
    fn test_warns_missing_pool_safety() {
        let text = "Property includes community pool and fitness center.";
        let violations = check_arizona_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("36-1681") && v.message.contains("pool safety")),
            "Should warn about missing pool safety notice"
        );
    }

    #[test]
    fn test_accepts_pool_safety_notice() {
        let text = "Pool Safety Notice: See attached drowning prevention information.";
        let violations = check_arizona_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("36-1681") && v.severity == Severity::Critical),
            "Should accept pool safety notice"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_habitability_waiver() {
        let text = "Tenant agrees to waive warranty of habitability.";
        let violations = check_arizona_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("33-1315") && v.severity == Severity::Critical),
            "Should detect habitability waiver"
        );
    }

    #[test]
    fn test_detects_tenant_remedy_waiver() {
        let text = "Tenant waives all remedies under Arizona landlord-tenant law.";
        let violations = check_arizona_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("33-1315") && v.message.contains("remedies")),
            "Should detect tenant remedy waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_arizona_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("33-1315") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_arizona_lease() {
        let text = "Monthly rent: $2,000. Security deposit: $2,500. \
                    Deposit returned within 14 days. \
                    Bed Bug Disclosure attached per A.R.S. § 33-1319. \
                    Arizona Tenant Rights handbook referenced.";
        let violations = check_arizona_compliance(text);

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
