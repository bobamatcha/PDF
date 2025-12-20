//! Native extraction backend using enhanced lopdf parsing
//!
//! High-fidelity text extraction with proper encoding support.
//! Uses enhanced lopdf parsing with UTF-8, UTF-16BE, and Latin-1 fallback.
//!
//! ## Optimization
//!
//! Supports extraction from pre-parsed Document to avoid double-parsing
//! when the router has already parsed for analysis.

use super::types::*;
use lopdf::Document;

/// Native backend using enhanced lopdf for high-fidelity extraction
pub struct NativeExtractor;

impl NativeExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract from pre-parsed Document (avoids double-parsing)
    ///
    /// Use this when the router has already parsed the document for analysis.
    pub fn extract_from_document(
        &self,
        doc: &Document,
    ) -> Result<Vec<PageContent>, ExtractionError> {
        extract_from_lopdf_document(doc)
    }

    /// Check if the text quality is acceptable
    #[allow(dead_code)]
    fn is_quality_acceptable(text: &str) -> bool {
        if text.is_empty() {
            return false;
        }

        let validation = analyze_text_quality(text);
        validation.is_valid
    }
}

impl Default for NativeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfBackend for NativeExtractor {
    fn name(&self) -> &'static str {
        "native"
    }

    fn can_handle(&self, data: &[u8]) -> bool {
        // Native can attempt any valid PDF
        data.len() > 4 && &data[0..4] == b"%PDF"
    }

    fn extract_sync(&self, data: &[u8]) -> Result<Vec<PageContent>, ExtractionError> {
        // Use enhanced lopdf parsing with better encoding handling
        enhanced_lopdf_extract(data)
    }

    fn validate_output(&self, pages: &[PageContent]) -> ValidationResult {
        let all_text: String = pages.iter().map(|p| p.raw_text.as_str()).collect();
        analyze_text_quality(&all_text)
    }
}

/// Enhanced lopdf extraction with better encoding handling (parses internally)
fn enhanced_lopdf_extract(data: &[u8]) -> Result<Vec<PageContent>, ExtractionError> {
    let doc = Document::load_mem(data).map_err(|e| ExtractionError::ParseError(e.to_string()))?;

    extract_from_lopdf_document(&doc)
}

/// Core extraction from pre-parsed Document (no parsing overhead)
fn extract_from_lopdf_document(doc: &Document) -> Result<Vec<PageContent>, ExtractionError> {
    let mut pages = Vec::new();
    let page_ids = doc.get_pages();

    for (&page_num, &page_id) in page_ids.iter() {
        let mut page_text = String::new();

        // Try to extract text from content stream
        if let Ok(content) = doc.get_page_content(page_id) {
            // Parse content stream operations
            if let Ok(operations) = lopdf::content::Content::decode(&content) {
                for op in operations.operations {
                    match op.operator.as_str() {
                        "Tj" | "TJ" | "'" | "\"" => {
                            // Text showing operators
                            for operand in &op.operands {
                                if let Ok(text) = extract_text_from_operand(doc, operand) {
                                    page_text.push_str(&text);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Get page dimensions
        let (width, height) = get_page_dimensions(doc, page_id);

        pages.push(PageContent {
            page_number: page_num,
            text_items: vec![],
            raw_text: page_text,
            width,
            height,
        });
    }

    // Validate output quality
    let all_text: String = pages.iter().map(|p| p.raw_text.as_str()).collect();
    let validation = analyze_text_quality(&all_text);

    if !validation.is_valid && !all_text.is_empty() {
        return Err(ExtractionError::EncodingFailure {
            details: validation.details,
            recoverable: true,
        });
    }

    Ok(pages)
}

fn extract_text_from_operand(doc: &lopdf::Document, operand: &lopdf::Object) -> Result<String, ()> {
    let _ = doc; // Suppress unused warning, kept for future ToUnicode support
    match operand {
        lopdf::Object::String(bytes, _) => {
            // Try UTF-8 first
            if let Ok(s) = String::from_utf8(bytes.clone()) {
                return Ok(s);
            }
            // Try UTF-16BE (common in PDFs)
            if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
                let chars: Vec<u16> = bytes[2..]
                    .chunks(2)
                    .filter_map(|chunk| {
                        if chunk.len() == 2 {
                            Some(u16::from_be_bytes([chunk[0], chunk[1]]))
                        } else {
                            None
                        }
                    })
                    .collect();
                if let Ok(s) = String::from_utf16(&chars) {
                    return Ok(s);
                }
            }
            // Fallback to Latin-1
            Ok(bytes.iter().map(|&b| b as char).collect())
        }
        lopdf::Object::Array(arr) => {
            let mut text = String::new();
            for item in arr {
                match item {
                    lopdf::Object::String(_, _) => {
                        if let Ok(s) = extract_text_from_operand(doc, item) {
                            text.push_str(&s);
                        }
                    }
                    lopdf::Object::Integer(n) => {
                        // Negative numbers often represent kerning adjustments
                        if *n < -100 {
                            text.push(' ');
                        }
                    }
                    _ => {}
                }
            }
            Ok(text)
        }
        _ => Err(()),
    }
}

fn get_page_dimensions(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> (f64, f64) {
    if let Ok(page) = doc.get_object(page_id) {
        if let Ok(dict) = page.as_dict() {
            if let Ok(media_box) = dict.get(b"MediaBox") {
                if let Ok(arr) = media_box.as_array() {
                    if arr.len() >= 4 {
                        let width = arr[2].as_float().unwrap_or(612.0) as f64;
                        let height = arr[3].as_float().unwrap_or(792.0) as f64;
                        return (width, height);
                    }
                }
            }
        }
    }
    (612.0, 792.0) // Default US Letter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_extractor_name() {
        let extractor = NativeExtractor::new();
        assert_eq!(extractor.name(), "native");
    }

    #[test]
    fn test_quality_check_clean() {
        assert!(NativeExtractor::is_quality_acceptable(
            "This is clean text."
        ));
    }

    #[test]
    fn test_quality_check_empty() {
        assert!(!NativeExtractor::is_quality_acceptable(""));
    }

    #[test]
    fn test_quality_check_garbage() {
        let garbage = "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}";
        assert!(!NativeExtractor::is_quality_acceptable(garbage));
    }
}
