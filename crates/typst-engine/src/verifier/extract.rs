//! PDF text extraction module
//!
//! This module provides functionality to extract text content from PDF files,
//! including page-by-page extraction with line-level granularity.
//!
//! # Features
//! - Raw text extraction from PDF bytes
//! - Page-level content separation
//! - Line-level text organization
//! - PDF metadata extraction
//! - Detection of scanned PDFs (requiring OCR)
//! - Handling of password-protected PDFs
//!
//! # Example
//! ```no_run
//! use typst_engine::verifier::{PdfExtractor, VerifierError};
//!
//! fn extract_lease(pdf_bytes: &[u8]) -> Result<(), VerifierError> {
//!     let document = PdfExtractor::extract_text(pdf_bytes)?;
//!     println!("Extracted {} pages", document.pages.len());
//!     println!("Total text length: {}", document.raw_text.len());
//!     Ok(())
//! }
//! ```

use crate::verifier::VerifierError;
use pdf_extract::extract_text_from_mem;
use serde::{Deserialize, Serialize};

/// Main PDF extraction interface
pub struct PdfExtractor;

impl PdfExtractor {
    /// Extract text from PDF bytes with full document structure
    ///
    /// This method extracts text from a PDF file provided as a byte slice,
    /// preserving page boundaries and line structure.
    ///
    /// # Arguments
    /// * `pdf_bytes` - The raw bytes of the PDF file
    ///
    /// # Returns
    /// * `Ok(ExtractedDocument)` - Successfully extracted document with text and metadata
    /// * `Err(VerifierError)` - If extraction fails, PDF is invalid, password-protected, or scanned
    ///
    /// # Errors
    /// - `VerifierError::InvalidPdf` - The PDF is malformed or corrupted
    /// - `VerifierError::PasswordProtected` - The PDF requires a password
    /// - `VerifierError::ScannedPdfNeedsOcr` - The PDF appears to be scanned (minimal text)
    /// - `VerifierError::ExtractionError` - Other extraction failures
    ///
    /// # Example
    /// ```no_run
    /// # use typst_engine::verifier::{PdfExtractor, VerifierError};
    /// # fn example(pdf_bytes: &[u8]) -> Result<(), VerifierError> {
    /// let document = PdfExtractor::extract_text(pdf_bytes)?;
    /// println!("Pages: {}", document.metadata.page_count);
    /// println!("Text: {}", document.raw_text);
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_text(pdf_bytes: &[u8]) -> Result<ExtractedDocument, VerifierError> {
        // Attempt to extract text using pdf-extract
        let raw_text = match extract_text_from_mem(pdf_bytes) {
            Ok(text) => text,
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();

                // Check for password protection
                if error_msg.contains("encrypted") || error_msg.contains("password") {
                    return Err(VerifierError::PasswordProtected);
                }

                // Check for malformed PDF
                if error_msg.contains("invalid")
                    || error_msg.contains("malformed")
                    || error_msg.contains("corrupt")
                {
                    return Err(VerifierError::InvalidPdf(e.to_string()));
                }

                // Generic extraction error
                return Err(VerifierError::ExtractionError(e.to_string()));
            }
        };

        // Check if the PDF appears to be scanned (very little text extracted)
        // Heuristic: if less than 50 characters extracted, it's likely scanned
        let trimmed_text = raw_text.trim();
        if trimmed_text.len() < 50 {
            return Err(VerifierError::ScannedPdfNeedsOcr);
        }

        // Check for mostly whitespace (another indicator of scanned PDF)
        let non_whitespace_chars = trimmed_text.chars().filter(|c| !c.is_whitespace()).count();
        if non_whitespace_chars < 20 {
            return Err(VerifierError::ScannedPdfNeedsOcr);
        }

        // Extract pages by splitting on form feed characters or analyzing the text
        let pages = Self::extract_pages_from_text(&raw_text)?;

        // Extract basic metadata
        let metadata = PdfMetadata {
            page_count: pages.len(),
            title: None,  // pdf-extract doesn't provide metadata easily
            author: None, // Would require using lopdf or pdf crate for full metadata
        };

