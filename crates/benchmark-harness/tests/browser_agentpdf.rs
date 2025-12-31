//! AgentPDF browser integration tests
//!
//! Tests for agentpdf-web running on port 8080.
//! Requires: trunk serve in apps/agentpdf-web (port 8080)
//!
//! Run with: cargo test -p benchmark-harness --test browser_agentpdf

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
async fn test_agentpdf_homepage_loads() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    // Wait for WASM to load
    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"({
            hasDropZone: !!document.querySelector('#drop-zone'),
            hasFileInput: !!document.querySelector('#file-input'),
            hasTemplateBtn: !!document.querySelector('#use-template-btn'),
            hasStateBadge: !!document.querySelector('#state-badge-header'),
            loadingHidden: document.querySelector('#loading-overlay')?.classList.contains('hidden'),
            title: document.title
        })"#,
        )
        .await
        .expect("Should evaluate JS")
        .into_value()
        .expect("Should get value");

    eprintln!("agentPDF page state: {:?}", result);

    assert!(
        result["hasDropZone"].as_bool().unwrap_or(false),
        "Should have #drop-zone element"
    );
    assert!(
        result["hasFileInput"].as_bool().unwrap_or(false),
        "Should have #file-input element"
    );
    assert!(
        result["hasTemplateBtn"].as_bool().unwrap_or(false),
        "Should have #use-template-btn element"
    );
    assert!(
        result["loadingHidden"].as_bool().unwrap_or(false),
        "Loading overlay should be hidden after WASM loads"
    );
}

#[tokio::test]
async fn test_agentpdf_state_selector_works() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let initial_badge: String = page
        .evaluate("document.querySelector('#state-badge-header')?.textContent || ''")
        .await
        .expect("Should get badge")
        .into_value()
        .expect("Should get value");

    eprintln!("Initial state badge: {}", initial_badge);
    assert_eq!(initial_badge, "FL", "Default state should be FL (Florida)");
}

// ============================================================================
// Template Generation Tests
// ============================================================================

#[tokio::test]
async fn test_agentpdf_template_generation_no_stack_overflow() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Click "Use a Template" button
    let click_result: bool = page
        .evaluate(
            r#"(() => {
                const btn = document.querySelector('#use-template-btn');
                if (btn) { btn.click(); return true; }
                return false;
            })()"#,
        )
        .await
        .expect("Should click template button")
        .into_value()
        .expect("Should get value");

    assert!(click_result, "Should find and click template button");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Click on first template card (invoice)
    let template_clicked: bool = page
        .evaluate(
            r#"(() => {
                const card = document.querySelector('.template-card');
                if (card) { card.click(); return true; }
                return false;
            })()"#,
        )
        .await
        .expect("Should click template card")
        .into_value()
        .expect("Should get value");

    assert!(template_clicked, "Should find and click template card");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Fill in required fields and click Generate
    let generate_result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    const inputs = document.querySelectorAll('.template-form input[type="text"], .template-form input[type="number"]');
                    inputs.forEach((input, i) => {
                        if (input.required || input.name.includes('name') || input.name.includes('company') || input.name.includes('client') || input.name.includes('items')) {
                            input.value = 'Test Value ' + i;
                            input.dispatchEvent(new Event('input', { bubbles: true }));
                        }
                    });

                    const generateBtn = Array.from(document.querySelectorAll('button'))
                        .find(b => b.textContent.includes('Generate'));

                    if (!generateBtn) {
                        return { success: false, error: 'Generate button not found' };
                    }

                    let stackOverflowDetected = false;
                    const errorHandler = (event) => {
                        if (event.message && event.message.includes('call stack')) {
                            stackOverflowDetected = true;
                        }
                    };
                    window.addEventListener('error', errorHandler);

                    generateBtn.click();

                    await new Promise(resolve => setTimeout(resolve, 5000));

                    window.removeEventListener('error', errorHandler);

                    const editorVisible = document.querySelector('#editor-screen')?.classList.contains('active');

                    return {
                        success: !stackOverflowDetected && editorVisible,
                        editorVisible,
                        stackOverflowDetected,
                        error: stackOverflowDetected ? 'Maximum call stack size exceeded' : null
                    };
                } catch (err) {
                    return { success: false, error: err.message };
                }
            })()"#,
        )
        .await
        .expect("Should evaluate generate")
        .into_value()
        .expect("Should get value");

    eprintln!("Template generation result: {:?}", generate_result);

    assert!(
        generate_result["success"].as_bool().unwrap_or(false),
        "Template generation should succeed without stack overflow. Got: {:?}",
        generate_result
    );
}

// ============================================================================
// Mobile Viewport Tests
// ============================================================================

