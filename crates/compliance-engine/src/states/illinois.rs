//! Illinois Residential Landlord-Tenant Law Compliance
//!
//! Illinois Landlord and Tenant Act (765 ILCS 705-750)
//! Chicago RLTO (Chicago Municipal Code 5-12)
//! Key requirements based on LEASE_RESEARCH.md:
//! - Chicago RLTO Summary requirement (voidable without it)
//! - Security Deposit Interest (Chicago)
//! - Electronic Payment Ban (2025) - cannot require electronic-only
//! - Landlord Retaliation Act (2025)
//! - Bed Bug Disclosure (Chicago)

use lazy_static::lazy_static;
use regex::Regex;
use shared_types::{Severity, Violation};

// Chicago zip codes (partial list - major ranges)
const CHICAGO_ZIP_PREFIXES: &[&str] = &[
    "606", "607", "608", // Chicago proper
];

lazy_static! {
    /// Electronic payment requirement pattern
    static ref ELECTRONIC_ONLY_PATTERN: Regex =
        Regex::new(r"(?i)(must\s+pay|payment\s+required|only\s+accept).*(online|electronic|app|portal)").unwrap();

    /// Security deposit pattern
    static ref DEPOSIT_AMOUNT_PATTERN: Regex =
        Regex::new(r"(?i)(?:security\s+)?deposit[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    /// Monthly rent pattern
    static ref RENT_AMOUNT_PATTERN: Regex =
        Regex::new(r"(?i)(?:monthly\s+)?rent[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();
}

/// Check if zip code is in Chicago
fn is_chicago_zip(text: &str) -> bool {
    for prefix in CHICAGO_ZIP_PREFIXES {
        if text.contains(prefix) {
            return true;
        }
    }
    // Also check explicit mentions
    text.to_lowercase().contains("chicago")
}

/// Check all Illinois-specific compliance requirements
pub fn check_illinois_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let is_chicago = is_chicago_zip(text);

    // Chicago-specific checks
    if is_chicago {
        violations.extend(check_rlto_summary(text));
        violations.extend(check_deposit_interest(text));
        violations.extend(check_bed_bug_disclosure(text));
    }

    // Statewide checks
    violations.extend(check_electronic_payment(text));
    violations.extend(check_security_deposit(text));
    violations.extend(check_retaliation_disclosure(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// Chicago RLTO Requirements
// ============================================================================

/// Check RLTO Summary requirement for Chicago
///
/// Per Chicago Municipal Code 5-12-170:
/// Landlord must attach RLTO Summary or lease is voidable by tenant
pub fn check_rlto_summary(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for RLTO summary attachment
    let has_rlto_summary = text_lower.contains("rlto")
        || text_lower.contains("residential landlord and tenant ordinance")
        || text_lower.contains("chicago ordinance")
        || text_lower.contains("5-12-170")
        || (text_lower.contains("summary") && text_lower.contains("attached"));

    if !has_rlto_summary {
        violations.push(Violation {
            statute: "Chicago Mun. Code § 5-12-170".to_string(),
            severity: Severity::Critical,
            message: "RLTO Summary required for Chicago properties. \
                     Lease is voidable at tenant's option without attached RLTO Summary."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check security deposit interest disclosure for Chicago
///
/// Per Chicago Municipal Code 5-12-080:
/// Landlord must pay interest on security deposits and provide receipt
pub fn check_deposit_interest(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if deposit is mentioned
    let has_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if has_deposit {
        // Check for interest disclosure
        let has_interest_disclosure = text_lower.contains("interest")
            || text_lower.contains("5-12-080")
            || text_lower.contains("deposit interest");

        if !has_interest_disclosure {
            violations.push(Violation {
                statute: "Chicago Mun. Code § 5-12-080".to_string(),
                severity: Severity::Warning,
                message: "Security deposit interest disclosure recommended for Chicago. \
                         Landlord must pay interest and provide receipt within 14 days."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

/// Check bed bug disclosure for Chicago
///
/// Per Chicago Municipal Code 5-12-100:
/// Landlord must provide bed bug disclosure and brochure
pub fn check_bed_bug_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for bed bug disclosure
    let has_bed_bug_disclosure = text_lower.contains("bed bug")
        || text_lower.contains("bedbug")
        || text_lower.contains("5-12-100");

    if !has_bed_bug_disclosure {
        violations.push(Violation {
            statute: "Chicago Mun. Code § 5-12-100".to_string(),
            severity: Severity::Warning,
            message: "Bed bug disclosure recommended for Chicago properties. \
                     Consider attaching city-provided bed bug brochure."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Statewide Requirements
// ============================================================================

/// Check electronic payment requirement (2025 law)
///
/// Per 2025 Illinois law:
/// Landlord cannot require electronic-only payment
pub fn check_electronic_payment(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    if ELECTRONIC_ONLY_PATTERN.is_match(text) {
        violations.push(Violation {
            statute: "765 ILCS 705/2.5".to_string(),
            severity: Severity::Critical,
            message: "Cannot require electronic-only payments. \
                     Illinois law requires acceptance of alternative payment methods."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check security deposit requirements
///
/// Per 765 ILCS 710:
/// Specific requirements for deposit handling
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

    // Illinois has no statutory cap, but check for return timeline
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        // Check for return timeline (30-45 days depending on itemization)
        let has_return_timeline = text_lower.contains("30 days")
            || text_lower.contains("45 days")
            || text_lower.contains("return")
            || text_lower.contains("refund");

        if !has_return_timeline {
            violations.push(Violation {
                statute: "765 ILCS 710".to_string(),
                severity: Severity::Info,
                message: "Consider specifying deposit return timeline. \
                         Illinois requires return within 30-45 days of move-out."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Warn about excessive deposits (more than 1.5x rent is unusual)
    if let (Some(deposit_amt), Some(rent_amt)) = (deposit, rent) {
        if deposit_amt > rent_amt * 1.5 {
            violations.push(Violation {
                statute: "765 ILCS 710".to_string(),
                severity: Severity::Info,
                message: format!(
                    "Security deposit (${:.2}) is higher than typical (1-1.5x rent). \
                     Ensure compliance with deposit handling requirements.",
                    deposit_amt
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

/// Check landlord retaliation disclosure (2025 law)
///
/// Per 765 ILCS 720 (updated 2025):
/// Tenants have rights regarding repair requests
pub fn check_retaliation_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions repairs
    if text_lower.contains("repair") || text_lower.contains("maintenance") {
        // Check for retaliation protection acknowledgment
        let has_retaliation_protection = text_lower.contains("retaliation")
            || text_lower.contains("765 ilcs 720")
            || text_lower.contains("retaliatory eviction");

        if !has_retaliation_protection {
            violations.push(Violation {
                statute: "765 ILCS 720".to_string(),
                severity: Severity::Info,
                message: "Consider noting tenant's protection against retaliation. \
                         Illinois prohibits retaliation for exercising tenant rights."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

/// Check for void clauses under Illinois law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of landlord liability
    if text_lower.contains("waive")
        && (text_lower.contains("negligence") || text_lower.contains("liability"))
    {
        violations.push(Violation {
            statute: "765 ILCS 705/1".to_string(),
            severity: Severity::Critical,
            message: "Waiver of landlord's liability for negligence is void under Illinois law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "735 ILCS 5/2-1301".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses are void in Illinois residential leases."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for waiver of right to jury trial
    if text_lower.contains("waive") && text_lower.contains("jury") {
        violations.push(Violation {
            statute: "765 ILCS 705/1.5".to_string(),
            severity: Severity::Critical,
            message: "Waiver of right to jury trial is void under Illinois law.".to_string(),
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
    // Chicago RLTO Tests
    // ========================================================================

    #[test]
    fn test_detects_missing_rlto_summary_chicago() {
        let text = "Property located at 123 Main St, Chicago, IL 60601. Monthly rent: $1,500.";
        let violations = check_illinois_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("5-12-170") && v.severity == Severity::Critical),
            "Should detect missing RLTO summary for Chicago"
        );
    }

    #[test]
    fn test_accepts_rlto_summary_present() {
        let text = "Property in Chicago, IL 60601. \
                    RLTO Summary attached. Residential Landlord and Tenant Ordinance applies.";
        let violations = check_illinois_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("5-12-170") && v.severity == Severity::Critical),
            "Should accept lease with RLTO summary"
        );
    }

    #[test]
    fn test_no_rlto_requirement_outside_chicago() {
        let text = "Property in Springfield, IL 62701. Monthly rent: $1,200.";
        let violations = check_illinois_compliance(text);

        assert!(
            !violations.iter().any(|v| v.statute.contains("5-12-170")),
            "Should not require RLTO summary outside Chicago"
        );
    }

    #[test]
    fn test_warns_missing_bed_bug_disclosure_chicago() {
        let text = "Chicago, IL 60602. Deposit: $1,500. RLTO Summary attached.";
        let violations = check_illinois_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("5-12-100") && v.message.contains("bed bug")),
            "Should warn about missing bed bug disclosure in Chicago"
        );
    }

    #[test]
    fn test_accepts_bed_bug_disclosure() {
        let text = "Chicago, IL 60602. RLTO Summary attached. \
                    Bed Bug Disclosure: City brochure provided.";
        let violations = check_illinois_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("5-12-100") && v.severity == Severity::Critical),
            "Should accept bed bug disclosure"
        );
    }

    // ========================================================================
    // Electronic Payment Tests
    // ========================================================================

    #[test]
    fn test_detects_electronic_only_requirement() {
        let text = "Tenant must pay rent online through the landlord portal. \
                    No other payment methods accepted.";
        let violations = check_illinois_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("705/2.5") && v.severity == Severity::Critical),
            "Should detect electronic-only payment requirement"
        );
    }

    #[test]
    fn test_accepts_multiple_payment_methods() {
        let text = "Rent may be paid by check, money order, or online portal.";
        let violations = check_illinois_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("705/2.5") && v.severity == Severity::Critical),
            "Should accept multiple payment methods"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_liability_waiver() {
        let text = "Tenant waives any claims of negligence against landlord.";
        let violations = check_illinois_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("705/1") && v.severity == Severity::Critical),
            "Should detect void liability waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_illinois_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("2-1301") && v.severity == Severity::Critical),
            "Should detect void confession of judgment"
        );
    }

    #[test]
    fn test_detects_jury_waiver() {
        let text = "Tenant waives right to jury trial in any dispute.";
        let violations = check_illinois_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("705/1.5") && v.message.contains("jury")),
            "Should detect void jury waiver"
        );
    }

    // ========================================================================
    // Compliant Lease Tests
    // ========================================================================

    #[test]
    fn test_compliant_chicago_lease() {
        let text =
            "Property in Chicago, IL 60601. Monthly rent: $1,800. Security deposit: $1,800. \
                    RLTO Summary attached per Chicago ordinance. \
                    Deposit earns interest per § 5-12-080. \
                    Bed bug disclosure and brochure provided. \
                    Deposit returned within 30 days of move-out. \
                    Rent may be paid by check or online.";
        let violations = check_illinois_compliance(text);

        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(
            critical.is_empty(),
            "Compliant Chicago lease should have no critical violations"
        );
    }

    #[test]
    fn test_compliant_illinois_non_chicago_lease() {
        let text = "Property in Springfield, IL 62701. Monthly rent: $1,200. \
                    Security deposit: $1,200 returned within 30 days. \
                    Rent may be paid by check or money order.";
        let violations = check_illinois_compliance(text);

        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(
            critical.is_empty(),
            "Compliant non-Chicago lease should have no critical violations"
        );
    }
}
