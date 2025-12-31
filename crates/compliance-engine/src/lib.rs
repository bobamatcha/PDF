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
pub use states::florida_contractor::{
    check_florida_contractor_compliance, covered_contractor_statutes, ContractorDocumentType,
};
pub use states::florida_realestate::{
    check_florida_realestate_compliance, covered_realestate_statutes, RealEstateDocumentType,
};

use shared_types::{ComplianceReport, LeaseDocument, Violation};

/// Document type for compliance checking
///
/// Hierarchical organization:
/// - Florida::Lease::{Agreement, TerminationNotice, Eviction}
/// - Florida::Purchase::{Contract, Contingencies::*, Addendum::*}
/// - Florida::Listing::{Exclusive}
/// - Florida::Contractor::{Invoice, CostOfMaterialsBill, NoticeOfCommencement, NoticeToOwner, ClaimOfLien, ReleaseOfLien, DisputeLien, FraudulentLienReport}
/// - Florida::BillOfSale::{Car, Boat, Trailer, JetSki, MobileHome} (Phase 1.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentType {
    // ========================================================================
    // Lease Documents (Chapter 83)
    // ========================================================================
    /// Residential lease agreement
    #[default]
    Lease,
    /// Lease termination notice (§ 83.56, § 83.57)
    LeaseTerminationNotice,
    /// Eviction notice (§ 83.56, § 83.59)
    EvictionNotice,

    // ========================================================================
    // Real Estate Purchase Documents (Chapter 475, 689)
    // ========================================================================
    /// Real estate purchase contract
    RealEstatePurchase,
    /// Purchase contract - As-Is version
    RealEstatePurchaseAsIs,
    /// Inspection contingency addendum
    InspectionContingency,
    /// Financing contingency addendum
    FinancingContingency,
    /// Escalation addendum
    EscalationAddendum,
    /// Appraisal contingency addendum
    AppraisalContingency,

    // ========================================================================
    // Listing Documents (Chapter 475)
    // ========================================================================
    /// Exclusive listing agreement
    ListingAgreement,

    // ========================================================================
    // Contractor Documents (Chapter 713)
    // ========================================================================
    /// Contractor invoice
    ContractorInvoice,
    /// Cost of materials bill
    CostOfMaterialsBill,
    /// Notice of Commencement (§ 713.13)
    NoticeOfCommencement,
    /// Notice to Owner / Preliminary Notice (§ 713.06)
    NoticeToOwner,
    /// Claim of Lien (§ 713.08)
    ClaimOfLien,
    /// Release of Lien (§ 713.21)
    ReleaseOfLien,
    /// Dispute of Lien (§ 713.22)
    DisputeLien,
    /// Fraudulent Lien Report (§ 713.31)
    FraudulentLienReport,
    /// Final Payment Affidavit (§ 713.06)
    FinalPaymentAffidavit,

    // ========================================================================
    // Bill of Sale Documents (Chapter 319) - Phase 1.1
    // ========================================================================
    /// Bill of Sale - Car
    BillOfSaleCar,
    /// Bill of Sale - Boat
    BillOfSaleBoat,
    /// Bill of Sale - Trailer
    BillOfSaleTrailer,
    /// Bill of Sale - Jet Ski / Personal Watercraft
    BillOfSaleJetSki,
    /// Bill of Sale - Mobile Home
    BillOfSaleMobileHome,

    // ========================================================================
    // Unknown / Other
    // ========================================================================
    /// Unknown document type (will attempt auto-detection)
    Unknown,
}