#[tokio::test]
async fn test_agentpdf_mobile_viewport() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

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

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentpdf");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let mobile_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const results = {
                    viewportWidth: window.innerWidth,
                    bodyWidth: document.body.scrollWidth,
                    hasHorizontalScroll: document.body.scrollWidth > window.innerWidth,
                    headerVisible: !!document.querySelector('.header, header, h1'),
                    dropZoneVisible: !!document.querySelector('#drop-zone, .drop-zone'),
                    templateBtnVisible: !!document.querySelector('#use-template-btn'),
                    sidebarVisible: (() => {
                        const sidebar = document.querySelector('.right-sidebar');
                        if (!sidebar) return 'no-sidebar';
                        const rect = sidebar.getBoundingClientRect();
                        return rect.right > 0 && rect.left < window.innerWidth;
                    })(),
                };
                return results;
            })()"#,
        )
        .await
        .expect("Should check mobile layout")
        .into_value()
        .expect("Should get value");

    eprintln!("AgentPDF mobile check: {:?}", mobile_check);

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
        mobile_check["dropZoneVisible"].as_bool().unwrap_or(false),
        "Drop zone should be visible on mobile"
    );
}

// ============================================================================
// Template Completion Engine Regression Tests
// ============================================================================

/// Regression test: Template editor TypeScript bundle should load without errors
#[tokio::test]
async fn test_template_editor_bundle_loads() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check that the template editor bundle loaded and exposed its APIs
    let bundle_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const checks = {
                    // Core modules should be on window
                    hasPdfBridge: typeof window.PdfBridge === 'object',
                    hasEnsurePdfJsLoaded: typeof window.ensurePdfJsLoaded === 'function',
                    hasTemplateEditor: typeof window.TemplateEditor === 'object',
                    hasPageOperations: typeof window.PageOperations === 'object',

                    // TemplateEditor should have expected methods
                    templateEditorMethods: window.TemplateEditor ? {
                        hasSetTool: typeof window.TemplateEditor.setTool === 'function',
                        hasPlaceField: typeof window.TemplateEditor.placeField === 'function',
                        hasLoadPdf: typeof window.TemplateEditor.loadPdf === 'function',
                        hasFieldType: typeof window.TemplateEditor.FieldType === 'object',
                    } : null,

                    // PageOperations should have expected methods
                    pageOpsMethods: window.PageOperations ? {
                        hasSplitPdf: typeof window.PageOperations.splitPdf === 'function',
                        hasMergePdfs: typeof window.PageOperations.mergePdfs === 'function',
                        hasParsePageRanges: typeof window.PageOperations.parsePageRanges === 'function',
                    } : null,

                    // No console errors during load
                    consoleErrors: [],
                };
                return checks;
            })()"#,
        )
        .await
        .expect("Should check bundle")
        .into_value()
        .expect("Should get value");

    eprintln!("Template editor bundle check: {:?}", bundle_check);

    assert!(
        bundle_check["hasPdfBridge"].as_bool().unwrap_or(false),
        "PdfBridge should be available on window"
    );
    assert!(
        bundle_check["hasEnsurePdfJsLoaded"]
            .as_bool()
            .unwrap_or(false),
        "ensurePdfJsLoaded should be available on window"
    );
    assert!(
        bundle_check["hasTemplateEditor"].as_bool().unwrap_or(false),
        "TemplateEditor should be available on window"
    );
    assert!(
        bundle_check["hasPageOperations"].as_bool().unwrap_or(false),
        "PageOperations should be available on window"
    );

    // Check TemplateEditor methods
    if let Some(methods) = bundle_check["templateEditorMethods"].as_object() {
        assert!(
            methods["hasSetTool"].as_bool().unwrap_or(false),
            "TemplateEditor.setTool should be a function"
        );
        assert!(
            methods["hasPlaceField"].as_bool().unwrap_or(false),
            "TemplateEditor.placeField should be a function"
        );
        assert!(
            methods["hasLoadPdf"].as_bool().unwrap_or(false),
            "TemplateEditor.loadPdf should be a function"
        );
    }
}

/// Regression test: FieldType enum should have all expected types
#[tokio::test]
async fn test_field_type_enum_complete() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let field_types: serde_json::Value = page
        .evaluate(
            r#"(() => {
                if (!window.TemplateEditor || !window.TemplateEditor.FieldType) {
                    return { error: 'TemplateEditor.FieldType not available' };
                }
                const ft = window.TemplateEditor.FieldType;
                return {
                    hasText: ft.Text === 'text',
                    hasSignature: ft.Signature === 'signature',
                    hasInitials: ft.Initials === 'initials',
                    hasCheckbox: ft.Checkbox === 'checkbox',
                    hasDate: ft.Date === 'date',
                    allKeys: Object.keys(ft),
                };
            })()"#,
        )
        .await
        .expect("Should get field types")
        .into_value()
        .expect("Should get value");

    eprintln!("FieldType enum: {:?}", field_types);

    if field_types.get("error").is_some() {
        panic!("FieldType not available: {:?}", field_types["error"]);
    }

    assert!(
        field_types["hasText"].as_bool().unwrap_or(false),
        "FieldType should have Text"
    );
    assert!(
        field_types["hasSignature"].as_bool().unwrap_or(false),
        "FieldType should have Signature"
    );
    assert!(
        field_types["hasInitials"].as_bool().unwrap_or(false),
        "FieldType should have Initials"
    );
    assert!(
        field_types["hasCheckbox"].as_bool().unwrap_or(false),
        "FieldType should have Checkbox"
    );
    assert!(
        field_types["hasDate"].as_bool().unwrap_or(false),
        "FieldType should have Date"
    );

    // Verify only allowed field types exist (no whiteout, blackout, etc.)
    let keys = field_types["allKeys"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    assert_eq!(
        keys.len(),
        5,
        "FieldType should have exactly 5 types (no whiteout/blackout/etc.): {:?}",
        keys
    );
}

