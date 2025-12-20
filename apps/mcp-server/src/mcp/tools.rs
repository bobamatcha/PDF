//! MCP Tool definitions and handlers

use serde_json::json;

use super::protocol::{Tool, ToolResultContent};
use crate::compiler::errors::ServerError;
use crate::compiler::{compile_document, validate_syntax, RenderRequest};
use crate::templates;
use crate::verifier::verify_lease;
use crate::world::fonts::global_font_cache;

/// Get all tool definitions
pub fn get_tool_definitions() -> Vec<Tool> {
    vec![
        Tool {
            name: "render_document".to_string(),
            description: Some(
                "Compiles a Typst template with dynamic data into PDF, SVG, or PNG format"
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Raw Typst source code OR a template URI (e.g., typst://templates/invoice)"
                    },
                    "inputs": {
                        "type": "object",
                        "description": "Variables injected into sys.inputs in the template",
                        "additionalProperties": true
                    },
                    "assets": {
                        "type": "object",
                        "description": "Binary assets as base64 strings, keyed by filename (e.g., {\"logo.png\": \"base64...\"})",
                        "additionalProperties": { "type": "string" }
                    },
                    "format": {
                        "type": "string",
                        "enum": ["pdf", "svg", "png"],
                        "default": "pdf",
                        "description": "Output format"
                    },
                    "ppi": {
                        "type": "integer",
                        "description": "Pixels per inch for PNG output (default: 144)",
                        "default": 144
                    }
                },
                "required": ["source"]
            }),
        },
        Tool {
            name: "validate_syntax".to_string(),
            description: Some(
                "Parses Typst source and returns syntax errors without full compilation"
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Raw Typst source code to validate"
                    }
                },
                "required": ["source"]
            }),
        },
        Tool {
            name: "list_fonts".to_string(),
            description: Some("Returns a list of all fonts available in the server".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "list_templates".to_string(),
            description: Some(
                "Returns a list of available template URIs with their descriptions".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "verify_lease".to_string(),
            description: Some(
                "Verify a lease PDF for Florida compliance and detect anomalies".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pdf_base64": {
                        "type": "string",
                        "description": "Base64-encoded PDF file"
                    },
                    "state": {
                        "type": "string",
                        "enum": ["florida"],
                        "default": "florida",
                        "description": "State-specific rules to apply"
                    },
                    "detect_anomalies": {
                        "type": "boolean",
                        "default": true,
                        "description": "Enable surreptitious clause detection"
                    }
                },
                "required": ["pdf_base64"]
            }),
        },
    ]
}

/// Handle a tool call
pub async fn handle_tool_call(
    name: &str,
    arguments: serde_json::Value,
    timeout_ms: u64,
) -> Result<Vec<ToolResultContent>, ServerError> {
    match name {
        "render_document" => handle_render_document(arguments, timeout_ms).await,
        "validate_syntax" => handle_validate_syntax(arguments),
        "list_fonts" => handle_list_fonts(),
        "list_templates" => handle_list_templates(),
        "verify_lease" => handle_verify_lease(arguments),
        _ => Err(ServerError::UnknownTool(name.to_string())),
    }
}

async fn handle_render_document(
    args: serde_json::Value,
    timeout_ms: u64,
) -> Result<Vec<ToolResultContent>, ServerError> {
    let request: RenderRequest = serde_json::from_value(args)?;
    let response = compile_document(request, timeout_ms).await?;

    let result_json = serde_json::to_string_pretty(&response)?;
    Ok(vec![ToolResultContent::Text { text: result_json }])
}

fn handle_validate_syntax(args: serde_json::Value) -> Result<Vec<ToolResultContent>, ServerError> {
    let source = args
        .get("source")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServerError::InvalidArgument("source is required".to_string()))?;

    let errors = validate_syntax(source);
    let result = json!({
        "valid": errors.is_empty(),
        "errors": errors
    });

    Ok(vec![ToolResultContent::Text {
        text: serde_json::to_string_pretty(&result)?,
    }])
}

fn handle_list_fonts() -> Result<Vec<ToolResultContent>, ServerError> {
    let cache = global_font_cache();
    let families = cache.list_font_families();
    let all_fonts = cache.list_all_fonts();

    // Group fonts by family for better readability
    let fonts_by_family: Vec<_> = families
        .iter()
        .map(|family| {
            let variants = all_fonts
                .iter()
                .filter(|f| &f.family == family)
                .map(|f| {
                    json!({
                        "style": &f.style,
                        "weight": f.weight,
                        "stretch": &f.stretch
                    })
                })
                .collect::<Vec<_>>();

            json!({
                "family": family,
                "variants": variants
            })
        })
        .collect();

    let result = json!({
        "families": fonts_by_family,
        "total_families": families.len(),
        "total_fonts": all_fonts.len()
    });

    Ok(vec![ToolResultContent::Text {
        text: serde_json::to_string_pretty(&result)?,
    }])
}

fn handle_list_templates() -> Result<Vec<ToolResultContent>, ServerError> {
    let templates = templates::list_templates();
    let result = json!({
        "templates": templates,
        "count": templates.len()
    });

    Ok(vec![ToolResultContent::Text {
        text: serde_json::to_string_pretty(&result)?,
    }])
}

fn handle_verify_lease(args: serde_json::Value) -> Result<Vec<ToolResultContent>, ServerError> {
    // Extract pdf_base64 (required)
    let pdf_base64 = args
        .get("pdf_base64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServerError::InvalidArgument("pdf_base64 is required".to_string()))?;

    // Extract state (optional, default to "florida")
    let state = args
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("florida");

    // Extract detect_anomalies (optional, default to true)
    let detect_anomalies = args
        .get("detect_anomalies")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Decode base64 PDF
    let pdf_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, pdf_base64)
        .map_err(|e| ServerError::InvalidArgument(format!("Invalid base64: {}", e)))?;

    // Call verify_lease function
    let report = verify_lease(&pdf_bytes, state, detect_anomalies)
        .map_err(|e| ServerError::SourceError(format!("Verification failed: {}", e)))?;

    // Return JSON response
    let result = json!({
        "status": report.summary.status,
        "compliance_checks": report.compliance_results,
        "anomalies": report.anomalies,
        "summary": report.summary
    });

    Ok(vec![ToolResultContent::Text {
        text: serde_json::to_string_pretty(&result)?,
    }])
}
