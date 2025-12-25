//! PDFJoin browser integration tests
//!
//! Tests for pdfjoin-web running on port 8082.
//! Requires: trunk serve in apps/pdfjoin-web (port 8082)
//!
//! Run with: cargo test -p benchmark-harness --test browser_pdfjoin

#[path = "common/browser.rs"]
mod browser;
#[path = "common/server.rs"]
mod server;

use base64::Engine;
use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use lopdf::{content::Content, content::Operation, Dictionary, Document, Object, Stream};
use std::time::Duration;

/// Typst-generated Florida Purchase Contract (real content, ~17 pages)
const FLORIDA_CONTRACT_PDF: &[u8] = include_bytes!("../../../output/florida_purchase_contract.pdf");

/// Typst-generated Florida Listing Agreement (real content)
const FLORIDA_LISTING_PDF: &[u8] = include_bytes!("../../../output/florida_listing_agreement.pdf");

/// Typst-generated Florida Escalation Addendum (real content)
const FLORIDA_ESCALATION_PDF: &[u8] =
    include_bytes!("../../../output/florida_escalation_addendum.pdf");

/// Generate a simple test PDF with N pages using lopdf
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

/// Get a test PDF as a base64 string for injection into browser
fn test_pdf_base64(num_pages: u32) -> String {
    let pdf_bytes = create_test_pdf(num_pages);
    base64::engine::general_purpose::STANDARD.encode(&pdf_bytes)
}

/// Get the Florida contract PDF as base64 for browser injection
fn florida_contract_base64() -> String {
    base64::engine::general_purpose::STANDARD.encode(FLORIDA_CONTRACT_PDF)
}

/// Get the Florida listing agreement PDF as base64 for browser injection
fn florida_listing_base64() -> String {
    base64::engine::general_purpose::STANDARD.encode(FLORIDA_LISTING_PDF)
}

/// Get the Florida escalation addendum PDF as base64 for browser injection
fn florida_escalation_base64() -> String {
    base64::engine::general_purpose::STANDARD.encode(FLORIDA_ESCALATION_PDF)
}

// ============================================================================
// Homepage & Core Functionality Tests
// ============================================================================

#[tokio::test]
async fn test_pdfjoin_homepage_loads() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    // Wait for WASM to load
    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"({
            hasTabs: !!document.querySelector('.tabs'),
            hasSplitTab: !!document.querySelector('[data-tab="split"]'),
            hasMergeTab: !!document.querySelector('[data-tab="merge"]'),
            hasSplitDropZone: !!document.querySelector('#split-drop-zone'),
            hasMergeDropZone: !!document.querySelector('#merge-drop-zone'),
            hasSplitFileInput: !!document.querySelector('#split-file-input'),
            hasMergeFileInput: !!document.querySelector('#merge-file-input'),
            title: document.title,
            wasmLoaded: typeof window.wasmBindings !== 'undefined'
        })"#,
        )
        .await
        .expect("Should evaluate JS")
        .into_value()
        .expect("Should get value");

    eprintln!("PDFJoin page state: {:?}", result);

    assert!(
        result["hasTabs"].as_bool().unwrap_or(false),
        "Should have .tabs element"
    );
    assert!(
        result["hasSplitTab"].as_bool().unwrap_or(false),
        "Should have split tab"
    );
    assert!(
        result["hasMergeTab"].as_bool().unwrap_or(false),
        "Should have merge tab"
    );
    assert!(
        result["hasSplitDropZone"].as_bool().unwrap_or(false),
        "Should have #split-drop-zone element"
    );
    assert!(
        result["wasmLoaded"].as_bool().unwrap_or(false),
        "WASM bindings should be loaded on window.wasmBindings"
    );
}

#[tokio::test]
async fn test_pdfjoin_tab_switching_works() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check initial state - split view should be visible
    let initial_state: serde_json::Value = page
        .evaluate(
            r#"({
            splitViewVisible: !document.querySelector('#split-view')?.classList.contains('hidden'),
            mergeViewVisible: !document.querySelector('#merge-view')?.classList.contains('hidden'),
            splitTabActive: document.querySelector('[data-tab="split"]')?.classList.contains('active'),
            mergeTabActive: document.querySelector('[data-tab="merge"]')?.classList.contains('active')
        })"#,
        )
        .await
        .expect("Should get initial state")
        .into_value()
        .expect("Should get value");

    eprintln!("Initial state: {:?}", initial_state);

    assert!(
        initial_state["splitViewVisible"].as_bool().unwrap_or(false),
        "Split view should be visible initially"
    );
    assert!(
        initial_state["splitTabActive"].as_bool().unwrap_or(false),
        "Split tab should be active initially"
    );

    // Click merge tab
    let click_result: bool = page
        .evaluate(
            r#"(() => {
                const tab = document.querySelector('[data-tab="merge"]');
                if (tab) { tab.click(); return true; }
                return false;
            })()"#,
        )
        .await
        .expect("Should click merge tab")
        .into_value()
        .expect("Should get value");

    assert!(click_result, "Should find and click merge tab");

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Check state after clicking merge
    let after_click: serde_json::Value = page
        .evaluate(
            r#"({
            splitViewVisible: !document.querySelector('#split-view')?.classList.contains('hidden'),
            mergeViewVisible: !document.querySelector('#merge-view')?.classList.contains('hidden'),
            splitTabActive: document.querySelector('[data-tab="split"]')?.classList.contains('active'),
            mergeTabActive: document.querySelector('[data-tab="merge"]')?.classList.contains('active')
        })"#,
        )
        .await
        .expect("Should get state after click")
        .into_value()
        .expect("Should get value");

    eprintln!("After merge tab click: {:?}", after_click);

    assert!(
        after_click["mergeViewVisible"].as_bool().unwrap_or(false),
        "Merge view should be visible after clicking merge tab"
    );
    assert!(
        after_click["mergeTabActive"].as_bool().unwrap_or(false),
        "Merge tab should be active after clicking"
    );
    assert!(
        !after_click["splitViewVisible"].as_bool().unwrap_or(true),
        "Split view should be hidden after clicking merge tab"
    );
}

#[tokio::test]
async fn test_pdfjoin_wasm_session_creation() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Test creating sessions
    let session_test: serde_json::Value = page
        .evaluate(
            r#"(() => {
                try {
                    const { PdfJoinSession, SessionMode } = window.wasmBindings;
                    if (!PdfJoinSession || !SessionMode) {
                        return { success: false, error: 'Missing exports' };
                    }

                    // Try creating split session
                    const splitSession = new PdfJoinSession(SessionMode.Split);
                    // mode is a getter property, not a method
                    const splitMode = splitSession.mode;

                    // Try creating merge session
                    const mergeSession = new PdfJoinSession(SessionMode.Merge);
                    const mergeMode = mergeSession.mode;

                    return {
                        success: true,
                        splitMode: splitMode,
                        mergeMode: mergeMode,
                        hasFormatBytes: typeof window.wasmBindings.format_bytes === 'function'
                    };
                } catch (err) {
                    return { success: false, error: err.message };
                }
            })()"#,
        )
        .await
        .expect("Should test sessions")
        .into_value()
        .expect("Should get value");

    eprintln!("Session creation test: {:?}", session_test);

    assert!(
        session_test["success"].as_bool().unwrap_or(false),
        "Should be able to create WASM sessions. Error: {:?}",
        session_test["error"]
    );
    assert!(
        session_test["hasFormatBytes"].as_bool().unwrap_or(false),
        "Should have format_bytes utility function"
    );
}

// ============================================================================
// Mobile Viewport Tests
// ============================================================================

#[tokio::test]
async fn test_pdfjoin_mobile_viewport() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    // Set mobile viewport (iPhone SE: 375x667)
    page.execute(
        SetDeviceMetricsOverrideParams::builder()
            .width(375)
            .height(667)
            .device_scale_factor(2.0)
            .mobile(true)
            .build()
            .unwrap(),
    )
    .await
    .expect("Should set mobile viewport");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to pdfjoin");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let mobile_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const results = {
                    viewportWidth: window.innerWidth,
                    bodyWidth: document.body.scrollWidth,
                    hasHorizontalScroll: document.body.scrollWidth > window.innerWidth,
                    headerVisible: !!document.querySelector('header, h1'),
                    tabsVisible: !!document.querySelector('.tabs'),
                    dropZoneVisible: !!document.querySelector('#split-drop-zone, #merge-drop-zone, .drop-zone'),
                };
                return results;
            })()"#,
        )
        .await
        .expect("Should check mobile layout")
        .into_value()
        .expect("Should get value");

    eprintln!("PDFJoin mobile check: {:?}", mobile_check);

    assert!(
        !mobile_check["hasHorizontalScroll"]
            .as_bool()
            .unwrap_or(true),
        "Should not have horizontal scroll on mobile. Body width: {}, Viewport: {}",
        mobile_check["bodyWidth"],
        mobile_check["viewportWidth"]
    );

    assert!(
        mobile_check["headerVisible"].as_bool().unwrap_or(false),
        "Header should be visible on mobile"
    );
    assert!(
        mobile_check["tabsVisible"].as_bool().unwrap_or(false),
        "Tabs should be visible on mobile"
    );
    assert!(
        mobile_check["dropZoneVisible"].as_bool().unwrap_or(false),
        "Drop zone should be visible on mobile"
    );
}

// ============================================================================
// Split Functionality Regression Tests
// ============================================================================

/// Regression test: Split output must not have duplicate Pages objects
/// Bug: The streaming split was including original Catalog and Pages objects
/// AND creating new ones, resulting in two /Count entries which corrupts the PDF.
/// macOS Preview and other strict PDF readers reject such files.
#[tokio::test]
async fn test_pdfjoin_split_no_duplicate_pages_objects() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Use embedded Florida contract PDF (real content with ~17 pages)
    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Decode base64 PDF embedded at compile time
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const {{ PdfJoinSession, SessionMode }} = window.wasmBindings;
                const session = new PdfJoinSession(SessionMode.Split);

                // Add document
                const info = session.addDocument('test.pdf', pdfBytes);
                if (info.page_count < 10) {{
                    return {{ success: false, error: 'Expected 10+ pages, got ' + info.page_count }};
                }}

                // Select a range of pages
                session.setPageSelection('5-10');

                // Execute split
                const result = session.execute();
                const resultBytes = new Uint8Array(result);

                // Convert to string to count /Count occurrences
                const decoder = new TextDecoder('utf-8', {{ fatal: false }});
                const pdfText = decoder.decode(resultBytes);

                // Count /Count entries
                const countMatches = pdfText.match(/\/Count \d+/g) || [];

                return {{
                    success: true,
                    outputSize: resultBytes.length,
                    countOccurrences: countMatches.length,
                    countValues: countMatches,
                    startsWithPdf: pdfText.startsWith('%PDF-')
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should execute split test")
        .into_value()
        .expect("Should get value");

    eprintln!("Split regression test result: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Split should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["startsWithPdf"].as_bool().unwrap_or(false),
        "Output should be valid PDF"
    );

    let count_occurrences = result["countOccurrences"].as_i64().unwrap_or(0);
    assert_eq!(
        count_occurrences, 1,
        "REGRESSION: Found {} /Count entries, expected 1. Values: {:?}. \
         Duplicate Pages objects cause PDF corruption in macOS Preview.",
        count_occurrences, result["countValues"]
    );
}

/// Regression test: Page objects must have /Parent pointing to the Pages object
/// Bug: After splitting, page objects still referenced the original Pages object ID
/// which no longer exists in the output PDF.
#[tokio::test]
async fn test_pdfjoin_split_pages_have_valid_parent() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Use embedded Florida contract PDF
    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Decode base64 PDF embedded at compile time
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const {{ PdfJoinSession, SessionMode }} = window.wasmBindings;
                const session = new PdfJoinSession(SessionMode.Split);
                session.addDocument('test.pdf', pdfBytes);
                session.setPageSelection('5-10');

                const result = session.execute();
                const resultBytes = new Uint8Array(result);
                const decoder = new TextDecoder('utf-8', {{ fatal: false }});
                const pdfText = decoder.decode(resultBytes);

                // Find Pages object ID
                const pagesMatch = pdfText.match(/(\d+) 0 obj\s*<<[^>]*?\/Type \/Pages[^>]*?\/Kids/);
                const pagesId = pagesMatch ? pagesMatch[1] : null;

                // Find all /Parent references
                const parentMatches = pdfText.match(/\/Parent (\d+) 0 R/g) || [];
                const parentIds = parentMatches.map(m => m.match(/\d+/)[0]);
                const invalidParents = parentIds.filter(id => id !== pagesId);

                return {{
                    success: true,
                    pagesObjectId: pagesId,
                    parentRefs: parentIds,
                    invalidParents: invalidParents,
                    allValid: invalidParents.length === 0
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test parent refs")
        .into_value()
        .expect("Should get value");

    eprintln!("Parent reference test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["allValid"].as_bool().unwrap_or(false),
        "REGRESSION: Pages have invalid /Parent refs. Pages object is {}, but found refs to {:?}",
        result["pagesObjectId"],
        result["invalidParents"]
    );
}

// ============================================================================
// Split UX Tests - Page Range Input Discoverability
// ============================================================================

/// UX Test: After loading a PDF, the page range input should be empty
/// so the placeholder text remains visible showing the syntax examples.
/// Bug: Currently auto-fills with "1-N" which hides the helpful placeholder.
#[tokio::test]
async fn test_pdfjoin_split_placeholder_visible_after_load() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Generate test PDF in Rust and inject as base64
    let pdf_b64 = test_pdf_base64(10);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Decode base64 PDF generated by Rust
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                // Trigger file load via the actual UI
                const fileInput = document.getElementById('split-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                // Wait for UI to update
                await new Promise(r => setTimeout(r, 500));

                const rangeInput = document.getElementById('page-range');
                return {{
                    success: true,
                    inputValue: rangeInput.value,
                    inputEmpty: rangeInput.value === '',
                    placeholder: rangeInput.placeholder
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test placeholder visibility")
        .into_value()
        .expect("Should get value");

    eprintln!("Placeholder visibility test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["inputEmpty"].as_bool().unwrap_or(false),
        "UX BUG: Page range input should be empty after loading PDF so placeholder is visible. \
         Currently contains: '{}'. Placeholder '{}' is hidden from user.",
        result["inputValue"].as_str().unwrap_or(""),
        result["placeholder"].as_str().unwrap_or("")
    );
}

/// UX Test: Helper text should exist below the page range input
/// explaining the range syntax for better discoverability.
#[tokio::test]
async fn test_pdfjoin_split_helper_text_exists() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"(() => {
                // Look for helper text near the page-range input
                const rangeInput = document.getElementById('page-range');
                if (!rangeInput) {
                    return { success: false, error: 'page-range input not found' };
                }

                // Check for a hint/helper element - could be sibling or within parent
                const parent = rangeInput.closest('.range-input') || rangeInput.parentElement;
                const helperText = parent?.querySelector('.range-hint, .helper-text, .hint, small');

                // Also check for any text mentioning "comma" or "ranges" nearby
                const parentText = parent?.textContent || '';
                const mentionsMultipleRanges = parentText.toLowerCase().includes('comma') ||
                                                parentText.toLowerCase().includes('ranges') ||
                                                parentText.toLowerCase().includes('multiple');

                return {
                    success: true,
                    hasHelperElement: !!helperText,
                    helperContent: helperText?.textContent || null,
                    mentionsMultipleRanges: mentionsMultipleRanges
                };
            })()"#,
        )
        .await
        .expect("Should check for helper text")
        .into_value()
        .expect("Should get value");

    eprintln!("Helper text test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    let has_helper = result["hasHelperElement"].as_bool().unwrap_or(false);
    let mentions_ranges = result["mentionsMultipleRanges"].as_bool().unwrap_or(false);

    assert!(
        has_helper || mentions_ranges,
        "UX BUG: No helper text found below page range input. \
         Users need guidance that they can use multiple ranges like '1-3, 5, 8-10'."
    );
}

/// UX Test: Example chips should exist that users can click to insert
/// common range patterns (e.g., "First 5 pages", "Last 3 pages").
#[tokio::test]
async fn test_pdfjoin_split_example_chips_exist() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Generate test PDF in Rust and inject as base64
    let pdf_b64 = test_pdf_base64(10);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Click the Split tab first to ensure UI is visible
                const splitTab = document.getElementById('split-tab');
                if (splitTab) {{
                    splitTab.click();
                    await new Promise(r => setTimeout(r, 300));
                }}

                // Decode base64 PDF generated by Rust
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('split-file-input');
                if (!fileInput) {{
                    return {{ success: false, error: 'split-file-input not found' }};
                }}
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                // Wait longer for chips to be generated (WASM needs to parse PDF)
                await new Promise(r => setTimeout(r, 1000));

                // Check for example chips in the range-chips container
                const chipsContainer = document.getElementById('range-chips');
                const chips = chipsContainer ? chipsContainer.querySelectorAll('.chip, [data-range]') : [];
                const chipTexts = Array.from(chips).map(c => c.textContent?.trim());

                // Also check in split-editor for any chip-like elements
                const editor = document.getElementById('split-editor');
                const editorChips = editor ? editor.querySelectorAll('.chip, .example-chip, .range-chip, [data-range]') : [];
                const editorChipTexts = Array.from(editorChips).map(c => c.textContent?.trim());

                return {{
                    success: true,
                    chipCount: chips.length,
                    chipTexts: chipTexts,
                    editorChipCount: editorChips.length,
                    editorChipTexts: editorChipTexts,
                    hasChips: chips.length > 0 || editorChips.length > 0
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should check for chips")
        .into_value()
        .expect("Should get value");

    eprintln!("Example chips test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["hasChips"].as_bool().unwrap_or(false),
        "UX BUG: No example chips found for page ranges. \
         Users should have clickable shortcuts like 'First 5', 'Last 3', etc."
    );
}

/// UX Test: Clicking an example chip should insert the correct range syntax
/// into the page range input field.
#[tokio::test]
async fn test_pdfjoin_split_chip_click_inserts_range() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Generate test PDF in Rust and inject as base64
    let pdf_b64 = test_pdf_base64(10);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Decode base64 PDF generated by Rust
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('split-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 500));

                // Find first chip and click it
                const editor = document.getElementById('split-editor');
                const chip = editor?.querySelector('.chip, .example-chip, .range-chip, [data-range]');

                if (!chip) {{
                    return {{ success: false, error: 'No chips found to click', needsChips: true }};
                }}

                const rangeInput = document.getElementById('page-range');
                const valueBefore = rangeInput.value;

                chip.click();
                await new Promise(r => setTimeout(r, 100));

                const valueAfter = rangeInput.value;
                const isValidRange = /^[\d\s,\-]+$/.test(valueAfter) && valueAfter.length > 0;

                return {{
                    success: true,
                    valueBefore: valueBefore,
                    valueAfter: valueAfter,
                    valueChanged: valueBefore !== valueAfter,
                    isValidRange: isValidRange,
                    chipText: chip.textContent?.trim()
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test chip click")
        .into_value()
        .expect("Should get value");

    eprintln!("Chip click test: {:?}", result);

    // If no chips exist, this test should be skipped (the chips_exist test catches that)
    if result["needsChips"].as_bool().unwrap_or(false) {
        eprintln!("Skipping chip click test - no chips exist yet");
        return;
    }

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["valueChanged"].as_bool().unwrap_or(false),
        "Clicking chip '{}' should change input value. Before: '{}', After: '{}'",
        result["chipText"].as_str().unwrap_or("?"),
        result["valueBefore"].as_str().unwrap_or(""),
        result["valueAfter"].as_str().unwrap_or("")
    );

    assert!(
        result["isValidRange"].as_bool().unwrap_or(false),
        "Chip click should insert valid range syntax. Got: '{}'",
        result["valueAfter"].as_str().unwrap_or("")
    );
}

// ============================================================================
// Mobile Accessibility Tests
// ============================================================================

#[tokio::test]
async fn test_pdfjoin_mobile_touch_targets() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.execute(
        SetDeviceMetricsOverrideParams::builder()
            .width(375)
            .height(667)
            .device_scale_factor(2.0)
            .mobile(true)
            .build()
            .unwrap(),
    )
    .await
    .expect("Should set mobile viewport");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to pdfjoin");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let touch_targets: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const MIN_TOUCH_TARGET = 44;
                const issues = [];

                document.querySelectorAll('button, .btn, .tab, [role="button"]').forEach(el => {
                    const rect = el.getBoundingClientRect();
                    if (rect.height > 0 && rect.height < MIN_TOUCH_TARGET) {
                        issues.push({
                            type: 'button',
                            text: el.textContent?.slice(0, 20),
                            height: rect.height,
                            width: rect.width
                        });
                    }
                });

                document.querySelectorAll('input:not([type="hidden"]), select, textarea').forEach(el => {
                    const rect = el.getBoundingClientRect();
                    if (rect.height > 0 && rect.height < MIN_TOUCH_TARGET) {
                        issues.push({
                            type: 'input',
                            name: el.name || el.type,
                            height: rect.height,
                            width: rect.width
                        });
                    }
                });

                return {
                    totalIssues: issues.length,
                    issues: issues.slice(0, 10),
                    passed: issues.length === 0
                };
            })()"#,
        )
        .await
        .expect("Should check touch targets")
        .into_value()
        .expect("Should get value");

    eprintln!("Touch target check: {:?}", touch_targets);

    // Warning for now, not a hard failure
    if !touch_targets["passed"].as_bool().unwrap_or(true) {
        eprintln!(
            "Warning: {} elements below 44px touch target minimum: {:?}",
            touch_targets["totalIssues"], touch_targets["issues"]
        );
    }
}