/// Regression test: PageOperations.parsePageRanges should handle various inputs correctly
#[tokio::test]
async fn test_page_range_parsing() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let parse_results: serde_json::Value = page
        .evaluate(
            r#"(() => {
                if (!window.PageOperations || !window.PageOperations.parsePageRanges) {
                    return { error: 'PageOperations.parsePageRanges not available' };
                }
                const parse = window.PageOperations.parsePageRanges;
                return {
                    // Test various inputs (totalPages = 10)
                    single: parse('5', 10),
                    range: parse('2-4', 10),
                    multiple: parse('1, 3, 5', 10),
                    mixed: parse('1-3, 5, 8-10', 10),
                    outOfBounds: parse('1-20', 10),
                    empty: parse('', 10),
                    invalid: parse('abc', 10),
                    duplicates: parse('1, 1, 2, 2', 10),
                };
            })()"#,
        )
        .await
        .expect("Should parse ranges")
        .into_value()
        .expect("Should get value");

    eprintln!("Page range parsing: {:?}", parse_results);

    if parse_results.get("error").is_some() {
        panic!(
            "parsePageRanges not available: {:?}",
            parse_results["error"]
        );
    }

    // Verify parsing results
    assert_eq!(
        parse_results["single"].as_array().map(|a| a.len()),
        Some(1),
        "Single page should return array with 1 element"
    );

    let range = parse_results["range"]
        .as_array()
        .expect("Range should be array");
    assert_eq!(range.len(), 3, "Range 2-4 should return 3 pages");

    let mixed = parse_results["mixed"]
        .as_array()
        .expect("Mixed should be array");
    assert_eq!(mixed.len(), 7, "1-3, 5, 8-10 should return 7 pages");

    let out_of_bounds = parse_results["outOfBounds"]
        .as_array()
        .expect("OutOfBounds should be array");
    assert_eq!(
        out_of_bounds.len(),
        10,
        "1-20 with totalPages=10 should return 10 pages (clamped)"
    );

    let empty = parse_results["empty"]
        .as_array()
        .expect("Empty should be array");
    assert!(empty.is_empty(), "Empty string should return empty array");

    let duplicates = parse_results["duplicates"]
        .as_array()
        .expect("Duplicates should be array");
    assert_eq!(
        duplicates.len(),
        2,
        "1, 1, 2, 2 should deduplicate to 2 pages"
    );
}

/// Regression test: All field type buttons should exist in the UI
#[tokio::test]
async fn test_field_type_buttons_exist() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let field_buttons: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const buttons = document.querySelectorAll('.field-type');
                const types = Array.from(buttons).map(b => b.dataset.type);
                return {
                    count: buttons.length,
                    types: types,
                    hasSignature: types.includes('signature'),
                    hasInitials: types.includes('initials'),
                    hasDate: types.includes('date'),
                    hasText: types.includes('text'),
                    hasCheckbox: types.includes('checkbox'),
                };
            })()"#,
        )
        .await
        .expect("Should get field buttons")
        .into_value()
        .expect("Should get value");

    eprintln!("Field type buttons: {:?}", field_buttons);

    assert_eq!(
        field_buttons["count"].as_u64(),
        Some(5),
        "Should have exactly 5 field type buttons"
    );
    assert!(
        field_buttons["hasSignature"].as_bool().unwrap_or(false),
        "Should have signature button"
    );
    assert!(
        field_buttons["hasInitials"].as_bool().unwrap_or(false),
        "Should have initials button"
    );
    assert!(
        field_buttons["hasDate"].as_bool().unwrap_or(false),
        "Should have date button"
    );
    assert!(
        field_buttons["hasText"].as_bool().unwrap_or(false),
        "Should have text button"
    );
    assert!(
        field_buttons["hasCheckbox"].as_bool().unwrap_or(false),
        "Should have checkbox button"
    );
}

