//! Florida Residential Landlord-Tenant Law Compliance
//!
//! Florida Statutes Chapter 83, Part II (§ 83.40-83.682)
//! "Florida Residential Landlord and Tenant Act"

use crate::extractors::numeric::{
    extract_days_near_deposit_return, has_bank_location, has_claim_context,
};
use crate::patterns::{
    contains_semantic_cluster, extract_snippet, find_text_position, AS_IS_KEYWORDS,
    DISPOSAL_KEYWORDS, FL_LAW_KEYWORDS, NOTICE_KEYWORDS, PROPERTY_KEYWORDS, RIGHTS_KEYWORDS,
    STRUCTURAL_KEYWORDS, TENANT_KEYWORDS, TERMINATION_KEYWORDS, WAIVER_KEYWORDS,
};
use regex::Regex;
use shared_types::{Severity, TextPosition, Violation};

/// Check all Florida-specific compliance requirements
pub fn check_florida_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_prohibited_provisions(text));
    violations.extend(check_security_deposit(text));
    violations.extend(check_attorney_fees(text));
    violations.extend(check_notice_requirements(text));

    violations
}

// ============================================================================
// § 83.47 - Prohibited Provisions
// ============================================================================

/// Check for prohibited provisions under Florida Statute § 83.47
pub fn check_prohibited_provisions(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of notice (§ 83.47(1)(c))
    if check_waiver_of_notice(&text_lower) {
        let text_position = find_text_position(text, "waive").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });
        violations.push(Violation {
            statute: "F.S. § 83.47(1)(a)".to_string(),
            severity: Severity::Critical,
            message: "Lease contains prohibited waiver of tenant's right to notice before termination or eviction".to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "waive")),
            text_position,
        });
    }

    // Check for property disposal clause (§ 83.47(1)(b))
    if check_property_disposal(&text_lower) {
        let text_position = find_text_position(text, "dispose").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });
        violations.push(Violation {
            statute: "F.S. § 83.47(1)(b)".to_string(),
            severity: Severity::Critical,
            message: "Lease contains prohibited authorization for landlord to dispose of tenant's property".to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "dispose")),
            text_position,
        });
    }

    // Check for AS-IS clauses that waive landlord's structural obligations (§ 83.51)
    if check_as_is_structural(&text_lower) {
        let text_position = find_text_position(text, "as-is")
            .or_else(|| find_text_position(text, "as is"))
            .map(|(start, end)| TextPosition {
                start_offset: start,
                end_offset: end,
            });
        violations.push(Violation {
            statute: "F.S. § 83.51(2)(a)".to_string(),
            severity: Severity::Critical,
            message: "AS-IS clause may improperly waive landlord's obligation to maintain structural components".to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "as-is")),
            text_position,
        });
    }

    // Check for general waiver of rights (§ 83.47(1)(a))
    if check_general_rights_waiver(&text_lower) {
        let text_position = find_text_position(text, "waive").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });
        violations.push(Violation {
            statute: "F.S. § 83.47(1)(a)".to_string(),
            severity: Severity::Critical,
            message: "Lease contains prohibited waiver of tenant's rights under Florida landlord-tenant law".to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "waive")),
            text_position,
        });
    }

    violations
}

fn check_waiver_of_notice(text: &str) -> bool {
    contains_semantic_cluster(
        text,
        &[WAIVER_KEYWORDS, NOTICE_KEYWORDS, TERMINATION_KEYWORDS],
    )
}

fn check_property_disposal(text: &str) -> bool {
    let has_disposal = DISPOSAL_KEYWORDS.iter().any(|kw| text.contains(kw));
    let has_property = PROPERTY_KEYWORDS.iter().any(|kw| text.contains(kw));
    let has_tenant_context =
        TENANT_KEYWORDS.iter().any(|kw| text.contains(kw)) || text.contains("left by");

    has_disposal && has_property && has_tenant_context
}

fn check_as_is_structural(text: &str) -> bool {
    let has_as_is = AS_IS_KEYWORDS.iter().any(|kw| text.contains(kw));
    let has_structural = STRUCTURAL_KEYWORDS.iter().any(|kw| text.contains(kw));

    has_as_is && has_structural
}

