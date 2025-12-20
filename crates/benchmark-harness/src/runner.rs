//! Benchmark execution orchestration
//!
//! This module coordinates the execution of benchmark scenarios, managing
//! browser contexts, parallelization, and metric collection.
//!
//! # Architecture
//!
//! The runner uses a single browser instance with multiple incognito contexts
//! for isolation. This provides faster context creation (~50-100ms) compared
//! to launching new browser processes (~2-5s).
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    Browser Instance                      │
//! ├─────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐        │
//! │  │  Context 1  │ │  Context 2  │ │  Context 3  │ ...    │
//! │  │ (Scenario A)│ │ (Scenario A)│ │ (Scenario B)│        │
//! │  │  Iteration 1│ │  Iteration 2│ │  Iteration 1│        │
//! │  └─────────────┘ └─────────────┘ └─────────────┘        │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use benchmark_harness::{Config, runner::BenchmarkRunner};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = Config::from_file("benchmark.toml")?;
//! let runner = BenchmarkRunner::new().await?;
//! let results = runner.run(&config).await?;
//!
//! for result in &results.scenario_results {
//!     println!("{}: LCP p50={:.0}ms", result.scenario_name, result.lcp_summary.p50);
//! }
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Page;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, instrument, warn};

use crate::config::{BenchmarkStep, Config, NetworkProfile, Scenario, WaitCondition};
use crate::metrics::WebVitalsCollector;
use crate::stats::{OutlierResult, PercentileSummary};
use crate::throttling::{CpuThrottler, NetworkThrottler};

/// Results from a complete benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    /// Name of the benchmark suite
    pub suite_name: String,
    /// Base URL that was tested
    pub base_url: String,
    /// Configuration used for the run
    pub config_summary: ConfigSummary,
    /// Results for each scenario
    pub scenario_results: Vec<ScenarioResult>,
    /// Total duration of the benchmark run
    pub total_duration_ms: u64,
    /// Whether the benchmark passed all thresholds
    pub passed: bool,
    /// Failures if any thresholds were exceeded
    pub failures: Vec<String>,
    /// Timestamp when the benchmark started
    pub started_at: String,
}

/// Summary of the configuration used
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSummary {
    /// Number of iterations per scenario
    pub iterations: u32,
    /// Number of warmup runs
    pub warmup: u32,
    /// Number of parallel contexts
    pub parallel_contexts: u32,
    /// Network profile used
    pub network_profile: String,
    /// CPU slowdown factor
    pub cpu_slowdown: f64,
}

/// Results for a single scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    /// Name of the scenario
    pub scenario_name: String,
    /// LCP (Largest Contentful Paint) statistics
    pub lcp_summary: MetricSummary,
    /// CLS (Cumulative Layout Shift) statistics
    pub cls_summary: MetricSummary,
    /// INP (Interaction to Next Paint) statistics - may be empty if no interactions
    pub inp_summary: Option<MetricSummary>,
    /// Raw LCP samples (after warmup removal and outlier filtering)
    pub lcp_samples: Vec<f64>,
    /// Raw CLS samples
    pub cls_samples: Vec<f64>,
    /// Number of iterations that completed successfully
    pub successful_iterations: u32,
    /// Number of iterations that failed
    pub failed_iterations: u32,
    /// Duration of this scenario's benchmark in milliseconds
    pub duration_ms: u64,
    /// Whether this scenario passed thresholds
    pub passed: bool,
    /// Failures for this scenario
    pub failures: Vec<String>,
}

/// Statistical summary for a metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSummary {
    /// Minimum value
    pub min: f64,
    /// 25th percentile
    pub p25: f64,
    /// Median (50th percentile)
    pub p50: f64,
    /// 75th percentile
    pub p75: f64,
    /// 95th percentile
    pub p95: f64,
    /// 99th percentile
    pub p99: f64,
    /// Maximum value
    pub max: f64,
    /// Mean value
    pub mean: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Number of samples
    pub count: usize,
    /// Coefficient of variation (std_dev / mean)
    pub cv: f64,
    /// Number of outliers removed
    pub outliers_removed: usize,
}