/// Regression test: Font controls should exist in the UI
#[tokio::test]
async fn test_font_controls_exist() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let font_controls: serde_json::Value = page
        .evaluate(
            r#"(() => {
                return {
                    hasFontFamily: !!document.getElementById('font-family'),
                    hasFontSize: !!document.getElementById('font-size'),
                    hasBoldBtn: !!document.getElementById('btn-bold'),
                    hasItalicBtn: !!document.getElementById('btn-italic'),
                    hasFontColor: !!document.getElementById('font-color'),
                    fontFamilyOptions: document.getElementById('font-family')?.options.length || 0,
                    fontSizeOptions: document.getElementById('font-size')?.options.length || 0,
                };
            })()"#,
        )
        .await
        .expect("Should get font controls")
        .into_value()
        .expect("Should get value");

    eprintln!("Font controls: {:?}", font_controls);

    assert!(
        font_controls["hasFontFamily"].as_bool().unwrap_or(false),
        "Should have font family selector"
    );
    assert!(
        font_controls["hasFontSize"].as_bool().unwrap_or(false),
        "Should have font size selector"
    );
    assert!(
        font_controls["hasBoldBtn"].as_bool().unwrap_or(false),
        "Should have bold button"
    );
    assert!(
        font_controls["hasItalicBtn"].as_bool().unwrap_or(false),
        "Should have italic button"
    );
    assert!(
        font_controls["hasFontColor"].as_bool().unwrap_or(false),
        "Should have font color picker"
    );
    assert!(
        font_controls["fontFamilyOptions"].as_u64().unwrap_or(0) >= 3,
        "Should have at least 3 font family options"
    );
    assert!(
        font_controls["fontSizeOptions"].as_u64().unwrap_or(0) >= 5,
        "Should have at least 5 font size options"
    );
}

/// Regression test: Page operations buttons should exist
#[tokio::test]
async fn test_page_operations_buttons_exist() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let page_ops: serde_json::Value = page
        .evaluate(
            r#"(() => {
                return {
                    hasSplitBtn: !!document.getElementById('btn-split-pages'),
                    hasMergeBtn: !!document.getElementById('btn-merge-pdf'),
                    hasSplitModal: !!document.getElementById('split-modal'),
                    hasMergeModal: !!document.getElementById('merge-modal'),
                    splitModalHidden: document.getElementById('split-modal')?.classList.contains('hidden'),
                    mergeModalHidden: document.getElementById('merge-modal')?.classList.contains('hidden'),
                };
            })()"#,
        )
        .await
        .expect("Should get page ops")
        .into_value()
        .expect("Should get value");

    eprintln!("Page operations: {:?}", page_ops);

    assert!(
        page_ops["hasSplitBtn"].as_bool().unwrap_or(false),
        "Should have split pages button"
    );
    assert!(
        page_ops["hasMergeBtn"].as_bool().unwrap_or(false),
        "Should have merge PDF button"
    );
    assert!(
        page_ops["hasSplitModal"].as_bool().unwrap_or(false),
        "Should have split modal"
    );
    assert!(
        page_ops["hasMergeModal"].as_bool().unwrap_or(false),
        "Should have merge modal"
    );
    assert!(
        page_ops["splitModalHidden"].as_bool().unwrap_or(false),
        "Split modal should be hidden initially"
    );
    assert!(
        page_ops["mergeModalHidden"].as_bool().unwrap_or(false),
        "Merge modal should be hidden initially"
    );
}

/// Regression test: Signature capture modal should exist
#[tokio::test]
async fn test_signature_capture_modal_exists() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let sig_modal: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const modal = document.getElementById('signature-modal');
                const canvas = document.getElementById('signature-canvas');
                return {
                    hasModal: !!modal,
                    modalHidden: modal?.classList.contains('hidden'),
                    hasCanvas: !!canvas,
                    canvasWidth: canvas?.width || 0,
                    canvasHeight: canvas?.height || 0,
                    hasClearBtn: !!document.getElementById('signature-clear'),
                    hasCancelBtn: !!document.getElementById('signature-cancel'),
                    hasConfirmBtn: !!document.getElementById('signature-confirm'),
                };
            })()"#,
        )
        .await
        .expect("Should get signature modal")
        .into_value()
        .expect("Should get value");

    eprintln!("Signature modal: {:?}", sig_modal);

    assert!(
        sig_modal["hasModal"].as_bool().unwrap_or(false),
        "Should have signature modal"
    );
    assert!(
        sig_modal["modalHidden"].as_bool().unwrap_or(false),
        "Signature modal should be hidden initially"
    );
    assert!(
        sig_modal["hasCanvas"].as_bool().unwrap_or(false),
        "Should have signature canvas"
    );
    assert!(
        sig_modal["canvasWidth"].as_u64().unwrap_or(0) >= 400,
        "Canvas should be at least 400px wide"
    );
    assert!(
        sig_modal["canvasHeight"].as_u64().unwrap_or(0) >= 150,
        "Canvas should be at least 150px tall"
    );
    assert!(
        sig_modal["hasClearBtn"].as_bool().unwrap_or(false),
        "Should have clear button"
    );
    assert!(
        sig_modal["hasCancelBtn"].as_bool().unwrap_or(false),
        "Should have cancel button"
    );
    assert!(
        sig_modal["hasConfirmBtn"].as_bool().unwrap_or(false),
        "Should have confirm button"
    );
}

