//! Ohio Residential Landlord-Tenant Law Compliance
//!
//! Ohio Revised Code Chapter 5321 (Landlords and Tenants)
//! Key requirements based on LEASE_RESEARCH.md:
//! - 30-day deposit return
//! - Specific deduction notice required
//! - No statutory deposit cap
//! - Written lease recommended for 3+ years

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

/// Check all Ohio-specific compliance requirements
pub fn check_ohio_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_security_deposit(text));
    violations.extend(check_landlord_obligations(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// Security Deposit Requirements
// ============================================================================

/// Check security deposit requirements
///
/// Per O.R.C. § 5321.16:
/// - No statutory cap, but reasonable amounts expected
/// - Must return within 30 days
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

    // Warn about high deposits (more than 2 months is unusual)
    if let (Some(deposit_amt), Some(rent_amt)) = (deposit, rent) {
        if deposit_amt > rent_amt * 2.0 {
            violations.push(Violation {
                statute: "O.R.C. § 5321.16".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Security deposit (${:.2}) is high (>2 months rent). While Ohio has no cap, \
                     excessive deposits may be challenged.",
                    deposit_amt
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for 30-day return requirement
    let text_lower = text.to_lowercase();
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_return_timeline = text_lower.contains("30 days")
            || text_lower.contains("thirty days")
            || text_lower.contains("30-day");

        if !has_return_timeline {
            violations.push(Violation {
                statute: "O.R.C. § 5321.16".to_string(),
                severity: Severity::Warning,
                message: "Must specify 30-day deposit return timeline. Ohio requires return \
                         within 30 days with itemized deduction statement."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }

        // Check for itemized deduction statement
        let has_itemized = text_lower.contains("itemized")
            || text_lower.contains("deduction statement")
            || text_lower.contains("list of deductions");

        if !has_itemized {
            violations.push(Violation {
                statute: "O.R.C. § 5321.16".to_string(),
                severity: Severity::Info,
                message: "Consider specifying itemized deduction requirement. Ohio requires \
                         landlords to provide itemized statement of any deductions."
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
// Landlord Obligations
// ============================================================================

/// Check landlord obligation disclosures
///
/// Per O.R.C. § 5321.04:
/// - Landlord must maintain premises
/// - Must comply with building codes
pub fn check_landlord_obligations(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease disclaims habitability
    if text_lower.contains("as-is") || text_lower.contains("as is") {
        violations.push(Violation {
            statute: "O.R.C. § 5321.04".to_string(),
            severity: Severity::Warning,
            message: "As-is clauses do not waive landlord's statutory duty to maintain premises \
                     under Ohio law."
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

/// Check for void clauses under Ohio law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of landlord's duty to maintain
    if text_lower.contains("waive")
        && (text_lower.contains("habitability")
            || text_lower.contains("repair")
            || text_lower.contains("landlord obligations"))
    {
        violations.push(Violation {
            statute: "O.R.C. § 5321.06".to_string(),
            severity: Severity::Critical,
            message: "Waiver of landlord's statutory duties is void under Ohio law.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for waiver of tenant remedies
    if text_lower.contains("waive") && text_lower.contains("tenant") && text_lower.contains("remed")
    {
        violations.push(Violation {
            statute: "O.R.C. § 5321.06".to_string(),
            severity: Severity::Critical,
            message: "Waiver of tenant remedies is void under Ohio law.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "O.R.C. § 2323.13".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses are prohibited in Ohio residential leases."
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
    // Security Deposit Tests
    // ========================================================================

    #[test]
    fn test_warns_high_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $5,000.";
        let violations = check_ohio_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("5321.16") && v.message.contains("high")),
            "Should warn about high deposit"
        );
    }

    #[test]
    fn test_accepts_reasonable_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $3,000.";
        let violations = check_ohio_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("5321.16") && v.message.contains("high")),
            "Should accept deposit of 2 months rent"
        );
    }

    #[test]
    fn test_warns_missing_30_day_return() {
        let text = "Security deposit: $1,500. Deposit will be returned after move-out.";
        let violations = check_ohio_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("5321.16") && v.message.contains("30-day")),
            "Should warn about missing 30-day return timeline"
        );
    }

    #[test]
    fn test_accepts_30_day_return() {
        let text = "Security deposit: $1,500 returned within 30 days with itemized statement.";
        let violations = check_ohio_compliance(text);

        assert!(
            !violations.iter().any(|v| v.statute.contains("5321.16")
                && v.message.contains("30-day")
                && v.severity == Severity::Warning),
            "Should accept 30-day return timeline"
        );
    }

    // ========================================================================
    // Landlord Obligations Tests
    // ========================================================================

    #[test]
    fn test_warns_as_is_clause() {
        let text = "Property rented as-is with no warranty.";
        let violations = check_ohio_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("5321.04") && v.message.contains("As-is")),
            "Should warn about as-is clause"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_habitability_waiver() {
        let text = "Tenant agrees to waive warranty of habitability.";
        let violations = check_ohio_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("5321.06") && v.severity == Severity::Critical),
            "Should detect habitability waiver"
        );
    }

    #[test]
    fn test_detects_tenant_remedy_waiver() {
        let text = "Tenant waives all remedies under Ohio landlord-tenant law.";
        let violations = check_ohio_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("5321.06") && v.message.contains("remedies")),
            "Should detect tenant remedy waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_ohio_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("2323.13") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_ohio_lease() {
        let text = "Monthly rent: $1,800. Security deposit: $1,800. \
                    Deposit returned within 30 days with itemized deduction statement. \
                    Landlord maintains property per O.R.C. § 5321.04.";
        let violations = check_ohio_compliance(text);

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
