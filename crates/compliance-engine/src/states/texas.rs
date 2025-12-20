//! Texas Residential Landlord-Tenant Law Compliance
//!
//! Texas Property Code Title 8, Chapter 92
//! Key requirements based on LEASE_RESEARCH.md:
//! - Tenant Screening Transparency (2025)
//! - Lockout Policy Formatting (must be bold/underlined)
//! - Parking & Towing Addendum
//! - Security Deposit Return (30 days)
//! - Repair Request Procedures

use lazy_static::lazy_static;
use regex::Regex;
use shared_types::{Severity, TextPosition, Violation};

// Keyword lists for Texas compliance
const LOCKOUT_KEYWORDS: &[&str] = &[
    "lockout",
    "lock out",
    "lock-out",
    "change lock",
    "changing lock",
    "locked out",
    "deny access",
    "denying access",
];

const SCREENING_KEYWORDS: &[&str] = &[
    "application fee",
    "screening fee",
    "background check",
    "credit check",
    "tenant screening",
    "rental application",
];

const SELECTION_CRITERIA_KEYWORDS: &[&str] = &[
    "selection criteria",
    "screening criteria",
    "qualification criteria",
    "rental criteria",
];

const PARKING_TOWING_KEYWORDS: &[&str] = &[
    "tow",
    "towing",
    "towed",
    "parking",
    "vehicle removal",
    "unauthorized vehicle",
];

lazy_static! {
    static ref DEPOSIT_RETURN_PATTERN: Regex =
        Regex::new(r"(?i)(?:return|refund).*?(?:deposit|security).*?(?:within\s+)?(\d+)\s*days?")
            .unwrap();
    static ref DEPOSIT_RETURN_PATTERN_ALT: Regex =
        Regex::new(r"(?i)(?:deposit|security).*?(?:return|refund).*?(?:within\s+)?(\d+)\s*days?")
            .unwrap();
}