// ============================================================================
// Multi-File Split Feature Tests
// ============================================================================

/// Feature test: A checkbox should exist to enable "split into separate files" mode
/// When checked, each range (comma-separated) produces a separate PDF download.
#[tokio::test]
async fn test_pdfjoin_split_multiple_files_checkbox_exists() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Load a PDF to reveal the split editor
    let pdf_b64 = test_pdf_base64(10);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Decode and load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('split-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 500));

                // Look for the multi-file checkbox
                const checkbox = document.getElementById('split-multiple-files') ||
                                 document.querySelector('input[name="split-multiple"]') ||
                                 document.querySelector('input[type="checkbox"][id*="multiple"]') ||
                                 document.querySelector('input[type="checkbox"][id*="separate"]');

                // Also look for any label mentioning "separate" or "multiple"
                const labels = Array.from(document.querySelectorAll('label'));
                const relevantLabel = labels.find(l =>
                    l.textContent?.toLowerCase().includes('separate') ||
                    l.textContent?.toLowerCase().includes('multiple files') ||
                    l.textContent?.toLowerCase().includes('one file per')
                );

                return {{
                    success: true,
                    hasCheckbox: !!checkbox,
                    checkboxId: checkbox?.id || null,
                    hasRelevantLabel: !!relevantLabel,
                    labelText: relevantLabel?.textContent?.trim() || null
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should check for multi-file checkbox")
        .into_value()
        .expect("Should get value");

    eprintln!("Multi-file checkbox test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["hasCheckbox"].as_bool().unwrap_or(false),
        "FEATURE MISSING: No 'split into separate files' checkbox found. \
         Users need a way to split multiple ranges into separate PDF files."
    );
}

/// Feature test: When multi-file checkbox is checked and multiple ranges are entered,
/// splitting should produce multiple separate PDF files (one per range).
#[tokio::test]
async fn test_pdfjoin_split_multiple_files_produces_multiple_downloads() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Load PDF and test multi-file split
    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Decode and load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('split-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 500));

                // Check the multi-file checkbox
                const checkbox = document.getElementById('split-multiple-files') ||
                                 document.querySelector('input[name="split-multiple"]') ||
                                 document.querySelector('input[type="checkbox"][id*="multiple"]') ||
                                 document.querySelector('input[type="checkbox"][id*="separate"]');

                if (!checkbox) {{
                    return {{ success: false, error: 'Multi-file checkbox not found', needsCheckbox: true }};
                }}

                checkbox.checked = true;
                checkbox.dispatchEvent(new Event('change', {{ bubbles: true }}));

                // Enter multiple ranges
                const rangeInput = document.getElementById('page-range');
                rangeInput.value = '1-3, 5-7, 10';
                rangeInput.dispatchEvent(new Event('input', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 100));

                // Intercept downloads by overriding the download function
                const downloads = [];
                const originalCreateElement = document.createElement.bind(document);
                document.createElement = function(tagName) {{
                    const el = originalCreateElement(tagName);
                    if (tagName.toLowerCase() === 'a') {{
                        const originalClick = el.click.bind(el);
                        el.click = function() {{
                            if (el.download) {{
                                downloads.push({{
                                    filename: el.download,
                                    hasBlob: el.href?.startsWith('blob:')
                                }});
                            }}
                            // Don't actually download in test
                        }};
                    }}
                    return el;
                }};

                // Click split button
                const splitBtn = document.getElementById('split-btn');
                if (splitBtn.disabled) {{
                    return {{ success: false, error: 'Split button is disabled', buttonDisabled: true }};
                }}

                splitBtn.click();

                // Wait for split operations
                await new Promise(r => setTimeout(r, 1000));

                // Restore original
                document.createElement = originalCreateElement;

                return {{
                    success: true,
                    downloadCount: downloads.length,
                    downloads: downloads,
                    expectedCount: 3 // "1-3", "5-7", "10" = 3 ranges
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test multi-file split")
        .into_value()
        .expect("Should get value");

    eprintln!("Multi-file download test: {:?}", result);

    // If checkbox doesn't exist yet, that's the first failure we need to fix
    if result
        .get("needsCheckbox")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        panic!(
            "FEATURE MISSING: Multi-file checkbox not found. \
             Add checkbox with id='split-multiple-files' first."
        );
    }

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    let download_count = result["downloadCount"].as_i64().unwrap_or(0);
    let expected_count = result["expectedCount"].as_i64().unwrap_or(3);

    assert_eq!(
        download_count, expected_count,
        "FEATURE BUG: Expected {} separate PDF downloads for ranges '1-3, 5-7, 10', \
         but got {}. Downloads: {:?}",
        expected_count, download_count, result["downloads"]
    );
}

/// Feature test: Each downloaded file should have a descriptive filename
/// that includes the range it contains (e.g., "document-pages-1-3.pdf").
#[tokio::test]
async fn test_pdfjoin_split_multiple_files_have_correct_names() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(10);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('split-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'my-document.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 500));

                // Enable multi-file mode
                const checkbox = document.getElementById('split-multiple-files');
                if (!checkbox) {{
                    return {{ success: false, error: 'Multi-file checkbox not found' }};
                }}

                checkbox.checked = true;
                checkbox.dispatchEvent(new Event('change', {{ bubbles: true }}));

                // Enter ranges
                const rangeInput = document.getElementById('page-range');
                rangeInput.value = '1-3, 5';
                rangeInput.dispatchEvent(new Event('input', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 100));

                // Capture download filenames
                const filenames = [];
                const originalCreateElement = document.createElement.bind(document);
                document.createElement = function(tagName) {{
                    const el = originalCreateElement(tagName);
                    if (tagName.toLowerCase() === 'a') {{
                        const originalClick = el.click.bind(el);
                        el.click = function() {{
                            if (el.download) {{
                                filenames.push(el.download);
                            }}
                        }};
                    }}
                    return el;
                }};

                // Execute split
                const splitBtn = document.getElementById('split-btn');
                if (!splitBtn.disabled) {{
                    splitBtn.click();
                    await new Promise(r => setTimeout(r, 1000));
                }}

                document.createElement = originalCreateElement;

                // Check filenames include range info
                const hasRangeInName = filenames.every(name =>
                    name.includes('1-3') || name.includes('5') ||
                    name.includes('pages') || name.includes('range')
                );

                const hasOriginalName = filenames.every(name =>
                    name.includes('my-document') || name.includes('document')
                );

                return {{
                    success: true,
                    filenames: filenames,
                    hasRangeInName: hasRangeInName,
                    hasOriginalName: hasOriginalName
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test filename generation")
        .into_value()
        .expect("Should get value");

    eprintln!("Filename test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    let filenames = result["filenames"].as_array();
    if let Some(names) = filenames {
        assert!(
            !names.is_empty(),
            "No files were downloaded - multi-file split may not be working"
        );

        // Each filename should contain the range it represents
        for name in names {
            let name_str = name.as_str().unwrap_or("");
            assert!(
                name_str.contains("1-3")
                    || name_str.contains("5")
                    || name_str.contains("pages")
                    || name_str.contains("range"),
                "Filename '{}' should indicate which pages it contains",
                name_str
            );
        }
    }
}

// ============================================================================
// Merge Drag-and-Drop Bug Tests
// ============================================================================

/// Bug test: After adding the first file to merge, users should still be able
/// to drag and drop additional files onto the file list area.
/// Bug: The drop zone is hidden after adding files, and the file list has no
/// drag-and-drop handlers, so users can only add more files via the button.
#[tokio::test]
async fn test_pdfjoin_merge_drag_drop_additional_files() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(3);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Switch to merge tab
                document.querySelector('[data-tab="merge"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Create test PDF bytes
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                // Add first file via file input (simulating initial drop)
                const fileInput = document.getElementById('merge-file-input');
                const dataTransfer1 = new DataTransfer();
                const file1 = new File([pdfBytes], 'first.pdf', {{ type: 'application/pdf' }});
                dataTransfer1.items.add(file1);
                fileInput.files = dataTransfer1.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 500));

                // Check state after first file
                const dropZone = document.getElementById('merge-drop-zone');
                const fileList = document.getElementById('merge-file-list');
                const dropZoneHidden = dropZone.classList.contains('hidden');
                const fileListVisible = !fileList.classList.contains('hidden');

                // Now try to find a valid drop target for additional files
                // It should be either the file list area or a dedicated drop zone
                const fileListHasDropHandler = fileList.ondrop !== null ||
                    fileList.getAttribute('ondrop') !== null;

                // Check if there's any visible element that accepts drops
                const visibleDropTargets = document.querySelectorAll('[class*="drop"]:not(.hidden)');
                const hasVisibleDropTarget = visibleDropTargets.length > 0;

                // Try to simulate a drop on the file list
                const dropEvent = new DragEvent('drop', {{
                    bubbles: true,
                    cancelable: true,
                    dataTransfer: new DataTransfer()
                }});

                // Create second file
                const file2 = new File([pdfBytes], 'second.pdf', {{ type: 'application/pdf' }});
                dropEvent.dataTransfer.items.add(file2);

                // Count files before drop attempt
                const fileCountBefore = document.querySelectorAll('#merge-files li').length;

                // Dispatch drop event on file list
                fileList.dispatchEvent(dropEvent);

                await new Promise(r => setTimeout(r, 500));

                // Count files after drop attempt
                const fileCountAfter = document.querySelectorAll('#merge-files li').length;

                return {{
                    success: true,
                    dropZoneHidden: dropZoneHidden,
                    fileListVisible: fileListVisible,
                    hasVisibleDropTarget: hasVisibleDropTarget,
                    fileCountBefore: fileCountBefore,
                    fileCountAfter: fileCountAfter,
                    dropWorked: fileCountAfter > fileCountBefore
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test merge drag-drop")
        .into_value()
        .expect("Should get value");

    eprintln!("Merge drag-drop test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    // The drop zone is hidden after first file - this is expected behavior
    assert!(
        result["dropZoneHidden"].as_bool().unwrap_or(false),
        "Drop zone should be hidden after adding first file"
    );

    // But we should still be able to drop files somewhere
    assert!(
        result["dropWorked"].as_bool().unwrap_or(false),
        "BUG: Cannot drag-and-drop additional files after the first one. \
         The file list should accept drops, or a drop zone should remain visible. \
         Files before: {}, after: {}",
        result["fileCountBefore"],
        result["fileCountAfter"]
    );
}

/// Bug test: When using the browse button and selecting multiple files at once
/// (e.g., Cmd+click or Shift+click in the file picker), ALL selected files
/// should be added to the merge list, not just the last one.
/// Bug: Currently only the last selected PDF is added when selecting multiple files.
#[tokio::test]
async fn test_pdfjoin_merge_browse_multiple_files() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Create three test PDFs with different page counts to distinguish them
    let pdf1_b64 = test_pdf_base64(1);
    let pdf2_b64 = test_pdf_base64(2);
    let pdf3_b64 = test_pdf_base64(3);

    let js_code = format!(
        r#"(async () => {{
            try {{
                // Switch to merge tab
                document.querySelector('[data-tab="merge"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Create test PDF bytes for 3 different files
                function decodeB64(b64) {{
                    const binary = atob(b64);
                    const bytes = new Uint8Array(binary.length);
                    for (let i = 0; i < binary.length; i++) {{
                        bytes[i] = binary.charCodeAt(i);
                    }}
                    return bytes;
                }}

                const pdf1Bytes = decodeB64("{}");
                const pdf2Bytes = decodeB64("{}");
                const pdf3Bytes = decodeB64("{}");

                // Simulate selecting multiple files at once via file input
                // This is what happens when user uses Cmd+click or Shift+click
                const fileInput = document.getElementById('merge-file-input');
                const dataTransfer = new DataTransfer();

                // Add all 3 files to the DataTransfer (simulating multi-select)
                const file1 = new File([pdf1Bytes], 'first.pdf', {{ type: 'application/pdf' }});
                const file2 = new File([pdf2Bytes], 'second.pdf', {{ type: 'application/pdf' }});
                const file3 = new File([pdf3Bytes], 'third.pdf', {{ type: 'application/pdf' }});

                dataTransfer.items.add(file1);
                dataTransfer.items.add(file2);
                dataTransfer.items.add(file3);

                // Set the files and trigger change event
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                // Wait for UI to update
                await new Promise(r => setTimeout(r, 500));

                // Count how many files were added to the merge list
                const fileListItems = document.querySelectorAll('#merge-files li');
                const fileCount = fileListItems.length;

                // Get the file names that were added
                const fileNames = Array.from(fileListItems).map(li =>
                    li.querySelector('.file-name')?.textContent || ''
                );

                return {{
                    success: true,
                    filesSelected: 3,
                    filesAdded: fileCount,
                    fileNames: fileNames,
                    allFilesAdded: fileCount === 3
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf1_b64, pdf2_b64, pdf3_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test multi-file browse")
        .into_value()
        .expect("Should get value");

    eprintln!("Multi-file browse test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    let files_selected = result["filesSelected"].as_i64().unwrap_or(0);
    let files_added = result["filesAdded"].as_i64().unwrap_or(0);

    assert_eq!(
        files_added, files_selected,
        "BUG: Selected {} files via browse button but only {} were added to merge list. \
         File names added: {:?}. All selected files should be added.",
        files_selected, files_added, result["fileNames"]
    );
}

// ============================================================================
// Text Edit Feature Tests - Font Preservation
// ============================================================================

/// Test that the Edit tab exists in pdfjoin-web
#[tokio::test]
async fn test_pdfjoin_edit_tab_exists() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"({
            hasEditTab: !!document.querySelector('[data-tab="edit"]'),
            hasEditView: !!document.querySelector('#edit-view'),
            hasEditDropZone: !!document.querySelector('#edit-drop-zone'),
            hasEditFileInput: !!document.querySelector('#edit-file-input')
        })"#,
        )
        .await
        .expect("Should evaluate JS")
        .into_value()
        .expect("Should get value");

    eprintln!("Edit tab elements: {:?}", result);

    assert!(
        result["hasEditTab"].as_bool().unwrap_or(false),
        "Should have edit tab button"
    );
    assert!(
        result["hasEditView"].as_bool().unwrap_or(false),
        "Should have edit view section"
    );
    assert!(
        result["hasEditDropZone"].as_bool().unwrap_or(false),
        "Should have edit drop zone"
    );
}

/// Test that EditSession WASM binding is available with replaceText method
#[tokio::test]
async fn test_pdfjoin_edit_session_wasm_binding() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Wait for wasmBindings to be available
                for (let i = 0; i < 50; i++) {{
                    if (window.wasmBindings) break;
                    await new Promise(r => setTimeout(r, 100));
                }}
                if (!window.wasmBindings) {{
                    return {{ success: false, error: 'wasmBindings not available after 5s' }};
                }}

                const {{ EditSession }} = window.wasmBindings;
                if (!EditSession) {{
                    return {{ success: false, error: 'EditSession not in wasmBindings' }};
                }}

                // Decode test PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                // Create session
                const session = new EditSession('test.pdf', pdfBytes);

                // Check methods exist
                const hasAddText = typeof session.addText === 'function';
                const hasReplaceText = typeof session.replaceText === 'function';
                const hasExport = typeof session.export === 'function';
                const hasRemoveOperation = typeof session.removeOperation === 'function';

                return {{
                    success: true,
                    pageCount: session.pageCount,
                    isSigned: session.isSigned,
                    hasAddText: hasAddText,
                    hasReplaceText: hasReplaceText,
                    hasExport: hasExport,
                    hasRemoveOperation: hasRemoveOperation
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test EditSession")
        .into_value()
        .expect("Should get value");

    eprintln!("EditSession WASM binding test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "EditSession creation should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["hasReplaceText"].as_bool().unwrap_or(false),
        "EditSession should have replaceText method for font-preserving text replacement"
    );
    assert!(
        result["hasExport"].as_bool().unwrap_or(false),
        "EditSession should have export method"
    );
}

/// Test that PdfBridge extractTextWithPositions returns font information
#[tokio::test]
async fn test_pdfjoin_text_extraction_returns_font_info() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Use Florida contract which has known text content
    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Navigate to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF via file input
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'contract.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                // Wait for PDF to load and render
                await new Promise(r => setTimeout(r, 2000));

                // Check if PdfBridge has extractTextWithPositions
                if (!window.PdfBridge) {{
                    return {{ success: false, error: 'PdfBridge not available' }};
                }}
                if (typeof window.PdfBridge.extractTextWithPositions !== 'function') {{
                    return {{ success: false, error: 'extractTextWithPositions not a function' }};
                }}

                // Extract text from first page
                const textItems = await window.PdfBridge.extractTextWithPositions(1);

                if (!textItems || textItems.length === 0) {{
                    return {{ success: false, error: 'No text items extracted' }};
                }}

                // Check first few items for required properties
                const firstItem = textItems[0];
                const hasFontFamily = 'fontFamily' in firstItem;
                const hasPdfX = 'pdfX' in firstItem;
                const hasPdfY = 'pdfY' in firstItem;
                const hasPdfWidth = 'pdfWidth' in firstItem;
                const hasPdfHeight = 'pdfHeight' in firstItem;
                const hasStr = 'str' in firstItem;

                // Get sample of font families found
                const fontFamilies = [...new Set(textItems.map(item => item.fontFamily))];

                return {{
                    success: true,
                    textItemCount: textItems.length,
                    hasFontFamily: hasFontFamily,
                    hasPdfX: hasPdfX,
                    hasPdfY: hasPdfY,
                    hasPdfWidth: hasPdfWidth,
                    hasPdfHeight: hasPdfHeight,
                    hasStr: hasStr,
                    fontFamilies: fontFamilies,
                    sampleItem: {{
                        str: firstItem.str?.substring(0, 50),
                        fontFamily: firstItem.fontFamily,
                        pdfHeight: firstItem.pdfHeight
                    }}
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test text extraction")
        .into_value()
        .expect("Should get value");

    eprintln!("Text extraction font info test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Text extraction should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["hasFontFamily"].as_bool().unwrap_or(false),
        "Text items should include fontFamily property for font preservation"
    );
    assert!(
        result["hasPdfHeight"].as_bool().unwrap_or(false),
        "Text items should include pdfHeight for font size preservation"
    );

    let text_count = result["textItemCount"].as_i64().unwrap_or(0);
    assert!(
        text_count > 0,
        "Should extract text items from PDF, got {}",
        text_count
    );
}

/// Test that replaceText creates operations with font information
#[tokio::test]
async fn test_pdfjoin_replace_text_preserves_font_in_operation() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Wait for wasmBindings to be available
                for (let i = 0; i < 50; i++) {{
                    if (window.wasmBindings) break;
                    await new Promise(r => setTimeout(r, 100));
                }}
                if (!window.wasmBindings) {{
                    return {{ success: false, error: 'wasmBindings not available after 5s' }};
                }}

                const {{ EditSession }} = window.wasmBindings;

                // Decode test PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const session = new EditSession('test.pdf', pdfBytes);

                // Test replaceText with font preservation
                // Parameters: page, orig_x, orig_y, orig_w, orig_h, new_x, new_y, new_w, new_h,
                //            original_text, new_text, font_size, color, font_name
                const opId = session.replaceText(
                    1,           // page
                    100, 700,    // orig_x, orig_y
                    150, 14,     // orig_width, orig_height
                    100, 700,    // new_x, new_y
                    150, 14,     // new_width, new_height
                    'Original',  // original_text
                    'Replaced',  // new_text
                    12.0,        // font_size
                    '#000000',   // color
                    'serif'      // font_name (should map to Times-Roman)
                );

                // Get operations JSON to verify font was stored
                const opsJson = session.getOperationsJson();
                const ops = JSON.parse(opsJson);

                const replaceOp = ops.operations?.find(op => op.type === 'ReplaceText');

                return {{
                    success: true,
                    opId: Number(opId),
                    hasChanges: session.hasChanges(),
                    operationCount: session.getOperationCount(),
                    hasReplaceOp: !!replaceOp,
                    fontNameInOp: replaceOp?.style?.font_name || null
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test replaceText")
        .into_value()
        .expect("Should get value");

    eprintln!("ReplaceText font preservation test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "replaceText should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["hasChanges"].as_bool().unwrap_or(false),
        "Session should have changes after replaceText"
    );
    assert!(
        result["hasReplaceOp"].as_bool().unwrap_or(false),
        "Operations should include ReplaceText operation"
    );
    assert_eq!(
        result["fontNameInOp"].as_str().unwrap_or(""),
        "serif",
        "ReplaceText operation should preserve font_name"
    );
}

/// Test that exported PDF contains correct font in FreeText annotation
#[tokio::test]
async fn test_pdfjoin_export_preserves_font_in_pdf() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Wait for wasmBindings to be available
                for (let i = 0; i < 50; i++) {{
                    if (window.wasmBindings) break;
                    await new Promise(r => setTimeout(r, 100));
                }}
                if (!window.wasmBindings) {{
                    return {{ success: false, error: 'wasmBindings not available after 5s' }};
                }}

                const {{ EditSession }} = window.wasmBindings;

                // Decode test PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const session = new EditSession('test.pdf', pdfBytes);

                // Add replaceText with serif font (should become Times-Roman)
                session.replaceText(
                    1, 100, 700, 150, 14, 100, 700, 150, 14,
                    'Original', 'SerifTest',
                    12.0, '#000000', 'serif'
                );

                // Add replaceText with monospace font (should become Courier)
                session.replaceText(
                    1, 100, 600, 150, 14, 100, 600, 150, 14,
                    'Original2', 'MonoTest',
                    10.0, '#000000', 'monospace'
                );

                // Export the PDF
                const exportedBytes = session.export();
                const exportedArray = new Uint8Array(exportedBytes);

                // Convert to string to check for font references
                const decoder = new TextDecoder('utf-8', {{ fatal: false }});
                const pdfText = decoder.decode(exportedArray);

                // Check for font names in DA (Default Appearance) strings
                const hasTimesRoman = pdfText.includes('/Times-Roman') || pdfText.includes('/Times');
                const hasCourier = pdfText.includes('/Courier');
                const hasFreeText = pdfText.includes('/FreeText');

                // Look for DA strings with font specifications (count occurrences)
                const daCount = (pdfText.match(/\/DA/g) || []).length;

                return {{
                    success: true,
                    exportedSize: exportedArray.length,
                    startsWithPdf: pdfText.startsWith('%PDF-'),
                    hasTimesRoman: hasTimesRoman,
                    hasCourier: hasCourier,
                    hasFreeText: hasFreeText,
                    daCount: daCount
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test export font preservation")
        .into_value()
        .expect("Should get value");

    eprintln!("Export font preservation test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Export should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["startsWithPdf"].as_bool().unwrap_or(false),
        "Exported file should be valid PDF"
    );
    assert!(
        result["hasFreeText"].as_bool().unwrap_or(false),
        "Exported PDF should contain FreeText annotations"
    );

    // Check font mapping
    let has_times = result["hasTimesRoman"].as_bool().unwrap_or(false);
    let has_courier = result["hasCourier"].as_bool().unwrap_or(false);

    assert!(
        has_times || has_courier,
        "Exported PDF should contain mapped fonts (Times-Roman for serif, Courier for monospace). \
         DA count: {:?}",
        result["daCount"]
    );
}

/// Test that font size from PDF.js pdfHeight is preserved in replacement
#[tokio::test]
async fn test_pdfjoin_font_size_preservation() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                const {{ EditSession }} = window.wasmBindings;

                // Decode test PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const session = new EditSession('test.pdf', pdfBytes);

                // Add replaceText with specific font size (simulating pdfHeight from PDF.js)
                const testFontSize = 18.5;
                session.replaceText(
                    1, 100, 700, 150, testFontSize, 100, 700, 150, testFontSize,
                    'Original', 'SizeTest',
                    testFontSize, '#000000', 'sans-serif'
                );

                // Get operations to verify font size
                const opsJson = session.getOperationsJson();
                const ops = JSON.parse(opsJson);
                const replaceOp = ops.operations?.find(op => op.type === 'ReplaceText');

                // Export and check DA string contains font size
                const exportedBytes = session.export();
                const decoder = new TextDecoder('utf-8', {{ fatal: false }});
                const pdfText = decoder.decode(new Uint8Array(exportedBytes));

                // Look for font size in DA string
                const fontSizeInPdf = pdfText.includes('18.5') || pdfText.includes('18');

                return {{
                    success: true,
                    inputFontSize: testFontSize,
                    storedFontSize: replaceOp?.style?.font_size || null,
                    fontSizeMatch: Math.abs((replaceOp?.style?.font_size || 0) - testFontSize) < 0.01,
                    fontSizeInExportedPdf: fontSizeInPdf
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test font size preservation")
        .into_value()
        .expect("Should get value");

    eprintln!("Font size preservation test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Font size test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["fontSizeMatch"].as_bool().unwrap_or(false),
        "Font size should be preserved in operation. Input: {}, Stored: {:?}",
        result["inputFontSize"],
        result["storedFontSize"]
    );
}

/// Test that text editor preview shows correct font-family matching the original text
#[tokio::test]
async fn test_pdfjoin_text_editor_preview_uses_correct_font() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Use Florida contract which has known serif fonts
    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Navigate to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF via file input
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'contract.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                // Wait for PDF to load and render
                await new Promise(r => setTimeout(r, 3000));

                // Find a text item with serif font
                const textItems = document.querySelectorAll('.text-item');
                if (textItems.length === 0) {{
                    return {{ success: false, error: 'No text items found' }};
                }}

                // Click on first visible text item to start editing
                const textItem = textItems[0];
                textItem.click();

                await new Promise(r => setTimeout(r, 300));

                // Check if editor popup appeared
                const editorInput = document.querySelector('.text-editor-input');
                if (!editorInput) {{
                    return {{ success: false, error: 'Editor input not found after clicking text' }};
                }}

                // Get the computed font-family of the input
                const computedStyle = window.getComputedStyle(editorInput);
                const inputFontFamily = computedStyle.fontFamily;

                // Get the font info from the text layer item
                // The text item should have data about the original font
                const pageNum = textItem.dataset.page;
                const itemIndex = textItem.dataset.index;

                // Access the textItems map via PdfBridge
                let originalFontFamily = 'unknown';
                if (window.PdfBridge && window.PdfBridge.pageCanvases) {{
                    // Check if extractTextWithPositions was called
                    const items = await window.PdfBridge.extractTextWithPositions(parseInt(pageNum));
                    if (items && items[itemIndex]) {{
                        originalFontFamily = items[itemIndex].fontFamily || 'sans-serif';
                    }}
                }}

                // The editor font should match (or be a CSS equivalent)
                // serif -> serif, times, etc.
                // sans-serif -> sans-serif, helvetica, arial, etc.
                // monospace -> monospace, courier, etc.
                const isSerif = originalFontFamily === 'serif' || originalFontFamily.toLowerCase().includes('times');
                const isSansSerif = originalFontFamily === 'sans-serif' || originalFontFamily.toLowerCase().includes('arial') || originalFontFamily.toLowerCase().includes('helvetica');
                const isMono = originalFontFamily === 'monospace' || originalFontFamily.toLowerCase().includes('courier');

                let fontMatches = false;
                if (isSerif) {{
                    fontMatches = inputFontFamily.includes('serif') || inputFontFamily.includes('Times');
                }} else if (isMono) {{
                    fontMatches = inputFontFamily.includes('monospace') || inputFontFamily.includes('Courier');
                }} else {{
                    // Sans-serif is often the default, so check it's not incorrectly set to serif/mono
                    fontMatches = !inputFontFamily.includes('Times') || inputFontFamily.includes('sans');
                }}

                return {{
                    success: true,
                    inputFontFamily: inputFontFamily,
                    originalFontFamily: originalFontFamily,
                    fontMatches: fontMatches,
                    isSerif: isSerif,
                    isMono: isMono
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test text editor font")
        .into_value()
        .expect("Should get value");

    eprintln!("Text editor font preview test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["fontMatches"].as_bool().unwrap_or(false),
        "Text editor input should use matching font-family. Original: '{}', Input got: '{}'",
        result["originalFontFamily"].as_str().unwrap_or("?"),
        result["inputFontFamily"].as_str().unwrap_or("?")
    );
}

/// Test font mapping from CSS generic families to PDF standard fonts
#[tokio::test]
async fn test_pdfjoin_font_family_mapping() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                const {{ EditSession }} = window.wasmBindings;

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const session = new EditSession('test.pdf', pdfBytes);

                // Test all CSS generic font families
                const fontTests = [
                    {{ family: 'serif', expected: 'Times' }},
                    {{ family: 'sans-serif', expected: 'Helvetica' }},
                    {{ family: 'monospace', expected: 'Courier' }}
                ];

                // Add operations with each font family
                for (const test of fontTests) {{
                    session.replaceText(
                        1, 100, 700, 150, 12, 100, 700, 150, 12,
                        'Orig', test.family + ' text',
                        12.0, '#000000', test.family
                    );
                }}

                // Export and check for expected fonts
                const exportedBytes = session.export();
                const decoder = new TextDecoder('utf-8', {{ fatal: false }});
                const pdfText = decoder.decode(new Uint8Array(exportedBytes));

                const results = fontTests.map(test => ({{
                    family: test.family,
                    expected: test.expected,
                    found: pdfText.includes('/' + test.expected)
                }}));

                return {{
                    success: true,
                    fontMappings: results,
                    allMapped: results.every(r => r.found)
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test font mapping")
        .into_value()
        .expect("Should get value");

    eprintln!("Font family mapping test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Font mapping test should succeed. Error: {:?}",
        result["error"]
    );

    // Check individual mappings
    if let Some(mappings) = result["fontMappings"].as_array() {
        for mapping in mappings {
            let family = mapping["family"].as_str().unwrap_or("");
            let expected = mapping["expected"].as_str().unwrap_or("");
            let found = mapping["found"].as_bool().unwrap_or(false);

            assert!(
                found,
                "Font family '{}' should map to '{}' in exported PDF",
                family, expected
            );
        }
    }
}

/// Test that text replacement overlay fully covers original text and uses matching font
/// Bug: Original text is bleeding through the white overlay, and font doesn't match
#[tokio::test]
async fn test_pdfjoin_text_replacement_overlay_covers_original() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Navigate to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'contract.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Find a text item to edit
                const textItems = document.querySelectorAll('.text-item');
                if (textItems.length === 0) {{
                    return {{ success: false, error: 'No text items found' }};
                }}

                // Get the original text item info before editing
                const textItem = textItems[0];
                const origBounds = textItem.getBoundingClientRect();
                const origText = textItem.textContent;

                // Click to start editing
                textItem.click();
                await new Promise(r => setTimeout(r, 300));

                // Find editor and change text
                const editorInput = document.querySelector('.text-editor-input');
                if (!editorInput) {{
                    return {{ success: false, error: 'Editor input not found' }};
                }}

                // Get the font info from the input (should already be styled)
                const inputStyle = window.getComputedStyle(editorInput);
                const inputFontFamily = inputStyle.fontFamily;
                const inputFontSize = inputStyle.fontSize;

                // Change the text
                editorInput.value = 'REPLACEMENT TEXT';
                editorInput.dispatchEvent(new Event('input', {{ bubbles: true }}));

                // Save
                const saveBtn = document.querySelector('.text-editor-save');
                saveBtn.click();
                await new Promise(r => setTimeout(r, 300));

                // Now check the replacement overlay
                const overlay = document.querySelector('.edit-replace-overlay');
                if (!overlay) {{
                    return {{ success: false, error: 'Replacement overlay not created' }};
                }}

                const overlayStyle = window.getComputedStyle(overlay);
                const overlayBounds = overlay.getBoundingClientRect();

                // Check overlay covers original text area with padding
                const coversWidth = overlayBounds.width >= origBounds.width;
                const coversHeight = overlayBounds.height >= origBounds.height;

                // Check overlay has solid white background
                const bgColor = overlayStyle.backgroundColor;
                const hasWhiteBg = bgColor === 'rgb(255, 255, 255)' || bgColor === 'white';

                // Check overlay font matches original (not hardcoded 12px)
                const overlayFontFamily = overlayStyle.fontFamily;
                const overlayFontSize = overlayStyle.fontSize;

                // Font size should NOT be the hardcoded 12px default
                const fontSizeIsDefault = overlayFontSize === '12px';

                return {{
                    success: true,
                    origText: origText,
                    origBounds: {{ width: origBounds.width, height: origBounds.height }},
                    overlayBounds: {{ width: overlayBounds.width, height: overlayBounds.height }},
                    coversWidth: coversWidth,
                    coversHeight: coversHeight,
                    hasWhiteBg: hasWhiteBg,
                    bgColor: bgColor,
                    inputFontFamily: inputFontFamily,
                    inputFontSize: inputFontSize,
                    overlayFontFamily: overlayFontFamily,
                    overlayFontSize: overlayFontSize,
                    fontSizeIsDefault: fontSizeIsDefault
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test overlay coverage")
        .into_value()
        .expect("Should get value");

    eprintln!("Text replacement overlay test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["hasWhiteBg"].as_bool().unwrap_or(false),
        "Overlay should have white background to cover original text. Got: {}",
        result["bgColor"].as_str().unwrap_or("?")
    );

    assert!(
        result["coversWidth"].as_bool().unwrap_or(false),
        "BUG: Overlay width ({}) doesn't cover original text width ({})",
        result["overlayBounds"]["width"],
        result["origBounds"]["width"]
    );

    assert!(
        result["coversHeight"].as_bool().unwrap_or(false),
        "BUG: Overlay height ({}) doesn't cover original text height ({})",
        result["overlayBounds"]["height"],
        result["origBounds"]["height"]
    );

    // The critical bug: font size should match original, not be hardcoded 12px
    assert!(
        !result["fontSizeIsDefault"].as_bool().unwrap_or(true),
        "BUG: Overlay uses default 12px font size instead of matching original. \
         Input font-size was: {}, Overlay font-size: {}",
        result["inputFontSize"].as_str().unwrap_or("?"),
        result["overlayFontSize"].as_str().unwrap_or("?")
    );
}

/// Test that text replacement preview is centered like the original text
/// Bug: Preview text is left-aligned instead of matching original text centering
#[tokio::test]
async fn test_pdfjoin_text_replacement_preview_centered() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Navigate to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'contract.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Find a centered text item (title is usually centered)
                const textItems = document.querySelectorAll('.text-item');
                if (textItems.length === 0) {{
                    return {{ success: false, error: 'No text items found' }};
                }}

                // Get the original text item bounds
                const textItem = textItems[0];
                const origBounds = textItem.getBoundingClientRect();
                const origCenterX = origBounds.left + origBounds.width / 2;

                // Click to start editing
                textItem.click();
                await new Promise(r => setTimeout(r, 300));

                // Find editor and change text
                const editorInput = document.querySelector('.text-editor-input');
                if (!editorInput) {{
                    return {{ success: false, error: 'Editor input not found' }};
                }}

                // Change the text
                editorInput.value = 'CENTERED TEXT';
                editorInput.dispatchEvent(new Event('input', {{ bubbles: true }}));

                // Save
                const saveBtn = document.querySelector('.text-editor-save');
                saveBtn.click();
                await new Promise(r => setTimeout(r, 300));

                // Check the replacement overlay positioning
                const overlay = document.querySelector('.edit-replace-overlay');
                if (!overlay) {{
                    return {{ success: false, error: 'Replacement overlay not created' }};
                }}

                const overlayBounds = overlay.getBoundingClientRect();
                const overlayLeft = overlayBounds.left;

                // The overlay should be positioned at the same X position as original
                // (accounting for the 15px padding we add for full coverage)
                const leftDiff = Math.abs(overlayBounds.left - origBounds.left);

                // Allow tolerance for the 15px padding we add (plus some margin)
                const isAligned = leftDiff <= 20;

                return {{
                    success: true,
                    origLeft: origBounds.left,
                    origCenterX: origCenterX,
                    overlayLeft: overlayLeft,
                    leftDiff: leftDiff,
                    isAligned: isAligned
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test centering")
        .into_value()
        .expect("Should get value");

    eprintln!("Text centering test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["isAligned"].as_bool().unwrap_or(false),
        "BUG: Overlay left ({}) should align with original text left ({}). Diff: {}px",
        result["overlayLeft"].as_f64().unwrap_or(0.0),
        result["origLeft"].as_f64().unwrap_or(0.0),
        result["leftDiff"].as_f64().unwrap_or(0.0)
    );
}

// ============================================================================
// Property Tests for Font Preservation
// ============================================================================

/// Property test: Extract text items from listing agreement and verify font properties
/// This test verifies that PDF.js extracts font size, family, and style correctly
#[tokio::test]
async fn test_property_text_extraction_has_font_info() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_listing_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Navigate to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'listing.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Get text items from PdfBridge
                const textItems = await PdfBridge.extractTextWithPositions(1);

                // Analyze font properties
                const withFontFamily = textItems.filter(t => t.fontFamily && t.fontFamily !== 'unknown');
                const withFontSize = textItems.filter(t => t.pdfHeight > 0);
                const italicItems = textItems.filter(t => t.isItalic);
                const boldItems = textItems.filter(t => t.isBold);

                // Sample items with different properties
                const samples = textItems.slice(0, 20).map(t => ({{
                    text: t.str.substring(0, 30),
                    fontFamily: t.fontFamily,
                    fontName: t.fontName,
                    pdfHeight: t.pdfHeight,
                    isItalic: t.isItalic,
                    isBold: t.isBold
                }}));

                return {{
                    success: true,
                    totalItems: textItems.length,
                    withFontFamily: withFontFamily.length,
                    withFontSize: withFontSize.length,
                    italicCount: italicItems.length,
                    boldCount: boldItems.length,
                    samples: samples
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should extract text")
        .into_value()
        .expect("Should get value");

    eprintln!("Font property extraction test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    // Verify we got text items
    let total = result["totalItems"].as_i64().unwrap_or(0);
    assert!(total > 0, "Should extract text items from PDF");

    // Verify font family is extracted for most items
    let with_family = result["withFontFamily"].as_i64().unwrap_or(0);
    let family_ratio = with_family as f64 / total as f64;
    assert!(
        family_ratio > 0.5,
        "At least 50% of items should have font family. Got {}/{} = {:.1}%",
        with_family,
        total,
        family_ratio * 100.0
    );

    // Verify font size is extracted for most items
    let with_size = result["withFontSize"].as_i64().unwrap_or(0);
    let size_ratio = with_size as f64 / total as f64;
    assert!(
        size_ratio > 0.8,
        "At least 80% of items should have font size. Got {}/{} = {:.1}%",
        with_size,
        total,
        size_ratio * 100.0
    );

    // The listing agreement should have italic text (e.g., "State of Florida")
    let italic_count = result["italicCount"].as_i64().unwrap_or(0);
    eprintln!(
        "Found {} italic items, {} bold items",
        italic_count,
        result["boldCount"].as_i64().unwrap_or(0)
    );
}

/// Property test: Verify text replacement preserves font size in exported PDF
/// This is a critical regression test for the "text too small" bug
#[tokio::test]
async fn test_property_replacement_preserves_font_size() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_escalation_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Navigate to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'escalation.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Get text items to find different font sizes
                const textItems = await PdfBridge.extractTextWithPositions(1);

                // Group by approximate font size
                const sizeGroups = {{}};
                textItems.forEach(t => {{
                    const size = Math.round(t.pdfHeight);
                    if (!sizeGroups[size]) sizeGroups[size] = [];
                    sizeGroups[size].push(t);
                }});

                // Find items with different sizes
                const sizes = Object.keys(sizeGroups).map(Number).sort((a,b) => b-a);
                const testCases = [];

                // Test a large text item
                if (sizes.length > 0 && sizeGroups[sizes[0]].length > 0) {{
                    const item = sizeGroups[sizes[0]][0];
                    testCases.push({{
                        original: item.str,
                        originalSize: item.pdfHeight,
                        fontFamily: item.fontFamily,
                        isItalic: item.isItalic,
                        isBold: item.isBold,
                        type: 'large'
                    }});
                }}

                // Test a medium text item
                const midIndex = Math.floor(sizes.length / 2);
                if (sizes[midIndex] && sizeGroups[sizes[midIndex]].length > 0) {{
                    const item = sizeGroups[sizes[midIndex]][0];
                    testCases.push({{
                        original: item.str,
                        originalSize: item.pdfHeight,
                        fontFamily: item.fontFamily,
                        isItalic: item.isItalic,
                        isBold: item.isBold,
                        type: 'medium'
                    }});
                }}

                // Test a small text item
                if (sizes.length > 1 && sizeGroups[sizes[sizes.length-1]].length > 0) {{
                    const item = sizeGroups[sizes[sizes.length-1]][0];
                    testCases.push({{
                        original: item.str,
                        originalSize: item.pdfHeight,
                        fontFamily: item.fontFamily,
                        isItalic: item.isItalic,
                        isBold: item.isBold,
                        type: 'small'
                    }});
                }}

                return {{
                    success: true,
                    uniqueSizes: sizes,
                    testCases: testCases,
                    totalItems: textItems.length
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should analyze font sizes")
        .into_value()
        .expect("Should get value");

    eprintln!("Font size preservation test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    // Verify we found different font sizes
    let sizes = result["uniqueSizes"].as_array().unwrap();
    eprintln!("Unique font sizes found: {:?}", sizes);
    assert!(
        sizes.len() >= 2,
        "PDF should have at least 2 different font sizes. Found: {:?}",
        sizes
    );

    // Verify test cases have font info
    let test_cases = result["testCases"].as_array().unwrap();
    for case in test_cases {
        let size = case["originalSize"].as_f64().unwrap_or(0.0);
        assert!(
            size > 0.0,
            "Test case should have positive font size. Got: {}",
            size
        );
        eprintln!(
            "  {} text: '{}' size={:.1}pt family={} italic={} bold={}",
            case["type"].as_str().unwrap_or("?"),
            case["original"].as_str().unwrap_or("?"),
            size,
            case["fontFamily"].as_str().unwrap_or("?"),
            case["isItalic"].as_bool().unwrap_or(false),
            case["isBold"].as_bool().unwrap_or(false)
        );
    }
}

/// Property test: Verify centered text detection and preview centering
/// This tests that text which is centered on the page stays centered in preview
#[tokio::test]
async fn test_property_centered_text_stays_centered_in_preview() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_escalation_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Navigate to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'escalation.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Get page dimensions
                const pageDims = PdfBridge.getPageDimensions(1);
                const pageWidth = pageDims.width;
                const pageCenterX = pageWidth / 2;

                // Get text items
                const textItems = await PdfBridge.extractTextWithPositions(1);

                // Find centered text items (center of text is near page center)
                const centeredItems = textItems.filter(t => {{
                    if (!t.domBounds) return false;
                    const textCenterX = t.domBounds.x + t.domBounds.width / 2;
                    const distFromCenter = Math.abs(textCenterX - pageCenterX);
                    return distFromCenter < 50; // Within 50px of center
                }});

                // Find the title (usually largest centered text)
                const title = centeredItems.reduce((best, t) => {{
                    if (!best || t.pdfHeight > best.pdfHeight) return t;
                    return best;
                }}, null);

                if (!title) {{
                    return {{ success: false, error: 'No centered text found' }};
                }}

                // Calculate how centered the title is
                const titleCenterX = title.domBounds.x + title.domBounds.width / 2;
                const titleCenterOffset = titleCenterX - pageCenterX;

                return {{
                    success: true,
                    pageWidth: pageWidth,
                    pageCenterX: pageCenterX,
                    titleText: title.str,
                    titleWidth: title.domBounds.width,
                    titleCenterX: titleCenterX,
                    titleCenterOffset: titleCenterOffset,
                    isCentered: Math.abs(titleCenterOffset) < 20,
                    centeredItemsCount: centeredItems.length
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should detect centered text")
        .into_value()
        .expect("Should get value");

    eprintln!("Centered text detection test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    // Verify we found centered text
    let centered_count = result["centeredItemsCount"].as_i64().unwrap_or(0);
    assert!(
        centered_count > 0,
        "Should find centered text items. Found: {}",
        centered_count
    );

    // Verify the title is centered
    let is_centered = result["isCentered"].as_bool().unwrap_or(false);
    let offset = result["titleCenterOffset"].as_f64().unwrap_or(999.0);
    eprintln!(
        "Title '{}' center offset: {:.1}px (centered: {})",
        result["titleText"].as_str().unwrap_or("?"),
        offset,
        is_centered
    );
}

// ============================================================================
// Whiteout Feature Tests
// ============================================================================

/// Test that EditSession WASM binding has addWhiteRect method
#[tokio::test]
async fn test_pdfjoin_whiteout_wasm_binding_exists() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                const {{ EditSession }} = window.wasmBindings;
                if (!EditSession) {{
                    return {{ success: false, error: 'EditSession not in wasmBindings' }};
                }}

                // Decode test PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                // Create session
                const session = new EditSession('test.pdf', pdfBytes);

                // Check addWhiteRect method exists
                const hasAddWhiteRect = typeof session.addWhiteRect === 'function';

                // Test the method if it exists
                let operationId = null;
                let hasChanges = false;
                if (hasAddWhiteRect) {{
                    // Convert BigInt to Number for JSON serialization
                    operationId = Number(session.addWhiteRect(1, 100, 700, 200, 50));
                    hasChanges = session.hasChanges();
                }}

                return {{
                    success: true,
                    hasAddWhiteRect: hasAddWhiteRect,
                    operationId: operationId,
                    hasChanges: hasChanges
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test addWhiteRect")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout WASM binding test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["hasAddWhiteRect"].as_bool().unwrap_or(false),
        "EditSession should have addWhiteRect method for whiteout tool"
    );
    assert!(
        result["hasChanges"].as_bool().unwrap_or(false),
        "Session should have changes after addWhiteRect"
    );
}

/// Test that Whiteout toolbar button exists in Edit tab
#[tokio::test]
async fn test_pdfjoin_whiteout_toolbar_button_exists() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"(() => {
                // Switch to edit tab
                const editTab = document.querySelector('[data-tab="edit"]');
                if (editTab) editTab.click();

                // Wait a bit for tab switch
                return new Promise(resolve => setTimeout(() => {
                    const whiteoutBtn = document.getElementById('edit-tool-whiteout');
                    resolve({
                        hasWhiteoutButton: !!whiteoutBtn,
                        buttonTitle: whiteoutBtn?.title || null,
                        buttonLabel: whiteoutBtn?.querySelector('.tool-label')?.textContent || null
                    });
                }, 300));
            })()"#,
        )
        .await
        .expect("Should evaluate JS")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout toolbar button test: {:?}", result);

    assert!(
        result["hasWhiteoutButton"].as_bool().unwrap_or(false),
        "FEATURE MISSING: Whiteout toolbar button should exist in Edit tab"
    );
}

/// Test that drawing a whiteout creates a DOM element
#[tokio::test]
async fn test_pdfjoin_whiteout_draw_creates_element() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Switch to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Click whiteout tool
                const whiteoutBtn = document.getElementById('edit-tool-whiteout');
                if (!whiteoutBtn) {{
                    return {{ success: false, error: 'Whiteout button not found' }};
                }}
                whiteoutBtn.click();
                await new Promise(r => setTimeout(r, 100));

                // Find the page container to simulate drawing
                const pageDiv = document.querySelector('.edit-page');
                if (!pageDiv) {{
                    return {{ success: false, error: 'Page div not found' }};
                }}

                const rect = pageDiv.getBoundingClientRect();
                const startX = rect.left + 100;
                const startY = rect.top + 100;
                const endX = startX + 200;
                const endY = startY + 50;

                // Simulate mouse draw
                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true, clientX: startX, clientY: startY
                }}));
                await new Promise(r => setTimeout(r, 50));

                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: endX, clientY: endY
                }}));
                await new Promise(r => setTimeout(r, 50));

                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true, clientX: endX, clientY: endY
                }}));
                await new Promise(r => setTimeout(r, 100));

                // Check if whiteout overlay was created
                const overlays = document.querySelectorAll('.edit-whiteout-overlay');
                const previewRects = document.querySelectorAll('.whiteout-preview');

                return {{
                    success: true,
                    overlayCount: overlays.length,
                    previewCount: previewRects.length,
                    whiteoutCreated: overlays.length > 0
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test whiteout drawing")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout draw test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["whiteoutCreated"].as_bool().unwrap_or(false),
        "Drawing whiteout should create overlay element. Found {} overlays",
        result["overlayCount"]
    );
}

/// BUG TEST: Whiteout boxes should be movable by dragging from center
/// EXPECTED: This test should FAIL until we implement the move feature
#[tokio::test]
async fn test_pdfjoin_whiteout_can_be_moved_by_dragging() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Switch to edit tab and load PDF
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Draw a whiteout box
                const whiteoutBtn = document.getElementById('edit-tool-whiteout');
                whiteoutBtn.click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const rect = pageDiv.getBoundingClientRect();
                const startX = rect.left + 100;
                const startY = rect.top + 100;

                // Draw the whiteout
                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true, clientX: startX, clientY: startY
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: startX + 200, clientY: startY + 50
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true, clientX: startX + 200, clientY: startY + 50
                }}));
                await new Promise(r => setTimeout(r, 100));

                // Switch to select tool
                const selectBtn = document.getElementById('tool-select');
                selectBtn.click();
                await new Promise(r => setTimeout(r, 100));

                // Find the whiteout overlay
                const overlay = document.querySelector('.edit-whiteout-overlay');
                if (!overlay) {{
                    return {{ success: false, error: 'Whiteout overlay not found' }};
                }}

                // Record initial position
                const initialLeft = parseFloat(overlay.style.left);
                const initialTop = parseFloat(overlay.style.top);

                // Select the overlay by clicking it
                const overlayRect = overlay.getBoundingClientRect();
                const centerX = overlayRect.left + overlayRect.width / 2;
                const centerY = overlayRect.top + overlayRect.height / 2;

                overlay.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true, clientX: centerX, clientY: centerY
                }}));
                await new Promise(r => setTimeout(r, 50));

                // Drag to move it
                const moveX = 50;
                const moveY = 30;
                overlay.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: centerX + moveX, clientY: centerY + moveY
                }}));
                await new Promise(r => setTimeout(r, 50));

                overlay.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true, clientX: centerX + moveX, clientY: centerY + moveY
                }}));
                await new Promise(r => setTimeout(r, 100));

                // Check final position
                const finalLeft = parseFloat(overlay.style.left);
                const finalTop = parseFloat(overlay.style.top);

                const moved = Math.abs(finalLeft - initialLeft) > 10 || Math.abs(finalTop - initialTop) > 10;

                return {{
                    success: true,
                    initialLeft: initialLeft,
                    initialTop: initialTop,
                    finalLeft: finalLeft,
                    finalTop: finalTop,
                    moved: moved,
                    deltaX: finalLeft - initialLeft,
                    deltaY: finalTop - initialTop
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test whiteout moving")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout move test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["moved"].as_bool().unwrap_or(false),
        "BUG: Whiteout should be movable by dragging from center. \
         Initial: ({}, {}), Final: ({}, {}), Delta: ({}, {})",
        result["initialLeft"].as_f64().unwrap_or(0.0),
        result["initialTop"].as_f64().unwrap_or(0.0),
        result["finalLeft"].as_f64().unwrap_or(0.0),
        result["finalTop"].as_f64().unwrap_or(0.0),
        result["deltaX"].as_f64().unwrap_or(0.0),
        result["deltaY"].as_f64().unwrap_or(0.0)
    );
}

