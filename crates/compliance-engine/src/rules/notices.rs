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

/// Check 15-day notice for month-to-month termination per § 83.57
fn check_termination_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Pattern to detect notice periods for termination of month-to-month tenancy
    let termination_patterns = [
        r"(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice).*?(?:terminat|end|cancel).*?(?:month-to-month|monthly)",
        r"(?:month-to-month|monthly).*?(?:terminat|end|cancel).*?(\d+)\s*(?:day|business\s*day)s?\s*(?:notice|written\s*notice)",
    ];

    for pattern in &termination_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(text) {
                if let Some(days_match) = caps.get(1) {
                    if let Ok(days) = days_match.as_str().parse::<u32>() {
                        // § 83.57 requires 15 days minimum
                        if days < 15 {
                            violations.push(Violation {
                                statute: "83.57".to_string(),
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
}
