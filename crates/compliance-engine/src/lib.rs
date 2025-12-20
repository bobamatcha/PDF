//! Compliance Engine - Multi-Jurisdiction Lease Compliance Checking
//!
//! Implements the "Layer Cake" architecture for residential lease compliance:
//! - Federal Layer: Lead paint, Fair Housing (baseline for all)
//! - State Layer: State-specific statutory requirements
//! - Local Layer: Municipal ordinances (Chicago RLTO, NYC rent control, etc.)
//!
//! # Example
//!
//! ```rust
//! use compliance_engine::{ComplianceEngine, Jurisdiction, State};
//! use shared_types::LeaseDocument;
//!
//! let engine = ComplianceEngine::new();
//! let jurisdiction = Jurisdiction::new(State::FL);
//!
//! let document = LeaseDocument {
//!     id: "test".to_string(),
//!     filename: "lease.pdf".to_string(),
//!     pages: 1,
//!     text_content: vec!["Tenant waives all rights.".to_string()],
//!     created_at: 0,
//! };
//!
//! let report = engine.check_compliance(&jurisdiction, &document, None);
//! assert!(!report.violations.is_empty());
//! ```

pub mod calendar;
pub mod extractors;
pub mod jurisdiction;
pub mod layers;
pub mod patterns;
pub mod rules;
pub mod states;

pub use jurisdiction::{Jurisdiction, Locality, State, Tier};

use shared_types::{ComplianceReport, LeaseDocument, Violation};

/// Multi-jurisdiction compliance engine
///
/// Checks documents against federal, state, and local compliance rules
/// following the Layer Cake architecture.
pub struct ComplianceEngine;

impl ComplianceEngine {
    pub fn new() -> Self {
        Self
    }

    /// Check compliance for a specific jurisdiction
    ///
    /// # Arguments
    /// * `jurisdiction` - The state and optional locality
    /// * `document` - The lease document to check
    /// * `year_built` - Optional year the property was built (for lead paint)
    ///
    /// # Returns
    /// ComplianceReport with all violations found
    pub fn check_compliance(
        &self,
        jurisdiction: &Jurisdiction,
        document: &LeaseDocument,
        year_built: Option<u32>,
    ) -> ComplianceReport {
        let full_text = document.text_content.join("\n");
        let violations = self.check_text_with_jurisdiction(jurisdiction, &full_text, year_built);

        ComplianceReport {
            document_id: document.id.clone(),
            violations,
            checked_at: chrono::Utc::now().timestamp() as u64,
        }
    }

    /// Check compliance on raw text for a specific jurisdiction
    pub fn check_text_with_jurisdiction(
        &self,
        jurisdiction: &Jurisdiction,
        text: &str,
        year_built: Option<u32>,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Layer 1: Federal (applies to all)
        violations.extend(layers::check_federal_compliance(text, year_built));

        // Layer 2: State-specific
        violations.extend(states::check_state_compliance(jurisdiction.state, text));

        // Layer 3: Local overrides (future)
        // if let Some(ref locality) = jurisdiction.locality {
        //     violations.extend(layers::local::check_local_compliance(locality, text));
        // }

        violations
    }

    /// Legacy: Check compliance using Florida rules (backwards compatibility)
    #[deprecated(note = "Use check_compliance with Jurisdiction instead")]
    pub fn check_text(&self, text: &str) -> Vec<Violation> {
        // Maintain backwards compatibility with Florida-only checks
        let mut violations = Vec::new();
        violations.extend(rules::prohibited::check_prohibited_provisions(text));
        violations.extend(rules::deposit::check_security_deposit(text));
        violations.extend(rules::attorney_fees::check_attorney_fees(text));
        violations.extend(rules::notices::check_notice_requirements(text));
        violations
    }

    /// Get list of supported states
    pub fn supported_states(&self) -> Vec<State> {
        State::implemented_states()
    }

    /// Check if a state is fully supported
    pub fn is_state_supported(&self, state: State) -> bool {
        states::has_implementation(state)
    }

