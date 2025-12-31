//! Florida Construction Lien Law Compliance (Chapter 713)
//!
//! Compliance rules for Florida construction documents including contractor invoices,
//! notices of commencement, notices to owner, and lien-related documents.
//!
//! Key Statutes:
//! - § 713.13 - Notice of Commencement
//! - § 713.06 - Notice to Owner (Preliminary Notice)
//! - § 713.04 - Lien Rights Disclosure
//! - § 713.08 - Claim of Lien Requirements
//! - § 713.21 - Release/Satisfaction of Lien
//! - § 713.31 - Fraudulent Liens
//! - § 713.346 - Final Payment Affidavit
//! - § 713.23 - Contractor's Affidavit

use crate::patterns::{extract_snippet, find_text_position};
use regex::Regex;
use shared_types::{Severity, TextPosition, Violation};

/// Document type for Florida contractor/construction documents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractorDocumentType {
    /// Contractor invoice for services/materials
    Invoice,
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
    /// Dispute of Lien / Contest of Lien (§ 713.22)
    DisputeLien,
    /// Fraudulent Lien Report (§ 713.31)
    FraudulentLienReport,
    /// Contractor's Final Payment Affidavit (§ 713.06)
    FinalPaymentAffidavit,
    /// Unknown document type
    Unknown,
}

impl ContractorDocumentType {
    /// Detect document type from text content
    pub fn detect(text: &str) -> Self {
        let text_lower = text.to_lowercase();

        // Notice of Commencement (most specific first)
        if (text_lower.contains("notice of commencement")
            || text_lower.contains("713.13")
            || text_lower.contains("commencement of work"))
            && (text_lower.contains("owner") || text_lower.contains("property"))
        {
            return Self::NoticeOfCommencement;
        }

        // Notice to Owner / Preliminary Notice
        if (text_lower.contains("notice to owner")
            || text_lower.contains("preliminary notice")
            || text_lower.contains("713.06"))
            && (text_lower.contains("lien") || text_lower.contains("furnish"))
        {
            return Self::NoticeToOwner;
        }

        // Claim of Lien
        if (text_lower.contains("claim of lien") || text_lower.contains("713.08"))
            && (text_lower.contains("amount") || text_lower.contains("owed"))
        {
            return Self::ClaimOfLien;
        }

        // Release of Lien
        if (text_lower.contains("release of lien")
            || text_lower.contains("satisfaction of lien")
            || text_lower.contains("lien waiver")
            || text_lower.contains("713.21"))
            && (text_lower.contains("release") || text_lower.contains("waive"))
        {
            return Self::ReleaseOfLien;
        }

        // Dispute/Contest of Lien
        if (text_lower.contains("contest of lien")
            || text_lower.contains("dispute")
            || text_lower.contains("713.22"))
            && text_lower.contains("lien")
        {
            return Self::DisputeLien;
        }

        // Fraudulent Lien Report
        if (text_lower.contains("fraudulent lien")
            || text_lower.contains("713.31")
            || text_lower.contains("false lien"))
            && (text_lower.contains("report") || text_lower.contains("complaint"))
        {
            return Self::FraudulentLienReport;
        }

        // Final Payment Affidavit
        if text_lower.contains("final payment")
            && (text_lower.contains("affidavit") || text_lower.contains("713.06"))
        {
            return Self::FinalPaymentAffidavit;
        }

        // Cost of Materials Bill
        if (text_lower.contains("materials") || text_lower.contains("supplies"))
            && (text_lower.contains("bill") || text_lower.contains("cost"))
            && (text_lower.contains("amount") || text_lower.contains("total"))
        {
            return Self::CostOfMaterialsBill;
        }

        // Contractor Invoice
        if (text_lower.contains("invoice") || text_lower.contains("billing"))
            && (text_lower.contains("contractor")
                || text_lower.contains("services")
                || text_lower.contains("labor"))
        {
            return Self::Invoice;
        }

        Self::Unknown
    }
}

/// Check all Florida contractor document compliance requirements
///
/// Automatically detects document type and applies appropriate rules.
pub fn check_florida_contractor_compliance(text: &str) -> Vec<Violation> {
    let doc_type = ContractorDocumentType::detect(text);

    match doc_type {
        ContractorDocumentType::Invoice => check_contractor_invoice(text),
        ContractorDocumentType::CostOfMaterialsBill => check_cost_of_materials(text),
        ContractorDocumentType::NoticeOfCommencement => check_notice_of_commencement(text),
        ContractorDocumentType::NoticeToOwner => check_notice_to_owner(text),
        ContractorDocumentType::ClaimOfLien => check_claim_of_lien(text),
        ContractorDocumentType::ReleaseOfLien => check_release_of_lien(text),
        ContractorDocumentType::DisputeLien => check_dispute_lien(text),
        ContractorDocumentType::FraudulentLienReport => check_fraudulent_lien_report(text),
        ContractorDocumentType::FinalPaymentAffidavit => check_final_payment_affidavit(text),
        ContractorDocumentType::Unknown => {
            // If unknown, check for potential lien rights disclosure issues
            check_lien_rights_disclosure(text)
        }
    }
}

