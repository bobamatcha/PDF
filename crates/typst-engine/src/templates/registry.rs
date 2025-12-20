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
                // HB 615 - Electronic Notice Consent
                "email_consent".to_string(),
                // SB 948 - Flood Disclosure (§ 83.512)
                "has_prior_flooding".to_string(),
                "has_flood_claims".to_string(),
                "has_fema_assistance".to_string(),
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
}
