//! PDF Split algorithm
//!
//! Extracts pages from a PDF using "Construction by Whitelist".

use crate::error::PdfJoinError;
use lopdf::Document;
use std::collections::HashSet;

/// Split a PDF, extracting only the specified pages (1-indexed)
///
/// Uses "Construction by Whitelist" algorithm:
/// 1. Identify target page objects
/// 2. Traverse dependency graph to find all required resources
/// 3. Build new document with only required objects
/// 4. Rebuild page tree
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
}
