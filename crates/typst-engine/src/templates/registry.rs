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
            template.required_inputs.contains(&"seller_name".to_string()),
            "Purchase contract should require seller_name"
        );
        assert!(
            template.required_inputs.contains(&"buyer_name".to_string()),
            "Purchase contract should require buyer_name"
        );
        assert!(
            template.required_inputs.contains(&"purchase_price".to_string()),
            "Purchase contract should require purchase_price"
        );
        assert!(
            template.required_inputs.contains(&"closing_date".to_string()),
            "Purchase contract should require closing_date"
        );

        // Flood disclosure fields (§ 689.302)
        assert!(
            template.optional_inputs.contains(&"has_prior_flooding".to_string()),
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
            template.required_inputs.contains(&"base_purchase_price".to_string()),
            "Escalation addendum should require base_purchase_price"
        );
        assert!(
            template.required_inputs.contains(&"escalation_increment".to_string()),
            "Escalation addendum should require escalation_increment"
        );
        assert!(
            template.required_inputs.contains(&"maximum_purchase_price".to_string()),
            "Escalation addendum MUST require maximum_purchase_price (critical for compliance)"
        );

        // Appraisal gap fields
        assert!(
            template.optional_inputs.contains(&"appraisal_gap_coverage".to_string()),
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
            template.required_inputs.contains(&"listing_expiration_date".to_string()),
            "Listing agreement MUST require expiration date (§ 475.25 compliance)"
        );

        // § 475.278 brokerage relationship
        assert!(
            template.optional_inputs.contains(&"brokerage_relationship".to_string()),
            "Listing should have brokerage relationship field for § 475.278"
        );

        // License requirement
        assert!(
            template.required_inputs.contains(&"broker_license".to_string()),
            "Listing agreement should require broker license"
        );

        // Commission structure
        assert!(
            template.required_inputs.contains(&"commission_rate".to_string()),
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

        // Should have at least 6 templates now:
        // invoice, letter, florida_lease, florida_purchase_contract,
        // florida_escalation_addendum, florida_listing_agreement
        assert!(
            templates.len() >= 6,
            "Should have at least 6 templates including new Florida real estate templates. Got: {}",
            templates.len()
        );

        // Count Florida templates specifically
        let florida_count = templates
            .iter()
            .filter(|t| t.name.starts_with("florida_"))
            .count();

        assert!(
            florida_count >= 4,
            "Should have at least 4 Florida templates. Got: {}",
            florida_count
        );
    }
}
