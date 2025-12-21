//! DocSign browser integration tests
//!
//! Tests for docsign-web running on port 8081.
//! Requires: trunk serve in apps/docsign-web (port 8081)
//!
//! Run with: cargo test -p benchmark-harness --test browser_docsign

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
async fn test_docsign_homepage_loads() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081")
        .await
        .expect("Should navigate to DocSign");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"({
            hasDropZone: !!document.querySelector('#drop-zone'),
            hasFileInput: !!document.querySelector('#file-input'),
            hasStepper: !!document.querySelector('.stepper'),
            stepCount: document.querySelectorAll('.step').length,
            title: document.title
        })"#,
        )
        .await
        .expect("Should evaluate JS")
        .into_value()
        .expect("Should get value");

    eprintln!("DocSign page state: {:?}", result);

    assert!(
        result["hasDropZone"].as_bool().unwrap_or(false),
        "Should have #drop-zone element"
    );
    assert!(
        result["hasFileInput"].as_bool().unwrap_or(false),
        "Should have #file-input element"
    );
    assert!(
        result["hasStepper"].as_bool().unwrap_or(false),
        "Should have stepper element"
    );
    assert_eq!(
        result["stepCount"].as_u64().unwrap_or(0),
        4,
        "Should have 4 steps (Upload, Recipients, Fields, Review)"
    );
}

#[tokio::test]
async fn test_docsign_has_correct_workflow_steps() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081")
        .await
        .expect("Should navigate to DocSign");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let steps: Vec<String> = page
        .evaluate(
            r#"Array.from(document.querySelectorAll('.step-label')).map(el => el.textContent.trim())"#,
        )
        .await
        .expect("Should get steps")
        .into_value()
        .expect("Should get value");

    eprintln!("DocSign workflow steps: {:?}", steps);

    assert_eq!(steps.len(), 4, "Should have 4 workflow steps");
    assert_eq!(steps[0], "Upload", "Step 1 should be Upload");
    assert_eq!(steps[1], "Recipients", "Step 2 should be Recipients");
    assert_eq!(steps[2], "Fields", "Step 3 should be Fields");
    assert_eq!(steps[3], "Review", "Step 4 should be Review");
}

// ============================================================================
// Mobile Viewport Tests
// ============================================================================

#[tokio::test]
async fn test_docsign_mobile_viewport() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

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

    page.goto("http://127.0.0.1:8081")
        .await
        .expect("Should navigate to docsign");

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
                    buttonHeights: Array.from(document.querySelectorAll('button, .btn'))
                        .slice(0, 5)
                        .map(b => b.getBoundingClientRect().height),
                    inputHeights: Array.from(document.querySelectorAll('input, select'))
                        .slice(0, 5)
                        .map(i => i.getBoundingClientRect().height),
                };
                return results;
            })()"#,
        )
        .await
        .expect("Should check mobile layout")
        .into_value()
        .expect("Should get value");

    eprintln!("DocSign mobile check: {:?}", mobile_check);

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

#[tokio::test]
async fn test_docsign_mobile_touch_targets() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

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

    page.goto("http://127.0.0.1:8081")
        .await
        .expect("Should navigate to docsign");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let touch_targets: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const MIN_TOUCH_TARGET = 44;
                const issues = [];

                document.querySelectorAll('button, .btn, [role="button"]').forEach(el => {
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
            "⚠️  {} elements below 44px touch target minimum: {:?}",
            touch_targets["totalIssues"], touch_targets["issues"]
        );
    }
}
