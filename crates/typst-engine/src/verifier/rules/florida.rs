//! Florida-specific compliance rules for residential leases
//!
//! Implements compliance checks for Florida Statutes Chapter 83 (Landlord-Tenant Law)
//! and federal regulations applicable to Florida residential leases.

use super::{ComplianceRule, RuleResult, Severity};
use crate::verifier::parser::ParsedLease;
use once_cell::sync::Lazy;
use regex::Regex;

// ============================================================================
// DISCLOSURE RULES
// ============================================================================

/// Radon Gas Disclosure Rule (F.S. § 404.056)
///
/// Florida law MANDATES that all residential leases include a radon gas
/// disclosure notification. This is CRITICAL for lease enforceability.
///
/// The disclosure must contain specific statutory language about radon being
/// a "naturally occurring radioactive gas."
pub struct RadonDisclosureRule;

impl ComplianceRule for RadonDisclosureRule {
    fn name(&self) -> &str {
        "Radon Gas Disclosure"
    }

    fn statute_reference(&self) -> &str {
        "F.S. § 404.056"
    }

    fn check(&self, lease: &ParsedLease) -> RuleResult {
        // Combine all section content into one text for searching
        let full_text = lease
            .sections
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let full_text_lower = full_text.to_lowercase();

        // Check for key radon disclosure language
        let has_radon_mention = full_text_lower.contains("radon");
        let has_radioactive = full_text_lower.contains("radioactive");
        let has_gas = full_text_lower.contains("gas");

        // The disclosure must contain language about radon being a radioactive gas
        if has_radon_mention && has_radioactive && has_gas {
            RuleResult::Pass
        } else if has_radon_mention {
            RuleResult::Fail {
                reason: "Radon disclosure found but does not contain required language about 'naturally occurring radioactive gas'".to_string(),
                severity: Severity::Critical,
            }
        } else {
            RuleResult::Fail {
                reason: "Missing required radon gas disclosure. Florida law (F.S. § 404.056) mandates radon notification in all residential leases.".to_string(),
                severity: Severity::Critical,
            }
        }
    }
}

/// Security Deposit Disclosure Rule (F.S. § 83.49)
///
/// If a security deposit is collected, Florida law requires specific disclosures:
/// 1. The manner in which the deposit is being held (separate account, surety bond, etc.)
/// 2. The name and address of the depository (bank)
/// 3. The return timeline (within 15-60 days)
pub struct SecurityDepositDisclosureRule;

impl ComplianceRule for SecurityDepositDisclosureRule {
    fn name(&self) -> &str {
        "Security Deposit Disclosure"
    }

    fn statute_reference(&self) -> &str {
        "F.S. § 83.49"
    }

    fn check(&self, lease: &ParsedLease) -> RuleResult {
        // Only applicable if there's a security deposit
        if let Some(deposit) = lease.financial.security_deposit {
            if deposit <= 0.0 {
                return RuleResult::NotApplicable;
            }
        } else {
            return RuleResult::NotApplicable;
        }

        let full_text = lease
            .sections
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let full_text_lower = full_text.to_lowercase();

        // Check for required disclosure elements
        let has_deposit_method = full_text_lower.contains("separate account")
            || full_text_lower.contains("surety bond")
            || full_text_lower.contains("manner in which")
            || full_text_lower.contains("held in");

        let has_bank_info = full_text_lower.contains("bank")
            || full_text_lower.contains("depository")
            || full_text_lower.contains("financial institution");

        let has_return_timeline = full_text_lower.contains("15 days")
            || full_text_lower.contains("fifteen days")
            || full_text_lower.contains("30 days")
            || full_text_lower.contains("thirty days")
            || full_text_lower.contains("60 days")
            || full_text_lower.contains("sixty days");

        let mut missing_elements = Vec::new();

        if !has_deposit_method {
            missing_elements.push("deposit holding method (separate account, surety bond, etc.)");
        }
        if !has_bank_info {
            missing_elements.push("bank/depository name and address");
        }
        if !has_return_timeline {
            missing_elements.push("deposit return timeline (within 15-60 days)");
        }

        if missing_elements.is_empty() {
            RuleResult::Pass
        } else {
            RuleResult::Fail {
                reason: format!(
                    "Security deposit disclosure incomplete. Missing: {}",
                    missing_elements.join(", ")
                ),
                severity: Severity::Critical,
            }
        }
    }
}

/// Lead-Based Paint Disclosure Rule (24 CFR Part 35)
///
/// Federal law requires lead-based paint disclosure for properties built before 1978.
/// The disclosure must include EPA pamphlet acknowledgment and notice of any known
/// lead-based paint hazards.
pub struct LeadPaintDisclosureRule;

