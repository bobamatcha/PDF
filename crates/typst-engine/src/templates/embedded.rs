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

/// Get an embedded template by name
pub fn get_embedded_template(name: &str) -> Option<String> {
    match name {
        "invoice" => Some(INVOICE_TEMPLATE.to_string()),
        "letter" => Some(LETTER_TEMPLATE.to_string()),
        "florida_lease" => Some(FLORIDA_LEASE_TEMPLATE.to_string()),
        _ => None,
    }
}

/// List all available embedded template names
pub fn list_embedded_templates() -> Vec<&'static str> {
    vec!["invoice", "letter", "florida_lease"]
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
    fn test_unknown_template() {
        let template = get_embedded_template("nonexistent");
        assert!(template.is_none());
    }

    #[test]
    fn test_list_embedded_templates() {
        let templates = list_embedded_templates();
        assert_eq!(templates.len(), 3);
        assert!(templates.contains(&"invoice"));
        assert!(templates.contains(&"letter"));
        assert!(templates.contains(&"florida_lease"));
    }
}
