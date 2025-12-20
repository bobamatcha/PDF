//! Michigan Residential Landlord-Tenant Law Compliance
//!
//! Michigan Truth in Renting Act (M.C.L. 554.631 et seq.)
//! Key requirements based on LEASE_RESEARCH.md:
//! - Source of Income Protection (SB 205-207)
//! - Security Deposit: 1.5 months max
//! - Inventory checklist required
//! - 30-day deposit return

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

    /// Source of income discrimination pattern
    static ref INCOME_DISCRIMINATION_PATTERN: Regex =
        Regex::new(r"(?i)(no\s+section\s+8|no\s+voucher|no\s+housing\s+assistance|income\s+source\s+(?:not\s+accepted|prohibited))").unwrap();
}

/// Check all Michigan-specific compliance requirements
pub fn check_michigan_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_source_of_income(text));
    violations.extend(check_security_deposit(text));
    violations.extend(check_inventory_checklist(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// Source of Income Protection (SB 205-207)
// ============================================================================

/// Check source of income discrimination
///
/// Per Michigan SB 205-207:
/// - Cannot discriminate based on lawful source of income
/// - Includes Section 8 vouchers, housing assistance
pub fn check_source_of_income(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    if INCOME_DISCRIMINATION_PATTERN.is_match(text) {
        violations.push(Violation {
            statute: "M.C.L. 37.2502a (SB 205-207)".to_string(),
            severity: Severity::Critical,
            message: "Source of income discrimination prohibited. Michigan law prohibits \
                     discrimination based on lawful source of income including housing vouchers."
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
/// Per M.C.L. 554.602:
/// - Maximum 1.5 months rent
/// - Must return within 30 days
/// - Inventory checklist required
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
                statute: "M.C.L. 554.602".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit (${:.2}) exceeds Michigan limit of 1.5 months' rent (${:.2}).",
                    deposit_amt,
                    rent_amt * 1.5
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
                statute: "M.C.L. 554.609".to_string(),
                severity: Severity::Info,
                message: "Consider specifying 30-day deposit return timeline as required by \
                         Michigan law."
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
// Inventory Checklist
// ============================================================================

/// Check inventory checklist requirement
///
/// Per M.C.L. 554.608:
/// - Landlord must provide inventory checklist
/// - Tenant must have opportunity to inspect
pub fn check_inventory_checklist(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if deposit is mentioned
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_inventory = text_lower.contains("inventory")
            || text_lower.contains("checklist")
            || text_lower.contains("move-in inspection")
            || text_lower.contains("condition report");

        if !has_inventory {
            violations.push(Violation {
                statute: "M.C.L. 554.608".to_string(),
                severity: Severity::Warning,
                message:
                    "Inventory checklist required. Michigan law requires landlords to provide \
                         inventory checklist within 7 days of tenant's request."
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

/// Check for void clauses under Michigan law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of statutory rights
    if text_lower.contains("waive")
        && (text_lower.contains("truth in renting") || text_lower.contains("tenant rights"))
    {
        violations.push(Violation {
            statute: "M.C.L. 554.633".to_string(),
            severity: Severity::Critical,
            message: "Waiver of Truth in Renting Act rights is void under Michigan law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "M.C.L. 554.633".to_string(),
            severity: Severity::Critical,
            message:
                "Confession of judgment clauses are prohibited in Michigan residential leases."
                    .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for liability waiver
    if text_lower.contains("waive") && text_lower.contains("negligence") {
        violations.push(Violation {
            statute: "M.C.L. 554.139".to_string(),
            severity: Severity::Critical,
            message: "Waiver of landlord's liability for negligence is void under Michigan law."
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
    // Source of Income Tests
    // ========================================================================

    #[test]
    fn test_detects_section_8_discrimination() {
        let text = "No Section 8 vouchers accepted. All applicants must have employment income.";
        let violations = check_michigan_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("37.2502a") && v.severity == Severity::Critical),
            "Should detect Section 8 discrimination"
        );
    }

    #[test]
    fn test_detects_housing_assistance_discrimination() {
        let text = "No housing assistance or government subsidies accepted.";
        let violations = check_michigan_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.message.contains("source of income")),
            "Should detect housing assistance discrimination"
        );
    }

    #[test]
    fn test_no_violation_accepts_vouchers() {
        let text = "All lawful sources of income accepted including housing vouchers.";
        let violations = check_michigan_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("37.2502a") && v.severity == Severity::Critical),
            "Should not flag when vouchers accepted"
        );
    }

    // ========================================================================
    // Security Deposit Tests
    // ========================================================================

    #[test]
    fn test_detects_excessive_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $3,000.";
        let violations = check_michigan_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("554.602") && v.severity == Severity::Critical),
            "Should detect deposit exceeding 1.5 months rent"
        );
    }

    #[test]
    fn test_accepts_compliant_deposit() {
        let text = "Monthly rent: $2,000. Security deposit: $3,000.";
        let violations = check_michigan_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("554.602") && v.severity == Severity::Critical),
            "Should accept deposit equal to 1.5 months rent"
        );
    }

    // ========================================================================
    // Inventory Checklist Tests
    // ========================================================================

    #[test]
    fn test_warns_missing_inventory() {
        let text = "Security deposit: $2,000. Deposit held by landlord.";
        let violations = check_michigan_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("554.608") && v.message.contains("inventory")),
            "Should warn about missing inventory checklist"
        );
    }

    #[test]
    fn test_accepts_inventory_checklist() {
        let text = "Security deposit: $2,000. Move-in inventory checklist attached.";
        let violations = check_michigan_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("554.608") && v.severity == Severity::Critical),
            "Should accept inventory checklist"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_truth_in_renting_waiver() {
        let text = "Tenant agrees to waive all rights under Truth in Renting Act.";
        let violations = check_michigan_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("554.633") && v.message.contains("Truth in Renting")),
            "Should detect Truth in Renting waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_michigan_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("554.633") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    #[test]
    fn test_detects_negligence_waiver() {
        let text = "Tenant waives landlord's liability for negligence.";
        let violations = check_michigan_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("554.139") && v.message.contains("negligence")),
            "Should detect negligence waiver"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_michigan_lease() {
        let text = "Monthly rent: $2,000. Security deposit: $2,500. \
                    Deposit returned within 30 days. Inventory checklist attached. \
                    All lawful income sources accepted.";
        let violations = check_michigan_compliance(text);

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
