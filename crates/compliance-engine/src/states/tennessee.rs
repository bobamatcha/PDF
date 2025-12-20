//! Tennessee Residential Landlord-Tenant Law Compliance
//!
//! Uniform Residential Landlord and Tenant Act (URLTA)
//! T.C.A. Title 66, Chapter 28
//! Key requirements based on LEASE_RESEARCH.md:
//! - County population determines URLTA applicability (75,000+)
//! - No statutory deposit cap
//! - Security deposit return varies by county
//! - Landlord must provide written lease for terms > 1 year

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

// Major Tennessee counties with URLTA applicability (population 75,000+)
const URLTA_COUNTIES: &[&str] = &[
    "davidson",
    "shelby",
    "knox",
    "hamilton",
    "rutherford",
    "williamson",
    "sumner",
    "montgomery",
    "wilson",
    "blount",
    "sullivan",
    "washington",
    "maury",
    "sevier",
    "madison",
    // Major cities indicating URLTA counties
    "nashville",
    "memphis",
    "knoxville",
    "chattanooga",
    "murfreesboro",
    "franklin",
    "clarksville",
];

/// Check all Tennessee-specific compliance requirements
pub fn check_tennessee_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Determine if URLTA applies
    let urlta_applies = URLTA_COUNTIES
        .iter()
        .any(|county| text_lower.contains(county));

    if urlta_applies {
        violations.extend(check_urlta_requirements(text));
    } else {
        violations.extend(check_non_urlta_requirements(text));
    }

    violations.extend(check_security_deposit(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// URLTA Requirements (Counties with 75,000+ population)
// ============================================================================

/// Check URLTA-specific requirements
///
/// Per T.C.A. § 66-28-101 et seq.:
/// - Applies to counties with population 75,000+
/// - Landlord must maintain habitable premises
/// - 14-day notice for rent nonpayment
pub fn check_urlta_requirements(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for URLTA acknowledgment
    let has_urlta_reference = text_lower.contains("urlta")
        || text_lower.contains("uniform residential landlord")
        || text_lower.contains("66-28");

    if !has_urlta_reference {
        violations.push(Violation {
            statute: "T.C.A. § 66-28-102".to_string(),
            severity: Severity::Info,
            message: "Property appears to be in URLTA county. Consider referencing Tennessee \
                     Uniform Residential Landlord and Tenant Act."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for proper notice periods
    if text_lower.contains("nonpayment") || text_lower.contains("non-payment") {
        let has_14_day = text_lower.contains("14 days")
            || text_lower.contains("fourteen days")
            || text_lower.contains("14-day");

        if !has_14_day {
            violations.push(Violation {
                statute: "T.C.A. § 66-28-505".to_string(),
                severity: Severity::Warning,
                message: "URLTA requires 14-day notice for rent nonpayment in URLTA counties."
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
// Non-URLTA Requirements
// ============================================================================

/// Check requirements for non-URLTA counties
///
/// In counties with population < 75,000:
/// - Common law and general statutes apply
/// - Fewer tenant protections
pub fn check_non_urlta_requirements(_text: &str) -> Vec<Violation> {
    // Just informational - no specific requirements
    vec![Violation {
        statute: "T.C.A. § 66-28-102".to_string(),
        severity: Severity::Info,
        message: "Property may be in non-URLTA county. URLTA applies only to counties with \
                 population of 75,000 or more."
            .to_string(),
        page: None,
        text_snippet: None,
        text_position: None,
    }]
}

// ============================================================================
// Security Deposit Requirements
// ============================================================================

/// Check security deposit requirements
///
/// Per T.C.A. § 66-28-301:
/// - No statutory cap on deposit amount
/// - Must return within reasonable time (no specific period for non-URLTA)
/// - URLTA counties: landlord must provide itemized statement
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
                statute: "T.C.A. § 66-28-301".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Security deposit (${:.2}) is high (>2 months rent). While Tennessee has \
                     no statutory cap, excessive deposits may be challenged.",
                    deposit_amt
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for return timeline
    let text_lower = text.to_lowercase();
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_return_timeline = text_lower.contains("days")
            && (text_lower.contains("return") || text_lower.contains("refund"));

        if !has_return_timeline {
            violations.push(Violation {
                statute: "T.C.A. § 66-28-301".to_string(),
                severity: Severity::Info,
                message: "Consider specifying deposit return timeline. URLTA counties require \
                         reasonable return with itemized deductions."
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

/// Check for void clauses under Tennessee law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of landlord's duty to maintain (URLTA counties)
    if text_lower.contains("waive")
        && (text_lower.contains("habitability") || text_lower.contains("repair"))
    {
        violations.push(Violation {
            statute: "T.C.A. § 66-28-104".to_string(),
            severity: Severity::Critical,
            message: "Waiver of landlord's duty to maintain premises is void under Tennessee \
                     URLTA (in applicable counties)."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "T.C.A. § 66-28-104".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses are prohibited in Tennessee residential \
                     leases."
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
            statute: "T.C.A. § 66-28-104".to_string(),
            severity: Severity::Critical,
            message: "Waiver of tenant remedies is void under Tennessee URLTA.".to_string(),
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
    // URLTA Applicability Tests
    // ========================================================================

    #[test]
    fn test_recognizes_urlta_county_nashville() {
        let text = "Property located in Nashville, Davidson County, TN.";
        let violations = check_tennessee_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("66-28") && v.message.contains("URLTA")),
            "Should recognize Nashville as URLTA county"
        );
    }

    #[test]
    fn test_recognizes_urlta_county_memphis() {
        let text = "Property in Memphis, Shelby County.";
        let violations = check_tennessee_compliance(text);

        assert!(
            violations.iter().any(|v| v.message.contains("URLTA")),
            "Should recognize Memphis as URLTA county"
        );
    }

    #[test]
    fn test_warns_non_urlta_county() {
        let text = "Property in rural Johnson County, TN.";
        let violations = check_tennessee_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.message.contains("non-URLTA") || v.message.contains("75,000")),
            "Should warn about non-URLTA county"
        );
    }

    // ========================================================================
    // URLTA Notice Period Tests
    // ========================================================================

    #[test]
    fn test_warns_missing_14_day_notice() {
        let text = "Property in Nashville. For nonpayment of rent, 7 days notice required.";
        let violations = check_tennessee_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("66-28-505") && v.message.contains("14-day")),
            "Should warn about missing 14-day notice in URLTA county"
        );
    }

    // ========================================================================
    // Security Deposit Tests
    // ========================================================================

    #[test]
    fn test_warns_high_deposit() {
        let text = "Property in Nashville. Monthly rent: $1,500. Security deposit: $5,000.";
        let violations = check_tennessee_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("66-28-301") && v.message.contains("high")),
            "Should warn about high deposit"
        );
    }

    #[test]
    fn test_accepts_reasonable_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $3,000.";
        let violations = check_tennessee_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("66-28-301") && v.message.contains("high")),
            "Should accept deposit of 2 months rent"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_habitability_waiver() {
        let text = "Tenant agrees to waive warranty of habitability.";
        let violations = check_tennessee_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("66-28-104") && v.severity == Severity::Critical),
            "Should detect habitability waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_tennessee_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("66-28-104") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    #[test]
    fn test_detects_tenant_remedy_waiver() {
        let text = "Tenant waives all remedies under Tennessee law.";
        let violations = check_tennessee_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("66-28-104") && v.message.contains("remedies")),
            "Should detect tenant remedy waiver"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_tennessee_urlta_lease() {
        let text = "Property in Nashville, Davidson County. \
                    Monthly rent: $1,800. Security deposit: $1,800. \
                    Deposit returned within 30 days with itemized statement. \
                    Per URLTA, 14 days notice for nonpayment.";
        let violations = check_tennessee_compliance(text);

        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(
            critical.is_empty(),
            "Compliant URLTA lease should have no critical violations"
        );
    }
}