    /// Get covered statutes for a state
    pub fn covered_statutes(&self, state: State) -> Vec<&'static str> {
        states::covered_statutes(state)
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
    fn test_florida_jurisdiction_check() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::new(State::FL);
        let text = "Tenant waives notice. Deposit returned in 45 days.";

        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, None);

        assert!(violations.len() >= 2);
        assert!(violations.iter().any(|v| v.statute.contains("83.")));
    }

    #[test]
    fn test_texas_jurisdiction_check() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::new(State::TX);
        let text = "Application fee $50. Deposit returned in 60 days. Vehicles will be towed.";

        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, None);

        assert!(violations.len() >= 2);
        assert!(violations.iter().any(|v| v.statute.contains("92.")));
    }

    #[test]
    fn test_federal_layer_applies_to_all() {
        let engine = ComplianceEngine::new();

        // Test with Florida
        let fl = Jurisdiction::new(State::FL);
        let text = "Built 1965. No children allowed.";
        let fl_violations = engine.check_text_with_jurisdiction(&fl, text, None);

        // Test with Texas
        let tx = Jurisdiction::new(State::TX);
        let tx_violations = engine.check_text_with_jurisdiction(&tx, text, None);

        // Both should have Fair Housing violations
        assert!(fl_violations.iter().any(|v| v.statute.contains("3604")));
        assert!(tx_violations.iter().any(|v| v.statute.contains("3604")));
    }

    #[test]
    fn test_lead_paint_disclosure() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::new(State::FL);

        // Pre-1978 without disclosure
        let text = "Property built in 1960.";
        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, Some(1960));

        assert!(violations.iter().any(|v| v.statute.contains("4852d")));
    }

    #[test]
    fn test_compliant_florida_lease() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::new(State::FL);
        let text = "This residential lease is for property built in 1995. \
                    Security deposit held at First National Bank, Miami, Florida. \
                    Landlord returns deposit within 15 days if no claim. \
                    The prevailing party shall be entitled to reasonable attorney fees. \
                    Tenant shall receive 3 business days notice for nonpayment of rent.";

        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, Some(1995));

        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(critical.is_empty());
    }

    #[test]
    fn test_compliant_texas_lease() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::new(State::TX);
        let text = "Property built in 2000. \
                    Please review our Selection Criteria before applying. \
                    See Parking Addendum for vehicle rules. \
                    Security deposit returned within 30 days. \
                    Repair requests must be in writing. \
                    Landlord will respond within reasonable time.";

        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, Some(2000));

        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(critical.is_empty());
    }

    #[test]
    fn test_supported_states() {
        let engine = ComplianceEngine::new();
        let states = engine.supported_states();

        assert!(states.contains(&State::FL));
        assert!(states.contains(&State::TX));
        assert_eq!(states.len(), 2);
    }

    #[test]
    fn test_covered_statutes() {
        let engine = ComplianceEngine::new();

        let fl_statutes = engine.covered_statutes(State::FL);
        assert!(!fl_statutes.is_empty());
        assert!(fl_statutes.iter().any(|s| s.contains("83.47")));

        let tx_statutes = engine.covered_statutes(State::TX);
        assert!(!tx_statutes.is_empty());
        assert!(tx_statutes.iter().any(|s| s.contains("92.")));
    }

    #[test]
    #[allow(deprecated)]
    fn test_legacy_check_text() {
        let engine = ComplianceEngine::new();
        let text = "Tenant waives notice before termination";

        let violations = engine.check_text(text);
        assert!(violations.iter().any(|v| v.statute.contains("83.47")));
    }

    #[test]
    fn test_jurisdiction_from_zip() {
        // Chicago zip
        let chicago = Jurisdiction::from_zip(State::IL, "60601");
        assert_eq!(chicago.locality, Some(Locality::Chicago));

        // NYC zip
        let nyc = Jurisdiction::from_zip(State::NY, "10001");
        assert_eq!(nyc.locality, Some(Locality::NewYorkCity));

        // Texas zip (no special locality)
        let tx = Jurisdiction::from_zip(State::TX, "75001");
        assert_eq!(tx.locality, None);
    }
}
