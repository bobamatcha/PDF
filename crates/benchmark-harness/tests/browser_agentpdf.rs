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