impl DocumentType {
    /// Get the category of the document type
    pub fn category(&self) -> DocumentCategory {
        match self {
            DocumentType::Lease
            | DocumentType::LeaseTerminationNotice
            | DocumentType::EvictionNotice => DocumentCategory::Lease,

            DocumentType::RealEstatePurchase
            | DocumentType::RealEstatePurchaseAsIs
            | DocumentType::InspectionContingency
            | DocumentType::FinancingContingency
            | DocumentType::EscalationAddendum
            | DocumentType::AppraisalContingency => DocumentCategory::Purchase,

            DocumentType::ListingAgreement => DocumentCategory::Listing,

            DocumentType::ContractorInvoice
            | DocumentType::CostOfMaterialsBill
            | DocumentType::NoticeOfCommencement
            | DocumentType::NoticeToOwner
            | DocumentType::ClaimOfLien
            | DocumentType::ReleaseOfLien
            | DocumentType::DisputeLien
            | DocumentType::FraudulentLienReport
            | DocumentType::FinalPaymentAffidavit => DocumentCategory::Contractor,

            DocumentType::BillOfSaleCar
            | DocumentType::BillOfSaleBoat
            | DocumentType::BillOfSaleTrailer
            | DocumentType::BillOfSaleJetSki
            | DocumentType::BillOfSaleMobileHome => DocumentCategory::BillOfSale,

            DocumentType::Unknown => DocumentCategory::Unknown,
        }
    }

    /// Get all document types for a category
    pub fn types_for_category(category: DocumentCategory) -> Vec<DocumentType> {
        match category {
            DocumentCategory::Lease => vec![
                DocumentType::Lease,
                DocumentType::LeaseTerminationNotice,
                DocumentType::EvictionNotice,
            ],
            DocumentCategory::Purchase => vec![
                DocumentType::RealEstatePurchase,
                DocumentType::RealEstatePurchaseAsIs,
                DocumentType::InspectionContingency,
                DocumentType::FinancingContingency,
                DocumentType::EscalationAddendum,
                DocumentType::AppraisalContingency,
            ],
            DocumentCategory::Listing => vec![DocumentType::ListingAgreement],
            DocumentCategory::Contractor => vec![
                DocumentType::ContractorInvoice,
                DocumentType::CostOfMaterialsBill,
                DocumentType::NoticeOfCommencement,
                DocumentType::NoticeToOwner,
                DocumentType::ClaimOfLien,
                DocumentType::ReleaseOfLien,
                DocumentType::DisputeLien,
                DocumentType::FraudulentLienReport,
                DocumentType::FinalPaymentAffidavit,
            ],
            DocumentCategory::BillOfSale => vec![
                DocumentType::BillOfSaleCar,
                DocumentType::BillOfSaleBoat,
                DocumentType::BillOfSaleTrailer,
                DocumentType::BillOfSaleJetSki,
                DocumentType::BillOfSaleMobileHome,
            ],
            DocumentCategory::Unknown => vec![DocumentType::Unknown],
        }
    }
}

