//! Browser integration tests for benchmark-harness
//!
//! These tests require Chrome/Chromium to be installed.
//! Run with: cargo test -p benchmark-harness --test browser_tests
//!
//! In CI, Chrome is installed via `browser-actions/setup-chrome@v1`
//!
//! To skip these tests locally when Chrome isn't installed:
//!   SKIP_BROWSER_TESTS=1 cargo test -p benchmark-harness --test browser_tests

use anyhow::Result;
use benchmark_harness::config::{Config, NetworkProfile};
use benchmark_harness::runner::BenchmarkRunner;
use benchmark_harness::throttling::{CpuThrottler, NetworkThrottler};
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use std::time::Duration;

/// Check if browser tests should be skipped (when Chrome isn't available)
fn should_skip() -> bool {
    std::env::var("SKIP_BROWSER_TESTS").is_ok()
}

/// Macro to skip test if Chrome isn't available
macro_rules! skip_if_no_chrome {
    () => {
        if should_skip() {
            eprintln!("Skipping test: SKIP_BROWSER_TESTS is set");
            return;
        }
    };
}

/// Find Chrome for Testing installed by Puppeteer
fn find_chrome_for_testing() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let puppeteer_cache = std::path::Path::new(&home).join(".cache/puppeteer/chrome");

    if puppeteer_cache.exists() {
        if let Ok(entries) = std::fs::read_dir(&puppeteer_cache) {
            let mut versions: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            versions.sort_by_key(|v| std::cmp::Reverse(v.path()));

            for version_dir in versions {
                // macOS arm64
                let chrome_app = version_dir.path().join(
                    "chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
                );
                if chrome_app.exists() {
                    return Some(chrome_app);
                }
                // macOS x64
                let chrome_app_x64 = version_dir.path().join(
                    "chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
                );
                if chrome_app_x64.exists() {
                    return Some(chrome_app_x64);
                }
                // Linux
                let chrome_linux = version_dir.path().join("chrome-linux64/chrome");
                if chrome_linux.exists() {
                    return Some(chrome_linux);
                }
            }
        }
    }
    None
}

/// Helper to create a headless browser for testing
async fn create_test_browser() -> Result<(Browser, tokio::task::JoinHandle<()>)> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static BROWSER_ID: AtomicU64 = AtomicU64::new(0);

    let mut builder = BrowserConfig::builder();

    // Use Chrome for Testing if available (same as Puppeteer)
    if let Some(chrome_path) = find_chrome_for_testing() {
        eprintln!("Using Chrome for Testing: {}", chrome_path.display());
        builder = builder.chrome_executable(chrome_path);
    }

    // Use unique user data directory to avoid conflicts when running tests in parallel
    let browser_id = BROWSER_ID.fetch_add(1, Ordering::SeqCst);
    let user_data_dir = std::env::temp_dir().join(format!("benchmark-harness-test-{}", browser_id));
    builder = builder.user_data_dir(user_data_dir);

    let config = builder
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build browser config: {}", e))?;

    let (browser, mut handler) = Browser::launch(config).await?;

    let handle = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(e) = event {
                eprintln!("Browser handler error: {:?}", e);
                break;
            }
        }
    });

    // Give browser a moment to fully initialize
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok((browser, handle))
}

