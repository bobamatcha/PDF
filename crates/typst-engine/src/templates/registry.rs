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
}
