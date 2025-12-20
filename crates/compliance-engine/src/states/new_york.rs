//! New York Residential Landlord-Tenant Law Compliance
//!
//! NY Real Property Law Article 7, Housing Stability and Tenant Protection Act (2019)
//! Key requirements based on LEASE_RESEARCH.md:
//! - Good Cause Eviction Rider (2024+) - NYC and opted-in municipalities
//! - Late Fee Cap - $50 or 5% (whichever is less)
//! - Rent Stabilization - DHCR Rider for pre-1974 buildings (NYC)
//! - Security Deposit Cap - 1 month max (statewide)

use lazy_static::lazy_static;
use regex::Regex;
use shared_types::{Severity, Violation};

lazy_static! {
    /// Late fee pattern
    static ref LATE_FEE_PATTERN: Regex =
        Regex::new(r"(?i)late\s*fee[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    /// Late fee percentage pattern
    static ref LATE_FEE_PERCENT_PATTERN: Regex =
        Regex::new(r"(?i)late\s*fee.*?(\d+)\s*%").unwrap();

    /// Monthly rent pattern
    static ref RENT_AMOUNT_PATTERN: Regex =
        Regex::new(r"(?i)(?:monthly\s+)?rent[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    /// Security deposit pattern
    static ref DEPOSIT_AMOUNT_PATTERN: Regex =
        Regex::new(r"(?i)(?:security\s+)?deposit[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    /// NYC zip code ranges
    static ref NYC_ZIP_PATTERN: Regex =
        Regex::new(r"\b(100\d{2}|101\d{2}|102\d{2}|103\d{2}|104\d{2}|110\d{2}|111\d{2}|112\d{2}|113\d{2}|114\d{2}|116\d{2})\b").unwrap();

    /// Pre-1974 building pattern
    static ref PRE_1974_PATTERN: Regex =
        Regex::new(r"(?i)(?:built|constructed|year\s+built)(?:\s+in)?[:\s]+(19[0-6]\d|19[7][0-3])").unwrap();
}

/// Check all New York-specific compliance requirements
pub fn check_new_york_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_late_fee_cap(text));
    violations.extend(check_security_deposit_cap(text));
    violations.extend(check_good_cause_disclosure(text));
    violations.extend(check_rent_stabilization(text));
    violations.extend(check_lease_renewal_notice(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// Late Fee Cap - RPL § 238-a
// ============================================================================

/// Check late fee cap requirements
///
/// Per NY Real Property Law § 238-a:
/// Late fee cannot exceed $50 or 5% of monthly rent, whichever is less
pub fn check_late_fee_cap(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Extract rent amount
    let rent = RENT_AMOUNT_PATTERN
        .captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().replace(",", "").parse::<f64>().ok());

    // Check for dollar amount late fee
    if let Some(caps) = LATE_FEE_PATTERN.captures(text) {
        if let Some(fee_str) = caps.get(1) {
            if let Ok(fee) = fee_str.as_str().replace(",", "").parse::<f64>() {
                let max_allowed = if let Some(rent_amt) = rent {
                    (rent_amt * 0.05).min(50.0)
                } else {
                    50.0
                };

                if fee > max_allowed {
                    violations.push(Violation {
                        statute: "NY RPL § 238-a".to_string(),
                        severity: Severity::Critical,
                        message: format!(
                            "Late fee (${:.2}) exceeds maximum allowed (${:.2}). \
                             NY law caps late fees at $50 or 5% of rent, whichever is less.",
                            fee, max_allowed
                        ),
                        page: None,
                        text_snippet: None,
                        text_position: None,
                    });
                }
            }
        }
    }

    // Check for percentage late fee
    if let Some(caps) = LATE_FEE_PERCENT_PATTERN.captures(text) {
        if let Some(pct_str) = caps.get(1) {
            if let Ok(pct) = pct_str.as_str().parse::<u32>() {
                if pct > 5 {
                    violations.push(Violation {
                        statute: "NY RPL § 238-a".to_string(),
                        severity: Severity::Critical,
                        message: format!(
                            "Late fee percentage ({}%) exceeds 5% maximum allowed under NY law.",
                            pct
                        ),
                        page: None,
                        text_snippet: None,
                        text_position: None,
                    });
                }
            }
        }
    }

    violations
}

// ============================================================================
// Security Deposit Cap - RPL § 7-108
// ============================================================================

/// Check security deposit cap
///
/// Per NY Real Property Law § 7-108:
/// Security deposit cannot exceed 1 month's rent (statewide)
pub fn check_security_deposit_cap(text: &str) -> Vec<Violation> {
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

    if let (Some(deposit_amt), Some(rent_amt)) = (deposit, rent) {
        if deposit_amt > rent_amt {
            violations.push(Violation {
                statute: "NY RPL § 7-108".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit (${:.2}) exceeds 1 month's rent (${:.2}). \
                     NY law caps deposits at 1 month's rent.",
                    deposit_amt, rent_amt
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

// ============================================================================
// Good Cause Eviction - RPL § 226-c (2024+)
// ============================================================================

/// Check Good Cause Eviction disclosure
///
/// Per RPL § 226-c (effective 2024):
/// NYC and opted-in municipalities require Good Cause eviction notice
pub fn check_good_cause_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if this might be NYC (based on zip codes or explicit mention)
    let is_likely_nyc = NYC_ZIP_PATTERN.is_match(text)
        || text_lower.contains("new york city")
        || text_lower.contains("nyc")
        || text_lower.contains("manhattan")
        || text_lower.contains("brooklyn")
        || text_lower.contains("queens")
        || text_lower.contains("bronx")
        || text_lower.contains("staten island");

    if is_likely_nyc {
        // Check for Good Cause disclosure
        let has_good_cause = text_lower.contains("good cause")
            || text_lower.contains("just cause")
            || text_lower.contains("rpl 226-c")
            || text_lower.contains("housing stability");

        if !has_good_cause {
            violations.push(Violation {
                statute: "NY RPL § 226-c".to_string(),
                severity: Severity::Warning,
                message: "Good Cause Eviction disclosure recommended for NYC properties. \
                         Include Good Cause notice or exemption statement if applicable."
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
// Rent Stabilization - NYC Admin Code
// ============================================================================

/// Check rent stabilization requirements
///
/// For NYC properties built before 1974:
/// May require DHCR Lease Rider for rent stabilized units
pub fn check_rent_stabilization(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for pre-1974 building
    let is_pre_1974 = PRE_1974_PATTERN.is_match(text);

    // Check if NYC
    let is_nyc = NYC_ZIP_PATTERN.is_match(text) || text_lower.contains("new york city");

    if is_pre_1974 && is_nyc {
        // Check for rent stabilization disclosure
        let has_rent_stab = text_lower.contains("rent stabiliz")
            || text_lower.contains("dhcr")
            || text_lower.contains("division of housing")
            || text_lower.contains("rent controlled")
            || text_lower.contains("421-a")
            || text_lower.contains("j-51");

        if !has_rent_stab {
            violations.push(Violation {
                statute: "NYC Admin Code § 26-504".to_string(),
                severity: Severity::Warning,
                message: "Pre-1974 NYC building may be rent stabilized. \
                         Consider including DHCR Lease Rider if unit is rent stabilized."
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
// Lease Renewal Notice - RPL § 226-c
// ============================================================================

/// Check lease renewal notice requirements
///
/// Per NY law, landlord must provide advance notice of non-renewal:
/// - 30 days for tenancies < 1 year
/// - 60 days for tenancies 1-2 years
/// - 90 days for tenancies > 2 years
pub fn check_lease_renewal_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions renewal or non-renewal
    if text_lower.contains("renewal") || text_lower.contains("non-renewal") {
        // Check for proper notice disclosure
        let has_notice_period = text_lower.contains("30 days")
            || text_lower.contains("60 days")
            || text_lower.contains("90 days")
            || text_lower.contains("advance notice");

        if !has_notice_period {
            violations.push(Violation {
                statute: "NY RPL § 226-c".to_string(),
                severity: Severity::Info,
                message: "Consider specifying renewal notice periods. \
                         NY requires 30/60/90 day non-renewal notice based on tenancy length."
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
// Void Clauses - GOL § 5-321
// ============================================================================

/// Check for void clauses under General Obligations Law
///
/// Per NY GOL § 5-321:
/// Certain waivers and exemptions are void
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for liability waiver
    if (text_lower.contains("waive") || text_lower.contains("release"))
        && (text_lower.contains("liability") || text_lower.contains("negligence"))
    {
        violations.push(Violation {
            statute: "NY GOL § 5-321".to_string(),
            severity: Severity::Critical,
            message: "Landlord liability waiver is void. NY law prohibits agreements \
                     exempting landlord from liability for negligence."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "NY CPLR § 3218".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses in residential leases are void.".to_string(),
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
    // Late Fee Cap Tests
    // ========================================================================

    #[test]
    fn test_detects_excessive_late_fee_dollar() {
        let text = "Monthly rent: $2,000. Late fee: $100 if rent not received by 5th.";
        let violations = check_new_york_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("238-a") && v.severity == Severity::Critical),
            "Should detect late fee exceeding $50 cap"
        );
    }

    #[test]
    fn test_detects_excessive_late_fee_percent() {
        let text = "Monthly rent: $1,500. Late fee shall be 10% of monthly rent.";
        let violations = check_new_york_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("238-a") && v.message.contains("10%")),
            "Should detect late fee exceeding 5% cap"
        );
    }

    #[test]
    fn test_accepts_compliant_late_fee() {
        let text = "Monthly rent: $2,000. Late fee: $50 if rent not paid by 5th.";
        let violations = check_new_york_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("238-a") && v.severity == Severity::Critical),
            "Should accept $50 late fee"
        );
    }

    #[test]
    fn test_accepts_5_percent_late_fee() {
        let text = "Monthly rent: $1,000. Late fee: 5% of rent.";
        let violations = check_new_york_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("238-a") && v.severity == Severity::Critical),
            "Should accept 5% late fee"
        );
    }

    // ========================================================================
    // Security Deposit Cap Tests
    // ========================================================================

    #[test]
    fn test_detects_excessive_deposit() {
        let text = "Monthly rent: $2,000. Security deposit: $4,000.";
        let violations = check_new_york_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("7-108") && v.severity == Severity::Critical),
            "Should detect deposit exceeding 1 month rent"
        );
    }

    #[test]
    fn test_accepts_one_month_deposit() {
        let text = "Monthly rent: $2,500. Security deposit: $2,500.";
        let violations = check_new_york_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("7-108") && v.severity == Severity::Critical),
            "Should accept deposit equal to 1 month rent"
        );
    }

    // ========================================================================
    // Good Cause Eviction Tests
    // ========================================================================

    #[test]
    fn test_warns_nyc_without_good_cause() {
        let text = "Property located at 123 Main St, New York City, NY 10001. \
                    Landlord may terminate for any lease violation.";
        let violations = check_new_york_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("226-c") && v.message.contains("Good Cause")),
            "Should warn about missing Good Cause disclosure for NYC"
        );
    }

    #[test]
    fn test_accepts_good_cause_disclosure() {
        let text = "Property in Brooklyn, NY 11201. \
                    This lease is subject to Good Cause eviction requirements under RPL 226-c.";
        let violations = check_new_york_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("226-c") && v.severity == Severity::Critical),
            "Should accept Good Cause disclosure"
        );
    }

    // ========================================================================
    // Rent Stabilization Tests
    // ========================================================================

    #[test]
    fn test_warns_pre1974_nyc_no_stabilization_info() {
        let text = "Building constructed in 1965. Property in NYC 10021.";
        let violations = check_new_york_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("26-504") || v.message.contains("rent stabilized")),
            "Should warn about potential rent stabilization for pre-1974 NYC building"
        );
    }

    #[test]
    fn test_accepts_rent_stabilization_disclosure() {
        let text = "Building built in 1960. NYC 10001. \
                    This unit is rent stabilized. DHCR Lease Rider attached.";
        let violations = check_new_york_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("26-504") && v.severity == Severity::Critical),
            "Should accept rent stabilization disclosure"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_liability_waiver() {
        let text =
            "Tenant agrees to waive any claims of liability against landlord for negligence.";
        let violations = check_new_york_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("5-321") && v.severity == Severity::Critical),
            "Should detect void liability waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for any unpaid rent.";
        let violations = check_new_york_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("3218") && v.severity == Severity::Critical),
            "Should detect void confession of judgment"
        );
    }

    // ========================================================================
    // Compliant Lease Test
    // ========================================================================

    #[test]
    fn test_compliant_new_york_lease() {
        let text = "Monthly rent: $2,000. Security deposit: $2,000. \
                    Late fee: $50 if payment received after 5th of month. \
                    This lease is subject to Good Cause eviction requirements. \
                    Landlord maintains property per warranty of habitability. \
                    Non-renewal notice: 60 days advance notice required.";
        let violations = check_new_york_compliance(text);

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