/// Document category for grouping document types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentCategory {
    /// Lease documents (Chapter 83)
    Lease,
    /// Real estate purchase documents (Chapter 475, 689)
    Purchase,
    /// Listing documents (Chapter 475)
    Listing,
    /// Contractor/construction lien documents (Chapter 713)
    Contractor,
    /// Bill of sale documents (Chapter 319)
    BillOfSale,
    /// Unknown category
    Unknown,
}

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

        // Layer 3: Local overrides (municipality-specific ordinances)
        violations.extend(layers::check_local_compliance(jurisdiction, text));

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

    // ========================================================================
    // Real Estate Document Compliance
    // ========================================================================

    /// Check compliance for a real estate document (purchase contract, listing, etc.)
    ///
    /// # Arguments
    /// * `jurisdiction` - The state and optional locality
    /// * `document` - The document to check
    /// * `doc_type` - Type of real estate document
    /// * `year_built` - Optional year the property was built (for lead paint)
    ///
    /// # Returns
    /// ComplianceReport with all violations found
    pub fn check_realestate_compliance(
        &self,
        jurisdiction: &Jurisdiction,
        document: &LeaseDocument,
        doc_type: DocumentType,
        year_built: Option<u32>,
    ) -> ComplianceReport {
        let full_text = document.text_content.join("\n");
        let violations = self.check_realestate_text_with_jurisdiction(
            jurisdiction,
            &full_text,
            doc_type,
            year_built,
        );

        ComplianceReport {
            document_id: document.id.clone(),
            violations,
            checked_at: chrono::Utc::now().timestamp() as u64,
        }
    }

    /// Check real estate compliance on raw text for a specific jurisdiction
    ///
    /// Note: For full document type support, use `check_document_text_compliance` instead.
    pub fn check_realestate_text_with_jurisdiction(
        &self,
        jurisdiction: &Jurisdiction,
        text: &str,
        doc_type: DocumentType,
        year_built: Option<u32>,
    ) -> Vec<Violation> {
        // Route to the new unified compliance checker
        self.check_document_text_compliance(jurisdiction, text, doc_type, year_built)
    }

    /// Auto-detect document type and check appropriate compliance rules
    pub fn check_auto_detect(
        &self,
        jurisdiction: &Jurisdiction,
        document: &LeaseDocument,
        year_built: Option<u32>,
    ) -> ComplianceReport {
        let full_text = document.text_content.join("\n");
        let doc_type = self.detect_document_type(&full_text);

        match doc_type {
            DocumentType::Lease => self.check_compliance(jurisdiction, document, year_built),
            _ => self.check_realestate_compliance(jurisdiction, document, doc_type, year_built),
        }
    }

    /// Detect document type from text content
    pub fn detect_document_type(&self, text: &str) -> DocumentType {
        let text_lower = text.to_lowercase();

        // ====================================================================
        // Contractor Documents (Chapter 713) - Most specific first
        // ====================================================================

        // Notice of Commencement
        if (text_lower.contains("notice of commencement") || text_lower.contains("713.13"))
            && (text_lower.contains("owner") || text_lower.contains("property"))
        {
            return DocumentType::NoticeOfCommencement;
        }

        // Notice to Owner / Preliminary Notice
        if (text_lower.contains("notice to owner")
            || text_lower.contains("preliminary notice")
            || text_lower.contains("713.06"))
            && (text_lower.contains("lien") || text_lower.contains("furnish"))
        {
            return DocumentType::NoticeToOwner;
        }

        // Claim of Lien
        if (text_lower.contains("claim of lien") || text_lower.contains("713.08"))
            && (text_lower.contains("amount") || text_lower.contains("owed"))
        {
            return DocumentType::ClaimOfLien;
        }

        // Release of Lien
        if (text_lower.contains("release of lien")
            || text_lower.contains("satisfaction of lien")
            || text_lower.contains("lien waiver")
            || text_lower.contains("713.21"))
            && (text_lower.contains("release") || text_lower.contains("waive"))
        {
            return DocumentType::ReleaseOfLien;
        }

        // Dispute of Lien
        if text_lower.contains("contest of lien")
            || text_lower.contains("dispute") && text_lower.contains("lien")
            || text_lower.contains("713.22")
        {
            return DocumentType::DisputeLien;
        }

        // Fraudulent Lien Report
        if text_lower.contains("fraudulent lien")
            || text_lower.contains("713.31")
            || text_lower.contains("false lien")
        {
            return DocumentType::FraudulentLienReport;
        }

        // Final Payment Affidavit
        if text_lower.contains("final payment")
            && (text_lower.contains("affidavit") || text_lower.contains("713.06"))
        {
            return DocumentType::FinalPaymentAffidavit;
        }

        // Cost of Materials Bill
        if (text_lower.contains("materials") || text_lower.contains("supplies"))
            && (text_lower.contains("bill") || text_lower.contains("cost"))
            && (text_lower.contains("amount") || text_lower.contains("total"))
        {
            return DocumentType::CostOfMaterialsBill;
        }

        // Contractor Invoice
        if (text_lower.contains("invoice") || text_lower.contains("billing"))
            && (text_lower.contains("contractor")
                || text_lower.contains("services")
                || text_lower.contains("labor"))
        {
            return DocumentType::ContractorInvoice;
        }

        // ====================================================================
        // Bill of Sale Documents (Chapter 319)
        // ====================================================================

        if text_lower.contains("bill of sale") {
            if text_lower.contains("mobile home") || text_lower.contains("manufactured home") {
                return DocumentType::BillOfSaleMobileHome;
            }
            if text_lower.contains("jet ski")
                || text_lower.contains("personal watercraft")
                || text_lower.contains("pwc")
            {
                return DocumentType::BillOfSaleJetSki;
            }
            if text_lower.contains("boat") || text_lower.contains("vessel") {
                return DocumentType::BillOfSaleBoat;
            }
            if text_lower.contains("trailer") {
                return DocumentType::BillOfSaleTrailer;
            }
            if text_lower.contains("vehicle")
                || text_lower.contains("automobile")
                || text_lower.contains("car")
                || text_lower.contains("vin")
            {
                return DocumentType::BillOfSaleCar;
            }
        }

        // ====================================================================
        // Real Estate Purchase Documents (Chapter 475, 689)
        // ====================================================================

        // Escalation addendum
        if text_lower.contains("escalation")
            && (text_lower.contains("addendum") || text_lower.contains("clause"))
            && text_lower.contains("maximum")
        {
            return DocumentType::EscalationAddendum;
        }

        // Inspection contingency
        if text_lower.contains("inspection")
            && (text_lower.contains("contingency") || text_lower.contains("addendum"))
            && (text_lower.contains("days") || text_lower.contains("period"))
        {
            return DocumentType::InspectionContingency;
        }

        // Financing contingency
        if (text_lower.contains("financing") || text_lower.contains("mortgage"))
            && (text_lower.contains("contingency") || text_lower.contains("addendum"))
        {
            return DocumentType::FinancingContingency;
        }

        // Appraisal contingency
        if text_lower.contains("appraisal")
            && (text_lower.contains("contingency") || text_lower.contains("addendum"))
        {
            return DocumentType::AppraisalContingency;
        }

        // ====================================================================
        // Listing Documents (Chapter 475)
        // ====================================================================

        if (text_lower.contains("listing agreement") || text_lower.contains("exclusive listing"))
            && text_lower.contains("broker")
            && text_lower.contains("commission")
        {
            return DocumentType::ListingAgreement;
        }

        // ====================================================================
        // Real Estate Purchase Contracts
        // ====================================================================

        // As-Is purchase contract
        if (text_lower.contains("purchase") || text_lower.contains("sale"))
            && text_lower.contains("as-is")
            && (text_lower.contains("buyer") || text_lower.contains("seller"))
        {
            return DocumentType::RealEstatePurchaseAsIs;
        }

        // Standard purchase contract
        if (text_lower.contains("purchase") && text_lower.contains("contract"))
            || (text_lower.contains("sale") && text_lower.contains("agreement"))
                && (text_lower.contains("buyer") || text_lower.contains("seller"))
                && text_lower.contains("property")
        {
            return DocumentType::RealEstatePurchase;
        }

        // ====================================================================
        // Lease Documents (Chapter 83)
        // ====================================================================

        // Eviction notice
        if text_lower.contains("eviction")
            || (text_lower.contains("unlawful detainer") && text_lower.contains("notice"))
            || text_lower.contains("83.59")
        {
            return DocumentType::EvictionNotice;
        }

        // Termination notice
        if text_lower.contains("termination")
            && text_lower.contains("notice")
            && (text_lower.contains("lease") || text_lower.contains("tenancy"))
        {
            return DocumentType::LeaseTerminationNotice;
        }

        // Default to lease if it has lease-like characteristics, otherwise unknown
        if text_lower.contains("lease")
            || text_lower.contains("tenant")
            || text_lower.contains("landlord")
            || text_lower.contains("rent")
        {
            return DocumentType::Lease;
        }

        DocumentType::Unknown
    }

    /// Check compliance for any Florida document type
    ///
    /// This is the unified entry point that routes to the appropriate
    /// compliance checker based on document type.
    pub fn check_document_compliance(
        &self,
        jurisdiction: &Jurisdiction,
        document: &LeaseDocument,
        doc_type: DocumentType,
        year_built: Option<u32>,
    ) -> ComplianceReport {
        let full_text = document.text_content.join("\n");
        let violations =
            self.check_document_text_compliance(jurisdiction, &full_text, doc_type, year_built);

        ComplianceReport {
            document_id: document.id.clone(),
            violations,
            checked_at: chrono::Utc::now().timestamp() as u64,
        }
    }

    /// Check document compliance on raw text
    pub fn check_document_text_compliance(
        &self,
        jurisdiction: &Jurisdiction,
        text: &str,
        doc_type: DocumentType,
        year_built: Option<u32>,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Layer 1: Federal (applies to most document types)
        if matches!(
            doc_type.category(),
            DocumentCategory::Lease | DocumentCategory::Purchase
        ) {
            violations.extend(layers::check_federal_compliance(text, year_built));
        }

        // Layer 2: Route to appropriate state-specific checker
        if jurisdiction.state == State::FL {
            match doc_type.category() {
                DocumentCategory::Lease => {
                    violations.extend(states::check_state_compliance(State::FL, text));
                    violations.extend(layers::check_local_compliance(jurisdiction, text));
                }
                DocumentCategory::Purchase => {
                    let re_doc_type = match doc_type {
                        DocumentType::RealEstatePurchase | DocumentType::RealEstatePurchaseAsIs => {
                            RealEstateDocumentType::PurchaseContract
                        }
                        DocumentType::EscalationAddendum => {
                            RealEstateDocumentType::EscalationAddendum
                        }
                        DocumentType::InspectionContingency
                        | DocumentType::FinancingContingency
                        | DocumentType::AppraisalContingency => {
                            // Contingencies are checked as part of purchase contract
                            RealEstateDocumentType::PurchaseContract
                        }
                        _ => RealEstateDocumentType::Unknown,
                    };
                    violations.extend(states::florida_realestate::check_document_type(
                        text,
                        re_doc_type,
                    ));
                }
                DocumentCategory::Listing => {
                    violations.extend(states::florida_realestate::check_listing_agreement(text));
                }
                DocumentCategory::Contractor => {
                    let contractor_doc_type = match doc_type {
                        DocumentType::ContractorInvoice => ContractorDocumentType::Invoice,
                        DocumentType::CostOfMaterialsBill => {
                            ContractorDocumentType::CostOfMaterialsBill
                        }
                        DocumentType::NoticeOfCommencement => {
                            ContractorDocumentType::NoticeOfCommencement
                        }
                        DocumentType::NoticeToOwner => ContractorDocumentType::NoticeToOwner,
                        DocumentType::ClaimOfLien => ContractorDocumentType::ClaimOfLien,
                        DocumentType::ReleaseOfLien => ContractorDocumentType::ReleaseOfLien,
                        DocumentType::DisputeLien => ContractorDocumentType::DisputeLien,
                        DocumentType::FraudulentLienReport => {
                            ContractorDocumentType::FraudulentLienReport
                        }
                        DocumentType::FinalPaymentAffidavit => {
                            ContractorDocumentType::FinalPaymentAffidavit
                        }
                        _ => ContractorDocumentType::Unknown,
                    };
                    violations.extend(states::florida_contractor::check_document_type(
                        text,
                        contractor_doc_type,
                    ));
                }
                DocumentCategory::BillOfSale => {
                    // Bill of Sale compliance (Chapter 319 - Motor Vehicle Titles)
                    use states::florida_billofsale::{check_bill_of_sale, BillOfSaleType};

                    let bill_of_sale_type = match doc_type {
                        DocumentType::BillOfSaleCar => BillOfSaleType::Car,
                        DocumentType::BillOfSaleBoat => BillOfSaleType::Boat,
                        DocumentType::BillOfSaleTrailer => BillOfSaleType::Trailer,
                        DocumentType::BillOfSaleJetSki => BillOfSaleType::JetSki,
                        DocumentType::BillOfSaleMobileHome => BillOfSaleType::MobileHome,
                        _ => BillOfSaleType::detect(text),
                    };
                    violations.extend(check_bill_of_sale(text, bill_of_sale_type));
                }
                DocumentCategory::Unknown => {
                    // Try auto-detection
                    let detected = self.detect_document_type(text);
                    if detected != DocumentType::Unknown {
                        return self.check_document_text_compliance(
                            jurisdiction,
                            text,
                            detected,
                            year_built,
                        );
                    }
                }
            }
        }

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
    fn test_florida_jurisdiction_check() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::new(State::FL);
        let text = "Tenant waives notice. Deposit returned in 45 days.";

        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, None);

        assert!(violations.len() >= 2);
        assert!(violations.iter().any(|v| v.statute.contains("83.")));
    }

    #[test]
    #[cfg(feature = "all-states")]
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
    #[cfg(feature = "all-states")]
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

        // Florida always supported
        assert!(states.contains(&State::FL));

        #[cfg(feature = "all-states")]
        {
            // Tier 1: Big Five + Florida
            assert!(states.contains(&State::TX));
            assert!(states.contains(&State::CA));
            assert!(states.contains(&State::NY));
            assert!(states.contains(&State::GA));
            assert!(states.contains(&State::IL));
            // Tier 2: Growth Hubs
            assert!(states.contains(&State::PA));
            assert!(states.contains(&State::NJ));
            assert!(states.contains(&State::VA));
            assert!(states.contains(&State::MA));
            assert!(states.contains(&State::OH));
            assert!(states.contains(&State::MI));
            assert!(states.contains(&State::WA));
            assert!(states.contains(&State::AZ));
            assert!(states.contains(&State::NC));
            assert!(states.contains(&State::TN));
            assert_eq!(states.len(), 16);
        }

        #[cfg(not(feature = "all-states"))]
        {
            // Florida-only mode
            assert_eq!(states.len(), 1);
        }
    }

    #[test]
    fn test_covered_statutes() {
        let engine = ComplianceEngine::new();

        // Florida statutes always available
        let fl_statutes = engine.covered_statutes(State::FL);
        assert!(!fl_statutes.is_empty());
        assert!(fl_statutes.iter().any(|s| s.contains("83.47")));

        #[cfg(feature = "all-states")]
        {
            let tx_statutes = engine.covered_statutes(State::TX);
            assert!(!tx_statutes.is_empty());
            assert!(tx_statutes.iter().any(|s| s.contains("92.")));
        }

        #[cfg(not(feature = "all-states"))]
        {
            // Non-Florida states return empty in florida-only mode
            let tx_statutes = engine.covered_statutes(State::TX);
            assert!(tx_statutes.is_empty());
        }
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

    // ========================================================================
    // Locality-based compliance tests
    // These tests verify that locality detection from ZIP codes affects
    // compliance checking, even when the lease text doesn't mention the city.
    // ========================================================================

    #[test]
    #[cfg(feature = "all-states")]
    fn test_chicago_rlto_applies_via_zip_not_text() {
        // BUG TEST: When user provides Chicago ZIP but lease text doesn't mention Chicago,
        // RLTO requirements should still apply based on the jurisdiction locality.
        let engine = ComplianceEngine::new();

        // Create jurisdiction from ZIP code (not from text)
        let jurisdiction = Jurisdiction::from_zip(State::IL, "60601");
        assert_eq!(jurisdiction.locality, Some(Locality::Chicago));

        // Lease text does NOT mention Chicago or any Chicago ZIP
        let text = "Monthly rent: $2,000. Security deposit: $2,000.";

        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, None);

        // RLTO Summary should be required for Chicago properties
        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("5-12-170") && v.message.contains("RLTO")),
            "Chicago RLTO requirements should apply when jurisdiction has Chicago locality, \
             even if lease text doesn't mention Chicago. Got violations: {:?}",
            violations.iter().map(|v| &v.statute).collect::<Vec<_>>()
        );
    }

    #[test]
    #[cfg(feature = "all-states")]
    fn test_nyc_rent_rules_apply_via_zip_not_text() {
        // BUG TEST: When user provides NYC ZIP but lease text doesn't mention NYC,
        // NYC-specific rules should still apply.
        let engine = ComplianceEngine::new();

        // Create jurisdiction from ZIP code
        let jurisdiction = Jurisdiction::from_zip(State::NY, "10001");
        assert_eq!(jurisdiction.locality, Some(Locality::NewYorkCity));

        // Lease text does NOT mention NYC but has EXCESSIVE deposit (> 1 month rent)
        let text = "Monthly rent: $3,000. Security deposit: $4,500.";

        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, None);

        // Should detect excessive deposit (NYC only allows 1 month)
        assert!(
            violations
                .iter()
                .any(|v| v.statute.contains("7-108") && v.severity == Severity::Critical),
            "NYC security deposit limit should apply when jurisdiction has NYC locality, \
             even if lease text doesn't mention NYC. Got violations: {:?}",
            violations.iter().map(|v| &v.statute).collect::<Vec<_>>()
        );
    }
}
