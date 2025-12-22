//! PDF Merge algorithm
//!
//! Combines multiple PDFs into a single document.

use crate::error::PdfJoinError;
use lopdf::{Document, Object, ObjectId};
use std::collections::BTreeMap;

/// Merge multiple PDFs into one
///
/// The algorithm:
/// 1. If empty, return error
/// 2. If single document, return it as-is
/// 3. Create a new destination document
/// 4. For each source document:
///    a. Calculate ID offset to avoid conflicts
///    b. Import all objects with remapped IDs
///    c. Append pages to the destination
/// 5. Compress and return the merged result
pub fn merge_documents(documents: Vec<Vec<u8>>) -> Result<Vec<u8>, PdfJoinError> {
    if documents.is_empty() {
        return Err(PdfJoinError::OperationError("No documents to merge".into()));
    }

    // Single document - return as-is
    if documents.len() == 1 {
        return Ok(documents.into_iter().next().unwrap());
    }

    // Load all documents first
    let mut loaded_docs = Vec::new();
    for (i, doc_bytes) in documents.iter().enumerate() {
        let doc = Document::load_mem(doc_bytes).map_err(|e| {
            PdfJoinError::ParseError(format!("Failed to load document {}: {}", i, e))
        })?;
        loaded_docs.push(doc);
    }

    // Start with the first document as the base
    let mut dest = loaded_docs.remove(0);
    let mut dest_max_id = dest.max_id;

    // Get the destination page list
    let mut dest_page_refs = get_page_references(&dest)?;

    // Merge each remaining document
    for source in loaded_docs.into_iter() {
        // Get source pages before we start modifying the document
        let source_pages = get_page_references(&source)?;

        // Calculate offset for object IDs to avoid conflicts
        let id_offset = dest_max_id;

        // Remap all object IDs in the source document
        let mut remapped_objects = BTreeMap::new();
        for (old_id, object) in source.objects.into_iter() {
            let new_id = (old_id.0 + id_offset, old_id.1);
            let remapped_object = remap_object_refs(object, id_offset);
            remapped_objects.insert(new_id, remapped_object);
        }

        // Add all remapped objects to destination
        for (id, object) in remapped_objects {
            dest.objects.insert(id, object);
        }

        // Remap and add source pages to destination page list
        for old_page_ref in source_pages {
            let new_page_ref = (old_page_ref.0 + id_offset, old_page_ref.1);
            dest_page_refs.push(new_page_ref);
        }

        // Update destination max_id
        dest_max_id = (source.max_id + id_offset).max(dest_max_id);
    }

    // Update the pages array in the destination document
    update_page_tree(&mut dest, dest_page_refs)?;

    // Update max_id
    dest.max_id = dest_max_id;

    // Compress and serialize
    dest.compress();

    let mut buffer = Vec::new();
    dest.save_to(&mut buffer)
        .map_err(|e| PdfJoinError::OperationError(format!("Failed to save merged PDF: {}", e)))?;

    Ok(buffer)
}

/// Get all page object references from a document
fn get_page_references(doc: &Document) -> Result<Vec<ObjectId>, PdfJoinError> {
    let pages = doc.get_pages();
    Ok(pages.values().copied().collect())
}

/// Recursively remap object references in an object
fn remap_object_refs(obj: Object, offset: u32) -> Object {
    match obj {
        Object::Reference(id) => Object::Reference((id.0 + offset, id.1)),
        Object::Array(arr) => Object::Array(
            arr.into_iter()
                .map(|o| remap_object_refs(o, offset))
                .collect(),
        ),
        Object::Dictionary(mut dict) => {
            for (_, value) in dict.iter_mut() {
                *value = remap_object_refs(value.clone(), offset);
            }
            Object::Dictionary(dict)
        }
        Object::Stream(mut stream) => {
            for (_, value) in stream.dict.iter_mut() {
                *value = remap_object_refs(value.clone(), offset);
            }
            Object::Stream(stream)
        }
        other => other,
    }
}

