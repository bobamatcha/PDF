//! Pennsylvania Residential Landlord-Tenant Law Compliance
//!
//! Pennsylvania Landlord and Tenant Act of 1951 (68 P.S. § 250.101-250.602)
//! Key requirements based on LEASE_RESEARCH.md:
//! - Plain Language Consumer Contract Act (73 P.S. § 2201-2212)
//! - Security Deposit Limits (2 months first year, 1 month thereafter)
//! - Interest on Security Deposits (over $100 after 2 years)
//! - 30-day notice for termination
//! - Written lease required for rentals > 1 year

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

    /// Complex legal jargon patterns (Plain Language Act)
    static ref LEGAL_JARGON_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)\b(hereinafter|heretofore|herein|whereas)\b").unwrap(),
        Regex::new(r"(?i)\b(witnesseth|aforementioned|aforesaid)\b").unwrap(),
        Regex::new(r"(?i)\b(notwithstanding|shall\s+be\s+deemed)\b").unwrap(),
    ];

    /// Late fee pattern
    static ref LATE_FEE_PATTERN: Regex =
        Regex::new(r"(?i)late\s+(?:fee|charge|penalty)[:\s]+\$?([\d,]+(?:\.\d{2})?)").unwrap();
}

/// Check all Pennsylvania-specific compliance requirements
pub fn check_pennsylvania_compliance(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    violations.extend(check_security_deposit(text));
    violations.extend(check_plain_language(text));
    violations.extend(check_deposit_interest(text));
    violations.extend(check_termination_notice(text));
    violations.extend(check_void_clauses(text));

    violations
}

// ============================================================================
// Security Deposit Requirements
// ============================================================================

