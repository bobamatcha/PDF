//! Florida Real Estate Transaction Compliance
//!
//! Compliance rules for Florida real estate purchase contracts, escalation
//! addendums, and listing agreements.
//!
//! Key Statutes:
//! - § 404.056 - Radon Gas Disclosure
//! - § 689.261 - Property Tax Disclosure
//! - § 689.302 - Flood Disclosure (SB 948, October 2025)
//! - § 720.401 - HOA Disclosure
//! - § 553.996 - Energy Efficiency Disclosure
//! - § 475.278 - Brokerage Relationship Disclosure
//! - § 475.25 - Definite Expiration Date (Listing Agreements)
//! - 42 U.S.C. § 4852d - Lead Paint Disclosure (pre-1978)
//! - Johnson v. Davis (1985) - Material Defect Disclosure

use crate::patterns::{extract_snippet, find_text_position};
use regex::Regex;
use shared_types::{Severity, TextPosition, Violation};

/// Document type for Florida real estate transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealEstateDocumentType {
    /// Residential purchase contract
    PurchaseContract,
    /// Escalation addendum (price escalation clause)
    EscalationAddendum,
    /// Listing agreement (agent/seller)
    ListingAgreement,
    /// Unknown document type
    Unknown,
}

impl RealEstateDocumentType {
    /// Detect document type from text content
    pub fn detect(text: &str) -> Self {
        let text_lower = text.to_lowercase();

        // Check for escalation addendum first (most specific)
        if text_lower.contains("escalation")
            && (text_lower.contains("addendum") || text_lower.contains("clause"))
            && text_lower.contains("maximum")
            && text_lower.contains("purchase price")
        {
            return Self::EscalationAddendum;
        }

        // Check for listing agreement
        if (text_lower.contains("listing agreement") || text_lower.contains("exclusive listing"))
            && (text_lower.contains("broker") || text_lower.contains("agent"))
            && text_lower.contains("commission")
        {
            return Self::ListingAgreement;
        }

        // Check for purchase contract
        if (text_lower.contains("purchase")
            || text_lower.contains("sale")
            || text_lower.contains("contract"))
            && (text_lower.contains("buyer") || text_lower.contains("purchaser"))
            && (text_lower.contains("seller") || text_lower.contains("vendor"))
            && (text_lower.contains("property") || text_lower.contains("real estate"))
        {
            return Self::PurchaseContract;
        }

        Self::Unknown
    }
}

/// Check all Florida real estate compliance requirements
///
/// Automatically detects document type and applies appropriate rules.
pub fn check_florida_realestate_compliance(text: &str) -> Vec<Violation> {
    let doc_type = RealEstateDocumentType::detect(text);

    match doc_type {
        RealEstateDocumentType::PurchaseContract => check_purchase_contract(text),
        RealEstateDocumentType::EscalationAddendum => check_escalation_addendum(text),
        RealEstateDocumentType::ListingAgreement => check_listing_agreement(text),
        RealEstateDocumentType::Unknown => {
            // If unknown, apply general real estate checks
            let mut violations = Vec::new();
            violations.extend(check_radon_disclosure(text));
            violations.extend(check_lead_paint_disclosure(text, None));
            violations
        }
    }
}

/// Check compliance for a specific document type
pub fn check_document_type(text: &str, doc_type: RealEstateDocumentType) -> Vec<Violation> {
    match doc_type {
        RealEstateDocumentType::PurchaseContract => check_purchase_contract(text),
        RealEstateDocumentType::EscalationAddendum => check_escalation_addendum(text),
        RealEstateDocumentType::ListingAgreement => check_listing_agreement(text),
        RealEstateDocumentType::Unknown => check_florida_realestate_compliance(text),
    }
}

// ============================================================================
// Purchase Contract Compliance
// ============================================================================

/// Check Florida purchase contract compliance
pub fn check_purchase_contract(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Mandatory disclosures
    violations.extend(check_radon_disclosure(text));
    violations.extend(check_property_tax_disclosure(text));
    violations.extend(check_flood_disclosure_realestate(text));
    violations.extend(check_hoa_disclosure(text));
    violations.extend(check_energy_efficiency_disclosure(text));
    violations.extend(check_material_defect_disclosure(text));

    // Lead paint (year-dependent)
    violations.extend(check_lead_paint_disclosure(text, None));

    // Contract structure
    violations.extend(check_earnest_money(text));
    violations.extend(check_closing_provisions(text));

    violations
}

// ============================================================================
// § 404.056 - Radon Gas Disclosure
// ============================================================================

/// Check for required radon gas disclosure per Florida Statute § 404.056
pub fn check_radon_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required elements of radon disclosure
    let has_radon_header = text_lower.contains("radon")
        && (text_lower.contains("disclosure") || text_lower.contains("404.056"));

    let has_radon_gas_explanation = text_lower.contains("radon")
        && text_lower.contains("gas")
        && (text_lower.contains("naturally occurring")
            || text_lower.contains("radioactive")
            || text_lower.contains("colorless")
            || text_lower.contains("odorless"));

    let has_health_warning = text_lower.contains("health risk")
        || text_lower.contains("lung cancer")
        || text_lower.contains("hazard");

    let has_testing_recommendation = text_lower.contains("test") && text_lower.contains("radon");

    // Check statutory language presence
    let has_statutory_language = text_lower.contains("404.056")
        || (text_lower.contains("florida statutes") && text_lower.contains("radon"));

    let is_compliant = has_radon_header
        && has_radon_gas_explanation
        && has_health_warning
        && has_testing_recommendation;

    if !is_compliant {
        let text_position = find_text_position(text, "radon").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });

        let mut missing = Vec::new();
        if !has_radon_header {
            missing.push("Radon Disclosure header");
        }
        if !has_radon_gas_explanation {
            missing.push("explanation of radon gas");
        }
        if !has_health_warning {
            missing.push("health risk warning");
        }
        if !has_testing_recommendation {
            missing.push("testing recommendation");
        }
        if !has_statutory_language {
            missing.push("§ 404.056 reference");
        }

        violations.push(Violation {
            statute: "F.S. § 404.056".to_string(),
            severity: Severity::Critical,
            message: format!(
                "Missing required radon gas disclosure. Florida law requires sellers to provide \
                 written disclosure about radon gas. Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: if text_lower.contains("radon") {
                Some(extract_snippet(text, "radon"))
            } else {
                Some(text.chars().take(100).collect())
            },
            text_position,
        });
    }

    violations
}

// ============================================================================
// § 689.261 - Property Tax Disclosure
// ============================================================================

