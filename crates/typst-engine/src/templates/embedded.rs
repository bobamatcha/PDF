//! Embedded template loader
//!
//! This module loads Typst templates from external files at compile time,
//! embedding them directly in the binary.

/// Invoice template - loaded from templates/invoice.typ
const INVOICE_TEMPLATE: &str = include_str!("../../templates/invoice.typ");

/// Letter template - loaded from templates/letter.typ
const LETTER_TEMPLATE: &str = include_str!("../../templates/letter.typ");

/// Florida Lease template - loaded from templates/florida_lease.typ
const FLORIDA_LEASE_TEMPLATE: &str = include_str!("../../templates/florida_lease.typ");

/// Florida Purchase Contract template - loaded from templates/florida_purchase_contract.typ
const FLORIDA_PURCHASE_CONTRACT_TEMPLATE: &str =
    include_str!("../../templates/florida_purchase_contract.typ");

/// Florida Escalation Addendum template - loaded from templates/florida_escalation_addendum.typ
const FLORIDA_ESCALATION_ADDENDUM_TEMPLATE: &str =
    include_str!("../../templates/florida_escalation_addendum.typ");

/// Florida Listing Agreement template - loaded from templates/florida_listing_agreement.typ
const FLORIDA_LISTING_AGREEMENT_TEMPLATE: &str =
    include_str!("../../templates/florida_listing_agreement.typ");

/// Texas Lease template - loaded from templates/texas_lease.typ
const TEXAS_LEASE_TEMPLATE: &str = include_str!("../../templates/texas_lease.typ");

/// Florida Flood Disclosure template - loaded from templates/florida_flood_disclosure.typ
/// Standalone SB 948 / HB 1015 flood disclosure (§ 83.512) for quick generation
const FLORIDA_FLOOD_DISCLOSURE_TEMPLATE: &str =
    include_str!("../../templates/florida_flood_disclosure.typ");

/// Florida Purchase As-Is Contract template - loaded from templates/florida_purchase_as_is.typ
/// FAR/BAR "As-Is" Residential Contract with sole discretion inspection period
const FLORIDA_PURCHASE_AS_IS_TEMPLATE: &str =
    include_str!("../../templates/florida_purchase_as_is.typ");

/// Florida Commercial Lease template - loaded from templates/florida_commercial_lease.typ
/// Chapter 83 Part I (Non-Residential) - No habitability requirements
const FLORIDA_COMMERCIAL_LEASE_TEMPLATE: &str =
    include_str!("../../templates/florida_commercial_lease.typ");

/// Get an embedded template by name
pub fn get_embedded_template(name: &str) -> Option<String> {
    match name {
        "invoice" => Some(INVOICE_TEMPLATE.to_string()),
        "letter" => Some(LETTER_TEMPLATE.to_string()),
        "florida_lease" => Some(FLORIDA_LEASE_TEMPLATE.to_string()),
        "florida_purchase_contract" => Some(FLORIDA_PURCHASE_CONTRACT_TEMPLATE.to_string()),
        "florida_escalation_addendum" => Some(FLORIDA_ESCALATION_ADDENDUM_TEMPLATE.to_string()),
        "florida_listing_agreement" => Some(FLORIDA_LISTING_AGREEMENT_TEMPLATE.to_string()),
        "texas_lease" => Some(TEXAS_LEASE_TEMPLATE.to_string()),
        "florida_flood_disclosure" => Some(FLORIDA_FLOOD_DISCLOSURE_TEMPLATE.to_string()),
        "florida_purchase_as_is" => Some(FLORIDA_PURCHASE_AS_IS_TEMPLATE.to_string()),
        "florida_commercial_lease" => Some(FLORIDA_COMMERCIAL_LEASE_TEMPLATE.to_string()),
        _ => None,
    }
}