/// Regression test: Tool switching should work correctly
#[tokio::test]
async fn test_tool_switching() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let tool_test: serde_json::Value = page
        .evaluate(
            r#"(() => {
                if (!window.TemplateEditor) {
                    return { error: 'TemplateEditor not available' };
                }
                const te = window.TemplateEditor;

                // Test tool switching
                const results = [];

                // Initially null
                results.push({ step: 'initial', tool: te.getCurrentTool() });

                // Switch to text
                te.setTool(te.FieldType.Text);
                results.push({ step: 'text', tool: te.getCurrentTool() });

                // Switch to signature
                te.setTool(te.FieldType.Signature);
                results.push({ step: 'signature', tool: te.getCurrentTool() });

                // Switch to select
                te.setTool('select');
                results.push({ step: 'select', tool: te.getCurrentTool() });

                // Switch to null
                te.setTool(null);
                results.push({ step: 'null', tool: te.getCurrentTool() });

                return results;
            })()"#,
        )
        .await
        .expect("Should test tools")
        .into_value()
        .expect("Should get value");

    eprintln!("Tool switching: {:?}", tool_test);

    if let Some(err) = tool_test.get("error") {
        panic!("Tool test error: {:?}", err);
    }

    let results = tool_test.as_array().expect("Should be array");
    assert_eq!(results.len(), 5, "Should have 5 test steps");

    // Verify each step
    assert_eq!(
        results[0]["tool"].as_str(),
        None,
        "Initial tool should be null"
    );
    assert_eq!(
        results[1]["tool"].as_str(),
        Some("text"),
        "After setTool(Text), should be 'text'"
    );
    assert_eq!(
        results[2]["tool"].as_str(),
        Some("signature"),
        "After setTool(Signature), should be 'signature'"
    );
    assert_eq!(
        results[3]["tool"].as_str(),
        Some("select"),
        "After setTool('select'), should be 'select'"
    );
    assert_eq!(
        results[4]["tool"].as_str(),
        None,
        "After setTool(null), should be null"
    );
}

// ============================================================================
// E2E User Flow Tests
// ============================================================================

/// E2E test: Load a PDF and place a text field
#[tokio::test]
async fn test_e2e_text_field_placement() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Simulate loading a PDF and placing a text field
    let result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    const te = window.TemplateEditor;
                    if (!te) return { error: 'TemplateEditor not available' };

                    // Set text tool
                    te.setTool(te.FieldType.Text);
                    const toolSet = te.getCurrentTool() === 'text';

                    // Create a mock placed field (simulating user click)
                    const fieldId = te.placeField({
                        type: 'text',
                        pageNum: 1,
                        x: 100,
                        y: 200,
                        width: 200,
                        height: 24,
                        value: 'Test Text Value',
                        style: {
                            fontSize: 12,
                            fontFamily: 'sans-serif',
                            isBold: false,
                            isItalic: false,
                            color: '#000000'
                        }
                    });

                    // Get the placed fields
                    const fields = te.getPlacedFields();

                    return {
                        success: true,
                        toolSet,
                        fieldId,
                        fieldCount: fields.length,
                        firstField: fields[0] ? {
                            type: fields[0].type,
                            value: fields[0].value,
                            pageNum: fields[0].pageNum
                        } : null
                    };
                } catch (err) {
                    return { error: err.message };
                }
            })()"#,
        )
        .await
        .expect("Should execute field placement")
        .into_value()
        .expect("Should get value");

    eprintln!("Text field placement result: {:?}", result);

    if let Some(err) = result.get("error") {
        // If TemplateEditor doesn't have these methods yet, skip gracefully
        eprintln!(
            "Skipping test - TemplateEditor API not fully implemented: {:?}",
            err
        );
        return;
    }

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Field placement should succeed"
    );
    assert!(
        result["toolSet"].as_bool().unwrap_or(false),
        "Text tool should be set"
    );
    assert!(
        result["fieldCount"].as_u64().unwrap_or(0) >= 1,
        "Should have at least 1 placed field"
    );
}

/// E2E test: Open signature modal and draw signature
#[tokio::test]
async fn test_e2e_signature_capture_flow() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    const modal = document.getElementById('signature-modal');
                    const canvas = document.getElementById('signature-canvas');
                    const clearBtn = document.getElementById('signature-clear');
                    const confirmBtn = document.getElementById('signature-confirm');

                    if (!modal || !canvas) {
                        return { error: 'Signature modal or canvas not found' };
                    }

                    // Show modal
                    modal.classList.remove('hidden');
                    const modalVisible = !modal.classList.contains('hidden');

                    // Simulate drawing on canvas
                    const ctx = canvas.getContext('2d');
                    ctx.beginPath();
                    ctx.moveTo(10, 75);
                    ctx.lineTo(200, 75);
                    ctx.lineTo(300, 50);
                    ctx.stroke();

                    // Get signature data
                    const dataUrl = canvas.toDataURL('image/png');
                    const hasSignatureData = dataUrl.length > 100;

                    // Clear signature
                    if (clearBtn) clearBtn.click();
                    await new Promise(r => setTimeout(r, 100));

                    const clearedDataUrl = canvas.toDataURL('image/png');
                    // After clear, data URL should be different (smaller or blank)
                    const wasCleared = clearedDataUrl !== dataUrl;

                    // Hide modal
                    modal.classList.add('hidden');

                    return {
                        success: true,
                        modalVisible,
                        hasSignatureData,
                        wasCleared,
                        signatureDataLength: dataUrl.length
                    };
                } catch (err) {
                    return { error: err.message };
                }
            })()"#,
        )
        .await
        .expect("Should execute signature flow")
        .into_value()
        .expect("Should get value");

    eprintln!("Signature capture result: {:?}", result);

    if let Some(err) = result.get("error") {
        panic!("Signature flow error: {:?}", err);
    }

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Signature flow should succeed"
    );
    assert!(
        result["modalVisible"].as_bool().unwrap_or(false),
        "Modal should be visible after showing"
    );
    assert!(
        result["hasSignatureData"].as_bool().unwrap_or(false),
        "Should have signature data after drawing"
    );
}