/// Check for required property tax disclosure per Florida Statute § 689.261
pub fn check_property_tax_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required elements
    let has_tax_disclosure = text_lower.contains("property tax")
        || text_lower.contains("ad valorem")
        || text_lower.contains("689.261");

    let has_reassessment_warning = text_lower.contains("reassess")
        || text_lower.contains("change of ownership")
        || text_lower.contains("taxes may increase");

    let has_exemption_info = text_lower.contains("homestead") || text_lower.contains("exemption");

    let is_compliant = has_tax_disclosure && has_reassessment_warning;

    if !is_compliant {
        let text_position = find_text_position(text, "tax").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });

        let mut missing = Vec::new();
        if !has_tax_disclosure {
            missing.push("property tax disclosure");
        }
        if !has_reassessment_warning {
            missing.push("reassessment warning");
        }
        if !has_exemption_info {
            missing.push("homestead exemption information");
        }

        violations.push(Violation {
            statute: "F.S. § 689.261".to_string(),
            severity: Severity::Critical,
            message: format!(
                "Missing required property tax disclosure. Buyers must be informed that property \
                 taxes may substantially increase upon change of ownership. Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: if text_lower.contains("tax") {
                Some(extract_snippet(text, "tax"))
            } else {
                Some(text.chars().take(100).collect())
            },
            text_position,
        });
    }

    violations
}

// ============================================================================
// § 689.302 - Flood Disclosure (SB 948 - October 2025)
// ============================================================================

/// Check for required flood disclosure per Florida Statute § 689.302
///
/// Effective October 1, 2025, sellers must disclose:
/// 1. Known prior flooding
/// 2. Flood insurance claims history
/// 3. Federal flood assistance received
pub fn check_flood_disclosure_realestate(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required disclosure elements per SB 948
    let has_flood_header =
        text_lower.contains("flood disclosure") || text_lower.contains("689.302");

    let has_flooding_history = text_lower.contains("knowledge of")
        && (text_lower.contains("flood") || text_lower.contains("flooding"))
        || text_lower.contains("prior flooding")
        || text_lower.contains("past flooding")
        || text_lower.contains("no knowledge of flood");

    let has_insurance_claims = (text_lower.contains("flood insurance")
        && text_lower.contains("claim"))
        || (text_lower.contains("insurance claim") && text_lower.contains("flood"));

    let has_federal_assistance = text_lower.contains("fema")
        || (text_lower.contains("federal")
            && (text_lower.contains("flood") || text_lower.contains("assistance")))
        || text_lower.contains("federal flood assistance");

    let is_fully_compliant =
        has_flood_header && has_flooding_history && has_insurance_claims && has_federal_assistance;

    if !is_fully_compliant {
        let text_position = find_text_position(text, "flood").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });

        let mut missing = Vec::new();
        if !has_flood_header {
            missing.push("Flood Disclosure header/§ 689.302 reference");
        }
        if !has_flooding_history {
            missing.push("disclosure of seller's knowledge of past flooding");
        }
        if !has_insurance_claims {
            missing.push("disclosure of flood insurance claims");
        }
        if !has_federal_assistance {
            missing.push("disclosure of federal flood assistance (FEMA)");
        }

        violations.push(Violation {
            statute: "F.S. § 689.302 (SB 948)".to_string(),
            severity: Severity::Critical,
            message: format!(
                "Missing required flood disclosure elements per § 689.302. Effective October 1, 2025, \
                 sellers must disclose flooding history, insurance claims, and federal assistance. Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: if text_lower.contains("flood") {
                Some(extract_snippet(text, "flood"))
            } else {
                Some(text.chars().take(100).collect())
            },
            text_position,
        });
    }

    violations
}

// ============================================================================
// § 720.401 - HOA Disclosure
// ============================================================================

/// Check for required HOA disclosure per Florida Statute § 720.401
pub fn check_hoa_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check if property appears to have HOA
    let has_hoa_mention = text_lower.contains("hoa")
        || text_lower.contains("homeowner")
        || text_lower.contains("association")
        || text_lower.contains("720.401");

    if !has_hoa_mention {
        // If no HOA mentioned, check if there's an explicit statement
        let has_no_hoa_statement =
            text_lower.contains("no hoa") || text_lower.contains("not subject to");

        if !has_no_hoa_statement {
            violations.push(Violation {
                statute: "F.S. § 720.401".to_string(),
                severity: Severity::Warning,
                message: "Contract should indicate whether property is subject to HOA. \
                         If applicable, buyer must receive HOA disclosure summary."
                    .to_string(),
                page: None,
                text_snippet: Some(text.chars().take(100).collect()),
                text_position: None,
            });
        }
        return violations;
    }

    // If HOA exists, check for required disclosure elements
    let has_assessment_info = text_lower.contains("assessment")
        || text_lower.contains("dues")
        || text_lower.contains("fee");

    let has_contact_info = text_lower.contains("contact")
        || text_lower.contains("management")
        || text_lower.contains("address");

    let has_document_reference = text_lower.contains("governing document")
        || text_lower.contains("declaration")
        || text_lower.contains("covenants")
        || text_lower.contains("bylaws");

    let has_cancellation_right = text_lower.contains("cancel")
        || text_lower.contains("rescind")
        || text_lower.contains("3 days")
        || text_lower.contains("three days");

    if !has_assessment_info || !has_contact_info {
        let mut missing = Vec::new();
        if !has_assessment_info {
            missing.push("HOA assessment/fee amounts");
        }
        if !has_contact_info {
            missing.push("HOA contact information");
        }
        if !has_document_reference {
            missing.push("reference to governing documents");
        }
        if !has_cancellation_right {
            missing.push("buyer's 3-day cancellation right");
        }

        violations.push(Violation {
            statute: "F.S. § 720.401".to_string(),
            severity: Severity::Warning,
            message: format!(
                "HOA disclosure may be incomplete. § 720.401 requires disclosure of \
                 assessment amounts, governing documents, and buyer's cancellation rights. Missing: {}",
                missing.join(", ")
            ),
            page: None,
            text_snippet: Some(extract_snippet(text, "hoa")),
            text_position: find_text_position(text, "hoa").map(|(start, end)| TextPosition {
                start_offset: start,
                end_offset: end,
            }),
        });
    }

    violations
}

// ============================================================================
// § 553.996 - Energy Efficiency Disclosure
// ============================================================================

