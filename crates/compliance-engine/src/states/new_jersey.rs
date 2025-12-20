//! New Jersey Residential Landlord-Tenant Law Compliance
//!
//! NJ Anti-Eviction Act (N.J.S.A. 2A:18-61.1 et seq.)
//! Truth in Renting Act (N.J.S.A. 46:8-43 et seq.)
//! Key requirements based on LEASE_RESEARCH.md:
//! - Truth in Renting Statement required (DCA booklet)
//! - Security Deposit: 1.5 months max
//! - Window Guard Notice (buildings with children)
//! - Lead Paint Disclosure
//! - Rent Control (local municipalities)

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

    /// Late fee pattern
    static ref LATE_FEE_PATTERN: Regex =
        Regex::new(r"(?i)late\s+(?:fee|charge|penalty)[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();
}

// Rent control municipalities in New Jersey (partial list)
const RENT_CONTROL_MUNICIPALITIES: &[&str] = &[
    "newark",
    "jersey city",
    "hoboken",
    "elizabeth",
    "east orange",
    "fort lee",
    "weehawken",
    "west new york",
];

/// Check all New Jersey-specific compliance requirements
pub fn check_new_jersey_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_truth_in_renting(text));
    violations.extend(check_security_deposit(text));
    violations.extend(check_window_guard_notice(text));
    violations.extend(check_rent_control(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// Truth in Renting Act
// ============================================================================

/// Check Truth in Renting Statement requirement
///
/// Per N.J.S.A. 46:8-45:
/// - Landlord must provide DCA Truth in Renting booklet
/// - Statement must be in lease or attached
pub fn check_truth_in_renting(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for Truth in Renting disclosure
    let has_truth_in_renting = text_lower.contains("truth in renting")
        || text_lower.contains("46:8-45")
        || text_lower.contains("dca")
        || text_lower.contains("tenant rights statement")
        || (text_lower.contains("statement") && text_lower.contains("attached"));

    if !has_truth_in_renting {
        violations.push(Violation {
            statute: "N.J.S.A. 46:8-45".to_string(),
            severity: Severity::Warning,
            message: "Truth in Renting Statement required. New Jersey law requires landlords to \
                     provide the DCA Truth in Renting booklet to all tenants."
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
/// Per N.J.S.A. 46:8-21.2:
/// - Maximum 1.5 months rent
/// - Must be held in interest-bearing account
/// - Return within 30 days (or 5 days if fire/flood)
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
                statute: "N.J.S.A. 46:8-21.2".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit (${:.2}) exceeds New Jersey limit of 1.5 months' rent (${:.2}).",
                    deposit_amt,
                    rent_amt * 1.5
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for interest-bearing account disclosure
    let text_lower = text.to_lowercase();
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_interest_account = text_lower.contains("interest")
            || text_lower.contains("bank account")
            || text_lower.contains("escrow");

        if !has_interest_account {
            violations.push(Violation {
                statute: "N.J.S.A. 46:8-19".to_string(),
                severity: Severity::Warning,
                message: "Security deposit must be held in interest-bearing account. \
                         New Jersey requires landlords to hold deposits in designated banks."
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
// Window Guard Notice
// ============================================================================

/// Check window guard notice requirements
///
/// Per N.J.A.C. 5:10-27.1:
/// - Buildings with children under 10 must have window guards
/// - Notice must be provided to tenants
pub fn check_window_guard_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions children or window guards
    if text_lower.contains("child") || text_lower.contains("minor") {
        let has_window_guard_notice = text_lower.contains("window guard")
            || text_lower.contains("window safety")
            || text_lower.contains("5:10-27");

        if !has_window_guard_notice {
            violations.push(Violation {
                statute: "N.J.A.C. 5:10-27.1".to_string(),
                severity: Severity::Info,
                message: "Consider adding window guard notice. New Jersey requires window guards \
                         in units with children under 10."
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
// Rent Control Notice
// ============================================================================

/// Check rent control notice for applicable municipalities
pub fn check_rent_control(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if property is in rent control municipality
    let in_rent_control_area = RENT_CONTROL_MUNICIPALITIES
        .iter()
        .any(|city| text_lower.contains(city));

    if in_rent_control_area {
        // Check for rent control disclosure
        let has_rent_control_notice = text_lower.contains("rent control")
            || text_lower.contains("rent leveling")
            || text_lower.contains("maximum rent");

        if !has_rent_control_notice {
            violations.push(Violation {
                statute: "N.J.S.A. 2A:42-84.1".to_string(),
                severity: Severity::Warning,
                message: "Property may be subject to local rent control. Consider adding rent \
                         control disclosure for properties in regulated municipalities."
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

/// Check for void clauses under New Jersey law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of habitability
    if text_lower.contains("waive")
        && (text_lower.contains("habitability") || text_lower.contains("repair"))
    {
        violations.push(Violation {
            statute: "N.J.S.A. 46:8-48".to_string(),
            severity: Severity::Critical,
            message: "Waiver of implied warranty of habitability is void under New Jersey law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for waiver of statutory rights
    if text_lower.contains("waive")
        && (text_lower.contains("anti-eviction") || text_lower.contains("tenant rights"))
    {
        violations.push(Violation {
            statute: "N.J.S.A. 2A:18-61.1".to_string(),
            severity: Severity::Critical,
            message: "Waiver of Anti-Eviction Act protections is void under New Jersey law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "N.J.S.A. 2A:16-9.3".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses are prohibited in New Jersey residential \
                     leases."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for acceleration clause
    if text_lower.contains("accelerat") && text_lower.contains("rent") {
        violations.push(Violation {
            statute: "N.J.S.A. 2A:42-6.1".to_string(),
            severity: Severity::Warning,
            message: "Rent acceleration clauses may be unenforceable under New Jersey law."
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
    // Truth in Renting Tests
    // ========================================================================

    #[test]
    fn test_warns_missing_truth_in_renting() {
        let text = "Property in Newark, NJ. Monthly rent: $1,500.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("46:8-45") && v.message.contains("Truth in Renting")),
            "Should warn about missing Truth in Renting statement"
        );
    }

    #[test]
    fn test_accepts_truth_in_renting() {
        let text = "Truth in Renting Statement attached per DCA requirements.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("46:8-45") && v.severity == Severity::Critical),
            "Should accept Truth in Renting disclosure"
        );
    }

    // ========================================================================
    // Security Deposit Tests
    // ========================================================================

    #[test]
    fn test_detects_excessive_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $3,000.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("46:8-21.2") && v.severity == Severity::Critical),
            "Should detect deposit exceeding 1.5 months rent"
        );
    }

    #[test]
    fn test_accepts_compliant_deposit() {
        let text = "Monthly rent: $2,000. Security deposit: $3,000.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("46:8-21.2") && v.severity == Severity::Critical),
            "Should accept deposit equal to 1.5 months rent"
        );
    }

    #[test]
    fn test_warns_missing_interest_account() {
        let text = "Security deposit: $2,000 will be held by landlord.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("46:8-19") && v.message.contains("interest")),
            "Should warn about missing interest-bearing account"
        );
    }

    // ========================================================================
    // Rent Control Tests
    // ========================================================================

    #[test]
    fn test_warns_rent_control_jersey_city() {
        let text = "Property located in Jersey City, NJ. Monthly rent: $2,500.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.message.contains("rent control")),
            "Should warn about rent control in Jersey City"
        );
    }

    #[test]
    fn test_no_rent_control_warning_outside_regulated_area() {
        let text = "Property located in Princeton, NJ. Monthly rent: $2,500.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("42-84") && v.message.contains("rent control")),
            "Should not warn about rent control outside regulated municipalities"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_habitability_waiver() {
        let text = "Tenant agrees to waive the implied warranty of habitability.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("46:8-48") && v.severity == Severity::Critical),
            "Should detect habitability waiver"
        );
    }

    #[test]
    fn test_detects_anti_eviction_waiver() {
        let text = "Tenant waives all tenant rights under the Anti-Eviction Act.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            violations.iter().any(|v| v.statute.contains("2A:18-61.1")),
            "Should detect Anti-Eviction Act waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("2A:16-9.3") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    #[test]
    fn test_warns_acceleration_clause() {
        let text = "Upon default, all remaining rent shall be accelerated and due immediately.";
        let violations = check_new_jersey_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.message.contains("acceleration")),
            "Should warn about acceleration clause"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_new_jersey_lease() {
        let text = "Monthly rent: $2,000. Security deposit: $2,500 held in interest-bearing bank account. \
                    Truth in Renting Statement attached per DCA requirements. \
                    Landlord maintains property in habitable condition.";
        let violations = check_new_jersey_compliance(text);

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
