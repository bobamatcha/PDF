//! Stateful PDF session management
//!
//! Provides a session-based API that holds document state in Rust,
//! minimizing JavaScript state management.

use crate::page_info::PageInfo;
use crate::validation::{validate_pdf, PdfInfo};
use lopdf::Document;
use wasm_bindgen::prelude::*;

/// Document entry with metadata
struct DocumentEntry {
    name: String,
    bytes: Vec<u8>,
    document: Document,
    info: PdfInfo,
}

/// Session mode determines available operations
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    /// Split mode: single document, extract pages
    Split,
    /// Merge mode: multiple documents, combine
    Merge,
}

/// Stateful PDF session that holds documents in Rust memory
#[wasm_bindgen]
pub struct PdfJoinSession {
    mode: SessionMode,
    documents: Vec<DocumentEntry>,
    selected_pages: Vec<u32>,
    progress_callback: Option<js_sys::Function>,
}

#[wasm_bindgen]
impl PdfJoinSession {
    /// Create a new session in the specified mode
    #[wasm_bindgen(constructor)]
    pub fn new(mode: SessionMode) -> Self {
        Self {
            mode,
            documents: Vec::new(),
            selected_pages: Vec::new(),
            progress_callback: None,
        }
    }

    /// Get the session mode
    #[wasm_bindgen(getter)]
    pub fn mode(&self) -> SessionMode {
        self.mode
    }

    /// Set a progress callback function
    /// Callback signature: (current: number, total: number, message: string) => void
    #[wasm_bindgen(js_name = setProgressCallback)]
    pub fn set_progress_callback(&mut self, callback: js_sys::Function) {
        self.progress_callback = Some(callback);
    }

    /// Internal method to add a document (testable without JsValue)
    fn add_document_internal(&mut self, name: &str, bytes: &[u8]) -> Result<PdfInfo, String> {
        // In split mode, only allow one document
        if self.mode == SessionMode::Split && !self.documents.is_empty() {
            return Err(
                "Split mode only allows one document. Remove existing document first.".to_string(),
            );
        }

        // Validate and parse the PDF
        let info = validate_pdf(bytes)?;

        let document =
            Document::load_mem(bytes).map_err(|e| format!("Failed to parse PDF: {}", e))?;

        let entry = DocumentEntry {
            name: name.to_string(),
            bytes: bytes.to_vec(),
            document,
            info: info.clone(),
        };

        self.documents.push(entry);

        // In split mode, auto-select all pages
        if self.mode == SessionMode::Split {
            self.selected_pages = (1..=info.page_count).collect();
        }

        Ok(info)
    }