/// Check for required energy efficiency disclosure per Florida Statute § 553.996
pub fn check_energy_efficiency_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let has_energy_disclosure = text_lower.contains("energy")
        && (text_lower.contains("efficiency") || text_lower.contains("rating"))
        || text_lower.contains("553.996");

    let has_florida_energy_code =
        text_lower.contains("florida energy code") || text_lower.contains("building code");

    let _has_rating_info = text_lower.contains("energy rating")
        || text_lower.contains("hers")
        || text_lower.contains("energy performance");

    if !has_energy_disclosure && !has_florida_energy_code {
        violations.push(Violation {
            statute: "F.S. § 553.996".to_string(),
            severity: Severity::Info,
            message: "Energy efficiency disclosure recommended. Florida law requires disclosure \
                     of energy-efficiency rating information when available."
                .to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Johnson v. Davis - Material Defect Disclosure
// ============================================================================

/// Check for material defect disclosure per Johnson v. Davis (1985)
pub fn check_material_defect_disclosure(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let has_defect_disclosure = text_lower.contains("defect")
        || text_lower.contains("disclose")
        || text_lower.contains("condition");

    let has_seller_disclosure_section = text_lower.contains("seller disclosure")
        || text_lower.contains("seller's disclosure")
        || text_lower.contains("property disclosure");

    let has_known_issues_section = text_lower.contains("known")
        && (text_lower.contains("defect")
            || text_lower.contains("problem")
            || text_lower.contains("issue"));

    if !has_defect_disclosure || !has_seller_disclosure_section || !has_known_issues_section {
        violations.push(Violation {
            statute: "Johnson v. Davis (Fla. 1985)".to_string(),
            severity: Severity::Warning,
            message: "Contract should include seller's disclosure of known material defects. \
                     Under Johnson v. Davis, sellers must disclose facts materially affecting \
                     property value that are not readily observable."
                .to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Lead Paint Disclosure (Pre-1978)
// ============================================================================

/// Check for lead paint disclosure required for pre-1978 properties
pub fn check_lead_paint_disclosure(text: &str, year_built: Option<u32>) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Try to detect year from text if not provided
    let effective_year = year_built.or_else(|| extract_year_built(&text_lower));

    // Check if property is pre-1978
    // If we have an explicit year, use it; otherwise fall back to text pattern matching
    let is_pre_1978 = effective_year.map(|y| y < 1978).unwrap_or(false);

    // Only check for "pre-1978" mentions if we don't have an explicit year
    // Note: Just mentioning "1978" isn't enough - we need explicit pre-1978 indicators
    let mentions_pre_1978 = effective_year.is_none()
        && (text_lower.contains("pre-1978")
            || text_lower.contains("before 1978")
            || text_lower.contains("prior to 1978")
            || (text_lower.contains("built")
                && text_lower.contains("before")
                && text_lower.contains("1978")));

    // If post-1978 or unknown (with no pre-1978 mentions), skip
    if !is_pre_1978 && !mentions_pre_1978 {
        return violations;
    }

    // Check for lead paint disclosure elements
    let has_lead_disclosure = text_lower.contains("lead")
        && (text_lower.contains("paint") || text_lower.contains("based"));

    let has_epa_pamphlet = text_lower.contains("protect your family")
        || text_lower.contains("epa")
        || text_lower.contains("pamphlet");

    let has_disclosure_form = text_lower.contains("lead-based paint disclosure")
        || text_lower.contains("lead disclosure");

    let has_inspection_opportunity = text_lower.contains("inspection")
        && text_lower.contains("lead")
        || text_lower.contains("10 days")
        || text_lower.contains("ten days");

    if !has_lead_disclosure {
        violations.push(Violation {
            statute: "42 U.S.C. § 4852d".to_string(),
            severity: Severity::Critical,
            message: "Missing required lead-based paint disclosure. Federal law requires sellers \
                     of pre-1978 housing to disclose known lead-based paint hazards."
                .to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    } else {
        // Check for complete disclosure
        let mut missing = Vec::new();
        if !has_epa_pamphlet {
            missing.push("EPA pamphlet provision");
        }
        if !has_disclosure_form {
            missing.push("lead-based paint disclosure form");
        }
        if !has_inspection_opportunity {
            missing.push("10-day inspection opportunity");
        }

        if !missing.is_empty() {
            violations.push(Violation {
                statute: "42 U.S.C. § 4852d".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Lead-based paint disclosure may be incomplete. Missing: {}",
                    missing.join(", ")
                ),
                page: None,
                text_snippet: Some(extract_snippet(text, "lead")),
                text_position: find_text_position(text, "lead").map(|(start, end)| TextPosition {
                    start_offset: start,
                    end_offset: end,
                }),
            });
        }
    }

    violations
}

fn extract_year_built(text: &str) -> Option<u32> {
    // Match patterns like "built in 1965", "year built: 1970", "constructed 1955"
    let patterns = [
        r"(?:built|constructed|year\s*built)[:\s]+(\d{4})",
        r"(\d{4})\s*(?:construction|built)",
    ];

    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(text) {
                if let Some(year_match) = caps.get(1) {
                    if let Ok(year) = year_match.as_str().parse::<u32>() {
                        if (1900..=2030).contains(&year) {
                            return Some(year);
                        }
                    }
                }
            }
        }
    }
    None
}

// ============================================================================
// Earnest Money / Escrow Requirements
// ============================================================================

/// Check earnest money and escrow provisions
pub fn check_earnest_money(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let has_earnest_money = text_lower.contains("earnest money")
        || text_lower.contains("deposit")
        || text_lower.contains("escrow");

    if !has_earnest_money {
        return violations;
    }

    // Check for escrow agent
    let has_escrow_agent = text_lower.contains("escrow agent")
        || text_lower.contains("title company")
        || text_lower.contains("closing agent");

    // Check for deposit due date
    let has_deposit_timing = text_lower.contains("within")
        && (text_lower.contains("day") || text_lower.contains("business"));

    // Check for dispute provisions
    let has_dispute_provisions = text_lower.contains("dispute")
        || text_lower.contains("interplead")
        || text_lower.contains("stakeholder");

    if !has_escrow_agent {
        violations.push(Violation {
            statute: "Florida Escrow Requirements".to_string(),
            severity: Severity::Warning,
            message: "Earnest money provisions should identify the escrow agent/title company \
                     who will hold the deposit."
                .to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "earnest")),
            text_position: None,
        });
    }

    if !has_deposit_timing {
        violations.push(Violation {
            statute: "Florida Escrow Requirements".to_string(),
            severity: Severity::Info,
            message: "Consider specifying when earnest money deposit is due (e.g., within 3 business days)."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    if !has_dispute_provisions {
        violations.push(Violation {
            statute: "Florida Escrow Requirements".to_string(),
            severity: Severity::Info,
            message: "Consider including provisions for earnest money disputes \
                     (e.g., interpleader procedure)."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Closing Provisions
// ============================================================================

/// Check closing date and provisions
pub fn check_closing_provisions(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for closing date
    let has_closing_date = text_lower.contains("closing date")
        || text_lower.contains("close on")
        || text_lower.contains("settlement date");

    if !has_closing_date {
        violations.push(Violation {
            statute: "Florida Contract Requirements".to_string(),
            severity: Severity::Warning,
            message: "Contract should specify a closing date or timeframe.".to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    // Check for title insurance provisions
    let has_title_insurance =
        text_lower.contains("title insurance") || text_lower.contains("title policy");

    if !has_title_insurance {
        violations.push(Violation {
            statute: "Florida Title Requirements".to_string(),
            severity: Severity::Info,
            message: "Consider including title insurance provisions.".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Escalation Addendum Compliance
// ============================================================================

/// Check Florida escalation addendum compliance
pub fn check_escalation_addendum(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Required elements
    let has_base_price = text_lower.contains("base")
        && (text_lower.contains("price") || text_lower.contains("offer"));

    let has_increment = text_lower.contains("increment")
        || text_lower.contains("escalate")
        || text_lower.contains("exceed");

    let has_maximum = text_lower.contains("maximum")
        || text_lower.contains("cap")
        || text_lower.contains("not to exceed");

    let has_bona_fide = text_lower.contains("bona fide")
        || text_lower.contains("good faith")
        || text_lower.contains("competing offer");

    let has_proof_requirement = text_lower.contains("proof")
        || text_lower.contains("copy")
        || text_lower.contains("evidence");

    // Check for complete escalation clause structure
    if !has_base_price {
        violations.push(Violation {
            statute: "Florida Escalation Clause Requirements".to_string(),
            severity: Severity::Critical,
            message: "Escalation addendum must specify the base purchase price.".to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    if !has_increment {
        violations.push(Violation {
            statute: "Florida Escalation Clause Requirements".to_string(),
            severity: Severity::Warning,
            message: "Escalation addendum should specify the escalation increment amount."
                .to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    if !has_maximum {
        violations.push(Violation {
            statute: "Florida Escalation Clause Requirements".to_string(),
            severity: Severity::Critical,
            message: "Escalation addendum must specify a maximum purchase price cap.".to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    if !has_bona_fide || !has_proof_requirement {
        violations.push(Violation {
            statute: "Florida Escalation Clause Best Practices".to_string(),
            severity: Severity::Warning,
            message: "Escalation clause should require proof of bona fide competing offer. \
                     Consider requiring a copy of the competing offer or written verification."
                .to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    // Check for appraisal gap provisions
    let has_appraisal_gap = text_lower.contains("appraisal")
        && (text_lower.contains("gap") || text_lower.contains("difference"));

    if !has_appraisal_gap {
        violations.push(Violation {
            statute: "Florida Escalation Clause Best Practices".to_string(),
            severity: Severity::Info,
            message: "Consider including appraisal gap coverage provisions in escalation clause."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

// ============================================================================
// Listing Agreement Compliance
// ============================================================================

/// Check Florida listing agreement compliance
pub fn check_listing_agreement(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Check brokerage relationship disclosure (§ 475.278)
    violations.extend(check_brokerage_relationship(text));

    // Check definite expiration date (§ 475.25)
    violations.extend(check_listing_expiration(text));

    // Check commission provisions
    violations.extend(check_commission_provisions(text));

    // Check broker license
    violations.extend(check_broker_license(text));

    violations
}

/// Check brokerage relationship disclosure per § 475.278
pub fn check_brokerage_relationship(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let has_relationship_disclosure = text_lower.contains("brokerage relationship")
        || text_lower.contains("475.278")
        || text_lower.contains("agency relationship");

    let has_single_agent = text_lower.contains("single agent");
    let has_transaction_broker = text_lower.contains("transaction broker");
    let has_no_brokerage = text_lower.contains("no brokerage relationship");

    let has_relationship_type = has_single_agent || has_transaction_broker || has_no_brokerage;

    if !has_relationship_disclosure {
        violations.push(Violation {
            statute: "F.S. § 475.278".to_string(),
            severity: Severity::Critical,
            message: "Missing required brokerage relationship disclosure. Florida law requires \
                     written disclosure of the brokerage relationship before signing a listing agreement."
                .to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    } else if !has_relationship_type {
        violations.push(Violation {
            statute: "F.S. § 475.278".to_string(),
            severity: Severity::Warning,
            message: "Brokerage relationship disclosure should specify the type: \
                     single agent, transaction broker, or no brokerage relationship."
                .to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "brokerage")),
            text_position: find_text_position(text, "brokerage").map(|(start, end)| TextPosition {
                start_offset: start,
                end_offset: end,
            }),
        });
    }

    // Check for duties disclosure based on relationship type
    if has_single_agent {
        let has_duties = text_lower.contains("duties")
            || text_lower.contains("fiduciary")
            || text_lower.contains("loyalty")
            || text_lower.contains("confidentiality");

        if !has_duties {
            violations.push(Violation {
                statute: "F.S. § 475.278".to_string(),
                severity: Severity::Warning,
                message: "Single agent relationship should include disclosure of agent's duties \
                         (loyalty, confidentiality, obedience, full disclosure, accounting, skill/care/diligence)."
                    .to_string(),
                page: None,
                text_snippet: None,
                text_position: None,
            });
        }
    }

    violations
}

/// Check for definite expiration date per § 475.25
pub fn check_listing_expiration(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let has_expiration = text_lower.contains("expir")
        || text_lower.contains("terminat")
        || text_lower.contains("end date");

    let has_specific_date = Regex::new(r"\b\d{1,2}[/\-]\d{1,2}[/\-]\d{2,4}\b")
        .map(|re| re.is_match(&text_lower))
        .unwrap_or(false)
        || text_lower.contains("january")
        || text_lower.contains("february")
        || text_lower.contains("march")
        || text_lower.contains("april")
        || text_lower.contains("may")
        || text_lower.contains("june")
        || text_lower.contains("july")
        || text_lower.contains("august")
        || text_lower.contains("september")
        || text_lower.contains("october")
        || text_lower.contains("november")
        || text_lower.contains("december");

    if !has_expiration || !has_specific_date {
        violations.push(Violation {
            statute: "F.S. § 475.25".to_string(),
            severity: Severity::Critical,
            message: "Listing agreement must have a definite expiration date. \
                     Florida law prohibits listing agreements without specific termination dates."
                .to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
    }

    violations
}

/// Check commission provisions
pub fn check_commission_provisions(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let has_commission = text_lower.contains("commission")
        || text_lower.contains("compensation")
        || text_lower.contains("fee");

    if !has_commission {
        violations.push(Violation {
            statute: "Florida Listing Agreement Requirements".to_string(),
            severity: Severity::Critical,
            message: "Listing agreement must specify commission/compensation terms.".to_string(),
            page: None,
            text_snippet: Some(text.chars().take(100).collect()),
            text_position: None,
        });
        return violations;
    }

    // Check for commission amount
    let has_percentage = Regex::new(r"\d+\.?\d*\s*%")
        .map(|re| re.is_match(&text_lower))
        .unwrap_or(false);

    let has_flat_fee = text_lower.contains("flat fee")
        || text_lower.contains("fixed fee")
        || Regex::new(r"\$\d+")
            .map(|re| re.is_match(&text_lower))
            .unwrap_or(false);

    if !has_percentage && !has_flat_fee {
        violations.push(Violation {
            statute: "Florida Listing Agreement Requirements".to_string(),
            severity: Severity::Warning,
            message: "Commission terms should specify the rate (percentage) or flat fee amount."
                .to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "commission")),
            text_position: None,
        });
    }

    // Check for protection period
    let has_protection = text_lower.contains("protection period")
        || text_lower.contains("tail period")
        || text_lower.contains("after expiration");

    if !has_protection {
        violations.push(Violation {
            statute: "Florida Listing Agreement Best Practices".to_string(),
            severity: Severity::Info,
            message: "Consider including a protection period clause for commission after listing expiration."
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check broker license information
pub fn check_broker_license(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let has_license = text_lower.contains("license")
        || text_lower.contains("licensed")
        || Regex::new(r"(?i)(BK|SL)\d{5,}")
            .map(|re| re.is_match(text))
            .unwrap_or(false);

    if !has_license {
        violations.push(Violation {
            statute: "F.S. Chapter 475".to_string(),
            severity: Severity::Warning,
            message: "Listing agreement should include broker's license number.".to_string(),
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

/// Get list of statutes/codes covered for Florida real estate
pub fn covered_realestate_statutes() -> Vec<&'static str> {
    vec![
        "F.S. § 404.056 - Radon Gas Disclosure",
        "F.S. § 689.261 - Property Tax Disclosure",
        "F.S. § 689.302 - Flood Disclosure (SB 948)",
        "F.S. § 720.401 - HOA Disclosure",
        "F.S. § 553.996 - Energy Efficiency Disclosure",
        "F.S. § 475.278 - Brokerage Relationship Disclosure",
        "F.S. § 475.25 - Definite Expiration Date",
        "42 U.S.C. § 4852d - Lead Paint Disclosure",
        "Johnson v. Davis (1985) - Material Defect Disclosure",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Document Type Detection
    // ========================================================================

    #[test]
    fn test_detect_purchase_contract() {
        let text = "PURCHASE AND SALE CONTRACT. Buyer agrees to purchase from Seller \
                    the real property located at 123 Main St.";
        assert_eq!(
            RealEstateDocumentType::detect(text),
            RealEstateDocumentType::PurchaseContract
        );
    }

    #[test]
    fn test_detect_escalation_addendum() {
        let text = "ESCALATION ADDENDUM. Buyer's base purchase price shall be escalated \
                    by $5,000 above competing offers, with maximum purchase price of $500,000.";
        assert_eq!(
            RealEstateDocumentType::detect(text),
            RealEstateDocumentType::EscalationAddendum
        );
    }

    #[test]
    fn test_detect_listing_agreement() {
        let text = "EXCLUSIVE LISTING AGREEMENT. Seller grants Broker the exclusive right \
                    to sell property. Commission rate: 6%.";
        assert_eq!(
            RealEstateDocumentType::detect(text),
            RealEstateDocumentType::ListingAgreement
        );
    }

    // ========================================================================
    // Radon Disclosure
    // ========================================================================

    #[test]
    fn test_radon_disclosure_missing() {
        let text = "Purchase contract for property at 123 Main St.";
        let violations = check_radon_disclosure(text);
        assert!(
            violations.iter().any(|v| v.statute.contains("404.056")),
            "Should detect missing radon disclosure"
        );
    }

    #[test]
    fn test_radon_disclosure_present() {
        let text = "RADON DISCLOSURE: Pursuant to § 404.056, radon gas is a naturally occurring \
                    radioactive gas that poses health risks including lung cancer. \
                    Testing for radon is recommended.";
        let violations = check_radon_disclosure(text);
        assert!(
            violations.is_empty(),
            "Should not flag when radon disclosure is present"
        );
    }

    // ========================================================================
    // Property Tax Disclosure
    // ========================================================================

    #[test]
    fn test_property_tax_disclosure_missing() {
        let text = "Purchase contract for property.";
        let violations = check_property_tax_disclosure(text);
        assert!(
            violations.iter().any(|v| v.statute.contains("689.261")),
            "Should detect missing property tax disclosure"
        );
    }

    #[test]
    fn test_property_tax_disclosure_present() {
        let text = "PROPERTY TAX DISCLOSURE: Per § 689.261, property taxes may increase \
                    substantially upon change of ownership due to reassessment.";
        let violations = check_property_tax_disclosure(text);
        assert!(
            violations.is_empty(),
            "Should not flag when property tax disclosure is present"
        );
    }

    // ========================================================================
    // Flood Disclosure
    // ========================================================================

    #[test]
    fn test_flood_disclosure_missing() {
        let text = "Purchase contract for property at coastal location.";
        let violations = check_flood_disclosure_realestate(text);
        assert!(
            violations.iter().any(|v| v.statute.contains("689.302")),
            "Should detect missing flood disclosure"
        );
    }

    #[test]
    fn test_flood_disclosure_present() {
        let text = "FLOOD DISCLOSURE: Per § 689.302, Seller discloses: \
                    (1) No knowledge of prior flooding. \
                    (2) No flood insurance claims filed. \
                    (3) No FEMA or federal flood assistance received.";
        let violations = check_flood_disclosure_realestate(text);
        assert!(
            violations.is_empty(),
            "Should not flag when complete flood disclosure is present"
        );
    }

    // ========================================================================
    // Brokerage Relationship
    // ========================================================================

    #[test]
    fn test_brokerage_relationship_missing() {
        let text = "Listing agreement between seller and broker for property sale.";
        let violations = check_brokerage_relationship(text);
        assert!(
            violations.iter().any(|v| v.statute.contains("475.278")),
            "Should detect missing brokerage relationship disclosure"
        );
    }

    #[test]
    fn test_brokerage_relationship_single_agent() {
        let text = "BROKERAGE RELATIONSHIP DISCLOSURE: Per § 475.278, Broker acts as \
                    SINGLE AGENT for Seller with duties of loyalty, confidentiality, \
                    and full disclosure.";
        let violations = check_brokerage_relationship(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(
            critical.is_empty(),
            "Should not have critical violations for proper single agent disclosure"
        );
    }

    // ========================================================================
    // Listing Expiration
    // ========================================================================

    #[test]
    fn test_listing_expiration_missing() {
        let text = "Listing agreement for property sale.";
        let violations = check_listing_expiration(text);
        assert!(
            violations.iter().any(|v| v.statute.contains("475.25")),
            "Should detect missing expiration date"
        );
    }

    #[test]
    fn test_listing_expiration_present() {
        let text = "This listing agreement expires on June 30, 2026.";
        let violations = check_listing_expiration(text);
        assert!(
            violations.is_empty(),
            "Should not flag when expiration date is present"
        );
    }

    // ========================================================================
    // Escalation Addendum
    // ========================================================================

    #[test]
    fn test_escalation_missing_maximum() {
        let text = "Buyer will escalate offer by $5,000 above competing offers.";
        let violations = check_escalation_addendum(text);
        assert!(
            violations
                .iter()
                .any(|v| v.message.contains("maximum") && v.severity == Severity::Critical),
            "Should detect missing maximum price cap"
        );
    }

    #[test]
    fn test_escalation_complete() {
        let text = "ESCALATION CLAUSE: Base offer price is $450,000. \
                    Buyer will escalate by increment of $5,000 above any bona fide competing offer, \
                    not to exceed maximum of $500,000. \
                    Seller must provide proof/copy of competing offer.";
        let violations = check_escalation_addendum(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(
            critical.is_empty(),
            "Should not have critical violations for complete escalation clause"
        );
    }

    // ========================================================================
    // Lead Paint
    // ========================================================================

    #[test]
    fn test_lead_paint_pre_1978() {
        let text = "Property built in 1965.";
        let violations = check_lead_paint_disclosure(text, Some(1965));
        assert!(
            violations.iter().any(|v| v.statute.contains("4852d")),
            "Should require lead paint disclosure for pre-1978 property"
        );
    }

    #[test]
    fn test_lead_paint_post_1978() {
        let text = "Property built in 2005.";
        let violations = check_lead_paint_disclosure(text, Some(2005));
        assert!(
            violations.is_empty(),
            "Should not require lead paint disclosure for post-1978 property"
        );
    }

    // ========================================================================
    // Complete Document Checks
    // ========================================================================

    #[test]
    fn test_compliant_purchase_contract() {
        let text = r#"
            FLORIDA RESIDENTIAL REAL ESTATE PURCHASE CONTRACT

            RADON DISCLOSURE: Pursuant to F.S. § 404.056, radon gas is a naturally
            occurring radioactive gas that poses health risks including lung cancer.
            Testing for radon is recommended before purchase.

            PROPERTY TAX DISCLOSURE: Per § 689.261, property taxes may substantially
            increase upon change of ownership due to reassessment. Homestead exemption
            may apply.

            FLOOD DISCLOSURE: Per § 689.302, Seller discloses:
            (1) No knowledge of prior flooding at this property.
            (2) No flood insurance claims have been filed.
            (3) No FEMA or federal flood assistance received.

            HOA DISCLOSURE: Property is subject to Palm Beach HOA. Assessment: $350/month.
            Contact: Palm Beach Property Management. Buyer has 3 days to cancel.

            ENERGY DISCLOSURE: Per § 553.996, energy efficiency rating information
            is available upon request.

            SELLER'S DISCLOSURE: Seller discloses all known defects affecting the
            property value.

            EARNEST MONEY: $15,000 to be held by ABC Title Company (escrow agent).

            CLOSING DATE: February 15, 2026. Title insurance to be provided.
        "#;

        let violations = check_purchase_contract(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(
            critical.is_empty(),
            "Compliant contract should have no critical violations. Got: {:?}",
            critical
        );
    }

    // ========================================================================
    // New Template Integration Tests
    // ========================================================================

    #[test]
    fn test_florida_purchase_contract_template_compliance() {
        // Test that a fully-populated Florida purchase contract template passes compliance
        let text = r#"
            FLORIDA RESIDENTIAL REAL ESTATE PURCHASE CONTRACT

            This Contract is entered into between John Seller ("Seller") and Jane Buyer ("Buyer")
            for the purchase of the property located at 123 Palm Beach Road, Miami, FL 33101.

            RADON GAS DISCLOSURE (F.S. § 404.056)
            Radon is a naturally occurring radioactive gas that, when accumulated in a building
            in sufficient quantities, may present health risks. Levels of radon that exceed
            federal and state guidelines have been found in buildings in Florida. Additional
            information regarding radon and radon testing may be obtained from your county
            health department. Testing for radon is recommended prior to purchase.

            PROPERTY TAX DISCLOSURE (F.S. § 689.261)
            The ad valorem property taxes for this property may increase substantially upon
            change of ownership and reassessment by the county property appraiser.
            Buyer should consult with the county tax office regarding potential tax liability.
            Information about homestead exemption available from property appraiser.

            FLOOD DISCLOSURE (F.S. § 689.302 / SB 948)
            Seller has knowledge of the following regarding flooding:
            - Prior flooding: Seller has no knowledge of prior flooding at the property.
            - Flood insurance claims: No flood insurance claims have been filed for this property.
            - Federal flood assistance: No FEMA assistance has been received for this property.

            HOA DISCLOSURE (F.S. § 720.401)
            Property is not subject to HOA.

            ENERGY EFFICIENCY DISCLOSURE (F.S. § 553.996)
            Per Florida energy code, energy efficiency rating information is available
            upon request from the builder or current owner.

            SELLER'S DISCLOSURE OF KNOWN DEFECTS (Johnson v. Davis)
            Seller discloses all known material defects that affect the property value
            and are not readily observable to the Buyer.

            EARNEST MONEY: $25,000.00 due within 3 business days to XYZ Title Company (escrow agent).

            CLOSING DATE: March 15, 2026 at Miami Title Insurance Company.
            Title insurance to be provided by Seller.
        "#;

        let violations = check_purchase_contract(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();

        assert!(
            critical.is_empty(),
            "Florida purchase contract template should pass compliance. Critical violations: {:?}",
            critical
        );
    }

    #[test]
    fn test_florida_escalation_addendum_template_compliance() {
        // Test that a fully-populated Florida escalation addendum passes compliance
        let text = r#"
            ESCALATION ADDENDUM TO PURCHASE AND SALE CONTRACT

            This Addendum is attached to the Contract dated January 15, 2026 between
            John Seller ("Seller") and Jane Buyer ("Buyer") for the property at
            456 Ocean Drive, Tampa, FL 33607.

            ESCALATION CLAUSE:
            Buyer's base purchase price offer is $450,000.00.

            Buyer agrees to escalate the purchase price by an increment of $5,000.00
            above any bona fide competing offer received by Seller.

            Maximum purchase price cap: Buyer's escalated purchase price shall not exceed
            $500,000.00 under any circumstances.

            PROOF OF COMPETING OFFER:
            Seller must provide Buyer with a copy of the competing offer or written
            evidence of the bona fide offer within 24 hours of escalation activation.

            APPRAISAL GAP COVERAGE:
            In the event the appraised value is less than the escalated purchase price,
            Buyer agrees to cover the difference up to $15,000.00 (appraisal gap).

            EARNEST MONEY INCREASE:
            Upon escalation, Buyer will deposit an additional 1% of the escalated price
            within 3 business days.
        "#;

        let violations = check_escalation_addendum(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();

        assert!(
            critical.is_empty(),
            "Florida escalation addendum template should pass compliance. Critical violations: {:?}",
            critical
        );
    }

    #[test]
    fn test_florida_listing_agreement_template_compliance() {
        // Test that a fully-populated Florida listing agreement passes compliance
        let text = r#"
            EXCLUSIVE LISTING AGREEMENT

            This Listing Agreement is entered into on January 1, 2026 between
            John Seller ("Seller") and ABC Realty ("Broker").

            BROKERAGE RELATIONSHIP DISCLOSURE (F.S. § 475.278)
            Broker will act as SINGLE AGENT for Seller with the following duties:
            - Loyalty to Seller
            - Confidentiality of all information
            - Obedience to lawful instructions
            - Full disclosure of all material facts
            - Accounting for all funds
            - Skill, care, and diligence in the transaction

            PROPERTY: 789 Bayshore Blvd, St. Petersburg, FL 33701

            LISTING TERMS (F.S. Chapter 475 Compliance):
            Listing Price: $599,000.00
            Listing Start Date: January 1, 2026
            Listing Expiration Date: June 30, 2026

            COMMISSION STRUCTURE:
            Listing broker fee: 3% of final sale price
            Commission is negotiable and not fixed by law.

            BROKER LICENSE INFORMATION:
            Broker: ABC Realty, License #BK123456
            Agent: Mary Agent, License #SL789012

            PROTECTION PERIOD:
            If property is sold within 90 days after expiration to a buyer who
            was introduced during the listing period, commission shall still be due.
        "#;

        let violations = check_listing_agreement(text);
        let critical: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();

        assert!(
            critical.is_empty(),
            "Florida listing agreement template should pass compliance. Critical violations: {:?}",
            critical
        );
    }
}

// ============================================================================
// PROPERTY TESTS - Fuzz testing for compliance rules
// ============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use shared_types::Severity;

    // ========================================================================
    // Document Type Detection Property Tests
    // ========================================================================

    proptest! {
        /// Property: Document type detection should never panic on arbitrary input
        #[test]
        fn document_type_detection_no_panic(text in "\\PC*") {
            // Just ensure it doesn't panic
            let _ = RealEstateDocumentType::detect(&text);
        }

        /// Property: Escalation addendum detection requires key terms together
        #[test]
        fn escalation_detection_requires_keywords(
            prefix in "\\PC{0,100}",
            suffix in "\\PC{0,100}"
        ) {
            // Without all required keywords, should not detect as escalation addendum
            let text_without_keywords = format!("{} Some random contract text {}", prefix, suffix);
            let doc_type = RealEstateDocumentType::detect(&text_without_keywords);

            // If text doesn't contain all escalation keywords, shouldn't be detected as escalation
            let lower = text_without_keywords.to_lowercase();
            let has_all = lower.contains("escalation")
                && (lower.contains("addendum") || lower.contains("clause"))
                && lower.contains("maximum")
                && lower.contains("purchase price");

            if !has_all {
                prop_assert_ne!(doc_type, RealEstateDocumentType::EscalationAddendum);
            }
        }

        /// Property: Adding escalation keywords should detect as escalation addendum
        #[test]
        fn escalation_detection_with_keywords(
            prefix in "\\PC{0,50}",
            suffix in "\\PC{0,50}"
        ) {
            let text = format!(
                "{} ESCALATION ADDENDUM. The maximum purchase price shall be $500,000. {}",
                prefix, suffix
            );
            let doc_type = RealEstateDocumentType::detect(&text);
            prop_assert_eq!(doc_type, RealEstateDocumentType::EscalationAddendum);
        }
    }

    // ========================================================================
    // Radon Disclosure Property Tests
    // ========================================================================

    proptest! {
        /// Property: Any text without radon disclosure should trigger a violation
        #[test]
        fn radon_disclosure_required_when_missing(text in "[a-zA-Z0-9 ]{10,100}") {
            // Skip if text accidentally contains radon-related keywords
            let lower = text.to_lowercase();
            prop_assume!(!lower.contains("radon"));
            prop_assume!(!lower.contains("404.056"));

            let violations = check_radon_disclosure(&text);
            prop_assert!(
                violations.iter().any(|v| v.statute.contains("404.056")),
                "Missing radon disclosure should be flagged"
            );
        }

        /// Property: Proper radon disclosure should not trigger critical violations
        #[test]
        fn proper_radon_disclosure_passes(
            property_address in "[0-9]+ [A-Za-z]+ Street, [A-Za-z]+, FL [0-9]{5}"
        ) {
            let text = format!(
                "RADON DISCLOSURE for property at {}. \
                 Pursuant to F.S. § 404.056, radon gas is a naturally occurring \
                 radioactive gas that poses health risks including lung cancer. \
                 Testing for radon is recommended.",
                property_address
            );
            let violations = check_radon_disclosure(&text);
            prop_assert!(violations.is_empty(), "Proper radon disclosure should pass");
        }
    }

    // ========================================================================
    // Flood Disclosure Property Tests
    // ========================================================================

    proptest! {
        /// Property: Complete flood disclosure should pass
        #[test]
        fn complete_flood_disclosure_passes(
            flooding_choice in prop::bool::ANY,
            claims_choice in prop::bool::ANY,
            fema_choice in prop::bool::ANY
        ) {
            let flooding = if flooding_choice {
                "Seller has knowledge of prior flooding"
            } else {
                "Seller has no knowledge of flooding"
            };
            let claims = if claims_choice {
                "Flood insurance claims have been filed"
            } else {
                "No flood insurance claims have been filed"
            };
            let fema = if fema_choice {
                "FEMA assistance received"
            } else {
                "No FEMA or federal flood assistance received"
            };

            let text = format!(
                "FLOOD DISCLOSURE: Per § 689.302, Seller discloses: {}. {}. {}.",
                flooding, claims, fema
            );

            let violations = check_flood_disclosure_realestate(&text);
            prop_assert!(
                violations.is_empty(),
                "Complete flood disclosure should pass. Got: {:?}",
                violations
            );
        }
    }

    // ========================================================================
    // Escalation Addendum Property Tests
    // ========================================================================

    proptest! {
        /// Property: Escalation without maximum cap should trigger critical violation
        #[test]
        fn escalation_without_cap_fails(
            base_price in 100000u32..2000000u32,
            increment in 1000u32..50000u32
        ) {
            let text = format!(
                "ESCALATION: Base offer price is ${}. \
                 Buyer will escalate by ${}.",
                base_price, increment
            );

            let violations = check_escalation_addendum(&text);
            prop_assert!(
                violations.iter().any(|v| v.message.contains("maximum") && v.severity == Severity::Critical),
                "Escalation without cap should fail. Got: {:?}",
                violations
            );
        }

        /// Property: Complete escalation clause should not have critical violations
        #[test]
        fn complete_escalation_passes(
            base_price in 100000u32..1000000u32,
            increment in 1000u32..25000u32,
            max_above_base in 10000u32..100000u32
        ) {
            let max_price = base_price + max_above_base;
            let text = format!(
                "ESCALATION CLAUSE: Base offer price is ${}. \
                 Buyer will escalate by increment of ${} above any bona fide competing offer. \
                 Maximum purchase price not to exceed ${}. \
                 Seller must provide proof/copy of competing offer.",
                base_price, increment, max_price
            );

            let violations = check_escalation_addendum(&text);
            let critical: Vec<_> = violations
                .iter()
                .filter(|v| v.severity == Severity::Critical)
                .collect();

            prop_assert!(
                critical.is_empty(),
                "Complete escalation should pass. Critical: {:?}",
                critical
            );
        }
    }

    // ========================================================================
    // Listing Agreement Property Tests
    // ========================================================================

    proptest! {
        /// Property: Listing without expiration date should fail
        #[test]
        fn listing_without_expiration_fails(
            broker_name in "[A-Z][a-z]+ Realty",
            commission in 1u32..10u32
        ) {
            let text = format!(
                "EXCLUSIVE LISTING AGREEMENT. \
                 Broker: {}. \
                 Commission: {}%.",
                broker_name, commission
            );

            let violations = check_listing_expiration(&text);
            prop_assert!(
                violations.iter().any(|v| v.statute.contains("475.25")),
                "Listing without expiration should fail. Got: {:?}",
                violations
            );
        }

        /// Property: Listing with valid expiration date should pass expiration check
        #[test]
        fn listing_with_expiration_passes(
            month in 1u32..12u32,
            day in 1u32..28u32,
            year in 2025u32..2030u32
        ) {
            let months = ["January", "February", "March", "April", "May", "June",
                         "July", "August", "September", "October", "November", "December"];
            let month_name = months[(month - 1) as usize];

            let text = format!(
                "This listing agreement expires on {} {}, {}.",
                month_name, day, year
            );

            let violations = check_listing_expiration(&text);
            prop_assert!(
                violations.is_empty(),
                "Listing with expiration should pass. Got: {:?}",
                violations
            );
        }

        /// Property: Listing without brokerage relationship disclosure should fail
        #[test]
        fn listing_without_brokerage_disclosure_fails(
            seller_name in "[A-Z][a-z]+ [A-Z][a-z]+",
            broker_name in "[A-Z][a-z]+ Realty"
        ) {
            let text = format!(
                "LISTING AGREEMENT between {} (Seller) and {} (Broker).",
                seller_name, broker_name
            );

            let violations = check_brokerage_relationship(&text);
            prop_assert!(
                violations.iter().any(|v| v.statute.contains("475.278")),
                "Missing brokerage disclosure should fail. Got: {:?}",
                violations
            );
        }
    }

    // ========================================================================
    // Lead Paint Property Tests
    // ========================================================================

    proptest! {
        /// Property: Post-1978 properties should not require lead paint disclosure
        #[test]
        fn post_1978_no_lead_disclosure(year in 1978u32..2025u32) {
            let text = format!("Property built in {}.", year);
            let violations = check_lead_paint_disclosure(&text, Some(year));
            prop_assert!(
                violations.is_empty(),
                "Post-1978 should not require lead disclosure. Got: {:?}",
                violations
            );
        }

        /// Property: Pre-1978 properties should require lead paint disclosure
        #[test]
        fn pre_1978_requires_lead_disclosure(year in 1900u32..1978u32) {
            let text = format!("Property built in {}. No other disclosures.", year);
            let violations = check_lead_paint_disclosure(&text, Some(year));
            prop_assert!(
                violations.iter().any(|v| v.statute.contains("4852d")),
                "Pre-1978 should require lead disclosure. Got: {:?}",
                violations
            );
        }
    }

    // ========================================================================
    // Complete Document Property Tests
    // ========================================================================

    proptest! {
        /// Property: Auto-detection should categorize based on key phrases
        #[test]
        fn auto_detection_consistent(text in "\\PC{50,200}") {
            let doc_type = RealEstateDocumentType::detect(&text);

            // Detection should be consistent
            let doc_type2 = RealEstateDocumentType::detect(&text);
            prop_assert_eq!(doc_type, doc_type2);
        }

        /// Property: Checking compliance should never panic on arbitrary input
        #[test]
        fn compliance_check_no_panic(text in "\\PC*") {
            // These should never panic
            let _ = check_radon_disclosure(&text);
            let _ = check_property_tax_disclosure(&text);
            let _ = check_flood_disclosure_realestate(&text);
            let _ = check_hoa_disclosure(&text);
            let _ = check_energy_efficiency_disclosure(&text);
            let _ = check_material_defect_disclosure(&text);
            let _ = check_earnest_money(&text);
            let _ = check_closing_provisions(&text);
            let _ = check_escalation_addendum(&text);
            let _ = check_listing_agreement(&text);
            let _ = check_brokerage_relationship(&text);
            let _ = check_listing_expiration(&text);
            let _ = check_commission_provisions(&text);
            let _ = check_broker_license(&text);
            let _ = check_lead_paint_disclosure(&text, None);
        }
    }
}
