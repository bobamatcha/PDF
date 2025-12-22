//! PDFJoin benchmark: lopdf vs streaming implementation
//!
//! Compares performance of:
//! - lopdf (full parse) vs streaming (byte-level)
//!
//! Run with: cargo test -p benchmark-harness --test pdfjoin_benchmark -- --nocapture

use lopdf::{content::Content, content::Operation, Dictionary, Document, Object, Stream};
use std::path::PathBuf;
use std::time::{Duration, Instant};

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
fn create_synthetic_pdf(num_pages: u32) -> Vec<u8> {
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

/// Benchmark result
struct BenchResult {
    name: String,
    lopdf_time: Duration,
    streaming_time: Duration,
    speedup: f64,
}

impl BenchResult {
    fn print(&self) {
        eprintln!(
            "{:<40} lopdf: {:>10.2?}  streaming: {:>10.2?}  speedup: {:>6.1}x",
            self.name, self.lopdf_time, self.streaming_time, self.speedup
        );
    }
}

fn bench_split(pdf: &[u8], pages: Vec<u32>, name: &str) -> BenchResult {
    // Warm up
    let _ = pdfjoin_core::split_document(pdf, pages.clone());

    // Benchmark lopdf
    let start = Instant::now();
    let _ = pdfjoin_core::split_document(pdf, pages.clone());
    let lopdf_time = start.elapsed();

    // Benchmark streaming
    let start = Instant::now();
    let streaming_result = pdfjoin_core::split_streaming(pdf, pages.clone());
    let streaming_time = start.elapsed();

    let speedup = lopdf_time.as_secs_f64() / streaming_time.as_secs_f64().max(0.000001);

    // Verify streaming produces valid PDF (if it succeeded)
    if let Ok(result) = &streaming_result {
        if !result.starts_with(b"%PDF-") {
            eprintln!("WARNING: streaming output is not valid PDF for {}", name);
        }
    } else {
        eprintln!(
            "WARNING: streaming failed for {}: {:?}",
            name,
            streaming_result.err()
        );
    }

    BenchResult {
        name: name.to_string(),
        lopdf_time,
        streaming_time,
        speedup,
    }
}

fn bench_merge(pdfs: Vec<Vec<u8>>, name: &str) -> BenchResult {
    // Warm up
    let _ = pdfjoin_core::merge_documents(pdfs.clone());

    // Benchmark lopdf
    let start = Instant::now();
    let _ = pdfjoin_core::merge_documents(pdfs.clone());
    let lopdf_time = start.elapsed();

    // Benchmark streaming
    let start = Instant::now();
    let streaming_result = pdfjoin_core::merge_streaming(pdfs.clone());
    let streaming_time = start.elapsed();

    let speedup = lopdf_time.as_secs_f64() / streaming_time.as_secs_f64().max(0.000001);

    // Verify streaming produces valid PDF
    if let Ok(result) = &streaming_result {
        if !result.starts_with(b"%PDF-") {
            eprintln!("WARNING: streaming output is not valid PDF for {}", name);
        }
    } else {
        eprintln!(
            "WARNING: streaming failed for {}: {:?}",
            name,
            streaming_result.err()
        );
    }

    BenchResult {
        name: name.to_string(),
        lopdf_time,
        streaming_time,
        speedup,
    }
}

// =============================================================================
// Benchmark Tests
// =============================================================================

#[test]
fn benchmark_split_synthetic() {
    eprintln!("\n========== SPLIT BENCHMARKS (Synthetic PDFs) ==========\n");

    let sizes = [10, 50, 100, 500, 1000];

    for &num_pages in &sizes {
        let pdf = create_synthetic_pdf(num_pages);
        eprintln!("Created {}-page PDF: {} bytes", num_pages, pdf.len());

        // Split: extract first page
        let result = bench_split(
            &pdf,
            vec![1],
            &format!("{} pages -> extract page 1", num_pages),
        );
        result.print();

        // Split: extract last page
        let result = bench_split(
            &pdf,
            vec![num_pages],
            &format!("{} pages -> extract page {}", num_pages, num_pages),
        );
        result.print();

        // Split: extract 10% of pages
        let pages: Vec<u32> = (1..=num_pages).step_by(10).collect();
        let result = bench_split(
            &pdf,
            pages.clone(),
            &format!("{} pages -> extract {} pages (10%)", num_pages, pages.len()),
        );
        result.print();

        eprintln!();
    }
}

#[test]
fn benchmark_merge_synthetic() {
    eprintln!("\n========== MERGE BENCHMARKS (Synthetic PDFs) ==========\n");

    // Merge 2 documents
    let docs: Vec<Vec<u8>> = (0..2).map(|_| create_synthetic_pdf(10)).collect();
    let result = bench_merge(docs, "Merge 2 x 10-page docs");
    result.print();

    // Merge 5 documents
    let docs: Vec<Vec<u8>> = (0..5).map(|_| create_synthetic_pdf(10)).collect();
    let result = bench_merge(docs, "Merge 5 x 10-page docs");
    result.print();

    // Merge 10 documents
    let docs: Vec<Vec<u8>> = (0..10).map(|_| create_synthetic_pdf(10)).collect();
    let result = bench_merge(docs, "Merge 10 x 10-page docs");
    result.print();

    // Merge 20 documents
    let docs: Vec<Vec<u8>> = (0..20).map(|_| create_synthetic_pdf(5)).collect();
    let result = bench_merge(docs, "Merge 20 x 5-page docs");
    result.print();

    // Merge 50 single-page documents
    let docs: Vec<Vec<u8>> = (0..50).map(|_| create_synthetic_pdf(1)).collect();
    let result = bench_merge(docs, "Merge 50 x 1-page docs");
    result.print();

    eprintln!();
}

#[test]
fn benchmark_real_pdfs() {
    eprintln!("\n========== BENCHMARKS (Real Demo PDFs) ==========\n");

    let demo_pdfs = load_demo_pdfs();

    if demo_pdfs.is_empty() {
        eprintln!("SKIPPED: No demo PDFs found in output/");
        eprintln!("Run: cargo run --example generate_florida_realestate --features server");
        return;
    }

    for (name, bytes) in &demo_pdfs {
        eprintln!("Testing {}: {} bytes", name, bytes.len());

        // Get page count
        let page_count = pdfjoin_core::get_page_count(bytes).unwrap_or(1);
        eprintln!("  Pages: {}", page_count);

        // Split: extract first page
        let result = bench_split(bytes, vec![1], &format!("{} -> page 1", name));
        result.print();

        // Split: extract all pages
        let all_pages: Vec<u32> = (1..=page_count).collect();
        let result = bench_split(bytes, all_pages, &format!("{} -> all pages", name));
        result.print();

        eprintln!();
    }

    // Merge all demo PDFs
    if demo_pdfs.len() >= 2 {
        let docs: Vec<Vec<u8>> = demo_pdfs.iter().map(|(_, b)| b.clone()).collect();
        let result = bench_merge(docs, "Merge all demo PDFs");
        result.print();
        eprintln!();
    }
}

#[test]
fn benchmark_worst_case_1000_pages() {
    eprintln!("\n========== WORST CASE: 1000-page PDF ==========\n");

    let pdf = create_synthetic_pdf(1000);
    eprintln!("Created 1000-page PDF: {} bytes", pdf.len());

    // This is the worst case for lopdf - extracting 1 page from huge doc
    let result = bench_split(&pdf, vec![500], "1000 pages -> extract middle page");
    result.print();

    let result = bench_split(&pdf, vec![1], "1000 pages -> extract first page");
    result.print();

    let result = bench_split(&pdf, vec![1000], "1000 pages -> extract last page");
    result.print();

    // Extract 100 scattered pages
    let scattered: Vec<u32> = (1..=1000).step_by(10).collect();
    let result = bench_split(
        &pdf,
        scattered.clone(),
        &format!("1000 pages -> extract {} scattered", scattered.len()),
    );
    result.print();

    eprintln!();
}

#[test]
fn benchmark_summary() {
    eprintln!("\n========== BENCHMARK SUMMARY ==========\n");

    // Quick comparison
    let pdf_100 = create_synthetic_pdf(100);
    let pdf_500 = create_synthetic_pdf(500);

    eprintln!("Split single page from N-page document:");
    bench_split(&pdf_100, vec![1], "  100 pages").print();
    bench_split(&pdf_500, vec![1], "  500 pages").print();

    eprintln!("\nMerge N single-page documents:");
    let docs_10: Vec<Vec<u8>> = (0..10).map(|_| create_synthetic_pdf(1)).collect();
    let docs_50: Vec<Vec<u8>> = (0..50).map(|_| create_synthetic_pdf(1)).collect();
    bench_merge(docs_10, "  10 documents").print();
    bench_merge(docs_50, "  50 documents").print();

    eprintln!("\n===========================================\n");
}