    /// Add a document to the session
    /// Returns document info as JSON on success
    #[wasm_bindgen(js_name = addDocument)]
    pub fn add_document(&mut self, name: &str, bytes: &[u8]) -> Result<JsValue, JsValue> {
        let info = self
            .add_document_internal(name, bytes)
            .map_err(|e| JsValue::from_str(&e))?;

        serde_wasm_bindgen::to_value(&info)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Remove a document by index
    #[wasm_bindgen(js_name = removeDocument)]
    pub fn remove_document(&mut self, index: usize) -> Result<(), JsValue> {
        if index >= self.documents.len() {
            return Err(JsValue::from_str("Document index out of bounds"));
        }
        self.documents.remove(index);

        // Clear page selection if no documents remain
        if self.documents.is_empty() {
            self.selected_pages.clear();
        }

        Ok(())
    }

    /// Reorder documents (for merge mode)
    /// new_order is an array of current indices in the desired new order
    #[wasm_bindgen(js_name = reorderDocuments)]
    pub fn reorder_documents(&mut self, new_order: &[usize]) -> Result<(), JsValue> {
        if self.mode != SessionMode::Merge {
            return Err(JsValue::from_str("Reorder only available in merge mode"));
        }

        if new_order.len() != self.documents.len() {
            return Err(JsValue::from_str("Invalid order: wrong number of indices"));
        }

        // Validate all indices
        let mut seen = vec![false; self.documents.len()];
        for &idx in new_order {
            if idx >= self.documents.len() {
                return Err(JsValue::from_str("Invalid order: index out of bounds"));
            }
            if seen[idx] {
                return Err(JsValue::from_str("Invalid order: duplicate index"));
            }
            seen[idx] = true;
        }

        // Reorder
        let mut new_docs = Vec::with_capacity(self.documents.len());
        for &idx in new_order {
            new_docs.push(std::mem::take(&mut self.documents[idx]));
        }
        self.documents = new_docs;

        Ok(())
    }

    /// Set page selection for split mode
    /// Input: "1-3, 5, 8-10" format
    #[wasm_bindgen(js_name = setPageSelection)]
    pub fn set_page_selection(&mut self, range_str: &str) -> Result<(), JsValue> {
        if self.mode != SessionMode::Split {
            return Err(JsValue::from_str(
                "Page selection only available in split mode",
            ));
        }

        if self.documents.is_empty() {
            return Err(JsValue::from_str("No document loaded"));
        }

        let pages = pdfjoin_core::parse_ranges(range_str)
            .map_err(|e| JsValue::from_str(&format!("Invalid range: {}", e)))?;

        let max_page = self.documents[0].info.page_count;
        for &page in &pages {
            if page == 0 || page > max_page {
                return Err(JsValue::from_str(&format!(
                    "Page {} is out of range (1-{})",
                    page, max_page
                )));
            }
        }

        self.selected_pages = pages;
        Ok(())
    }

    /// Get selected pages as array
    #[wasm_bindgen(js_name = getSelectedPages)]
    pub fn get_selected_pages(&self) -> Vec<u32> {
        self.selected_pages.clone()
    }

    /// Get info for a specific page
    #[wasm_bindgen(js_name = getPageInfo)]
    pub fn get_page_info(&self, doc_index: usize, page_num: u32) -> Result<JsValue, JsValue> {
        if doc_index >= self.documents.len() {
            return Err(JsValue::from_str("Document index out of bounds"));
        }

        let doc = &self.documents[doc_index];
        let info =
            PageInfo::from_document(&doc.document, page_num).map_err(|e| JsValue::from_str(&e))?;

        serde_wasm_bindgen::to_value(&info)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Get all document infos
    #[wasm_bindgen(js_name = getDocumentInfos)]
    pub fn get_document_infos(&self) -> Result<JsValue, JsValue> {
        let infos: Vec<_> = self
            .documents
            .iter()
            .map(|d| DocumentInfoJs {
                name: d.name.clone(),
                page_count: d.info.page_count,
                size_bytes: d.bytes.len(),
                version: d.info.version.clone(),
                encrypted: d.info.encrypted,
            })
            .collect();

        serde_wasm_bindgen::to_value(&infos)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Get total page count across all documents
    #[wasm_bindgen(js_name = getTotalPageCount)]
    pub fn get_total_page_count(&self) -> u32 {
        self.documents.iter().map(|d| d.info.page_count).sum()
    }

    /// Get document count
    #[wasm_bindgen(js_name = getDocumentCount)]
    pub fn get_document_count(&self) -> usize {
        self.documents.len()
    }

    /// Check if session is ready for execution
    #[wasm_bindgen(js_name = canExecute)]
    pub fn can_execute(&self) -> bool {
        match self.mode {
            SessionMode::Split => !self.documents.is_empty() && !self.selected_pages.is_empty(),
            SessionMode::Merge => self.documents.len() >= 2,
        }
    }

    /// Execute the operation and return result as Uint8Array
    pub fn execute(&self) -> Result<js_sys::Uint8Array, JsValue> {
        if !self.can_execute() {
            return Err(JsValue::from_str("Session not ready for execution"));
        }

        self.report_progress(0, 100, "Starting...")?;

        let result = match self.mode {
            SessionMode::Split => self.execute_split()?,
            SessionMode::Merge => self.execute_merge()?,
        };

        self.report_progress(100, 100, "Complete")?;

        let array = js_sys::Uint8Array::new_with_length(result.len() as u32);
        array.copy_from(&result);
        Ok(array)
    }

    /// Execute split operation
    fn execute_split(&self) -> Result<Vec<u8>, JsValue> {
        self.report_progress(10, 100, "Extracting pages...")?;

        let result =
            pdfjoin_core::split_document(&self.documents[0].bytes, self.selected_pages.clone())
                .map_err(|e| JsValue::from_str(&format!("Split failed: {}", e)))?;

        self.report_progress(90, 100, "Finalizing...")?;

        Ok(result)
    }

    /// Execute merge operation
    fn execute_merge(&self) -> Result<Vec<u8>, JsValue> {
        let total = self.documents.len();

        self.report_progress(5, 100, "Preparing documents...")?;

        let docs: Vec<Vec<u8>> = self
            .documents
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let _ = self.report_progress(
                    5 + (i * 80 / total) as u32,
                    100,
                    &format!("Processing document {}/{}...", i + 1, total),
                );
                d.bytes.clone()
            })
            .collect();

        self.report_progress(85, 100, "Merging...")?;

        let result = pdfjoin_core::merge_documents(docs)
            .map_err(|e| JsValue::from_str(&format!("Merge failed: {}", e)))?;

        self.report_progress(95, 100, "Finalizing...")?;

        Ok(result)
    }

    /// Report progress to JavaScript callback
    fn report_progress(&self, current: u32, total: u32, message: &str) -> Result<(), JsValue> {
        if let Some(ref callback) = self.progress_callback {
            let this = JsValue::null();
            let _ = callback.call3(
                &this,
                &JsValue::from(current),
                &JsValue::from(total),
                &JsValue::from_str(message),
            );
        }
        Ok(())
    }
}

// Implement Default for DocumentEntry to allow std::mem::take
impl Default for DocumentEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            bytes: Vec::new(),
            document: Document::with_version("1.7"),
            info: PdfInfo::default(),
        }
    }
}

