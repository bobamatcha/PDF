//! API handlers for the agentPDF server
//!
//! Provides REST endpoints for:
//! - Template rendering
//! - Compliance checking
//! - Template listing

use std::collections::HashMap;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::ServerError;
use crate::AppState;

// Re-export from typst-engine for consistent types
use typst_engine::compiler::{OutputFormat, RenderRequest, RenderStatus};
use typst_engine::templates::list_templates;

// Compliance engine types
use compliance_engine::{
    ComplianceEngine, DocumentType as CEDocType, Jurisdiction, State as CEState,
};
use shared_types::LeaseDocument;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

/// Handler: GET /health
pub async fn handle_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        service: "agentpdf-server",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Template list response
#[derive(Serialize)]
pub struct TemplateListResponse {
    pub success: bool,
    pub templates: Vec<TemplateInfo>,
    pub count: usize,
}

/// Template metadata
#[derive(Serialize)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub uri: String,
    pub required_inputs: Vec<String>,
    pub optional_inputs: Vec<String>,
}

/// Handler: GET /api/templates
pub async fn handle_list_templates() -> Json<TemplateListResponse> {
    let templates: Vec<TemplateInfo> = list_templates()
        .into_iter()
        .map(|t| TemplateInfo {
            name: t.name,
            description: t.description,
            uri: t.uri,
            required_inputs: t.required_inputs,
            optional_inputs: t.optional_inputs,
        })
        .collect();

    let count = templates.len();

    Json(TemplateListResponse {
        success: true,
        templates,
        count,
    })
}

/// Render request body
#[derive(Deserialize)]
pub struct RenderApiRequest {
    /// Template name (e.g., "florida_lease") or raw Typst source
    pub template: String,

    /// True if `template` is a template name, false if raw Typst source
    #[serde(default = "default_is_template")]
    pub is_template: bool,

    /// Input values for template variables
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,

    /// Output format: "pdf", "svg", or "png"
    #[serde(default = "default_format")]
    pub format: String,

    /// PPI for PNG output (optional)
    pub ppi: Option<u32>,
}

fn default_is_template() -> bool {
    true
}

fn default_format() -> String {
    "pdf".to_string()
}

/// Render response
#[derive(Serialize)]
pub struct RenderApiResponse {
    pub success: bool,
    /// Base64-encoded output (PDF/SVG/PNG)
    pub data: Option<String>,
    /// MIME type of output
    pub mime_type: Option<String>,
    /// Number of pages (for PDF)
    pub page_count: Option<usize>,
    /// Error message if failed
    pub error: Option<String>,
    /// Compilation warnings
    pub warnings: Option<Vec<String>>,
}

/// Handler: POST /api/render
pub async fn handle_render_template(
    State(state): State<AppState>,
    Json(req): Json<RenderApiRequest>,
) -> Result<Json<RenderApiResponse>, ServerError> {
    info!(
        "Render request: template={}, format={}",
        req.template, req.format
    );
    debug!("Inputs: {:?}", req.inputs);

    // Build source URI or raw source
    let source = if req.is_template {
        format!("typst://templates/{}", req.template)
    } else {
        req.template.clone()
    };

    // Parse output format
    let format = match req.format.to_lowercase().as_str() {
        "pdf" => OutputFormat::Pdf,
        "svg" => OutputFormat::Svg,
        "png" => OutputFormat::Png,
        other => {
            return Err(ServerError::InvalidRequest(format!(
                "Invalid format '{}'. Must be 'pdf', 'svg', or 'png'",
                other
            )));
        }
    };

    // Build render request
    let render_req = RenderRequest {
        source,
        inputs: req.inputs,
        assets: HashMap::new(),
        format,
        ppi: req.ppi,
    };

    // Render with timeout
    let response = typst_engine::compile_document(render_req, state.timeout_ms)
        .await
        .map_err(ServerError::from)?;

    // Extract warnings
    let warnings: Vec<String> = response
        .warnings
        .iter()
        .map(|w| w.message.clone())
        .collect();

    match response.status {
        RenderStatus::Success => {
            let artifact = response.artifact.ok_or_else(|| {
                ServerError::Internal("Render succeeded but no artifact produced".into())
            })?;

            Ok(Json(RenderApiResponse {
                success: true,
                data: Some(artifact.data_base64),
                mime_type: Some(artifact.mime_type),
                page_count: Some(artifact.page_count),
                error: None,
                warnings: if warnings.is_empty() {
                    None
                } else {
                    Some(warnings)
                },
            }))
        }
        RenderStatus::Error => {
            let error_msg = response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");

            Err(ServerError::CompileError(error_msg))
        }
    }
}

/// Compliance check request
#[derive(Deserialize)]
pub struct ComplianceRequest {
    /// Document text to check
    pub text: String,

