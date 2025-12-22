//! PDF Split algorithm
//!
//! Extracts pages from a PDF using a two-tier approach:
//! 1. Try fast streaming (byte-level) first - 3-5x faster for most PDFs
//! 2. Fall back to lopdf (full parse) for edge cases

use crate::error::PdfJoinError;
use crate::streaming;
use lopdf::Document;
use std::collections::HashSet;

/// Split a PDF, extracting only the specified pages (1-indexed)
///
/// Uses streaming (byte-level) approach first for speed, with lopdf fallback
/// for compatibility with edge-case PDF formats.
pub fn split_document(bytes: &[u8], pages: Vec<u32>) -> Result<Vec<u8>, PdfJoinError> {
    if pages.is_empty() {
        return Err(PdfJoinError::InvalidRange("No pages specified".into()));
    }

    // Validate page numbers are > 0
    if pages.contains(&0) {
        return Err(PdfJoinError::InvalidRange(
            "Page numbers must be >= 1".into(),
        ));
    }

    // Try fast streaming approach first (works for 99% of real-world PDFs)
    match streaming::split_streaming(bytes, pages.clone()) {
        Ok(result) => return Ok(result),
        Err(_) => {
            // Fall back to lopdf for edge cases (e.g., xref streams, unusual formats)
        }
    }

    // Fallback: Full parse with lopdf
    split_document_lopdf(bytes, pages)
}

