//! PDFJoin stress tests
//!
//! Tests for pdfjoin-core with large files and many documents.
//! Uses real demo PDFs from the typst-engine output folder.
//!
//! Run with: cargo test -p benchmark-harness --test pdfjoin_stress -- --nocapture

use lopdf::{content::Content, content::Operation, Dictionary, Document, Object, Stream};
use std::path::PathBuf;
use std::time::Instant;

/// Get the project root directory
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Load demo PDFs from the output folder if they exist
fn load_demo_pdfs() -> Vec<(String, Vec<u8>)> {
    let output_dir = project_root().join("output");
    let mut pdfs = Vec::new();

    let demo_files = [
        "florida_purchase_contract.pdf",
        "florida_listing_agreement.pdf",
        "florida_escalation_addendum.pdf",
    ];

    for filename in demo_files {
        let path = output_dir.join(filename);
        if path.exists() {
            if let Ok(bytes) = std::fs::read(&path) {
                pdfs.push((filename.to_string(), bytes));
            }
        }
    }

    pdfs
}

/// Create a synthetic test PDF with the specified number of pages
fn create_synthetic_pdf(num_pages: u32, content_prefix: &str) -> Vec<u8> {
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
                        format!("{} Page {}", content_prefix, i + 1).into_bytes(),
                        lopdf::StringFormat::Literal,
                    )],
                ),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(Dictionary::new(), content.encode().unwrap()));

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

// ============================================================================
// Stress Tests with Real Demo PDFs
// ============================================================================

#[test]
fn test_merge_real_demo_pdfs() {
    let demo_pdfs = load_demo_pdfs();

    if demo_pdfs.is_empty() {
        eprintln!("SKIPPED: No demo PDFs found in output/");
        eprintln!("Run: cargo run --example generate_florida_realestate --features server");
        return;
    }

    eprintln!("Found {} demo PDFs:", demo_pdfs.len());
    for (name, bytes) in &demo_pdfs {
        eprintln!("  - {} ({} bytes)", name, bytes.len());
    }

    let docs: Vec<Vec<u8>> = demo_pdfs.iter().map(|(_, b)| b.clone()).collect();

    let start = Instant::now();
    let merged = pdfjoin_core::merge_documents(docs).expect("Should merge demo PDFs");
    let elapsed = start.elapsed();

    eprintln!("Merged in {:?}", elapsed);
    eprintln!("Output size: {} bytes", merged.len());

    // Verify the merged PDF is valid
    let doc = Document::load_mem(&merged).expect("Merged PDF should be valid");
    let pages = doc.get_pages();
    eprintln!("Total pages: {}", pages.len());

    assert!(
        pages.len() >= 3,
        "Should have at least 3 pages from demo PDFs"
    );
}

#[test]
fn test_merge_demo_pdfs_multiple_times() {
    let demo_pdfs = load_demo_pdfs();

    if demo_pdfs.is_empty() {
        eprintln!("SKIPPED: No demo PDFs found");
        return;
    }

    // Merge same PDFs 5 times
    let mut docs: Vec<Vec<u8>> = Vec::new();
    for _ in 0..5 {
        for (_, bytes) in &demo_pdfs {
            docs.push(bytes.clone());
        }
    }

    eprintln!("Merging {} documents...", docs.len());

    let start = Instant::now();
    let merged = pdfjoin_core::merge_documents(docs).expect("Should merge repeated PDFs");
    let elapsed = start.elapsed();

    eprintln!("Merged in {:?}", elapsed);
    eprintln!("Output size: {} bytes", merged.len());

    let doc = Document::load_mem(&merged).expect("Should be valid");
    eprintln!("Total pages: {}", doc.get_pages().len());
}

// ============================================================================
// Synthetic Stress Tests
// ============================================================================

#[test]
fn test_merge_10_documents() {
    let docs: Vec<Vec<u8>> = (0..10)
        .map(|i| create_synthetic_pdf(5, &format!("Doc{}", i)))
        .collect();

    let start = Instant::now();
    let merged = pdfjoin_core::merge_documents(docs).expect("Should merge 10 documents");
    let elapsed = start.elapsed();

    eprintln!("Merged 10 documents (50 pages) in {:?}", elapsed);

    let doc = Document::load_mem(&merged).unwrap();
    assert_eq!(doc.get_pages().len(), 50);
}

#[test]
fn test_merge_50_documents() {
    let docs: Vec<Vec<u8>> = (0..50)
        .map(|i| create_synthetic_pdf(2, &format!("Doc{}", i)))
        .collect();

    let start = Instant::now();
    let merged = pdfjoin_core::merge_documents(docs).expect("Should merge 50 documents");
    let elapsed = start.elapsed();

    eprintln!("Merged 50 documents (100 pages) in {:?}", elapsed);
    eprintln!("Output size: {} bytes", merged.len());

    let doc = Document::load_mem(&merged).unwrap();
    assert_eq!(doc.get_pages().len(), 100);

    // Should complete in reasonable time (< 5 seconds)
    assert!(
        elapsed.as_secs() < 5,
        "Merge should complete within 5 seconds"
    );
}