impl ComplianceRule for LeadPaintDisclosureRule {
    fn name(&self) -> &str {
        "Lead-Based Paint Disclosure"
    }

    fn statute_reference(&self) -> &str {
        "24 CFR Part 35"
    }

    fn check(&self, lease: &ParsedLease) -> RuleResult {
        // Check if property was built before 1978
        let needs_disclosure = if let Some(year) = lease.property.year_built {
            year < 1978
        } else {
            // If year not specified, we should warn that it might be needed
            true
        };

        if !needs_disclosure {
            return RuleResult::NotApplicable;
        }

        let full_text = lease
            .sections
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let full_text_lower = full_text.to_lowercase();

        // Check for lead paint disclosure language
        let has_lead_disclosure = full_text_lower.contains("lead-based paint")
            || full_text_lower.contains("lead based paint")
            || full_text_lower.contains("lead paint");

        let has_epa_pamphlet = full_text_lower.contains("epa")
            || full_text_lower.contains("environmental protection agency")
            || full_text_lower.contains("protect your family");

        if has_lead_disclosure && has_epa_pamphlet {
            RuleResult::Pass
        } else if has_lead_disclosure {
            RuleResult::Warning {
                reason:
                    "Lead paint disclosure found but EPA pamphlet acknowledgment may be incomplete"
                        .to_string(),
            }
        } else if lease.property.year_built.is_none() {
            RuleResult::Warning {
                reason: "Property year built not specified. If built before 1978, lead-based paint disclosure is required by federal law (24 CFR Part 35).".to_string(),
            }
        } else {
            RuleResult::Fail {
                reason: "Property built before 1978 requires lead-based paint disclosure and EPA pamphlet acknowledgment (24 CFR Part 35).".to_string(),
                severity: Severity::High,
            }
        }
    }
}

// ============================================================================
// PROHIBITED TERMS RULES
// ============================================================================

/// Prohibited Terms Rule (F.S. § 83.47)
///
/// Florida law prohibits certain provisions in residential leases:
/// - Cannot waive tenant's rights under Chapter 83
/// - Cannot authorize confession of judgment
/// - Cannot waive right to jury trial
/// - Cannot make tenant liable for attorney fees regardless of outcome
///
/// These provisions are void even if included in the lease.
static PROHIBITED_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        (
            Regex::new(r"(?i)waive.*right.*jury").unwrap(),
            "Waiver of jury trial rights",
        ),
        (
            Regex::new(r"(?i)confession.*judgment").unwrap(),
            "Confession of judgment clause",
        ),
        (
            Regex::new(r"(?i)waive.*rights.*chapter\s*83").unwrap(),
            "Waiver of Chapter 83 rights",
        ),
        (
            Regex::new(r"(?i)tenant.*liable.*attorney.*fees.*regardless").unwrap(),
            "Improper attorney fee clause (tenant liable regardless of outcome)",
        ),
        (
            Regex::new(r"(?i)waive.*right.*notice").unwrap(),
            "Waiver of notice rights",
        ),
        (
            Regex::new(r"(?i)waive.*right.*habitability").unwrap(),
            "Waiver of habitability rights",
        ),
        (
            Regex::new(r"(?i)waive.*statutory.*rights").unwrap(),
            "Waiver of statutory rights",
        ),
    ]
});

pub struct ProhibitedTermsRule;

impl ComplianceRule for ProhibitedTermsRule {
    fn name(&self) -> &str {
        "Prohibited Terms Detector"
    }

    fn statute_reference(&self) -> &str {
        "F.S. § 83.47"
    }

    fn check(&self, lease: &ParsedLease) -> RuleResult {
        let full_text = lease
            .sections
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let mut violations = Vec::new();

        for (pattern, description) in PROHIBITED_PATTERNS.iter() {
            if pattern.is_match(&full_text) {
                violations.push(*description);
            }
        }

        if violations.is_empty() {
            RuleResult::Pass
        } else {
            RuleResult::Fail {
                reason: format!(
                    "Lease contains prohibited provisions (void under F.S. § 83.47): {}",
                    violations.join("; ")
                ),
                severity: Severity::Critical,
            }
        }
    }
}

// ============================================================================
// NUMERIC VALIDATION RULES
// ============================================================================

/// Security Deposit Limit Rule
///
/// Florida has no statutory limit on security deposits, but best practice
/// is typically 1-2 months rent. We flag deposits exceeding 3x monthly rent
/// as a warning.
pub struct SecurityDepositLimitRule;