/// Check all Texas-specific compliance requirements
pub fn check_texas_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_lockout_formatting(text));
    violations.extend(check_screening_notice(text));
    violations.extend(check_parking_towing(text));
    violations.extend(check_security_deposit(text));
    violations.extend(check_repair_procedures(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// § 92.0081 - Lockout Requirements
// ============================================================================

/// Check lockout clause formatting requirements
///
/// Per Texas Property Code § 92.0081:
/// Lockout clauses must be in bold or underlined text
pub fn check_lockout_formatting(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let has_lockout = LOCKOUT_KEYWORDS.iter().any(|kw| text_lower.contains(kw));

    if has_lockout {
        // Check for bold/underline indicators (common in PDFs/HTML)
        let has_formatting = text.contains("<b>")
            || text.contains("<strong>")
            || text.contains("<u>")
            || text.contains("**")
            || text.contains("__")
            || text_lower.contains("bold")
            || text_lower.contains("underline");

        // Also check for ALL CAPS as acceptable emphasis
        let lockout_pos = LOCKOUT_KEYWORDS.iter().find_map(|kw| text_lower.find(kw));

        let has_caps_emphasis = if let Some(pos) = lockout_pos {
            // Check if there's uppercase text around the lockout clause
            let start = pos.saturating_sub(50);
            let end = (pos + 100).min(text.len());
            let context = &text[start..end];
            context
                .chars()
                .filter(|c| c.is_alphabetic())
                .take(20)
                .all(|c| c.is_uppercase())
        } else {
            false
        };

        if !has_formatting && !has_caps_emphasis {
            let snippet = extract_context(text, lockout_pos.unwrap_or(0));
            violations.push(Violation {
                statute: "Tex. Prop. Code § 92.0081".to_string(),
                severity: Severity::Warning,
                message: "Lockout clause must be in bold or underlined text. Texas law requires prominent formatting for lockout provisions.".to_string(),
                page: None,
                text_snippet: Some(snippet),
                text_position: lockout_pos.map(|start| TextPosition {
                    start_offset: start,
                    end_offset: start + 20,
                }),
            });
        }
    }

    violations
}

// ============================================================================
// § 92.3515 - Tenant Screening Transparency (2025)
// ============================================================================

/// Check tenant screening notice requirements
///
/// Per Texas Property Code § 92.3515 (effective 2025):
/// Must provide Notice of Selection Criteria before accepting application fee
pub fn check_screening_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions application/screening fees
    let has_screening_fee = SCREENING_KEYWORDS.iter().any(|kw| text_lower.contains(kw));

    if has_screening_fee {
        // Check for selection criteria notice
        let has_criteria_notice = SELECTION_CRITERIA_KEYWORDS
            .iter()
            .any(|kw| text_lower.contains(kw));

        if !has_criteria_notice {
            violations.push(Violation {
                statute: "Tex. Prop. Code § 92.3515".to_string(),
                severity: Severity::Critical,
                message: "Must attach or reference Notice of Selection Criteria before accepting application fee. Include tenant screening criteria disclosure.".to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

// ============================================================================
// Parking & Towing Requirements
// ============================================================================

/// Check parking and towing addendum requirements
///
/// Texas requires specific addendum for towing authorization
pub fn check_parking_towing(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions towing
    let has_towing = PARKING_TOWING_KEYWORDS
        .iter()
        .any(|kw| text_lower.contains(kw));

    if has_towing {
        // Check for parking addendum reference
        let has_parking_addendum = text_lower.contains("parking addendum")
            || text_lower.contains("parking rules addendum")
            || text_lower.contains("vehicle addendum")
            || text_lower.contains("towing addendum")
            || (text_lower.contains("addendum") && text_lower.contains("parking"));

        if !has_parking_addendum {
            violations.push(Violation {
                statute: "Tex. Prop. Code § 92.0131".to_string(),
                severity: Severity::Warning,
                message: "Parking Rules Addendum required for towing authorization. Texas law requires separate parking addendum to authorize vehicle towing.".to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

// ============================================================================
// § 92.104 - Security Deposit Return
// ============================================================================

/// Check security deposit return timeline
///
/// Per Texas Property Code § 92.104:
/// Security deposit must be returned within 30 days
pub fn check_security_deposit(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Try both patterns
    let days = DEPOSIT_RETURN_PATTERN
        .captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
        .or_else(|| {
            DEPOSIT_RETURN_PATTERN_ALT
                .captures(text)
                .and_then(|caps| caps.get(1))
                .and_then(|m| m.as_str().parse::<u32>().ok())
        });

    if let Some(days) = days {
        if days > 30 {
            violations.push(Violation {
                statute: "Tex. Prop. Code § 92.104".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit must be returned within 30 days of move-out. Found: {} days",
                    days
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for forwarding address requirement
    let text_lower = text.to_lowercase();
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit && !text_lower.contains("forwarding address") {
        violations.push(Violation {
            statute: "Tex. Prop. Code § 92.107".to_string(),
            severity: Severity::Info,
            message: "Consider adding forwarding address provision. Texas requires tenant to provide forwarding address; failure may affect deposit return obligation.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// § 92.056 - Repair Request Procedures
// ============================================================================

/// Check repair request procedures
///
/// Per Texas Property Code § 92.056:
/// Must specify procedure for repair requests
pub fn check_repair_procedures(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease addresses repair requests
    let mentions_repair = text_lower.contains("repair")
        || text_lower.contains("maintenance")
        || text_lower.contains("fix");

    if mentions_repair {
        // Check for written notice requirement
        let has_written_notice = text_lower.contains("written") && text_lower.contains("notice");

        // Check for reasonable time provision
        let has_reasonable_time = text_lower.contains("reasonable time")
            || text_lower.contains("reasonable period")
            || text_lower.contains("7 days")
            || text_lower.contains("seven days");

        if !has_written_notice {
            violations.push(Violation {
                statute: "Tex. Prop. Code § 92.056".to_string(),
                severity: Severity::Warning,
                message: "Repair procedures should specify written notice requirement for repair requests.".to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }

        if !has_reasonable_time {
            violations.push(Violation {
                statute: "Tex. Prop. Code § 92.0561".to_string(),
                severity: Severity::Info,
                message: "Consider specifying 'reasonable time' for repairs. Texas law allows tenant remedies if landlord fails to make repairs within reasonable time.".to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

// ============================================================================
// Void Clauses Detection
// ============================================================================

/// Check for clauses that are void under Texas law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of landlord's repair duties (void under § 92.006)
    if text_lower.contains("waive")
        && (text_lower.contains("repair") || text_lower.contains("habitability"))
    {
        violations.push(Violation {
            statute: "Tex. Prop. Code § 92.006".to_string(),
            severity: Severity::Critical,
            message: "Lease cannot waive landlord's duty to repair or maintain habitability. Such provisions are void under Texas law.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for waiver of jury trial (void under § 92.0062)
    if text_lower.contains("waive") && text_lower.contains("jury") {
        violations.push(Violation {
            statute: "Tex. Prop. Code § 92.0062".to_string(),
            severity: Severity::Critical,
            message: "Waiver of right to jury trial is void under Texas Property Code.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for excessive late fees
    // Texas allows "reasonable" late fees, typically 10-12% is accepted
    let late_fee_pattern = Regex::new(r"(?i)late\s*fee.*?(\d+)\s*%").unwrap();
    if let Some(caps) = late_fee_pattern.captures(&text_lower) {
        if let Some(pct) = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok()) {
            if pct > 12 {
                violations.push(Violation {
                    statute: "Tex. Prop. Code § 92.019".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Late fee of {}% may be considered unreasonable. Texas courts typically accept 10-12% as reasonable.",
                        pct
                    ),
                    page: None,
                    text_snippet: None,
                    text_position: None,
                });
            }
        }
    }

    violations
}

/// Extract context around a position for display
fn extract_context(text: &str, pos: usize) -> String {
    let start = pos.saturating_sub(30);
    let end = (pos + 70).min(text.len());

    let mut result = String::new();
    if start > 0 {
        result.push_str("...");
    }
    result.push_str(&text[start..end]);
    if end < text.len() {
        result.push_str("...");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_screening_without_criteria() {
        let text = "Application fee of $50 required for background check.";
        let violations = check_texas_compliance(text);

        assert!(violations.iter().any(|v| v.statute.contains("92.3515")));
    }

    #[test]
    fn test_accepts_screening_with_criteria() {
        let text = "Application fee of $50 required. Please review our Selection Criteria before applying.";
        let violations = check_texas_compliance(text);

        assert!(!violations
            .iter()
            .any(|v| v.statute.contains("92.3515") && v.severity == Severity::Critical));
    }

    #[test]
    fn test_detects_towing_without_addendum() {
        let text = "Unauthorized vehicles will be towed at owner's expense.";
        let violations = check_texas_compliance(text);

        assert!(violations.iter().any(|v| v.statute.contains("92.0131")));
    }

    #[test]
    fn test_accepts_towing_with_addendum() {
        let text = "See Parking Addendum for vehicle rules. Unauthorized vehicles may be towed.";
        let violations = check_texas_compliance(text);

        assert!(!violations.iter().any(|v| v.statute.contains("92.0131")));
    }

    #[test]
    fn test_detects_excessive_deposit_return() {
        let text = "Security deposit will be returned within 60 days of move-out.";
        let violations = check_texas_compliance(text);

        assert!(violations
            .iter()
            .any(|v| v.statute.contains("92.104") && v.message.contains("30 days")));
    }

    #[test]
    fn test_accepts_30_day_deposit_return() {
        let text = "Security deposit will be returned within 30 days of move-out.";
        let violations = check_texas_compliance(text);

        assert!(!violations
            .iter()
            .any(|v| v.statute.contains("92.104") && v.severity == Severity::Critical));
    }

    #[test]
    fn test_detects_waiver_of_repair_duty() {
        let text = "Tenant waives landlord's duty to repair.";
        let violations = check_texas_compliance(text);

        assert!(violations.iter().any(|v| v.statute.contains("92.006")));
    }

    #[test]
    fn test_detects_jury_waiver() {
        let text = "Tenant hereby waives the right to jury trial.";
        let violations = check_texas_compliance(text);

        assert!(violations.iter().any(|v| v.statute.contains("92.0062")));
    }

    #[test]
    fn test_detects_excessive_late_fee() {
        let text = "Late fee shall be 20% of monthly rent.";
        let violations = check_texas_compliance(text);

        assert!(violations.iter().any(|v| v.statute.contains("92.019")));
    }

    #[test]
    fn test_accepts_reasonable_late_fee() {
        let text = "Late fee shall be 10% of monthly rent.";
        let violations = check_texas_compliance(text);

        assert!(!violations.iter().any(|v| v.statute.contains("92.019")));
    }

    #[test]
    fn test_compliant_texas_lease() {
        let text = "This lease includes the Parking Addendum. \
                    See Selection Criteria notice attached. \
                    Security deposit returned within 30 days. \
                    Repair requests must be in writing. \
                    Landlord will respond within reasonable time. \
                    **LOCKOUT POLICY**: Tenant may be locked out for nonpayment.";
        let violations = check_texas_compliance(text);

        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(critical.is_empty());
    }
}
