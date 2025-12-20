//! PDF.js integration for rendering PDFs in the browser via WASM

use js_sys::{Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

// External JavaScript functions from pdf-bridge.js
#[wasm_bindgen(module = "/www/js/pdf-bridge.js")]
extern "C" {
    #[wasm_bindgen(js_name = initPdfJs)]
    pub async fn init_pdf_js_internal(worker_src: &str) -> JsValue;

    #[wasm_bindgen(js_name = loadDocument)]
    pub async fn load_document_internal(data: Uint8Array) -> JsValue;

    #[wasm_bindgen(js_name = renderPage)]
    pub async fn render_page_internal(
        page_num: u32,
        canvas: &HtmlCanvasElement,
        scale: f64,
    ) -> JsValue;

    #[wasm_bindgen(js_name = getPageDimensions)]
    pub async fn get_page_dimensions_internal(page_num: u32) -> JsValue;
}

/// PdfViewer wraps pdf.js interaction for rendering PDFs in the browser
#[wasm_bindgen]
pub struct PdfViewer {
    document_proxy: Option<JsValue>,
    page_count: u32,
}

#[wasm_bindgen]
impl PdfViewer {
    /// Create a new PdfViewer instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self {
            document_proxy: None,
            page_count: 0,
        }
    }

    /// Load a PDF document from bytes
    #[wasm_bindgen]
    pub async fn load(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        // Convert Rust bytes to Uint8Array
        let uint8_array = Uint8Array::new_with_length(bytes.len() as u32);
        uint8_array.copy_from(bytes);

        // Load the document via JavaScript bridge
        let doc_result = load_document_internal(uint8_array).await;

        // Check if the result is an error
        if doc_result.is_undefined() || doc_result.is_null() {
            return Err(JsValue::from_str("Failed to load PDF document"));
        }

        // Extract numPages from the result object
        if let Ok(num_pages) = Reflect::get(&doc_result, &JsValue::from_str("numPages")) {
            if let Some(count) = num_pages.as_f64() {
                self.page_count = count as u32;
            }
        }

        // Store the document proxy
        self.document_proxy = Some(doc_result);

        Ok(())
    }

    /// Get the number of pages in the loaded document
    #[wasm_bindgen]
    pub fn page_count(&self) -> u32 {
        self.page_count
    }

    /// Render a specific page to a canvas element
    #[wasm_bindgen]
    pub async fn render_page(
        &self,
        page_num: u32,
        canvas: HtmlCanvasElement,
    ) -> Result<(), JsValue> {
        if self.document_proxy.is_none() {
            return Err(JsValue::from_str("No document loaded"));
        }

        if page_num < 1 || page_num > self.page_count {
            return Err(JsValue::from_str(&format!(
                "Invalid page number: {} (document has {} pages)",
                page_num, self.page_count
            )));
        }

        // Render the page with default scale of 1.0
        render_page_internal(page_num, &canvas, 1.0).await;

        Ok(())
    }

    /// Render a page with custom scale
    #[wasm_bindgen]
    pub async fn render_page_with_scale(
        &self,
        page_num: u32,
        canvas: HtmlCanvasElement,
        scale: f64,
    ) -> Result<(), JsValue> {
        if self.document_proxy.is_none() {
            return Err(JsValue::from_str("No document loaded"));
        }

        if page_num < 1 || page_num > self.page_count {
            return Err(JsValue::from_str(&format!(
                "Invalid page number: {} (document has {} pages)",
                page_num, self.page_count
            )));
        }

        // Render the page with specified scale
        render_page_internal(page_num, &canvas, scale).await;

        Ok(())
    }

    /// Get page dimensions (width, height) for a specific page
    #[wasm_bindgen]
    pub async fn get_page_dimensions(&self, page_num: u32) -> Result<JsValue, JsValue> {
        if self.document_proxy.is_none() {
            return Err(JsValue::from_str("No document loaded"));
        }

        if page_num < 1 || page_num > self.page_count {
            return Err(JsValue::from_str(&format!(
                "Invalid page number: {} (document has {} pages)",
                page_num, self.page_count
            )));
        }

        // Get dimensions from JavaScript bridge
        let dimensions = get_page_dimensions_internal(page_num).await;

        Ok(dimensions)
    }

    /// Check if a document is currently loaded
    #[wasm_bindgen]
    pub fn is_loaded(&self) -> bool {
        self.document_proxy.is_some() && self.page_count > 0
    }
}

impl Default for PdfViewer {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize PDF.js library with default worker
/// Must be called before creating PdfViewer instances
#[wasm_bindgen]
pub async fn init_pdf_js() -> Result<(), JsValue> {
    init_pdf_js_internal(
        "https://cdn.jsdelivr.net/npm/pdfjs-dist@3.11.174/build/pdf.worker.min.js",
    )
    .await;
    Ok(())
}

/// Initialize PDF.js library with custom worker URL
#[wasm_bindgen]
pub async fn init_pdf_js_with_worker(worker_src: &str) -> Result<(), JsValue> {
    init_pdf_js_internal(worker_src).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_viewer_creation() {
        // Test PdfViewer struct can be created
        let viewer = PdfViewer::new();
        assert_eq!(viewer.page_count(), 0);
        assert!(viewer.document_proxy.is_none());
        assert!(!viewer.is_loaded());
    }

    #[test]
    fn test_pdf_page_count() {
        // Test page count extraction
        let viewer = PdfViewer::new();
        assert_eq!(viewer.page_count(), 0);

        // After loading, page count should be updated
        // For now, we test the initial state
    }

    #[test]
    fn test_pdf_viewer_default() {
        // Test that Default trait works
        let viewer = PdfViewer::default();
        assert_eq!(viewer.page_count(), 0);
        assert!(!viewer.is_loaded());
    }

    #[test]
    fn test_is_loaded_initial_state() {
        // Test that a newly created viewer is not loaded
        let viewer = PdfViewer::new();
        assert!(!viewer.is_loaded());
    }

    #[test]
    fn test_page_count_bounds() {
        // Test that page count is properly initialized to 0
        let viewer = PdfViewer::new();
        let count = viewer.page_count();
        assert_eq!(count, 0);
    }
}