impl ComplianceRule for SecurityDepositLimitRule {
    fn name(&self) -> &str {
        "Security Deposit Reasonableness"
    }

    fn statute_reference(&self) -> &str {
        "Best Practice (No FL statutory limit)"
    }

    fn check(&self, lease: &ParsedLease) -> RuleResult {
        if let (Some(deposit), Some(rent)) = (
            lease.financial.security_deposit,
            lease.financial.monthly_rent,
        ) {
            if deposit <= 0.0 || rent <= 0.0 {
                return RuleResult::NotApplicable;
            }

            let ratio = deposit / rent;

            if ratio > 3.0 {
                RuleResult::Warning {
                    reason: format!(
                        "Security deposit (${:.2}) exceeds 3x monthly rent (${:.2}). Ratio: {:.1}x. \
                        While Florida has no statutory limit, this may be excessive.",
                        deposit, rent, ratio
                    ),
                }
            } else if ratio > 2.0 {
                RuleResult::Warning {
                    reason: format!(
                        "Security deposit (${:.2}) exceeds 2x monthly rent (${:.2}). Ratio: {:.1}x. \
                        Consider if this is reasonable for your market.",
                        deposit, rent, ratio
                    ),
                }
            } else {
                RuleResult::Pass
            }
        } else {
            RuleResult::NotApplicable
        }
    }
}

/// Late Fee Rule
///
/// Florida law requires late fees to be "reasonable." Best practice is
/// typically 5% or less of monthly rent, with a reasonable grace period.
pub struct LateFeeRule;

impl ComplianceRule for LateFeeRule {
    fn name(&self) -> &str {
        "Late Fee Reasonableness"
    }

    fn statute_reference(&self) -> &str {
        "F.S. § 83.56 (Reasonableness requirement)"
    }

    fn check(&self, lease: &ParsedLease) -> RuleResult {
        if let (Some(late_fee), Some(rent)) =
            (lease.financial.late_fee, lease.financial.monthly_rent)
        {
            if late_fee <= 0.0 || rent <= 0.0 {
                return RuleResult::NotApplicable;
            }

            let percentage = (late_fee / rent) * 100.0;

            if percentage > 10.0 {
                RuleResult::Fail {
                    reason: format!(
                        "Late fee (${:.2}) is {:.1}% of monthly rent (${:.2}). \
                        Florida law requires late fees to be reasonable. Fees exceeding 10% \
                        may be challenged as unreasonable.",
                        late_fee, percentage, rent
                    ),
                    severity: Severity::Medium,
                }
            } else if percentage > 5.0 {
                RuleResult::Warning {
                    reason: format!(
                        "Late fee (${:.2}) is {:.1}% of monthly rent (${:.2}). \
                        Best practice is typically 5% or less. Verify this is reasonable for your jurisdiction.",
                        late_fee, percentage, rent
                    ),
                }
            } else {
                RuleResult::Pass
            }
        } else {
            RuleResult::NotApplicable
        }

        // TODO: Also check for grace period requirement
        // Florida doesn't mandate a specific grace period, but it's best practice
        // and some local ordinances may require it
    }
}

/// Grace Period Rule
///
/// While Florida law doesn't mandate a specific grace period for rent,
/// many local ordinances do, and it's considered best practice.
/// This rule checks if a grace period is mentioned.
pub struct GracePeriodRule;

impl ComplianceRule for GracePeriodRule {
    fn name(&self) -> &str {
        "Grace Period Disclosure"
    }

    fn statute_reference(&self) -> &str {
        "Best Practice (Check local ordinances)"
    }

    fn check(&self, lease: &ParsedLease) -> RuleResult {
        let full_text = lease
            .sections
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let full_text_lower = full_text.to_lowercase();

        // Check for grace period language
        let has_grace_period = full_text_lower.contains("grace period")
            || full_text_lower.contains("days after due date")
            || (full_text_lower.contains("late fee")
                && (full_text_lower.contains("after") || full_text_lower.contains("within")));

        if has_grace_period {
            RuleResult::Pass
        } else {
            RuleResult::Warning {
                reason: "No grace period mentioned for late rent. While not required by Florida state law, \
                        many local ordinances require grace periods (typically 3-5 days), and it's considered \
                        best practice. Verify local requirements.".to_string(),
            }
        }
    }
}

// ============================================================================
// ADDITIONAL FLORIDA-SPECIFIC RULES
// ============================================================================

/// Bed Bug Disclosure Rule (F.S. § 83.50)
///
/// Landlords must disclose known bed bug infestation history.
pub struct BedBugDisclosureRule;

impl ComplianceRule for BedBugDisclosureRule {
    fn name(&self) -> &str {
        "Bed Bug Disclosure"
    }

