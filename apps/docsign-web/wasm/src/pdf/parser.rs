//! PDF parsing and manipulation using lopdf

use lopdf::{Document, Object, ObjectId};

/// Wrapper around lopdf::Document for WASM-friendly operations
pub struct PdfDocument {
    pub(crate) doc: Document,
    pub(crate) bytes: Vec<u8>,
}

impl PdfDocument {
    /// Load a PDF from raw bytes
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let doc = Document::load_mem(&bytes).map_err(|e| format!("PDF parse error: {}", e))?;
        Ok(Self { doc, bytes })
    }

    /// Get the raw bytes
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Get the number of pages
    pub fn page_count(&self) -> usize {
        self.doc.get_pages().len()
    }

    /// Get page object ID for a given page number (1-indexed)
    pub fn page_id(&self, page_num: u32) -> Option<ObjectId> {
        self.doc.get_pages().get(&page_num).copied()
    }

    /// Get page dimensions (MediaBox) as [x, y, width, height]
    pub fn page_dimensions(&self, page_num: u32) -> Result<[f64; 4], String> {
        let page_id = self
            .page_id(page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;

        let page = self
            .doc
            .get_object(page_id)
            .map_err(|e| format!("Failed to get page object: {}", e))?;

        let page_dict = page.as_dict().map_err(|_| "Page is not a dictionary")?;

        // Try to get MediaBox, falling back to parent if needed
        let media_box = self.get_media_box(page_dict, page_id)?;

        Ok(media_box)
    }

    /// Extract MediaBox from page dictionary, traversing parent if needed
    fn get_media_box(
        &self,
        page_dict: &lopdf::Dictionary,
        _page_id: ObjectId,
    ) -> Result<[f64; 4], String> {
        // First try direct MediaBox
        if let Ok(media_box) = page_dict.get(b"MediaBox") {
            return self.parse_rect(media_box);
        }

        // Try to get from Parent
        if let Ok(parent_ref) = page_dict.get(b"Parent") {
            if let Ok(parent_id) = parent_ref.as_reference() {
                if let Ok(parent) = self.doc.get_object(parent_id) {
                    if let Ok(parent_dict) = parent.as_dict() {
                        if let Ok(media_box) = parent_dict.get(b"MediaBox") {
                            return self.parse_rect(media_box);
                        }
                    }
                }
            }
        }

        // Default to US Letter size
        Ok([0.0, 0.0, 612.0, 792.0])
    }

    /// Parse a PDF rectangle array into [x, y, width, height]
    fn parse_rect(&self, obj: &Object) -> Result<[f64; 4], String> {
        let arr = match obj {
            Object::Array(a) => a,
            Object::Reference(id) => {
                let resolved = self
                    .doc
                    .get_object(*id)
                    .map_err(|e| format!("Failed to resolve reference: {}", e))?;
                resolved
                    .as_array()
                    .map_err(|_| "MediaBox reference is not an array")?
            }
            _ => return Err("MediaBox is not an array".to_string()),
        };

        if arr.len() != 4 {
            return Err(format!("MediaBox has {} elements, expected 4", arr.len()));
        }

        let mut values = [0.0f64; 4];
        for (i, obj) in arr.iter().enumerate() {
            values[i] = self.extract_number(obj)?;
        }

        // Convert from [x1, y1, x2, y2] to [x, y, width, height]
        Ok([
            values[0],
            values[1],
            values[2] - values[0], // width
            values[3] - values[1], // height
        ])
    }

    /// Extract a number from a PDF object
    fn extract_number(&self, obj: &Object) -> Result<f64, String> {
        match obj {
            Object::Integer(i) => Ok(*i as f64),
            Object::Real(r) => Ok(*r as f64),
            Object::Reference(id) => {
                let resolved = self
                    .doc
                    .get_object(*id)
                    .map_err(|e| format!("Failed to resolve: {}", e))?;
                self.extract_number(resolved)
            }
            _ => Err("Expected number in rectangle".to_string()),
        }
    }

    /// Get mutable access to the internal document
    pub fn doc_mut(&mut self) -> &mut Document {
        &mut self.doc
    }

    /// Save the document to bytes
    pub fn save_to_bytes(&mut self) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::new();
        self.doc
            .save_to(&mut buffer)
            .map_err(|e| format!("Failed to save PDF: {}", e))?;
        self.bytes = buffer.clone();
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use actual PDF from typst output
    const TEST_PDF: &[u8] = include_bytes!("../../../../../output/florida_listing_agreement.pdf");

    #[test]
    fn test_from_bytes_valid_pdf() {
        // This test ensures valid PDF bytes can be loaded
        let result = PdfDocument::from_bytes(TEST_PDF.to_vec());
        match result {
            Ok(pdf) => assert!(pdf.page_count() > 0, "Should have at least 1 page"),
            Err(e) => panic!("Valid PDF should parse successfully, got: {}", e),
        }
    }

    #[test]
    fn test_from_bytes_html_fails_with_invalid_header() {
        // This is the regression test for the bug where HTML was passed instead of PDF
        // (e.g., when fetch() returns SPA fallback instead of actual PDF)
        let html_bytes = b"<!DOCTYPE html><html><head></head><body>Not a PDF</body></html>";
        let result = PdfDocument::from_bytes(html_bytes.to_vec());
        match result {
            Ok(_) => panic!("HTML should not parse as PDF"),
            Err(err) => {
                assert!(
                    err.contains("Invalid file header") || err.contains("PDF parse error"),
                    "Error should mention invalid header, got: {}",
                    err
                );
            }
        }
    }

    #[test]
    fn test_from_bytes_empty_fails() {
        let result = PdfDocument::from_bytes(vec![]);
        assert!(result.is_err(), "Empty bytes should fail");
    }

    #[test]
    fn test_from_bytes_garbage_fails() {
        let garbage = vec![0u8; 100]; // All zeros
        let result = PdfDocument::from_bytes(garbage);
        assert!(result.is_err(), "Garbage bytes should fail");
    }

    #[test]
    fn test_pdf_document_struct() {
        // Test that PdfDocument can be created with a Document
        let doc = lopdf::Document::new();
        let pdf = PdfDocument { doc, bytes: vec![] };
        assert_eq!(pdf.page_count(), 0);
    }

    #[test]
    fn test_parse_rect_array() {
        // Create a document and test the parse_rect helper
        let doc = lopdf::Document::new();
        let pdf = PdfDocument { doc, bytes: vec![] };

        // Test with a valid array
        let arr = lopdf::Object::Array(vec![
            lopdf::Object::Integer(0),
            lopdf::Object::Integer(0),
            lopdf::Object::Integer(612),
            lopdf::Object::Integer(792),
        ]);

        let result = pdf.parse_rect(&arr);
        assert!(result.is_ok());
        let dims = result.unwrap();
        assert_eq!(dims[0], 0.0);
        assert_eq!(dims[1], 0.0);
        assert_eq!(dims[2], 612.0); // width
        assert_eq!(dims[3], 792.0); // height
    }

    #[test]
    fn test_extract_number() {
        let doc = lopdf::Document::new();
        let pdf = PdfDocument { doc, bytes: vec![] };

        // Test integer extraction
        let int_obj = lopdf::Object::Integer(42);
        assert_eq!(pdf.extract_number(&int_obj).unwrap(), 42.0);

        // Test real extraction
        let real_obj = lopdf::Object::Real(1.234);
        assert!((pdf.extract_number(&real_obj).unwrap() - 1.234).abs() < 0.001);
    }
}