/// Check security deposit limits
///
/// Per 68 P.S. § 250.511a:
/// - First year: Max 2 months rent
/// - After first year: Max 1 month rent
/// - Must return within 30 days
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

    // Check deposit cap (2 months max in first year)
    if let (Some(deposit_amt), Some(rent_amt)) = (deposit, rent) {
        if deposit_amt > rent_amt * 2.0 {
            violations.push(Violation {
                statute: "68 P.S. § 250.511a".to_string(),
                severity: Severity::Critical,
                message: format!(
                    "Security deposit (${:.2}) exceeds Pennsylvania limit of 2 months' rent (${:.2}) \
                     for first year of tenancy.",
                    deposit_amt,
                    rent_amt * 2.0
                ),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    // Check for 30-day return requirement
    let text_lower = text.to_lowercase();
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        let has_return_timeline = text_lower.contains("30 days")
            || text_lower.contains("thirty days")
            || text_lower.contains("30-day");

        if !has_return_timeline {
            violations.push(Violation {
                statute: "68 P.S. § 250.512".to_string(),
                severity: Severity::Info,
                message: "Consider specifying 30-day deposit return timeline as required by \
                         Pennsylvania law."
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
// Plain Language Consumer Contract Act
// ============================================================================

/// Check Plain Language requirements
///
/// Per 73 P.S. § 2201-2212:
/// - Residential leases must be written in plain language
/// - Avoid legal jargon and complex terminology
pub fn check_plain_language(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Check for legal jargon
    for pattern in LEGAL_JARGON_PATTERNS.iter() {
        if pattern.is_match(text) {
            violations.push(Violation {
                statute: "73 P.S. § 2205".to_string(),
                severity: Severity::Warning,
                message: "Lease contains legal jargon. Pennsylvania Plain Language Act requires \
                         residential contracts to use clear, understandable language."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
            break; // Only flag once for jargon
        }
    }

    // Check average sentence length (very long sentences are harder to read)
    let sentences: Vec<&str> = text.split(['.', '!', '?']).collect();
    let total_words: usize = sentences.iter().map(|s| s.split_whitespace().count()).sum();
    let avg_words_per_sentence = if !sentences.is_empty() {
        total_words / sentences.len()
    } else {
        0
    };

    if avg_words_per_sentence > 35 {
        violations.push(Violation {
            statute: "73 P.S. § 2205".to_string(),
            severity: Severity::Info,
            message: format!(
                "Average sentence length ({} words) may reduce readability. \
                 Consider shorter sentences for Plain Language compliance.",
                avg_words_per_sentence
            ),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Security Deposit Interest
// ============================================================================

/// Check security deposit interest requirements
///
/// Per 68 P.S. § 250.511b:
/// - Deposits over $100 held for 2+ years must earn interest
/// - Interest must be paid annually to tenant
pub fn check_deposit_interest(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if deposit is mentioned
    let mentions_deposit = text_lower.contains("deposit") || text_lower.contains("security");

    if mentions_deposit {
        // Check for interest disclosure
        let has_interest_disclosure = text_lower.contains("interest")
            || text_lower.contains("250.511b")
            || text_lower.contains("escrow");

        // Only warn if no interest disclosure found
        if !has_interest_disclosure {
            violations.push(Violation {
                statute: "68 P.S. § 250.511b".to_string(),
                severity: Severity::Info,
                message: "Consider adding security deposit interest disclosure. Pennsylvania \
                         requires interest on deposits over $100 held for more than 2 years."
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
// Termination Notice Requirements
// ============================================================================

/// Check termination notice requirements
///
/// Per 68 P.S. § 250.501:
/// - 30 days notice for month-to-month tenancies
/// - Lease must specify notice period
pub fn check_termination_notice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if lease mentions termination
    if text_lower.contains("terminat") || text_lower.contains("end of lease") {
        // Check for proper notice period
        let has_notice_period = text_lower.contains("30 days")
            || text_lower.contains("thirty days")
            || text_lower.contains("30-day")
            || text_lower.contains("notice period");

        if !has_notice_period {
            violations.push(Violation {
                statute: "68 P.S. § 250.501".to_string(),
                severity: Severity::Info,
                message: "Consider specifying 30-day notice requirement for lease termination."
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

/// Check for void clauses under Pennsylvania law
pub fn check_void_clauses(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of landlord's duty to maintain
    if text_lower.contains("waive")
        && (text_lower.contains("habitability") || text_lower.contains("repair"))
    {
        violations.push(Violation {
            statute: "68 P.S. § 250.103".to_string(),
            severity: Severity::Critical,
            message: "Waiver of implied warranty of habitability is void under Pennsylvania law."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for confession of judgment
    if text_lower.contains("confession of judgment") || text_lower.contains("confess judgment") {
        violations.push(Violation {
            statute: "68 P.S. § 250.513".to_string(),
            severity: Severity::Critical,
            message: "Confession of judgment clauses are prohibited in Pennsylvania residential \
                     leases."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for waiver of right to jury trial
    if text_lower.contains("waive") && text_lower.contains("jury") {
        violations.push(Violation {
            statute: "Pa. Const. Art. I § 6".to_string(),
            severity: Severity::Critical,
            message: "Waiver of right to jury trial may be unenforceable in Pennsylvania."
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
    // Security Deposit Tests
    // ========================================================================

    #[test]
    fn test_detects_excessive_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $5,000.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("250.511a") && v.severity == Severity::Critical),
            "Should detect deposit exceeding 2 months rent"
        );
    }

    #[test]
    fn test_accepts_two_month_deposit() {
        let text = "Monthly rent: $1,500. Security deposit: $3,000.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("250.511a") && v.severity == Severity::Critical),
            "Should accept deposit equal to 2 months rent"
        );
    }

    #[test]
    fn test_accepts_one_month_deposit() {
        let text = "Monthly rent: $2,000. Security deposit: $2,000.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("250.511a") && v.severity == Severity::Critical),
            "Should accept deposit equal to 1 month rent"
        );
    }

    // ========================================================================
    // Plain Language Tests
    // ========================================================================

    #[test]
    fn test_detects_legal_jargon_hereinafter() {
        let text = "The Tenant, hereinafter referred to as the Lessee, agrees to pay rent.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("2205") && v.message.contains("jargon")),
            "Should detect 'hereinafter' as legal jargon"
        );
    }

    #[test]
    fn test_detects_legal_jargon_witnesseth() {
        let text = "WITNESSETH: The landlord agrees to provide housing.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("2205") && v.message.contains("Plain Language")),
            "Should detect 'witnesseth' as legal jargon"
        );
    }

    #[test]
    fn test_accepts_plain_language() {
        let text = "The tenant agrees to pay rent of $1,500 per month. \
                    The landlord will maintain the property in good condition.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("2205") && v.message.contains("jargon")),
            "Should accept plain language lease"
        );
    }

    // ========================================================================
    // Deposit Interest Tests
    // ========================================================================

    #[test]
    fn test_warns_missing_interest_disclosure() {
        let text = "Security deposit: $1,500. Deposit will be held by landlord.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("250.511b") && v.message.contains("interest")),
            "Should warn about missing interest disclosure"
        );
    }

    #[test]
    fn test_accepts_interest_disclosure() {
        let text = "Security deposit: $1,500. Deposit held in escrow account with interest.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            !violations
                .iter()
                .any(|v| v.statute.contains("250.511b") && v.severity == Severity::Critical),
            "Should accept lease with interest disclosure"
        );
    }

    // ========================================================================
    // Void Clauses Tests
    // ========================================================================

    #[test]
    fn test_detects_habitability_waiver() {
        let text = "Tenant agrees to waive implied warranty of habitability.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("250.103") && v.severity == Severity::Critical),
            "Should detect habitability waiver"
        );
    }

    #[test]
    fn test_detects_confession_of_judgment() {
        let text = "Tenant agrees to confession of judgment for unpaid rent.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("250.513") && v.message.contains("Confession")),
            "Should detect confession of judgment"
        );
    }

    #[test]
    fn test_detects_jury_waiver() {
        let text = "Tenant waives right to jury trial in any dispute.";
        let violations = check_pennsylvania_compliance(text);

        assert!(
            violations.iter().any(|v| v.message.contains("jury")),
            "Should detect jury waiver"
        );
    }

    // ========================================================================
    // Compliant Lease Tests
    // ========================================================================

    #[test]
    fn test_compliant_pennsylvania_lease() {
        let text = "Monthly rent: $1,800. Security deposit: $1,800 held in escrow with interest. \
                    Deposit returned within 30 days of lease termination. \
                    Landlord maintains property in habitable condition.";
        let violations = check_pennsylvania_compliance(text);

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