fn check_general_rights_waiver(text: &str) -> bool {
    let has_waiver = WAIVER_KEYWORDS.iter().any(|kw| text.contains(kw));
    let has_rights = RIGHTS_KEYWORDS.iter().any(|kw| text.contains(kw));
    let has_law_ref = FL_LAW_KEYWORDS.iter().any(|kw| text.contains(kw));

    has_waiver && has_rights && (has_law_ref || text.contains("all"))
}

// ============================================================================
// § 83.49 - Security Deposits
// ============================================================================

/// Validates security deposit return timelines and requirements
pub fn check_security_deposit(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Check for deposit return period
    if let Some(days) = extract_days_near_deposit_return(text) {
        let has_claim = has_claim_context(text);

        // 15-day rule for no claim (§ 83.49(3)(a))
        if days > 15 && !has_claim {
            violations.push(Violation {
                statute: "F.S. § 83.49(3)(a)".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit must be returned within 15 days if no claim is made (found: {} days)",
                    days
                ),
                page: None,
                text_snippet: Some(text.chars().take(100).collect()),
                text_position: None,
            });
        }

        // 30-day ambiguous case
        if days == 30 && !has_claim {
            violations.push(Violation {
                statute: "F.S. § 83.49(3)(b)".to_string(),
                severity: Severity::Warning,
                message: "30-day deposit return period found without clear claim context. Florida law requires 15 days if no claim, or notice within 30 days if claiming deductions.".to_string(),
                page: None,
                text_snippet: Some(text.chars().take(100).collect()),
                text_position: None,
            });
        }

        // Over 30 days is always a violation (§ 83.49(3)(b))
        if days > 30 {
            violations.push(Violation {
                statute: "F.S. § 83.49(3)(b)".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Deposit notice period exceeds statutory maximum of 30 days (found: {} days)",
                    days
                ),
                page: None,
                text_snippet: Some(text.chars().take(100).collect()),
                text_position: None,
            });
        }
    }

    // Check for bank location requirement (§ 83.49(2))
    let text_lower = text.to_lowercase();
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit && !has_bank_location(text) {
        violations.push(Violation {
            statute: "F.S. § 83.49(2)".to_string(),
            severity: Severity::Warning,
            message: "Lease should specify the name and address of the Florida banking institution where the security deposit is held, or evidence of surety bond.".to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// § 83.48 - Attorney Fees Reciprocity
// ============================================================================

/// Checks attorney fee clauses for reciprocity per Florida Statute § 83.48
pub fn check_attorney_fees(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for compliant reciprocal clauses first
    let has_prevailing_party = text_lower.contains("prevailing party");
    let has_both_parties = text_lower.contains("both parties")
        || text_lower.contains("either party")
        || (text_lower.contains("both")
            && text_lower.contains("landlord")
            && text_lower.contains("tenant"));

    if has_prevailing_party || has_both_parties {
        return violations;
    }

    // Check for attorney fee mentions
    let has_attorney_fees = text_lower.contains("attorney")
        && (text_lower.contains("fee") || text_lower.contains("cost"));

    if !has_attorney_fees {
        return violations;
    }

    // Check for landlord rights to attorney fees
    let has_landlord_fee_right = text_lower.contains("landlord is entitled to attorney fee")
        || text_lower.contains("landlord shall be entitled to attorney fee")
        || text_lower.contains("lessor is entitled to attorney fee")
        || text_lower.contains("landlord is entitled to recover attorney fee")
        || text_lower.contains("landlord may recover attorney fee");

    // Check if tenant has obligation to pay landlord's fees/costs
    let has_tenant_pay_obligation = text_lower.contains("tenant shall pay")
        && (text_lower.contains("landlord") || text_lower.contains("attorney"))
        || text_lower.contains("tenant agrees to pay")
            && (text_lower.contains("landlord")
                || text_lower.contains("attorney")
                || text_lower.contains("legal"));

    // Check if tenant has reciprocal rights
    let has_tenant_fee_right = text_lower.contains("tenant is entitled to attorney fee")
        || text_lower.contains("tenant shall be entitled to attorney fee")
        || text_lower.contains("lessee is entitled to attorney fee")
        || text_lower.contains("tenant is entitled to recover attorney fee")
        || text_lower.contains("tenant may recover attorney fee");

    if (has_landlord_fee_right || has_tenant_pay_obligation) && !has_tenant_fee_right {
        violations.push(Violation {
            statute: "F.S. § 83.48".to_string(),
            severity: Severity::Critical,
            message: "Attorney fee clause is not reciprocal. Florida Statute § 83.48 requires that if the landlord can recover attorney fees, the tenant must have the same right. Use 'prevailing party' language or ensure mutual fee recovery rights.".to_string(),
            page: None,
            text_snippet: Some(if text.len() <= 200 { text.to_string() } else { format!("{}...", &text[..200]) }),
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// § 83.56/83.57 - Notice Requirements
// ============================================================================

/// Check notice requirements per Florida Statutes § 83.56 and § 83.57
pub fn check_notice_requirements(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    violations.extend(check_nonpayment_notice(&text_lower));
    violations.extend(check_lease_violation_notice(&text_lower));
    violations.extend(check_termination_notice(&text_lower));

    violations
}

/// Check 3-day notice for nonpayment per § 83.56(3)
fn check_nonpayment_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    let nonpayment_patterns = [
        r"(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice).*?(?:nonpayment|non-payment|rent)",
        r"(?:nonpayment|non-payment|rent).*?(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice)",
    ];

    for pattern in &nonpayment_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(text) {
                if let Some(days_match) = caps.get(1) {
                    if let Ok(days) = days_match.as_str().parse::<u32>() {
                        if days < 3 {
                            violations.push(Violation {
                                statute: "F.S. § 83.56(3)".to_string(),
                                severity: Severity::Critical,
                                message: format!(
                                    "Notice period for nonpayment of rent must be at least 3 business days. Found: {} day(s)",
                                    days
                                ),
                                page: None,
                                text_snippet: Some(caps.get(0).unwrap().as_str().to_string()),
                                text_position: None,
                            });
                        }
                    }
                }
            }
        }
    }

    violations
}

/// Check 7-day notice for lease violations per § 83.56(2)
fn check_lease_violation_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    let violation_patterns = [
        r"(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice).*?(?:violation|breach|default|noncompliance|non-compliance)",
        r"(?:violation|breach|default|noncompliance|non-compliance).*?(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice)",
    ];

    for pattern in &violation_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(text) {
                if let Some(days_match) = caps.get(1) {
                    if let Ok(days) = days_match.as_str().parse::<u32>() {
                        if days < 7 {
                            violations.push(Violation {
                                statute: "F.S. § 83.56(2)".to_string(),
                                severity: Severity::Critical,
                                message: format!(
                                    "Notice period for lease violations must be at least 7 days. Found: {} day(s)",
                                    days
                                ),
                                page: None,
                                text_snippet: Some(caps.get(0).unwrap().as_str().to_string()),
                                text_position: None,
                            });
                        }
                    }
                }
            }
        }
    }

    violations
}

/// Check 15-day notice for month-to-month termination per § 83.57
fn check_termination_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    let termination_patterns = [
        r"(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice).*?(?:terminat|end|cancel).*?(?:month-to-month|monthly)",
        r"(?:month-to-month|monthly).*?(?:terminat|end|cancel).*?(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice)",
    ];

    for pattern in &termination_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(text) {
                if let Some(days_match) = caps.get(1) {
                    if let Ok(days) = days_match.as_str().parse::<u32>() {
                        if days < 15 {
                            violations.push(Violation {
                                statute: "F.S. § 83.57".to_string(),
                                severity: Severity::Critical,
                                message: format!(
                                    "Notice period for month-to-month termination must be at least 15 days. Found: {} day(s)",
                                    days
                                ),
                                page: None,
                                text_snippet: Some(caps.get(0).unwrap().as_str().to_string()),
                                text_position: None,
                            });
                        }
                    }
                }
            }
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_florida_combined_check() {
        let text = "Tenant waives notice. Deposit returned in 45 days. Tenant pays landlord attorney fees.";
        let violations = check_florida_compliance(text);

        // Should detect multiple violations
        assert!(violations.len() >= 2);
    }

    #[test]
    fn test_compliant_florida_lease() {
        let text = "This residential lease is for property at 123 Main St. \
                    Security deposit held at First National Bank, Miami, Florida. \
                    Landlord returns deposit within 15 days if no claim. \
                    The prevailing party shall be entitled to reasonable attorney fees. \
                    Tenant shall receive 3 business days notice for nonpayment of rent.";
        let violations = check_florida_compliance(text);

        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(critical.is_empty());
    }
}