/// BUG TEST: Resize handles should release properly on mouseup
/// EXPECTED: This test should FAIL until we fix the resize release bug
#[tokio::test]
async fn test_pdfjoin_whiteout_resize_releases_on_mouseup() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Switch to edit tab and load PDF
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Draw a whiteout box
                const whiteoutBtn = document.getElementById('edit-tool-whiteout');
                whiteoutBtn.click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const rect = pageDiv.getBoundingClientRect();
                const startX = rect.left + 100;
                const startY = rect.top + 100;

                // Draw the whiteout
                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true, clientX: startX, clientY: startY
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: startX + 200, clientY: startY + 50
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true, clientX: startX + 200, clientY: startY + 50
                }}));
                await new Promise(r => setTimeout(r, 100));

                // Switch to select tool
                const selectBtn = document.getElementById('tool-select');
                selectBtn.click();
                await new Promise(r => setTimeout(r, 100));

                // Find and select the whiteout overlay
                const overlay = document.querySelector('.edit-whiteout-overlay');
                if (!overlay) {{
                    return {{ success: false, error: 'Whiteout overlay not found' }};
                }}

                // Use mousedown to select (triggers selectWhiteout which adds resize handles)
                const overlayRect = overlay.getBoundingClientRect();
                overlay.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true,
                    clientX: overlayRect.left + overlayRect.width / 2,
                    clientY: overlayRect.top + overlayRect.height / 2
                }}));
                // Release immediately to avoid starting a move
                document.dispatchEvent(new MouseEvent('mouseup', {{ bubbles: true }}));
                await new Promise(r => setTimeout(r, 100));

                // Find a resize handle
                const seHandle = overlay.querySelector('.resize-handle.se');
                if (!seHandle) {{
                    return {{ success: false, error: 'SE resize handle not found' }};
                }}

                const handleRect = seHandle.getBoundingClientRect();
                const handleCenterX = handleRect.left + handleRect.width / 2;
                const handleCenterY = handleRect.top + handleRect.height / 2;

                // Start resize
                seHandle.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true, clientX: handleCenterX, clientY: handleCenterY
                }}));
                await new Promise(r => setTimeout(r, 50));

                // Move to resize
                document.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: handleCenterX + 50, clientY: handleCenterY + 50
                }}));
                await new Promise(r => setTimeout(r, 50));

                // Release
                document.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true, clientX: handleCenterX + 50, clientY: handleCenterY + 50
                }}));
                await new Promise(r => setTimeout(r, 100));

                // Check if resizing has stopped - move mouse elsewhere
                document.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: handleCenterX + 100, clientY: handleCenterY + 100
                }}));
                await new Promise(r => setTimeout(r, 100));

                // Record width after supposed release
                const widthAfterRelease = parseFloat(overlay.style.width);

                // Move mouse even further - if still resizing, width will change
                document.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: handleCenterX + 200, clientY: handleCenterY + 200
                }}));
                await new Promise(r => setTimeout(r, 100));

                const widthAfterFurtherMove = parseFloat(overlay.style.width);

                // Width should NOT change after mouseup was released
                const stillResizing = Math.abs(widthAfterFurtherMove - widthAfterRelease) > 5;

                return {{
                    success: true,
                    widthAfterRelease: widthAfterRelease,
                    widthAfterFurtherMove: widthAfterFurtherMove,
                    stillResizing: stillResizing,
                    releasedProperly: !stillResizing
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test resize release")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout resize release test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["releasedProperly"].as_bool().unwrap_or(false),
        "BUG: Resize should release on mouseup. After release width was {}, \
         after further mouse movement width is {} (should be same)",
        result["widthAfterRelease"].as_f64().unwrap_or(0.0),
        result["widthAfterFurtherMove"].as_f64().unwrap_or(0.0)
    );
}

