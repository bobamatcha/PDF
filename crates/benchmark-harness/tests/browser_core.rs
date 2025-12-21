//! Core browser tests for benchmark-harness
//!
//! These tests verify the browser automation infrastructure works correctly.
//! They don't require local servers - they test against example.com or about:blank.
//!
//! Run with: cargo test -p benchmark-harness --test browser_core

#[path = "common/browser.rs"]
mod browser;

use benchmark_harness::config::{Config, NetworkProfile};
use benchmark_harness::runner::BenchmarkRunner;
use benchmark_harness::throttling::{CpuThrottler, NetworkThrottler};
use std::time::Duration;

#[tokio::test]
async fn test_browser_launches_headless() {
    skip_if_no_chrome!();

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

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

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    let result = page.goto("https://example.com").await;
    assert!(result.is_ok(), "Should navigate to example.com");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let title = page.evaluate("document.title").await;
    assert!(title.is_ok(), "Should be able to evaluate JS");
}

#[tokio::test]
async fn test_network_throttling_applies() {
    skip_if_no_chrome!();

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    let result = NetworkThrottler::apply(&page, NetworkProfile::Fast3G).await;
    assert!(result.is_ok(), "Should apply network throttling");

    let result = NetworkThrottler::clear(&page).await;
    assert!(result.is_ok(), "Should clear network throttling");
}

#[tokio::test]
async fn test_cpu_throttling_applies() {
    skip_if_no_chrome!();

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    let result = CpuThrottler::apply(&page, 4.0).await;
    assert!(result.is_ok(), "Should apply CPU throttling");

    let result = CpuThrottler::clear(&page).await;
    assert!(result.is_ok(), "Should clear CPU throttling");
}

#[tokio::test]
async fn test_multiple_pages_parallel() {
    skip_if_no_chrome!();

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

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

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.expect("Task should complete"));
    }

    assert_eq!(results.len(), 3, "All 3 pages should complete");
}

#[tokio::test]
async fn test_benchmark_runner_creation() {
    skip_if_no_chrome!();

    match BenchmarkRunner::new().await {
        Ok(_) => (),
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
            }
        }
        Err(e) => eprintln!("Benchmark failed: {:?}", e),
    }
    assert!(results.is_ok(), "Benchmark should complete");

    let results = results.unwrap();
    assert_eq!(results.suite_name, "Test Benchmark");
    assert_eq!(results.scenario_results.len(), 1);
}