#[test]
fn test_merge_100_single_page_documents() {
    let docs: Vec<Vec<u8>> = (0..100)
        .map(|i| create_synthetic_pdf(1, &format!("Page{}", i)))
        .collect();

    let start = Instant::now();
    let merged = pdfjoin_core::merge_documents(docs).expect("Should merge 100 documents");
    let elapsed = start.elapsed();

    eprintln!("Merged 100 single-page documents in {:?}", elapsed);

    let doc = Document::load_mem(&merged).unwrap();
    assert_eq!(doc.get_pages().len(), 100);
}

#[test]
fn test_large_document_100_pages() {
    let doc = create_synthetic_pdf(100, "LargeDoc");
    eprintln!("Created 100-page document: {} bytes", doc.len());

    // Test splitting various ranges
    let start = Instant::now();
    let split = pdfjoin_core::split_document(&doc, vec![1, 50, 100]).expect("Should split");
    let elapsed = start.elapsed();

    eprintln!("Split 3 pages from 100-page doc in {:?}", elapsed);

    let split_doc = Document::load_mem(&split).unwrap();
    assert_eq!(split_doc.get_pages().len(), 3);
}

#[test]
fn test_large_document_500_pages() {
    let doc = create_synthetic_pdf(500, "VeryLargeDoc");
    eprintln!("Created 500-page document: {} bytes", doc.len());

    // Test splitting all pages
    let pages: Vec<u32> = (1..=500).collect();
    let start = Instant::now();
    let split = pdfjoin_core::split_document(&doc, pages).expect("Should split all 500 pages");
    let elapsed = start.elapsed();

    eprintln!("Split all 500 pages in {:?}", elapsed);

    let split_doc = Document::load_mem(&split).unwrap();
    assert_eq!(split_doc.get_pages().len(), 500);

    assert!(elapsed.as_secs() < 10, "Should complete within 10 seconds");
}

#[test]
fn test_split_every_other_page() {
    let doc = create_synthetic_pdf(100, "EvenOdd");

    // Extract odd pages: 1, 3, 5, ..., 99
    let odd_pages: Vec<u32> = (1..=100).filter(|p| p % 2 == 1).collect();

    let start = Instant::now();
    let split = pdfjoin_core::split_document(&doc, odd_pages).expect("Should split odd pages");
    let elapsed = start.elapsed();

    eprintln!("Split 50 odd pages from 100-page doc in {:?}", elapsed);

    let split_doc = Document::load_mem(&split).unwrap();
    assert_eq!(split_doc.get_pages().len(), 50);
}

#[test]
fn test_merge_then_split_roundtrip() {
    // Create 3 documents
    let doc1 = create_synthetic_pdf(10, "First");
    let doc2 = create_synthetic_pdf(10, "Second");
    let doc3 = create_synthetic_pdf(10, "Third");

    // Merge them
    let merged = pdfjoin_core::merge_documents(vec![doc1, doc2, doc3]).expect("Should merge");

    let merged_doc = Document::load_mem(&merged).unwrap();
    assert_eq!(merged_doc.get_pages().len(), 30);

    // Split out the middle document (pages 11-20)
    let middle_pages: Vec<u32> = (11..=20).collect();
    let split = pdfjoin_core::split_document(&merged, middle_pages).expect("Should split middle");

    let split_doc = Document::load_mem(&split).unwrap();
    assert_eq!(split_doc.get_pages().len(), 10);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_split_single_page_from_large_doc() {
    let doc = create_synthetic_pdf(1000, "Huge");
    eprintln!("Created 1000-page document: {} bytes", doc.len());

    let start = Instant::now();
    let split = pdfjoin_core::split_document(&doc, vec![500]).expect("Should split page 500");
    let elapsed = start.elapsed();

    eprintln!("Extracted single page from 1000-page doc in {:?}", elapsed);

    let split_doc = Document::load_mem(&split).unwrap();
    assert_eq!(split_doc.get_pages().len(), 1);
}

#[test]
fn test_merge_identical_documents() {
    let doc = create_synthetic_pdf(5, "Identical");

    // Merge the same document 10 times
    let docs: Vec<Vec<u8>> = (0..10).map(|_| doc.clone()).collect();

    let merged = pdfjoin_core::merge_documents(docs).expect("Should merge identical docs");

    let merged_doc = Document::load_mem(&merged).unwrap();
    assert_eq!(merged_doc.get_pages().len(), 50);
}

#[test]
fn test_parse_ranges_stress() {
    // Test parsing complex ranges
    let complex_range = "1-10, 15, 20-30, 35, 40-50, 55, 60-70, 75, 80-90, 95, 100";
    let pages = pdfjoin_core::parse_ranges(complex_range).expect("Should parse complex range");

    assert!(!pages.is_empty());
    eprintln!("Parsed {} pages from complex range", pages.len());
}
