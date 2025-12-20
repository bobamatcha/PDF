//! Local Layer - Municipal/County Ordinance Compliance
//!
//! Layer 3 of the Layer Cake: Local overrides that supersede state law.
//! These are triggered by locality (from ZIP code or explicit selection).
//!
//! Key localities:
//! - Chicago (IL): RLTO Summary, deposit interest, bed bug disclosure
//! - NYC (NY): Rent stabilization, good cause eviction, 1-month deposit cap
//! - San Francisco (CA): Rent ordinance, just cause eviction
//! - Los Angeles (CA): RSO, relocation assistance
//! - Seattle (WA): Just cause eviction, first-in-time

use crate::jurisdiction::{Jurisdiction, Locality};
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

/// Check local/municipal compliance requirements
///
/// This is Layer 3 of the Layer Cake - local ordinances that override state defaults.
/// Only runs if jurisdiction has a locality set.
pub fn check_local_compliance(jurisdiction: &Jurisdiction, text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Only check if locality is specified
    if let Some(ref locality) = jurisdiction.locality {
        match locality {
            Locality::Chicago => {
                violations.extend(check_chicago_rlto(text));
            }
            Locality::NewYorkCity => {
                violations.extend(check_nyc_requirements(text));
            }
            Locality::SanFrancisco => {
                violations.extend(check_sf_requirements(text));
            }
            Locality::LosAngeles => {
                violations.extend(check_la_requirements(text));
            }
            Locality::SantaMonica => {
                violations.extend(check_santa_monica_requirements(text));
            }
            Locality::WestHollywood => {
                // Similar to LA RSO
                violations.extend(check_la_requirements(text));
            }
            Locality::Oakland | Locality::Berkeley => {
                // Bay Area rent control cities
                violations.extend(check_bay_area_rent_control(text));
            }
            Locality::WashingtonDC => {
                violations.extend(check_dc_requirements(text));
            }
            Locality::Custom(_) => {
                // Custom localities - no specific rules
            }
        }
    }

    violations
}

// ============================================================================
// Chicago RLTO (Residential Landlord and Tenant Ordinance)
// ============================================================================