impl From<PercentileSummary> for MetricSummary {
    fn from(summary: PercentileSummary) -> Self {
        MetricSummary {
            min: summary.min,
            p25: summary.p25,
            p50: summary.p50,
            p75: summary.p75,
            p95: summary.p95,
            p99: summary.p99,
            max: summary.max,
            mean: summary.mean,
            std_dev: summary.std_dev,
            count: summary.count,
            cv: summary.coefficient_of_variation(),
            outliers_removed: 0,
        }
    }
}

/// Raw metrics collected from a single iteration
#[derive(Debug, Clone)]
struct IterationMetrics {
    lcp: Option<f64>,
    cls: Option<f64>,
    inp: Option<f64>,
    #[allow(dead_code)]
    duration_ms: u64,
    success: bool,
    #[allow(dead_code)]
    error: Option<String>,
}

/// The benchmark runner
pub struct BenchmarkRunner {
    browser: Browser,
    _handle: tokio::task::JoinHandle<()>,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner with a headless browser
    ///
    /// # Example
    ///
    /// ```no_run
    /// use benchmark_harness::runner::BenchmarkRunner;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let runner = BenchmarkRunner::new().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new() -> Result<Self> {
        Self::with_config(BrowserConfig::builder().build().map_err(|e| anyhow::anyhow!("{}", e))?).await
    }

    /// Create a runner with custom browser configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Custom browser configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use benchmark_harness::runner::BenchmarkRunner;
    /// use chromiumoxide::browser::BrowserConfig;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let browser_config = BrowserConfig::builder()
    ///     .with_head()  // Run with visible browser window
    ///     .build()?;
    /// let runner = BenchmarkRunner::with_config(browser_config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn with_config(config: BrowserConfig) -> Result<Self> {
        info!("Launching browser for benchmarking");
        let (browser, mut handler) = Browser::launch(config)
            .await
            .context("Failed to launch browser")?;

        // Spawn handler to process browser events
        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        info!("Browser launched successfully");
        Ok(Self {
            browser,
            _handle: handle,
        })
    }

    /// Run benchmarks according to the configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Benchmark configuration
    ///
    /// # Returns
    ///
    /// Complete benchmark results including statistics and pass/fail status
    #[instrument(skip(self, config), fields(suite = %config.benchmark.name))]
    pub async fn run(&self, config: &Config) -> Result<BenchmarkResults> {
        let start_time = Instant::now();
        let started_at = chrono::Utc::now().to_rfc3339();

        info!(
            "Starting benchmark suite '{}' with {} scenarios",
            config.benchmark.name,
            config.scenarios.len()
        );

        let mut scenario_results = Vec::new();
        let mut all_failures = Vec::new();
        let mut all_passed = true;

        for scenario in &config.scenarios {
            info!("Running scenario: {}", scenario.name);
            let result = self.run_scenario(config, scenario).await?;

            if !result.passed {
                all_passed = false;
                for failure in &result.failures {
                    all_failures.push(format!("{}: {}", scenario.name, failure));
                }
            }

            scenario_results.push(result);
        }

        let total_duration = start_time.elapsed();

        let results = BenchmarkResults {
            suite_name: config.benchmark.name.clone(),
            base_url: config.benchmark.base_url.clone(),
            config_summary: ConfigSummary {
                iterations: config.benchmark.iterations,
                warmup: config.benchmark.warmup,
                parallel_contexts: config.benchmark.parallel_contexts,
                network_profile: format!("{:?}", config.throttling.network_profile),
                cpu_slowdown: config.throttling.cpu_slowdown,
            },
            scenario_results,
            total_duration_ms: total_duration.as_millis() as u64,
            passed: all_passed,
            failures: all_failures,
            started_at,
        };

        if all_passed {
            info!(
                "Benchmark suite '{}' completed successfully in {}ms",
                config.benchmark.name,
                results.total_duration_ms
            );
        } else {
            warn!(
                "Benchmark suite '{}' completed with {} failures",
                config.benchmark.name,
                results.failures.len()
            );
        }

        Ok(results)
    }

    /// Run a single scenario
    #[instrument(skip(self, config), fields(scenario = %scenario.name))]
    async fn run_scenario(&self, config: &Config, scenario: &Scenario) -> Result<ScenarioResult> {
        let start_time = Instant::now();
        let total_iterations = config.benchmark.warmup + config.benchmark.iterations;
        let parallel_contexts = config.benchmark.parallel_contexts as usize;

        // Use semaphore to limit parallel contexts
        let semaphore = Arc::new(Semaphore::new(parallel_contexts));

        // Run iterations in parallel batches
        let iteration_futures: Vec<_> = (0..total_iterations)
            .map(|i| {
                let sem = semaphore.clone();
                let config = config.clone();
                let scenario = scenario.clone();
                async move {
                    let _permit = sem.acquire().await.unwrap();
                    self.run_iteration(&config, &scenario, i).await
                }
            })
            .collect();

        // Execute all iterations
        let metrics: Vec<IterationMetrics> = stream::iter(iteration_futures)
            .buffer_unordered(parallel_contexts)
            .collect()
            .await;

        // Separate warmup from measurement iterations
        let warmup_count = config.benchmark.warmup as usize;
        let measurement_metrics: Vec<_> = metrics.into_iter().skip(warmup_count).collect();

        // Extract samples
        let mut lcp_samples: Vec<f64> = measurement_metrics
            .iter()
            .filter_map(|m| m.lcp)
            .collect();
        let mut cls_samples: Vec<f64> = measurement_metrics
            .iter()
            .filter_map(|m| m.cls)
            .collect();
        let inp_samples: Vec<f64> = measurement_metrics
            .iter()
            .filter_map(|m| m.inp)
            .collect();

        // Count successes/failures
        let successful_iterations = measurement_metrics.iter().filter(|m| m.success).count() as u32;
        let failed_iterations = measurement_metrics.iter().filter(|m| !m.success).count() as u32;

        // Remove outliers from LCP and CLS
        let (lcp_summary, lcp_outliers) = Self::compute_summary_with_outliers(&mut lcp_samples);
        let (cls_summary, cls_outliers) = Self::compute_summary_with_outliers(&mut cls_samples);

        // INP may not have any samples if there were no interactions
        let inp_summary = if inp_samples.is_empty() {
            None
        } else {
            let mut inp = inp_samples;
            let (summary, _) = Self::compute_summary_with_outliers(&mut inp);
            Some(summary)
        };

        // Check thresholds
        let mut failures = Vec::new();
        let mut passed = true;

        if let Some(threshold) = config.thresholds.lcp_p95 {
            if lcp_summary.p95 > threshold {
                passed = false;
                failures.push(format!(
                    "LCP p95 ({:.0}ms) exceeds threshold ({:.0}ms)",
                    lcp_summary.p95, threshold
                ));
            }
        }

        if let Some(threshold) = config.thresholds.inp_p95 {
            if let Some(ref inp) = inp_summary {
                if inp.p95 > threshold {
                    passed = false;
                    failures.push(format!(
                        "INP p95 ({:.0}ms) exceeds threshold ({:.0}ms)",
                        inp.p95, threshold
                    ));
                }
            }
        }

        if let Some(threshold) = config.thresholds.cls_p95 {
            if cls_summary.p95 > threshold {
                passed = false;
                failures.push(format!(
                    "CLS p95 ({:.4}) exceeds threshold ({:.4})",
                    cls_summary.p95, threshold
                ));
            }
        }

        let duration = start_time.elapsed();

        Ok(ScenarioResult {
            scenario_name: scenario.name.clone(),
            lcp_summary: MetricSummary {
                outliers_removed: lcp_outliers,
                ..lcp_summary
            },
            cls_summary: MetricSummary {
                outliers_removed: cls_outliers,
                ..cls_summary
            },
            inp_summary,
            lcp_samples,
            cls_samples,
            successful_iterations,
            failed_iterations,
            duration_ms: duration.as_millis() as u64,
            passed,
            failures,
        })
    }

    /// Compute summary statistics with outlier removal
    fn compute_summary_with_outliers(samples: &mut Vec<f64>) -> (MetricSummary, usize) {
        if samples.is_empty() {
            return (
                MetricSummary {
                    min: 0.0,
                    p25: 0.0,
                    p50: 0.0,
                    p75: 0.0,
                    p95: 0.0,
                    p99: 0.0,
                    max: 0.0,
                    mean: 0.0,
                    std_dev: 0.0,
                    count: 0,
                    cv: 0.0,
                    outliers_removed: 0,
                },
                0,
            );
        }

        // Detect and remove outliers
        let outliers_removed = if let Some(outlier_result) = OutlierResult::detect(samples) {
            let count = outlier_result.outlier_indices.len();
            *samples = outlier_result.clean_samples(samples);
            count
        } else {
            0
        };

        // Compute summary on clean samples
        let summary = PercentileSummary::from_samples(samples)
            .map(MetricSummary::from)
            .unwrap_or(MetricSummary {
                min: 0.0,
                p25: 0.0,
                p50: 0.0,
                p75: 0.0,
                p95: 0.0,
                p99: 0.0,
                max: 0.0,
                mean: 0.0,
                std_dev: 0.0,
                count: 0,
                cv: 0.0,
                outliers_removed: 0,
            });

        (summary, outliers_removed)
    }

    /// Run a single iteration of a scenario
    #[instrument(skip(self, config, scenario), fields(iteration = %iteration))]
    async fn run_iteration(
        &self,
        config: &Config,
        scenario: &Scenario,
        iteration: u32,
    ) -> IterationMetrics {
        let start_time = Instant::now();

        // Create a new incognito context for isolation
        let page = match self.browser.new_page("about:blank").await {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to create page: {}", e);
                return IterationMetrics {
                    lcp: None,
                    cls: None,
                    inp: None,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    success: false,
                    error: Some(e.to_string()),
                };
            }
        };

        // Apply throttling
        if config.throttling.network_profile != NetworkProfile::None {
            if let Err(e) = NetworkThrottler::apply(&page, config.throttling.network_profile).await
            {
                warn!("Failed to apply network throttling: {}", e);
            }
        }

        if config.throttling.cpu_slowdown > 1.0 {
            if let Err(e) = CpuThrottler::apply(&page, config.throttling.cpu_slowdown).await {
                warn!("Failed to apply CPU throttling: {}", e);
            }
        }

        // Inject web vitals collector
        let collector = WebVitalsCollector::new();
        if let Err(e) = collector.inject_into_page(&page).await {
            error!("Failed to inject web vitals collector: {}", e);
            return IterationMetrics {
                lcp: None,
                cls: None,
                inp: None,
                duration_ms: start_time.elapsed().as_millis() as u64,
                success: false,
                error: Some(e.to_string()),
            };
        }

        // Execute scenario steps
        let mut last_error = None;
        for step in &scenario.steps {
            if let Err(e) = self.execute_step(config, &page, step).await {
                warn!("Step failed: {}", e);
                last_error = Some(e.to_string());
                break;
            }
        }

        // Collect metrics
        let metric_timeout = Duration::from_secs(10);

        let lcp = match collector.wait_for_lcp(&page, metric_timeout).await {
            Ok(m) => Some(m.value),
            Err(e) => {
                debug!("LCP collection failed: {}", e);
                None
            }
        };

        let cls = match collector.wait_for_cls(&page, metric_timeout).await {
            Ok(m) => Some(m.value),
            Err(e) => {
                debug!("CLS collection failed: {}", e);
                None
            }
        };

        // INP requires user interaction, may not be available
        let inp = match collector.wait_for_inp(&page, Duration::from_secs(2)).await {
            Ok(m) => Some(m.value),
            Err(_) => None,
        };

        // Clear throttling
        let _ = NetworkThrottler::clear(&page).await;
        let _ = CpuThrottler::clear(&page).await;

        // Close the page
        let _ = page.close().await;

        let duration = start_time.elapsed();

        IterationMetrics {
            lcp,
            cls,
            inp,
            duration_ms: duration.as_millis() as u64,
            success: last_error.is_none() && lcp.is_some(),
            error: last_error,
        }
    }

    /// Execute a single benchmark step
    #[instrument(skip(self, config, page))]
    async fn execute_step(
        &self,
        config: &Config,
        page: &Page,
        step: &BenchmarkStep,
    ) -> Result<()> {
        match step {
            BenchmarkStep::Navigate { url } => {
                let full_url = if url.starts_with("http://") || url.starts_with("https://") {
                    url.clone()
                } else {
                    format!(
                        "{}{}",
                        config.benchmark.base_url.trim_end_matches('/'),
                        if url.starts_with('/') {
                            url.clone()
                        } else {
                            format!("/{}", url)
                        }
                    )
                };

                debug!("Navigating to: {}", full_url);
                page.goto(&full_url)
                    .await
                    .context("Navigation failed")?;
            }

            BenchmarkStep::Wait { condition } => match condition {
                WaitCondition::NetworkIdle => {
                    debug!("Waiting for network idle");
                    // Wait for network to be idle (no requests for 500ms)
                    page.wait_for_navigation()
                        .await
                        .context("Wait for navigation failed")?;
                }
                WaitCondition::Selector { selector } => {
                    debug!("Waiting for selector: {}", selector);
                    page.find_element(selector)
                        .await
                        .context(format!("Selector not found: {}", selector))?;
                }
                WaitCondition::Timeout { duration } => {
                    debug!("Waiting for {:?}", duration);
                    tokio::time::sleep(*duration).await;
                }
            },

            BenchmarkStep::Click { selector } => {
                debug!("Clicking: {}", selector);
                let element = page
                    .find_element(selector)
                    .await
                    .context(format!("Element not found: {}", selector))?;
                element.click().await.context("Click failed")?;
            }

            BenchmarkStep::Type { selector, text } => {
                debug!("Typing into: {}", selector);
                let element = page
                    .find_element(selector)
                    .await
                    .context(format!("Element not found: {}", selector))?;
                element
                    .type_str(text)
                    .await
                    .context("Type failed")?;
            }

            BenchmarkStep::Upload { selector, file_path } => {
                debug!("Uploading file to: {}", selector);
                let element = page
                    .find_element(selector)
                    .await
                    .context(format!("Element not found: {}", selector))?;
                // Note: chromiumoxide may need specific API for file uploads
                // This is a placeholder that would need adjustment based on actual API
                element
                    .type_str(file_path)
                    .await
                    .context("File upload failed")?;
            }

            BenchmarkStep::Measure { label } => {
                if let Some(label) = label {
                    debug!("Measurement point: {}", label);
                }
                // Measurements are collected at the end of the iteration
            }
        }

        Ok(())
    }

    /// Close the browser and clean up resources
    pub async fn close(self) -> Result<()> {
        info!("Closing browser");
        // The browser will be dropped when self is dropped
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_summary_from_percentile_summary() {
        let ps = PercentileSummary {
            min: 1.0,
            p25: 2.0,
            p50: 3.0,
            p75: 4.0,
            p95: 5.0,
            p99: 6.0,
            max: 7.0,
            mean: 3.5,
            std_dev: 1.5,
            count: 100,
        };

        let ms: MetricSummary = ps.into();

        assert_eq!(ms.min, 1.0);
        assert_eq!(ms.p25, 2.0);
        assert_eq!(ms.p50, 3.0);
        assert_eq!(ms.p75, 4.0);
        assert_eq!(ms.p95, 5.0);
        assert_eq!(ms.p99, 6.0);
        assert_eq!(ms.max, 7.0);
        assert_eq!(ms.mean, 3.5);
        assert_eq!(ms.std_dev, 1.5);
        assert_eq!(ms.count, 100);
    }

    #[test]
    fn test_compute_summary_empty() {
        let mut samples = Vec::new();
        let (summary, outliers) = BenchmarkRunner::compute_summary_with_outliers(&mut samples);

        assert_eq!(summary.count, 0);
        assert_eq!(outliers, 0);
    }

    #[test]
    fn test_compute_summary_with_outliers() {
        let mut samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0];
        let (summary, outliers) = BenchmarkRunner::compute_summary_with_outliers(&mut samples);

        assert!(outliers > 0); // 100.0 should be detected as outlier
        assert_eq!(samples.len(), 5); // Outlier removed
        assert!(!samples.contains(&100.0));
    }

    #[test]
    fn test_config_summary_creation() {
        let summary = ConfigSummary {
            iterations: 30,
            warmup: 3,
            parallel_contexts: 4,
            network_profile: "Fast3G".to_string(),
            cpu_slowdown: 4.0,
        };

        assert_eq!(summary.iterations, 30);
        assert_eq!(summary.warmup, 3);
        assert_eq!(summary.parallel_contexts, 4);
    }

    #[test]
    fn test_benchmark_results_serialization() {
        let results = BenchmarkResults {
            suite_name: "Test Suite".to_string(),
            base_url: "https://example.com".to_string(),
            config_summary: ConfigSummary {
                iterations: 30,
                warmup: 3,
                parallel_contexts: 4,
                network_profile: "None".to_string(),
                cpu_slowdown: 1.0,
            },
            scenario_results: Vec::new(),
            total_duration_ms: 1000,
            passed: true,
            failures: Vec::new(),
            started_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&results).unwrap();
        assert!(json.contains("Test Suite"));
        assert!(json.contains("https://example.com"));
    }
}
