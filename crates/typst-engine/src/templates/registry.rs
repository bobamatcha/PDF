//! Template registry and metadata

use super::embedded;
use crate::compiler::errors::ServerError;
use serde::{Deserialize, Serialize};

/// Information about an available template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// Template name (used in URIs)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Full URI for this template
    pub uri: String,
    /// Required input fields
    pub required_inputs: Vec<String>,
    /// Optional input fields
    pub optional_inputs: Vec<String>,
}

/// List all available templates
pub fn list_templates() -> Vec<TemplateInfo> {
    vec![
        TemplateInfo {
            name: "invoice".to_string(),
            description: "Professional invoice template".to_string(),
            uri: "typst://templates/invoice".to_string(),
            required_inputs: vec![
                "company_name".to_string(),
                "client_name".to_string(),
                "items".to_string(),
            ],
            optional_inputs: vec![
                "company_address".to_string(),
                "client_address".to_string(),
                "invoice_number".to_string(),
                "date".to_string(),
                "due_date".to_string(),
                "notes".to_string(),
            ],
        },
        TemplateInfo {
            name: "letter".to_string(),
            description: "Formal business letter template".to_string(),
            uri: "typst://templates/letter".to_string(),
            required_inputs: vec![
                "sender_name".to_string(),
                "recipient_name".to_string(),
                "body".to_string(),
            ],
            optional_inputs: vec![
                "sender_address".to_string(),
                "recipient_address".to_string(),
                "date".to_string(),
                "subject".to_string(),
                "closing".to_string(),
            ],
        },
        TemplateInfo {
            name: "florida_lease".to_string(),
            description:
                "Florida residential lease with HB 615 Email Consent & SB 948 Flood Disclosure"
                    .to_string(),
            uri: "typst://templates/florida_lease".to_string(),
            required_inputs: vec![
                "landlord_name".to_string(),
                "tenant_name".to_string(),
                "property_address".to_string(),
                "monthly_rent".to_string(),
                "lease_start".to_string(),
                "lease_end".to_string(),
            ],
            optional_inputs: vec![
                "landlord_address".to_string(),
                "landlord_email".to_string(),
                "tenant_email".to_string(),
                "year_built".to_string(),
                "is_pre_1978".to_string(),
                "deposit_details".to_string(),
                // NOTE: email_consent (HB 615) belongs in SIGNATURE CEREMONY,
                // not template form. The TENANT consents during signing, not
                // the landlord filling out the template. See docsign-web.
                //
                // SB 948 - Flood Disclosure (§ 83.512)
                // Using neutral tristate fields per scrivener adherence:
                // - "yes" = Property has flooded / Claims filed / FEMA received
                // - "no" = No known flooding / No claims / No FEMA
                // - "unknown" = I don't know / Property recently acquired
                "flood_history_status".to_string(), // tristate: yes/no/unknown
                "flood_claims_status".to_string(),  // tristate: yes/no/unknown
                "flood_fema_status".to_string(),    // tristate: yes/no/unknown
                "flood_status_unknown".to_string(), // marker for "unknown" selection
                "flooding_description".to_string(),
                // HOA/Condo Association Addendum (FL_LEASE.md §3.1)
                "has_hoa".to_string(),           // boolean: enables HOA addendum
                "hoa_name".to_string(),          // Association name
                "hoa_management".to_string(),    // Management company name
                "hoa_contact".to_string(),       // Contact phone/email
                "hoa_approval_required".to_string(), // boolean: requires tenant approval
                "hoa_monthly_fee".to_string(),   // Monthly HOA/maintenance fees
                "hoa_rules_url".to_string(),     // URL to governing documents
                // CDD Disclosure (§ 190.048)
                "in_cdd".to_string(),            // boolean: property is in CDD
                "cdd_name".to_string(),          // CDD name
                "cdd_contact".to_string(),       // CDD contact info
                "cdd_assessment".to_string(),    // Annual CDD assessment amount
                // Liquidated Damages / Early Termination (§ 83.595)
                "early_termination_fee".to_string(), // boolean: enables addendum
                "early_termination_amount".to_string(), // fee amount (max 2 months rent)
                // Mold Prevention Addendum
                "mold_addendum".to_string(), // boolean: enables mold prevention addendum
            ],
        },
        // Florida Real Estate Purchase Contract
        TemplateInfo {
            name: "florida_purchase_contract".to_string(),
            description: "Florida residential real estate purchase contract with all mandatory disclosures (§ 404.056 Radon, § 689.261 Tax, § 689.302 Flood, § 720.401 HOA, § 553.996 Energy, Lead Paint)".to_string(),
            uri: "typst://templates/florida_purchase_contract".to_string(),
            required_inputs: vec![
                "seller_name".to_string(),
                "buyer_name".to_string(),
                "property_address".to_string(),
                "property_city".to_string(),
                "property_county".to_string(),
                "property_zip".to_string(),
                "purchase_price".to_string(),
                "earnest_money".to_string(),
                "closing_date".to_string(),
            ],
            optional_inputs: vec![
                // Party information
                "seller_address".to_string(),
                "seller_phone".to_string(),
                "seller_email".to_string(),
                "buyer_address".to_string(),
                "buyer_phone".to_string(),
                "buyer_email".to_string(),
                // Property details
                "parcel_id".to_string(),
                "legal_description".to_string(),
                "property_type".to_string(),
                "year_built".to_string(),
                // Financing
                "financing_type".to_string(), // cash, conventional, fha, va
                "loan_amount".to_string(),
                "max_interest_rate".to_string(),
                "loan_term".to_string(),
                "loan_application_deadline".to_string(),
                "loan_approval_deadline".to_string(),
                // Deposits
                "additional_deposit".to_string(),
                "earnest_money_due_date".to_string(),
                "escrow_agent_name".to_string(),
                "escrow_agent_address".to_string(),
                // Closing
                "closing_location".to_string(),
                "title_company".to_string(),
                "title_insurance_paid_by".to_string(),
                "doc_stamps_paid_by".to_string(),
                // Inspections
                "inspection_period_days".to_string(),
                "inspection_contingency_type".to_string(), // standard, as_is
                // Disclosures - Flood (§ 689.302)
                "has_prior_flooding".to_string(),
                "flooding_description".to_string(),
                "has_flood_claims".to_string(),
                "flood_claims_details".to_string(),
                "has_flood_assistance".to_string(),
                "flood_assistance_source".to_string(),
                "flood_assistance_details".to_string(),
                // HOA (§ 720.401)
                "has_hoa".to_string(),
                "hoa_name".to_string(),
                "hoa_assessment".to_string(),
                "hoa_assessment_frequency".to_string(),
                "hoa_contact".to_string(),
                // Lead Paint (pre-1978)
                "lead_paint_known".to_string(),
                "lead_paint_details".to_string(),
                "lead_reports_available".to_string(),
                "lead_inspection_waived".to_string(),
                // Seller's disclosure
                "known_defects".to_string(),
                "past_repairs".to_string(),
                "has_environmental_issues".to_string(),
                "environmental_details".to_string(),
                // Additional
                "additional_provisions".to_string(),
                "mediation_required".to_string(),
            ],
        },
        // Florida Escalation Addendum
        TemplateInfo {
            name: "florida_escalation_addendum".to_string(),
            description: "Escalation addendum for competitive purchase offers with maximum price cap and bona fide offer verification".to_string(),
            uri: "typst://templates/florida_escalation_addendum".to_string(),
            required_inputs: vec![
                "seller_name".to_string(),
                "buyer_name".to_string(),
                "property_address".to_string(),
                "contract_date".to_string(),
                "base_purchase_price".to_string(),
                "escalation_increment".to_string(),
                "maximum_purchase_price".to_string(),
            ],
            optional_inputs: vec![
                // Escalation terms
                "escalation_deadline".to_string(),
                "require_full_offer_copy".to_string(),
                "proof_deadline_hours".to_string(),
                // Appraisal gap
                "appraisal_gap_coverage".to_string(),
                "appraisal_gap_amount".to_string(),
                "appraisal_waiver".to_string(),
                // Financing
                "financing_type".to_string(),
                "updated_proof_days".to_string(),
                "additional_down_payment_available".to_string(),
                "additional_funds".to_string(),
                // Earnest money
                "increase_earnest_money".to_string(),
                "additional_earnest_percentage".to_string(),
                "additional_earnest_days".to_string(),
                // Additional parties
                "has_additional_parties".to_string(),
                "additional_buyer_name".to_string(),
                "additional_seller_name".to_string(),
                "additional_terms".to_string(),
            ],
        },
        // Florida Listing Agreement
        TemplateInfo {
            name: "florida_listing_agreement".to_string(),
            description: "Florida exclusive listing agreement with § 475.278 brokerage relationship disclosure (single agent or transaction broker)".to_string(),
            uri: "typst://templates/florida_listing_agreement".to_string(),
            required_inputs: vec![
                "seller_name".to_string(),
                "broker_name".to_string(),
                "broker_license".to_string(),
                "property_address".to_string(),
                "listing_price".to_string(),
                "listing_start_date".to_string(),
                "listing_expiration_date".to_string(),
                "commission_rate".to_string(),
            ],
            optional_inputs: vec![
                // Brokerage relationship (§ 475.278)
                "brokerage_relationship".to_string(), // single_agent or transaction_broker
                // Seller information
                "seller_address".to_string(),
                "seller_phone".to_string(),
                "seller_email".to_string(),
                "has_additional_seller".to_string(),
                "additional_seller_name".to_string(),
                // Broker/Agent information
                "brokerage_firm".to_string(),
                "broker_address".to_string(),
                "broker_phone".to_string(),
                "broker_email".to_string(),
                "agent_name".to_string(),
                "agent_license".to_string(),
                // Property details
                "property_city".to_string(),
                "property_county".to_string(),
                "property_zip".to_string(),
                "parcel_id".to_string(),
                "legal_description".to_string(),
                "property_type".to_string(),
                // Pricing
                "minimum_price".to_string(),
                "accept_cash".to_string(),
                "accept_conventional".to_string(),
                "accept_fha".to_string(),
                "accept_va".to_string(),
                // Items
                "included_items".to_string(),
                "excluded_items".to_string(),
                // Commission
                "commission_type".to_string(), // percentage or flat
                "flat_fee".to_string(),
                "coop_commission_rate".to_string(),
                "coop_flat_fee".to_string(),
                "protection_period_days".to_string(),
                // Marketing
                "list_on_mls".to_string(),
                "professional_photos".to_string(),
                "virtual_tour".to_string(),
                "open_houses".to_string(),
                // Access
                "lockbox_authorized".to_string(),
                "showing_instructions".to_string(),
                // Property status
                "property_occupied".to_string(),
                "occupant_type".to_string(),
                "has_hoa".to_string(),
                // Additional
                "mediation_required".to_string(),
                "additional_provisions".to_string(),
                "agreement_date".to_string(),
            ],
        },
        // Texas Residential Lease
        TemplateInfo {
            name: "texas_lease".to_string(),
            description: "Texas residential lease with Ch. 92 compliance (§ 92.0081 lockout, § 92.104 deposit return, § 92.3515 screening)".to_string(),
            uri: "typst://templates/texas_lease".to_string(),
            required_inputs: vec![
                "landlord_name".to_string(),
                "tenant_name".to_string(),
                "property_address".to_string(),
                "property_city".to_string(),
                "property_zip".to_string(),
                "monthly_rent".to_string(),
                "security_deposit".to_string(),
                "lease_start".to_string(),
                "lease_end".to_string(),
            ],
            optional_inputs: vec![
                // Landlord info
                "landlord_address".to_string(),
                "landlord_phone".to_string(),
                "landlord_email".to_string(),
                // Tenant info
                "tenant_phone".to_string(),
                "tenant_email".to_string(),
                "additional_tenant_name".to_string(),
                // Property details
                "property_county".to_string(),
                "property_type".to_string(),
                "bedrooms".to_string(),
                "bathrooms".to_string(),
                "year_built".to_string(),
                // Rent and fees
                "rent_due_day".to_string(),
                "payment_method".to_string(),
                "late_fee_percent".to_string(),
                "grace_period_days".to_string(),
                // Security deposit (§ 92.103-109)
                "deposit_holder".to_string(),
                "deposit_bank_name".to_string(),
                // Lockout policy (§ 92.0081)
                "lockout_policy_acknowledged".to_string(),
                // Landlord disclosure (§ 92.201)
                "owner_name".to_string(),
                "owner_address".to_string(),
                "manager_name".to_string(),
                "manager_address".to_string(),
                "emergency_phone".to_string(),
                // Repair procedures (§ 92.056)
                "repair_contact".to_string(),
                "repair_email".to_string(),
                "repair_address".to_string(),
                // Parking (§ 92.0131)
                "parking_included".to_string(),
                "parking_spaces".to_string(),
                "towing_company".to_string(),
                "towing_phone".to_string(),
                "towing_address".to_string(),
                // Lead paint (pre-1978)
                "lead_paint_known".to_string(),
                "lead_paint_details".to_string(),
                "lead_reports_available".to_string(),
                "lead_inspection_waived".to_string(),
                // Flood disclosure
                "flood_history_status".to_string(),
                "flooding_description".to_string(),
                "flood_claims_status".to_string(),
                "flood_claims_details".to_string(),
                "flood_zone".to_string(),
                "flood_insurance_required".to_string(),
                // Utilities
                "landlord_pays_electric".to_string(),
                "landlord_pays_gas".to_string(),
                "landlord_pays_water".to_string(),
                "landlord_pays_trash".to_string(),
                "landlord_pays_internet".to_string(),
                // Pets
                "pets_allowed".to_string(),
                "pet_types".to_string(),
                "pet_deposit".to_string(),
                "pet_rent".to_string(),
                // Other
                "smoking_allowed".to_string(),
            ],
        },
        // Florida Flood Disclosure (Standalone - SB 948 / HB 1015 - § 83.512)
        TemplateInfo {
            name: "florida_flood_disclosure".to_string(),
            description:
                "Standalone Florida flood disclosure form (§ 83.512) - quick-generate for Oct 2025 compliance"
                    .to_string(),
            uri: "typst://templates/florida_flood_disclosure".to_string(),
            required_inputs: vec![
                "property_address".to_string(),
                "landlord_name".to_string(),
            ],
            optional_inputs: vec![
                // Property details
                "property_city".to_string(),
                "property_zip".to_string(),
                // Landlord info
                "landlord_address".to_string(),
                // Tenant info
                "tenant_name".to_string(),
                "additional_tenant_name".to_string(),
                // Flood disclosure - tristate fields (yes/no/unknown)
                "flood_history_status".to_string(),
                "flooding_description".to_string(),
                "flood_claims_status".to_string(),
                "flood_claims_details".to_string(),
                "flood_fema_status".to_string(),
                "fema_details".to_string(),
            ],
        },
        // Florida Purchase As-Is Contract - FAR/BAR "As-Is" with sole discretion inspection
        TemplateInfo {
            name: "florida_purchase_as_is".to_string(),
            description:
                "Florida 'As-Is' Residential Purchase Contract - sole discretion inspection period"
                    .to_string(),
            uri: "typst://templates/florida_purchase_as_is".to_string(),
            required_inputs: vec![
                "property_address".to_string(),
                "buyer_name".to_string(),
                "seller_name".to_string(),
                "purchase_price".to_string(),
            ],
            optional_inputs: vec![
                // Property details
                "property_city".to_string(),
                "property_county".to_string(),
                "property_zip".to_string(),
                "parcel_id".to_string(),
                "legal_description".to_string(),
                "property_year_built".to_string(),
                // Parties
                "buyer_address".to_string(),
                "buyer_phone".to_string(),
                "buyer_email".to_string(),
                "seller_address".to_string(),
                "seller_phone".to_string(),
                "seller_email".to_string(),
                "additional_buyer_name".to_string(),
                "additional_seller_name".to_string(),
                // Deposits and escrow
                "initial_deposit".to_string(),
                "additional_deposit".to_string(),
                "escrow_agent".to_string(),
                "escrow_address".to_string(),
                "escrow_phone".to_string(),
                // Financing
                "financing_type".to_string(),
                "loan_amount".to_string(),
                "max_interest_rate".to_string(),
                "loan_term".to_string(),
                "financing_application_days".to_string(),
                "financing_approval_days".to_string(),
                "has_appraisal_contingency".to_string(),
                "has_appraisal_gap".to_string(),
                "appraisal_gap_amount".to_string(),
                // Inspection period (key As-Is feature)
                "inspection_period_days".to_string(),
                "inspection_end_date".to_string(),
                // Title
                "title_commitment_days".to_string(),
                "title_objection_days".to_string(),
                "title_cure_days".to_string(),
                "title_company".to_string(),
                "survey_required".to_string(),
                "survey_paid_by".to_string(),
                // Closing
                "closing_date".to_string(),
                "closing_location".to_string(),
                // Disclosures
                "current_taxes".to_string(),
                "has_homestead".to_string(),
                "flood_zone".to_string(),
                "fema_flood_zone".to_string(),
                "flood_history".to_string(),
                "flood_history_details".to_string(),
                "lead_paint_known".to_string(),
                "has_hoa".to_string(),
                "hoa_name".to_string(),
                "hoa_management".to_string(),
                "hoa_contact".to_string(),
                "hoa_monthly_fee".to_string(),
                "hoa_special_assessment".to_string(),
                // Additional terms
                "additional_terms".to_string(),
                "contract_date".to_string(),
            ],
        },
        // Florida Commercial Lease - Chapter 83 Part I (Non-Residential)
        TemplateInfo {
            name: "florida_commercial_lease".to_string(),
            description:
                "Florida Commercial Lease - Chapter 83 Part I (no habitability requirements)"
                    .to_string(),
            uri: "typst://templates/florida_commercial_lease".to_string(),
            required_inputs: vec![
                "property_address".to_string(),
                "landlord_name".to_string(),
                "tenant_name".to_string(),
                "monthly_rent".to_string(),
            ],
            optional_inputs: vec![
                // Property details
                "property_city".to_string(),
                "property_zip".to_string(),
                "suite_number".to_string(),
                "square_feet".to_string(),
                // Parties
                "landlord_address".to_string(),
                "landlord_phone".to_string(),
                "landlord_email".to_string(),
                "tenant_address".to_string(),
                "tenant_phone".to_string(),
                "tenant_email".to_string(),
                "tenant_entity_type".to_string(),
                "tenant_state".to_string(),
                // Lease terms
                "lease_start".to_string(),
                "lease_end".to_string(),
                "lease_term_months".to_string(),
                "permitted_use".to_string(),
                // Rent
                "annual_rent".to_string(),
                "rent_per_sf".to_string(),
                "rent_payee".to_string(),
                "rent_address".to_string(),
                "has_rent_escalation".to_string(),
                "escalation_type".to_string(),
                "escalation_percent".to_string(),
                "late_fee_grace_period".to_string(),
                "late_fee_percent".to_string(),
                // Lease type and CAM
                "lease_type".to_string(),
                "pro_rata_share".to_string(),
                "building_total_sf".to_string(),
                "management_fee_cap".to_string(),
                "estimated_monthly_cam".to_string(),
                // Security
                "security_deposit".to_string(),
                "deposit_return_days".to_string(),
                // Utilities
                "tenant_pays_electric".to_string(),
                "tenant_pays_gas".to_string(),
                "tenant_pays_water".to_string(),
                "tenant_pays_trash".to_string(),
                "tenant_pays_internet".to_string(),
                "tenant_pays_janitorial".to_string(),
                // Maintenance
                "tenant_pays_roof".to_string(),
                // Insurance
                "liability_coverage".to_string(),
                // Default
                "rent_default_days".to_string(),
                "other_default_days".to_string(),
                // Termination
                "termination_notice_days".to_string(),
                "holdover_notice_days".to_string(),
                "has_early_termination".to_string(),
                "early_termination_fee".to_string(),
                "early_termination_notice".to_string(),
                "early_termination_lockout".to_string(),
                // Renewal
                "has_renewal_option".to_string(),
                "renewal_terms".to_string(),
                "renewal_period".to_string(),
                "renewal_notice_days".to_string(),
                "renewal_rent_terms".to_string(),
                // Early possession
                "early_possession".to_string(),
                "early_possession_date".to_string(),
                "early_possession_purpose".to_string(),
                // Assignment
                "assignment_unrestricted".to_string(),
                // Signage
                "signage_allowed".to_string(),
                "signage_description".to_string(),
                // Agricultural (§ 83.08)
                "is_agricultural".to_string(),
                // Guaranty
                "requires_guaranty".to_string(),
                "guarantor_name".to_string(),
                "guarantor_address".to_string(),
                // Exclusivity
                "has_exclusivity".to_string(),
                "exclusivity_description".to_string(),
                "exclusivity_threshold".to_string(),
                "exclusivity_abatement".to_string(),
                // Additional terms
                "additional_terms".to_string(),
            ],
        },
        // CMA (Comparative Market Analysis) Report - Professional 5-Page Template
        TemplateInfo {
            name: "cma".to_string(),
            description: "Professional 5-page Comparative Market Analysis with adjustment grid, neighborhood analysis, and investment metrics".to_string(),
            uri: "typst://templates/cma".to_string(),
            required_inputs: vec![
                "subject_address".to_string(),
                "site_name".to_string(),
            ],
            optional_inputs: vec![
                // Site branding
                "site_color".to_string(),
                // Subject property details
                "subject_city".to_string(),
                "subject_zip".to_string(),
                "subject_beds".to_string(),
                "subject_baths".to_string(),
                "subject_sqft".to_string(),
                "subject_year_built".to_string(),
                // Valuation
                "estimated_value".to_string(),
                "estimated_low".to_string(),
                "estimated_high".to_string(),
                "confidence_score".to_string(),
                // Comparable sales (array with adjustments)
                "comps".to_string(),
                "comp_count".to_string(),
                // Adjustment configuration
                "adjustment_config".to_string(),
                // Investment metrics
                "rental_yield".to_string(),
                "rent_estimate".to_string(),
                "price_per_sqft".to_string(),
                // Neighborhood scores
                "school_score".to_string(),
                "school_elementary".to_string(),
                "school_middle".to_string(),
                "school_high".to_string(),
                "school_avg".to_string(),
                "crime_score".to_string(),
                "flood_zone".to_string(),
                // Market position
                "list_price".to_string(),
                "market_position".to_string(),
                "recommendation".to_string(),
                // Metadata
                "generated_at".to_string(),
            ],
        },
    ]
}

