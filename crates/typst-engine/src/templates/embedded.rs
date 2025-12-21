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

/// Get an embedded template by name
pub fn get_embedded_template(name: &str) -> Option<String> {
    match name {
        "invoice" => Some(INVOICE_TEMPLATE.to_string()),
        "letter" => Some(LETTER_TEMPLATE.to_string()),
        "florida_lease" => Some(FLORIDA_LEASE_TEMPLATE.to_string()),
        "florida_purchase_contract" => Some(FLORIDA_PURCHASE_CONTRACT_TEMPLATE.to_string()),
        "florida_escalation_addendum" => Some(FLORIDA_ESCALATION_ADDENDUM_TEMPLATE.to_string()),
        "florida_listing_agreement" => Some(FLORIDA_LISTING_AGREEMENT_TEMPLATE.to_string()),
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

    #[test]
    fn test_unknown_template() {
        let template = get_embedded_template("nonexistent");
        assert!(template.is_none());
    }

    #[test]
    fn test_list_embedded_templates() {
        let templates = list_embedded_templates();
        assert_eq!(templates.len(), 6);
        assert!(templates.contains(&"invoice"));
        assert!(templates.contains(&"letter"));
        assert!(templates.contains(&"florida_lease"));
        assert!(templates.contains(&"florida_purchase_contract"));
        assert!(templates.contains(&"florida_escalation_addendum"));
        assert!(templates.contains(&"florida_listing_agreement"));
    }
}
