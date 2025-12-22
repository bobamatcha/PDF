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

                // Check for example chips
                const editor = document.getElementById('split-editor');
                if (!editor) {{
                    return {{ success: false, error: 'split-editor not found' }};
                }}

                const chips = editor.querySelectorAll('.chip, .example-chip, .range-chip, [data-range]');
                const chipTexts = Array.from(chips).map(c => c.textContent?.trim());

                const smallButtons = editor.querySelectorAll('.range-examples button, .chips button');
                const buttonTexts = Array.from(smallButtons).map(b => b.textContent?.trim());

                return {{
                    success: true,
                    chipCount: chips.length,
                    chipTexts: chipTexts,
                    buttonCount: smallButtons.length,
                    buttonTexts: buttonTexts,
                    hasChips: chips.length > 0 || smallButtons.length > 0
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
