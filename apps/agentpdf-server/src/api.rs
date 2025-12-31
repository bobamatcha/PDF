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

    /// Document type - supports all Florida document categories:
    ///
    /// **Lease Documents (Chapter 83):**
    /// - `lease` - Residential lease agreement (default)
    /// - `lease_termination` - Lease termination notice
    /// - `eviction` - Eviction notice
    ///
    /// **Real Estate Purchase (Chapter 475, 689):**
    /// - `purchase` - Real estate purchase contract
    /// - `purchase_as_is` - As-Is purchase contract
    /// - `inspection_contingency` - Inspection contingency addendum
    /// - `financing_contingency` - Financing contingency addendum
    /// - `escalation` - Escalation addendum
    /// - `appraisal_contingency` - Appraisal contingency addendum
    ///
    /// **Listing Documents (Chapter 475):**
    /// - `listing` - Exclusive listing agreement
    ///
    /// **Contractor Documents (Chapter 713):**
    /// - `contractor_invoice` - Contractor invoice
    /// - `cost_of_materials` - Cost of materials bill
    /// - `notice_of_commencement` - Notice of Commencement (§ 713.13)
    /// - `notice_to_owner` - Notice to Owner (§ 713.06)
    /// - `claim_of_lien` - Claim of Lien (§ 713.08)
    /// - `release_of_lien` - Release of Lien (§ 713.21)
    /// - `dispute_lien` - Dispute of Lien (§ 713.22)
    /// - `fraudulent_lien` - Fraudulent Lien Report (§ 713.31)
    /// - `final_payment_affidavit` - Final Payment Affidavit
    ///
    /// **Bill of Sale (Chapter 319) - Phase 1.1:**
    /// - `bill_of_sale_car` - Bill of Sale for car
    /// - `bill_of_sale_boat` - Bill of Sale for boat
    /// - `bill_of_sale_trailer` - Bill of Sale for trailer
    /// - `bill_of_sale_jetski` - Bill of Sale for jet ski
    /// - `bill_of_sale_mobile_home` - Bill of Sale for mobile home
    ///
    /// **Auto-detect:**
    /// - `auto` - Automatically detect document type
    #[serde(default = "default_doc_type")]
    pub document_type: String,
}

fn default_state() -> String {
    "FL".to_string()
}

fn default_doc_type() -> String {
    "lease".to_string()
}

/// Supported document types response
#[derive(Serialize)]
pub struct DocumentTypesResponse {
    pub success: bool,
    pub categories: Vec<DocumentCategoryInfo>,
    pub total_types: usize,
}

/// Document category information
#[derive(Serialize)]
pub struct DocumentCategoryInfo {
    pub category: String,
    pub chapter: String,
    pub types: Vec<DocumentTypeInfo>,
}

/// Document type information
#[derive(Serialize)]
pub struct DocumentTypeInfo {
    pub api_value: String,
    pub name: String,
    pub description: String,
    pub statutes: Vec<String>,
}