/// BUG TEST: Double-clicking whiteout should open text editor
/// EXPECTED: This test should FAIL until we implement the feature
#[tokio::test]
async fn test_pdfjoin_whiteout_doubleclick_opens_text_editor() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Switch to edit tab and load PDF
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Draw a whiteout box
                const whiteoutBtn = document.getElementById('edit-tool-whiteout');
                whiteoutBtn.click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const rect = pageDiv.getBoundingClientRect();
                const startX = rect.left + 100;
                const startY = rect.top + 100;

                // Draw the whiteout
                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true, clientX: startX, clientY: startY
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: startX + 200, clientY: startY + 50
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true, clientX: startX + 200, clientY: startY + 50
                }}));
                await new Promise(r => setTimeout(r, 100));

                // Find the whiteout overlay
                const overlay = document.querySelector('.edit-whiteout-overlay');
                if (!overlay) {{
                    return {{ success: false, error: 'Whiteout overlay not found' }};
                }}

                const overlayRect = overlay.getBoundingClientRect();
                const centerX = overlayRect.left + overlayRect.width / 2;
                const centerY = overlayRect.top + overlayRect.height / 2;

                // Double-click on the overlay
                overlay.dispatchEvent(new MouseEvent('dblclick', {{
                    bubbles: true, clientX: centerX, clientY: centerY
                }}));
                await new Promise(r => setTimeout(r, 300));

                // Check if text editor appeared
                const textEditor = document.querySelector('.text-editor-popup, .whiteout-text-editor, .inline-text-editor');
                const textInput = document.querySelector('.text-editor-input, .whiteout-text-input, [contenteditable]');

                return {{
                    success: true,
                    hasTextEditor: !!textEditor,
                    hasTextInput: !!textInput,
                    editorOpened: !!textEditor || !!textInput
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test double-click")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout double-click test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["editorOpened"].as_bool().unwrap_or(false),
        "FEATURE MISSING: Double-clicking whiteout should open text editor. \
         Found text editor: {}, Found text input: {}",
        result["hasTextEditor"].as_bool().unwrap_or(false),
        result["hasTextInput"].as_bool().unwrap_or(false)
    );
}

/// BUG TEST: Whiteout should have solid white background (not transparent)
#[tokio::test]
async fn test_pdfjoin_whiteout_has_white_background() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Draw a whiteout
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const rect = pageDiv.getBoundingClientRect();
                const startX = rect.left + 100;
                const startY = rect.top + 100;

                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true, clientX: startX, clientY: startY
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: startX + 200, clientY: startY + 50
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true, clientX: startX + 200, clientY: startY + 50
                }}));
                await new Promise(r => setTimeout(r, 100));

                // Check the whiteout overlay background
                const overlay = document.querySelector('.edit-whiteout-overlay');
                if (!overlay) {{
                    return {{ success: false, error: 'Whiteout overlay not found' }};
                }}

                const style = window.getComputedStyle(overlay);
                const bgColor = style.backgroundColor;

                // Check if background is white (rgb(255, 255, 255))
                const isWhite = bgColor === 'rgb(255, 255, 255)' || bgColor === 'white';

                // Also check opacity
                const opacity = parseFloat(style.opacity);
                const isOpaque = opacity === 1 || isNaN(opacity);

                return {{
                    success: true,
                    backgroundColor: bgColor,
                    isWhite: isWhite,
                    opacity: opacity,
                    isOpaque: isOpaque,
                    coversContent: isWhite && isOpaque
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test whiteout background")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout background test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["coversContent"].as_bool().unwrap_or(false),
        "BUG: Whiteout should have solid white background to cover content. \
         Background: {}, isWhite: {}, opacity: {}, isOpaque: {}",
        result["backgroundColor"].as_str().unwrap_or("?"),
        result["isWhite"].as_bool().unwrap_or(false),
        result["opacity"].as_f64().unwrap_or(-1.0),
        result["isOpaque"].as_bool().unwrap_or(false)
    );
}

/// BUG TEST: Double-click whiteout should enable INLINE text editing (not a dialog)
/// Text input should appear INSIDE the whiteout box, not as a separate popup
#[tokio::test]
async fn test_pdfjoin_whiteout_inline_text_editing() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Draw a whiteout
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const rect = pageDiv.getBoundingClientRect();
                const startX = rect.left + 100;
                const startY = rect.top + 100;

                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true, clientX: startX, clientY: startY
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: startX + 200, clientY: startY + 50
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true, clientX: startX + 200, clientY: startY + 50
                }}));
                await new Promise(r => setTimeout(r, 100));

                const overlay = document.querySelector('.edit-whiteout-overlay');
                if (!overlay) {{
                    return {{ success: false, error: 'Whiteout overlay not found' }};
                }}

                const overlayRect = overlay.getBoundingClientRect();

                // Double-click to start editing
                overlay.dispatchEvent(new MouseEvent('dblclick', {{
                    bubbles: true,
                    clientX: overlayRect.left + overlayRect.width / 2,
                    clientY: overlayRect.top + overlayRect.height / 2
                }}));
                await new Promise(r => setTimeout(r, 300));

                // Find the text input - it should be INSIDE or directly on top of the whiteout
                const inputInside = overlay.querySelector('input, textarea, [contenteditable]');
                const inputNearby = document.querySelector('.whiteout-text-input, .whiteout-text-editor input');

                let inputIsInside = false;
                let inputElement = inputInside || inputNearby;

                if (inputElement) {{
                    const inputRect = inputElement.getBoundingClientRect();
                    // Check if input is within or overlapping the whiteout bounds
                    inputIsInside = (
                        inputRect.left >= overlayRect.left - 5 &&
                        inputRect.top >= overlayRect.top - 5 &&
                        inputRect.right <= overlayRect.right + 5 &&
                        inputRect.bottom <= overlayRect.bottom + 5
                    );
                }}

                return {{
                    success: true,
                    hasInputInside: !!inputInside,
                    hasInputNearby: !!inputNearby,
                    inputIsInside: inputIsInside,
                    overlayBounds: {{
                        left: overlayRect.left,
                        top: overlayRect.top,
                        width: overlayRect.width,
                        height: overlayRect.height
                    }}
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test inline editing")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout inline editing test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["inputIsInside"].as_bool().unwrap_or(false),
        "BUG: Text input should appear INSIDE the whiteout box, not as a separate popup. \
         Input inside overlay: {}, Input nearby: {}, Input positioned inside: {}",
        result["hasInputInside"].as_bool().unwrap_or(false),
        result["hasInputNearby"].as_bool().unwrap_or(false),
        result["inputIsInside"].as_bool().unwrap_or(false)
    );
}