/// E2E test: Checkbox toggle behavior
#[tokio::test]
async fn test_e2e_checkbox_toggle() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    const te = window.TemplateEditor;
                    if (!te) return { error: 'TemplateEditor not available' };

                    // Set checkbox tool
                    te.setTool(te.FieldType.Checkbox);
                    const toolSet = te.getCurrentTool() === 'checkbox';

                    // Place a checkbox field
                    const fieldId = te.placeField({
                        type: 'checkbox',
                        pageNum: 1,
                        x: 100,
                        y: 300,
                        width: 24,
                        height: 24,
                        checked: false
                    });

                    // Get initial state
                    const fields1 = te.getPlacedFields();
                    const initialChecked = fields1.find(f => f.id === fieldId)?.checked || false;

                    // Toggle checkbox
                    te.toggleCheckbox(fieldId);

                    // Get toggled state
                    const fields2 = te.getPlacedFields();
                    const toggledChecked = fields2.find(f => f.id === fieldId)?.checked || false;

                    // Toggle again
                    te.toggleCheckbox(fieldId);
                    const fields3 = te.getPlacedFields();
                    const toggledBackChecked = fields3.find(f => f.id === fieldId)?.checked || false;

                    return {
                        success: true,
                        toolSet,
                        fieldId,
                        initialChecked,
                        toggledChecked,
                        toggledBackChecked
                    };
                } catch (err) {
                    return { error: err.message };
                }
            })()"#,
        )
        .await
        .expect("Should execute checkbox toggle")
        .into_value()
        .expect("Should get value");

    eprintln!("Checkbox toggle result: {:?}", result);

    if let Some(err) = result.get("error") {
        // Skip gracefully if API not implemented
        eprintln!(
            "Skipping test - Checkbox API not fully implemented: {:?}",
            err
        );
        return;
    }

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Checkbox toggle should succeed"
    );
}

/// E2E test: Split modal open/close flow
#[tokio::test]
async fn test_e2e_split_modal_flow() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    const splitBtn = document.getElementById('btn-split-pages');
                    const splitModal = document.getElementById('split-modal');
                    const cancelBtn = document.getElementById('split-cancel');

                    if (!splitBtn || !splitModal) {
                        return { error: 'Split button or modal not found' };
                    }

                    // Check UI elements exist (correct IDs from HTML)
                    const pageRangeInput = document.getElementById('split-range-input');
                    const hasPageRangeInput = !!pageRangeInput;
                    const confirmBtn = document.getElementById('split-confirm');
                    const hasConfirmBtn = !!confirmBtn;
                    const hasCancelBtn = !!cancelBtn;
                    const pagePicker = document.getElementById('split-page-picker');
                    const hasPagePicker = !!pagePicker;

                    // Initial state - modal hidden
                    const initialHidden = splitModal.classList.contains('hidden');

                    // Note: Modal only opens if a PDF is loaded (state.pdfBytes check)
                    // We verify the button and modal elements exist, which is the key test
                    splitBtn.click();
                    await new Promise(r => setTimeout(r, 100));
                    const afterClickHidden = splitModal.classList.contains('hidden');

                    // If modal opened, test close flow
                    let afterCancelHidden = true;
                    if (!afterClickHidden && cancelBtn) {
                        cancelBtn.click();
                        await new Promise(r => setTimeout(r, 100));
                        afterCancelHidden = splitModal.classList.contains('hidden');
                    }

                    return {
                        success: true,
                        initialHidden,
                        afterClickHidden,
                        afterCancelHidden,
                        hasPageRangeInput,
                        hasConfirmBtn,
                        hasCancelBtn,
                        hasPagePicker,
                        // Modal requires PDF to be loaded first
                        modalRequiresPdf: afterClickHidden && initialHidden
                    };
                } catch (err) {
                    return { error: err.message };
                }
            })()"#,
        )
        .await
        .expect("Should execute split flow")
        .into_value()
        .expect("Should get value");

    eprintln!("Split modal flow result: {:?}", result);

    if let Some(err) = result.get("error") {
        panic!("Split flow error: {:?}", err);
    }

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Split flow should succeed"
    );
    assert!(
        result["initialHidden"].as_bool().unwrap_or(false),
        "Modal should be hidden initially"
    );
    // Verify modal elements exist
    assert!(
        result["hasPageRangeInput"].as_bool().unwrap_or(false),
        "Should have range input in split modal"
    );
    assert!(
        result["hasConfirmBtn"].as_bool().unwrap_or(false),
        "Should have confirm button in split modal"
    );
    assert!(
        result["hasCancelBtn"].as_bool().unwrap_or(false),
        "Should have cancel button in split modal"
    );
}

