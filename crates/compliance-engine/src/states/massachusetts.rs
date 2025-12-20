//! Massachusetts Residential Landlord-Tenant Law Compliance
//!
//! Massachusetts General Laws Chapter 186
//! Key requirements based on LEASE_RESEARCH.md:
//! - Broker Fee Reform (Aug 2025) - landlord pays own broker
//! - Security Deposit: 1 month max
//! - Last Month's Rent: Separate from deposit
//! - Tenant Rights Statement required
//! - Interest on security deposits

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

    /// Broker fee pattern
    static ref BROKER_FEE_PATTERN: Regex =
        Regex::new(r"(?i)(broker|realtor|agent)\s+(fee|commission)[:\s]+").unwrap();
}

/// Check all Massachusetts-specific compliance requirements
pub fn check_massachusetts_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_security_deposit(text));
    violations.extend(check_broker_fee(text));
    violations.extend(check_tenant_rights_statement(text));
    violations.extend(check_deposit_interest(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// Security Deposit Requirements
// ============================================================================

/// Check security deposit limits
///
/// Per M.G.L. c. 186 § 15B:
/// - Maximum 1 month rent as security deposit
/// - Last month's rent separate
/// - Must hold in separate interest-bearing account
/// - Return within 30 days
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

    // Check deposit cap (1 month max)
    if let (Some(deposit_amt), Some(rent_amt)) = (deposit, rent) {
        if deposit_amt > rent_amt {
            violations.push(Violation {
                statute: "M.G.L. c. 186 § 15B".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit (${:.2}) exceeds Massachusetts limit of 1 month's rent (${:.2}). \
                     Note: Last month's rent must be collected separately.",
                    deposit_amt, rent_amt
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for separate account requirement
    let text_lower = text.to_lowercase();
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_separate_account = text_lower.contains("separate account")
            || text_lower.contains("escrow")
            || text_lower.contains("interest-bearing");

        if !has_separate_account {
            violations.push(Violation {
                statute: "M.G.L. c. 186 § 15B".to_string(),
                severity: Severity::Warning,
                message: "Security deposit must be held in separate, interest-bearing account \
                         in a Massachusetts bank."
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
// Broker Fee Reform (2025)
// ============================================================================

/// Check broker fee requirements
///
/// Per 2025 Massachusetts broker fee reform:
/// - Landlord must pay their own broker's fee
/// - Cannot pass broker fees to tenant
pub fn check_broker_fee(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for landlord-pays or no-fee scenarios first (these are OK)
    let landlord_pays = text_lower.contains("landlord pays")
        || text_lower.contains("landlord responsible for")
        || text_lower.contains("owner pays");

    let no_tenant_fee = text_lower.contains("no broker fee")
        || text_lower.contains("no fee to tenant")
        || text_lower.contains("not charged to tenant");

    // If landlord pays or no tenant fee, skip the check
    if landlord_pays || no_tenant_fee {
        return violations;
    }

    // Check if tenant is being charged broker fee
    if BROKER_FEE_PATTERN.is_match(text) {
        let tenant_pays = (text_lower.contains("tenant") && text_lower.contains("broker fee"))
            || text_lower.contains("tenant pays broker")
            || text_lower.contains("finder's fee");

        if tenant_pays {
            violations.push(Violation {
                statute: "M.G.L. c. 186 (2025 Reform)".to_string(),
                severity: Severity::Critical,
                message: "Tenant cannot be required to pay landlord's broker fee. \
                         Massachusetts 2025 broker fee reform requires landlord to pay own broker."
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
// Tenant Rights Statement
// ============================================================================

/// Check tenant rights statement requirement
///
/// Per M.G.L. c. 186 § 15B:
/// - Must provide statement of tenant rights
pub fn check_tenant_rights_statement(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for tenant rights statement
    let has_rights_statement = text_lower.contains("tenant rights")
        || text_lower.contains("statement of condition")
        || text_lower.contains("186 § 15b")
        || text_lower.contains("tenant's rights");

    if !has_rights_statement {
        violations.push(Violation {
            statute: "M.G.L. c. 186 § 15B".to_string(),
            severity: Severity::Info,
            message: "Consider including Statement of Tenant Rights. Massachusetts requires \
                     landlords to provide information about tenant rights."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Security Deposit Interest
// ============================================================================

/// Check security deposit interest requirements
///
/// Per M.G.L. c. 186 § 15B:
/// - Landlord must pay interest on security deposit annually
pub fn check_deposit_interest(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if deposit is mentioned
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        // Check for interest disclosure
        let has_interest_disclosure =
            text_lower.contains("interest") || text_lower.contains("annual payment");

        if !has_interest_disclosure {
            violations.push(Violation {
                statute: "M.G.L. c. 186 § 15B".to_string(),
                severity: Severity::Warning,
                message: "Security deposit interest disclosure required. Massachusetts landlords \
                         must pay interest on deposits annually."
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

/// Check for void clauses under Massachusetts law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of security deposit rights
    if text_lower.contains("waive") && text_lower.contains("deposit") {
        violations.push(Violation {
            statute: "M.G.L. c. 186 § 15B".to_string(),
            severity: Severity::Critical,
            message: "Waiver of security deposit rights is void under Massachusetts law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for waiver of quiet enjoyment
    if text_lower.contains("waive") && text_lower.contains("quiet enjoyment") {
        violations.push(Violation {
            statute: "M.G.L. c. 186 § 14".to_string(),
            severity: Severity::Critical,
            message: "Waiver of covenant of quiet enjoyment is void under Massachusetts law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "M.G.L. c. 231 § 13A".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses are prohibited in Massachusetts residential \
                     leases."
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
        let text = "Monthly rent: $2,000. Security deposit: $4,000.";
        let violations = check_massachusetts_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("186 § 15B") && v.severity == Severity::Critical),
            "Should detect deposit exceeding 1 month rent"
        );
    }

    #[test]
    fn test_accepts_one_month_deposit() {
        let text = "Monthly rent: $2,000. Security deposit: $2,000.";
        let violations = check_massachusetts_compliance(text);

        assert!(
            !violations.iter().any(|v| v.statute.contains("186 § 15B")
                && v.severity == Severity::Critical
                && v.message.contains("exceeds")),
            "Should accept deposit equal to 1 month rent"
        );
    }

    #[test]
    fn test_warns_missing_separate_account() {
        let text = "Security deposit: $1,500 held by landlord.";
        let violations = check_massachusetts_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.message.contains("separate") || v.message.contains("interest-bearing")),
            "Should warn about missing separate account disclosure"
        );
    }

    // ========================================================================
    // Broker Fee Tests
    // ========================================================================

    #[test]
    fn test_detects_tenant_broker_fee() {
        let text = "Tenant pays broker fee of one month's rent. \
                    Agent commission due at signing.";
        let violations = check_massachusetts_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.message.contains("broker") && v.severity == Severity::Critical),
            "Should detect tenant-paid broker fee"
        );
    }

    #[test]
    fn test_no_violation_landlord_pays_broker() {
        let text = "Landlord pays all broker fees. No broker fee to tenant.";
        let violations = check_massachusetts_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.message.contains("broker") && v.severity == Severity::Critical),
            "Should accept landlord-paid broker fee"
        );
    }

    // ========================================================================
    // Deposit Interest Tests
    // ========================================================================

    #[test]
    fn test_warns_missing_interest_disclosure() {
        let text = "Security deposit: $2,000. Deposit held in escrow account.";
        let violations = check_massachusetts_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.message.contains("interest") && v.severity == Severity::Warning),
            "Should warn about missing interest disclosure"
        );
    }

    #[test]
    fn test_accepts_interest_disclosure() {
        let text = "Security deposit held in interest-bearing separate account. \
                    Interest paid annually to tenant.";
        let violations = check_massachusetts_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.message.contains("interest") && v.severity == Severity::Critical),
            "Should accept proper interest disclosure"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_deposit_rights_waiver() {
        let text = "Tenant agrees to waive all rights regarding security deposit.";
        let violations = check_massachusetts_compliance(text);

        assert!(
            violations.iter().any(|v| v.statute.contains("186 § 15B")
                && v.severity == Severity::Critical
                && v.message.contains("Waiver")),
            "Should detect deposit rights waiver"
        );
    }

    #[test]
    fn test_detects_quiet_enjoyment_waiver() {
        let text = "Tenant waives covenant of quiet enjoyment.";
        let violations = check_massachusetts_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("186 § 14") && v.message.contains("quiet enjoyment")),
            "Should detect quiet enjoyment waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_massachusetts_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("231 § 13A") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_massachusetts_lease() {
        let text = "Monthly rent: $2,500. Security deposit: $2,500. \
                    Deposit held in separate interest-bearing account. \
                    Interest paid annually per M.G.L. c. 186 § 15B. \
                    Statement of tenant rights attached. \
                    No broker fee charged to tenant.";
        let violations = check_massachusetts_compliance(text);

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