/// Handler: GET /api/document-types
pub async fn handle_list_document_types() -> Json<DocumentTypesResponse> {
    let categories = vec![
        DocumentCategoryInfo {
            category: "Lease".to_string(),
            chapter: "Chapter 83".to_string(),
            types: vec![
                DocumentTypeInfo {
                    api_value: "lease".to_string(),
                    name: "Residential Lease Agreement".to_string(),
                    description: "Standard residential lease agreement".to_string(),
                    statutes: vec![
                        "§ 83.47 - Prohibited provisions".to_string(),
                        "§ 83.48 - Attorney fees".to_string(),
                        "§ 83.49 - Security deposits".to_string(),
                    ],
                },
                DocumentTypeInfo {
                    api_value: "lease_termination".to_string(),
                    name: "Lease Termination Notice".to_string(),
                    description: "Notice to terminate tenancy".to_string(),
                    statutes: vec![
                        "§ 83.56 - Termination".to_string(),
                        "§ 83.57 - Month-to-month".to_string(),
                    ],
                },
                DocumentTypeInfo {
                    api_value: "eviction".to_string(),
                    name: "Eviction Notice".to_string(),
                    description: "Notice of eviction proceedings".to_string(),
                    statutes: vec!["§ 83.59 - Right of action".to_string()],
                },
            ],
        },
        DocumentCategoryInfo {
            category: "Real Estate Purchase".to_string(),
            chapter: "Chapter 475, 689".to_string(),
            types: vec![
                DocumentTypeInfo {
                    api_value: "purchase".to_string(),
                    name: "Purchase Contract".to_string(),
                    description: "Standard residential purchase contract".to_string(),
                    statutes: vec![
                        "§ 404.056 - Radon disclosure".to_string(),
                        "§ 689.261 - Property tax disclosure".to_string(),
                        "§ 689.302 - Flood disclosure".to_string(),
                    ],
                },
                DocumentTypeInfo {
                    api_value: "purchase_as_is".to_string(),
                    name: "As-Is Purchase Contract".to_string(),
                    description: "Purchase contract with as-is provisions".to_string(),
                    statutes: vec!["Same as purchase contract".to_string()],
                },
                DocumentTypeInfo {
                    api_value: "escalation".to_string(),
                    name: "Escalation Addendum".to_string(),
                    description: "Price escalation clause addendum".to_string(),
                    statutes: vec!["Escalation clause best practices".to_string()],
                },
                DocumentTypeInfo {
                    api_value: "inspection_contingency".to_string(),
                    name: "Inspection Contingency".to_string(),
                    description: "Home inspection contingency addendum".to_string(),
                    statutes: vec![],
                },
                DocumentTypeInfo {
                    api_value: "financing_contingency".to_string(),
                    name: "Financing Contingency".to_string(),
                    description: "Mortgage financing contingency addendum".to_string(),
                    statutes: vec![],
                },
            ],
        },
        DocumentCategoryInfo {
            category: "Listing".to_string(),
            chapter: "Chapter 475".to_string(),
            types: vec![DocumentTypeInfo {
                api_value: "listing".to_string(),
                name: "Exclusive Listing Agreement".to_string(),
                description: "Exclusive right to sell listing agreement".to_string(),
                statutes: vec![
                    "§ 475.278 - Brokerage relationship".to_string(),
                    "§ 475.25 - Expiration date".to_string(),
                ],
            }],
        },
        DocumentCategoryInfo {
            category: "Contractor".to_string(),
            chapter: "Chapter 713".to_string(),
            types: vec![
                DocumentTypeInfo {
                    api_value: "notice_of_commencement".to_string(),
                    name: "Notice of Commencement".to_string(),
                    description: "Notice recorded before construction begins".to_string(),
                    statutes: vec!["§ 713.13 - Notice of Commencement".to_string()],
                },
                DocumentTypeInfo {
                    api_value: "notice_to_owner".to_string(),
                    name: "Notice to Owner".to_string(),
                    description: "Preliminary notice to preserve lien rights".to_string(),
                    statutes: vec!["§ 713.06 - Notice to Owner".to_string()],
                },
                DocumentTypeInfo {
                    api_value: "claim_of_lien".to_string(),
                    name: "Claim of Lien".to_string(),
                    description: "Construction lien claim".to_string(),
                    statutes: vec!["§ 713.08 - Claim of Lien".to_string()],
                },
                DocumentTypeInfo {
                    api_value: "release_of_lien".to_string(),
                    name: "Release of Lien".to_string(),
                    description: "Lien release/satisfaction".to_string(),
                    statutes: vec!["§ 713.21 - Release of Lien".to_string()],
                },
                DocumentTypeInfo {
                    api_value: "dispute_lien".to_string(),
                    name: "Dispute of Lien".to_string(),
                    description: "Contest of lien filing".to_string(),
                    statutes: vec!["§ 713.22 - Contest of Lien".to_string()],
                },
                DocumentTypeInfo {
                    api_value: "fraudulent_lien".to_string(),
                    name: "Fraudulent Lien Report".to_string(),
                    description: "Report of allegedly fraudulent lien".to_string(),
                    statutes: vec!["§ 713.31 - Fraudulent Liens".to_string()],
                },
                DocumentTypeInfo {
                    api_value: "contractor_invoice".to_string(),
                    name: "Contractor Invoice".to_string(),
                    description: "Invoice for contractor services".to_string(),
                    statutes: vec![],
                },
                DocumentTypeInfo {
                    api_value: "cost_of_materials".to_string(),
                    name: "Cost of Materials Bill".to_string(),
                    description: "Bill for construction materials".to_string(),
                    statutes: vec![],
                },
            ],
        },
    ];

    let total_types = categories.iter().map(|c| c.types.len()).sum();

    Json(DocumentTypesResponse {
        success: true,
        categories,
        total_types,
    })
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

    // Parse document type
    let doc_type = parse_document_type(&req.document_type)?;

    // Use the unified compliance checker
    let violations = if doc_type == CEDocType::Unknown {
        // Auto-detect mode
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
    } else {
        engine.check_document_text_compliance(&jurisdiction, &req.text, doc_type, req.year_built)
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

/// Parse document type string into DocumentType enum
fn parse_document_type(doc_type: &str) -> Result<CEDocType, ServerError> {
    match doc_type.to_lowercase().as_str() {
        // Lease Documents (Chapter 83)
        "lease" => Ok(CEDocType::Lease),
        "lease_termination" => Ok(CEDocType::LeaseTerminationNotice),
        "eviction" => Ok(CEDocType::EvictionNotice),

        // Real Estate Purchase (Chapter 475, 689)
        "purchase" => Ok(CEDocType::RealEstatePurchase),
        "purchase_as_is" => Ok(CEDocType::RealEstatePurchaseAsIs),
        "inspection_contingency" => Ok(CEDocType::InspectionContingency),
        "financing_contingency" => Ok(CEDocType::FinancingContingency),
        "escalation" => Ok(CEDocType::EscalationAddendum),
        "appraisal_contingency" => Ok(CEDocType::AppraisalContingency),

        // Listing Documents (Chapter 475)
        "listing" => Ok(CEDocType::ListingAgreement),

        // Contractor Documents (Chapter 713)
        "contractor_invoice" => Ok(CEDocType::ContractorInvoice),
        "cost_of_materials" => Ok(CEDocType::CostOfMaterialsBill),
        "notice_of_commencement" => Ok(CEDocType::NoticeOfCommencement),
        "notice_to_owner" => Ok(CEDocType::NoticeToOwner),
        "claim_of_lien" => Ok(CEDocType::ClaimOfLien),
        "release_of_lien" => Ok(CEDocType::ReleaseOfLien),
        "dispute_lien" => Ok(CEDocType::DisputeLien),
        "fraudulent_lien" => Ok(CEDocType::FraudulentLienReport),
        "final_payment_affidavit" => Ok(CEDocType::FinalPaymentAffidavit),

        // Bill of Sale (Chapter 319) - Phase 1.1
        "bill_of_sale_car" => Ok(CEDocType::BillOfSaleCar),
        "bill_of_sale_boat" => Ok(CEDocType::BillOfSaleBoat),
        "bill_of_sale_trailer" => Ok(CEDocType::BillOfSaleTrailer),
        "bill_of_sale_jetski" => Ok(CEDocType::BillOfSaleJetSki),
        "bill_of_sale_mobile_home" => Ok(CEDocType::BillOfSaleMobileHome),

        // Auto-detect
        "auto" => Ok(CEDocType::Unknown),

        other => Err(ServerError::InvalidRequest(format!(
            "Unknown document type '{}'. Use GET /api/document-types for list of supported types.",
            other
        ))),
    }
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
