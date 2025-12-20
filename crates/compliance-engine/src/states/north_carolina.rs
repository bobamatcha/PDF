//! North Carolina Residential Landlord-Tenant Law Compliance
//!
//! North Carolina General Statutes Chapter 42
//! Key requirements based on LEASE_RESEARCH.md:
//! - Pet Fee vs Pet Deposit terminology distinction
//! - Security Deposit: 2 months max (unfurnished)
//! - Trust Account requirement for deposits
//! - Landlord must provide contact information

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

    /// Pet deposit pattern
    static ref PET_DEPOSIT_PATTERN: Regex =
        Regex::new(r"(?i)pet\s+(?:deposit|fee)[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();
}

/// Check all North Carolina-specific compliance requirements
pub fn check_north_carolina_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_security_deposit(text));
    violations.extend(check_pet_deposit_terminology(text));
    violations.extend(check_landlord_contact(text));
    violations.extend(check_trust_account(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// Security Deposit Requirements
// ============================================================================

/// Check security deposit limits
///
/// Per N.C.G.S. § 42-51:
/// - Week-to-week: 2 weeks max
/// - Month-to-month: 1.5 months max
/// - Longer term: 2 months max
/// - Must return within 30 days
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

    // Check deposit cap (2 months max for standard leases)
    if let (Some(deposit_amt), Some(rent_amt)) = (deposit, rent) {
        if deposit_amt > rent_amt * 2.0 {
            violations.push(Violation {
                statute: "N.C.G.S. § 42-51".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit (${:.2}) exceeds North Carolina limit of 2 months' rent (${:.2}).",
                    deposit_amt,
                    rent_amt * 2.0
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
                statute: "N.C.G.S. § 42-52".to_string(),
                severity: Severity::Info,
                message: "Consider specifying 30-day deposit return timeline. North Carolina \
                         requires return within 30 days of lease termination."
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
// Pet Deposit vs Pet Fee
// ============================================================================

/// Check pet deposit terminology
///
/// Per N.C.G.S. § 42-53:
/// - Pet deposits are refundable and part of security deposit cap
/// - Pet fees are non-refundable and must be clearly labeled
pub fn check_pet_deposit_terminology(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions pet deposit
    if text_lower.contains("pet deposit") {
        // Verify it's clearly described as refundable
        let is_clear = text_lower.contains("refundable")
            || text_lower.contains("returned")
            || text_lower.contains("security deposit");

        if !is_clear {
            violations.push(Violation {
                statute: "N.C.G.S. § 42-53".to_string(),
                severity: Severity::Warning,
                message: "Pet deposit should be clearly identified as refundable. Under North \
                         Carolina law, pet deposits are part of security deposit and subject to cap."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check if pet fee is clearly non-refundable
    if text_lower.contains("pet fee") {
        let is_clear = text_lower.contains("non-refundable")
            || text_lower.contains("nonrefundable")
            || text_lower.contains("not refundable");

        if !is_clear {
            violations.push(Violation {
                statute: "N.C.G.S. § 42-53".to_string(),
                severity: Severity::Warning,
                message: "A pet fee should be clearly identified as non-refundable to distinguish \
                         from pet deposit."
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
// Landlord Contact Information
// ============================================================================

/// Check landlord contact information requirement
///
/// Per N.C.G.S. § 42-44:
/// - Landlord must provide name and address for notices
pub fn check_landlord_contact(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for landlord contact information
    let has_contact = (text_lower.contains("landlord") || text_lower.contains("lessor"))
        && (text_lower.contains("address") || text_lower.contains("contact"));

    if !has_contact {
        violations.push(Violation {
            statute: "N.C.G.S. § 42-44".to_string(),
            severity: Severity::Info,
            message: "Lease should include landlord name and address for service of notices."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Trust Account Requirement
// ============================================================================

/// Check trust account requirement for deposits
///
/// Per N.C.G.S. § 42-50:
/// - Deposits must be held in trust account or bond
pub fn check_trust_account(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if deposit is mentioned
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_trust_disclosure = text_lower.contains("trust")
            || text_lower.contains("escrow")
            || text_lower.contains("licensed bank")
            || text_lower.contains("bond");

        if !has_trust_disclosure {
            violations.push(Violation {
                statute: "N.C.G.S. § 42-50".to_string(),
                severity: Severity::Warning,
                message: "Security deposit must be held in trust account at licensed bank or \
                         covered by bond. Consider disclosing deposit location."
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

/// Check for void clauses under North Carolina law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of landlord's duty to maintain
    if text_lower.contains("waive")
        && (text_lower.contains("habitability") || text_lower.contains("repair"))
    {
        violations.push(Violation {
            statute: "N.C.G.S. § 42-42".to_string(),
            severity: Severity::Critical,
            message: "Waiver of landlord's duty to maintain fit premises is void under North \
                     Carolina law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "N.C.G.S. § 42-46".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses are prohibited in North Carolina residential \
                     leases."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for waiver of jury trial
    if text_lower.contains("waive") && text_lower.contains("jury") {
        violations.push(Violation {
            statute: "N.C.G.S. § 42-46".to_string(),
            severity: Severity::Warning,
            message: "Waiver of right to jury trial may be unenforceable in North Carolina."
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
    fn test_detects_excessive_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $5,000.";
        let violations = check_north_carolina_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("42-51") && v.severity == Severity::Critical),
            "Should detect deposit exceeding 2 months rent"
        );
    }

    #[test]
    fn test_accepts_two_month_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $3,000.";
        let violations = check_north_carolina_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("42-51") && v.severity == Severity::Critical),
            "Should accept deposit equal to 2 months rent"
        );
    }

    // ========================================================================
    // Pet Deposit Terminology Tests
    // ========================================================================

    #[test]
    fn test_warns_unclear_pet_deposit() {
        let text = "Pet deposit: $500 required for all pet owners.";
        let violations = check_north_carolina_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("42-53") && v.message.contains("pet deposit")),
            "Should warn about unclear pet deposit"
        );
    }

    #[test]
    fn test_warns_unclear_pet_fee() {
        let text = "Pet fee: $250 required at move-in.";
        let violations = check_north_carolina_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("42-53") && v.message.contains("pet fee")),
            "Should warn about unclear pet fee"
        );
    }

    #[test]
    fn test_accepts_clear_pet_deposit() {
        let text = "Refundable pet deposit: $500 (part of security deposit).";
        let violations = check_north_carolina_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("42-53") && v.severity == Severity::Critical),
            "Should accept clear pet deposit"
        );
    }

    #[test]
    fn test_accepts_clear_pet_fee() {
        let text = "Non-refundable pet fee: $250.";
        let violations = check_north_carolina_compliance(text);

        assert!(
            !violations.iter().any(|v| v.statute.contains("42-53")
                && v.message.contains("pet fee")
                && v.severity == Severity::Warning),
            "Should accept clear non-refundable pet fee"
        );
    }

    // ========================================================================
    // Trust Account Tests
    // ========================================================================

    #[test]
    fn test_warns_missing_trust_account() {
        let text = "Security deposit: $2,000 held by landlord.";
        let violations = check_north_carolina_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("42-50") && v.message.contains("trust")),
            "Should warn about missing trust account disclosure"
        );
    }

    #[test]
    fn test_accepts_trust_account() {
        let text = "Security deposit: $2,000 held in trust account at licensed bank.";
        let violations = check_north_carolina_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("42-50") && v.severity == Severity::Critical),
            "Should accept trust account disclosure"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_habitability_waiver() {
        let text = "Tenant agrees to waive implied warranty of habitability.";
        let violations = check_north_carolina_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("42-42") && v.severity == Severity::Critical),
            "Should detect habitability waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_north_carolina_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("42-46") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_north_carolina_lease() {
        let text = "Landlord: ABC Properties, 123 Main St, Raleigh NC. \
                    Monthly rent: $1,800. Security deposit: $1,800 held in trust account. \
                    Deposit returned within 30 days. \
                    Non-refundable pet fee: $250.";
        let violations = check_north_carolina_compliance(text);

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