/// Check compliance for a specific document type
pub fn check_document_type(text: &str, doc_type: ContractorDocumentType) -> Vec<Violation> {
    match doc_type {
        ContractorDocumentType::Invoice => check_contractor_invoice(text),
        ContractorDocumentType::CostOfMaterialsBill => check_cost_of_materials(text),
        ContractorDocumentType::NoticeOfCommencement => check_notice_of_commencement(text),
        ContractorDocumentType::NoticeToOwner => check_notice_to_owner(text),
        ContractorDocumentType::ClaimOfLien => check_claim_of_lien(text),
        ContractorDocumentType::ReleaseOfLien => check_release_of_lien(text),
        ContractorDocumentType::DisputeLien => check_dispute_lien(text),
        ContractorDocumentType::FraudulentLienReport => check_fraudulent_lien_report(text),
        ContractorDocumentType::FinalPaymentAffidavit => check_final_payment_affidavit(text),
        ContractorDocumentType::Unknown => check_florida_contractor_compliance(text),
    }
}

// ============================================================================
// § 713.13 - Notice of Commencement
// ============================================================================

/// Check Notice of Commencement compliance per § 713.13
///
/// Required before construction begins on improvements > $2,500.
/// Must be recorded in public records and posted at job site.
pub fn check_notice_of_commencement(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required elements per § 713.13(1)

    // (a) Legal description of property
    let has_legal_description = text_lower.contains("legal description")
        || text_lower.contains("lot")
        || text_lower.contains("block")
        || text_lower.contains("subdivision")
        || text_lower.contains("parcel")
        || text_lower.contains("folio");

    // (b) General description of improvement
    let has_improvement_desc = text_lower.contains("improvement")
        || text_lower.contains("construction")
        || text_lower.contains("building")
        || text_lower.contains("renovation");

    // (c) Owner information
    let has_owner_info = text_lower.contains("owner")
        && (text_lower.contains("name") || text_lower.contains("address"));

    // (d) Contractor information
    let has_contractor_info = text_lower.contains("contractor")
        && (text_lower.contains("name")
            || text_lower.contains("address")
            || text_lower.contains("license"));

    // (e) Surety/Bond information (if applicable)
    let has_bond_info = text_lower.contains("surety")
        || text_lower.contains("bond")
        || text_lower.contains("no bond");

    // (f) Lender information (if applicable)
    let has_lender_info = text_lower.contains("lender")
        || text_lower.contains("construction loan")
        || text_lower.contains("no lender")
        || text_lower.contains("no construction loan");

    // (g) Expiration date
    let has_expiration = text_lower.contains("expir")
        || text_lower.contains("valid until")
        || text_lower.contains("effective period");

    // Owner's signature required
    let has_signature = text_lower.contains("signature")
        || text_lower.contains("signed")
        || text_lower.contains("notary")
        || text_lower.contains("sworn");

    // Build list of missing elements
    let mut missing = Vec::new();
    if !has_legal_description {
        missing.push("legal description of property");
    }
    if !has_improvement_desc {
        missing.push("description of improvement");
    }
    if !has_owner_info {
        missing.push("owner name and address");
    }
    if !has_contractor_info {
        missing.push("contractor information");
    }
    if !has_bond_info {
        missing.push("surety/bond information");
    }
    if !has_lender_info {
        missing.push("lender information");
    }
    if !has_expiration {
        missing.push("expiration date");
    }
    if !has_signature {
        missing.push("owner signature/notarization");
    }

    if !missing.is_empty() {
        let text_position =
            find_text_position(text, "commencement").map(|(start, end)| TextPosition {
                start_offset: start,
                end_offset: end,
            });

        let severity = if missing.len() >= 4 {
            Severity::Critical
        } else {
            Severity::Warning
        };

        violations.push(Violation {
            statute: "F.S. § 713.13".to_string(),
            severity,
            message: format!(
                "Notice of Commencement may be incomplete. Per § 713.13, required elements \
                 include property description, owner/contractor info, bond/lender details, \
                 and expiration date. Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: if text_lower.contains("commencement") {
                Some(extract_snippet(text, "commencement"))
            } else {
                Some(text.chars().take(100).collect())
            },
            text_position,
        });
    }

    // Check for recording requirement notice
    let has_recording_notice = text_lower.contains("record")
        || text_lower.contains("clerk")
        || text_lower.contains("public record");

    if !has_recording_notice {
        violations.push(Violation {
            statute: "F.S. § 713.13(1)(a)".to_string(),
            severity: Severity::Info,
            message: "Notice of Commencement must be recorded in public records and a certified \
                     copy posted at the job site per § 713.13(1)(d)."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// § 713.06 - Notice to Owner (Preliminary Notice)
// ============================================================================

/// Check Notice to Owner compliance per § 713.06
///
/// Required for lienors (except contractors with direct contracts) to preserve lien rights.
/// Must be served within 45 days of first furnishing labor/materials.
pub fn check_notice_to_owner(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required content per § 713.06(2)

    // Name and address of lienor
    let has_lienor_info = (text_lower.contains("lienor")
        || text_lower.contains("subcontractor")
        || text_lower.contains("supplier")
        || text_lower.contains("materialman"))
        && (text_lower.contains("name") || text_lower.contains("address"));

    // Description of labor/materials
    let has_description = text_lower.contains("labor")
        || text_lower.contains("material")
        || text_lower.contains("services")
        || text_lower.contains("supplies")
        || text_lower.contains("furnish");

    // Name of person ordering materials/services
    let has_ordering_party = text_lower.contains("ordered by")
        || text_lower.contains("contracted with")
        || text_lower.contains("employed by")
        || text_lower.contains("contractor");

    // Description of property
    let has_property_desc = text_lower.contains("property")
        || text_lower.contains("address")
        || text_lower.contains("lot")
        || text_lower.contains("location");

    // Statutory warning language
    let has_warning = text_lower.contains("this is not a lien")
        || text_lower.contains("warning")
        || text_lower.contains("lien right");

    // Build list of missing elements
    let mut missing = Vec::new();
    if !has_lienor_info {
        missing.push("lienor name and address");
    }
    if !has_description {
        missing.push("description of labor/materials");
    }
    if !has_ordering_party {
        missing.push("name of ordering party");
    }
    if !has_property_desc {
        missing.push("property description");
    }
    if !has_warning {
        missing.push("statutory warning language");
    }

    if !missing.is_empty() {
        let text_position = find_text_position(text, "notice").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });

        let severity = if missing.len() >= 3 {
            Severity::Critical
        } else {
            Severity::Warning
        };

        violations.push(Violation {
            statute: "F.S. § 713.06(2)".to_string(),
            severity,
            message: format!(
                "Notice to Owner may be incomplete. Per § 713.06(2), required elements \
                 include lienor info, description of work, ordering party, and property \
                 description. Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: if text_lower.contains("notice") {
                Some(extract_snippet(text, "notice"))
            } else {
                Some(text.chars().take(100).collect())
            },
            text_position,
        });
    }

    // Check for 45-day service requirement
    let has_service_timing = text_lower.contains("45 day")
        || text_lower.contains("forty-five")
        || text_lower.contains("first furnish");

    if !has_service_timing {
        violations.push(Violation {
            statute: "F.S. § 713.06(2)(a)".to_string(),
            severity: Severity::Info,
            message: "Notice to Owner must be served within 45 days of first furnishing \
                     labor, services, or materials to preserve lien rights."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// § 713.08 - Claim of Lien
// ============================================================================

/// Check Claim of Lien compliance per § 713.08
///
/// Must be recorded within 90 days after final furnishing to preserve lien rights.
pub fn check_claim_of_lien(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required elements per § 713.08(4)

    // (a) Name and address of lienor
    let has_lienor = (text_lower.contains("lienor")
        || text_lower.contains("claimant")
        || text_lower.contains("contractor"))
        && text_lower.contains("address");

    // (b) Name of owner
    let has_owner = text_lower.contains("owner");

    // (c) Description of property
    let has_property = text_lower.contains("property")
        || text_lower.contains("legal description")
        || text_lower.contains("lot")
        || text_lower.contains("parcel");

    // (d) Description of improvement
    let has_improvement = text_lower.contains("improvement")
        || text_lower.contains("construction")
        || text_lower.contains("work");

    // (e) Amount owed
    let has_amount =
        text_lower.contains("amount") || Regex::new(r"\$[\d,]+").is_ok_and(|re| re.is_match(text));

    // (f) Time within which payment was to be made
    let has_payment_terms = text_lower.contains("due")
        || text_lower.contains("payment")
        || text_lower.contains("within");

    // (g) Signature
    let has_signature = text_lower.contains("signature")
        || text_lower.contains("signed")
        || text_lower.contains("sworn");

    let mut missing = Vec::new();
    if !has_lienor {
        missing.push("lienor name and address");
    }
    if !has_owner {
        missing.push("owner name");
    }
    if !has_property {
        missing.push("property description");
    }
    if !has_improvement {
        missing.push("improvement description");
    }
    if !has_amount {
        missing.push("amount owed");
    }
    if !has_payment_terms {
        missing.push("payment terms");
    }
    if !has_signature {
        missing.push("signature");
    }

    if !missing.is_empty() {
        let text_position = find_text_position(text, "lien").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });

        violations.push(Violation {
            statute: "F.S. § 713.08(4)".to_string(),
            severity: Severity::Critical,
            message: format!(
                "Claim of Lien is missing required elements. Per § 713.08(4), a claim of \
                 lien must include lienor info, owner name, property and improvement \
                 descriptions, amount owed, and signature. Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: if text_lower.contains("lien") {
                Some(extract_snippet(text, "lien"))
            } else {
                Some(text.chars().take(100).collect())
            },
            text_position,
        });
    }

    // Check for 90-day recording deadline warning
    let has_deadline_warning = text_lower.contains("90 day")
        || text_lower.contains("ninety")
        || text_lower.contains("final furnishing");

    if !has_deadline_warning {
        violations.push(Violation {
            statute: "F.S. § 713.08(5)".to_string(),
            severity: Severity::Warning,
            message: "Claim of Lien must be recorded within 90 days after final furnishing \
                     of labor, services, or materials. Consider adding deadline information."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// § 713.21 - Release of Lien
// ============================================================================

/// Check Release of Lien compliance per § 713.21
pub fn check_release_of_lien(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required elements for a valid lien release

    // Identification of the lien being released
    let has_lien_id = text_lower.contains("claim of lien")
        || text_lower.contains("recorded")
        || text_lower.contains("book")
        || text_lower.contains("page")
        || text_lower.contains("instrument");

    // Property description
    let has_property = text_lower.contains("property")
        || text_lower.contains("lot")
        || text_lower.contains("parcel");

    // Release language
    let has_release = text_lower.contains("release")
        || text_lower.contains("discharge")
        || text_lower.contains("satisfy")
        || text_lower.contains("waive");

    // Consideration (payment)
    let has_consideration = text_lower.contains("payment")
        || text_lower.contains("consideration")
        || text_lower.contains("received")
        || Regex::new(r"\$[\d,]+").is_ok_and(|re| re.is_match(text));

    // Signature
    let has_signature = text_lower.contains("signature")
        || text_lower.contains("signed")
        || text_lower.contains("notary");

    let mut missing = Vec::new();
    if !has_lien_id {
        missing.push("lien identification (recording info)");
    }
    if !has_property {
        missing.push("property description");
    }
    if !has_release {
        missing.push("release language");
    }
    if !has_consideration {
        missing.push("consideration/payment acknowledgment");
    }
    if !has_signature {
        missing.push("signature/notarization");
    }

    if !missing.is_empty() {
        violations.push(Violation {
            statute: "F.S. § 713.21".to_string(),
            severity: Severity::Warning,
            message: format!(
                "Release of Lien may be incomplete. Required elements include lien \
                 identification, property description, release language, and signature. \
                 Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// § 713.22 - Contest of Lien
// ============================================================================

/// Check Dispute/Contest of Lien compliance per § 713.22
pub fn check_dispute_lien(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required elements for contesting a lien

    // Identification of contested lien
    let has_lien_id = text_lower.contains("claim of lien")
        || text_lower.contains("recorded")
        || text_lower.contains("book")
        || text_lower.contains("instrument");

    // Grounds for contest
    let has_grounds = text_lower.contains("invalid")
        || text_lower.contains("improper")
        || text_lower.contains("defect")
        || text_lower.contains("untimely")
        || text_lower.contains("not owed")
        || text_lower.contains("paid")
        || text_lower.contains("dispute")
        || text_lower.contains("contest");

    // Property description
    let has_property = text_lower.contains("property") || text_lower.contains("address");

    let mut missing = Vec::new();
    if !has_lien_id {
        missing.push("identification of contested lien");
    }
    if !has_grounds {
        missing.push("grounds for contesting lien");
    }
    if !has_property {
        missing.push("property description");
    }

    if !missing.is_empty() {
        violations.push(Violation {
            statute: "F.S. § 713.22".to_string(),
            severity: Severity::Warning,
            message: format!(
                "Dispute of Lien may be incomplete. Should include identification of \
                 the contested lien, grounds for dispute, and property description. \
                 Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    // Check for 60-day enforcement deadline warning
    let has_deadline = text_lower.contains("60 day")
        || text_lower.contains("sixty")
        || text_lower.contains("1 year")
        || text_lower.contains("one year")
        || text_lower.contains("foreclose");

    if !has_deadline {
        violations.push(Violation {
            statute: "F.S. § 713.22(1)".to_string(),
            severity: Severity::Info,
            message: "Owner may shorten lienor's enforcement period to 60 days by filing \
                     Notice of Contest of Lien per § 713.22. Otherwise, lienor has 1 year \
                     from recording to enforce the lien."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// § 713.31 - Fraudulent Liens
// ============================================================================

/// Check Fraudulent Lien Report compliance per § 713.31
///
/// § 713.31 prohibits filing false liens and provides remedies including
/// attorney's fees and potential criminal penalties.
pub fn check_fraudulent_lien_report(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required elements for fraudulent lien report

    // Identification of alleged fraudulent lien
    let has_lien_id = text_lower.contains("claim of lien")
        || text_lower.contains("lien")
        || text_lower.contains("recorded");

    // Description of fraudulent conduct
    let has_fraud_desc = text_lower.contains("fraud")
        || text_lower.contains("false")
        || text_lower.contains("willful")
        || text_lower.contains("exaggerat")
        || text_lower.contains("no contract")
        || text_lower.contains("never performed");

    // Reference to § 713.31
    let has_statute_ref = text_lower.contains("713.31");

    // Damages claimed
    let has_damages = text_lower.contains("damage")
        || text_lower.contains("attorney")
        || text_lower.contains("fee")
        || text_lower.contains("cost");

    let mut missing = Vec::new();
    if !has_lien_id {
        missing.push("identification of allegedly fraudulent lien");
    }
    if !has_fraud_desc {
        missing.push("description of fraudulent conduct");
    }
    if !has_statute_ref {
        missing.push("reference to F.S. § 713.31");
    }
    if !has_damages {
        missing.push("damages/remedies sought");
    }

    if !missing.is_empty() {
        violations.push(Violation {
            statute: "F.S. § 713.31".to_string(),
            severity: Severity::Warning,
            message: format!(
                "Fraudulent Lien Report may be incomplete. Should include identification \
                 of the lien, description of fraudulent conduct, statutory reference, \
                 and damages sought. Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    // Info about remedies under § 713.31
    let has_remedy_info = text_lower.contains("treble") || text_lower.contains("criminal");

    if !has_remedy_info {
        violations.push(Violation {
            statute: "F.S. § 713.31(2)".to_string(),
            severity: Severity::Info,
            message: "Under § 713.31(2), a person who files a fraudulent lien is liable for \
                     actual damages, attorney's fees, and court costs. Willful exaggeration \
                     may result in liability for the exaggerated amount."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// § 713.06 - Final Payment Affidavit
// ============================================================================

/// Check Final Payment Affidavit compliance per § 713.06
pub fn check_final_payment_affidavit(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required elements per § 713.06(3)(d)

    // Affirmation that all lienors have been paid
    let has_payment_affirmation = text_lower.contains("paid")
        || text_lower.contains("satisfied")
        || text_lower.contains("all amounts");

    // List of subcontractors/suppliers
    let has_lienor_list = text_lower.contains("subcontractor")
        || text_lower.contains("supplier")
        || text_lower.contains("materialman")
        || text_lower.contains("lienor");

    // Sworn statement
    let has_sworn = text_lower.contains("swear")
        || text_lower.contains("affirm")
        || text_lower.contains("oath")
        || text_lower.contains("notary")
        || text_lower.contains("under penalty of perjury");

    // Property/job identification
    let has_property = text_lower.contains("property")
        || text_lower.contains("project")
        || text_lower.contains("job")
        || text_lower.contains("address");

    let mut missing = Vec::new();
    if !has_payment_affirmation {
        missing.push("affirmation that all lienors paid");
    }
    if !has_lienor_list {
        missing.push("list of subcontractors/suppliers");
    }
    if !has_sworn {
        missing.push("sworn statement/notarization");
    }
    if !has_property {
        missing.push("property/project identification");
    }

    if !missing.is_empty() {
        violations.push(Violation {
            statute: "F.S. § 713.06(3)(d)".to_string(),
            severity: if missing.len() >= 2 {
                Severity::Critical
            } else {
                Severity::Warning
            },
            message: format!(
                "Final Payment Affidavit may be incomplete. Per § 713.06(3)(d), contractor \
                 must provide sworn statement that all lienors have been paid. Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Contractor Invoice Checks
// ============================================================================

/// Check contractor invoice for Florida compliance
pub fn check_contractor_invoice(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required business information
    let has_contractor_name = text_lower.contains("contractor") || text_lower.contains("company");

    let has_license = text_lower.contains("license")
        || Regex::new(r"(?i)(CBC|CCC|CGC|CPC|CRC|CMC|CFC)\d+").is_ok_and(|re| re.is_match(text));

    let has_contact = text_lower.contains("phone")
        || text_lower.contains("address")
        || text_lower.contains("email");

    // Invoice specifics
    let has_amount = Regex::new(r"\$[\d,]+\.?\d*").is_ok_and(|re| re.is_match(text));

    let has_description = text_lower.contains("description")
        || text_lower.contains("services")
        || text_lower.contains("labor")
        || text_lower.contains("material");

    let has_property_ref = text_lower.contains("property")
        || text_lower.contains("address")
        || text_lower.contains("job site")
        || text_lower.contains("project");

    let mut missing = Vec::new();
    if !has_contractor_name {
        missing.push("contractor name");
    }
    if !has_license {
        missing.push("contractor license number");
    }
    if !has_contact {
        missing.push("contact information");
    }
    if !has_amount {
        missing.push("invoice amount");
    }
    if !has_description {
        missing.push("description of work");
    }
    if !has_property_ref {
        missing.push("property/project reference");
    }

    if !missing.is_empty() {
        violations.push(Violation {
            statute: "Florida Contractor Requirements".to_string(),
            severity: Severity::Warning,
            message: format!(
                "Contractor invoice may be incomplete. Professional invoices should include \
                 contractor license number, description of work, and property reference. \
                 Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    // Lien rights warning for subcontractors
    violations.extend(check_lien_rights_disclosure(text));

    violations
}

// ============================================================================
// Cost of Materials Bill
// ============================================================================

/// Check cost of materials bill for Florida compliance
pub fn check_cost_of_materials(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required elements
    let has_supplier_info = text_lower.contains("supplier")
        || text_lower.contains("vendor")
        || text_lower.contains("from");

    let has_materials_list = text_lower.contains("material")
        || text_lower.contains("item")
        || text_lower.contains("quantity");

    let has_amount = Regex::new(r"\$[\d,]+\.?\d*").is_ok_and(|re| re.is_match(text));

    let has_delivery_info = text_lower.contains("deliver")
        || text_lower.contains("ship")
        || text_lower.contains("date");

    let has_property_ref = text_lower.contains("property")
        || text_lower.contains("job")
        || text_lower.contains("project")
        || text_lower.contains("deliver");

    let mut missing = Vec::new();
    if !has_supplier_info {
        missing.push("supplier information");
    }
    if !has_materials_list {
        missing.push("materials list/description");
    }
    if !has_amount {
        missing.push("cost/amount");
    }
    if !has_delivery_info {
        missing.push("delivery date");
    }
    if !has_property_ref {
        missing.push("property/project reference");
    }

    if !missing.is_empty() {
        violations.push(Violation {
            statute: "Florida Materialman Requirements".to_string(),
            severity: Severity::Warning,
            message: format!(
                "Cost of Materials Bill may be incomplete. Should include supplier info, \
                 materials description, costs, and delivery information. Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    // Lien rights notice for material suppliers
    violations.extend(check_lien_rights_disclosure(text));

    violations
}

// ============================================================================
// § 713.04 - Lien Rights Disclosure
// ============================================================================

/// Check for lien rights disclosure on applicable documents
pub fn check_lien_rights_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if this appears to be a contractor/supplier document
    let is_contractor_doc = text_lower.contains("contractor")
        || text_lower.contains("subcontractor")
        || text_lower.contains("supplier")
        || text_lower.contains("materialman")
        || text_lower.contains("invoice")
        || text_lower.contains("bill");

    if !is_contractor_doc {
        return violations;
    }

    // Check for lien rights disclosure
    let has_lien_disclosure = text_lower.contains("lien right")
        || text_lower.contains("713.04")
        || text_lower.contains("notice to owner")
        || text_lower.contains("preliminary notice")
        || text_lower.contains("may have lien");

    if !has_lien_disclosure {
        violations.push(Violation {
            statute: "F.S. § 713.04".to_string(),
            severity: Severity::Info,
            message: "Consider including lien rights disclosure. Per § 713.04, contractors \
                     should inform property owners of potential lien rights held by \
                     subcontractors and suppliers."
                .to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Covered Statutes
// ============================================================================

/// Get list of statutes/codes covered for Florida construction liens
pub fn covered_contractor_statutes() -> Vec<&'static str> {
    vec![
        "F.S. § 713.13 - Notice of Commencement",
        "F.S. § 713.06 - Notice to Owner",
        "F.S. § 713.04 - Lien Rights Disclosure",
        "F.S. § 713.08 - Claim of Lien",
        "F.S. § 713.21 - Release of Lien",
        "F.S. § 713.22 - Contest of Lien",
        "F.S. § 713.31 - Fraudulent Liens",
        "F.S. § 713.346 - Final Payment Affidavit",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Document Type Detection
    // ========================================================================

    #[test]
    fn test_detect_notice_of_commencement() {
        let text = "NOTICE OF COMMENCEMENT per F.S. § 713.13. Owner: John Smith. \
                    Property at 123 Main St.";
        assert_eq!(
            ContractorDocumentType::detect(text),
            ContractorDocumentType::NoticeOfCommencement
        );
    }

    #[test]
    fn test_detect_notice_to_owner() {
        let text = "NOTICE TO OWNER - PRELIMINARY NOTICE. We have furnished or will furnish \
                    labor and materials. Lien rights preserved per § 713.06.";
        assert_eq!(
            ContractorDocumentType::detect(text),
            ContractorDocumentType::NoticeToOwner
        );
    }

    #[test]
    fn test_detect_claim_of_lien() {
        let text = "CLAIM OF LIEN per F.S. § 713.08. Amount owed: $15,000.";
        assert_eq!(
            ContractorDocumentType::detect(text),
            ContractorDocumentType::ClaimOfLien
        );
    }

    #[test]
    fn test_detect_release_of_lien() {
        let text = "RELEASE OF LIEN. The undersigned hereby releases and waives all lien \
                    rights per § 713.21.";
        assert_eq!(
            ContractorDocumentType::detect(text),
            ContractorDocumentType::ReleaseOfLien
        );
    }

    #[test]
    fn test_detect_contractor_invoice() {
        let text = "CONTRACTOR INVOICE. Services rendered: Labor for renovation project.";
        assert_eq!(
            ContractorDocumentType::detect(text),
            ContractorDocumentType::Invoice
        );
    }

    #[test]
    fn test_detect_cost_of_materials() {
        let text = "COST OF MATERIALS BILL. Supplies delivered: Lumber, nails, screws. \
                    Total amount due: $5,000.";
        assert_eq!(
            ContractorDocumentType::detect(text),
            ContractorDocumentType::CostOfMaterialsBill
        );
    }

    // ========================================================================
    // Notice of Commencement Tests
    // ========================================================================

    #[test]
    fn test_notice_of_commencement_complete() {
        let text = r#"
            NOTICE OF COMMENCEMENT
            Pursuant to F.S. § 713.13

            Legal Description: Lot 5, Block 2, Palm Beach Subdivision
            Property Address: 123 Main Street, Miami, FL 33101

            Improvement: Single-family residence renovation

            Owner Name: John Smith
            Owner Address: 123 Main Street, Miami, FL 33101

            Contractor Name: ABC Construction LLC
            Contractor Address: 456 Oak Ave, Miami, FL 33102
            Contractor License: CGC123456

            Surety: XYZ Bonding Company
            Bond Amount: $100,000

            Construction Lender: First National Bank
            Lender Address: 789 Finance Blvd, Miami, FL 33103

            Expiration Date: This Notice expires 1 year from recording.

            Owner Signature: ___________________
            Notary acknowledgment attached.

            To be recorded in Official Records of Miami-Dade County.
        "#;

        let violations = check_notice_of_commencement(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();

        assert!(
            critical.is_empty(),
            "Complete Notice of Commencement should pass. Got: {:?}",
            critical
        );
    }

    #[test]
    fn test_notice_of_commencement_missing_elements() {
        let text = "NOTICE OF COMMENCEMENT. Property at 123 Main St.";
        let violations = check_notice_of_commencement(text);

        assert!(
            !violations.is_empty(),
            "Incomplete Notice of Commencement should have violations"
        );
        assert!(violations.iter().any(|v| v.statute.contains("713.13")));
    }

    // ========================================================================
    // Notice to Owner Tests
    // ========================================================================

    #[test]
    fn test_notice_to_owner_complete() {
        let text = r#"
            NOTICE TO OWNER / PRELIMINARY NOTICE
            Pursuant to F.S. § 713.06

            Lienor Name: XYZ Plumbing Supply
            Lienor Address: 100 Pipe Street, Tampa, FL 33601

            WARNING: This is NOT a lien. This is a notice required by Florida law
            to preserve lien rights.

            We have furnished or will furnish labor, services, or materials:
            Plumbing fixtures and supplies

            To or ordered by: ABC Construction LLC (Contractor)

            Property Description: 123 Main Street, Miami, FL 33101

            Served within 45 days of first furnishing materials.
        "#;

        let violations = check_notice_to_owner(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();

        assert!(
            critical.is_empty(),
            "Complete Notice to Owner should pass. Got: {:?}",
            critical
        );
    }

    #[test]
    fn test_notice_to_owner_missing_warning() {
        let text = "NOTICE TO OWNER. Lienor: XYZ Supply. Property at 123 Main St. \
                    Furnishing materials to ABC Construction.";
        let violations = check_notice_to_owner(text);

        assert!(
            violations
                .iter()
                .any(|v| v.message.to_lowercase().contains("warning")),
            "Should flag missing warning language"
        );
    }

    // ========================================================================
    // Claim of Lien Tests
    // ========================================================================

    #[test]
    fn test_claim_of_lien_complete() {
        let text = r#"
            CLAIM OF LIEN
            Pursuant to F.S. § 713.08

            Lienor/Claimant Name: XYZ Plumbing LLC
            Lienor Address: 100 Pipe Street, Tampa, FL 33601

            Owner Name: John Smith

            Property Description: Lot 5, Block 2, Palm Beach Subdivision
            Property Address: 123 Main Street, Miami, FL 33101

            Improvement: Plumbing work for bathroom renovation

            Amount Owed: $15,000.00
            Due within 30 days of invoice date

            Claim must be filed within 90 days after final furnishing.

            Signature: ___________________
            Date: January 15, 2026
        "#;

        let violations = check_claim_of_lien(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();

        assert!(
            critical.is_empty(),
            "Complete Claim of Lien should pass. Got: {:?}",
            critical
        );
    }

    #[test]
    fn test_claim_of_lien_missing_amount() {
        let text = "CLAIM OF LIEN. Lienor: XYZ LLC. Owner: John Smith. Property at 123 Main.";
        let violations = check_claim_of_lien(text);

        assert!(
            violations.iter().any(|v| v.statute.contains("713.08")),
            "Should flag incomplete Claim of Lien"
        );
    }

    // ========================================================================
    // Fraudulent Lien Tests
    // ========================================================================

    #[test]
    fn test_fraudulent_lien_report_complete() {
        let text = r#"
            FRAUDULENT LIEN REPORT
            Pursuant to F.S. § 713.31

            The following Claim of Lien is alleged to be fraudulent:
            - Recorded in Official Records Book 123, Page 456
            - Filed by: Fake Contractor LLC

            Fraudulent Conduct:
            The claimant never performed any work on the property. This is a
            false and willful claim with no contract or basis.

            Property: 123 Main Street, Miami, FL 33101

            Damages Sought:
            - Actual damages
            - Attorney's fees and costs per § 713.31(2)
        "#;

        let violations = check_fraudulent_lien_report(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();

        assert!(
            critical.is_empty(),
            "Complete Fraudulent Lien Report should pass. Got: {:?}",
            critical
        );
    }

    // ========================================================================
    // Contractor Invoice Tests
    // ========================================================================

    #[test]
    fn test_contractor_invoice_complete() {
        let text = r#"
            ABC CONSTRUCTION LLC
            CONTRACTOR INVOICE

            License: CGC123456
            Phone: (305) 555-1234
            Address: 456 Oak Ave, Miami, FL 33102

            Invoice #: 2026-001
            Date: January 15, 2026

            Property/Job Site: 123 Main Street, Miami, FL 33101

            Description of Services:
            - Labor for kitchen renovation
            - Material installation

            Amount Due: $25,000.00
        "#;

        let violations = check_contractor_invoice(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();

        assert!(
            critical.is_empty(),
            "Complete contractor invoice should pass. Got: {:?}",
            critical
        );
    }

    #[test]
    fn test_contractor_invoice_missing_license() {
        let text = "INVOICE from ABC Construction. Services: Labor. Amount: $5,000.";
        let violations = check_contractor_invoice(text);

        assert!(
            violations
                .iter()
                .any(|v| v.message.to_lowercase().contains("license")),
            "Should flag missing contractor license"
        );
    }

    // ========================================================================
    // Final Payment Affidavit Tests
    // ========================================================================

    #[test]
    fn test_final_payment_affidavit_complete() {
        let text = r#"
            CONTRACTOR'S FINAL PAYMENT AFFIDAVIT
            Pursuant to F.S. § 713.06(3)(d)

            Project: 123 Main Street, Miami, FL 33101

            The undersigned contractor does hereby swear and affirm under oath:

            All subcontractors, suppliers, and materialmen have been paid in full
            for all labor, services, and materials furnished for this project.

            Subcontractors/Suppliers Paid:
            - XYZ Plumbing LLC - Paid in Full
            - ABC Electrical Inc - Paid in Full
            - Miami Lumber Supply - Paid in Full

            All amounts due have been satisfied.

            Sworn before notary on this date.
            Notary Seal attached.
        "#;

        let violations = check_final_payment_affidavit(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();

        assert!(
            critical.is_empty(),
            "Complete Final Payment Affidavit should pass. Got: {:?}",
            critical
        );
    }
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: Document type detection should never panic
        #[test]
        fn document_type_detection_no_panic(text in "\\PC*") {
            let _ = ContractorDocumentType::detect(&text);
        }

        /// Property: All compliance checks should never panic on arbitrary input
        #[test]
        fn compliance_checks_no_panic(text in "\\PC*") {
            let _ = check_florida_contractor_compliance(&text);
            let _ = check_notice_of_commencement(&text);
            let _ = check_notice_to_owner(&text);
            let _ = check_claim_of_lien(&text);
            let _ = check_release_of_lien(&text);
            let _ = check_dispute_lien(&text);
            let _ = check_fraudulent_lien_report(&text);
            let _ = check_final_payment_affidavit(&text);
            let _ = check_contractor_invoice(&text);
            let _ = check_cost_of_materials(&text);
            let _ = check_lien_rights_disclosure(&text);
        }

        /// Property: Notice of Commencement with all keywords should detect correctly
        #[test]
        fn noc_detection_with_keywords(
            prefix in "\\PC{0,50}",
            suffix in "\\PC{0,50}"
        ) {
            let text = format!(
                "{} NOTICE OF COMMENCEMENT for owner at property address {}",
                prefix, suffix
            );
            let doc_type = ContractorDocumentType::detect(&text);
            prop_assert_eq!(doc_type, ContractorDocumentType::NoticeOfCommencement);
        }

        /// Property: Claim of Lien with amount should not flag missing amount
        #[test]
        fn claim_of_lien_amount_validation(amount in 1000u32..1000000u32) {
            let text = format!(
                "CLAIM OF LIEN. Lienor: ABC LLC address 123 Main. Owner: John Smith. \
                 Property: 123 Main St. Improvement: Construction. \
                 Amount owed: ${}.00. Due within 30 days. Signature attached.",
                amount
            );
            let violations = check_claim_of_lien(&text);

            // With amount present, the "Missing:" list should not contain "amount owed"
            // Note: The message template always mentions "amount owed" in the description,
            // so we check specifically for the "Missing: ...amount owed" pattern
            prop_assert!(
                !violations.iter().any(|v| v.message.contains("Missing:") &&
                    v.message.split("Missing:").nth(1).is_some_and(|s| s.contains("amount"))),
                "Should not flag amount as missing when present. Got: {:?}",
                violations
            );
        }

        /// Property: Missing owner info should flag Notice of Commencement
        #[test]
        fn noc_requires_owner_info(
            contractor in "[A-Z][a-z]+ Construction",
            property in "[0-9]+ [A-Za-z]+ Street"
        ) {
            let text = format!(
                "NOTICE OF COMMENCEMENT. Contractor: {}. Property: {}. \
                 Improvement: Renovation. Bond: None. Lender: None. \
                 Expires: 1 year. Signature attached.",
                contractor, property
            );
            let violations = check_notice_of_commencement(&text);

            // Should flag missing owner info
            prop_assert!(
                violations.iter().any(|v| v.message.to_lowercase().contains("owner")),
                "Should flag missing owner info. Got: {:?}",
                violations
            );
        }
    }
}