/// Check Chicago RLTO requirements
///
/// Per Chicago Municipal Code 5-12:
/// - RLTO Summary attachment required (5-12-170)
/// - Security deposit interest (5-12-080)
/// - Bed bug disclosure (5-12-100)
fn check_chicago_rlto(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // RLTO Summary requirement
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

    // Deposit interest disclosure
    let has_deposit = text_lower.contains("deposit") || text_lower.contains("security");
    if has_deposit {
        let has_interest_disclosure = text_lower.contains("interest")
            || text_lower.contains("5-12-080")
            || text_lower.contains("deposit interest");

        if !has_interest_disclosure {
            violations.push(Violation {
                statute: "Chicago Mun. Code § 5-12-080".to_string(),
                severity: Severity::Warning,
                message: "Security deposit interest disclosure required for Chicago. \
                         Landlord must pay interest and provide receipt within 14 days."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Bed bug disclosure
    let has_bed_bug_disclosure = text_lower.contains("bed bug")
        || text_lower.contains("bedbug")
        || text_lower.contains("5-12-100");

    if !has_bed_bug_disclosure {
        violations.push(Violation {
            statute: "Chicago Mun. Code § 5-12-100".to_string(),
            severity: Severity::Warning,
            message: "Bed bug disclosure required for Chicago properties. \
                     Must attach city-provided bed bug brochure."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// New York City Requirements
// ============================================================================

/// Check NYC-specific requirements
///
/// Per NYC Admin Code and NY Housing Stability Act:
/// - Security deposit cap (1 month max under HSTPA)
/// - Good Cause Eviction rider (2024+)
/// - Rent Stabilization (pre-1974 buildings)
fn check_nyc_requirements(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check security deposit cap (NY RPL 7-108: 1 month max statewide, but NYC enforces strictly)
    let deposit = DEPOSIT_AMOUNT_PATTERN
        .captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().replace(",", "").parse::<f64>().ok());

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
                    "Security deposit (${:.2}) exceeds NYC limit of 1 month's rent (${:.2}). \
                     Under Housing Stability and Tenant Protection Act, deposits are capped at 1 month.",
                    deposit_amt, rent_amt
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Good Cause Eviction (2024+)
    let has_good_cause_notice = text_lower.contains("good cause")
        || text_lower.contains("226-c")
        || text_lower.contains("just cause");

    if !has_good_cause_notice {
        violations.push(Violation {
            statute: "NY RPL § 226-c".to_string(),
            severity: Severity::Info,
            message: "Consider including Good Cause Eviction notice for NYC properties. \
                     Required for buildings with 10+ units unless exempt."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Rent Stabilization notice
    let has_rent_stab = text_lower.contains("rent stabiliz")
        || text_lower.contains("dhcr")
        || text_lower.contains("26-504");

    if !has_rent_stab {
        violations.push(Violation {
            statute: "NYC Admin Code § 26-504".to_string(),
            severity: Severity::Info,
            message: "For NYC properties built before 1974, verify if rent stabilization applies. \
                     DHCR Lease Rider may be required."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// San Francisco Requirements
// ============================================================================

/// Check San Francisco-specific requirements
fn check_sf_requirements(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // SF Rent Ordinance
    let has_rent_ordinance = text_lower.contains("rent ordinance")
        || text_lower.contains("rent board")
        || text_lower.contains("sf rent");

    if !has_rent_ordinance {
        violations.push(Violation {
            statute: "SF Admin Code Ch. 37".to_string(),
            severity: Severity::Info,
            message: "San Francisco Rent Ordinance may apply. Check if property is covered \
                     and include required disclosures."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Los Angeles Requirements
// ============================================================================

/// Check Los Angeles RSO requirements
fn check_la_requirements(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // LA RSO disclosure
    let has_rso = text_lower.contains("rent stabilization ordinance")
        || text_lower.contains("rso")
        || text_lower.contains("lahd");

    if !has_rso {
        violations.push(Violation {
            statute: "LAMC § 151.00 et seq.".to_string(),
            severity: Severity::Info,
            message:
                "Los Angeles RSO may apply. Check if property is subject to rent stabilization \
                     and include required disclosures."
                    .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Santa Monica Requirements
// ============================================================================

/// Check Santa Monica Rent Control requirements
fn check_santa_monica_requirements(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let has_rent_control =
        text_lower.contains("rent control") || text_lower.contains("santa monica rent");

    if !has_rent_control {
        violations.push(Violation {
            statute: "Santa Monica Mun. Code Ch. 4.36".to_string(),
            severity: Severity::Info,
            message: "Santa Monica Rent Control may apply. Verify exemption status and include \
                     required disclosures if covered."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Bay Area Rent Control (Oakland, Berkeley)
// ============================================================================

/// Check Bay Area rent control requirements
fn check_bay_area_rent_control(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let has_rent_control = text_lower.contains("rent control")
        || text_lower.contains("rent board")
        || text_lower.contains("just cause");

    if !has_rent_control {
        violations.push(Violation {
            statute: "Local Rent Control Ordinance".to_string(),
            severity: Severity::Info,
            message: "Bay Area rent control may apply. Check if property is covered and include \
                     required just cause eviction disclosures."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Washington DC Requirements
// ============================================================================

/// Check Washington DC requirements
fn check_dc_requirements(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // DC Rental Housing Act
    let has_rental_act = text_lower.contains("rental housing act")
        || text_lower.contains("d.c. official code")
        || text_lower.contains("rent control");

    if !has_rental_act {
        violations.push(Violation {
            statute: "D.C. Code § 42-3501 et seq.".to_string(),
            severity: Severity::Info,
            message:
                "DC Rental Housing Act may apply. Check if property is subject to rent control \
                     and include required disclosures."
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
    use crate::jurisdiction::State;

    // ========================================================================
    // Chicago RLTO Tests
    // ========================================================================

    #[test]
    fn test_chicago_requires_rlto_summary() {
        let jurisdiction = Jurisdiction::with_locality(State::IL, Locality::Chicago);
        let text = "Monthly rent: $2,000. Security deposit: $2,000.";

        let violations = check_local_compliance(&jurisdiction, text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("5-12-170") && v.message.contains("RLTO")),
            "Should require RLTO Summary for Chicago locality"
        );
    }

    #[test]
    fn test_chicago_accepts_rlto_summary() {
        let jurisdiction = Jurisdiction::with_locality(State::IL, Locality::Chicago);
        let text = "Monthly rent: $2,000. RLTO Summary attached. \
                    Bed bug disclosure provided. Deposit earns interest.";

        let violations = check_local_compliance(&jurisdiction, text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("5-12-170") && v.severity == Severity::Critical),
            "Should accept lease with RLTO summary"
        );
    }

    #[test]
    fn test_no_local_rules_without_locality() {
        let jurisdiction = Jurisdiction::new(State::IL);
        let text = "Monthly rent: $2,000. Security deposit: $2,000.";

        let violations = check_local_compliance(&jurisdiction, text);

        assert!(
            violations.is_empty(),
            "Should not apply local rules without locality"
        );
    }

    // ========================================================================
    // NYC Tests
    // ========================================================================

    #[test]
    fn test_nyc_detects_excessive_deposit() {
        let jurisdiction = Jurisdiction::with_locality(State::NY, Locality::NewYorkCity);
        let text = "Monthly rent: $3,000. Security deposit: $3,500.";

        let violations = check_local_compliance(&jurisdiction, text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("7-108") && v.severity == Severity::Critical),
            "Should detect deposit exceeding 1 month rent in NYC"
        );
    }

    #[test]
    fn test_nyc_accepts_valid_deposit() {
        let jurisdiction = Jurisdiction::with_locality(State::NY, Locality::NewYorkCity);
        let text = "Monthly rent: $3,000. Security deposit: $3,000. \
                    Good Cause eviction notice attached. DHCR rider included.";

        let violations = check_local_compliance(&jurisdiction, text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("7-108") && v.severity == Severity::Critical),
            "Should accept deposit equal to 1 month rent"
        );
    }

    // ========================================================================
    // San Francisco Tests
    // ========================================================================

    #[test]
    fn test_sf_warns_missing_rent_ordinance() {
        let jurisdiction = Jurisdiction::with_locality(State::CA, Locality::SanFrancisco);
        let text = "Monthly rent: $4,000.";

        let violations = check_local_compliance(&jurisdiction, text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("Ch. 37") && v.message.contains("Rent Ordinance")),
            "Should warn about SF Rent Ordinance"
        );
    }
}
