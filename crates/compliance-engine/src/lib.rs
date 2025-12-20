pub mod calendar;
pub mod extractors;
pub mod patterns;
pub mod rules;

use shared_types::{ComplianceReport, LeaseDocument, Violation};

/// ComplianceEngine entry point
pub struct ComplianceEngine;

impl ComplianceEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn check_compliance(&self, document: &LeaseDocument) -> ComplianceReport {
        let mut violations = Vec::new();

        // Combine all pages into a single text for analysis
        let full_text = document.text_content.join("\n");

        // Run all compliance checks on document text
        violations.extend(rules::prohibited::check_prohibited_provisions(&full_text));
        violations.extend(rules::deposit::check_security_deposit(&full_text));
        violations.extend(rules::attorney_fees::check_attorney_fees(&full_text));
        violations.extend(rules::notices::check_notice_requirements(&full_text));

        ComplianceReport {
            document_id: document.id.clone(),
            violations,
            checked_at: chrono::Utc::now().timestamp() as u64,
        }
    }

    /// Check compliance on raw text (for testing)
    pub fn check_text(&self, text: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        violations.extend(rules::prohibited::check_prohibited_provisions(text));
        violations.extend(rules::deposit::check_security_deposit(text));
        violations.extend(rules::attorney_fees::check_attorney_fees(text));
        violations.extend(rules::notices::check_notice_requirements(text));
        violations
    }
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::Severity;

    #[test]
    fn test_engine_detects_multiple_violations() {
        let engine = ComplianceEngine::new();
        let text = "Tenant waives notice. Landlord returns deposit within 45 days. Tenant pays attorney fees.";
        let violations = engine.check_text(text);

        // Should detect multiple rule violations
        assert!(violations.len() >= 2);
    }

    #[test]
    fn test_engine_detects_prohibited_provisions() {
        let engine = ComplianceEngine::new();
        let text = "Tenant hereby waives any right to notice before termination";
        let violations = engine.check_text(text);

        assert!(violations.iter().any(|v| v.statute.starts_with("83.47")));
    }

    #[test]
    fn test_engine_detects_deposit_violations() {
        let engine = ComplianceEngine::new();
        let text = "Landlord shall return deposit within 45 days";
        let violations = engine.check_text(text);

        assert!(violations.iter().any(|v| v.statute.starts_with("83.49")));
    }

    #[test]
    fn test_engine_detects_attorney_fee_violations() {
        let engine = ComplianceEngine::new();
        let text = "Tenant shall pay all landlord's attorney fees in any dispute";
        let violations = engine.check_text(text);

        assert!(violations.iter().any(|v| v.statute == "83.48"));
    }

    #[test]
    fn test_engine_detects_notice_violations() {
        let engine = ComplianceEngine::new();
        let text = "Tenant will be given 1 day notice for nonpayment";
        let violations = engine.check_text(text);

        assert!(violations.iter().any(|v| v.statute.contains("83.56")));
    }

    #[test]
    fn test_engine_accepts_compliant_lease() {
        let engine = ComplianceEngine::new();
        let text = "This residential lease is for property at 123 Main St. \
                    Security deposit held at First National Bank, Miami, Florida. \
                    Landlord returns deposit within 15 days if no claim. \
                    The prevailing party shall be entitled to reasonable attorney fees. \
                    Tenant shall receive 3 business days notice for nonpayment of rent.";
        let violations = engine.check_text(text);

        // Should have minimal or no critical violations
        let critical_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(critical_violations.is_empty());
    }
}