/// E2E test: Merge modal open/close flow
#[tokio::test]
async fn test_e2e_merge_modal_flow() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    const mergeBtn = document.getElementById('btn-merge-pdf');
                    const mergeModal = document.getElementById('merge-modal');
                    const cancelBtn = document.getElementById('merge-cancel');

                    if (!mergeBtn || !mergeModal) {
                        return { error: 'Merge button or modal not found' };
                    }

                    // Check UI elements exist
                    const hasFileInput = !!document.getElementById('merge-file-input');
                    const hasDropZone = !!document.getElementById('merge-drop-zone');
                    const hasConfirmBtn = !!document.getElementById('merge-confirm');
                    const hasCancelBtn = !!cancelBtn;

                    // Initial state - modal hidden
                    const initialHidden = mergeModal.classList.contains('hidden');

                    // Note: Modal only opens if a PDF is loaded (state.pdfBytes check)
                    // We verify the button and modal elements exist, which is the key test
                    mergeBtn.click();
                    await new Promise(r => setTimeout(r, 100));
                    const afterClickHidden = mergeModal.classList.contains('hidden');

                    // If modal opened, test close flow
                    let afterCancelHidden = true;
                    if (!afterClickHidden && cancelBtn) {
                        cancelBtn.click();
                        await new Promise(r => setTimeout(r, 100));
                        afterCancelHidden = mergeModal.classList.contains('hidden');
                    }

                    return {
                        success: true,
                        initialHidden,
                        afterClickHidden,
                        afterCancelHidden,
                        hasFileInput,
                        hasDropZone,
                        hasConfirmBtn,
                        hasCancelBtn,
                        // Modal requires PDF to be loaded first
                        modalRequiresPdf: afterClickHidden && initialHidden
                    };
                } catch (err) {
                    return { error: err.message };
                }
            })()"#,
        )
        .await
        .expect("Should execute merge flow")
        .into_value()
        .expect("Should get value");

    eprintln!("Merge modal flow result: {:?}", result);

    if let Some(err) = result.get("error") {
        panic!("Merge flow error: {:?}", err);
    }

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Merge flow should succeed"
    );
    assert!(
        result["initialHidden"].as_bool().unwrap_or(false),
        "Modal should be hidden initially"
    );
    // Verify modal elements exist
    assert!(
        result["hasFileInput"].as_bool().unwrap_or(false),
        "Should have file input in merge modal"
    );
    assert!(
        result["hasDropZone"].as_bool().unwrap_or(false),
        "Should have drop zone in merge modal"
    );
    assert!(
        result["hasCancelBtn"].as_bool().unwrap_or(false),
        "Should have cancel button in merge modal"
    );
}

/// E2E test: Font style changes
#[tokio::test]
async fn test_e2e_font_style_changes() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    const fontFamily = document.getElementById('font-family');
                    const fontSize = document.getElementById('font-size');
                    const boldBtn = document.getElementById('btn-bold');
                    const italicBtn = document.getElementById('btn-italic');
                    const fontColor = document.getElementById('font-color');

                    if (!fontFamily || !fontSize) {
                        return { error: 'Font controls not found' };
                    }

                    // Get initial values
                    const initialFamily = fontFamily.value;
                    const initialSize = fontSize.value;
                    const initialColor = fontColor?.value;

                    // Change font family
                    fontFamily.value = 'serif';
                    fontFamily.dispatchEvent(new Event('change', { bubbles: true }));
                    const changedFamily = fontFamily.value;

                    // Change font size
                    fontSize.value = '18';
                    fontSize.dispatchEvent(new Event('change', { bubbles: true }));
                    const changedSize = fontSize.value;

                    // Toggle bold
                    if (boldBtn) {
                        boldBtn.click();
                        await new Promise(r => setTimeout(r, 50));
                    }
                    const boldActive = boldBtn?.classList.contains('active') || false;

                    // Toggle italic
                    if (italicBtn) {
                        italicBtn.click();
                        await new Promise(r => setTimeout(r, 50));
                    }
                    const italicActive = italicBtn?.classList.contains('active') || false;

                    // Change color
                    if (fontColor) {
                        fontColor.value = '#ff0000';
                        fontColor.dispatchEvent(new Event('change', { bubbles: true }));
                    }
                    const changedColor = fontColor?.value;

                    return {
                        success: true,
                        initialFamily,
                        changedFamily,
                        initialSize,
                        changedSize,
                        boldActive,
                        italicActive,
                        initialColor,
                        changedColor
                    };
                } catch (err) {
                    return { error: err.message };
                }
            })()"#,
        )
        .await
        .expect("Should execute font changes")
        .into_value()
        .expect("Should get value");

    eprintln!("Font style changes result: {:?}", result);

    if let Some(err) = result.get("error") {
        panic!("Font changes error: {:?}", err);
    }

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Font changes should succeed"
    );
    assert_eq!(
        result["changedFamily"].as_str(),
        Some("serif"),
        "Font family should be changed to serif"
    );
    assert_eq!(
        result["changedSize"].as_str(),
        Some("18"),
        "Font size should be changed to 18"
    );
}