    fn statute_reference(&self) -> &str {
        "F.S. § 83.50"
    }

    fn check(&self, lease: &ParsedLease) -> RuleResult {
        let full_text = lease
            .sections
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let full_text_lower = full_text.to_lowercase();

        // Check for bed bug disclosure
        let has_bed_bug_disclosure = full_text_lower.contains("bed bug")
            || full_text_lower.contains("bedbug")
            || full_text_lower.contains("infestation");

        if has_bed_bug_disclosure {
            RuleResult::Pass
        } else {
            RuleResult::Warning {
                reason:
                    "No bed bug disclosure found. While only required if there's known infestation \
                        history, best practice is to include a disclosure statement (F.S. § 83.50)."
                        .to_string(),
            }
        }
    }
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Get all Florida-specific compliance rules
///
/// Returns a vector of boxed trait objects implementing ComplianceRule.
/// These rules check for compliance with Florida Statutes Chapter 83
/// and applicable federal regulations.
pub fn get_all_rules() -> Vec<Box<dyn ComplianceRule>> {
    vec![
        // Critical disclosure rules
        Box::new(RadonDisclosureRule),
        Box::new(SecurityDepositDisclosureRule),
        Box::new(LeadPaintDisclosureRule),
        // Prohibited terms
        Box::new(ProhibitedTermsRule),
        // Numeric validation rules
        Box::new(SecurityDepositLimitRule),
        Box::new(LateFeeRule),
        Box::new(GracePeriodRule),
        // Additional disclosures
        Box::new(BedBugDisclosureRule),
    ]
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::parser::{FinancialTerms, LeaseSection, PropertyInfo};

    fn create_test_lease(sections_text: Vec<&str>, financial: FinancialTerms) -> ParsedLease {
        ParsedLease {
            sections: sections_text
                .iter()
                .enumerate()
                .map(|(i, text)| LeaseSection {
                    title: format!("Section {}", i),
                    number: Some(i.to_string()),
                    content: text.to_string(),
                    start_line: i * 10,
                    end_line: (i + 1) * 10,
                })
                .collect(),
            parties: Default::default(),
            property: Default::default(),
            financial,
            dates: Default::default(),
            addenda: vec![],
            unknown_sections: vec![],
        }
    }

    #[test]
    fn test_radon_disclosure_pass() {
        let lease = create_test_lease(
            vec!["Radon is a naturally occurring radioactive gas."],
            FinancialTerms::default(),
        );

        let rule = RadonDisclosureRule;
        let result = rule.check(&lease);

        assert!(result.is_pass());
    }

    #[test]
    fn test_radon_disclosure_fail() {
        let lease = create_test_lease(
            vec!["This is a lease agreement."],
            FinancialTerms::default(),
        );

        let rule = RadonDisclosureRule;
        let result = rule.check(&lease);

        assert!(result.is_fail());
    }

    #[test]
    fn test_security_deposit_limit_warning() {
        let lease = create_test_lease(
            vec!["Security deposit rules."],
            FinancialTerms {
                monthly_rent: Some(1000.0),
                security_deposit: Some(3500.0),
                late_fee: None,
                late_fee_grace_period: None,
                pet_deposit: None,
            },
        );

        let rule = SecurityDepositLimitRule;
        let result = rule.check(&lease);

        assert!(result.is_warning());
    }

    #[test]
    fn test_late_fee_excessive() {
        let lease = create_test_lease(
            vec!["Late fee policy."],
            FinancialTerms {
                monthly_rent: Some(1000.0),
                security_deposit: None,
                late_fee: Some(150.0), // 15% - excessive
                late_fee_grace_period: None,
                pet_deposit: None,
            },
        );

        let rule = LateFeeRule;
        let result = rule.check(&lease);

        assert!(result.is_fail());
    }

    #[test]
    fn test_prohibited_terms_jury_waiver() {
        let lease = create_test_lease(
            vec!["Tenant hereby waives all rights to jury trial."],
            FinancialTerms::default(),
        );

        let rule = ProhibitedTermsRule;
        let result = rule.check(&lease);

        assert!(result.is_fail());
    }

    #[test]
    fn test_lead_paint_not_applicable() {
        let mut lease = create_test_lease(vec!["Modern building."], FinancialTerms::default());

        lease.property = PropertyInfo {
            address: None,
            unit: None,
            property_type: None,
            year_built: Some(2000),
        };

        let rule = LeadPaintDisclosureRule;
        let result = rule.check(&lease);

        assert_eq!(result, RuleResult::NotApplicable);
    }
}