/// List all available embedded template names
pub fn list_embedded_templates() -> Vec<&'static str> {
    vec![
        "invoice",
        "letter",
        "florida_lease",
        "florida_purchase_contract",
        "florida_escalation_addendum",
        "florida_listing_agreement",
        "texas_lease",
        "florida_flood_disclosure",
        "florida_purchase_as_is",
        "florida_commercial_lease",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_invoice_template() {
        let template = get_embedded_template("invoice");
        assert!(template.is_some());
        assert!(template.unwrap().contains("INVOICE"));
    }

    #[test]
    fn test_get_letter_template() {
        let template = get_embedded_template("letter");
        assert!(template.is_some());
        let content = template.unwrap();
        assert!(content.contains("sender_name") || content.contains("Letter"));
    }

    #[test]
    fn test_get_florida_lease_template() {
        let template = get_embedded_template("florida_lease");
        assert!(template.is_some());

        let content = template.unwrap();
        // Verify mandatory Florida disclosures are present
        assert!(content.contains("RADON") || content.contains("Radon"));
        assert!(content.contains("404.056"));
    }

    // ========================================================================
    // HOA/CONDO ASSOCIATION ADDENDUM TESTS (P0 Gap)
    // Per FL_LEASE.md §3.1: "Association Supremacy Clause", indemnity,
    // approval contingency
    // ========================================================================

    #[test]
    fn test_florida_lease_has_hoa_condo_addendum() {
        let template = get_embedded_template("florida_lease");
        assert!(template.is_some());

        let content = template.unwrap();
        // Must have HOA/Condo Association Addendum section
        assert!(
            content.contains("HOA")
                || content.contains("Homeowners Association")
                || content.contains("Condominium Association"),
            "Florida lease should have HOA/Condo Association Addendum"
        );
    }

    #[test]
    fn test_florida_lease_hoa_supremacy_clause() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Association Supremacy Clause - lease subordinate to HOA rules
        assert!(
            content.contains("subordinate")
                || content.contains("Supremacy")
                || content.contains("governing documents"),
            "Florida lease HOA addendum should include Association Supremacy Clause"
        );
    }

    #[test]
    fn test_florida_lease_hoa_eviction_indemnity() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Indemnity for Association eviction costs
        assert!(
            content.contains("indemnif")
                || content.contains("hold harmless")
                || (content.contains("association") && content.contains("eviction")),
            "Florida lease HOA addendum should include eviction cost indemnity"
        );
    }

    // ========================================================================
    // CDD DISCLOSURE TESTS (§ 190.048) (P0 Gap)
    // Per FL_LEASE.md: Boldfaced text required, assessment amounts
    // ========================================================================

    #[test]
    fn test_florida_lease_has_cdd_disclosure() {
        let template = get_embedded_template("florida_lease");
        assert!(template.is_some());

        let content = template.unwrap();
        // Must reference CDD statute § 190.048
        assert!(
            content.contains("190.048") || content.contains("Community Development District"),
            "Florida lease should have CDD Disclosure per § 190.048"
        );
    }

    #[test]
    fn test_florida_lease_cdd_assessment_disclosure() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // CDD disclosure must mention assessments
        assert!(
            content.contains("cdd_assessment")
                || content.contains("CDD assessment")
                || (content.contains("Community Development") && content.contains("assessment")),
            "Florida lease CDD disclosure should include assessment amounts"
        );
    }

    // ========================================================================
    // LIQUIDATED DAMAGES ADDENDUM TESTS (§ 83.595) (P1 Gap)
    // Per FL_LEASE.md §6.2: Separate addendum, max 2 months rent, bold language
    // ========================================================================

    #[test]
    fn test_florida_lease_has_liquidated_damages_addendum() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must have Liquidated Damages / Early Termination addendum
        assert!(
            content.contains("Liquidated Damages")
                || content.contains("LIQUIDATED DAMAGES")
                || content.contains("Early Termination"),
            "Florida lease should have Liquidated Damages Addendum (§ 83.595)"
        );
    }

    #[test]
    fn test_florida_lease_liquidated_damages_statute_reference() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must reference § 83.595
        assert!(
            content.contains("83.595"),
            "Liquidated Damages addendum should reference § 83.595"
        );
    }

    #[test]
    fn test_florida_lease_liquidated_damages_two_months_cap() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must mention 2 months rent cap
        assert!(
            content.contains("two months")
                || content.contains("2 months")
                || content.contains("two (2) months"),
            "Liquidated Damages should reference 2 months rent cap"
        );
    }

    // ========================================================================
    // 30-DAY NOTICE EXPLICIT REFERENCE TESTS (HB 1417 / § 83.57) (P1 Gap)
    // Per FL_LEASE.md: Should cite statute explicitly, not just use variable
    // ========================================================================

    #[test]
    fn test_florida_lease_30_day_notice_statute_reference() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must reference § 83.57 for month-to-month termination
        assert!(
            content.contains("83.57"),
            "Florida lease should explicitly reference § 83.57 for 30-day notice"
        );
    }

    #[test]
    fn test_florida_lease_30_day_notice_hb1417_reference() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Should reference HB 1417 or the 30-day requirement
        assert!(
            content.contains("HB 1417")
                || content.contains("1417")
                || (content.contains("30")
                    && content.contains("day")
                    && content.contains("notice")),
            "Florida lease should reference 30-day notice requirement (HB 1417)"
        );
    }

    // ========================================================================
    // JURY TRIAL WAIVER TESTS (P2 Gap)
    // Per FL_LEASE.md §6.3: Bold, all-caps clause
    // ========================================================================

    #[test]
    fn test_florida_lease_has_jury_trial_waiver() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must have jury trial waiver
        assert!(
            content.contains("JURY TRIAL") || content.contains("jury trial"),
            "Florida lease should have Jury Trial Waiver clause"
        );
    }

    #[test]
    fn test_florida_lease_jury_waiver_is_knowing() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must indicate knowing and voluntary waiver
        assert!(
            content.contains("KNOWINGLY") || content.contains("VOLUNTARILY"),
            "Jury trial waiver should indicate knowing and voluntary waiver"
        );
    }

    // ========================================================================
    // MOLD PREVENTION ADDENDUM TESTS (P2 Gap)
    // Per FL_LEASE.md §6.4: AC, humidity, leak reporting
    // ========================================================================

    #[test]
    fn test_florida_lease_has_mold_addendum() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must have mold prevention section
        assert!(
            content.contains("Mold") || content.contains("MOLD") || content.contains("mold"),
            "Florida lease should have Mold Prevention Addendum"
        );
    }

    #[test]
    fn test_florida_lease_mold_humidity_requirement() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must mention humidity control
        assert!(
            content.contains("humidity") || content.contains("60%") || content.contains("60 %"),
            "Mold Prevention should mention humidity control (60%)"
        );
    }

    // ========================================================================
    // HB 621 SQUATTER LANGUAGE TESTS (P2 Gap)
    // Per FL_LEASE.md §6.1: Unauthorized occupants = transient/trespasser
    // ========================================================================

    #[test]
    fn test_florida_lease_has_squatter_language() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must reference HB 621 or unauthorized occupants
        assert!(
            content.contains("HB 621")
                || content.contains("621")
                || content.contains("unauthorized occupant")
                || content.contains("Unauthorized Occupant"),
            "Florida lease should have HB 621 squatter language"
        );
    }

    #[test]
    fn test_florida_lease_squatter_transient_language() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must declare unauthorized as transient/trespasser
        assert!(
            content.contains("transient") || content.contains("trespasser"),
            "Squatter language should declare unauthorized occupants as transient/trespasser"
        );
    }

    // ========================================================================
    // SERVICE MEMBER RIGHTS TESTS (§ 83.682) (P2 Gap)
    // Per FL_LEASE.md: 35-mile radius termination right
    // ========================================================================

    #[test]
    fn test_florida_lease_has_service_member_rights() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must reference § 83.682 or service member rights
        assert!(
            content.contains("83.682")
                || content.contains("Service Member")
                || content.contains("military"),
            "Florida lease should have Service Member Rights disclosure (§ 83.682)"
        );
    }

    #[test]
    fn test_florida_lease_service_member_35_mile() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must mention 35-mile termination right
        assert!(
            content.contains("35") || content.contains("thirty-five"),
            "Service Member Rights should mention 35-mile radius termination"
        );
    }

    // ========================================================================
    // ESA FRAUD PREVENTION TESTS (SB 1084 / § 817.265) (P3 Gap)
    // Per FL_LEASE.md §6.5: Cite statute, require "personal knowledge"
    // ========================================================================

    #[test]
    fn test_florida_lease_has_esa_fraud_prevention() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must reference ESA or emotional support animal
        assert!(
            content.contains("ESA")
                || content.contains("Emotional Support Animal")
                || content.contains("emotional support animal")
                || content.contains("assistance animal"),
            "Florida lease should have ESA Fraud Prevention clause (SB 1084)"
        );
    }

    #[test]
    fn test_florida_lease_esa_statute_reference() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must reference § 817.265 (criminal misdemeanor for false documentation)
        assert!(
            content.contains("817.265") || content.contains("SB 1084"),
            "ESA clause should reference § 817.265 or SB 1084"
        );
    }

    #[test]
    fn test_florida_lease_esa_personal_knowledge() {
        let template = get_embedded_template("florida_lease");
        let content = template.unwrap();

        // Must require "personal knowledge" from healthcare provider
        assert!(
            content.contains("personal knowledge") || content.contains("Personal Knowledge"),
            "ESA clause should require 'personal knowledge' documentation"
        );
    }

    #[test]
    fn test_get_florida_purchase_contract_template() {
        let template = get_embedded_template("florida_purchase_contract");
        assert!(template.is_some());

        let content = template.unwrap();
        // Verify mandatory Florida disclosures are present
        assert!(content.contains("RADON") || content.contains("Radon"));
        assert!(content.contains("404.056")); // Radon disclosure
        assert!(content.contains("689.261")); // Property tax disclosure
        assert!(content.contains("689.302")); // Flood disclosure
        assert!(content.contains("553.996")); // Energy efficiency
        assert!(content.contains("720.401")); // HOA disclosure
        assert!(content.contains("Johnson v. Davis")); // Material defect disclosure
    }

    #[test]
    fn test_get_florida_escalation_addendum_template() {
        let template = get_embedded_template("florida_escalation_addendum");
        assert!(template.is_some());

        let content = template.unwrap();
        // Verify escalation addendum key elements
        assert!(content.contains("ESCALATION"));
        assert!(
            content.contains("maximum_purchase_price")
                || content.contains("Maximum Purchase Price")
        );
        assert!(content.contains("Bona Fide"));
    }

    #[test]
    fn test_get_florida_listing_agreement_template() {
        let template = get_embedded_template("florida_listing_agreement");
        assert!(template.is_some());

        let content = template.unwrap();
        // Verify Chapter 475 brokerage relationship disclosures
        assert!(content.contains("475.278")); // Brokerage relationship disclosure
        assert!(content.contains("SINGLE AGENT") || content.contains("single_agent"));
        assert!(content.contains("TRANSACTION BROKER") || content.contains("transaction_broker"));
        assert!(content.contains("475.25")); // Definite expiration date requirement
    }

    // ========================================================================
    // FLORIDA LISTING AGREEMENT - NAR SETTLEMENT COMPLIANCE TESTS
    // Per FL_LIST.md Part II: Mandatory fee negotiability disclosure
    // ========================================================================

    #[test]
    fn test_florida_listing_has_fee_negotiability_disclosure() {
        let template = get_embedded_template("florida_listing_agreement");
        let content = template.unwrap();

        // NAR Settlement requires conspicuous statement that fees are negotiable
        assert!(
            content.contains("NEGOTIABLE")
                || content.contains("negotiable")
                || content.contains("not set by law"),
            "Listing agreement must have fee negotiability disclosure per NAR Settlement"
        );
    }

    #[test]
    fn test_florida_listing_has_four_pillars() {
        let template = get_embedded_template("florida_listing_agreement");
        let content = template.unwrap();

        // Pillar 1: Definite expiration date
        assert!(
            content.contains("expiration") || content.contains("Expiration"),
            "Must have definite expiration date (Pillar 1)"
        );

        // Pillar 2: Legal description / Property ID
        assert!(
            content.contains("legal_description")
                || content.contains("parcel")
                || content.contains("Legal Description"),
            "Must have legal description field (Pillar 2)"
        );

        // Pillar 3: Price and terms
        assert!(
            content.contains("listing_price") || content.contains("Listing Price"),
            "Must have listing price (Pillar 3)"
        );

        // Pillar 4: Commission structure
        assert!(
            content.contains("commission") || content.contains("Commission"),
            "Must have commission structure (Pillar 4)"
        );
    }

    #[test]
    fn test_florida_listing_no_auto_renewal() {
        let template = get_embedded_template("florida_listing_agreement");
        let content = template.unwrap();

        // Must explicitly state no auto-renewal per § 475.25(1)(r)
        assert!(
            content.contains("NOT automatically renew")
                || content.contains("shall NOT auto")
                || content.contains("no automatic renewal"),
            "Listing agreement must state no auto-renewal per § 475.25(1)(r)"
        );
    }

    #[test]
    fn test_florida_listing_has_concession_section() {
        let template = get_embedded_template("florida_listing_agreement");
        let content = template.unwrap();

        // Post-NAR Settlement: Should have buyer concession authorization section
        assert!(
            content.contains("concession")
                || content.contains("Concession")
                || content.contains("buyer_broker")
                || content.contains("cooperating"),
            "Listing agreement should have buyer concession/cooperation section"
        );
    }

    #[test]
    fn test_unknown_template() {
        let template = get_embedded_template("nonexistent");
        assert!(template.is_none());
    }

    #[test]
    fn test_get_texas_lease_template() {
        let template = get_embedded_template("texas_lease");
        assert!(template.is_some());

        let content = template.unwrap();
        // Verify Texas Property Code references
        assert!(content.contains("92.103") || content.contains("Security Deposit"));
        assert!(content.contains("92.0081") || content.contains("Lockout"));
        assert!(content.contains("92.201") || content.contains("Landlord Disclosure"));
        assert!(content.contains("92.056") || content.contains("Repair"));
        // Verify parking addendum for towing (§ 92.0131)
        assert!(content.contains("Parking") || content.contains("92.0131"));
    }

    #[test]
    fn test_list_embedded_templates() {
        let templates = list_embedded_templates();
        assert_eq!(templates.len(), 10);
        assert!(templates.contains(&"invoice"));
        assert!(templates.contains(&"letter"));
        assert!(templates.contains(&"florida_lease"));
        assert!(templates.contains(&"florida_purchase_contract"));
        assert!(templates.contains(&"florida_escalation_addendum"));
        assert!(templates.contains(&"florida_listing_agreement"));
        assert!(templates.contains(&"texas_lease"));
        assert!(templates.contains(&"florida_flood_disclosure"));
        assert!(templates.contains(&"florida_purchase_as_is"));
        assert!(templates.contains(&"florida_commercial_lease"));
    }

    // ========================================================================
    // STANDALONE FLOOD DISCLOSURE (SB 948 / HB 1015 - § 83.512) (P2)
    // Quick-generate just the flood disclosure form for Oct 2025 compliance
    // ========================================================================

    #[test]
    fn test_get_florida_flood_disclosure_template() {
        let template = get_embedded_template("florida_flood_disclosure");
        assert!(
            template.is_some(),
            "Should have florida_flood_disclosure template"
        );

        let content = template.unwrap();
        // Must reference the statute
        assert!(
            content.contains("83.512") || content.contains("Flood Disclosure"),
            "Flood disclosure should reference § 83.512"
        );
    }

    #[test]
    fn test_florida_flood_disclosure_has_flood_history() {
        let template = get_embedded_template("florida_flood_disclosure");
        let content = template.unwrap();

        // Must ask about flood history
        assert!(
            content.contains("flood_history")
                || content.contains("flood damage")
                || content.contains("flooded"),
            "Flood disclosure should ask about flood history"
        );
    }

    #[test]
    fn test_florida_flood_disclosure_has_insurance_claims() {
        let template = get_embedded_template("florida_flood_disclosure");
        let content = template.unwrap();

        // Must ask about insurance claims
        assert!(
            content.contains("flood_claims")
                || content.contains("insurance claim")
                || content.contains("claim for flood"),
            "Flood disclosure should ask about insurance claims"
        );
    }

    #[test]
    fn test_florida_flood_disclosure_has_fema_assistance() {
        let template = get_embedded_template("florida_flood_disclosure");
        let content = template.unwrap();

        // Must ask about FEMA assistance
        assert!(
            content.contains("FEMA")
                || content.contains("federal assistance")
                || content.contains("flood_fema"),
            "Flood disclosure should ask about FEMA assistance"
        );
    }

    #[test]
    fn test_florida_flood_disclosure_has_renters_insurance_warning() {
        let template = get_embedded_template("florida_flood_disclosure");
        let content = template.unwrap();

        // Must warn about renter's insurance not covering floods
        assert!(
            content.contains("renter")
                || content.contains("does not cover flood")
                || content.contains("standard insurance"),
            "Flood disclosure should warn about renter's insurance limitations"
        );
    }

    // ========================================================================
    // FLORIDA PURCHASE AS-IS CONTRACT TESTS
    // Based on FAR/BAR "As-Is" Residential Contract
    // Key Feature: Sole discretion termination during inspection period
    // ========================================================================

    #[test]
    fn test_florida_purchase_as_is_template_exists() {
        let template = get_embedded_template("florida_purchase_as_is");
        assert!(
            template.is_some(),
            "florida_purchase_as_is template should exist"
        );
    }

    #[test]
    fn test_florida_purchase_as_is_has_inspection_period() {
        let template = get_embedded_template("florida_purchase_as_is");
        let content = template.unwrap();

        // Must have inspection period section (key As-Is feature)
        assert!(
            content.contains("INSPECTION PERIOD")
                || content.contains("Inspection Period")
                || content.contains("inspection_period"),
            "As-Is contract must have inspection period section"
        );
    }

    #[test]
    fn test_florida_purchase_as_is_sole_discretion_termination() {
        let template = get_embedded_template("florida_purchase_as_is");
        let content = template.unwrap();

        // Must have sole discretion termination right (the key As-Is feature)
        assert!(
            content.contains("sole discretion")
                || content.contains("SOLE DISCRETION")
                || content.contains("sole and absolute discretion")
                || content.contains("any reason or no reason"),
            "As-Is contract must grant buyer sole discretion termination right"
        );
    }

    #[test]
    fn test_florida_purchase_as_is_no_repair_obligation() {
        let template = get_embedded_template("florida_purchase_as_is");
        let content = template.unwrap();

        // Must clearly state no repair obligations
        assert!(
            content.contains("AS IS")
                || content.contains("as-is")
                || content.contains("no obligation to repair")
                || content.contains("no repairs"),
            "As-Is contract must clearly state property is sold as-is"
        );
    }

    #[test]
    fn test_florida_purchase_as_is_flood_disclosure() {
        let template = get_embedded_template("florida_purchase_as_is");
        let content = template.unwrap();

        // Must have flood disclosure per § 689.302
        assert!(
            content.contains("689.302") || content.contains("flood disclosure"),
            "As-Is contract must reference § 689.302 flood disclosure"
        );
    }

    #[test]
    fn test_florida_purchase_as_is_sb264_disclosure() {
        let template = get_embedded_template("florida_purchase_as_is");
        let content = template.unwrap();

        // Must have SB 264 foreign ownership disclosure
        assert!(
            content.contains("SB 264")
                || content.contains("foreign ownership")
                || content.contains("foreign principal")
                || content.contains("692.204"),
            "As-Is contract must have SB 264 foreign ownership disclosure"
        );
    }

    #[test]
    fn test_florida_purchase_as_is_radon_disclosure() {
        let template = get_embedded_template("florida_purchase_as_is");
        let content = template.unwrap();

        // Must have radon disclosure per § 404.056
        assert!(
            content.contains("404.056") || content.contains("RADON") || content.contains("radon"),
            "As-Is contract must have radon disclosure (§ 404.056)"
        );
    }

    #[test]
    fn test_florida_purchase_as_is_title_section() {
        let template = get_embedded_template("florida_purchase_as_is");
        let content = template.unwrap();

        // Must have title and survey section
        assert!(
            content.contains("TITLE")
                || content.contains("title insurance")
                || content.contains("marketable title"),
            "As-Is contract must have title section"
        );
    }

    #[test]
    fn test_florida_purchase_as_is_escrow_section() {
        let template = get_embedded_template("florida_purchase_as_is");
        let content = template.unwrap();

        // Must have escrow/earnest money section
        assert!(
            content.contains("ESCROW")
                || content.contains("escrow")
                || content.contains("earnest money")
                || content.contains("deposit"),
            "As-Is contract must have escrow/deposit section"
        );
    }

    #[test]
    fn test_florida_purchase_as_is_appraisal_gap_option() {
        let template = get_embedded_template("florida_purchase_as_is");
        let content = template.unwrap();

        // Should have optional appraisal gap clause
        assert!(
            content.contains("appraisal gap")
                || content.contains("appraisal_gap")
                || content.contains("Appraisal Gap"),
            "As-Is contract should have appraisal gap clause option"
        );
    }

    // ========================================================================
    // FLORIDA COMMERCIAL LEASE TESTS
    // Chapter 83 Part I - Non-Residential Tenancies
    // Key: No habitability requirements, more flexible terms
    // ========================================================================

    #[test]
    fn test_florida_commercial_lease_template_exists() {
        let template = get_embedded_template("florida_commercial_lease");
        assert!(
            template.is_some(),
            "florida_commercial_lease template should exist"
        );
    }

    #[test]
    fn test_florida_commercial_lease_part_i_reference() {
        let template = get_embedded_template("florida_commercial_lease");
        let content = template.unwrap();

        // Must reference Chapter 83 Part I (commercial), not Part II (residential)
        assert!(
            content.contains("Part I")
                || content.contains("non-residential")
                || content.contains("commercial")
                || content.contains("Chapter 83"),
            "Commercial lease must reference Chapter 83 Part I"
        );
    }

    #[test]
    fn test_florida_commercial_lease_sales_tax_clause() {
        let template = get_embedded_template("florida_commercial_lease");
        let content = template.unwrap();

        // Must address sales tax transition (repealed Oct 1, 2025)
        assert!(
            content.contains("sales tax")
                || content.contains("October 1, 2025")
                || content.contains("tax repeal"),
            "Commercial lease must address sales tax transition"
        );
    }

    #[test]
    fn test_florida_commercial_lease_no_habitability() {
        let template = get_embedded_template("florida_commercial_lease");
        let content = template.unwrap();

        // Should NOT have residential habitability language
        // OR should explicitly disclaim it
        let has_commercial_disclaimer = content.contains("no implied warranty")
            || content.contains("no habitability")
            || content.contains("AS-IS")
            || content.contains("not a dwelling");

        assert!(
            has_commercial_disclaimer,
            "Commercial lease should disclaim residential habitability warranties"
        );
    }

    #[test]
    fn test_florida_commercial_lease_triple_net_option() {
        let template = get_embedded_template("florida_commercial_lease");
        let content = template.unwrap();

        // Should have triple-net (NNN) options
        assert!(
            content.contains("triple net")
                || content.contains("Triple Net")
                || content.contains("NNN")
                || content.contains("net lease"),
            "Commercial lease should have triple-net lease option"
        );
    }

    #[test]
    fn test_florida_commercial_lease_cam_charges() {
        let template = get_embedded_template("florida_commercial_lease");
        let content = template.unwrap();

        // Should have CAM (Common Area Maintenance) section
        assert!(
            content.contains("CAM")
                || content.contains("Common Area")
                || content.contains("common area maintenance")
                || content.contains("operating expenses"),
            "Commercial lease should have CAM charges section"
        );
    }

    #[test]
    fn test_florida_commercial_lease_agricultural_lien_option() {
        let template = get_embedded_template("florida_commercial_lease");
        let content = template.unwrap();

        // Should have agricultural lien option per § 83.08
        assert!(
            content.contains("83.08")
                || content.contains("agricultural lien")
                || content.contains("crop lien")
                || content.contains("agricultural"),
            "Commercial lease should have agricultural lien option (§ 83.08)"
        );
    }

    #[test]
    fn test_florida_commercial_lease_flexible_termination() {
        let template = get_embedded_template("florida_commercial_lease");
        let content = template.unwrap();

        // Should have flexible termination provisions
        assert!(
            content.contains("termination notice")
                || content.contains("early termination")
                || content.contains("termination for convenience")
                || content.contains("Termination"),
            "Commercial lease should have termination provisions"
        );
    }

    #[test]
    fn test_florida_commercial_lease_use_clause() {
        let template = get_embedded_template("florida_commercial_lease");
        let content = template.unwrap();

        // Should have permitted use clause
        assert!(
            content.contains("PERMITTED USE")
                || content.contains("permitted use")
                || content.contains("use of premises")
                || content.contains("business use"),
            "Commercial lease should have permitted use clause"
        );
    }
}
