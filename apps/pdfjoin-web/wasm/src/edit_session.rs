//! Edit session for PDF editing operations
//!
//! This module provides a WASM-exposed session for editing PDFs.
//! It integrates with the pdfjoin-core operations module.

use lopdf::Document;
use pdfjoin_core::apply_operations::{apply_operations, apply_operations_flattened};
use pdfjoin_core::has_signatures;
use pdfjoin_core::operations::{
    ActionKind, EditOperation, OperationLog, PdfRect, StyledTextSegment, TextStyle,
};
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
        // Optional font styling parameters
        font_name: Option<String>,
        is_italic: bool,
        is_bold: bool,
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
                font_name,
                is_italic,
                is_bold,
            },
        };
        self.operations.add(op)
    }

    /// Add styled text with mixed formatting (bold/italic segments).
    /// segments_json: JSON array of segments, e.g., [{"text":"BOLD","is_bold":true,"is_italic":false},...]
    #[wasm_bindgen(js_name = addStyledText)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_styled_text(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        segments_json: &str,
        font_size: f64,
        color: &str,
        font_name: Option<String>,
    ) -> Result<u64, JsValue> {
        // Parse segments from JSON
        let segments: Vec<StyledTextSegment> = serde_json::from_str(segments_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid segments JSON: {}", e)))?;

        if segments.is_empty() {
            return Err(JsValue::from_str("At least one segment is required"));
        }

        let op = EditOperation::AddStyledText {
            id: 0,
            page,
            rect: PdfRect {
                x,
                y,
                width,
                height,
            },
            segments,
            style: TextStyle {
                font_size,
                color: color.to_string(),
                font_name,
                is_italic: false, // Base style - individual segments have their own flags
                is_bold: false,
            },
        };
        Ok(self.operations.add(op))
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

    /// Add an underline annotation (thin line below text)
    /// Distinct from highlight - creates a proper PDF Underline annotation
    #[wasm_bindgen(js_name = addUnderline)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_underline(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: &str,
        _opacity: f64, // Kept for API compatibility but not used
    ) -> u64 {
        // Use the dedicated AddUnderline operation (not AddHighlight!)
        let op = EditOperation::AddUnderline {
            id: 0,
            page,
            rect: PdfRect {
                x,
                y,
                width,
                height,
            },
            color: color.to_string(),
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

    /// Add a colored rectangle to cover/redact content
    /// color: Hex color string (e.g., "#FFFFFF" for white, "#000000" for black/redact)
    #[wasm_bindgen(js_name = addWhiteRect)]
    pub fn add_white_rect(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: Option<String>,
    ) -> u64 {
        let op = EditOperation::AddWhiteRect {
            id: 0,
            page,
            rect: PdfRect {
                x,
                y,
                width,
                height,
            },
            color: color.unwrap_or_else(|| "#FFFFFF".to_string()),
        };
        self.operations.add(op)
    }

    /// Remove an operation by ID
    #[wasm_bindgen(js_name = removeOperation)]
    pub fn remove_operation(&mut self, id: u64) -> bool {
        self.operations.remove(id)
    }

    /// Update the checked state of a checkbox operation
    /// Returns false if the operation is not found or is not a checkbox
    #[wasm_bindgen(js_name = setCheckbox)]
    pub fn set_checkbox(&mut self, id: u64, checked: bool) -> bool {
        self.operations.set_checkbox(id, checked)
    }

    /// Update the rect (position and size) of an operation
    /// Works for text, highlight, checkbox, and whiteout operations
    /// Returns false if the operation is not found
    #[wasm_bindgen(js_name = updateRect)]
    pub fn update_rect(&mut self, id: u64, x: f64, y: f64, width: f64, height: f64) -> bool {
        self.operations.update_rect(
            id,
            PdfRect {
                x,
                y,
                width,
                height,
            },
        )
    }

    /// Update the text content of a text operation
    /// Returns false if the operation is not found or is not a text operation
    #[wasm_bindgen(js_name = updateText)]
    pub fn update_text(&mut self, id: u64, new_text: &str) -> bool {
        self.operations.update_text(id, new_text, None)
    }

    /// Update the text content and style of a text operation
    /// Returns false if the operation is not found or is not a text operation
    #[wasm_bindgen(js_name = updateTextWithStyle)]
    #[allow(clippy::too_many_arguments)]
    pub fn update_text_with_style(
        &mut self,
        id: u64,
        new_text: &str,
        font_size: f64,
        color: &str,
        font_name: Option<String>,
        is_italic: bool,
        is_bold: bool,
    ) -> bool {
        let style = TextStyle {
            font_size,
            color: color.to_string(),
            font_name,
            is_italic,
            is_bold,
        };
        self.operations.update_text(id, new_text, Some(&style))
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

    // ============ Action-Based Undo/Redo (Phase 4) ============

    /// Begin a new action of the given kind.
    /// Kind values: "textbox", "whiteout", "checkbox", "highlight", "replacetext", "move", "resize", "delete"
    #[wasm_bindgen(js_name = beginAction)]
    pub fn begin_action(&mut self, kind: &str) {
        let action_kind = match kind.to_lowercase().as_str() {
            "textbox" => ActionKind::AddTextBox,
            "whiteout" => ActionKind::AddWhiteout,
            "checkbox" => ActionKind::AddCheckbox,
            "highlight" => ActionKind::AddHighlight,
            "replacetext" => ActionKind::ReplaceText,
            "move" => ActionKind::Move,
            "resize" => ActionKind::Resize,
            "delete" => ActionKind::Delete,
            _ => ActionKind::AddTextBox, // Default fallback
        };
        self.operations.begin_action(action_kind);
    }

    /// Commit the current action to the undo stack.
    /// Returns true if an action was committed.
    #[wasm_bindgen(js_name = commitAction)]
    pub fn commit_action(&mut self) -> bool {
        self.operations.commit_action()
    }

    /// Abort the current action and remove any operations that were added.
    #[wasm_bindgen(js_name = abortAction)]
    pub fn abort_action(&mut self) {
        self.operations.abort_action()
    }

    /// Undo the last action.
    /// Returns an array of operation IDs that were removed (for DOM cleanup).
    /// Returns null if there's nothing to undo.
    #[wasm_bindgen]
    pub fn undo(&mut self) -> Option<js_sys::BigInt64Array> {
        self.operations.undo().map(|ids| {
            let signed_ids: Vec<i64> = ids.iter().map(|&id| id as i64).collect();
            js_sys::BigInt64Array::from(&signed_ids[..])
        })
    }

    /// Redo the last undone action.
    /// Returns an array of operation IDs that were restored (for DOM recreation).
    /// Returns null if there's nothing to redo.
    #[wasm_bindgen]
    pub fn redo(&mut self) -> Option<js_sys::BigInt64Array> {
        self.operations.redo().map(|ids| {
            let signed_ids: Vec<i64> = ids.iter().map(|&id| id as i64).collect();
            js_sys::BigInt64Array::from(&signed_ids[..])
        })
    }

    /// Check if there are actions that can be undone.
    #[wasm_bindgen(js_name = canUndo)]
    pub fn can_undo(&self) -> bool {
        self.operations.can_undo()
    }

    /// Check if there are actions that can be redone.
    #[wasm_bindgen(js_name = canRedo)]
    pub fn can_redo(&self) -> bool {
        self.operations.can_redo()
    }

    /// Get operation details as JSON for recreation during redo.
    /// Returns null if operation not found.
    #[wasm_bindgen(js_name = getOperationJson)]
    pub fn get_operation_json(&self, id: u64) -> Option<String> {
        self.operations
            .get_operation(id)
            .and_then(|op| serde_json::to_string(op).ok())
    }

    /// Record that an operation was removed (for Delete actions).
    /// This stores the operation so it can be restored on undo.
    #[wasm_bindgen(js_name = recordRemovedOp)]
    pub fn record_removed_op(&mut self, id: u64) -> bool {
        if let Some(op) = self.operations.get_operation(id).cloned() {
            self.operations.record_removed_op(op);
            true
        } else {
            false
        }
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

    /// Apply all operations with flattening - writes directly to page content
    /// instead of creating annotations. This makes edits permanent and uneditable.
    #[wasm_bindgen(js_name = exportFlattened)]
    pub fn export_flattened(&self) -> Result<js_sys::Uint8Array, JsValue> {
        if self.is_signed {
            return Err(JsValue::from_str(
                "Cannot export: Document is signed. Editing would invalidate the signature.",
            ));
        }

        let result = apply_operations_flattened(&self.document_bytes, &self.operations)
            .map_err(|e| JsValue::from_str(&format!("Export error: {}", e)))?;

        let array = js_sys::Uint8Array::new_with_length(result.len() as u32);
        array.copy_from(&result);
        Ok(array)
    }

    /// Debug helper to verify flattened export works correctly.
    /// Returns diagnostic info about the export operation.
    #[wasm_bindgen(js_name = debugExportFlattened)]
    pub fn debug_export_flattened(&self) -> Result<String, JsValue> {
        let mut debug_log = String::new();

        // Get document pages
        let doc = Document::load_mem(&self.document_bytes)
            .map_err(|e| JsValue::from_str(&format!("Load error: {}", e)))?;
        let pages: Vec<u32> = doc.get_pages().keys().copied().collect();

        // Get operation pages
        let op_pages: Vec<u32> = self
            .operations
            .operations()
            .iter()
            .map(|op| op.page())
            .collect();

        debug_log.push_str(&format!("Document pages: {:?}\n", pages));
        debug_log.push_str(&format!("Operation pages: {:?}\n", op_pages));
        debug_log.push_str(&format!(
            "Operation count: {}\n",
            self.operations.operations().len()
        ));

        // Call actual flattened export
        match apply_operations_flattened(&self.document_bytes, &self.operations) {
            Ok(result) => {
                let result_str = String::from_utf8_lossy(&result);
                let has_white_rect = result_str.contains("1 1 1 rg");

                debug_log.push_str(&format!(
                    "Original size: {} bytes\n",
                    self.document_bytes.len()
                ));
                debug_log.push_str(&format!("Result size: {} bytes\n", result.len()));
                debug_log.push_str(&format!("Has '1 1 1 rg': {}\n", has_white_rect));
            }
            Err(e) => {
                debug_log.push_str(&format!("Export error: {}\n", e));
            }
        }

        Ok(debug_log)
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

        let id = session.add_text(
            1, 100.0, 700.0, 200.0, 20.0, "Hello", 12.0, "#000000", None, false, false,
        );
        assert_eq!(id, 0);
        assert!(session.has_changes());
        assert_eq!(session.get_operation_count(), 1);
    }

    #[test]
    fn test_remove_operation() {
        let pdf = create_test_pdf();
        let mut session = EditSession::new("test.pdf", &pdf).unwrap();

        let id = session.add_text(
            1, 100.0, 700.0, 200.0, 20.0, "Hello", 12.0, "#000000", None, false, false,
        );
        assert!(session.has_changes());

        assert!(session.remove_operation(id));
        assert!(!session.has_changes());
    }

    #[test]
    fn test_multiple_operations() {
        let pdf = create_test_pdf();
        let mut session = EditSession::new("test.pdf", &pdf).unwrap();

        session.add_text(
            1, 100.0, 700.0, 200.0, 20.0, "Title", 14.0, "#000000", None, false, false,
        );
        session.add_highlight(1, 50.0, 600.0, 300.0, 20.0, "#FFFF00", 0.5);
        session.add_checkbox(1, 100.0, 500.0, 20.0, 20.0, true);

        assert_eq!(session.get_operation_count(), 3);
    }

    #[test]
    fn test_add_bold_text() {
        let pdf = create_test_pdf();
        let mut session = EditSession::new("test.pdf", &pdf).unwrap();

        // Bold text
        let id = session.add_text(
            1, 100.0, 700.0, 200.0, 20.0, "Bold", 12.0, "#000000", None, false, true,
        );
        assert_eq!(id, 0);
        assert!(session.has_changes());
    }

    #[test]
    fn test_add_italic_text() {
        let pdf = create_test_pdf();
        let mut session = EditSession::new("test.pdf", &pdf).unwrap();

        // Italic text
        let id = session.add_text(
            1, 100.0, 700.0, 200.0, 20.0, "Italic", 12.0, "#000000", None, true, false,
        );
        assert_eq!(id, 0);
        assert!(session.has_changes());
    }

    // Note: test_export_flattened_applies_whiteout cannot run outside WASM context
    // because export_flattened returns js_sys::Uint8Array. The flatten functionality
    // is tested in pdfjoin-core unit tests instead.
}
