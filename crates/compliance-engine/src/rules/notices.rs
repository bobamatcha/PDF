use regex::Regex;
use shared_types::{Severity, Violation};

/// Check notice requirements per Florida Statutes § 83.56 and § 83.57
pub fn check_notice_requirements(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for notice period mentions
    violations.extend(check_nonpayment_notice(&text_lower));
    violations.extend(check_lease_violation_notice(&text_lower));
    violations.extend(check_termination_notice(&text_lower));

    violations
}

/// Check 3-day notice for nonpayment per § 83.56(3)
fn check_nonpayment_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Pattern to detect notice periods for nonpayment
    let nonpayment_patterns = [
        r"(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice).*?(?:nonpayment|non-payment|rent)",
        r"(?:nonpayment|non-payment|rent).*?(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice)",
    ];

    for pattern in &nonpayment_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(text) {
                if let Some(days_match) = caps.get(1) {
                    if let Ok(days) = days_match.as_str().parse::<u32>() {
                        // § 83.56(3) requires 3 business days minimum
                        if days < 3 {
                            violations.push(Violation {
                                statute: "83.56(3)".to_string(),
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

    // Pattern to detect notice periods for lease violations
    let violation_patterns = [
        r"(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice).*?(?:violation|breach|default|noncompliance|non-compliance)",
        r"(?:violation|breach|default|noncompliance|non-compliance).*?(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice)",
    ];

    for pattern in &violation_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(text) {
                if let Some(days_match) = caps.get(1) {
                    if let Ok(days) = days_match.as_str().parse::<u32>() {
                        // § 83.56(2) requires 7 days minimum
                        if days < 7 {
                            violations.push(Violation {
                                statute: "83.56(2)".to_string(),
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

/// Check 30-day notice for month-to-month termination per § 83.57
///
/// NOTE: HB 1417 (2023) changed the minimum from 15 days to 30 days
/// for month-to-month tenancy termination. This is a critical update.
fn check_termination_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if this is a month-to-month termination context
    let is_month_to_month_context =
        text_lower.contains("month-to-month") || text_lower.contains("monthly");
    let has_termination_context = text_lower.contains("terminat")
        || text_lower.contains("end")
        || text_lower.contains("cancel");

    if !is_month_to_month_context || !has_termination_context {
        return violations;
    }

    // Pattern to extract notice period days from the text
    let notice_patterns = [
        r"(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice)",
        r"(?:notice|written\s*notice)\s*(?:of\s*)?(\d+)\s*(?:day|business\s*day)s?",
    ];

    for pattern in &notice_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(&text_lower) {
                if let Some(days_match) = caps.get(1) {
                    if let Ok(days) = days_match.as_str().parse::<u32>() {
                        // § 83.57 as amended by HB 1417 (2023) requires 30 days minimum
                        if days < 30 {
                            violations.push(Violation {
                                statute: "83.57".to_string(),
                                severity: Severity::Critical,
                                message: format!(
                                    "Notice period for month-to-month termination must be at least 30 days per HB 1417 (2023). Found: {} day(s)",
                                    days
                                ),
                                page: None,
                                text_snippet: Some(caps.get(0).unwrap().as_str().to_string()),
                                text_position: None,
                            });
                        }
                        // Only report the first match to avoid duplicates
                        return violations;
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
    fn test_flags_short_notice_period() {
        let text = "Tenant will be given 1 day notice for nonpayment";
        let violations = check_notice_requirements(text);
        assert!(violations.iter().any(|v| v.statute.contains("83.56")));
    }

    #[test]
    fn test_accepts_valid_three_day() {
        let text = "Tenant shall receive 3 business days notice for nonpayment of rent";
        let violations = check_notice_requirements(text);
        let notice_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.statute.contains("83.56(3)"))
            .collect();
        assert!(notice_violations.is_empty());
    }

    // ========================================================================
    // HB 1417 (2023) - Month-to-Month Termination Tests
    // Changed from 15 days to 30 days minimum
    // ========================================================================

    #[test]
    fn test_month_to_month_termination_15_days_now_fails() {
        // Per HB 1417 (2023), 15 days is no longer sufficient - must be 30 days
        let text =
            "Either party may terminate this month-to-month tenancy with 15 days written notice.";
        let violations = check_termination_notice(text);
        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("83.57") && v.message.contains("30 days")),
            "15-day notice should trigger violation after HB 1417. Got: {:?}",
            violations
        );
    }

    #[test]
    fn test_month_to_month_termination_30_days_passes() {
        // 30 days is the new minimum per HB 1417 (2023)
        let text =
            "Either party may terminate this month-to-month tenancy with 30 days written notice.";
        let violations = check_termination_notice(text);
        assert!(
            violations.is_empty(),
            "30-day notice should pass. Got: {:?}",
            violations
        );
    }

    #[test]
    fn test_month_to_month_termination_7_days_fails() {
        let text = "Monthly tenancy may be ended with 7 days notice to terminate.";
        let violations = check_termination_notice(text);
        assert!(
            violations.iter().any(|v| v.statute.contains("83.57")),
            "7-day notice should trigger violation. Got: {:?}",
            violations
        );
    }

    #[test]
    fn test_month_to_month_termination_60_days_passes() {
        // More than 30 days is always fine
        let text = "Either party may cancel this monthly lease with 60 days written notice.";
        let violations = check_termination_notice(text);
        assert!(
            violations.is_empty(),
            "60-day notice should pass. Got: {:?}",
            violations
        );
    }
}