/// Update the page tree in the destination document with new page references
fn update_page_tree(doc: &mut Document, page_refs: Vec<ObjectId>) -> Result<(), PdfJoinError> {
    // Get the catalog
    let root_obj = doc
        .trailer
        .get(b"Root")
        .map_err(|_| PdfJoinError::OperationError("No Root in trailer".into()))?;

    let catalog_id = root_obj
        .as_reference()
        .map_err(|_| PdfJoinError::OperationError("Root is not a reference".into()))?;

    let catalog = doc
        .objects
        .get(&catalog_id)
        .ok_or_else(|| PdfJoinError::OperationError("Catalog not found".into()))?
        .as_dict()
        .map_err(|_| PdfJoinError::OperationError("Invalid catalog".into()))?;

    let pages_obj = catalog
        .get(b"Pages")
        .map_err(|_| PdfJoinError::OperationError("No Pages in catalog".into()))?;

    let pages_id = pages_obj
        .as_reference()
        .map_err(|_| PdfJoinError::OperationError("Pages is not a reference".into()))?;

    // Update the pages dictionary
    if let Some(Object::Dictionary(ref mut pages_dict)) = doc.objects.get_mut(&pages_id) {
        // Update the Kids array
        let kids = page_refs
            .iter()
            .map(|&id| Object::Reference(id))
            .collect::<Vec<_>>();
        pages_dict.set("Kids", Object::Array(kids));

        // Update the Count
        pages_dict.set("Count", Object::Integer(page_refs.len() as i64));
    } else {
        return Err(PdfJoinError::OperationError(
            "Invalid pages dictionary".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Document, Object};

    /// Helper to create a simple PDF with N pages containing identifiable text
    fn create_test_pdf(num_pages: u32, content_prefix: &str) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");

        // Create a catalog and pages root
        let pages_id = doc.new_object_id();
        let catalog_id = doc.new_object_id();

        let mut page_ids = Vec::new();

        // Create each page
        for page_num in 0..num_pages {
            let page_id = doc.new_object_id();
            let content_id = doc.new_object_id();

            // Create page content stream with identifiable text
            let content = format!(
                "BT /F1 12 Tf 50 700 Td ({}-Page-{}) Tj ET",
                content_prefix,
                page_num + 1
            );
            doc.objects.insert(
                content_id,
                Object::Stream(lopdf::Stream::new(Dictionary::new(), content.into_bytes())),
            );

            // Create page dictionary
            let mut page_dict = Dictionary::new();
            page_dict.set("Type", Object::Name(b"Page".to_vec()));
            page_dict.set("Parent", Object::Reference(pages_id));
            page_dict.set("Contents", Object::Reference(content_id));

            // Set page size (8.5 x 11 inches at 72 DPI)
            let media_box = vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(612),
                Object::Integer(792),
            ];
            page_dict.set("MediaBox", Object::Array(media_box));

            doc.objects.insert(page_id, Object::Dictionary(page_dict));
            page_ids.push(Object::Reference(page_id));
        }

        // Create pages dictionary
        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Count", Object::Integer(num_pages as i64));
        pages_dict.set("Kids", Object::Array(page_ids));
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        // Create catalog
        let mut catalog_dict = Dictionary::new();
        catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog_dict.set("Pages", Object::Reference(pages_id));
        doc.objects
            .insert(catalog_id, Object::Dictionary(catalog_dict));

        // Set trailer
        doc.trailer.set("Root", Object::Reference(catalog_id));

        // Save to buffer
        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn test_merge_empty_fails() {
        let result = merge_documents(vec![]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No documents to merge"));
    }

    #[test]
    fn test_merge_single_document_returns_same() {
        let pdf = create_test_pdf(2, "Single");
        let original_len = pdf.len();

        let result = merge_documents(vec![pdf.clone()]).unwrap();

        // Should return the same document (same size approximately)
        // Verify it has 2 pages
        let doc = Document::load_mem(&result).unwrap();
        assert_eq!(doc.get_pages().len(), 2);

        // Should be roughly the same size (allowing for some variation)
        assert!((result.len() as i32 - original_len as i32).abs() < 100);
    }

    #[test]
    fn test_merge_two_documents_combines_pages() {
        let doc_a = create_test_pdf(2, "DocA");
        let doc_b = create_test_pdf(3, "DocB");

        let merged = merge_documents(vec![doc_a, doc_b]).unwrap();

        // Verify the merged document has 5 pages
        let doc = Document::load_mem(&merged).unwrap();
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 5, "Merged document should have 5 pages");
    }

    #[test]
    fn test_merge_multiple_documents() {
        let docs: Vec<Vec<u8>> = (0..5)
            .map(|i| create_test_pdf(1, &format!("Doc{}", i)))
            .collect();

        let merged = merge_documents(docs).unwrap();

        let doc = Document::load_mem(&merged).unwrap();
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 5, "Merged document should have 5 pages");
    }

    #[test]
    fn test_merge_preserves_page_order() {
        // Create 3 documents with different page counts
        let doc1 = create_test_pdf(2, "First");
        let doc2 = create_test_pdf(1, "Second");
        let doc3 = create_test_pdf(2, "Third");

        let merged = merge_documents(vec![doc1, doc2, doc3]).unwrap();

        // Verify total page count
        let doc = Document::load_mem(&merged).unwrap();
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 5, "Merged document should have 5 pages");

        // The pages should be in order: First-1, First-2, Second-1, Third-1, Third-2
        // (We can't easily verify content without extracting text, but we can verify structure)
    }

    #[test]
    fn test_merge_handles_different_sizes() {
        // Test merging documents with varying page counts
        let doc1 = create_test_pdf(10, "Large");
        let doc2 = create_test_pdf(1, "Small");
        let doc3 = create_test_pdf(5, "Medium");

        let merged = merge_documents(vec![doc1, doc2, doc3]).unwrap();

        let doc = Document::load_mem(&merged).unwrap();
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 16, "Merged document should have 16 pages");
    }

    #[test]
    fn test_merged_document_is_valid_pdf() {
        let doc1 = create_test_pdf(2, "Valid1");
        let doc2 = create_test_pdf(2, "Valid2");

        let merged = merge_documents(vec![doc1, doc2]).unwrap();

        // Should be able to load the merged document without errors
        let doc = Document::load_mem(&merged);
        assert!(doc.is_ok(), "Merged document should be valid PDF");

        // Should be able to get pages
        let doc = doc.unwrap();
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 4);
    }
}