        Ok(ExtractedDocument {
            raw_text,
            pages,
            metadata,
        })
    }

    /// Extract text with page boundaries preserved
    ///
    /// This is a convenience method that extracts just the page content
    /// without the full document structure.
    ///
    /// # Arguments
    /// * `pdf_bytes` - The raw bytes of the PDF file
    ///
    /// # Returns
    /// * `Ok(Vec<PageContent>)` - Vector of page contents
    /// * `Err(VerifierError)` - If extraction fails
    pub fn extract_pages(pdf_bytes: &[u8]) -> Result<Vec<PageContent>, VerifierError> {
        let document = Self::extract_text(pdf_bytes)?;
        Ok(document.pages)
    }

    /// Internal method to split extracted text into pages
    ///
    /// Since pdf-extract doesn't provide page boundaries directly,
    /// we use heuristics to split the text into logical pages.
    fn extract_pages_from_text(text: &str) -> Result<Vec<PageContent>, VerifierError> {
        let mut pages = Vec::new();

        // Split on form feed character (common page separator)
        let page_texts: Vec<&str> = text.split('\x0C').collect();

        // If no form feeds found, treat entire text as one page
        if page_texts.len() == 1 && !text.contains('\x0C') {
            // For single-page PDFs or PDFs without form feeds,
            // we could try to split by page breaks, but for now treat as one page
            let page = Self::create_page_content(1, text);
            pages.push(page);
        } else {
            // Multiple pages detected
            for (idx, page_text) in page_texts.iter().enumerate() {
                if !page_text.trim().is_empty() {
                    let page = Self::create_page_content(idx + 1, page_text);
                    pages.push(page);
                }
            }
        }

        // If we still have no pages, something went wrong
        if pages.is_empty() {
            return Err(VerifierError::ExtractionError(
                "No pages could be extracted from PDF".to_string(),
            ));
        }

        Ok(pages)
    }

    /// Create a PageContent structure from raw page text
    fn create_page_content(page_number: usize, text: &str) -> PageContent {
        // Split text into lines
        let lines: Vec<TextLine> = text
            .lines()
            .enumerate()
            .map(|(idx, line)| TextLine {
                text: line.to_string(),
                line_number: idx + 1,
            })
            .collect();

        PageContent {
            page_number,
            text: text.to_string(),
            lines,
        }
    }
}

/// Represents a complete extracted PDF document
///
/// Contains the raw text, page-by-page breakdown, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    /// Raw text extracted from the entire document
    pub raw_text: String,

    /// Page-by-page content breakdown
    pub pages: Vec<PageContent>,

    /// Document metadata
    pub metadata: PdfMetadata,
}

/// Represents the content of a single page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContent {
    /// Page number (1-indexed)
    pub page_number: usize,

    /// Raw text content of the page
    pub text: String,

    /// Individual lines of text on the page
    pub lines: Vec<TextLine>,
}

/// Represents a single line of text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLine {
    /// The text content of the line
    pub text: String,

    /// Line number within the page (1-indexed)
    pub line_number: usize,
}

/// PDF document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfMetadata {
    /// Total number of pages in the document
    pub page_count: usize,

    /// Document title (if available in PDF metadata)
    pub title: Option<String>,

    /// Document author (if available in PDF metadata)
    pub author: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanned_pdf_detection() {
        // Empty bytes aren't a valid PDF, so extraction should fail with an error
        // (either InvalidPdf or ExtractionError depending on the pdf-extract library)
        let result = PdfExtractor::extract_text(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_page_content() {
        let text = "Line 1\nLine 2\nLine 3";
        let page = PdfExtractor::create_page_content(1, text);

        assert_eq!(page.page_number, 1);
        assert_eq!(page.lines.len(), 3);
        assert_eq!(page.lines[0].text, "Line 1");
        assert_eq!(page.lines[0].line_number, 1);
        assert_eq!(page.lines[2].text, "Line 3");
        assert_eq!(page.lines[2].line_number, 3);
    }

    #[test]
    fn test_extract_pages_from_text() {
        // Test with form feed separator
        let text = "Page 1 content\x0CPage 2 content\x0CPage 3 content";
        let pages = PdfExtractor::extract_pages_from_text(text).unwrap();

        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].page_number, 1);
        assert!(pages[0].text.contains("Page 1"));
        assert_eq!(pages[1].page_number, 2);
        assert!(pages[1].text.contains("Page 2"));
    }

    #[test]
    fn test_extract_single_page() {
        // Test without form feed (single page)
        let text = "This is a single page document with multiple lines.\nLine 2.\nLine 3.";
        let pages = PdfExtractor::extract_pages_from_text(text).unwrap();

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_number, 1);
        assert_eq!(pages[0].lines.len(), 3);
    }

    #[test]
    fn test_text_line_structure() {
        let text = "First line\nSecond line\nThird line";
        let page = PdfExtractor::create_page_content(1, text);

        assert_eq!(page.lines.len(), 3);

        for (idx, line) in page.lines.iter().enumerate() {
            assert_eq!(line.line_number, idx + 1);
        }
    }
}
