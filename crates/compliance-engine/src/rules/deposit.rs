// Security deposit validation per Florida Statute § 83.49
use crate::extractors::numeric::{
    extract_days_near_deposit_return, has_bank_location, has_claim_context,
};
use shared_types::{Severity, Violation};

/// Validates security deposit return timelines and requirements
pub fn check_security_deposit(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Check for deposit return period
    if let Some(days) = extract_days_near_deposit_return(text) {
        let has_claim = has_claim_context(text);

        // 15-day rule for no claim (§ 83.49(3)(a))
        if days > 15 && !has_claim {
            violations.push(Violation {
                statute: "83.49(3)(a)".to_string(),
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
                statute: "83.49(3)(b)".to_string(),
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
                statute: "83.49(3)(b)".to_string(),
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
            statute: "83.49(2)".to_string(),
            severity: Severity::Warning,
            message: "Lease should specify the name and address of the Florida banking institution where the security deposit is held, or evidence of surety bond.".to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_excessive_return_period() {
        let text = "Landlord shall return deposit within 45 days";
        let violations = check_security_deposit(text);
        assert!(violations
            .iter()
            .any(|v| v.statute == "83.49(3)(a)" && v.message.contains("15 days")));
    }

    #[test]
    fn test_flags_missing_bank_location() {
        let text = "Security deposit is $1000. Deposit will be returned within 15 days.";
        let violations = check_security_deposit(text);
        assert!(violations
            .iter()
            .any(|v| v.severity == Severity::Warning && v.message.contains("bank")));
    }

    #[test]
    fn test_accepts_compliant_deposit_clause() {
        let text = "Security deposit of $1000 held at First National Bank, Miami, Florida. Landlord returns deposit within 15 days if no claim.";
        let violations = check_security_deposit(text);
        let deposit_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.statute.starts_with("83.49"))
            .collect();
        assert!(deposit_violations.is_empty());
    }

    #[test]
    fn test_flags_30_day_without_claim_context() {
        let text = "Deposit returned within 30 days";
        let violations = check_security_deposit(text);
        // 30 days is only valid with claim, should flag as potential issue
        assert!(violations.iter().any(|v| v.severity == Severity::Warning));
    }

    #[test]
    fn test_accepts_30_day_with_claim() {
        let text = "If landlord intends to impose a claim on the deposit, written notice will be sent within 30 days";
        let violations = check_security_deposit(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(critical.is_empty());
    }
}
