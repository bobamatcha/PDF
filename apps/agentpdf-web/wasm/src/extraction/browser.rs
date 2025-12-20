//! Browser-based extraction using pdf.js via JavaScript bridge
//!
//! This backend leverages the browser's pdf.js library for robust
//! text extraction, especially for PDFs with encoding issues.

use super::types::*;
#[cfg(not(target_arch = "wasm32"))]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);
}

/// Browser backend using pdf.js
#[allow(dead_code)]
pub struct BrowserExtractor {
    /// Whether to use zero-copy memory transfer (for future optimization)
    use_zero_copy: bool,
}

impl BrowserExtractor {
    pub fn new() -> Self {
        Self {
            use_zero_copy: true,
        }
    }

    /// Check if pdf.js is available in the browser environment
    pub fn is_available() -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let window = match web_sys::window() {
                Some(w) => w,
                None => return false,
            };

            // Check for pdfjsLib global
            if let Ok(val) = Reflect::get(&window, &JsValue::from_str("pdfjsLib")) {
                return !val.is_undefined();
            }
            false
        }

        #[cfg(not(target_arch = "wasm32"))]
        false
    }

    /// Create a JavaScript Uint8Array view of the data (zero-copy when possible)
    pub fn create_js_array(data: &[u8], zero_copy: bool) -> Uint8Array {
        if zero_copy {
            // SAFETY: The view is only valid while `data` is alive
            // This avoids copying but requires careful lifetime management
            unsafe { Uint8Array::view(data) }
        } else {
            // Safe copy - doubles memory but no lifetime issues
            Uint8Array::from(data)
        }
    }

    /// Extract text using pdf.js via JavaScript interop
    #[cfg(target_arch = "wasm32")]
    pub async fn extract_with_pdfjs(
        &self,
        data: &[u8],
    ) -> Result<Vec<PageContent>, ExtractionError> {
        use web_sys::window;

        let window = window()
            .ok_or_else(|| ExtractionError::JsBridgeError("No window object".to_string()))?;

        // Get pdfjsLib
        let pdfjs = Reflect::get(&window, &JsValue::from_str("pdfjsLib"))
            .map_err(|_| ExtractionError::JsBridgeError("pdfjsLib not found".to_string()))?;

        if pdfjs.is_undefined() {
            return Err(ExtractionError::BackendUnavailable(
                "pdf.js not loaded".to_string(),
            ));
        }

        // Create typed array from data (prefer copy for safety in async context)
        let typed_array = Self::create_js_array(data, false);

        // Call pdfjsLib.getDocument
        let get_document =
            Reflect::get(&pdfjs, &JsValue::from_str("getDocument")).map_err(|e| {
                ExtractionError::JsBridgeError(format!("getDocument not found: {:?}", e))
            })?;

        let get_document_fn = get_document.dyn_ref::<js_sys::Function>().ok_or_else(|| {
            ExtractionError::JsBridgeError("getDocument is not a function".to_string())
        })?;

        // Create options object
        let options = Object::new();
        Reflect::set(&options, &JsValue::from_str("data"), &typed_array)
            .map_err(|e| ExtractionError::JsBridgeError(format!("Failed to set data: {:?}", e)))?;

        // Call getDocument
        let loading_task = get_document_fn
            .call1(&pdfjs, &options)
            .map_err(|e| ExtractionError::JsBridgeError(format!("getDocument failed: {:?}", e)))?;

        // Get promise property
        let promise = Reflect::get(&loading_task, &JsValue::from_str("promise"))
            .map_err(|e| ExtractionError::JsBridgeError(format!("No promise: {:?}", e)))?;

        let promise: Promise = promise
            .dyn_into()
            .map_err(|_| ExtractionError::JsBridgeError("Not a promise".to_string()))?;

        // Await the document
        let pdf_doc = JsFuture::from(promise).await.map_err(|e| {
            ExtractionError::JsBridgeError(format!("Document load failed: {:?}", e))
        })?;

        // Get page count
        let num_pages = Reflect::get(&pdf_doc, &JsValue::from_str("numPages"))
            .map_err(|e| ExtractionError::JsBridgeError(format!("numPages failed: {:?}", e)))?
            .as_f64()
            .unwrap_or(0.0) as u32;

        let mut pages = Vec::new();

        // Extract text from each page
        for page_num in 1..=num_pages {
            let page_content = self.extract_page(&pdf_doc, page_num).await?;
            pages.push(page_content);
        }

        Ok(pages)
    }

    #[cfg(target_arch = "wasm32")]
    async fn extract_page(
        &self,
        pdf_doc: &JsValue,
        page_num: u32,
    ) -> Result<PageContent, ExtractionError> {
        // Get page
        let get_page = Reflect::get(pdf_doc, &JsValue::from_str("getPage"))
            .map_err(|e| ExtractionError::JsBridgeError(format!("getPage not found: {:?}", e)))?;

        let get_page_fn = get_page.dyn_ref::<js_sys::Function>().ok_or_else(|| {
            ExtractionError::JsBridgeError("getPage is not a function".to_string())
        })?;

        let page_promise = get_page_fn
            .call1(pdf_doc, &JsValue::from_f64(page_num as f64))
            .map_err(|e| ExtractionError::JsBridgeError(format!("getPage call failed: {:?}", e)))?;

        let page_promise: Promise = page_promise
            .dyn_into()
            .map_err(|_| ExtractionError::JsBridgeError("Not a promise".to_string()))?;

        let page = JsFuture::from(page_promise)
            .await
            .map_err(|e| ExtractionError::JsBridgeError(format!("Page load failed: {:?}", e)))?;

        // Get viewport for dimensions
        let get_viewport = Reflect::get(&page, &JsValue::from_str("getViewport"))
            .map_err(|e| ExtractionError::JsBridgeError(format!("getViewport failed: {:?}", e)))?;

        let get_viewport_fn = get_viewport.dyn_ref::<js_sys::Function>().ok_or_else(|| {
            ExtractionError::JsBridgeError("getViewport is not a function".to_string())
        })?;

        let scale_obj = Object::new();
        Reflect::set(
            &scale_obj,
            &JsValue::from_str("scale"),
            &JsValue::from_f64(1.0),
        )
        .ok();

        let viewport = get_viewport_fn.call1(&page, &scale_obj).map_err(|e| {
            ExtractionError::JsBridgeError(format!("getViewport call failed: {:?}", e))
        })?;

        let width = Reflect::get(&viewport, &JsValue::from_str("width"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(612.0);

        let height = Reflect::get(&viewport, &JsValue::from_str("height"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(792.0);

        // Get text content
        let get_text_content =
            Reflect::get(&page, &JsValue::from_str("getTextContent")).map_err(|e| {
                ExtractionError::JsBridgeError(format!("getTextContent failed: {:?}", e))
            })?;

        let get_text_content_fn =
            get_text_content
                .dyn_ref::<js_sys::Function>()
                .ok_or_else(|| {
                    ExtractionError::JsBridgeError("getTextContent is not a function".to_string())
                })?;

        let text_promise = get_text_content_fn.call0(&page).map_err(|e| {
            ExtractionError::JsBridgeError(format!("getTextContent call failed: {:?}", e))
        })?;

        let text_promise: Promise = text_promise
            .dyn_into()
            .map_err(|_| ExtractionError::JsBridgeError("Not a promise".to_string()))?;

        let text_content = JsFuture::from(text_promise).await.map_err(|e| {
            ExtractionError::JsBridgeError(format!("Text extraction failed: {:?}", e))
        })?;

        // Parse text items
        let items = Reflect::get(&text_content, &JsValue::from_str("items"))
            .map_err(|e| ExtractionError::JsBridgeError(format!("items not found: {:?}", e)))?;

        let items_array: Array = items
            .dyn_into()
            .map_err(|_| ExtractionError::JsBridgeError("items is not an array".to_string()))?;

        let mut text_items = Vec::new();
        let mut raw_text = String::new();

        for i in 0..items_array.length() {
            if let Some(item) = items_array.get(i).dyn_ref::<Object>() {
                let text = Reflect::get(item, &JsValue::from_str("str"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();

                if !text.is_empty() {
                    // Get transform for position
                    let transform = Reflect::get(item, &JsValue::from_str("transform"))
                        .ok()
                        .and_then(|v| v.dyn_into::<Array>().ok());

                    let (x, y) = if let Some(t) = transform {
                        (
                            t.get(4).as_f64().unwrap_or(0.0),
                            t.get(5).as_f64().unwrap_or(0.0),
                        )
                    } else {
                        (0.0, 0.0)
                    };

                    let item_width = Reflect::get(item, &JsValue::from_str("width"))
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    let item_height = Reflect::get(item, &JsValue::from_str("height"))
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(12.0);

                    text_items.push(TextItem {
                        text: text.clone(),
                        x,
                        y,
                        width: item_width,
                        height: item_height,
                        font_name: Reflect::get(item, &JsValue::from_str("fontName"))
                            .ok()
                            .and_then(|v| v.as_string()),
                        font_size: Some(item_height),
                    });

                    raw_text.push_str(&text);

                    // Add space if this item has EOL flag
                    let has_eol = Reflect::get(item, &JsValue::from_str("hasEOL"))
                        .ok()
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if has_eol {
                        raw_text.push('\n');
                    } else {
                        raw_text.push(' ');
                    }
                }
            }
        }

        Ok(PageContent {
            page_number: page_num,
            text_items,
            raw_text: raw_text.trim().to_string(),
            width,
            height,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_with_pdfjs(
        &self,
        _data: &[u8],
    ) -> Result<Vec<PageContent>, ExtractionError> {
        Err(ExtractionError::BackendUnavailable(
            "Browser backend only available in WASM".to_string(),
        ))
    }
}

impl Default for BrowserExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfBackend for BrowserExtractor {
    fn name(&self) -> &'static str {
        "browser"
    }

    fn can_handle(&self, data: &[u8]) -> bool {
        // Browser can handle any PDF if pdf.js is available
        data.len() > 4 && &data[0..4] == b"%PDF" && Self::is_available()
    }

    fn extract_sync(&self, _data: &[u8]) -> Result<Vec<PageContent>, ExtractionError> {
        // Browser extraction is async-only
        Err(ExtractionError::Other(
            "Browser extraction requires async - use extract_with_pdfjs".to_string(),
        ))
    }

    fn validate_output(&self, pages: &[PageContent]) -> ValidationResult {
        let all_text: String = pages.iter().map(|p| p.raw_text.as_str()).collect();
        analyze_text_quality(&all_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_extractor_name() {
        let extractor = BrowserExtractor::new();
        assert_eq!(extractor.name(), "browser");
    }

    // This test only works in WASM environment
    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_create_js_array_copy() {
        let data = vec![1u8, 2, 3, 4, 5];
        let arr = BrowserExtractor::create_js_array(&data, false);
        assert_eq!(arr.length(), 5);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_browser_not_available_outside_wasm() {
        assert!(!BrowserExtractor::is_available());
    }
}