/// Document info for JS serialization
#[derive(serde::Serialize)]
struct DocumentInfoJs {
    name: String,
    page_count: u32,
    size_bytes: usize,
    version: String,
    encrypted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, content::Operation, Dictionary, Object, Stream};

    /// Create a valid test PDF with the specified number of pages
    /// Uses the same pattern as pdfjoin-core tests
    fn create_test_pdf(num_pages: u32) -> Vec<u8> {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();

        let mut page_ids = Vec::new();

        for i in 0..num_pages {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new(
                        "Tf",
                        vec![Object::Name(b"F1".to_vec()), Object::Integer(12)],
                    ),
                    Operation::new("Td", vec![Object::Integer(100), Object::Integer(700)]),
                    Operation::new(
                        "Tj",
                        vec![Object::String(
                            format!("Page {}", i + 1).into_bytes(),
                            lopdf::StringFormat::Literal,
                        )],
                    ),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id =
                doc.add_object(Stream::new(Dictionary::new(), content.encode().unwrap()));

            let page = Dictionary::from_iter(vec![
                ("Type", Object::Name(b"Page".to_vec())),
                ("Parent", Object::Reference(pages_id)),
                (
                    "MediaBox",
                    Object::Array(vec![
                        Object::Integer(0),
                        Object::Integer(0),
                        Object::Integer(612),
                        Object::Integer(792),
                    ]),
                ),
                ("Contents", Object::Reference(content_id)),
            ]);
            let page_id = doc.add_object(page);
            page_ids.push(page_id);
        }

        let pages = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Pages".to_vec())),
            ("Count", Object::Integer(num_pages as i64)),
            (
                "Kids",
                Object::Array(page_ids.iter().map(|id| Object::Reference(*id)).collect()),
            ),
        ]);
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Catalog".to_vec())),
            ("Pages", Object::Reference(pages_id)),
        ]);
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn test_session_mode_equality() {
        assert_eq!(SessionMode::Split, SessionMode::Split);
        assert_eq!(SessionMode::Merge, SessionMode::Merge);
        assert_ne!(SessionMode::Split, SessionMode::Merge);
    }

    #[test]
    fn test_new_session_is_empty() {
        let session = PdfJoinSession::new(SessionMode::Split);
        assert_eq!(session.get_document_count(), 0);
        assert_eq!(session.get_total_page_count(), 0);
        assert!(!session.can_execute());
    }

    #[test]
    fn test_split_session_add_document() {
        let mut session = PdfJoinSession::new(SessionMode::Split);
        let pdf = create_test_pdf(3);

        let result = session.add_document_internal("test.pdf", &pdf);
        assert!(result.is_ok());
        assert_eq!(session.get_document_count(), 1);
        assert_eq!(session.get_total_page_count(), 3);
    }

    #[test]
    fn test_split_session_auto_selects_all_pages() {
        let mut session = PdfJoinSession::new(SessionMode::Split);
        let pdf = create_test_pdf(5);

        session.add_document_internal("test.pdf", &pdf).unwrap();

        let pages = session.get_selected_pages();
        assert_eq!(pages.len(), 5);
        assert_eq!(pages, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_split_session_only_one_document() {
        let mut session = PdfJoinSession::new(SessionMode::Split);
        let pdf1 = create_test_pdf(2);
        let pdf2 = create_test_pdf(3);

        session.add_document_internal("first.pdf", &pdf1).unwrap();
        let result = session.add_document_internal("second.pdf", &pdf2);

        assert!(result.is_err()); // Should reject second document in split mode
    }

    #[test]
    fn test_merge_session_accepts_multiple_documents() {
        let mut session = PdfJoinSession::new(SessionMode::Merge);
        let pdf1 = create_test_pdf(2);
        let pdf2 = create_test_pdf(3);

        session.add_document_internal("first.pdf", &pdf1).unwrap();
        session.add_document_internal("second.pdf", &pdf2).unwrap();

        assert_eq!(session.get_document_count(), 2);
        assert_eq!(session.get_total_page_count(), 5);
    }

    #[test]
    fn test_split_can_execute_with_document_and_pages() {
        let mut session = PdfJoinSession::new(SessionMode::Split);
        let pdf = create_test_pdf(5);

        assert!(!session.can_execute()); // No document yet

        session.add_document_internal("test.pdf", &pdf).unwrap();
        assert!(session.can_execute()); // Document + auto-selected pages
    }

    #[test]
    fn test_merge_can_execute_with_two_documents() {
        let mut session = PdfJoinSession::new(SessionMode::Merge);
        let pdf1 = create_test_pdf(1);
        let pdf2 = create_test_pdf(1);

        assert!(!session.can_execute()); // No documents

        session.add_document_internal("first.pdf", &pdf1).unwrap();
        assert!(!session.can_execute()); // Only one document

        session.add_document_internal("second.pdf", &pdf2).unwrap();
        assert!(session.can_execute()); // Two documents
    }

    #[test]
    fn test_session_rejects_invalid_pdf() {
        let mut session = PdfJoinSession::new(SessionMode::Split);
        let result = session.add_document_internal("invalid.pdf", b"not a valid pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_add_document_returns_correct_info() {
        let mut session = PdfJoinSession::new(SessionMode::Split);
        let pdf = create_test_pdf(3);

        let info = session.add_document_internal("test.pdf", &pdf).unwrap();
        assert_eq!(info.page_count, 3);
        assert_eq!(info.version, "1.7");
        assert!(!info.encrypted);
        assert!(info.valid);
    }

    // Regression tests: ensure full split/merge execution works with valid PDFs
    // These tests verify the end-to-end flow that was broken when using invalid test PDFs

    #[test]
    fn test_split_execute_produces_valid_pdf() {
        let mut session = PdfJoinSession::new(SessionMode::Split);
        let pdf = create_test_pdf(5);

        session.add_document_internal("test.pdf", &pdf).unwrap();

        // Execute split - extract pages 1-3
        session.selected_pages = vec![1, 2, 3];
        let result = session.execute_split().unwrap();

        // Verify output is a valid PDF
        assert!(result.starts_with(b"%PDF-"));
        let output_doc = Document::load_mem(&result).unwrap();
        assert_eq!(output_doc.get_pages().len(), 3);
    }

    #[test]
    fn test_merge_execute_produces_valid_pdf() {
        let mut session = PdfJoinSession::new(SessionMode::Merge);
        let pdf1 = create_test_pdf(2);
        let pdf2 = create_test_pdf(3);

        session.add_document_internal("first.pdf", &pdf1).unwrap();
        session.add_document_internal("second.pdf", &pdf2).unwrap();

        let result = session.execute_merge().unwrap();

        // Verify output is a valid PDF with combined pages
        assert!(result.starts_with(b"%PDF-"));
        let output_doc = Document::load_mem(&result).unwrap();
        assert_eq!(output_doc.get_pages().len(), 5); // 2 + 3 = 5 pages
    }

    #[test]
    fn test_split_single_page_extraction() {
        let mut session = PdfJoinSession::new(SessionMode::Split);
        let pdf = create_test_pdf(10);

        session.add_document_internal("test.pdf", &pdf).unwrap();
        session.selected_pages = vec![5]; // Extract just page 5

        let result = session.execute_split().unwrap();
        let output_doc = Document::load_mem(&result).unwrap();
        assert_eq!(output_doc.get_pages().len(), 1);
    }
}
