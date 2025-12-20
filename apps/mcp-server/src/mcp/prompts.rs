//! MCP Prompt definitions

use std::collections::HashMap;

use super::protocol::{Prompt, PromptArgument, PromptContent, PromptMessage};
use crate::compiler::errors::ServerError;

/// Get all prompt definitions
pub fn get_prompt_definitions() -> Vec<Prompt> {
    vec![
        Prompt {
            name: "generate_invoice".to_string(),
            description: Some("Generate a professional invoice document".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "company_name".to_string(),
                    description: Some("Name of the invoicing company".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "client_name".to_string(),
                    description: Some("Name of the client being invoiced".to_string()),
                    required: Some(true),
                },
            ]),
        },
        Prompt {
            name: "generate_florida_lease".to_string(),
            description: Some(
                "Generate a Florida-compliant residential lease agreement".to_string(),
            ),
            arguments: Some(vec![
                PromptArgument {
                    name: "landlord_name".to_string(),
                    description: Some("Full legal name of the landlord".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "tenant_name".to_string(),
                    description: Some("Full legal name of the tenant".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "property_address".to_string(),
                    description: Some("Full address of the rental property".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "year_built".to_string(),
                    description: Some(
                        "Year the property was built (for lead paint disclosure)".to_string(),
                    ),
                    required: Some(true),
                },
            ]),
        },
    ]
}

/// Get a prompt by name with arguments
pub fn get_prompt(
    name: &str,
    arguments: HashMap<String, String>,
) -> Result<Vec<PromptMessage>, ServerError> {
    match name {
        "generate_invoice" => expand_invoice_prompt(arguments),
        "generate_florida_lease" => expand_florida_lease_prompt(arguments),
        _ => Err(ServerError::PromptNotFound(name.to_string())),
    }
}

fn expand_invoice_prompt(args: HashMap<String, String>) -> Result<Vec<PromptMessage>, ServerError> {
    let company = args
        .get("company_name")
        .map(|s| s.as_str())
        .unwrap_or("[Company]");
    let client = args
        .get("client_name")
        .map(|s| s.as_str())
        .unwrap_or("[Client]");

    let text = format!(
        r#"Generate an invoice for {} to bill {}.

Use the `render_document` tool with:
- source: "typst://templates/invoice"
- inputs: {{
    "company_name": "{}",
    "client_name": "{}",
    "items": [
      {{"description": "Service 1", "qty": 1, "price": 100}}
    ]
  }}
- format: "pdf"

The template will handle formatting and calculations automatically."#,
        company, client, company, client
    );

    Ok(vec![PromptMessage {
        role: "user".to_string(),
        content: PromptContent::Text { text },
    }])
}

fn expand_florida_lease_prompt(
    args: HashMap<String, String>,
) -> Result<Vec<PromptMessage>, ServerError> {
    let landlord = args
        .get("landlord_name")
        .map(|s| s.as_str())
        .unwrap_or("[Landlord]");
    let tenant = args
        .get("tenant_name")
        .map(|s| s.as_str())
        .unwrap_or("[Tenant]");
    let address = args
        .get("property_address")
        .map(|s| s.as_str())
        .unwrap_or("[Address]");
    let year_built: u32 = args
        .get("year_built")
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);

    let is_pre_1978 = year_built < 1978;
    let lead_paint_note = if is_pre_1978 {
        "**IMPORTANT**: Property was built before 1978. Lead-based paint disclosure is REQUIRED."
    } else {
        "Property was built after 1978. Lead-based paint disclosure is not required."
    };

    let text = format!(
        r#"Generate a Florida residential lease agreement.

## Property Information
- Landlord: {}
- Tenant: {}
- Property: {}
- Year Built: {}

{}

## Required Statutory Disclosures
1. **Radon Gas Notification** (F.S. § 404.056) - MANDATORY for all Florida leases
2. **Security Deposit Notice** (F.S. § 83.49) - Required if collecting a deposit

Use the `render_document` tool with:
- source: "typst://templates/florida_lease"
- inputs: {{
    "landlord_name": "{}",
    "tenant_name": "{}",
    "property_address": "{}",
    "year_built": "{}",
    "is_pre_1978": "{}",
    "monthly_rent": "[AMOUNT]",
    "lease_start": "[START DATE]",
    "lease_end": "[END DATE]"
  }}
- format: "pdf"

The template includes all mandatory Florida statutory disclosures."#,
        landlord,
        tenant,
        address,
        year_built,
        lead_paint_note,
        landlord,
        tenant,
        address,
        year_built,
        is_pre_1978
    );

    Ok(vec![PromptMessage {
        role: "user".to_string(),
        content: PromptContent::Text { text },
    }])
}
