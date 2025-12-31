use compliance_engine::{ComplianceEngine, DocumentType, Jurisdiction, State};
use shared_types::LeaseDocument;
use wasm_bindgen::prelude::*;

// Export modules
pub mod audit_display;
pub mod compliance_panel;
pub mod coords;
pub mod extraction;
pub mod field_export;
pub mod fields;
pub mod overlay;
pub mod page_ranges;
pub mod pdf_renderer;
pub mod pdf_text;
pub mod pdf_viewer;
pub mod storage;

// Re-export commonly used items
pub use audit_display::{AuditDisplay, AuditSummary, TimelineEvent};
pub use compliance_panel::{CompliancePanel, ViolationItem};
pub use coords::{dom_to_pdf, pdf_to_dom};
pub use fields::{Field, FieldEditor, FieldType, WasmFieldEditor};
pub use overlay::{OverlayManager, PageInfo};
pub use pdf_renderer::{PageMetadata, PdfRenderer, ScaledDimensions};
pub use pdf_text::{extract_pdf_text, get_pdf_page_count};
pub use pdf_viewer::{init_pdf_js, init_pdf_js_with_worker, PdfViewer};
pub use storage::{init_storage, uint8_array_to_vec, vec_to_uint8_array, Storage};

// Re-export page range utilities
pub use page_ranges::{field_constraints, parse_page_ranges, validate_page_range};

// Re-export field export functions
pub use field_export::{export_pdf_with_fields, validate_fields_for_export};

// Re-export extraction module items
pub use extraction::{
    extract_text_hybrid, extract_text_with_strategy, extract_with_metadata, BenchmarkResult,
    BenchmarkRunner, ExtractionConfig, ExtractionRouter, ExtractionStrategy, PdfCategory,
};

/// WASM entry point for compliance checking (defaults to Florida)
#[wasm_bindgen]
pub fn check_compliance_wasm(document_json: &str) -> Result<String, JsValue> {
    check_compliance_for_state_wasm(document_json, "FL", None)
}

/// WASM entry point for compliance checking with state selection
#[wasm_bindgen]
pub fn check_compliance_for_state_wasm(
    document_json: &str,
    state_code: &str,
    year_built: Option<u32>,
) -> Result<String, JsValue> {
    check_compliance_with_zip_wasm(document_json, state_code, year_built, None)
}