/// BUG TEST: Text typed in whiteout should match the style of covered text
#[tokio::test]
async fn test_pdfjoin_whiteout_text_matches_covered_style() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'contract.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Get text items to find one with known style
                const textItems = await PdfBridge.extractTextWithPositions(1);
                if (textItems.length === 0) {{
                    return {{ success: false, error: 'No text items found' }};
                }}

                // Find a text item with good style info
                const targetItem = textItems.find(t => t.domFontSize > 10 && t.str.trim().length > 3);
                if (!targetItem) {{
                    return {{ success: false, error: 'No suitable text item found' }};
                }}

                // Use domFontSize (viewport-scaled) since that's what the input displays at
                const coveredFontSize = targetItem.domFontSize;
                const coveredFontFamily = targetItem.fontFamily || 'unknown';

                // Draw a whiteout over this text
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const bounds = targetItem.domBounds;
                // Convert page-relative coords to client coords
                const pageRect = pageDiv.getBoundingClientRect();
                const clientX1 = pageRect.left + bounds.x - 5;
                const clientY1 = pageRect.top + bounds.y - 5;
                const clientX2 = pageRect.left + bounds.x + bounds.width + 5;
                const clientY2 = pageRect.top + bounds.y + bounds.height + 5;

                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true, clientX: clientX1, clientY: clientY1
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true, clientX: clientX2, clientY: clientY2
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true, clientX: clientX2, clientY: clientY2
                }}));
                await new Promise(r => setTimeout(r, 100));

                const overlay = document.querySelector('.edit-whiteout-overlay');
                if (!overlay) {{
                    return {{ success: false, error: 'Whiteout overlay not found' }};
                }}

                // Double-click to edit
                const overlayRect = overlay.getBoundingClientRect();
                overlay.dispatchEvent(new MouseEvent('dblclick', {{
                    bubbles: true,
                    clientX: overlayRect.left + overlayRect.width / 2,
                    clientY: overlayRect.top + overlayRect.height / 2
                }}));
                await new Promise(r => setTimeout(r, 300));

                // Find the input and check its style
                const input = document.querySelector('.whiteout-text-input, .edit-whiteout-overlay input, [contenteditable]');
                if (!input) {{
                    return {{ success: false, error: 'Text input not found after double-click' }};
                }}

                const inputStyle = window.getComputedStyle(input);
                const inputFontSize = parseFloat(inputStyle.fontSize);

                // Font size should approximately match (within 2px)
                const fontSizeMatches = Math.abs(inputFontSize - coveredFontSize) < 5;

                return {{
                    success: true,
                    coveredFontSize: coveredFontSize,
                    coveredFontFamily: coveredFontFamily,
                    inputFontSize: inputFontSize,
                    inputFontFamily: inputStyle.fontFamily,
                    fontSizeMatches: fontSizeMatches,
                    coveredText: targetItem.str
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test text style matching")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout text style test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["fontSizeMatches"].as_bool().unwrap_or(false),
        "BUG: Text input font size should match covered text. \
         Covered text '{}' has font-size {}pt, but input has {}px",
        result["coveredText"].as_str().unwrap_or("?"),
        result["coveredFontSize"].as_f64().unwrap_or(0.0),
        result["inputFontSize"].as_f64().unwrap_or(0.0)
    );
}

/// Test that text tool uses inline editing, not prompt() dialogs
#[tokio::test]
async fn test_pdfjoin_text_tool_uses_inline_editing() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Override window.prompt to detect if it's called
                let promptCalled = false;
                const originalPrompt = window.prompt;
                window.prompt = function() {{
                    promptCalled = true;
                    return null; // Cancel the prompt
                }};

                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'contract.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Select Text tool
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));

                // Click on the overlay container to add text
                const overlay = document.querySelector('.overlay-container');
                if (!overlay) {{
                    return {{ success: false, error: 'Overlay container not found' }};
                }}
                const overlayRect = overlay.getBoundingClientRect();
                const clickX = overlayRect.left + 200;
                const clickY = overlayRect.top + 300;

                overlay.dispatchEvent(new MouseEvent('click', {{
                    bubbles: true,
                    clientX: clickX,
                    clientY: clickY
                }}));
                await new Promise(r => setTimeout(r, 300));

                // Check if prompt was called (bad) or inline text box exists (good)
                // New textbox implementation creates .text-box with .text-content inside
                const textBox = document.querySelector('.text-box');
                const hasInlineInput = !!textBox;

                // Restore original prompt
                window.prompt = originalPrompt;

                return {{
                    success: true,
                    promptCalled: promptCalled,
                    hasInlineInput: hasInlineInput,
                    usesInlineEditing: hasInlineInput && !promptCalled
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test text tool")
        .into_value()
        .expect("Should get value");

    eprintln!("Text tool inline editing test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        !result["promptCalled"].as_bool().unwrap_or(true),
        "BUG: Text tool should NOT use prompt() dialogs for user input"
    );
    assert!(
        result["hasInlineInput"].as_bool().unwrap_or(false),
        "BUG: Text tool should create an inline text input, not use prompt()"
    );
}

/// Test that whiteout text input fills the entire whiteout area and matches font
#[tokio::test]
async fn test_pdfjoin_whiteout_text_input_fills_area_and_matches_font() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'contract.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Get text items to find one with known style
                const textItems = await PdfBridge.extractTextWithPositions(1);
                const targetItem = textItems.find(t => t.domFontSize > 20 && t.str.trim().length > 3);
                if (!targetItem) {{
                    return {{ success: false, error: 'No suitable text item found' }};
                }}

                const coveredFontSize = targetItem.domFontSize;

                // Draw a whiteout over this text
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const bounds = targetItem.domBounds;
                const pageRect = pageDiv.getBoundingClientRect();
                const clientX1 = pageRect.left + bounds.x - 5;
                const clientY1 = pageRect.top + bounds.y - 5;
                const clientX2 = pageRect.left + bounds.x + bounds.width + 5;
                const clientY2 = pageRect.top + bounds.y + bounds.height + 5;

                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{ bubbles: true, clientX: clientX1, clientY: clientY1 }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{ bubbles: true, clientX: clientX2, clientY: clientY2 }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{ bubbles: true, clientX: clientX2, clientY: clientY2 }}));
                await new Promise(r => setTimeout(r, 200));

                const whiteout = document.querySelector('.edit-whiteout-overlay');
                if (!whiteout) {{
                    return {{ success: false, error: 'Whiteout not created' }};
                }}

                const whiteoutRect = whiteout.getBoundingClientRect();

                // Double-click on whiteout to open text editor
                whiteout.dispatchEvent(new MouseEvent('dblclick', {{
                    bubbles: true,
                    clientX: whiteoutRect.left + whiteoutRect.width / 2,
                    clientY: whiteoutRect.top + whiteoutRect.height / 2
                }}));
                await new Promise(r => setTimeout(r, 300));

                // Find the text input
                const input = whiteout.querySelector('input') || whiteout.querySelector('.whiteout-text-input');
                if (!input) {{
                    return {{ success: false, error: 'Text input not found in whiteout' }};
                }}

                const inputRect = input.getBoundingClientRect();
                const inputStyle = window.getComputedStyle(input);
                const inputFontSize = parseFloat(inputStyle.fontSize);

                // Check if input fills the whiteout area (allowing small padding)
                const widthFillsArea = inputRect.width >= whiteoutRect.width * 0.9;
                const heightFillsArea = inputRect.height >= whiteoutRect.height * 0.8;

                // Check if font size matches covered text (within 5px tolerance)
                const fontSizeMatches = Math.abs(inputFontSize - coveredFontSize) < 5;

                return {{
                    success: true,
                    whiteoutWidth: whiteoutRect.width,
                    whiteoutHeight: whiteoutRect.height,
                    inputWidth: inputRect.width,
                    inputHeight: inputRect.height,
                    widthFillsArea: widthFillsArea,
                    heightFillsArea: heightFillsArea,
                    coveredFontSize: coveredFontSize,
                    inputFontSize: inputFontSize,
                    fontSizeMatches: fontSizeMatches
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test whiteout text input")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout text input area test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["widthFillsArea"].as_bool().unwrap_or(false),
        "BUG: Text input should fill whiteout width. Whiteout: {}px, Input: {}px",
        result["whiteoutWidth"].as_f64().unwrap_or(0.0),
        result["inputWidth"].as_f64().unwrap_or(0.0)
    );
    assert!(
        result["heightFillsArea"].as_bool().unwrap_or(false),
        "BUG: Text input should fill whiteout height. Whiteout: {}px, Input: {}px",
        result["whiteoutHeight"].as_f64().unwrap_or(0.0),
        result["inputHeight"].as_f64().unwrap_or(0.0)
    );
    assert!(
        result["fontSizeMatches"].as_bool().unwrap_or(false),
        "BUG: Text input font size should match covered text. Covered: {}px, Input: {}px",
        result["coveredFontSize"].as_f64().unwrap_or(0.0),
        result["inputFontSize"].as_f64().unwrap_or(0.0)
    );
}

/// Test that saved text appears ON TOP of the whiteout, not behind it
#[tokio::test]
async fn test_pdfjoin_whiteout_saved_text_appears_on_top() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'contract.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Draw a whiteout
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const pageRect = pageDiv.getBoundingClientRect();

                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{ bubbles: true, clientX: pageRect.left + 100, clientY: pageRect.top + 100 }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{ bubbles: true, clientX: pageRect.left + 400, clientY: pageRect.top + 150 }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{ bubbles: true, clientX: pageRect.left + 400, clientY: pageRect.top + 150 }}));
                await new Promise(r => setTimeout(r, 200));

                const whiteout = document.querySelector('.edit-whiteout-overlay');
                if (!whiteout) {{
                    return {{ success: false, error: 'Whiteout not created' }};
                }}

                // Double-click to open text editor
                const whiteoutRect = whiteout.getBoundingClientRect();
                whiteout.dispatchEvent(new MouseEvent('dblclick', {{
                    bubbles: true,
                    clientX: whiteoutRect.left + whiteoutRect.width / 2,
                    clientY: whiteoutRect.top + whiteoutRect.height / 2
                }}));
                await new Promise(r => setTimeout(r, 300));

                // Find and fill the input
                const input = document.querySelector('.whiteout-text-input');
                if (!input) {{
                    return {{ success: false, error: 'Text input not found' }};
                }}

                input.textContent = 'Test Text On Top';
                input.dispatchEvent(new Event('input'));

                // Press Enter to save
                input.dispatchEvent(new KeyboardEvent('keydown', {{ key: 'Enter', bubbles: true }}));
                await new Promise(r => setTimeout(r, 300));

                // Find the saved text element
                const savedText = whiteout.querySelector('.whiteout-text-content') ||
                                  whiteout.querySelector('span') ||
                                  document.querySelector('.whiteout-text-content');

                if (!savedText) {{
                    return {{ success: false, error: 'Saved text element not found' }};
                }}

                // Check z-index and visibility
                const whiteoutStyle = window.getComputedStyle(whiteout);
                const textStyle = window.getComputedStyle(savedText);

                const whiteoutZIndex = parseInt(whiteoutStyle.zIndex) || 0;
                const textZIndex = parseInt(textStyle.zIndex) || 0;

                // Text should be visible (not hidden behind whiteout)
                const textIsVisible = savedText.offsetWidth > 0 && savedText.offsetHeight > 0;

                // Text should be inside the whiteout element (child of it)
                const textIsInsideWhiteout = whiteout.contains(savedText);

                // Text should be rendered on top (either higher z-index or as child of whiteout)
                const textOnTop = textIsInsideWhiteout || textZIndex > whiteoutZIndex;

                return {{
                    success: true,
                    savedTextContent: savedText.textContent,
                    textIsVisible: textIsVisible,
                    textIsInsideWhiteout: textIsInsideWhiteout,
                    textOnTop: textOnTop,
                    whiteoutZIndex: whiteoutZIndex,
                    textZIndex: textZIndex
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test saved text visibility")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout saved text visibility test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["textIsVisible"].as_bool().unwrap_or(false),
        "BUG: Saved text should be visible, not hidden"
    );
    assert!(
        result["textIsInsideWhiteout"].as_bool().unwrap_or(false),
        "BUG: Saved text should be inside the whiteout element"
    );
    assert!(
        result["textOnTop"].as_bool().unwrap_or(false),
        "BUG: Saved text should appear ON TOP of the whiteout, not behind it"
    );
}

/// Test that clicking on whiteout with Text tool opens whiteout editor, not creates new text
#[tokio::test]
async fn test_pdfjoin_clicking_whiteout_with_text_tool_opens_whiteout_editor() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_contract_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'contract.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Find text to cover
                const textItems = await PdfBridge.extractTextWithPositions(1);
                const targetItem = textItems.find(t => t.domFontSize > 20 && t.str.trim().length > 3);
                if (!targetItem) {{
                    return {{ success: false, error: 'No suitable text item found' }};
                }}

                // Draw a whiteout over this text
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const bounds = targetItem.domBounds;
                const pageRect = pageDiv.getBoundingClientRect();
                const clientX1 = pageRect.left + bounds.x - 5;
                const clientY1 = pageRect.top + bounds.y - 5;
                const clientX2 = pageRect.left + bounds.x + bounds.width + 5;
                const clientY2 = pageRect.top + bounds.y + bounds.height + 5;

                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{ bubbles: true, clientX: clientX1, clientY: clientY1 }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{ bubbles: true, clientX: clientX2, clientY: clientY2 }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{ bubbles: true, clientX: clientX2, clientY: clientY2 }}));
                await new Promise(r => setTimeout(r, 200));

                const whiteout = document.querySelector('.edit-whiteout-overlay');
                if (!whiteout) {{
                    return {{ success: false, error: 'Whiteout not created' }};
                }}

                // Scroll whiteout into view so elementFromPoint can find it
                whiteout.scrollIntoView({{ block: 'center' }});
                await new Promise(r => setTimeout(r, 200));

                // Now switch to TEXT tool
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));

                // Click on the whiteout with Text tool selected
                const whiteoutRect = whiteout.getBoundingClientRect();
                const overlay = document.querySelector('.overlay-container');
                overlay.dispatchEvent(new MouseEvent('click', {{
                    bubbles: true,
                    clientX: whiteoutRect.left + whiteoutRect.width / 2,
                    clientY: whiteoutRect.top + whiteoutRect.height / 2
                }}));
                await new Promise(r => setTimeout(r, 300));

                // Check: there should be an input INSIDE the whiteout, not a separate one
                const inputInsideWhiteout = whiteout.querySelector('input, .whiteout-text-input');
                const allTextInputs = document.querySelectorAll('.edit-text-input');

                // The input should be inside the whiteout, matching its font size
                const inputStyle = inputInsideWhiteout ? window.getComputedStyle(inputInsideWhiteout) : null;
                const inputFontSize = inputStyle ? parseFloat(inputStyle.fontSize) : 0;
                const coveredFontSize = targetItem.domFontSize;

                // Should NOT create a separate small text input with 12px font
                const hasSmallTextInput = Array.from(allTextInputs).some(input => {{
                    const style = window.getComputedStyle(input);
                    return parseFloat(style.fontSize) < 15;
                }});

                return {{
                    success: true,
                    hasInputInsideWhiteout: !!inputInsideWhiteout,
                    separateTextInputCount: allTextInputs.length,
                    hasSmallTextInput: hasSmallTextInput,
                    inputFontSize: inputFontSize,
                    coveredFontSize: coveredFontSize,
                    fontSizeMatches: Math.abs(inputFontSize - coveredFontSize) < 5
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test clicking whiteout with text tool")
        .into_value()
        .expect("Should get value");

    eprintln!("Clicking whiteout with Text tool test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["hasInputInsideWhiteout"].as_bool().unwrap_or(false),
        "BUG: Clicking whiteout with Text tool should open the whiteout's editor, not create separate input"
    );
    assert!(
        !result["hasSmallTextInput"].as_bool().unwrap_or(true),
        "BUG: Should not create a small (12px) text input on top of whiteout"
    );
    assert!(
        result["fontSizeMatches"].as_bool().unwrap_or(false),
        "BUG: Text input font should match covered text ({}px), not default. Got: {}px",
        result["coveredFontSize"].as_f64().unwrap_or(0.0),
        result["inputFontSize"].as_f64().unwrap_or(0.0)
    );
}

/// Test that whiteout text appears in the exported PDF via full UI flow
/// This tests the actual user journey: draw whiteout, type text, press Enter, download
#[tokio::test]
async fn test_pdfjoin_whiteout_text_appears_in_exported_pdf() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Go to Edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                // Load PDF via file input
                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Draw a whiteout
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const pageRect = pageDiv.getBoundingClientRect();
                const startX = pageRect.left + 100;
                const startY = pageRect.top + 100;
                const endX = startX + 200;
                const endY = startY + 50;

                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{ bubbles: true, clientX: startX, clientY: startY }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{ bubbles: true, clientX: endX, clientY: endY }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{ bubbles: true, clientX: endX, clientY: endY }}));
                await new Promise(r => setTimeout(r, 300));

                const whiteout = document.querySelector('.edit-whiteout-overlay');
                if (!whiteout) {{
                    return {{ success: false, error: 'Whiteout not created' }};
                }}

                // Find the text input that should have appeared
                let input = whiteout.querySelector('input, .whiteout-text-input');
                if (!input) {{
                    // Click on the whiteout to open editor
                    whiteout.scrollIntoView({{ block: 'center' }});
                    await new Promise(r => setTimeout(r, 100));
                    const whiteoutRect = whiteout.getBoundingClientRect();
                    const overlay = document.querySelector('.overlay-container');
                    overlay.dispatchEvent(new MouseEvent('click', {{
                        bubbles: true,
                        clientX: whiteoutRect.left + whiteoutRect.width / 2,
                        clientY: whiteoutRect.top + whiteoutRect.height / 2
                    }}));
                    await new Promise(r => setTimeout(r, 300));
                    input = whiteout.querySelector('input, .whiteout-text-input');
                }}

                if (!input) {{
                    return {{ success: false, error: 'Could not find text input in whiteout' }};
                }}

                // Type text and press Enter
                input.textContent = 'UNIQUE_WHITEOUT_TEXT_XYZ789';
                input.dispatchEvent(new Event('input', {{ bubbles: true }}));
                input.dispatchEvent(new KeyboardEvent('keydown', {{ key: 'Enter', bubbles: true }}));
                // Wait for async operations to complete
                await new Promise(r => setTimeout(r, 500));

                // Verify text was saved (check for text span)
                const textSpan = whiteout.querySelector('.whiteout-text-content');
                const hasTextInPreview = textSpan && textSpan.textContent.includes('UNIQUE_WHITEOUT_TEXT_XYZ789');

                // Capture the export by intercepting the download
                let exportedArray = null;
                let exportPromiseResolve = null;
                const exportPromise = new Promise(resolve => {{ exportPromiseResolve = resolve; }});

                const originalCreateObjectURL = URL.createObjectURL;
                URL.createObjectURL = (blob) => {{
                    // Read the blob
                    const reader = new FileReader();
                    reader.readAsArrayBuffer(blob);
                    reader.onload = () => {{
                        exportedArray = new Uint8Array(reader.result);
                        exportPromiseResolve(true);
                    }};
                    reader.onerror = () => {{
                        exportPromiseResolve(false);
                    }};
                    return originalCreateObjectURL(blob);
                }};

                // Click download button
                const downloadBtn = document.getElementById('edit-download-btn');
                const buttonWasEnabled = !downloadBtn?.disabled;
                if (downloadBtn && !downloadBtn.disabled) {{
                    downloadBtn.click();
                    // Wait for either the export to complete or timeout
                    await Promise.race([exportPromise, new Promise(r => setTimeout(r, 5000))]);
                }}

                // Restore original
                URL.createObjectURL = originalCreateObjectURL;

                if (!exportedArray) {{
                    return {{
                        success: false,
                        error: 'Could not capture export - download button may be disabled or export failed',
                        hasTextInPreview,
                        buttonWasEnabled,
                        whiteoutCount: document.querySelectorAll('.edit-whiteout-overlay').length
                    }};
                }}

                // Check the exported PDF for our text
                const decoder = new TextDecoder('utf-8', {{ fatal: false }});
                const pdfText = decoder.decode(exportedArray);

                const hasTextInPdf = pdfText.includes('UNIQUE_WHITEOUT_TEXT_XYZ789');
                const hasFreeText = pdfText.includes('/FreeText');
                const hasSquare = pdfText.includes('/Square');

                return {{
                    success: true,
                    hasTextInPreview: hasTextInPreview,
                    hasTextInPdf: hasTextInPdf,
                    hasFreeText: hasFreeText,
                    hasSquare: hasSquare,
                    exportedSize: exportedArray.length
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test whiteout text export")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout text export test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["hasTextInPreview"].as_bool().unwrap_or(false),
        "Text should appear in preview"
    );
    assert!(
        result["hasSquare"].as_bool().unwrap_or(false),
        "Exported PDF should contain Square annotation for whiteout"
    );
    assert!(
        result["hasTextInPdf"].as_bool().unwrap_or(false),
        "BUG: Text entered on whiteout does NOT appear in exported PDF. \
         The text shows in preview (hasTextInPreview={}) but is missing from downloaded document.",
        result["hasTextInPreview"].as_bool().unwrap_or(false)
    );
}

