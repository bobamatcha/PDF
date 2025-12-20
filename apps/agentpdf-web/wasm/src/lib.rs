use compliance_engine::ComplianceEngine;
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

/// WASM entry point for compliance checking
#[wasm_bindgen]
pub fn check_compliance_wasm(document_json: &str) -> Result<String, JsValue> {
    let document: LeaseDocument = serde_json::from_str(document_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse document: {}", e)))?;

    let engine = ComplianceEngine::new();
    let report = engine.check_compliance(&document);

    serde_json::to_string(&report)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
}
