//! Washington Residential Landlord-Tenant Law Compliance
//!
//! Residential Landlord-Tenant Act (RCW 59.18)
//! Key requirements based on LEASE_RESEARCH.md:
//! - 90-day rent increase notice (up from 60)
//! - Just Cause Eviction (Seattle, Tacoma, and expanding)
//! - Security Deposit: No statutory cap
//! - Move-in inspection checklist required
//! - Mold disclosure

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

    /// Rent increase notice pattern
    static ref RENT_INCREASE_NOTICE_PATTERN: Regex =
        Regex::new(r"(?i)(\d+)\s*(?:day|days)\s*(?:notice|prior).*(?:rent\s+increase|increase\s+rent)").unwrap();
}

// Just Cause cities in Washington
const JUST_CAUSE_CITIES: &[&str] = &["seattle", "tacoma", "olympia", "burien"];

/// Check all Washington-specific compliance requirements
pub fn check_washington_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_rent_increase_notice(text));
    violations.extend(check_just_cause(text));
    violations.extend(check_security_deposit(text));
    violations.extend(check_mold_disclosure(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// 90-Day Rent Increase Notice
// ============================================================================

/// Check rent increase notice requirements
///
/// Per RCW 59.18.140:
/// - 90 days written notice required for rent increases (2025 law)
pub fn check_rent_increase_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions rent increases
    if text_lower.contains("rent increase") || text_lower.contains("increase rent") {
        // Check for proper notice period
        if let Some(caps) = RENT_INCREASE_NOTICE_PATTERN.captures(text) {
            if let Some(days_str) = caps.get(1) {
                if let Ok(days) = days_str.as_str().parse::<u32>() {
                    if days < 90 {
                        violations.push(Violation {
                            statute: "RCW 59.18.140".to_string(),
                            severity: Severity::Critical,
                            message: format!(
                                "Rent increase notice ({} days) is insufficient. \
                                 Washington requires 90 days written notice for rent increases.",
                                days
                            ),
                            page: None,
                            text_snippet: None,
                            text_position: None,
                        });
                    }
                }
            }
        } else {
            // No notice period mentioned
            let has_90_days = text_lower.contains("90 days")
                || text_lower.contains("ninety days")
                || text_lower.contains("90-day");

            if !has_90_days {
                violations.push(Violation {
                    statute: "RCW 59.18.140".to_string(),
                    severity: Severity::Warning,
                    message: "Rent increase requires 90 days written notice under Washington law."
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
// Just Cause Eviction
// ============================================================================

/// Check Just Cause eviction requirements
///
/// Seattle, Tacoma, and other cities require Just Cause for termination
pub fn check_just_cause(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if property is in Just Cause city
    let in_just_cause_city = JUST_CAUSE_CITIES
        .iter()
        .any(|city| text_lower.contains(city));

    if in_just_cause_city {
        // Check for Just Cause disclosure
        let has_just_cause = text_lower.contains("just cause")
            || text_lower.contains("good cause")
            || text_lower.contains("for cause eviction");

        if !has_just_cause {
            violations.push(Violation {
                statute: "Seattle Mun. Code 22.206".to_string(),
                severity: Severity::Warning,
                message: "Just Cause eviction disclosure recommended. Property appears to be in \
                         a city with Just Cause eviction protections."
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
// Security Deposit Requirements
// ============================================================================

/// Check security deposit requirements
///
/// Per RCW 59.18.260:
/// - No statutory cap, but must be reasonable
/// - Must return within 21 days
/// - Written checklist required at move-in
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
                statute: "RCW 59.18.260".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Security deposit (${:.2}) is high. While Washington has no cap, \
                     excessive deposits may be challenged as unreasonable.",
                    deposit_amt
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for 21-day return requirement
    let text_lower = text.to_lowercase();
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_return_timeline = text_lower.contains("21 days")
            || text_lower.contains("twenty-one days")
            || text_lower.contains("21-day");

        if !has_return_timeline {
            violations.push(Violation {
                statute: "RCW 59.18.280".to_string(),
                severity: Severity::Warning,
                message: "Must specify 21-day deposit return timeline. Washington requires \
                         return within 21 days after termination and vacation."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }

        // Check for checklist requirement
        let has_checklist = text_lower.contains("checklist")
            || text_lower.contains("move-in inspection")
            || text_lower.contains("condition report");

        if !has_checklist {
            violations.push(Violation {
                statute: "RCW 59.18.260".to_string(),
                severity: Severity::Info,
                message: "Move-in checklist recommended. Washington requires written checklist \
                         to support deposit deductions."
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
/// Per RCW 59.18.060:
/// - Landlord must disclose mold if known
pub fn check_mold_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for mold mention without proper disclosure
    if text_lower.contains("mold") || text_lower.contains("mildew") {
        let has_proper_disclosure = text_lower.contains("mold disclosure")
            || text_lower.contains("59.18.060")
            || (text_lower.contains("mold") && text_lower.contains("disclosure"));

        if !has_proper_disclosure {
            violations.push(Violation {
                statute: "RCW 59.18.060".to_string(),
                severity: Severity::Warning,
                message: "Mold disclosure should include specific information about known mold \
                         conditions."
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

/// Check for void clauses under Washington law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of statutory rights
    if text_lower.contains("waive") && text_lower.contains("tenant rights") {
        violations.push(Violation {
            statute: "RCW 59.18.230".to_string(),
            severity: Severity::Critical,
            message: "Waiver of tenant rights under RLTA is void.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for waiver of habitability
    if text_lower.contains("waive")
        && (text_lower.contains("habitability") || text_lower.contains("repair"))
    {
        violations.push(Violation {
            statute: "RCW 59.18.060".to_string(),
            severity: Severity::Critical,
            message: "Waiver of landlord's duty to maintain habitable premises is void."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "RCW 59.18.230".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses are prohibited in Washington residential \
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
    // Rent Increase Notice Tests
    // ========================================================================

    #[test]
    fn test_detects_insufficient_rent_increase_notice() {
        let text = "60 days notice prior to any rent increase.";
        let violations = check_washington_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("59.18.140") && v.severity == Severity::Critical),
            "Should detect insufficient rent increase notice"
        );
    }

    #[test]
    fn test_accepts_90_day_rent_increase_notice() {
        let text = "90 days notice required for any rent increase.";
        let violations = check_washington_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("59.18.140") && v.severity == Severity::Critical),
            "Should accept 90-day rent increase notice"
        );
    }

    // ========================================================================
    // Just Cause Tests
    // ========================================================================

    #[test]
    fn test_warns_missing_just_cause_seattle() {
        let text = "Property located in Seattle, WA. Monthly rent: $2,500.";
        let violations = check_washington_compliance(text);

        assert!(
            violations.iter().any(|v| v.message.contains("Just Cause")),
            "Should warn about missing Just Cause disclosure in Seattle"
        );
    }

    #[test]
    fn test_accepts_just_cause_disclosure() {
        let text = "Property in Seattle. Just Cause eviction protections apply.";
        let violations = check_washington_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.message.contains("Just Cause") && v.severity == Severity::Critical),
            "Should accept Just Cause disclosure"
        );
    }

    // ========================================================================
    // Security Deposit Tests
    // ========================================================================

    #[test]
    fn test_warns_high_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $5,000.";
        let violations = check_washington_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("59.18.260") && v.message.contains("high")),
            "Should warn about high deposit"
        );
    }

    #[test]
    fn test_warns_missing_21_day_return() {
        let text = "Security deposit: $2,000. Deposit returned after move-out.";
        let violations = check_washington_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("59.18.280") && v.message.contains("21-day")),
            "Should warn about missing 21-day return timeline"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_tenant_rights_waiver() {
        let text = "Tenant waives all tenant rights under Washington law.";
        let violations = check_washington_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("59.18.230") && v.severity == Severity::Critical),
            "Should detect tenant rights waiver"
        );
    }

    #[test]
    fn test_detects_habitability_waiver() {
        let text = "Tenant agrees to waive warranty of habitability.";
        let violations = check_washington_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("59.18.060") && v.severity == Severity::Critical),
            "Should detect habitability waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_washington_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("59.18.230") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_washington_lease() {
        let text = "Monthly rent: $2,500. Security deposit: $2,500. \
                    Deposit returned within 21 days. Move-in checklist attached. \
                    90 days notice required for rent increase. \
                    Property in Seattle - Just Cause eviction applies.";
        let violations = check_washington_compliance(text);

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