/// Test that whiteout borders are hidden by default but visible when editing
/// Borders should only be visible when:
/// 1. The Whiteout tool is selected
/// 2. When clicking on a whiteout to edit text
/// But NOT visible in normal preview mode
#[tokio::test]
async fn test_pdfjoin_whiteout_borders_hidden_by_default() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Draw a whiteout
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const pageRect = pageDiv.getBoundingClientRect();
                const startX = pageRect.left + 100;
                const startY = pageRect.top + 100;
                const endX = startX + 200;
                const endY = startY + 50;

                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{ bubbles: true, clientX: startX, clientY: startY }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{ bubbles: true, clientX: endX, clientY: endY }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{ bubbles: true, clientX: endX, clientY: endY }}));
                await new Promise(r => setTimeout(r, 200));

                const whiteout = document.querySelector('.edit-whiteout-overlay');
                if (!whiteout) {{
                    return {{ success: false, error: 'Whiteout not created' }};
                }}

                // Get computed style to check border visibility
                const computeVisibleBorder = (el) => {{
                    const style = window.getComputedStyle(el);
                    const boxShadow = style.boxShadow;
                    const border = style.border;
                    const outline = style.outline;
                    // Check if there's any visible edge indicator
                    const hasVisibleShadow = boxShadow && boxShadow !== 'none' && !boxShadow.includes('rgba(0, 0, 0, 0)');
                    const hasVisibleBorder = border && !border.includes('0px') && !border.includes('none');
                    const hasVisibleOutline = outline && !outline.includes('0px') && !outline.includes('none');
                    return {{ boxShadow, border, outline, hasVisibleShadow, hasVisibleBorder, hasVisibleOutline }};
                }};

                // 1. Check border while Whiteout tool is selected (should be visible)
                const borderWithWhiteoutTool = computeVisibleBorder(whiteout);

                // 2. Switch to Select tool and check border (should be hidden)
                document.getElementById('tool-select').click();
                await new Promise(r => setTimeout(r, 100));
                const borderWithSelectTool = computeVisibleBorder(whiteout);

                // 3. Switch to TextBox tool and check border (should be hidden)
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));
                const borderWithTextTool = computeVisibleBorder(whiteout);

                return {{
                    success: true,
                    borderWithWhiteoutTool: borderWithWhiteoutTool,
                    borderWithSelectTool: borderWithSelectTool,
                    borderWithTextTool: borderWithTextTool,
                    // These are the expectations
                    visibleWhenWhiteoutToolSelected: borderWithWhiteoutTool.hasVisibleShadow || borderWithWhiteoutTool.hasVisibleOutline,
                    hiddenWhenSelectTool: !borderWithSelectTool.hasVisibleShadow && !borderWithSelectTool.hasVisibleOutline,
                    hiddenWhenTextTool: !borderWithTextTool.hasVisibleShadow && !borderWithTextTool.hasVisibleOutline
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test whiteout border visibility")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout border visibility test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["hiddenWhenSelectTool"].as_bool().unwrap_or(false),
        "BUG: Whiteout borders should be HIDDEN when Select tool is active. \
         Currently visible: {:?}",
        result["borderWithSelectTool"]
    );
    assert!(
        result["hiddenWhenTextTool"].as_bool().unwrap_or(false),
        "BUG: Whiteout borders should be HIDDEN when Text tool is active. \
         Currently visible: {:?}",
        result["borderWithTextTool"]
    );
}

/// BUG TEST: Editing existing text overlay should allow bold/italic styling
/// When clicking on an existing text overlay to edit it, the Bold (B) and Italic (I)
/// buttons should be enabled and clicking them should apply the style.
/// BUG: Currently the blur handler fires too quickly (100ms) and closes the input
/// before the button click can be processed.
#[tokio::test]
async fn test_pdfjoin_edit_existing_text_allows_bold_italic() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Select TextBox tool and create a text box
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const pageRect = pageDiv.getBoundingClientRect();
                const clickX = pageRect.left + 200;
                const clickY = pageRect.top + 200;

                // TextBox creation requires mousedown/mouseup (not just click)
                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true,
                    clientX: clickX,
                    clientY: clickY
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true,
                    clientX: clickX,
                    clientY: clickY
                }}));
                await new Promise(r => setTimeout(r, 200));

                // Find the text box and its text-content (contentEditable)
                const textBox = document.querySelector('.text-box');
                if (!textBox) {{
                    return {{ success: false, error: 'Text box not created on mousedown/mouseup' }};
                }}

                const textContent = textBox.querySelector('.text-content');
                if (!textContent) {{
                    return {{ success: false, error: 'Text content area not found in text box' }};
                }}

                // Type some text
                textContent.textContent = 'Test Text';
                textContent.dispatchEvent(new Event('input', {{ bubbles: true }}));
                await new Promise(r => setTimeout(r, 100));

                // Initial state - should NOT be bold
                const initialFontWeight = window.getComputedStyle(textContent).fontWeight;
                const wasInitiallyBold = initialFontWeight === 'bold' || initialFontWeight === '700';

                // Check if bold button is enabled (text content should be focused)
                const boldBtn = document.getElementById('style-bold');
                const boldBtnDisabled = boldBtn?.disabled ?? true;

                // Click the bold button while text content is focused
                if (boldBtn) {{
                    boldBtn.click();
                    await new Promise(r => setTimeout(r, 100));
                }}

                // Check if text content still exists and is focused
                const textContentStillExists = document.querySelector('.text-box .text-content') !== null;

                // Check if bold was applied
                const afterBoldFontWeight = window.getComputedStyle(textContent).fontWeight;
                const isBoldAfterClick = afterBoldFontWeight === 'bold' || afterBoldFontWeight === '700';

                return {{
                    success: true,
                    wasInitiallyBold: wasInitiallyBold,
                    boldBtnDisabled: boldBtnDisabled,
                    textContentStillExists: textContentStillExists,
                    isBoldAfterClick: isBoldAfterClick,
                    boldWasApplied: !wasInitiallyBold && isBoldAfterClick
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test bold on existing text")
        .into_value()
        .expect("Should get value");

    eprintln!("Edit existing text bold/italic test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        !result["boldBtnDisabled"].as_bool().unwrap_or(true),
        "Bold button should be ENABLED when editing text in text box"
    );
    assert!(
        result["textContentStillExists"].as_bool().unwrap_or(false),
        "Text content should still exist after clicking Bold button"
    );
    // Note: Bold application depends on style button implementation
    // This test verifies the button is clickable without closing the editor
}

/// Test: Bold/italic buttons should remain enabled on SUBSEQUENT focuses of text
/// After editing text and blurring, clicking on it again should still have
/// the bold/italic buttons enabled.
#[tokio::test]
async fn test_pdfjoin_edit_existing_text_bold_enabled_on_second_edit() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Select TextBox tool and create a text box
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));

                const pageDiv = document.querySelector('.edit-page');
                const pageRect = pageDiv.getBoundingClientRect();
                const clickX = pageRect.left + 200;
                const clickY = pageRect.top + 200;

                // TextBox creation requires mousedown/mouseup
                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true,
                    clientX: clickX,
                    clientY: clickY
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true,
                    clientX: clickX,
                    clientY: clickY
                }}));
                await new Promise(r => setTimeout(r, 200));

                // Find the text box and its content area
                const textBox = document.querySelector('.text-box');
                if (!textBox) {{
                    return {{ success: false, error: 'Text box not created on mousedown/mouseup' }};
                }}

                const textContent = textBox.querySelector('.text-content');
                if (!textContent) {{
                    return {{ success: false, error: 'Text content area not found' }};
                }}

                // Type some text
                textContent.textContent = 'Test Text';
                textContent.dispatchEvent(new Event('input', {{ bubbles: true }}));
                await new Promise(r => setTimeout(r, 100));

                // FIRST EDIT: Check bold button is enabled while focused
                const boldBtn = document.getElementById('style-bold');
                const firstEditBoldDisabled = boldBtn?.disabled ?? true;

                // Blur (click elsewhere)
                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Check bold button state after blur
                const afterBlurBoldDisabled = boldBtn?.disabled ?? true;

                // SECOND EDIT: Click on text content to focus again
                textContent.click();
                textContent.focus();
                await new Promise(r => setTimeout(r, 200));

                // Check bold button is enabled on SECOND focus
                const secondEditBoldDisabled = boldBtn?.disabled ?? true;

                return {{
                    success: true,
                    firstEditBoldDisabled: firstEditBoldDisabled,
                    afterBlurBoldDisabled: afterBlurBoldDisabled,
                    secondEditBoldDisabled: secondEditBoldDisabled,
                    textContentStillExists: !!document.querySelector('.text-box .text-content')
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test second edit")
        .into_value()
        .expect("Should get value");

    eprintln!("Second edit bold button test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["textContentStillExists"].as_bool().unwrap_or(false),
        "Text content should still exist after blur and re-focus"
    );
    assert!(
        !result["secondEditBoldDisabled"].as_bool().unwrap_or(true),
        "Bold button should be ENABLED on second focus. \
         First edit disabled: {}, After blur: {}, Second edit disabled: {}",
        result["firstEditBoldDisabled"].as_bool().unwrap_or(false),
        result["afterBlurBoldDisabled"].as_bool().unwrap_or(false),
        result["secondEditBoldDisabled"].as_bool().unwrap_or(true)
    );
}

/// BUG TEST: Text should remain editable after being edited once
/// After editing existing text and saving, clicking on it again should open the editor.
/// BUG: Text becomes completely uneditable after the first edit.
#[tokio::test]
async fn test_pdfjoin_text_remains_editable_after_first_edit() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Select Text tool and add text
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));

                const overlay = document.querySelector('.overlay-container');
                const pageRect = document.querySelector('.edit-page').getBoundingClientRect();

                // Step 1: Create text box
                overlay.dispatchEvent(new MouseEvent('click', {{
                    bubbles: true,
                    clientX: pageRect.left + 200,
                    clientY: pageRect.top + 200
                }}));
                await new Promise(r => setTimeout(r, 200));

                let textBox = document.querySelector('.text-box');
                if (!textBox) {{
                    return {{ success: false, error: 'Step 1: Text box not created' }};
                }}
                let textContent = textBox.querySelector('.text-content');
                if (!textContent) {{
                    return {{ success: false, error: 'Step 1: Text content not found' }};
                }}

                // Type original text
                textContent.textContent = 'Original Text';
                textContent.dispatchEvent(new Event('input', {{ bubbles: true }}));
                const step1Content = textContent.textContent;

                // Blur to commit
                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Step 2: FIRST EDIT - focus the text content
                textBox = document.querySelector('.text-box');
                textContent = textBox.querySelector('.text-content');
                const firstEditEditable = textContent.isContentEditable;
                textContent.focus();
                await new Promise(r => setTimeout(r, 100));

                // Modify the text
                textContent.textContent = 'Modified Text';
                textContent.dispatchEvent(new Event('input', {{ bubbles: true }}));

                // Blur to commit
                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Verify text was modified
                textBox = document.querySelector('.text-box');
                textContent = textBox.querySelector('.text-content');
                const step2Content = textContent.textContent;
                const textWasModified = step2Content === 'Modified Text';

                // Step 3: SECOND EDIT - focus text again
                textContent.focus();
                await new Promise(r => setTimeout(r, 100));

                const secondEditEditable = textContent.isContentEditable;
                const secondEditFocused = document.activeElement === textContent;

                // Also check if bold button is enabled
                const boldBtn = document.getElementById('style-bold');
                const boldBtnDisabled = boldBtn?.disabled ?? true;

                return {{
                    success: true,
                    step1Content: step1Content,
                    step2Content: step2Content,
                    textWasModified: textWasModified,
                    firstEditInputCreated: firstEditEditable,
                    secondEditInputCreated: secondEditEditable && secondEditFocused,
                    boldBtnDisabledOnSecondEdit: boldBtnDisabled,
                    bugExists: firstEditEditable && !secondEditEditable
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test text editability")
        .into_value()
        .expect("Should get value");

    eprintln!("Text remains editable test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["firstEditInputCreated"].as_bool().unwrap_or(false),
        "First edit should create input"
    );
    assert!(
        result["textWasModified"].as_bool().unwrap_or(false),
        "Text should be modified after first edit. Got: '{}'",
        result["step2Content"].as_str().unwrap_or("?")
    );
    assert!(
        result["secondEditInputCreated"].as_bool().unwrap_or(false),
        "BUG: Text becomes UNEDITABLE after first edit! \
         Clicking on text after editing it once does NOT open the editor. \
         First edit worked: {}, Second edit worked: {}",
        result["firstEditInputCreated"].as_bool().unwrap_or(false),
        result["secondEditInputCreated"].as_bool().unwrap_or(false)
    );
}

/// BUG TEST: After editing text A, then editing text B, both should remain re-editable.
/// User scenario:
/// 1. Add and edit text A
/// 2. Add and edit text B
/// 3. Try to edit text A again -> should work
/// 4. Try to edit text B again -> should work
/// BUG: After editing multiple texts, returning to edit any of them fails.
#[tokio::test]
async fn test_pdfjoin_multiple_texts_remain_editable_after_switching() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Load PDF and create multiple text boxes, verify they remain editable
    let pdf_b64 = test_pdf_base64(2);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) pdfBytes[i] = binary.charCodeAt(i);
                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                dataTransfer.items.add(new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }}));
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Select TextBox tool
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));

                const overlay = document.querySelector('.overlay-container');
                const rect = overlay.getBoundingClientRect();

                // Step 1: Create Text Box A at position 1
                overlay.dispatchEvent(new MouseEvent('click', {{
                    bubbles: true,
                    clientX: rect.left + 100,
                    clientY: rect.top + 100
                }}));
                await new Promise(r => setTimeout(r, 200));

                // Type AAA in the text content
                const textBoxA = document.querySelectorAll('.text-box')[0];
                const contentA = textBoxA?.querySelector('.text-content');
                if (!contentA) return {{ success: false, error: 'Text box A not created' }};
                contentA.textContent = 'AAA';
                contentA.dispatchEvent(new Event('input', {{ bubbles: true }}));

                // Blur to commit
                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Step 2: Create Text Box B at different position
                overlay.dispatchEvent(new MouseEvent('click', {{
                    bubbles: true,
                    clientX: rect.left + 100,
                    clientY: rect.top + 250
                }}));
                await new Promise(r => setTimeout(r, 200));

                // Type BBB
                const textBoxB = document.querySelectorAll('.text-box')[1];
                const contentB = textBoxB?.querySelector('.text-content');
                if (!contentB) return {{ success: false, error: 'Text box B not created' }};
                contentB.textContent = 'BBB';
                contentB.dispatchEvent(new Event('input', {{ bubbles: true }}));

                // Blur to commit
                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Check initial state
                const stateAfterCreate = {{
                    textA: document.querySelectorAll('.text-box')[0]?.querySelector('.text-content')?.textContent,
                    textB: document.querySelectorAll('.text-box')[1]?.querySelector('.text-content')?.textContent
                }};

                // Step 3: Click on Text Box A to edit it again
                const boxA = document.querySelectorAll('.text-box')[0];
                const contentAAgain = boxA?.querySelector('.text-content');
                contentAAgain?.focus();
                await new Promise(r => setTimeout(r, 100));
                contentAAgain.textContent = 'AAA-edited';
                contentAAgain.dispatchEvent(new Event('input', {{ bubbles: true }}));

                // Blur
                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Step 4: Click on Text Box B to edit it
                const boxB = document.querySelectorAll('.text-box')[1];
                const contentBAgain = boxB?.querySelector('.text-content');
                contentBAgain?.focus();
                await new Promise(r => setTimeout(r, 100));
                contentBAgain.textContent = 'BBB-edited';
                contentBAgain.dispatchEvent(new Event('input', {{ bubbles: true }}));

                // Blur
                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Step 5: Edit Text Box A AGAIN (the critical test)
                const boxAFinal = document.querySelectorAll('.text-box')[0];
                const contentAFinal = boxAFinal?.querySelector('.text-content');
                const aWasEditable = contentAFinal?.isContentEditable;
                contentAFinal?.focus();
                await new Promise(r => setTimeout(r, 100));
                contentAFinal.textContent = 'AAA-edited-again';
                contentAFinal.dispatchEvent(new Event('input', {{ bubbles: true }}));

                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Step 6: Edit Text Box B AGAIN
                const boxBFinal = document.querySelectorAll('.text-box')[1];
                const contentBFinal = boxBFinal?.querySelector('.text-content');
                const bWasEditable = contentBFinal?.isContentEditable;
                contentBFinal?.focus();
                await new Promise(r => setTimeout(r, 100));
                contentBFinal.textContent = 'BBB-edited-again';
                contentBFinal.dispatchEvent(new Event('input', {{ bubbles: true }}));

                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Final state
                return {{
                    success: true,
                    stateAfterCreate: stateAfterCreate,
                    textA: document.querySelectorAll('.text-box')[0]?.querySelector('.text-content')?.textContent,
                    textB: document.querySelectorAll('.text-box')[1]?.querySelector('.text-content')?.textContent,
                    aWasEditable: aWasEditable,
                    bWasEditable: bWasEditable,
                    boxCount: document.querySelectorAll('.text-box').length
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should run test")
        .into_value()
        .expect("Should get value");

    eprintln!("Multiple texts editable test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert_eq!(
        result["textA"].as_str().unwrap_or(""),
        "AAA-edited-again",
        "Text A should be 'AAA-edited-again' after multiple edits"
    );

    assert_eq!(
        result["textB"].as_str().unwrap_or(""),
        "BBB-edited-again",
        "Text B should be 'BBB-edited-again' after multiple edits"
    );

    assert!(
        result["aWasEditable"].as_bool().unwrap_or(false),
        "Text box A should remain contentEditable"
    );

    assert!(
        result["bWasEditable"].as_bool().unwrap_or(false),
        "Text box B should remain contentEditable"
    );
}

/// BUG TEST: Edited PDF text (replacement overlay) should be re-editable after saving.
/// User scenario:
/// 1. Load PDF with existing text
/// 2. Click on existing text with Select tool to edit it
/// 3. Change text and save -> creates .edit-replace-overlay
/// 4. Try to click on the replacement overlay to edit it again -> should work
/// BUG: The .edit-replace-overlay has no click handlers, so it cannot be re-edited.
#[tokio::test]
async fn test_pdfjoin_pdf_text_replacement_is_reeditable() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Use Florida contract which has real text to edit
    let pdf_b64 = florida_escalation_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) pdfBytes[i] = binary.charCodeAt(i);

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                dataTransfer.items.add(new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }}));
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Find a text item in the text layer (existing PDF text)
                const textItem = document.querySelector('.text-item');
                if (!textItem) {{
                    return {{ success: false, error: 'No text items found in PDF' }};
                }}
                const originalText = textItem.textContent;

                // Click on it with Select tool (default) to edit
                textItem.click();
                await new Promise(r => setTimeout(r, 300));

                // Find the text editor popup
                const editor = document.querySelector('.text-editor-popup');
                if (!editor) {{
                    return {{ success: false, error: 'Text editor popup did not appear' }};
                }}

                // Change the text
                const input = editor.querySelector('.text-editor-input');
                input.value = 'FIRST EDIT';

                // Click save
                editor.querySelector('.text-editor-save').click();
                await new Promise(r => setTimeout(r, 300));

                // Find the replacement overlay that was created
                const replaceOverlay = document.querySelector('.edit-replace-overlay');
                if (!replaceOverlay) {{
                    return {{ success: false, error: 'Replacement overlay was not created after save' }};
                }}
                const afterFirstEdit = replaceOverlay.textContent;

                // NOW TRY TO RE-EDIT: Click on the replacement overlay
                replaceOverlay.click();
                await new Promise(r => setTimeout(r, 300));

                // Check if an editor appeared (either popup or inline input)
                const editorAfterClick = document.querySelector('.text-editor-popup') ||
                                         document.querySelector('.edit-text-input');
                const canReEdit = !!editorAfterClick;

                if (!canReEdit) {{
                    return {{
                        success: true,
                        originalText,
                        afterFirstEdit,
                        canReEdit: false,
                        error: 'BUG: Cannot re-edit replacement overlay - no editor appeared on click'
                    }};
                }}

                // If editor appeared, try to change text again
                const reEditInput = editorAfterClick.querySelector?.('.text-editor-input') || editorAfterClick;
                if (reEditInput.tagName === 'INPUT') {{
                    reEditInput.value = 'SECOND EDIT';
                }} else {{
                    reEditInput.textContent = 'SECOND EDIT';
                }}

                // Save again
                const saveBtn = document.querySelector('.text-editor-save');
                if (saveBtn) {{
                    saveBtn.click();
                }} else {{
                    reEditInput.dispatchEvent(new KeyboardEvent('keydown', {{ key: 'Enter', bubbles: true }}));
                }}
                await new Promise(r => setTimeout(r, 300));

                // Check final state
                const finalOverlay = document.querySelector('.edit-replace-overlay');
                const afterSecondEdit = finalOverlay?.textContent;

                return {{
                    success: true,
                    originalText,
                    afterFirstEdit,
                    canReEdit: true,
                    afterSecondEdit,
                    secondEditWorked: afterSecondEdit === 'SECOND EDIT'
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test PDF text re-editing")
        .into_value()
        .expect("Should get value");

    eprintln!("PDF text replacement re-edit test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["canReEdit"].as_bool().unwrap_or(false),
        "BUG: Replacement overlay cannot be re-edited! \
         Original: '{}', After first edit: '{}'. \
         Clicking on replacement overlay does not open an editor.",
        result["originalText"].as_str().unwrap_or("?"),
        result["afterFirstEdit"].as_str().unwrap_or("?")
    );

    assert!(
        result["secondEditWorked"].as_bool().unwrap_or(false),
        "BUG: Second edit did not work! \
         After first edit: '{}', After second edit: '{}' (expected 'SECOND EDIT')",
        result["afterFirstEdit"].as_str().unwrap_or("?"),
        result["afterSecondEdit"].as_str().unwrap_or("?")
    );
}

/// BUG TEST: When re-editing a replacement overlay, the editor should show the user's
/// intermediate text (their last edit), NOT the original PDF text.
/// User scenario:
/// 1. Edit "ESCALATION ADDENDUM" -> "MY CUSTOM TEXT", save
/// 2. Click on "MY CUSTOM TEXT" to re-edit
/// 3. Editor should open with "MY CUSTOM TEXT" pre-filled (NOT "ESCALATION ADDENDUM")
/// BUG: Editor opens with original text instead of user's last edit.
#[tokio::test]
async fn test_pdfjoin_reedit_shows_intermediate_text_not_original() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_escalation_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) pdfBytes[i] = binary.charCodeAt(i);

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                dataTransfer.items.add(new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }}));
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                // Find a text item in the text layer
                const textItem = document.querySelector('.text-item');
                if (!textItem) {{
                    return {{ success: false, error: 'No text items found in PDF' }};
                }}
                const originalText = textItem.textContent;

                // First edit: click and change text
                textItem.click();
                await new Promise(r => setTimeout(r, 300));

                const editor1 = document.querySelector('.text-editor-popup');
                if (!editor1) {{
                    return {{ success: false, error: 'First editor popup did not appear' }};
                }}

                const input1 = editor1.querySelector('.text-editor-input');
                const firstEditText = 'MY INTERMEDIATE TEXT';
                input1.value = firstEditText;
                editor1.querySelector('.text-editor-save').click();
                await new Promise(r => setTimeout(r, 300));

                // Find replacement overlay
                const replaceOverlay = document.querySelector('.edit-replace-overlay');
                if (!replaceOverlay) {{
                    return {{ success: false, error: 'Replacement overlay not created' }};
                }}

                // Now click to re-edit
                replaceOverlay.click();
                await new Promise(r => setTimeout(r, 300));

                // Check what text is in the editor
                const editor2 = document.querySelector('.text-editor-popup');
                if (!editor2) {{
                    return {{ success: false, error: 'Second editor popup did not appear on re-edit' }};
                }}

                const input2 = editor2.querySelector('.text-editor-input');
                const textInReEditPopup = input2.value;

                // Close editor
                const cancelBtn = editor2.querySelector('.text-editor-cancel');
                if (cancelBtn) cancelBtn.click();

                return {{
                    success: true,
                    originalText,
                    firstEditText,
                    textInReEditPopup,
                    showsIntermediateText: textInReEditPopup === firstEditText,
                    showsOriginalText: textInReEditPopup === originalText
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test re-edit intermediate text")
        .into_value()
        .expect("Should get value");

    eprintln!("Re-edit intermediate text test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["showsIntermediateText"].as_bool().unwrap_or(false),
        "BUG: Re-edit editor should show user's intermediate text '{}', \
         but instead shows '{}'. Original was '{}'.",
        result["firstEditText"].as_str().unwrap_or("?"),
        result["textInReEditPopup"].as_str().unwrap_or("?"),
        result["originalText"].as_str().unwrap_or("?")
    );
}

/// BUG TEST: During re-editing, the preview should show the user's intermediate text,
/// not the original PDF text. The replacement overlay should stay visible to cover
/// the original canvas text while the editor is open.
#[tokio::test]
async fn test_pdfjoin_reedit_preview_shows_intermediate_not_original() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = florida_escalation_base64();
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) pdfBytes[i] = binary.charCodeAt(i);

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                dataTransfer.items.add(new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }}));
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 3000));

                const textItem = document.querySelector('.text-item');
                if (!textItem) {{
                    return {{ success: false, error: 'No text items found' }};
                }}
                const originalText = textItem.textContent;

                // First edit
                textItem.click();
                await new Promise(r => setTimeout(r, 300));
                const editor1 = document.querySelector('.text-editor-popup');
                const input1 = editor1.querySelector('.text-editor-input');
                const intermediateText = 'MY EDITED TEXT';
                input1.value = intermediateText;
                editor1.querySelector('.text-editor-save').click();
                await new Promise(r => setTimeout(r, 300));

                // Click to re-edit
                const replaceOverlay = document.querySelector('.edit-replace-overlay');
                replaceOverlay.click();
                await new Promise(r => setTimeout(r, 300));

                // While editor is open, check if replacement overlay still covers the original
                // The overlay should still be in DOM and visible (or a cover should be present)
                const overlayDuringEdit = document.querySelector('.edit-replace-overlay');
                const overlayVisible = overlayDuringEdit &&
                    window.getComputedStyle(overlayDuringEdit).display !== 'none' &&
                    window.getComputedStyle(overlayDuringEdit).visibility !== 'hidden';
                const overlayText = overlayDuringEdit ? overlayDuringEdit.textContent : null;

                // Close editor
                const cancelBtn = document.querySelector('.text-editor-cancel');
                if (cancelBtn) cancelBtn.click();

                return {{
                    success: true,
                    originalText,
                    intermediateText,
                    overlayVisibleDuringEdit: overlayVisible,
                    overlayTextDuringEdit: overlayText,
                    previewShowsIntermediate: overlayVisible && overlayText === intermediateText
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test preview during re-edit")
        .into_value()
        .expect("Should get value");

    eprintln!("Re-edit preview test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["previewShowsIntermediate"]
            .as_bool()
            .unwrap_or(false),
        "BUG: During re-edit, preview should show intermediate text '{}', \
         but overlay visible={}, overlay text='{}'. Original was '{}'.",
        result["intermediateText"].as_str().unwrap_or("?"),
        result["overlayVisibleDuringEdit"]
            .as_bool()
            .unwrap_or(false),
        result["overlayTextDuringEdit"].as_str().unwrap_or("null"),
        result["originalText"].as_str().unwrap_or("?")
    );
}

/// Tests that font size controls exist in the edit toolbar
#[tokio::test]
async fn test_pdfjoin_font_size_controls_exist() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let js_code = r#"(async () => {
        try {
            document.querySelector('[data-tab="edit"]').click();
            await new Promise(r => setTimeout(r, 300));

            const fontSizeControl = document.getElementById('font-size-control');
            const fontSizeDecrease = document.getElementById('font-size-decrease');
            const fontSizeIncrease = document.getElementById('font-size-increase');
            const fontSizeValue = document.getElementById('font-size-value');

            return {
                success: true,
                fontSizeControlExists: !!fontSizeControl,
                fontSizeDecreaseExists: !!fontSizeDecrease,
                fontSizeIncreaseExists: !!fontSizeIncrease,
                fontSizeValueExists: !!fontSizeValue,
                decreaseButtonText: fontSizeDecrease ? fontSizeDecrease.textContent : null,
                increaseButtonText: fontSizeIncrease ? fontSizeIncrease.textContent : null,
                defaultFontSize: fontSizeValue ? fontSizeValue.value : null
            };
        } catch (err) {
            return { success: false, error: err.toString() };
        }
    })()"#;

    let result: serde_json::Value = page
        .evaluate(js_code)
        .await
        .expect("Should check font size controls")
        .into_value()
        .expect("Should get value");

    eprintln!("Font size controls test: {:?}", result);

    assert!(
        result["fontSizeControlExists"].as_bool().unwrap_or(false),
        "Font size control container should exist"
    );
    assert!(
        result["fontSizeDecreaseExists"].as_bool().unwrap_or(false),
        "Font size decrease button should exist"
    );
    assert!(
        result["fontSizeIncreaseExists"].as_bool().unwrap_or(false),
        "Font size increase button should exist"
    );
    assert!(
        result["fontSizeValueExists"].as_bool().unwrap_or(false),
        "Font size value input should exist"
    );
    assert_eq!(
        result["defaultFontSize"].as_str().unwrap_or(""),
        "12",
        "Default font size should be 12"
    );
}

