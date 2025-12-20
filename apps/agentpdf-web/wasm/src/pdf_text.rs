//! PDF text extraction using pdf-extract
//! Handles CID fonts and ToUnicode CMaps properly

use wasm_bindgen::prelude::*;

/// Extract text from all pages of a PDF
#[wasm_bindgen]
pub fn extract_pdf_text(data: &[u8]) -> Result<String, JsValue> {
    pdf_extract::extract_text_from_mem(data)
        .map_err(|e| JsValue::from_str(&format!("PDF text extraction failed: {}", e)))
}

/// Get the number of pages in a PDF
#[wasm_bindgen]
pub fn get_pdf_page_count(data: &[u8]) -> Result<u32, JsValue> {
    // pdf-extract re-exports Document from lopdf
    let doc = pdf_extract::Document::load_mem(data)
        .map_err(|e| JsValue::from_str(&format!("Failed to load PDF: {}", e)))?;
    Ok(doc.get_pages().len() as u32)
}