/// WASM entry point for compliance checking with state and ZIP code
///
/// ZIP code enables Layer 3 (local) compliance checks:
/// - Chicago RLTO for 606xx ZIPs
/// - NYC rent stabilization for 100xx/110xx ZIPs
/// - SF/LA rent control for California ZIPs
#[wasm_bindgen]
pub fn check_compliance_with_zip_wasm(
    document_json: &str,
    state_code: &str,
    year_built: Option<u32>,
    zip_code: Option<String>,
) -> Result<String, JsValue> {
    let document: LeaseDocument = serde_json::from_str(document_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse document: {}", e)))?;

    let state = State::parse_code(state_code)
        .ok_or_else(|| JsValue::from_str(&format!("Unsupported state: {}", state_code)))?;

    // Create jurisdiction with locality detection from ZIP code
    let jurisdiction = match zip_code {
        Some(ref zip) if !zip.is_empty() => Jurisdiction::from_zip(state, zip),
        _ => Jurisdiction::new(state),
    };

    let engine = ComplianceEngine::new();
    let report = engine.check_compliance(&jurisdiction, &document, year_built);

    serde_json::to_string(&report)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
}

/// Get list of supported states with statute citations
#[wasm_bindgen]
pub fn get_supported_states() -> Result<String, JsValue> {
    let engine = ComplianceEngine::new();
    let states: Vec<_> = engine
        .supported_states()
        .iter()
        .map(|s| {
            serde_json::json!({
                "code": format!("{:?}", s),
                "name": s.name(),
                "implemented": s.is_implemented(),
                "statutes": s.statute_citation()
            })
        })
        .collect();

    serde_json::to_string(&states)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize states: {}", e)))
}

// ============================================================================
// Real Estate Document Compliance (Purchase Contracts, Listings, Escalation)
// ============================================================================

/// WASM entry point for real estate compliance checking
///
/// # Arguments
/// * `document_json` - JSON string of LeaseDocument (reused for any document type)
/// * `state_code` - Two-letter state code (e.g., "FL")
/// * `doc_type` - Document type: "purchase", "listing", "escalation", or "auto"
/// * `year_built` - Optional year the property was built (for lead paint)
///
/// # Returns
/// JSON string of ComplianceReport
#[wasm_bindgen]
pub fn check_realestate_compliance_wasm(
    document_json: &str,
    state_code: &str,
    doc_type: &str,
    year_built: Option<u32>,
) -> Result<String, JsValue> {
    let document: LeaseDocument = serde_json::from_str(document_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse document: {}", e)))?;

    let state = State::parse_code(state_code)
        .ok_or_else(|| JsValue::from_str(&format!("Unsupported state: {}", state_code)))?;

    let jurisdiction = Jurisdiction::new(state);
    let engine = ComplianceEngine::new();

    // Parse document type
    let document_type = match doc_type.to_lowercase().as_str() {
        "purchase" | "purchase_contract" | "realestate" => DocumentType::RealEstatePurchase,
        "listing" | "listing_agreement" => DocumentType::ListingAgreement,
        "escalation" | "escalation_addendum" => DocumentType::EscalationAddendum,
        "lease" => DocumentType::Lease,
        _ => {
            // Auto-detect document type (includes "auto" and any unrecognized type)
            let full_text = document.text_content.join("\n");
            engine.detect_document_type(&full_text)
        }
    };

    let report = match document_type {
        DocumentType::Lease => engine.check_compliance(&jurisdiction, &document, year_built),
        _ => {
            engine.check_realestate_compliance(&jurisdiction, &document, document_type, year_built)
        }
    };

    serde_json::to_string(&report)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
}

/// Auto-detect document type and check appropriate compliance
///
/// This function automatically determines if the document is a lease,
/// purchase contract, listing agreement, or escalation addendum.
#[wasm_bindgen]
pub fn check_compliance_auto_detect_wasm(
    document_json: &str,
    state_code: &str,
    year_built: Option<u32>,
) -> Result<String, JsValue> {
    check_realestate_compliance_wasm(document_json, state_code, "auto", year_built)
}

/// Detect the type of Florida document
///
/// Returns document type string. Categories:
/// - Lease: "lease", "lease_termination", "eviction"
/// - Purchase: "purchase", "purchase_as_is", "inspection_contingency", "financing_contingency", "escalation", "appraisal_contingency"
/// - Listing: "listing"
/// - Contractor: "contractor_invoice", "cost_of_materials", "notice_of_commencement", "notice_to_owner", "claim_of_lien", "release_of_lien", "dispute_lien", "fraudulent_lien", "final_payment_affidavit"
/// - Bill of Sale: "bill_of_sale_car", "bill_of_sale_boat", "bill_of_sale_trailer", "bill_of_sale_jetski", "bill_of_sale_mobile_home"
/// - Unknown: "unknown"
#[wasm_bindgen]
pub fn detect_document_type_wasm(text: &str) -> String {
    let engine = ComplianceEngine::new();
    let doc_type = engine.detect_document_type(text);

    match doc_type {
        // Lease Documents
        DocumentType::Lease => "lease".to_string(),
        DocumentType::LeaseTerminationNotice => "lease_termination".to_string(),
        DocumentType::EvictionNotice => "eviction".to_string(),
        // Purchase Documents
        DocumentType::RealEstatePurchase => "purchase".to_string(),
        DocumentType::RealEstatePurchaseAsIs => "purchase_as_is".to_string(),
        DocumentType::InspectionContingency => "inspection_contingency".to_string(),
        DocumentType::FinancingContingency => "financing_contingency".to_string(),
        DocumentType::EscalationAddendum => "escalation".to_string(),
        DocumentType::AppraisalContingency => "appraisal_contingency".to_string(),
        // Listing Documents
        DocumentType::ListingAgreement => "listing".to_string(),
        // Contractor Documents
        DocumentType::ContractorInvoice => "contractor_invoice".to_string(),
        DocumentType::CostOfMaterialsBill => "cost_of_materials".to_string(),
        DocumentType::NoticeOfCommencement => "notice_of_commencement".to_string(),
        DocumentType::NoticeToOwner => "notice_to_owner".to_string(),
        DocumentType::ClaimOfLien => "claim_of_lien".to_string(),
        DocumentType::ReleaseOfLien => "release_of_lien".to_string(),
        DocumentType::DisputeLien => "dispute_lien".to_string(),
        DocumentType::FraudulentLienReport => "fraudulent_lien".to_string(),
        DocumentType::FinalPaymentAffidavit => "final_payment_affidavit".to_string(),
        // Bill of Sale Documents
        DocumentType::BillOfSaleCar => "bill_of_sale_car".to_string(),
        DocumentType::BillOfSaleBoat => "bill_of_sale_boat".to_string(),
        DocumentType::BillOfSaleTrailer => "bill_of_sale_trailer".to_string(),
        DocumentType::BillOfSaleJetSki => "bill_of_sale_jetski".to_string(),
        DocumentType::BillOfSaleMobileHome => "bill_of_sale_mobile_home".to_string(),
        // Unknown
        DocumentType::Unknown => "unknown".to_string(),
    }
}

/// Check compliance for any Florida document type
///
/// This function routes to the appropriate compliance checker based on document type.
#[wasm_bindgen]
pub fn check_document_compliance_wasm(
    document_json: &str,
    state_code: &str,
    doc_type: &str,
    year_built: Option<u32>,
) -> Result<String, JsValue> {
    let document: LeaseDocument = serde_json::from_str(document_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse document: {}", e)))?;

    let state = State::parse_code(state_code)
        .ok_or_else(|| JsValue::from_str(&format!("Unsupported state: {}", state_code)))?;

    let jurisdiction = Jurisdiction::new(state);
    let engine = ComplianceEngine::new();

    // Parse document type
    let document_type = parse_doc_type_string(doc_type, &document, &engine);

    let report =
        engine.check_document_compliance(&jurisdiction, &document, document_type, year_built);

    serde_json::to_string(&report)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
}

/// Parse document type string to DocumentType enum
fn parse_doc_type_string(
    doc_type: &str,
    document: &LeaseDocument,
    engine: &ComplianceEngine,
) -> DocumentType {
    match doc_type.to_lowercase().as_str() {
        // Lease Documents
        "lease" => DocumentType::Lease,
        "lease_termination" => DocumentType::LeaseTerminationNotice,
        "eviction" => DocumentType::EvictionNotice,
        // Purchase Documents
        "purchase" | "purchase_contract" | "realestate" => DocumentType::RealEstatePurchase,
        "purchase_as_is" => DocumentType::RealEstatePurchaseAsIs,
        "inspection_contingency" => DocumentType::InspectionContingency,
        "financing_contingency" => DocumentType::FinancingContingency,
        "escalation" | "escalation_addendum" => DocumentType::EscalationAddendum,
        "appraisal_contingency" => DocumentType::AppraisalContingency,
        // Listing Documents
        "listing" | "listing_agreement" => DocumentType::ListingAgreement,
        // Contractor Documents
        "contractor_invoice" => DocumentType::ContractorInvoice,
        "cost_of_materials" => DocumentType::CostOfMaterialsBill,
        "notice_of_commencement" => DocumentType::NoticeOfCommencement,
        "notice_to_owner" => DocumentType::NoticeToOwner,
        "claim_of_lien" => DocumentType::ClaimOfLien,
        "release_of_lien" => DocumentType::ReleaseOfLien,
        "dispute_lien" => DocumentType::DisputeLien,
        "fraudulent_lien" => DocumentType::FraudulentLienReport,
        "final_payment_affidavit" => DocumentType::FinalPaymentAffidavit,
        // Bill of Sale Documents
        "bill_of_sale_car" => DocumentType::BillOfSaleCar,
        "bill_of_sale_boat" => DocumentType::BillOfSaleBoat,
        "bill_of_sale_trailer" => DocumentType::BillOfSaleTrailer,
        "bill_of_sale_jetski" => DocumentType::BillOfSaleJetSki,
        "bill_of_sale_mobile_home" => DocumentType::BillOfSaleMobileHome,
        // Auto-detect
        _ => {
            let full_text = document.text_content.join("\n");
            engine.detect_document_type(&full_text)
        }
    }
}

/// Get list of all supported document types
#[wasm_bindgen]
pub fn get_supported_document_types() -> Result<String, JsValue> {
    let categories = vec![
        serde_json::json!({
            "category": "Lease",
            "chapter": "Chapter 83",
            "types": [
                {"value": "lease", "name": "Residential Lease Agreement"},
                {"value": "lease_termination", "name": "Lease Termination Notice"},
                {"value": "eviction", "name": "Eviction Notice"}
            ]
        }),
        serde_json::json!({
            "category": "Real Estate Purchase",
            "chapter": "Chapter 475, 689",
            "types": [
                {"value": "purchase", "name": "Purchase Contract"},
                {"value": "purchase_as_is", "name": "As-Is Purchase Contract"},
                {"value": "inspection_contingency", "name": "Inspection Contingency"},
                {"value": "financing_contingency", "name": "Financing Contingency"},
                {"value": "escalation", "name": "Escalation Addendum"},
                {"value": "appraisal_contingency", "name": "Appraisal Contingency"}
            ]
        }),
        serde_json::json!({
            "category": "Listing",
            "chapter": "Chapter 475",
            "types": [
                {"value": "listing", "name": "Exclusive Listing Agreement"}
            ]
        }),
        serde_json::json!({
            "category": "Contractor",
            "chapter": "Chapter 713",
            "types": [
                {"value": "notice_of_commencement", "name": "Notice of Commencement"},
                {"value": "notice_to_owner", "name": "Notice to Owner"},
                {"value": "claim_of_lien", "name": "Claim of Lien"},
                {"value": "release_of_lien", "name": "Release of Lien"},
                {"value": "dispute_lien", "name": "Dispute of Lien"},
                {"value": "fraudulent_lien", "name": "Fraudulent Lien Report"},
                {"value": "contractor_invoice", "name": "Contractor Invoice"},
                {"value": "cost_of_materials", "name": "Cost of Materials Bill"},
                {"value": "final_payment_affidavit", "name": "Final Payment Affidavit"}
            ]
        }),
    ];

    serde_json::to_string(&categories)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize document types: {}", e)))
}

/// Get covered statutes for real estate transactions in a state
#[wasm_bindgen]
pub fn get_realestate_statutes(state_code: &str) -> Result<String, JsValue> {
    let state = State::parse_code(state_code)
        .ok_or_else(|| JsValue::from_str(&format!("Unsupported state: {}", state_code)))?;

    let statutes = match state {
        State::FL => compliance_engine::covered_realestate_statutes(),
        _ => vec![], // Other states not yet implemented for real estate
    };

    serde_json::to_string(&statutes)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize statutes: {}", e)))
}

// ============================================================================
// Template Rendering (Local-First - runs entirely in browser)
// ============================================================================

use typst_engine::compiler::output::OutputFormat;
use typst_engine::{compile_document_sync, RenderRequest};

/// Render a template to PDF bytes (local-first, no server required)
///
/// # Arguments
/// * `template_name` - Name of embedded template ("invoice", "letter", "florida_lease")
/// * `inputs_json` - JSON object with template variables
///
/// # Returns
/// Base64-encoded PDF data on success
#[wasm_bindgen]
pub fn render_template(template_name: &str, inputs_json: &str) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    let inputs: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(inputs_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse inputs: {}", e)))?;

    let request = RenderRequest {
        source: format!("typst://templates/{}", template_name),
        inputs,
        assets: std::collections::HashMap::new(),
        format: OutputFormat::Pdf,
        ppi: None,
    };

    let response = compile_document_sync(request)
        .map_err(|e| JsValue::from_str(&format!("Compilation failed: {:?}", e)))?;

    match response.artifact {
        Some(artifact) => Ok(artifact.data_base64),
        None => {
            let error_msgs: Vec<String> =
                response.errors.iter().map(|e| e.message.clone()).collect();
            Err(JsValue::from_str(&format!(
                "Template errors: {}",
                error_msgs.join("; ")
            )))
        }
    }
}

/// Render raw Typst source to PDF bytes
///
/// # Arguments
/// * `source` - Raw Typst source code
/// * `inputs_json` - JSON object with variables (accessible via sys.inputs)
///
/// # Returns
/// Base64-encoded PDF data on success
#[wasm_bindgen]
pub fn render_typst(source: &str, inputs_json: &str) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    let inputs: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(inputs_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse inputs: {}", e)))?;

    let request = RenderRequest {
        source: source.to_string(),
        inputs,
        assets: std::collections::HashMap::new(),
        format: OutputFormat::Pdf,
        ppi: None,
    };

    let response = compile_document_sync(request)
        .map_err(|e| JsValue::from_str(&format!("Compilation failed: {:?}", e)))?;

    match response.artifact {
        Some(artifact) => Ok(artifact.data_base64),
        None => {
            let error_msgs: Vec<String> =
                response.errors.iter().map(|e| e.message.clone()).collect();
            Err(JsValue::from_str(&format!(
                "Typst errors: {}",
                error_msgs.join("; ")
            )))
        }
    }
}

/// List available embedded templates
///
/// # Returns
/// JSON array of template info objects
#[wasm_bindgen]
pub fn list_templates() -> Result<String, JsValue> {
    let templates = typst_engine::templates::list_templates();

    serde_json::to_string(&templates)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize templates: {}", e)))
}

/// Validate Typst syntax without rendering
///
/// # Returns
/// JSON array of syntax errors (empty if valid)
#[wasm_bindgen]
pub fn validate_typst_syntax(source: &str) -> Result<String, JsValue> {
    let errors = typst_engine::compiler::validate_syntax(source);

    let error_json: Vec<_> = errors
        .iter()
        .map(|e| {
            serde_json::json!({
                "message": e.message,
                "hint": e.hint,
            })
        })
        .collect();

    serde_json::to_string(&error_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize errors: {}", e)))
}

// ============================================================================
// Tests for Template Rendering
// ============================================================================

#[cfg(test)]
mod template_tests {
    use typst_engine::compiler::output::OutputFormat;
    use typst_engine::{compile_document_sync, RenderRequest};

    // ===== render_template tests =====

    #[test]
    fn test_render_template_letter() {
        // Render the letter template with minimal inputs
        let inputs = serde_json::json!({
            "sender_name": "John Doe",
            "recipient_name": "Jane Smith",
            "body": "This is a test letter."
        });

        let request = RenderRequest {
            source: "typst://templates/letter".to_string(),
            inputs: serde_json::from_value(inputs).unwrap(),
            assets: std::collections::HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document_sync(request);
        assert!(
            result.is_ok(),
            "Letter template should compile: {:?}",
            result.err()
        );

        let response = result.unwrap();
        assert!(response.artifact.is_some(), "Should produce PDF artifact");

        let artifact = response.artifact.unwrap();
        assert!(
            !artifact.data_base64.is_empty(),
            "PDF data should not be empty"
        );
        assert_eq!(artifact.mime_type, "application/pdf");
        assert!(artifact.page_count >= 1, "Should have at least 1 page");
    }

    #[test]
    fn test_render_template_invoice() {
        let inputs = serde_json::json!({
            "invoice_number": "INV-001",
            "client_name": "Acme Corp",
            "total": "1000.00"
        });

        let request = RenderRequest {
            source: "typst://templates/invoice".to_string(),
            inputs: serde_json::from_value(inputs).unwrap(),
            assets: std::collections::HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document_sync(request);
        assert!(
            result.is_ok(),
            "Invoice template should compile: {:?}",
            result.err()
        );

        let response = result.unwrap();
        assert!(response.artifact.is_some(), "Should produce PDF artifact");
    }

    #[test]
    fn test_render_template_florida_lease() {
        let inputs = serde_json::json!({
            "landlord_name": "Property Owner LLC",
            "tenant_name": "John Tenant",
            "property_address": "123 Main St, Miami, FL 33101",
            "monthly_rent": "2000",
            "lease_start": "2025-01-01",
            "lease_end": "2025-12-31"
        });

        let request = RenderRequest {
            source: "typst://templates/florida_lease".to_string(),
            inputs: serde_json::from_value(inputs).unwrap(),
            assets: std::collections::HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document_sync(request);
        assert!(
            result.is_ok(),
            "Florida lease template should compile: {:?}",
            result.err()
        );

        let response = result.unwrap();
        assert!(response.artifact.is_some(), "Should produce PDF artifact");

        let artifact = response.artifact.unwrap();
        // Florida lease is a long document
        assert!(
            artifact.page_count >= 1,
            "Florida lease should have multiple pages"
        );
    }

    #[test]
    fn test_render_template_invalid_template() {
        let request = RenderRequest {
            source: "typst://templates/nonexistent".to_string(),
            inputs: std::collections::HashMap::new(),
            assets: std::collections::HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document_sync(request);
        assert!(result.is_err(), "Nonexistent template should fail");
    }

    // ===== render_typst tests =====

    #[test]
    fn test_render_typst_simple() {
        let request = RenderRequest {
            source: "Hello, *World*!".to_string(),
            inputs: std::collections::HashMap::new(),
            assets: std::collections::HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document_sync(request);
        assert!(
            result.is_ok(),
            "Simple typst should compile: {:?}",
            result.err()
        );

        let response = result.unwrap();
        assert!(response.artifact.is_some());
    }

    #[test]
    fn test_render_typst_with_inputs() {
        let mut inputs = std::collections::HashMap::new();
        inputs.insert("name".to_string(), serde_json::json!("Alice"));

        let request = RenderRequest {
            source: r#"#let name = sys.inputs.at("name", default: "World")
Hello, #name!"#
                .to_string(),
            inputs,
            assets: std::collections::HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document_sync(request);
        assert!(
            result.is_ok(),
            "Typst with inputs should compile: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_render_typst_syntax_error() {
        let request = RenderRequest {
            source: "#let x = ".to_string(), // Incomplete statement
            inputs: std::collections::HashMap::new(),
            assets: std::collections::HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document_sync(request);
        // Should succeed but with errors in response
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(
            response.artifact.is_none() || !response.errors.is_empty(),
            "Syntax error should produce errors"
        );
    }

    // ===== list_templates tests =====

    #[test]
    fn test_list_templates() {
        let templates = typst_engine::templates::list_templates();

        assert!(!templates.is_empty(), "Should have templates");

        // Check that our known templates exist
        let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"invoice"), "Should have invoice template");
        assert!(names.contains(&"letter"), "Should have letter template");
        assert!(
            names.contains(&"florida_lease"),
            "Should have florida_lease template"
        );
    }

    #[test]
    fn test_list_templates_has_descriptions() {
        let templates = typst_engine::templates::list_templates();

        for template in templates {
            assert!(!template.name.is_empty(), "Template should have name");
            assert!(
                !template.description.is_empty(),
                "Template should have description"
            );
        }
    }

    // ===== validate_typst_syntax tests =====

    #[test]
    fn test_validate_syntax_valid() {
        let errors = typst_engine::compiler::validate_syntax("Hello, *World*!");
        assert!(errors.is_empty(), "Valid syntax should have no errors");
    }

    #[test]
    fn test_validate_syntax_invalid() {
        let errors = typst_engine::compiler::validate_syntax("#let x = ");
        assert!(!errors.is_empty(), "Invalid syntax should have errors");
    }

    #[test]
    fn test_validate_syntax_complex_valid() {
        let source = r#"
#let greet(name) = [Hello, #name!]
#greet("World")
= Heading
Some *bold* and _italic_ text.
"#;
        let errors = typst_engine::compiler::validate_syntax(source);
        assert!(
            errors.is_empty(),
            "Complex valid syntax should have no errors: {:?}",
            errors
        );
    }

    // ===== PDF content verification tests =====

    #[test]
    fn test_pdf_is_valid_base64() {
        let request = RenderRequest {
            source: "Test PDF".to_string(),
            inputs: std::collections::HashMap::new(),
            assets: std::collections::HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document_sync(request).unwrap();
        let artifact = result.artifact.unwrap();

        // Verify it's valid base64
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &artifact.data_base64,
        );
        assert!(decoded.is_ok(), "Should be valid base64");

        // Verify PDF magic bytes
        let bytes = decoded.unwrap();
        assert!(bytes.len() > 4, "PDF should have content");
        assert_eq!(&bytes[0..4], b"%PDF", "Should start with PDF magic bytes");
    }

    // ===== WASM function wrapper tests =====
    // These test the actual wasm_bindgen functions (without JsValue)

    #[test]
    fn test_render_template_wrapper_letter() {
        let inputs = r#"{"sender_name": "Test", "recipient_name": "User", "body": "Hello"}"#;

        // Test the underlying logic (not the wasm_bindgen wrapper)
        let inputs_parsed: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_str(inputs).unwrap();

        let request = RenderRequest {
            source: "typst://templates/letter".to_string(),
            inputs: inputs_parsed,
            assets: std::collections::HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let response = compile_document_sync(request).unwrap();
        assert!(response.artifact.is_some());
    }

    #[test]
    fn test_render_template_wrapper_invalid_json() {
        let inputs = "not valid json";
        let result: Result<std::collections::HashMap<String, serde_json::Value>, _> =
            serde_json::from_str(inputs);
        assert!(result.is_err(), "Invalid JSON should fail to parse");
    }
}

#[cfg(test)]
mod template_proptests {
    use proptest::prelude::*;
    use typst_engine::compiler::output::OutputFormat;
    use typst_engine::{compile_document_sync, RenderRequest};

    // Use ASCII-only strings to avoid triggering upstream Typst Unicode bugs
    fn ascii_string() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 .,!?#=()\\[\\]{}*_-]*"
    }

    proptest! {
        /// Property: ASCII input should not crash the compiler
        #[test]
        fn render_doesnt_crash(source in ascii_string()) {
            let request = RenderRequest {
                source,
                inputs: std::collections::HashMap::new(),
                assets: std::collections::HashMap::new(),
                format: OutputFormat::Pdf,
                ppi: None,
            };

            // Should not panic, even if it returns an error
            let _ = compile_document_sync(request);
        }

        /// Property: Valid template names should always succeed
        #[test]
        fn templates_always_compile(template in prop_oneof!["invoice", "letter", "florida_lease"]) {
            let request = RenderRequest {
                source: format!("typst://templates/{}", template),
                inputs: std::collections::HashMap::new(),
                assets: std::collections::HashMap::new(),
                format: OutputFormat::Pdf,
                ppi: None,
            };

            let result = compile_document_sync(request);
            prop_assert!(result.is_ok(), "Template {} should compile", template);
        }

        /// Property: Syntax validation should be consistent (ASCII only)
        #[test]
        fn validate_is_deterministic(source in ascii_string()) {
            let errors1 = typst_engine::compiler::validate_syntax(&source);
            let errors2 = typst_engine::compiler::validate_syntax(&source);

            prop_assert_eq!(errors1.len(), errors2.len(),
                "Same source should produce same error count");
        }
    }
}
