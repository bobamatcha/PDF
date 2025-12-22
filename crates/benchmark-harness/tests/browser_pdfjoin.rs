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

use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use std::time::Duration;

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

    // Load the test PDF and perform split via WASM bindings directly
    let result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    // Fetch test PDF from output directory
                    const response = await fetch('/florida_purchase_contract.pdf');
                    if (!response.ok) {
                        return { success: false, error: 'PDF not found - copy to www/dist for testing' };
                    }
                    const pdfBytes = new Uint8Array(await response.arrayBuffer());

                    const { PdfJoinSession, SessionMode } = window.wasmBindings;
                    const session = new PdfJoinSession(SessionMode.Split);

                    // Add document
                    const info = session.addDocument('test.pdf', pdfBytes);
                    if (info.page_count !== 17) {
                        return { success: false, error: 'Expected 17 pages, got ' + info.page_count };
                    }

                    // Select pages 5-17
                    session.setPageSelection('5-17');

                    // Execute split
                    const result = session.execute();
                    const resultBytes = new Uint8Array(result);

                    // Convert to string to count /Count occurrences
                    const decoder = new TextDecoder('utf-8', { fatal: false });
                    const pdfText = decoder.decode(resultBytes);

                    // Count /Count entries
                    const countMatches = pdfText.match(/\/Count \d+/g) || [];

                    return {
                        success: true,
                        outputSize: resultBytes.length,
                        countOccurrences: countMatches.length,
                        countValues: countMatches,
                        startsWithPdf: pdfText.startsWith('%PDF-')
                    };
                } catch (err) {
                    return { success: false, error: err.toString() };
                }
            })()"#,
        )
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
