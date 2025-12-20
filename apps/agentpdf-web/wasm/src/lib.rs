use compliance_engine::{ComplianceEngine, Jurisdiction, State};
use shared_types::LeaseDocument;
use wasm_bindgen::prelude::*;

// Export modules
pub mod audit_display;
pub mod compliance_panel;
pub mod coords;
pub mod extraction;
pub mod fields;
pub mod overlay;
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
