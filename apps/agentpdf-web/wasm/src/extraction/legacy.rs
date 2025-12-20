//! Legacy extraction backend using pdf-extract/lopdf
//!
//! This is the current implementation wrapped as a backend.

use super::types::*;

/// Legacy backend using pdf-extract
pub struct LegacyExtractor;

impl LegacyExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LegacyExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfBackend for LegacyExtractor {
    fn name(&self) -> &'static str {
        "legacy"
    }

    fn can_handle(&self, data: &[u8]) -> bool {
        // Legacy can attempt any PDF
        data.len() > 4 && &data[0..4] == b"%PDF"
    }

    fn extract_sync(&self, data: &[u8]) -> Result<Vec<PageContent>, ExtractionError> {
        // Use existing pdf-extract implementation
        let text = pdf_extract::extract_text_from_mem(data).map_err(|e| {
            let error_str = e.to_string();

            // Check for known encoding failures
            if error_str.contains("Identity-H") || error_str.contains("Unimplemented") {
                ExtractionError::EncodingFailure {
                    details: error_str,
                    recoverable: true,
                }
            } else {
                ExtractionError::ParseError(error_str)
            }
        })?;

        // Get page count for splitting
        let page_count = match pdf_extract::Document::load_mem(data) {
            Ok(doc) => doc.get_pages().len() as u32,
            Err(_) => 1,
        };

        // Legacy doesn't provide per-page extraction easily, so we return all text as page 1
        // or split by form feed characters if present
        let pages = if text.contains('\x0C') {
            // Split by form feed
            text.split('\x0C')
                .enumerate()
                .map(|(i, page_text)| PageContent {
                    page_number: (i + 1) as u32,
                    text_items: vec![], // Legacy doesn't provide spatial info
                    raw_text: page_text.to_string(),
                    width: 612.0, // Default US Letter
                    height: 792.0,
                })
                .collect()
        } else {
            // Return as single page or estimate pages
            vec![PageContent {
                page_number: 1,
                text_items: vec![],
                raw_text: text.clone(),
                width: 612.0,
                height: 792.0,
            }]
        };

        // Pad to match actual page count if needed
        let mut pages = pages;
        while pages.len() < page_count as usize {
            pages.push(PageContent {
                page_number: pages.len() as u32 + 1,
                text_items: vec![],
                raw_text: String::new(),
                width: 612.0,
                height: 792.0,
            });
        }

        Ok(pages)
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
    fn test_legacy_extractor_name() {
        let extractor = LegacyExtractor::new();
        assert_eq!(extractor.name(), "legacy");
    }

    #[test]
    fn test_can_handle_pdf() {
        let extractor = LegacyExtractor::new();
        let pdf_header = b"%PDF-1.4 test";
        assert!(extractor.can_handle(pdf_header));
    }

    #[test]
    fn test_cannot_handle_non_pdf() {
        let extractor = LegacyExtractor::new();
        let not_pdf = b"Not a PDF file";
        assert!(!extractor.can_handle(not_pdf));
    }
}