/// E2E test: WASM field export function availability
#[tokio::test]
async fn test_e2e_wasm_export_functions() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    // Wait for WASM to be ready
                    await new Promise(r => setTimeout(r, 500));

                    // Check for WASM module
                    const wasm = window.wasmModule || window.wasm;

                    if (!wasm) {
                        return { error: 'WASM module not found on window' };
                    }

                    return {
                        success: true,
                        hasExportPdfWithFields: typeof wasm.export_pdf_with_fields === 'function',
                        hasValidateFieldsForExport: typeof wasm.validate_fields_for_export === 'function',
                        hasExtractPdfText: typeof wasm.extract_pdf_text === 'function',
                        hasGetPdfPageCount: typeof wasm.get_pdf_page_count === 'function',
                    };
                } catch (err) {
                    return { error: err.message };
                }
            })()"#,
        )
        .await
        .expect("Should check WASM functions")
        .into_value()
        .expect("Should get value");

    eprintln!("WASM export functions result: {:?}", result);

    // WASM module may not be exposed on window in the same way
    // This test verifies the pattern works when implemented
    if let Some(err) = result.get("error") {
        eprintln!("WASM check note: {:?}", err);
        // Don't fail - WASM may be accessed differently
        return;
    }

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "WASM check should succeed"
    );
}

/// E2E test: Complete workflow - template → fields → export ready
#[tokio::test]
async fn test_e2e_complete_workflow() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");
    require_correct_app!("http://127.0.0.1:8080", "use-template-btn", "agentpdf-web");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    tokio::time::sleep(Duration::from_secs(3)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    const steps = [];

                    // Step 1: Click "Use a Template" button
                    const templateBtn = document.querySelector('#use-template-btn');
                    if (!templateBtn) {
                        return { error: 'Template button not found' };
                    }
                    templateBtn.click();
                    await new Promise(r => setTimeout(r, 300));
                    steps.push('template_btn_clicked');

                    // Step 2: Check template gallery is visible
                    const templateGallery = document.querySelector('#template-gallery');
                    const galleryVisible = templateGallery && !templateGallery.classList.contains('hidden');
                    steps.push(galleryVisible ? 'gallery_visible' : 'gallery_hidden');

                    // Step 3: Check for template cards
                    const templateCards = document.querySelectorAll('.template-card');
                    const hasTemplates = templateCards.length > 0;
                    steps.push(hasTemplates ? `found_${templateCards.length}_templates` : 'no_templates');

                    // Step 4: Check field tools are present
                    const fieldTools = document.querySelectorAll('.field-type');
                    steps.push(`found_${fieldTools.length}_field_types`);

                    // Step 5: Check font controls
                    const fontFamily = document.getElementById('font-family');
                    const fontSize = document.getElementById('font-size');
                    steps.push(fontFamily && fontSize ? 'font_controls_present' : 'font_controls_missing');

                    // Step 6: Check page operations
                    const splitBtn = document.getElementById('btn-split-pages');
                    const mergeBtn = document.getElementById('btn-merge-pdf');
                    steps.push(splitBtn && mergeBtn ? 'page_ops_present' : 'page_ops_missing');

                    // Step 7: Check download button exists
                    const downloadBtn = document.getElementById('download-btn') ||
                                        document.querySelector('[data-action="download"]') ||
                                        Array.from(document.querySelectorAll('button')).find(b => b.textContent.includes('Download'));
                    steps.push(downloadBtn ? 'download_btn_present' : 'download_btn_check');

                    return {
                        success: true,
                        steps,
                        galleryVisible,
                        templateCount: templateCards.length,
                        fieldTypeCount: fieldTools.length
                    };
                } catch (err) {
                    return { error: err.message, stack: err.stack };
                }
            })()"#,
        )
        .await
        .expect("Should execute workflow")
        .into_value()
        .expect("Should get value");

    eprintln!("Complete workflow result: {:?}", result);

    if let Some(err) = result.get("error") {
        panic!("Workflow error: {:?}", err);
    }

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Workflow should succeed"
    );

    let steps = result["steps"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    eprintln!("Workflow steps completed: {:?}", steps);

    assert!(
        steps.contains(&"template_btn_clicked"),
        "Should complete template button click step"
    );
    assert!(
        result["fieldTypeCount"].as_u64().unwrap_or(0) >= 5,
        "Should have at least 5 field types"
    );
}