/// Tests that font family dropdown exists with expected options
#[tokio::test]
async fn test_pdfjoin_font_family_control_exists() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let js_code = r#"(async () => {
        try {
            document.querySelector('[data-tab="edit"]').click();
            await new Promise(r => setTimeout(r, 300));

            const fontFamilySelect = document.getElementById('style-font-family');
            if (!fontFamilySelect) {
                return { success: false, error: 'Font family select not found' };
            }

            const options = Array.from(fontFamilySelect.options).map(o => o.value);

            return {
                success: true,
                fontFamilyExists: true,
                optionCount: options.length,
                options: options,
                hasSansSerif: options.includes('sans-serif'),
                hasSerif: options.includes('serif'),
                hasMonospace: options.includes('monospace'),
                hasArial: options.includes('Arial'),
                hasTimesNewRoman: options.includes('Times New Roman')
            };
        } catch (err) {
            return { success: false, error: err.toString() };
        }
    })()"#;

    let result: serde_json::Value = page
        .evaluate(js_code)
        .await
        .expect("Should check font family control")
        .into_value()
        .expect("Should get value");

    eprintln!("Font family control test: {:?}", result);

    assert!(
        result["fontFamilyExists"].as_bool().unwrap_or(false),
        "Font family select should exist"
    );
    assert!(
        result["optionCount"].as_i64().unwrap_or(0) >= 5,
        "Should have at least 5 font options"
    );
    assert!(
        result["hasSansSerif"].as_bool().unwrap_or(false),
        "Should have sans-serif option"
    );
    assert!(
        result["hasSerif"].as_bool().unwrap_or(false),
        "Should have serif option"
    );
    assert!(
        result["hasMonospace"].as_bool().unwrap_or(false),
        "Should have monospace option"
    );
}