/// Try to create browser, skip test if Chrome not found
async fn require_browser() -> Option<(Browser, tokio::task::JoinHandle<()>)> {
    match create_test_browser().await {
        Ok(browser) => Some(browser),
        Err(e) => {
            if e.to_string().contains("Could not auto detect") {
                eprintln!("Skipping: Chrome not installed ({})", e);
                None
            } else {
                panic!("Unexpected browser error: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_browser_launches_headless() {
    skip_if_no_chrome!();

    let Some((browser, _handle)) = require_browser().await else {
        return; // Skip if Chrome not found
    };

    // Create a page and verify it works
    let page = browser.new_page("about:blank").await;
    match &page {
        Ok(_) => eprintln!("Page created successfully"),
        Err(e) => eprintln!("Page creation failed: {:?}", e),
    }
    assert!(page.is_ok(), "Should be able to create a new page");
}

#[tokio::test]
async fn test_navigate_to_example_com() {
    skip_if_no_chrome!();

    let Some((browser, _handle)) = require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    // Navigate to a real page
    let result = page.goto("https://example.com").await;
    assert!(result.is_ok(), "Should navigate to example.com");

    // Wait a moment for page to load
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify we can get the title
    let title = page.evaluate("document.title").await;
    assert!(title.is_ok(), "Should be able to evaluate JS");
}

#[tokio::test]
async fn test_network_throttling_applies() {
    skip_if_no_chrome!();

    let Some((browser, _handle)) = require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    // Apply Fast3G throttling
    let result = NetworkThrottler::apply(&page, NetworkProfile::Fast3G).await;
    assert!(result.is_ok(), "Should apply network throttling");

    // Clear throttling
    let result = NetworkThrottler::clear(&page).await;
    assert!(result.is_ok(), "Should clear network throttling");
}

#[tokio::test]
async fn test_cpu_throttling_applies() {
    skip_if_no_chrome!();

    let Some((browser, _handle)) = require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    // Apply 4x CPU throttling
    let result = CpuThrottler::apply(&page, 4.0).await;
    assert!(result.is_ok(), "Should apply CPU throttling");

    // Clear throttling
    let result = CpuThrottler::clear(&page).await;
    assert!(result.is_ok(), "Should clear CPU throttling");
}

#[tokio::test]
async fn test_multiple_pages_parallel() {
    skip_if_no_chrome!();

    let Some((browser, _handle)) = require_browser().await else {
        return;
    };

    // Create multiple pages in parallel (simulating parallel contexts)
    let mut handles = Vec::new();

    for i in 0..3 {
        let page = browser
            .new_page("about:blank")
            .await
            .expect("Should create page");

        handles.push(tokio::spawn(async move {
            let url = format!("https://example.com/?page={}", i);
            page.goto(&url).await.expect("Should navigate");
            tokio::time::sleep(Duration::from_millis(500)).await;
            i
        }));
    }

    // Wait for all to complete
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.expect("Task should complete"));
    }

    assert_eq!(results.len(), 3, "All 3 pages should complete");
}

#[tokio::test]
async fn test_benchmark_runner_creation() {
    skip_if_no_chrome!();

    // This tests the full runner initialization
    match BenchmarkRunner::new().await {
        Ok(_) => (), // Success
        Err(e) if e.to_string().contains("Could not auto detect") => {
            eprintln!("Skipping: Chrome not installed");
            return;
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[tokio::test]
async fn test_simple_benchmark_run() {
    skip_if_no_chrome!();

    // Create a minimal config for testing against example.com
    let toml = r#"
        [benchmark]
        name = "Test Benchmark"
        base_url = "https://example.com"
        iterations = 2
        warmup = 1
        parallel_contexts = 1

        [throttling]
        network_profile = "None"
        cpu_slowdown = 1.0

        [[scenarios]]
        name = "Simple Load"
        steps = [
            { type = "navigate", url = "/" },
            { type = "wait", wait_for = "timeout", duration = 1000 },
            { type = "measure", label = "loaded" }
        ]
    "#;

    let config = Config::parse(toml).expect("Config should parse");

    let runner = match BenchmarkRunner::new().await {
        Ok(r) => r,
        Err(e) if e.to_string().contains("Could not auto detect") => {
            eprintln!("Skipping: Chrome not installed");
            return;
        }
        Err(e) => panic!("Unexpected error: {}", e),
    };

    let results = runner.run(&config).await;
    match &results {
        Ok(r) => {
            eprintln!("Benchmark completed: suite={}", r.suite_name);
            eprintln!("Total duration: {}ms", r.total_duration_ms);
            for (i, scenario) in r.scenario_results.iter().enumerate() {
                eprintln!(
                    "Scenario {}: {} - success={}, failed={}",
                    i,
                    scenario.scenario_name,
                    scenario.successful_iterations,
                    scenario.failed_iterations
                );
                eprintln!(
                    "  LCP: p50={:.2}ms p95={:.2}ms count={}",
                    scenario.lcp_summary.p50, scenario.lcp_summary.p95, scenario.lcp_summary.count
                );
            }
        }
        Err(e) => eprintln!("Benchmark failed: {:?}", e),
    }
    assert!(results.is_ok(), "Benchmark should complete");

    let results = results.unwrap();
    assert_eq!(results.suite_name, "Test Benchmark");
    assert_eq!(results.scenario_results.len(), 1);
    // Note: In a minimal test, LCP might not be captured reliably from example.com
    // Just verify the benchmark ran without crashing
    eprintln!(
        "Final check: successful_iterations = {}",
        results.scenario_results[0].successful_iterations
    );
}

// ============================================================================
// Local App Tests (require trunk serve running on ports 8080/8081)
// ============================================================================

/// Check if a local server is available
async fn is_server_available(url: &str) -> bool {
    match reqwest::get(url).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Macro to skip test if local server isn't running
macro_rules! require_local_server {
    ($url:expr) => {{
        if !is_server_available($url).await {
            eprintln!("Skipping: Local server not running at {}", $url);
            return;
        }
    }};
}

#[tokio::test]
async fn test_agentpdf_homepage_loads() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");

    let Some((browser, _handle)) = require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    // Navigate to agentPDF
    page.goto("http://127.0.0.1:8080")
        .await
        .expect("Should navigate to agentPDF");

    // Wait for WASM to load (loading overlay should be hidden)
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Verify key elements exist
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

    let Some((browser, _handle)) = require_browser().await else {
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

    // Get initial state badge
    let initial_badge: String = page
        .evaluate("document.querySelector('#state-badge-header')?.textContent || ''")
        .await
        .expect("Should get badge")
        .into_value()
        .expect("Should get value");

    eprintln!("Initial state badge: {}", initial_badge);
    assert_eq!(initial_badge, "FL", "Default state should be FL (Florida)");
}

#[tokio::test]
async fn test_docsign_homepage_loads() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    // Navigate to DocSign
    page.goto("http://127.0.0.1:8081")
        .await
        .expect("Should navigate to DocSign");

    // Wait for page to load
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify key elements exist
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

    let Some((browser, _handle)) = require_browser().await else {
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

    // Get step labels
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

#[tokio::test]
async fn test_agentpdf_template_generation_no_stack_overflow() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8080");

    let Some((browser, _handle)) = require_browser().await else {
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

    // Wait for modal to appear
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

    // Wait for form modal
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Fill in required fields and click Generate
    let generate_result: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                try {
                    // Fill required fields
                    const inputs = document.querySelectorAll('.template-form input[type="text"], .template-form input[type="number"]');
                    inputs.forEach((input, i) => {
                        if (input.required || input.name.includes('name') || input.name.includes('company') || input.name.includes('client') || input.name.includes('items')) {
                            input.value = 'Test Value ' + i;
                            input.dispatchEvent(new Event('input', { bubbles: true }));
                        }
                    });

                    // Find and click generate button
                    const generateBtn = Array.from(document.querySelectorAll('button'))
                        .find(b => b.textContent.includes('Generate'));

                    if (!generateBtn) {
                        return { success: false, error: 'Generate button not found' };
                    }

                    // Set up error listener to catch stack overflow
                    let stackOverflowDetected = false;
                    const errorHandler = (event) => {
                        if (event.message && event.message.includes('call stack')) {
                            stackOverflowDetected = true;
                        }
                    };
                    window.addEventListener('error', errorHandler);

                    // Click generate
                    generateBtn.click();

                    // Wait for generation (or error)
                    await new Promise(resolve => setTimeout(resolve, 5000));

                    window.removeEventListener('error', errorHandler);

                    // Check if editor screen appeared (success) or we got an error
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

    // This test should FAIL if there's a stack overflow bug
    assert!(
        generate_result["success"].as_bool().unwrap_or(false),
        "Template generation should succeed without stack overflow. Got: {:?}",
        generate_result
    );
}