/// Split using lopdf (full parse) - slower but handles all PDF formats
fn split_document_lopdf(bytes: &[u8], pages: Vec<u32>) -> Result<Vec<u8>, PdfJoinError> {
    let doc = Document::load_mem(bytes).map_err(|e| PdfJoinError::ParseError(e.to_string()))?;

    let page_count = doc.get_pages().len() as u32;

    // Validate all page numbers exist
    for &page in &pages {
        if page > page_count {
            return Err(PdfJoinError::InvalidRange(format!(
                "Page {} does not exist (document has {} pages)",
                page, page_count
            )));
        }
    }

    // Clone the document for modification
    let mut new_doc = doc.clone();

    // Calculate pages to keep and delete
    let pages_to_keep: HashSet<u32> = pages.iter().copied().collect();
    let mut pages_to_delete: Vec<u32> = (1..=page_count)
        .filter(|p| !pages_to_keep.contains(p))
        .collect();

    // Delete unwanted pages (must delete in reverse order to maintain indices)
    pages_to_delete.reverse();
    for page_num in pages_to_delete {
        new_doc.delete_pages(&[page_num]);
    }

    // Compress to remove orphaned objects
    new_doc.prune_objects();
    new_doc.compress();

    // Serialize
    let mut buffer = Vec::new();
    new_doc
        .save_to(&mut buffer)
        .map_err(|e| PdfJoinError::OperationError(format!("Save failed: {}", e)))?;

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, content::Operation, Dictionary, Document, Object, Stream};

    // Helper to create a simple PDF with N pages
    fn create_test_pdf(num_pages: u32) -> Vec<u8> {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();

        let mut page_ids = Vec::new();

        for i in 0..num_pages {
            // Create content stream for page
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

            // Create page object
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

        // Create pages dictionary
        let pages = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Pages".to_vec())),
            ("Count", Object::Integer(num_pages as i64)),
            (
                "Kids",
                Object::Array(page_ids.iter().map(|id| Object::Reference(*id)).collect()),
            ),
        ]);
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        // Create catalog
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
    fn test_split_empty_pages_fails() {
        let pdf = create_test_pdf(5);
        let result = split_document(&pdf, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_split_extracts_single_page() {
        let pdf = create_test_pdf(5);
        let result = split_document(&pdf, vec![1]).unwrap();
        let doc = Document::load_mem(&result).unwrap();
        assert_eq!(doc.get_pages().len(), 1);
    }

    #[test]
    fn test_split_extracts_multiple_pages() {
        let pdf = create_test_pdf(5);
        let result = split_document(&pdf, vec![1, 3, 5]).unwrap();
        let doc = Document::load_mem(&result).unwrap();
        assert_eq!(doc.get_pages().len(), 3);
    }

    #[test]
    fn test_split_extracts_range() {
        let pdf = create_test_pdf(10);
        let result = split_document(&pdf, vec![2, 3, 4, 5]).unwrap();
        let doc = Document::load_mem(&result).unwrap();
        assert_eq!(doc.get_pages().len(), 4);
    }

    #[test]
    fn test_split_invalid_page_number_fails() {
        let pdf = create_test_pdf(5);
        let result = split_document(&pdf, vec![10]); // Page 10 doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_split_page_zero_fails() {
        let pdf = create_test_pdf(5);
        let result = split_document(&pdf, vec![0]); // Pages are 1-indexed
        assert!(result.is_err());
    }

    /// Regression test: splitting a range from a larger PDF must produce valid output
    /// This specifically tests the streaming implementation which had a bug where
    /// the xref table was malformed.
    #[test]
    fn test_split_range_from_larger_pdf_produces_valid_output() {
        // Create a 17-page PDF (similar to florida_purchase_contract.pdf)
        let pdf = create_test_pdf(17);

        // Split pages 5-17 (the exact range that failed in production)
        let pages: Vec<u32> = (5..=17).collect();
        let result = split_document(&pdf, pages.clone()).expect("split should succeed");

        // The output must be parseable by lopdf
        let doc = Document::load_mem(&result).expect("output PDF must be valid and parseable");

        // Verify correct page count
        assert_eq!(
            doc.get_pages().len(),
            13,
            "split output should have 13 pages (5-17)"
        );
    }

    /// Integration test with real PDF file (if available)
    /// This tests the actual bug: splitting pages 5-17 from florida_purchase_contract.pdf
    #[test]
    fn test_split_real_pdf_produces_valid_output() {
        use std::path::PathBuf;

        // Find project root
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let pdf_path = project_root.join("output/florida_purchase_contract.pdf");

        if !pdf_path.exists() {
            eprintln!("SKIP: Real PDF not found at {:?}", pdf_path);
            return;
        }

        let pdf_bytes = std::fs::read(&pdf_path).expect("read PDF");
        eprintln!("Loaded real PDF: {} bytes", pdf_bytes.len());

        // Get page count
        let page_count = crate::get_page_count(&pdf_bytes).expect("get page count");
        eprintln!("Page count: {}", page_count);
        assert_eq!(
            page_count, 17,
            "florida_purchase_contract should have 17 pages"
        );

        // Split pages 5-17 (the exact operation that failed)
        let pages: Vec<u32> = (5..=17).collect();
        let result = split_document(&pdf_bytes, pages).expect("split should succeed");
        eprintln!("Split result: {} bytes", result.len());

        // The critical test: output must be parseable by lopdf
        let doc = Document::load_mem(&result)
            .expect("REGRESSION: split output is corrupt and cannot be parsed");

        assert_eq!(
            doc.get_pages().len(),
            13,
            "split should produce 13 pages (5-17)"
        );

        eprintln!(
            "SUCCESS: Split output is valid PDF with {} pages",
            doc.get_pages().len()
        );
    }

    /// Test streaming split with real PDF (the actual implementation being tested)
    #[test]
    fn test_streaming_split_real_pdf() {
        use crate::streaming;
        use std::path::PathBuf;

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let pdf_path = project_root.join("output/florida_purchase_contract.pdf");

        if !pdf_path.exists() {
            eprintln!("SKIP: Real PDF not found");
            return;
        }

        let pdf_bytes = std::fs::read(&pdf_path).expect("read PDF");

        // This is the streaming implementation that's used by default
        let pages: Vec<u32> = (5..=17).collect();
        let result = streaming::split_streaming(&pdf_bytes, pages)
            .expect("streaming split should work for real PDFs");

        // Write to temp file for manual inspection if needed
        let temp_path = std::env::temp_dir().join("pdfjoin_test_streaming_split.pdf");
        std::fs::write(&temp_path, &result).expect("write temp file");
        eprintln!("Wrote streaming output to: {:?}", temp_path);

        // Output MUST be valid
        let doc = Document::load_mem(&result).expect("REGRESSION: streaming output is corrupt");

        assert_eq!(doc.get_pages().len(), 13);
    }

    /// Test split pages 4-14 and write to /tmp for manual Preview inspection
    /// This test specifically checks that page CONTENT is preserved, not just structure
    #[test]
    fn test_split_4_14_preserves_content() {
        use std::path::PathBuf;

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let pdf_path = project_root.join("output/florida_purchase_contract.pdf");

        if !pdf_path.exists() {
            eprintln!("SKIP: Real PDF not found at {:?}", pdf_path);
            return;
        }

        let pdf_bytes = std::fs::read(&pdf_path).expect("read PDF");
        eprintln!("Input PDF: {} bytes", pdf_bytes.len());

        // Split pages 4-14 (11 pages)
        let pages: Vec<u32> = (4..=14).collect();
        eprintln!("Splitting pages: {:?}", pages);

        let result = split_document(&pdf_bytes, pages).expect("split should succeed");
        eprintln!("Output PDF: {} bytes", result.len());

        // Write to /tmp for manual inspection
        let output_path = "/tmp/split_4_14_test.pdf";
        std::fs::write(output_path, &result).expect("write output");
        eprintln!("Written to: {}", output_path);
        eprintln!("Run: open -a Preview {}", output_path);

        // Verify structure
        let doc = Document::load_mem(&result).expect("output must be parseable");
        assert_eq!(doc.get_pages().len(), 11, "should have 11 pages");

        // Check that output has reasonable size (not empty/stripped)
        // Original is ~71KB for 17 pages, so 11 pages should be roughly 40-50KB
        // If content is stripped, output will be much smaller (< 5KB)
        assert!(
            result.len() > 10000,
            "Output too small ({} bytes) - content may be stripped!",
            result.len()
        );

        eprintln!("Output size check passed: {} bytes", result.len());
    }

    /// Regression test: streaming split must NOT produce duplicate Pages objects
    /// Bug: The streaming split was including the original Catalog and Pages objects
    /// AND creating new ones, resulting in two /Count entries which corrupts the PDF.
    /// macOS Preview and other strict PDF readers reject such files.
    #[test]
    fn test_streaming_split_no_duplicate_pages_objects() {
        use crate::streaming;
        use std::path::PathBuf;

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let pdf_path = project_root.join("output/florida_purchase_contract.pdf");

        if !pdf_path.exists() {
            eprintln!("SKIP: Real PDF not found");
            return;
        }

        let pdf_bytes = std::fs::read(&pdf_path).expect("read PDF");
        let pages: Vec<u32> = (5..=17).collect();
        let result =
            streaming::split_streaming(&pdf_bytes, pages).expect("streaming split should succeed");

        // Convert to string for pattern matching (safe for this test)
        let result_str = String::from_utf8_lossy(&result);

        // Count occurrences of "/Count " pattern (the Pages object marker)
        let count_occurrences = result_str.matches("/Count ").count();

        // There should be exactly ONE /Count entry (our new Pages object)
        // If there are multiple, the original Pages object was incorrectly included
        assert_eq!(
            count_occurrences, 1,
            "REGRESSION: Found {} /Count entries, expected 1. \
             Duplicate Pages objects cause PDF corruption.",
            count_occurrences
        );
    }

    /// Regression test: Page objects must have /Parent pointing to a valid Pages object
    /// Bug: After splitting, page objects still referenced the original Pages object ID
    /// which no longer exists in the output PDF. This causes macOS Preview to reject the file.
    #[test]
    fn test_streaming_split_pages_have_valid_parent() {
        use crate::streaming;
        use regex::Regex;
        use std::path::PathBuf;

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let pdf_path = project_root.join("output/florida_purchase_contract.pdf");

        if !pdf_path.exists() {
            eprintln!("SKIP: Real PDF not found");
            return;
        }

        let pdf_bytes = std::fs::read(&pdf_path).expect("read PDF");
        let pages: Vec<u32> = (5..=17).collect();
        let result =
            streaming::split_streaming(&pdf_bytes, pages).expect("streaming split should succeed");

        let result_str = String::from_utf8_lossy(&result);

        // Find the Pages object ID (the one with /Type /Pages and /Kids)
        let pages_obj_re = Regex::new(r"(\d+) 0 obj\s*<<[^>]*?/Type /Pages[^>]*?/Kids").unwrap();
        let pages_obj_id = pages_obj_re
            .captures(&result_str)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .expect("Should find Pages object");

        eprintln!("Pages object ID: {}", pages_obj_id);

        // Find all /Parent references in page objects
        // Note: Only Page objects should have /Parent pointing to the Pages object.
        // Other objects (like annotations) might have /Parent pointing elsewhere.
        let parent_re = Regex::new(r"/Parent (\d+) 0 R").unwrap();

        // First, find all /Type /Page objects and their parent refs
        let page_obj_re =
            Regex::new(r"(\d+) 0 obj[^>]*?/Type /Page[^s][^>]*?/Parent (\d+) 0 R").unwrap();

        for cap in page_obj_re.captures_iter(&result_str) {
            let obj_id = cap.get(1).unwrap().as_str();
            let parent_id = cap.get(2).unwrap().as_str();
            if parent_id != pages_obj_id {
                eprintln!(
                    "Page object {} has /Parent {} (expected {})",
                    obj_id, parent_id, pages_obj_id
                );
            }
            assert_eq!(
                parent_id, pages_obj_id,
                "REGRESSION: Page object {} has /Parent {} but Pages object is {}. \
                 Invalid parent references cause PDF corruption.",
                obj_id, parent_id, pages_obj_id
            );
        }

        // Also check all /Parent refs to see what's pointing to non-Pages objects
        eprintln!("\nAll /Parent references in output:");
        for cap in parent_re.captures_iter(&result_str) {
            let parent_id = cap.get(1).unwrap().as_str();
            if parent_id != pages_obj_id {
                // Find context around this parent ref
                let pos = cap.get(0).unwrap().start();
                let start = pos.saturating_sub(100);
                let end = (pos + 100).min(result_str.len());
                eprintln!(
                    "  Non-Pages parent {} at pos {}: ...{}...",
                    parent_id,
                    pos,
                    &result_str[start..end].replace('\n', "\\n")
                );
            }
        }
    }

    /// Test that get_page_count works with create_test_pdf generated PDFs
    #[test]
    fn test_get_page_count_with_generated_pdf() {
        let pdf = create_test_pdf(7);
        let result = crate::get_page_count(&pdf);
        assert!(
            result.is_ok(),
            "get_page_count should work. Error: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), 7);
    }
}
