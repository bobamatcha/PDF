//! Edit session for PDF editing operations
//!
//! This module provides a WASM-exposed session for editing PDFs.
//! It integrates with the pdfjoin-core operations module.

use pdfjoin_core::apply_operations::apply_operations;
use pdfjoin_core::has_signatures;
use pdfjoin_core::operations::{EditOperation, OperationLog, PdfRect, TextStyle};
use wasm_bindgen::prelude::*;

/// Session for editing a single PDF document
#[wasm_bindgen]
pub struct EditSession {
    document_bytes: Vec<u8>,
    document_name: String,
    page_count: u32,
    operations: OperationLog,
    is_signed: bool,
}

#[wasm_bindgen]
impl EditSession {
    /// Create a new edit session with the given PDF
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str, bytes: &[u8]) -> Result<EditSession, JsValue> {
        // Check for signatures first
        let is_signed =
            has_signatures(bytes).map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

        // Get page count
        let page_count = pdfjoin_core::get_page_count(bytes)
            .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

        Ok(EditSession {
            document_bytes: bytes.to_vec(),
            document_name: name.to_string(),
            page_count,
            operations: OperationLog::new(),
            is_signed,
        })
    }

    /// Check if the document is signed
    #[wasm_bindgen(getter, js_name = isSigned)]
    pub fn is_signed(&self) -> bool {
        self.is_signed
    }

    /// Get page count
    #[wasm_bindgen(getter, js_name = pageCount)]
    pub fn page_count(&self) -> u32 {
        self.page_count
    }

    /// Get document name
    #[wasm_bindgen(getter, js_name = documentName)]
    pub fn document_name(&self) -> String {
        self.document_name.clone()
    }

    /// Get document bytes for PDF.js rendering
    #[wasm_bindgen(js_name = getDocumentBytes)]
    pub fn get_document_bytes(&self) -> js_sys::Uint8Array {
        let array = js_sys::Uint8Array::new_with_length(self.document_bytes.len() as u32);
        array.copy_from(&self.document_bytes);
        array
    }

    /// Add a text annotation
    #[wasm_bindgen(js_name = addText)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_text(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        text: &str,
        font_size: f64,
        color: &str,
    ) -> u64 {
        let op = EditOperation::AddText {
            id: 0,
            page,
            rect: PdfRect {
                x,
                y,
                width,
                height,
            },
            text: text.to_string(),
            style: TextStyle {
                font_size,
                color: color.to_string(),
                font_name: None, // AddText uses default font
                is_italic: false,
                is_bold: false,
            },
        };
        self.operations.add(op)
    }

    /// Add a highlight annotation
    #[wasm_bindgen(js_name = addHighlight)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_highlight(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: &str,
        opacity: f64,
    ) -> u64 {
        let op = EditOperation::AddHighlight {
            id: 0,
            page,
            rect: PdfRect {
                x,
                y,
                width,
                height,
            },
            color: color.to_string(),
            opacity,
        };
        self.operations.add(op)
    }

    /// Add a checkbox annotation
    #[wasm_bindgen(js_name = addCheckbox)]
    pub fn add_checkbox(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        checked: bool,
    ) -> u64 {
        let op = EditOperation::AddCheckbox {
            id: 0,
            page,
            rect: PdfRect {
                x,
                y,
                width,
                height,
            },
            checked,
        };
        self.operations.add(op)
    }

    /// Replace text at a position (redact original + overlay new text)
    #[wasm_bindgen(js_name = replaceText)]
    #[allow(clippy::too_many_arguments)]
    pub fn replace_text(
        &mut self,
        page: u32,
        // Original text bounding box (to cover with white)
        orig_x: f64,
        orig_y: f64,
        orig_width: f64,
        orig_height: f64,
        // New text position (can be same as original)
        new_x: f64,
        new_y: f64,
        new_width: f64,
        new_height: f64,
        // Text content
        original_text: &str,
        new_text: &str,
        font_size: f64,
        color: &str,
        // Font name from PDF.js (e.g., "Times-Roman", "BCDEEE+ArialMT")
        font_name: Option<String>,
        // Font style flags
        is_italic: bool,
        is_bold: bool,
    ) -> u64 {
        let op = EditOperation::ReplaceText {
            id: 0,
            page,
            original_rect: PdfRect {
                x: orig_x,
                y: orig_y,
                width: orig_width,
                height: orig_height,
            },
            replacement_rect: PdfRect {
                x: new_x,
                y: new_y,
                width: new_width,
                height: new_height,
            },
            original_text: original_text.to_string(),
            new_text: new_text.to_string(),
            style: TextStyle {
                font_size,
                color: color.to_string(),
                font_name,
                is_italic,
                is_bold,
            },
        };
        self.operations.add(op)
    }

    /// Add a white rectangle to cover/redact content
    #[wasm_bindgen(js_name = addWhiteRect)]
    pub fn add_white_rect(&mut self, page: u32, x: f64, y: f64, width: f64, height: f64) -> u64 {
        let op = EditOperation::AddWhiteRect {
            id: 0,
            page,
            rect: PdfRect {
                x,
                y,
                width,
                height,
            },
        };
        self.operations.add(op)
    }

    /// Remove an operation by ID
    #[wasm_bindgen(js_name = removeOperation)]
    pub fn remove_operation(&mut self, id: u64) -> bool {
        self.operations.remove(id)
    }

    /// Check if there are unsaved changes
    #[wasm_bindgen(js_name = hasChanges)]
    pub fn has_changes(&self) -> bool {
        !self.operations.is_empty()
    }

    /// Get number of operations
    #[wasm_bindgen(js_name = getOperationCount)]
    pub fn get_operation_count(&self) -> usize {
        self.operations.operations().len()
    }

    /// Get operations as JSON (for debugging/persistence)
    #[wasm_bindgen(js_name = getOperationsJson)]
    pub fn get_operations_json(&self) -> Result<String, JsValue> {
        self.operations
            .to_json()
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Apply all operations and return the modified PDF
    pub fn export(&self) -> Result<js_sys::Uint8Array, JsValue> {
        if self.is_signed {
            return Err(JsValue::from_str(
                "Cannot export: Document is signed. Editing would invalidate the signature.",
            ));
        }

        let result = apply_operations(&self.document_bytes, &self.operations)
            .map_err(|e| JsValue::from_str(&format!("Export error: {}", e)))?;

        let array = js_sys::Uint8Array::new_with_length(result.len() as u32);
        array.copy_from(&result);
        Ok(array)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pdf() -> Vec<u8> {
        use lopdf::{dictionary, Document, Object};

        let mut doc = Document::with_version("1.7");
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        });
        if let Ok(page) = doc.get_object_mut(page_id) {
            if let Ok(dict) = page.as_dict_mut() {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn test_edit_session_creation() {
        let pdf = create_test_pdf();
        let session = EditSession::new("test.pdf", &pdf).unwrap();
        assert_eq!(session.document_name(), "test.pdf");
        assert_eq!(session.page_count(), 1);
        assert!(!session.is_signed());
        assert!(!session.has_changes());
    }

    #[test]
    fn test_add_text_operation() {
        let pdf = create_test_pdf();
        let mut session = EditSession::new("test.pdf", &pdf).unwrap();

        let id = session.add_text(1, 100.0, 700.0, 200.0, 20.0, "Hello", 12.0, "#000000");
        assert_eq!(id, 0);
        assert!(session.has_changes());
        assert_eq!(session.get_operation_count(), 1);
    }

    #[test]
    fn test_remove_operation() {
        let pdf = create_test_pdf();
        let mut session = EditSession::new("test.pdf", &pdf).unwrap();

        let id = session.add_text(1, 100.0, 700.0, 200.0, 20.0, "Hello", 12.0, "#000000");
        assert!(session.has_changes());

        assert!(session.remove_operation(id));
        assert!(!session.has_changes());
    }

    #[test]
    fn test_multiple_operations() {
        let pdf = create_test_pdf();
        let mut session = EditSession::new("test.pdf", &pdf).unwrap();

        session.add_text(1, 100.0, 700.0, 200.0, 20.0, "Title", 14.0, "#000000");
        session.add_highlight(1, 50.0, 600.0, 300.0, 20.0, "#FFFF00", 0.5);
        session.add_checkbox(1, 100.0, 500.0, 20.0, 20.0, true);

        assert_eq!(session.get_operation_count(), 3);
    }
}