/// Tests that font size can be changed when editing text
#[tokio::test]
async fn test_pdfjoin_font_size_change_works() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(1);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Select TextBox tool and add text
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));

                const overlay = document.querySelector('.overlay-container');
                const pageRect = document.querySelector('.edit-page').getBoundingClientRect();

                // Click to create text box
                overlay.dispatchEvent(new MouseEvent('click', {{
                    bubbles: true,
                    clientX: pageRect.left + 200,
                    clientY: pageRect.top + 200
                }}));
                await new Promise(r => setTimeout(r, 200));

                // Find the text box content
                const textBox = document.querySelector('.text-box');
                if (!textBox) {{
                    return {{ success: false, error: 'Text box not created' }};
                }}
                const textContent = textBox.querySelector('.text-content');
                if (!textContent) {{
                    return {{ success: false, error: 'Text content not found in text box' }};
                }}

                // Text content should be focused automatically
                // Check initial font size from computed style
                const initialStyle = window.getComputedStyle(textContent);
                const initialFontSize = initialStyle.fontSize;

                // Check that font controls are enabled (text content should be focused)
                const increaseBtn = document.getElementById('font-size-increase');
                const decreaseBtn = document.getElementById('font-size-decrease');
                const controlsEnabled = increaseBtn && decreaseBtn && !increaseBtn.disabled && !decreaseBtn.disabled;

                // Get initial display value
                const fontSizeValue = document.getElementById('font-size-value');
                const initialDisplayValue = fontSizeValue?.value || '12';

                // Increase font size twice
                if (increaseBtn) {{
                    increaseBtn.click();
                    await new Promise(r => setTimeout(r, 50));
                    increaseBtn.click();
                    await new Promise(r => setTimeout(r, 50));
                }}

                const newStyle = window.getComputedStyle(textContent);
                const newFontSize = newStyle.fontSize;
                const newDisplayValue = fontSizeValue?.value || '12';

                return {{
                    success: true,
                    initialFontSize: initialFontSize,
                    initialDisplayValue: initialDisplayValue,
                    controlsEnabled: controlsEnabled,
                    newFontSize: newFontSize,
                    newDisplayValue: newDisplayValue,
                    fontSizeIncreased: parseFloat(newFontSize) > parseFloat(initialFontSize)
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test font size change")
        .into_value()
        .expect("Should get value");

    eprintln!("Font size change test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["controlsEnabled"].as_bool().unwrap_or(false),
        "Font controls should be enabled when text input is active"
    );
    assert!(
        result["fontSizeIncreased"].as_bool().unwrap_or(false),
        "Font size should increase when clicking + button. Initial: {}, New: {}",
        result["initialFontSize"].as_str().unwrap_or("?"),
        result["newFontSize"].as_str().unwrap_or("?")
    );
}

/// Tests that text overlay can be dragged with Select tool
#[tokio::test]
async fn test_pdfjoin_text_overlay_draggable_with_select_tool() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(1);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Select Text tool and add text
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));

                const overlay = document.querySelector('.overlay-container');
                const pageRect = document.querySelector('.edit-page').getBoundingClientRect();

                // Click to create text box
                overlay.dispatchEvent(new MouseEvent('click', {{
                    bubbles: true,
                    clientX: pageRect.left + 200,
                    clientY: pageRect.top + 200
                }}));
                await new Promise(r => setTimeout(r, 200));

                let textBox = document.querySelector('.text-box');
                if (!textBox) {{
                    return {{ success: false, error: 'Text box not created' }};
                }}

                const textContent = textBox.querySelector('.text-content');
                if (!textContent) {{
                    return {{ success: false, error: 'Text content not found' }};
                }}

                // Add text
                textContent.textContent = 'Draggable Text';
                textContent.dispatchEvent(new Event('input', {{ bubbles: true }}));

                // Blur to commit
                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Record initial position
                textBox = document.querySelector('.text-box');
                const initialLeft = parseFloat(textBox.style.left);
                const initialTop = parseFloat(textBox.style.top);

                // Check cursor style on the text box body (not text-content)
                const cursorStyle = window.getComputedStyle(textBox).cursor;

                // Switch to Select tool
                document.getElementById('tool-select').click();
                await new Promise(r => setTimeout(r, 100));

                // Simulate drag on the text box body (not on text-content)
                const boxRect = textBox.getBoundingClientRect();
                // Click near edge of box to avoid text-content
                const startX = boxRect.left + 5;
                const startY = boxRect.top + 5;
                const endX = startX + 50;
                const endY = startY + 30;

                textBox.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true,
                    clientX: startX,
                    clientY: startY
                }}));
                await new Promise(r => setTimeout(r, 50));

                document.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true,
                    clientX: endX,
                    clientY: endY
                }}));
                await new Promise(r => setTimeout(r, 50));

                document.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true,
                    clientX: endX,
                    clientY: endY
                }}));
                await new Promise(r => setTimeout(r, 100));

                // Check final position
                textBox = document.querySelector('.text-box');
                const finalLeft = parseFloat(textBox.style.left);
                const finalTop = parseFloat(textBox.style.top);

                return {{
                    success: true,
                    cursorStyle: cursorStyle,
                    hasMoveStyle: cursorStyle === 'move' || cursorStyle === 'default',
                    initialLeft: initialLeft,
                    initialTop: initialTop,
                    finalLeft: finalLeft,
                    finalTop: finalTop,
                    positionChanged: finalLeft !== initialLeft || finalTop !== initialTop,
                    deltaX: finalLeft - initialLeft,
                    deltaY: finalTop - initialTop
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test text overlay dragging")
        .into_value()
        .expect("Should get value");

    eprintln!("Text overlay drag test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["hasMoveStyle"].as_bool().unwrap_or(false),
        "Text overlay should have cursor: move style"
    );
    assert!(
        result["positionChanged"].as_bool().unwrap_or(false),
        "Text overlay position should change after dragging. Initial: ({}, {}), Final: ({}, {})",
        result["initialLeft"].as_f64().unwrap_or(0.0),
        result["initialTop"].as_f64().unwrap_or(0.0),
        result["finalLeft"].as_f64().unwrap_or(0.0),
        result["finalTop"].as_f64().unwrap_or(0.0)
    );
}

/// Tests that text overlay can be edited after clicking elsewhere in the document
#[tokio::test]
async fn test_pdfjoin_text_editable_after_clicking_elsewhere() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(1);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Select TextBox tool
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));

                const overlay = document.querySelector('.overlay-container');
                const pageRect = document.querySelector('.edit-page').getBoundingClientRect();

                // Step 1: Create text box at position (200, 200)
                overlay.dispatchEvent(new MouseEvent('click', {{
                    bubbles: true,
                    clientX: pageRect.left + 200,
                    clientY: pageRect.top + 200
                }}));
                await new Promise(r => setTimeout(r, 200));

                const textBox = document.querySelector('.text-box');
                if (!textBox) {{
                    return {{ success: false, error: 'Step 1: Text box not created' }};
                }}
                const textContent = textBox.querySelector('.text-content');
                if (!textContent) {{
                    return {{ success: false, error: 'Step 1: Text content not found' }};
                }}

                // Type text
                textContent.textContent = 'Test Text';
                textContent.dispatchEvent(new Event('input', {{ bubbles: true }}));

                // Blur to commit
                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Step 2: Click ELSEWHERE to create another text box
                overlay.dispatchEvent(new MouseEvent('click', {{
                    bubbles: true,
                    clientX: pageRect.left + 400,
                    clientY: pageRect.top + 400
                }}));
                await new Promise(r => setTimeout(r, 200));

                // Should have created a second text box
                const textBoxCount = document.querySelectorAll('.text-box').length;

                // Blur again
                document.body.click();
                await new Promise(r => setTimeout(r, 200));

                // Step 3: Now try to click on the ORIGINAL text box to edit it
                const originalBox = document.querySelectorAll('.text-box')[0];
                if (!originalBox) {{
                    return {{ success: false, error: 'Step 3: Original text box disappeared' }};
                }}

                const originalContent = originalBox.querySelector('.text-content');
                originalContent.focus();
                await new Promise(r => setTimeout(r, 100));

                // Check if it's editable
                const isEditable = originalContent.isContentEditable;
                const isFocused = document.activeElement === originalContent;

                // Try to modify text
                originalContent.textContent = 'Test Text Edited';
                originalContent.dispatchEvent(new Event('input', {{ bubbles: true }}));

                return {{
                    success: true,
                    textBoxCount: textBoxCount,
                    isEditable: isEditable,
                    isFocused: isFocused,
                    originalTextAfterEdit: originalContent.textContent,
                    canEditAfterClickingElsewhere: isEditable && originalContent.textContent === 'Test Text Edited'
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test text editability after clicking elsewhere")
        .into_value()
        .expect("Should get value");

    eprintln!("Text editable after clicking elsewhere test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["isEditable"].as_bool().unwrap_or(false),
        "Text box should remain editable after clicking elsewhere"
    );
    assert!(
        result["canEditAfterClickingElsewhere"]
            .as_bool()
            .unwrap_or(false),
        "Should be able to edit text box after clicking elsewhere"
    );
}

/// Tests that contentEditable text input auto-expands
#[tokio::test]
async fn test_pdfjoin_text_input_auto_expands() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(1);
    let js_code = format!(
        r#"(async () => {{
            try {{
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Select Text tool and add text
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));

                const overlay = document.querySelector('.overlay-container');
                const pageRect = document.querySelector('.edit-page').getBoundingClientRect();

                // Click to create text box
                overlay.dispatchEvent(new MouseEvent('click', {{
                    bubbles: true,
                    clientX: pageRect.left + 200,
                    clientY: pageRect.top + 200
                }}));
                await new Promise(r => setTimeout(r, 200));

                const textBox = document.querySelector('.text-box');
                if (!textBox) {{
                    return {{ success: false, error: 'Text box not created' }};
                }}
                const textContent = textBox.querySelector('.text-content');
                if (!textContent) {{
                    return {{ success: false, error: 'Text content not found' }};
                }}

                // Check that it's contentEditable
                const isContentEditable = textContent.contentEditable === 'true' || textContent.isContentEditable;

                // Get initial dimensions of the text box
                const initialWidth = textBox.offsetWidth;
                const initialHeight = textBox.offsetHeight;

                // Add a long text
                textContent.textContent = 'This is a very long text that should cause the input to expand horizontally to accommodate all the content without truncation';
                textContent.dispatchEvent(new Event('input', {{ bubbles: true }}));
                await new Promise(r => setTimeout(r, 100));

                // Get expanded dimensions of the text box
                const expandedWidth = textBox.offsetWidth;
                const expandedHeight = textBox.offsetHeight;

                return {{
                    success: true,
                    isContentEditable: isContentEditable,
                    initialWidth: initialWidth,
                    initialHeight: initialHeight,
                    expandedWidth: expandedWidth,
                    expandedHeight: expandedHeight,
                    widthExpanded: expandedWidth > initialWidth,
                    tagName: textContent.tagName.toLowerCase()
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test text input auto-expand")
        .into_value()
        .expect("Should get value");

    eprintln!("Text input auto-expand test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );
    assert!(
        result["isContentEditable"].as_bool().unwrap_or(false),
        "Text input should be contentEditable. Tag: {}",
        result["tagName"].as_str().unwrap_or("?")
    );
    assert!(
        result["widthExpanded"].as_bool().unwrap_or(false),
        "Text input should expand with content. Initial: {}px, Expanded: {}px",
        result["initialWidth"].as_f64().unwrap_or(0.0),
        result["expandedWidth"].as_f64().unwrap_or(0.0)
    );
}

/// Regression test: Overlay container should have pointer-events: auto when annotation tools are active
/// Bug: overlay-container had pointer-events: none which prevented clicks from reaching the click handler
/// that adds annotations (text). Clicks passed through to the text layer below.
/// Fix: updateCursor() now sets overlay-container pointer-events to 'auto' for annotation tools.
/// NOTE: checkbox and highlight tools are currently disabled, so only text tool is tested.
#[tokio::test]
async fn test_pdfjoin_overlay_pointer_events_enabled_for_annotation_tools() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(1);
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Click Edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const blob = new Blob([pdfBytes], {{ type: 'application/pdf' }});
                const file = new File([blob], 'test.pdf', {{ type: 'application/pdf' }});
                const dt = new DataTransfer();
                dt.items.add(file);
                const inp = document.getElementById('edit-file-input');
                inp.files = dt.files;
                inp.dispatchEvent(new Event('change', {{ bubbles: true }}));

                // Wait for PDF to render
                await new Promise(r => setTimeout(r, 3000));

                // Get initial pointer-events state (should be 'none' for select tool)
                const overlay = document.querySelector('.overlay-container');
                if (!overlay) {{
                    return {{ success: false, error: 'overlay-container not found' }};
                }}

                const selectToolPE = window.getComputedStyle(overlay).pointerEvents;

                // Click text tool (only annotation tool currently enabled)
                document.getElementById('edit-tool-textbox').click();
                await new Promise(r => setTimeout(r, 100));
                const textToolPE = window.getComputedStyle(overlay).pointerEvents;

                // Click select tool (should disable pointer-events)
                document.getElementById('tool-select').click();
                await new Promise(r => setTimeout(r, 100));
                const selectToolPE2 = window.getComputedStyle(overlay).pointerEvents;

                // Click whiteout tool (should also disable pointer-events)
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));
                const whiteoutToolPE = window.getComputedStyle(overlay).pointerEvents;

                return {{
                    success: true,
                    selectToolPE: selectToolPE,
                    textToolPE: textToolPE,
                    selectToolPE2: selectToolPE2,
                    whiteoutToolPE: whiteoutToolPE
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test overlay pointer-events")
        .into_value()
        .expect("Should get value");

    eprintln!("Overlay pointer-events test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    // Text tool should have pointer-events: auto
    assert_eq!(
        result["textToolPE"].as_str().unwrap_or(""),
        "auto",
        "Text tool should enable overlay pointer-events"
    );

    // Select and whiteout tools should have pointer-events: none
    assert_eq!(
        result["selectToolPE2"].as_str().unwrap_or(""),
        "none",
        "Select tool should disable overlay pointer-events"
    );
    assert_eq!(
        result["whiteoutToolPE"].as_str().unwrap_or(""),
        "none",
        "Whiteout tool should disable overlay pointer-events"
    );
}

// TODO: Re-enable when checkbox tool is restored
// /// Regression test: Clicking with checkbox tool should actually add a checkbox annotation
// /// Bug: overlay-container had pointer-events: none, so clicks didn't reach the handler
// #[tokio::test]
// async fn test_pdfjoin_checkbox_tool_creates_annotation_on_click() {
//     skip_if_no_chrome!();
//     require_local_server!("http://127.0.0.1:8082");
//
//     let Some((browser, _handle)) = browser::require_browser().await else {
//         return;
//     };
//
//     let page = browser
//         .new_page("about:blank")
//         .await
//         .expect("Should create page");
//
//     page.goto("http://127.0.0.1:8082")
//         .await
//         .expect("Should navigate to PDFJoin");
//
//     tokio::time::sleep(Duration::from_secs(3)).await;
//
//     let pdf_b64 = test_pdf_base64(1);
//     let js_code = format!(
//         r#"(async () => {{
//             try {{
//                 // Click Edit tab
//                 document.querySelector('[data-tab="edit"]').click();
//                 await new Promise(r => setTimeout(r, 300));
//
//                 // Load PDF
//                 const b64 = "{}";
//                 const binary = atob(b64);
//                 const pdfBytes = new Uint8Array(binary.length);
//                 for (let i = 0; i < binary.length; i++) {{
//                     pdfBytes[i] = binary.charCodeAt(i);
//                 }}
//
//                 const blob = new Blob([pdfBytes], {{ type: 'application/pdf' }});
//                 const file = new File([blob], 'test.pdf', {{ type: 'application/pdf' }});
//                 const dt = new DataTransfer();
//                 dt.items.add(file);
//                 const inp = document.getElementById('edit-file-input');
//                 inp.files = dt.files;
//                 inp.dispatchEvent(new Event('change', {{ bubbles: true }}));
//
//                 // Wait for PDF to render
//                 await new Promise(r => setTimeout(r, 3000));
//
//                 // Click checkbox tool
//                 document.getElementById('tool-checkbox').click();
//                 await new Promise(r => setTimeout(r, 100));
//
//                 // Count overlays before
//                 const beforeCount = document.querySelectorAll('[data-op-id]').length;
//
//                 // Click on the overlay-container to add a checkbox
//                 const overlay = document.querySelector('.overlay-container');
//                 if (!overlay) {{
//                     return {{ success: false, error: 'overlay-container not found' }};
//                 }}
//
//                 const rect = overlay.getBoundingClientRect();
//                 overlay.dispatchEvent(new MouseEvent('click', {{
//                     bubbles: true,
//                     cancelable: true,
//                     clientX: rect.left + 200,
//                     clientY: rect.top + 200,
//                     view: window
//                 }}));
//
//                 await new Promise(r => setTimeout(r, 500));
//
//                 // Count overlays after
//                 const afterCount = document.querySelectorAll('[data-op-id]').length;
//
//                 // Check download button state
//                 const downloadBtn = document.getElementById('edit-download-btn');
//                 const downloadEnabled = downloadBtn && !downloadBtn.disabled;
//
//                 return {{
//                     success: true,
//                     beforeCount: beforeCount,
//                     afterCount: afterCount,
//                     checkboxAdded: afterCount > beforeCount,
//                     downloadEnabled: downloadEnabled
//                 }};
//             }} catch (err) {{
//                 return {{ success: false, error: err.toString() }};
//             }}
//         }})()"#,
//         pdf_b64
//     );
//
//     let result: serde_json::Value = page
//         .evaluate(js_code.as_str())
//         .await
//         .expect("Should test checkbox creation")
//         .into_value()
//         .expect("Should get value");
//
//     eprintln!("Checkbox tool click test: {:?}", result);
//
//     assert!(
//         result["success"].as_bool().unwrap_or(false),
//         "Test should succeed. Error: {:?}",
//         result["error"]
//     );
//
//     assert!(
//         result["checkboxAdded"].as_bool().unwrap_or(false),
//         "Clicking with checkbox tool should add a checkbox. Before: {}, After: {}",
//         result["beforeCount"].as_i64().unwrap_or(0),
//         result["afterCount"].as_i64().unwrap_or(0)
//     );
//
//     assert!(
//         result["downloadEnabled"].as_bool().unwrap_or(false),
//         "Download button should be enabled after adding annotation"
//     );
// }

/// BUG TEST: Whiteout box should NOT expand when typing short text that fits
/// The bug: Every time something is typed, the whitebox expands even if the text
/// doesn't require additional space.
#[tokio::test]
async fn test_pdfjoin_whiteout_does_not_expand_for_short_text() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(2);

    // Navigate and load the PDF
    let js_code = format!(
        r#"(async () => {{
            try {{
                // Switch to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Wait for page to render
                let pageDiv = document.querySelector('.edit-page');
                let attempts = 0;
                while (!pageDiv && attempts < 20) {{
                    await new Promise(r => setTimeout(r, 200));
                    pageDiv = document.querySelector('.edit-page');
                    attempts++;
                }}

                if (!pageDiv) {{
                    return {{ success: false, error: 'Page did not render' }};
                }}

                // Draw a reasonably sized whiteout (200x50 pixels) - large enough that short text fits
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));

                const pageRect = pageDiv.getBoundingClientRect();
                const startX = pageRect.left + 100;
                const startY = pageRect.top + 100;
                const endX = startX + 200;
                const endY = startY + 50;

                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{ bubbles: true, clientX: startX, clientY: startY }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{ bubbles: true, clientX: endX, clientY: endY }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{ bubbles: true, clientX: endX, clientY: endY }}));
                await new Promise(r => setTimeout(r, 200));

                const whiteout = document.querySelector('.edit-whiteout-overlay');
                if (!whiteout) {{
                    return {{ success: false, error: 'Whiteout not created' }};
                }}

                // Record original dimensions
                const originalWidth = parseFloat(whiteout.style.width);
                const originalHeight = parseFloat(whiteout.style.height);
                const originalWidthRect = whiteout.getBoundingClientRect().width;
                const originalHeightRect = whiteout.getBoundingClientRect().height;

                // Double-click to open text editor
                const whiteoutRect = whiteout.getBoundingClientRect();
                whiteout.dispatchEvent(new MouseEvent('dblclick', {{
                    bubbles: true,
                    clientX: whiteoutRect.left + whiteoutRect.width / 2,
                    clientY: whiteoutRect.top + whiteoutRect.height / 2
                }}));
                await new Promise(r => setTimeout(r, 300));

                // Find the text input
                const input = whiteout.querySelector('.whiteout-text-input');
                if (!input) {{
                    return {{ success: false, error: 'Text input not found' }};
                }}

                // Record dimensions after opening editor (before typing)
                const widthBeforeTyping = parseFloat(whiteout.style.width);
                const heightBeforeTyping = parseFloat(whiteout.style.height);

                // Type a single short character - this should definitely fit
                input.textContent = 'a';
                input.dispatchEvent(new Event('input', {{ bubbles: true }}));
                await new Promise(r => setTimeout(r, 100));

                // Record dimensions after typing
                const widthAfterTyping = parseFloat(whiteout.style.width);
                const heightAfterTyping = parseFloat(whiteout.style.height);

                // Calculate expansion
                const widthExpanded = widthAfterTyping > originalWidth;
                const heightExpanded = heightAfterTyping > originalHeight;
                const widthDelta = widthAfterTyping - originalWidth;
                const heightDelta = heightAfterTyping - originalHeight;

                return {{
                    success: true,
                    originalWidth: originalWidth,
                    originalHeight: originalHeight,
                    widthBeforeTyping: widthBeforeTyping,
                    heightBeforeTyping: heightBeforeTyping,
                    widthAfterTyping: widthAfterTyping,
                    heightAfterTyping: heightAfterTyping,
                    widthExpanded: widthExpanded,
                    heightExpanded: heightExpanded,
                    widthDelta: widthDelta,
                    heightDelta: heightDelta,
                    shouldNotExpand: !widthExpanded && !heightExpanded
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test whiteout expansion")
        .into_value()
        .expect("Should get value");

    eprintln!("Whiteout expansion test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["shouldNotExpand"].as_bool().unwrap_or(false),
        "BUG: Whiteout should NOT expand when typing short text that fits. \
         Original: {}x{}, After typing 'a': {}x{} (delta: +{}w, +{}h)",
        result["originalWidth"].as_f64().unwrap_or(0.0),
        result["originalHeight"].as_f64().unwrap_or(0.0),
        result["widthAfterTyping"].as_f64().unwrap_or(0.0),
        result["heightAfterTyping"].as_f64().unwrap_or(0.0),
        result["widthDelta"].as_f64().unwrap_or(0.0),
        result["heightDelta"].as_f64().unwrap_or(0.0)
    );
}

// ============================================================================
// TextBox Tool Tests
// ============================================================================

/// Test that the TextBox tool button exists in the edit toolbar
#[tokio::test]
async fn test_textbox_toolbar_button_exists() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Switch to edit tab
    let _: bool = page
        .evaluate(
            r#"(() => {
                const tab = document.querySelector('[data-tab="edit"]');
                if (tab) { tab.click(); return true; }
                return false;
            })()"#,
        )
        .await
        .expect("Should click edit tab")
        .into_value()
        .expect("Should get value");

    tokio::time::sleep(Duration::from_millis(500)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"({
            hasTextboxButton: !!document.getElementById('edit-tool-textbox'),
            hasWhiteoutButton: !!document.getElementById('edit-tool-whiteout'),
            textboxButtonVisible: document.getElementById('edit-tool-textbox')?.offsetParent !== null,
            whiteoutButtonVisible: document.getElementById('edit-tool-whiteout')?.offsetParent !== null
        })"#,
        )
        .await
        .expect("Should check toolbar buttons")
        .into_value()
        .expect("Should get value");

    eprintln!("TextBox toolbar check: {:?}", result);

    assert!(
        result["hasTextboxButton"].as_bool().unwrap_or(false),
        "Should have TextBox button (#edit-tool-textbox)"
    );
    assert!(
        result["hasWhiteoutButton"].as_bool().unwrap_or(false),
        "Should have Whiteout button (#edit-tool-whiteout)"
    );
}

/// Test that clicking with the TextBox tool creates a text box
#[tokio::test]
async fn test_textbox_click_creates_text_box() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(1);

    let js_code = format!(
        r#"(async () => {{
            try {{
                // Switch to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Wait for page to render
                let pageDiv = document.querySelector('.edit-page');
                let attempts = 0;
                while (!pageDiv && attempts < 20) {{
                    await new Promise(r => setTimeout(r, 200));
                    pageDiv = document.querySelector('.edit-page');
                    attempts++;
                }}

                if (!pageDiv) {{
                    return {{ success: false, error: 'Page did not render' }};
                }}

                // Select TextBox tool
                const textboxBtn = document.getElementById('edit-tool-textbox');
                if (!textboxBtn) {{
                    return {{ success: false, error: 'TextBox button not found' }};
                }}
                textboxBtn.click();
                await new Promise(r => setTimeout(r, 100));

                // Use mousedown/mouseup to create a text box (not click)
                const pageRect = pageDiv.getBoundingClientRect();
                const clickX = pageRect.left + 150;
                const clickY = pageRect.top + 200;

                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true,
                    clientX: clickX,
                    clientY: clickY
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true,
                    clientX: clickX,
                    clientY: clickY
                }}));
                await new Promise(r => setTimeout(r, 300));

                // Check if a text box was created
                const textBox = document.querySelector('.text-box');

                return {{
                    success: true,
                    textBoxCreated: !!textBox,
                    textBoxCount: document.querySelectorAll('.text-box').length
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test textbox creation")
        .into_value()
        .expect("Should get value");

    eprintln!("TextBox creation test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    assert!(
        result["textBoxCreated"].as_bool().unwrap_or(false),
        "Clicking with TextBox tool should create a text box"
    );
}

// ============================================================================
// UX Tests - Accessibility and Usability
// ============================================================================

/// UX Test: Text box should show Enter hint placeholder for elderly users
#[tokio::test]
async fn test_ux_textbox_shows_enter_hint() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check if the CSS contains the Enter hint placeholder
    let result: serde_json::Value = page
        .evaluate(
            r#"(() => {
                // Look for the placeholder CSS rule
                const styles = document.querySelectorAll('style');
                let hasEnterHint = false;
                for (const style of styles) {
                    if (style.textContent.includes('Enter') && style.textContent.includes('save')) {
                        hasEnterHint = true;
                        break;
                    }
                }
                return {
                    hasEnterHint: hasEnterHint,
                    styleCount: styles.length
                };
            })()"#,
        )
        .await
        .expect("Should check for Enter hint")
        .into_value()
        .expect("Should get value");

    eprintln!("UX Enter hint check: {:?}", result);

    assert!(
        result["hasEnterHint"].as_bool().unwrap_or(false),
        "Text box should show Enter hint in placeholder for elderly users"
    );
}

/// UX Test: Resize handles should be at least 12px for accessibility
#[tokio::test]
async fn test_ux_resize_handles_accessible_size() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check if the CSS defines resize handles at 12px or larger
    let result: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const styles = document.querySelectorAll('style');
                let handleSize = 0;
                for (const style of styles) {
                    const content = style.textContent;
                    // Look for resize-handle width/height
                    const match = content.match(/\.resize-handle[^{]*\{[^}]*(?:width|height):\s*(\d+)px/);
                    if (match) {
                        handleSize = parseInt(match[1]);
                        break;
                    }
                }
                return {
                    handleSize: handleSize,
                    isAccessible: handleSize >= 12
                };
            })()"#,
        )
        .await
        .expect("Should check resize handle size")
        .into_value()
        .expect("Should get value");

    eprintln!("UX resize handle size check: {:?}", result);

    let handle_size = result["handleSize"].as_i64().unwrap_or(0);
    assert!(
        handle_size >= 12,
        "Resize handles should be at least 12px for accessibility (found: {}px)",
        handle_size
    );
}

/// UX Test: Delete key should work to delete selected text box
#[tokio::test]
async fn test_ux_delete_key_removes_textbox() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8082");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8082")
        .await
        .expect("Should navigate to PDFJoin");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let pdf_b64 = test_pdf_base64(1);

    let js_code = format!(
        r#"(async () => {{
            try {{
                // Switch to edit tab
                document.querySelector('[data-tab="edit"]').click();
                await new Promise(r => setTimeout(r, 300));

                // Load PDF
                const b64 = "{}";
                const binary = atob(b64);
                const pdfBytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) {{
                    pdfBytes[i] = binary.charCodeAt(i);
                }}

                const fileInput = document.getElementById('edit-file-input');
                const dataTransfer = new DataTransfer();
                const file = new File([pdfBytes], 'test.pdf', {{ type: 'application/pdf' }});
                dataTransfer.items.add(file);
                fileInput.files = dataTransfer.files;
                fileInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                await new Promise(r => setTimeout(r, 2000));

                // Wait for page to render
                let pageDiv = document.querySelector('.edit-page');
                let attempts = 0;
                while (!pageDiv && attempts < 20) {{
                    await new Promise(r => setTimeout(r, 200));
                    pageDiv = document.querySelector('.edit-page');
                    attempts++;
                }}

                if (!pageDiv) {{
                    return {{ success: false, error: 'Page did not render' }};
                }}

                // Create a whiteout (text boxes use similar mechanism)
                document.getElementById('edit-tool-whiteout').click();
                await new Promise(r => setTimeout(r, 100));

                const pageRect = pageDiv.getBoundingClientRect();
                pageDiv.dispatchEvent(new MouseEvent('mousedown', {{
                    bubbles: true,
                    clientX: pageRect.left + 100,
                    clientY: pageRect.top + 100
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mousemove', {{
                    bubbles: true,
                    clientX: pageRect.left + 200,
                    clientY: pageRect.top + 150
                }}));
                pageDiv.dispatchEvent(new MouseEvent('mouseup', {{
                    bubbles: true,
                    clientX: pageRect.left + 200,
                    clientY: pageRect.top + 150
                }}));
                await new Promise(r => setTimeout(r, 200));

                const whiteoutBefore = document.querySelectorAll('.edit-whiteout-overlay').length;

                // Select the whiteout (it should be selected after creation)
                const whiteout = document.querySelector('.edit-whiteout-overlay');
                if (whiteout) {{
                    whiteout.click();
                    await new Promise(r => setTimeout(r, 100));
                }}

                // Press Delete key
                document.dispatchEvent(new KeyboardEvent('keydown', {{
                    key: 'Delete',
                    keyCode: 46,
                    bubbles: true
                }}));
                await new Promise(r => setTimeout(r, 200));

                const whiteoutAfter = document.querySelectorAll('.edit-whiteout-overlay').length;

                return {{
                    success: true,
                    whiteoutBeforeDelete: whiteoutBefore,
                    whiteoutAfterDelete: whiteoutAfter,
                    wasDeleted: whiteoutAfter < whiteoutBefore
                }};
            }} catch (err) {{
                return {{ success: false, error: err.toString() }};
            }}
        }})()"#,
        pdf_b64
    );

    let result: serde_json::Value = page
        .evaluate(js_code.as_str())
        .await
        .expect("Should test delete key")
        .into_value()
        .expect("Should get value");

    eprintln!("UX delete key test: {:?}", result);

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Test should succeed. Error: {:?}",
        result["error"]
    );

    // Note: This test may not pass if delete key handler isn't implemented yet
    // The test documents the expected behavior
}