/// Get the source code for a template by name
pub fn get_template_source(name: &str) -> Result<String, ServerError> {
    embedded::get_embedded_template(name)
        .ok_or_else(|| ServerError::TemplateNotFound(name.to_string()))
}

/// Parse a template URI and return the template name
pub fn parse_template_uri(uri: &str) -> Option<&str> {
    uri.strip_prefix("typst://templates/")
}

/// Check if a URI refers to a template
pub fn is_template_uri(uri: &str) -> bool {
    uri.starts_with("typst://templates/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates() {
        let templates = list_templates();
        assert!(!templates.is_empty());

        // Check that florida_lease exists
        assert!(templates.iter().any(|t| t.name == "florida_lease"));
    }

    #[test]
    fn test_get_template_source() {
        let source = get_template_source("invoice");
        assert!(source.is_ok());

        let source = source.unwrap();
        assert!(source.contains("Invoice") || source.contains("invoice"));
    }

    #[test]
    fn test_parse_template_uri() {
        assert_eq!(
            parse_template_uri("typst://templates/invoice"),
            Some("invoice")
        );
        assert_eq!(
            parse_template_uri("typst://templates/florida_lease"),
            Some("florida_lease")
        );
        assert_eq!(parse_template_uri("invalid://uri"), None);
    }

    // ============================================================
    // SCRIVENER ADHERENCE TESTS - Strict neutrality requirements
    // ============================================================
    //
    // Per STRATEGY.md: Forms must be neutral and not lead users toward
    // any particular outcome. Optional addenda should be offered without
    // pushing the user in any direction.

    #[test]
    fn test_email_consent_not_in_template_form() {
        // HB 615 Email Consent should be in the SIGNATURE CEREMONY,
        // not the template form. The TENANT signs this consent, not
        // the landlord filling out the template.
        //
        // Per STRATEGY.md lines 188-214: "Hardcode into the signature ceremony"
        let templates = list_templates();
        let florida_lease = templates
            .iter()
            .find(|t| t.name == "florida_lease")
            .unwrap();

        assert!(
            !florida_lease
                .optional_inputs
                .contains(&"email_consent".to_string()),
            "email_consent should NOT be in template optional_inputs - \
             it belongs in signature ceremony where TENANT consents"
        );
    }

    #[test]
    fn test_flood_disclosure_offers_unknown_option() {
        // Flood disclosure must offer "I don't know / Property recently acquired"
        // option to maintain scrivener neutrality. Binary Yes/No is leading.
        //
        // Per STRATEGY.md lines 147-186, the wizard should have 3 options:
        // - "Yes, the property has flooded"
        // - "No known flooding events"
        // - "I don't know / Property recently acquired"
        let templates = list_templates();
        let florida_lease = templates
            .iter()
            .find(|t| t.name == "florida_lease")
            .unwrap();

        // The field metadata should indicate tristate, not boolean
        // For now, check that we have flood_unknown field alongside yes/no
        assert!(
            florida_lease
                .optional_inputs
                .contains(&"flood_status_unknown".to_string()),
            "Flood disclosure must offer 'unknown' option for scrivener neutrality"
        );
    }

    #[test]
    fn test_flood_disclosure_uses_neutral_field_names() {
        // Field names should not imply a default or lead the user.
        // "has_prior_flooding" implies asking "did you have flooding?"
        // which is slightly leading. Better: "flood_history_status" with
        // explicit tristate options.
        let templates = list_templates();
        let florida_lease = templates
            .iter()
            .find(|t| t.name == "florida_lease")
            .unwrap();

        // Should use neutral status field, not leading has_* boolean fields
        assert!(
            florida_lease
                .optional_inputs
                .contains(&"flood_history_status".to_string()),
            "Should use neutral 'flood_history_status' field with tristate options"
        );
    }

    #[test]
    fn test_optional_addenda_clearly_labeled_optional() {
        // Per scrivener adherence, optional addenda must be clearly
        // presented as optional without pushing user toward inclusion.
        // The UI should ask "Would you like to include..." not assume inclusion.
        let templates = list_templates();
        let florida_lease = templates
            .iter()
            .find(|t| t.name == "florida_lease")
            .unwrap();

        // Flood disclosure is MANDATORY per § 83.512, but the template
        // generation should still present it neutrally (user answers questions,
        // system generates compliant disclosure based on answers)
        assert!(
            florida_lease
                .optional_inputs
                .iter()
                .any(|f| f.contains("flood")),
            "Flood disclosure fields should exist for § 83.512 compliance"
        );
    }

    // ============================================================
    // HOA/CONDO ASSOCIATION ADDENDUM TESTS (P0 Gap)
    // Per FL_LEASE.md §3.1
    // ============================================================

    #[test]
    fn test_florida_lease_has_hoa_fields() {
        let templates = list_templates();
        let florida_lease = templates
            .iter()
            .find(|t| t.name == "florida_lease")
            .unwrap();

        // Must have HOA-related optional inputs
        assert!(
            florida_lease
                .optional_inputs
                .iter()
                .any(|f| f.contains("hoa") || f.contains("association")),
            "Florida lease should have HOA/Association optional inputs"
        );
    }

    #[test]
    fn test_florida_lease_has_hoa_name_field() {
        let templates = list_templates();
        let florida_lease = templates
            .iter()
            .find(|t| t.name == "florida_lease")
            .unwrap();

        assert!(
            florida_lease
                .optional_inputs
                .contains(&"hoa_name".to_string()),
            "Florida lease should have hoa_name optional input"
        );
    }

    // ============================================================
    // CDD DISCLOSURE TESTS (§ 190.048) (P0 Gap)
    // Per FL_LEASE.md: Boldfaced text required, assessment amounts
    // ============================================================

    #[test]
    fn test_florida_lease_has_cdd_fields() {
        let templates = list_templates();
        let florida_lease = templates
            .iter()
            .find(|t| t.name == "florida_lease")
            .unwrap();

        // Must have CDD-related optional inputs
        assert!(
            florida_lease
                .optional_inputs
                .iter()
                .any(|f| f.contains("cdd")),
            "Florida lease should have CDD optional inputs for § 190.048"
        );
    }

    #[test]
    fn test_florida_lease_has_cdd_assessment_field() {
        let templates = list_templates();
        let florida_lease = templates
            .iter()
            .find(|t| t.name == "florida_lease")
            .unwrap();

        assert!(
            florida_lease
                .optional_inputs
                .contains(&"cdd_assessment".to_string()),
            "Florida lease should have cdd_assessment optional input"
        );
    }

    // ============================================================
    // NEW FLORIDA REAL ESTATE TEMPLATES TESTS
    // ============================================================

    #[test]
    fn test_florida_purchase_contract_template_exists() {
        let templates = list_templates();
        let purchase_contract = templates
            .iter()
            .find(|t| t.name == "florida_purchase_contract");

        assert!(
            purchase_contract.is_some(),
            "florida_purchase_contract template should exist"
        );

        let template = purchase_contract.unwrap();

        // Required inputs for purchase contract
        assert!(
            template
                .required_inputs
                .contains(&"seller_name".to_string()),
            "Purchase contract should require seller_name"
        );
        assert!(
            template.required_inputs.contains(&"buyer_name".to_string()),
            "Purchase contract should require buyer_name"
        );
        assert!(
            template
                .required_inputs
                .contains(&"purchase_price".to_string()),
            "Purchase contract should require purchase_price"
        );
        assert!(
            template
                .required_inputs
                .contains(&"closing_date".to_string()),
            "Purchase contract should require closing_date"
        );

        // Flood disclosure fields (§ 689.302)
        assert!(
            template
                .optional_inputs
                .contains(&"has_prior_flooding".to_string()),
            "Purchase contract should have flood disclosure fields for § 689.302"
        );

        // HOA disclosure fields (§ 720.401)
        assert!(
            template.optional_inputs.contains(&"has_hoa".to_string()),
            "Purchase contract should have HOA disclosure fields for § 720.401"
        );
    }

    #[test]
    fn test_florida_escalation_addendum_template_exists() {
        let templates = list_templates();
        let escalation = templates
            .iter()
            .find(|t| t.name == "florida_escalation_addendum");

        assert!(
            escalation.is_some(),
            "florida_escalation_addendum template should exist"
        );

        let template = escalation.unwrap();

        // Critical escalation fields - these must all be required
        assert!(
            template
                .required_inputs
                .contains(&"base_purchase_price".to_string()),
            "Escalation addendum should require base_purchase_price"
        );
        assert!(
            template
                .required_inputs
                .contains(&"escalation_increment".to_string()),
            "Escalation addendum should require escalation_increment"
        );
        assert!(
            template
                .required_inputs
                .contains(&"maximum_purchase_price".to_string()),
            "Escalation addendum MUST require maximum_purchase_price (critical for compliance)"
        );

        // Appraisal gap fields
        assert!(
            template
                .optional_inputs
                .contains(&"appraisal_gap_coverage".to_string()),
            "Escalation should have appraisal gap coverage option"
        );
    }

    #[test]
    fn test_florida_listing_agreement_template_exists() {
        let templates = list_templates();
        let listing = templates
            .iter()
            .find(|t| t.name == "florida_listing_agreement");

        assert!(
            listing.is_some(),
            "florida_listing_agreement template should exist"
        );

        let template = listing.unwrap();

        // § 475.25 requires definite expiration date
        assert!(
            template
                .required_inputs
                .contains(&"listing_expiration_date".to_string()),
            "Listing agreement MUST require expiration date (§ 475.25 compliance)"
        );

        // § 475.278 brokerage relationship
        assert!(
            template
                .optional_inputs
                .contains(&"brokerage_relationship".to_string()),
            "Listing should have brokerage relationship field for § 475.278"
        );

        // License requirement
        assert!(
            template
                .required_inputs
                .contains(&"broker_license".to_string()),
            "Listing agreement should require broker license"
        );

        // Commission structure
        assert!(
            template
                .required_inputs
                .contains(&"commission_rate".to_string()),
            "Listing agreement should require commission rate"
        );
    }

    #[test]
    fn test_all_florida_real_estate_templates_have_source() {
        let template_names = [
            "florida_purchase_contract",
            "florida_escalation_addendum",
            "florida_listing_agreement",
        ];

        for name in &template_names {
            let source = get_template_source(name);
            assert!(
                source.is_ok(),
                "Template '{}' should have source code available. Error: {:?}",
                name,
                source.err()
            );

            let source_code = source.unwrap();
            assert!(
                !source_code.is_empty(),
                "Template '{}' source should not be empty",
                name
            );
        }
    }

    #[test]
    fn test_florida_templates_uri_format() {
        let templates = list_templates();

        for template in &templates {
            if template.name.starts_with("florida_") {
                assert!(
                    template.uri.starts_with("typst://templates/"),
                    "Florida template '{}' should have proper URI format. Got: {}",
                    template.name,
                    template.uri
                );

                // Verify URI matches name
                let expected_uri = format!("typst://templates/{}", template.name);
                assert_eq!(
                    template.uri, expected_uri,
                    "Template '{}' URI should match name pattern",
                    template.name
                );
            }
        }
    }

    #[test]
    fn test_template_count_includes_new_florida_templates() {
        let templates = list_templates();

        // Should have at least 10 templates now:
        // invoice, letter, florida_lease, florida_purchase_contract,
        // florida_escalation_addendum, florida_listing_agreement, texas_lease,
        // florida_flood_disclosure, florida_purchase_as_is, florida_commercial_lease
        assert!(
            templates.len() >= 10,
            "Should have at least 10 templates including new real estate templates. Got: {}",
            templates.len()
        );

        // Count Florida templates specifically
        let florida_count = templates
            .iter()
            .filter(|t| t.name.starts_with("florida_"))
            .count();

        assert!(
            florida_count >= 7,
            "Should have at least 7 Florida templates. Got: {}",
            florida_count
        );
    }

    // ============================================================
    // TEXAS LEASE TEMPLATE TESTS
    // ============================================================

    #[test]
    fn test_texas_lease_template_exists() {
        let templates = list_templates();
        let texas_lease = templates.iter().find(|t| t.name == "texas_lease");

        assert!(texas_lease.is_some(), "texas_lease template should exist");

        let template = texas_lease.unwrap();

        // Required inputs for Texas lease
        assert!(
            template
                .required_inputs
                .contains(&"landlord_name".to_string()),
            "Texas lease should require landlord_name"
        );
        assert!(
            template
                .required_inputs
                .contains(&"tenant_name".to_string()),
            "Texas lease should require tenant_name"
        );
        assert!(
            template
                .required_inputs
                .contains(&"property_address".to_string()),
            "Texas lease should require property_address"
        );
        assert!(
            template
                .required_inputs
                .contains(&"monthly_rent".to_string()),
            "Texas lease should require monthly_rent"
        );
        assert!(
            template
                .required_inputs
                .contains(&"security_deposit".to_string()),
            "Texas lease should require security_deposit"
        );
    }

    #[test]
    fn test_texas_lease_ch92_compliance_fields() {
        let templates = list_templates();
        let texas_lease = templates.iter().find(|t| t.name == "texas_lease").unwrap();

        // § 92.0081 lockout policy fields
        assert!(
            texas_lease
                .optional_inputs
                .iter()
                .any(|f| f.contains("lockout")),
            "Texas lease should have lockout policy fields for § 92.0081 compliance"
        );

        // § 92.103-109 security deposit fields
        assert!(
            texas_lease
                .optional_inputs
                .contains(&"deposit_bank_name".to_string())
                || texas_lease
                    .required_inputs
                    .contains(&"security_deposit".to_string()),
            "Texas lease should have security deposit fields for § 92.103-109"
        );

        // § 92.201 landlord disclosure fields
        assert!(
            texas_lease
                .optional_inputs
                .iter()
                .any(|f| f.contains("owner") || f.contains("agent")),
            "Texas lease should have landlord disclosure fields for § 92.201"
        );

        // § 92.0131 parking/towing fields
        assert!(
            texas_lease
                .optional_inputs
                .iter()
                .any(|f| f.contains("parking")),
            "Texas lease should have parking fields for § 92.0131 compliance"
        );
    }

    #[test]
    fn test_texas_lease_has_source() {
        let source = get_template_source("texas_lease");
        assert!(
            source.is_ok(),
            "texas_lease template should have source code available. Error: {:?}",
            source.err()
        );

        let source_code = source.unwrap();
        assert!(
            !source_code.is_empty(),
            "texas_lease source should not be empty"
        );

        // Verify key Texas Property Code references
        assert!(
            source_code.contains("92.103") || source_code.contains("Security Deposit"),
            "Texas lease should reference § 92.103 (security deposit)"
        );
        assert!(
            source_code.contains("92.0081") || source_code.contains("Lockout"),
            "Texas lease should reference § 92.0081 (lockout policy)"
        );
    }

    #[test]
    fn test_texas_lease_uri_format() {
        let templates = list_templates();
        let texas_lease = templates.iter().find(|t| t.name == "texas_lease").unwrap();

        assert!(
            texas_lease.uri.starts_with("typst://templates/"),
            "Texas lease should have proper URI format. Got: {}",
            texas_lease.uri
        );

        assert_eq!(
            texas_lease.uri, "typst://templates/texas_lease",
            "Texas lease URI should match expected format"
        );
    }

    // ============================================================
    // STANDALONE FLOOD DISCLOSURE TEMPLATE TESTS (§ 83.512)
    // SB 948 / HB 1015 compliance - effective Oct 1, 2025
    // ============================================================

    #[test]
    fn test_florida_flood_disclosure_template_exists() {
        let templates = list_templates();
        let flood_disclosure = templates
            .iter()
            .find(|t| t.name == "florida_flood_disclosure");

        assert!(
            flood_disclosure.is_some(),
            "florida_flood_disclosure standalone template should exist"
        );
    }

    #[test]
    fn test_florida_flood_disclosure_required_inputs() {
        let templates = list_templates();
        let template = templates
            .iter()
            .find(|t| t.name == "florida_flood_disclosure")
            .expect("florida_flood_disclosure template should exist");

        // Property address is required to identify the property
        assert!(
            template
                .required_inputs
                .contains(&"property_address".to_string()),
            "Flood disclosure should require property_address"
        );

        // Landlord name required for disclosure
        assert!(
            template
                .required_inputs
                .contains(&"landlord_name".to_string()),
            "Flood disclosure should require landlord_name"
        );
    }

    #[test]
    fn test_florida_flood_disclosure_has_tristate_fields() {
        let templates = list_templates();
        let template = templates
            .iter()
            .find(|t| t.name == "florida_flood_disclosure")
            .expect("florida_flood_disclosure template should exist");

        // Should have tristate flood history field
        assert!(
            template
                .optional_inputs
                .iter()
                .any(|f| f.contains("flood_history")),
            "Flood disclosure should have flood_history field"
        );

        // Should have tristate flood claims field
        assert!(
            template
                .optional_inputs
                .iter()
                .any(|f| f.contains("flood_claims")),
            "Flood disclosure should have flood_claims field"
        );

        // Should have tristate FEMA field
        assert!(
            template
                .optional_inputs
                .iter()
                .any(|f| f.contains("flood_fema") || f.contains("fema")),
            "Flood disclosure should have FEMA assistance field"
        );
    }
}