    /// State code (e.g., "FL", "TX")
    #[serde(default = "default_state")]
    pub state: String,

    /// Year property was built (for lead paint checks)
    pub year_built: Option<u32>,

    /// ZIP code for locality-specific rules
    pub zip_code: Option<String>,

    /// Document type: "lease", "purchase", "listing"
    #[serde(default = "default_doc_type")]
    pub document_type: String,
}

fn default_state() -> String {
    "FL".to_string()
}

fn default_doc_type() -> String {
    "lease".to_string()
}

/// Compliance check response
#[derive(Serialize)]
pub struct ComplianceResponse {
    pub success: bool,
    pub compliant: bool,
    pub violations: Vec<ViolationInfo>,
    pub violation_count: usize,
}

/// Violation details
#[derive(Serialize)]
pub struct ViolationInfo {
    pub statute: String,
    pub message: String,
    pub severity: String,
    pub page: Option<u32>,
    pub text_snippet: Option<String>,
}

/// Handler: POST /api/compliance
pub async fn handle_check_compliance(
    Json(req): Json<ComplianceRequest>,
) -> Result<Json<ComplianceResponse>, ServerError> {
    info!(
        "Compliance check: state={}, doc_type={}",
        req.state, req.document_type
    );

    let engine = ComplianceEngine::new();

    // Parse state code
    let state = parse_state_code(&req.state)?;

    // Create jurisdiction (with optional locality from ZIP)
    let jurisdiction = if let Some(zip) = &req.zip_code {
        Jurisdiction::from_zip(state, zip)
    } else {
        Jurisdiction::new(state)
    };

    // Use compliance-engine to check document
    let violations = match req.document_type.as_str() {
        "lease" => engine.check_text_with_jurisdiction(&jurisdiction, &req.text, req.year_built),
        "purchase" => engine.check_realestate_text_with_jurisdiction(
            &jurisdiction,
            &req.text,
            CEDocType::RealEstatePurchase,
            req.year_built,
        ),
        "listing" => engine.check_realestate_text_with_jurisdiction(
            &jurisdiction,
            &req.text,
            CEDocType::ListingAgreement,
            req.year_built,
        ),
        "auto" => {
            // Auto-detect document type
            let doc = LeaseDocument {
                id: "api-check".to_string(),
                filename: "uploaded.pdf".to_string(),
                pages: 1,
                text_content: vec![req.text.clone()],
                created_at: 0,
            };
            engine
                .check_auto_detect(&jurisdiction, &doc, req.year_built)
                .violations
        }
        other => {
            return Err(ServerError::InvalidRequest(format!(
                "Unknown document type '{}'. Must be 'lease', 'purchase', 'listing', or 'auto'",
                other
            )));
        }
    };

    let violation_infos: Vec<ViolationInfo> = violations
        .iter()
        .map(|v| ViolationInfo {
            statute: v.statute.clone(),
            message: v.message.clone(),
            severity: format!("{:?}", v.severity),
            page: v.page,
            text_snippet: v.text_snippet.clone(),
        })
        .collect();

    let count = violation_infos.len();

    Ok(Json(ComplianceResponse {
        success: true,
        compliant: count == 0,
        violations: violation_infos,
        violation_count: count,
    }))
}

/// Parse state code string into State enum
fn parse_state_code(code: &str) -> Result<CEState, ServerError> {
    match code.to_uppercase().as_str() {
        "FL" => Ok(CEState::FL),
        "TX" => Ok(CEState::TX),
        "CA" => Ok(CEState::CA),
        "NY" => Ok(CEState::NY),
        "GA" => Ok(CEState::GA),
        "IL" => Ok(CEState::IL),
        "PA" => Ok(CEState::PA),
        "NJ" => Ok(CEState::NJ),
        "VA" => Ok(CEState::VA),
        "MA" => Ok(CEState::MA),
        "OH" => Ok(CEState::OH),
        "MI" => Ok(CEState::MI),
        "WA" => Ok(CEState::WA),
        "AZ" => Ok(CEState::AZ),
        "NC" => Ok(CEState::NC),
        "TN" => Ok(CEState::TN),
        other => Err(ServerError::InvalidRequest(format!(
            "Unsupported state code '{}'. Supported: FL, TX, CA, NY, GA, IL, PA, NJ, VA, MA, OH, MI, WA, AZ, NC, TN",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        let response = handle_health().await;
        assert_eq!(response.status, "healthy");
        assert_eq!(response.service, "agentpdf-server");
    }

    #[tokio::test]
    async fn test_list_templates() {
        let response = handle_list_templates().await;
        assert!(response.success);
        assert!(response.count > 0);

        // Check for Florida lease template
        let has_florida_lease = response.templates.iter().any(|t| t.name == "florida_lease");
        assert!(has_florida_lease, "Should have florida_lease template");
    }
}
