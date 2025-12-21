//! Core Web Vitals metrics collection via chromiumoxide
//!
//! This module provides the [`WebVitalsCollector`] which injects the web-vitals library
//! into browser pages and collects LCP, CLS, and INP metrics via console events.
//!
//! # How it works
//!
//! 1. The collector injects JavaScript that includes a simplified web-vitals implementation
//! 2. Metrics are reported via `console.log()` with the prefix `__BENCHMARK_METRIC__:`
//! 3. The collector listens for `Runtime.consoleAPICalled` events from the Chrome DevTools Protocol
//! 4. Metrics are parsed from JSON payloads and returned as [`WebVitalMetric`] structs
//!
//! # Example
//!
//! ```ignore
//! use benchmark_harness::metrics::web_vitals::WebVitalsCollector;
//! use chromiumoxide::Page;
//! use std::time::Duration;
//!
//! async fn example(page: &Page) -> anyhow::Result<()> {
//!     let collector = WebVitalsCollector::new();
//!     collector.inject_into_page(&page).await?;
//!
//!     page.goto("https://example.com").await?;
//!
//!     let lcp = collector.wait_for_lcp(&page, Duration::from_secs(30)).await?;
//!     println!("LCP: {}ms (delta: {}ms, id: {})", lcp.value, lcp.delta, lcp.id);
//!     Ok(())
//! }
//! ```

use anyhow::{Context, Result};
use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;
use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
use chromiumoxide::Page;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, error, instrument, trace, warn};

/// A Core Web Vital metric measurement
///
/// This struct captures a single metric value along with metadata about when
/// and how it was captured.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebVitalMetric {
    /// The metric name: "LCP", "CLS", or "INP"
    pub name: String,
    /// The raw metric value (milliseconds for LCP/INP, unitless score for CLS)
    pub value: f64,
    /// Change in value since the last report (for metrics that update over time)
    pub delta: f64,
    /// Unique identifier for this metric instance/interaction
    pub id: String,
    /// Unix timestamp in milliseconds when the metric was captured
    pub timestamp: u64,
}

/// Collected metrics from a page
#[derive(Debug, Clone, Default)]
pub struct CollectedMetrics {
    /// LCP value in milliseconds
    pub lcp: Option<f64>,
    /// CLS value (unitless)
    pub cls: Option<f64>,
    /// INP value in milliseconds
    pub inp: Option<f64>,
}

/// Handle to a running metrics collection task
pub struct MetricsHandle {
    metrics: Arc<Mutex<CollectedMetrics>>,
    _task: tokio::task::JoinHandle<()>,
}

impl MetricsHandle {
    /// Collect the metrics that have been captured so far
    pub async fn collect(self) -> CollectedMetrics {
        // Abort the listener task
        self._task.abort();
        // Return the collected metrics
        self.metrics.lock().await.clone()
    }
}

/// Web Vitals metrics collector
///
/// Injects the web-vitals library into browser pages and collects Core Web Vitals
/// metrics (LCP, CLS, INP) via the Chrome DevTools Protocol.
#[derive(Debug, Clone)]
pub struct WebVitalsCollector {
    // Internal state can be extended as needed
    _private: (),
}

impl WebVitalsCollector {
    /// Create a new Web Vitals collector
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Start collecting metrics in the background
    ///
    /// This spawns a task that listens for console events and accumulates metrics.
    /// Call this BEFORE navigating to ensure metrics logged during page load are captured.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to collect metrics from
    ///
    /// # Returns
    ///
    /// A handle that can be used to collect the accumulated metrics
    pub async fn start_collecting(&self, page: &Page) -> Result<MetricsHandle> {
        debug!("Starting background metrics collection");

        let metrics = Arc::new(Mutex::new(CollectedMetrics::default()));
        let metrics_clone = metrics.clone();

        // Subscribe to console API events
        let mut events = page
            .event_listener::<EventConsoleApiCalled>()
            .await
            .context("Failed to subscribe to console events")?;

        // Spawn a task to listen for console events
        let task = tokio::spawn(async move {
            while let Some(event) = events.next().await {
                // Check if this is a metric log
                if let Some(first_arg) = event.args.first() {
                    if let Some(value) = first_arg.value.as_ref() {
                        if let Some(message) = value.as_str() {
                            if message.starts_with("__BENCHMARK_METRIC__:") {
                                let json_start = "__BENCHMARK_METRIC__:".len();
                                let json_str = &message[json_start..];

                                if let Ok(metric) = serde_json::from_str::<WebVitalMetric>(json_str)
                                {
                                    let mut m = metrics_clone.lock().await;
                                    match metric.name.as_str() {
                                        "LCP" => {
                                            debug!("Captured LCP: {}ms", metric.value);
                                            m.lcp = Some(metric.value);
                                        }
                                        "CLS" => {
                                            debug!("Captured CLS: {}", metric.value);
                                            m.cls = Some(metric.value);
                                        }
                                        "INP" => {
                                            debug!("Captured INP: {}ms", metric.value);
                                            m.inp = Some(metric.value);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(MetricsHandle {
            metrics,
            _task: task,
        })
    }

    /// Inject the web-vitals library into a page
    ///
    /// This must be called before navigating to ensure the metrics are collected
    /// from page load. The script is injected using `addScriptToEvaluateOnNewDocument`
    /// which ensures it runs before any page scripts.
    ///
    /// # Arguments
    ///
    /// * `page` - The chromiumoxide Page to inject into
    ///
    /// # Errors
    ///
    /// Returns an error if the script injection fails
    #[instrument(skip(self, page))]
    pub async fn inject_into_page(&self, page: &Page) -> Result<()> {
        debug!("Injecting web-vitals script into page");

        let script = Self::web_vitals_script();

        // Inject the script to run on every new document
        let params = AddScriptToEvaluateOnNewDocumentParams::new(script);
        page.execute(params)
            .await
            .context("Failed to inject web-vitals script")?;

        debug!("Web-vitals script injected successfully");
        Ok(())
    }

    /// Wait for the LCP (Largest Contentful Paint) metric
    ///
    /// This method listens for console events and waits for an LCP metric to be reported.
    /// LCP measures the time from page load start to when the largest content element
    /// becomes visible in the viewport.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to collect metrics from
    /// * `timeout_duration` - Maximum time to wait for the metric
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The timeout is exceeded
    /// - The metric cannot be parsed
    /// - The console event listener fails
    #[instrument(skip(self, page))]
    pub async fn wait_for_lcp(
        &self,
        page: &Page,
        timeout_duration: Duration,
    ) -> Result<WebVitalMetric> {
        self.wait_for_metric(page, "LCP", timeout_duration).await
    }

    /// Wait for the CLS (Cumulative Layout Shift) metric
    ///
    /// This method listens for console events and waits for a CLS metric to be reported.
    /// CLS measures visual stability by tracking unexpected layout shifts.
    ///
    /// Note: CLS is measured throughout the page lifecycle. This returns the first
    /// reported value, but CLS may continue to update as the user interacts with the page.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to collect metrics from
    /// * `timeout_duration` - Maximum time to wait for the metric
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The timeout is exceeded
    /// - The metric cannot be parsed
    /// - The console event listener fails
    #[instrument(skip(self, page))]
    pub async fn wait_for_cls(
        &self,
        page: &Page,
        timeout_duration: Duration,
    ) -> Result<WebVitalMetric> {
        self.wait_for_metric(page, "CLS", timeout_duration).await
    }

    /// Wait for the INP (Interaction to Next Paint) metric
    ///
    /// This method listens for console events and waits for an INP metric to be reported.
    /// INP measures responsiveness by tracking the latency of user interactions.
    ///
    /// Note: INP requires user interaction to be measured. If no interactions occur,
    /// this will timeout.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to collect metrics from
    /// * `timeout_duration` - Maximum time to wait for the metric
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The timeout is exceeded
    /// - The metric cannot be parsed
    /// - The console event listener fails
    #[instrument(skip(self, page))]
    pub async fn wait_for_inp(
        &self,
        page: &Page,
        timeout_duration: Duration,
    ) -> Result<WebVitalMetric> {
        self.wait_for_metric(page, "INP", timeout_duration).await
    }

    /// Internal method to wait for a specific metric type
    #[instrument(skip(self, page))]
    async fn wait_for_metric(
        &self,
        page: &Page,
        metric_name: &str,
        timeout_duration: Duration,
    ) -> Result<WebVitalMetric> {
        debug!("Waiting for {} metric (timeout: {:?})", metric_name, timeout_duration);

        let metric_name = metric_name.to_string();
        let metric_name_clone = metric_name.clone();

        // Create a shared result that can be updated from the event stream
        let result: Arc<Mutex<Option<WebVitalMetric>>> = Arc::new(Mutex::new(None));
        let result_clone = result.clone();

        // Subscribe to console API events
        let mut events = page
            .event_listener::<EventConsoleApiCalled>()
            .await
            .context("Failed to subscribe to console events")?;

        // Spawn a task to listen for console events
        let listener = tokio::spawn(async move {
            while let Some(event) = events.next().await {
                if let Err(e) = Self::handle_console_event(&event, &metric_name_clone, &result_clone).await {
                    warn!("Error handling console event: {}", e);
                }

                // Check if we found the metric
                let found = result_clone.lock().await.is_some();
                if found {
                    break;
                }
            }
        });

        // Wait for the metric with timeout
        let timeout_result = timeout(timeout_duration, async {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let metric = result.lock().await.clone();
                if let Some(m) = metric {
                    return Ok(m);
                }
            }
        })
        .await;

        // Clean up the listener
        listener.abort();

        match timeout_result {
            Ok(Ok(metric)) => {
                debug!("Successfully captured {} metric: {:?}", metric_name, metric);
                Ok(metric)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                error!("Timeout waiting for {} metric after {:?}", metric_name, timeout_duration);
                anyhow::bail!("Timeout waiting for {} metric after {:?}", metric_name, timeout_duration)
            }
        }
    }

    /// Handle a console API event and extract metrics if present
    async fn handle_console_event(
        event: &EventConsoleApiCalled,
        metric_name: &str,
        result: &Arc<Mutex<Option<WebVitalMetric>>>,
    ) -> Result<()> {
        // Check if this is a metric log
        if let Some(first_arg) = event.args.first() {
            if let Some(value) = first_arg.value.as_ref() {
                if let Some(message) = value.as_str() {
                    if message.starts_with("__BENCHMARK_METRIC__:") {
                        trace!("Found benchmark metric message: {}", message);

                        // Extract JSON payload
                        let json_start = "__BENCHMARK_METRIC__:".len();
                        let json_str = &message[json_start..];

                        // Parse the metric
                        match serde_json::from_str::<WebVitalMetric>(json_str) {
                            Ok(metric) if metric.name == metric_name => {
                                debug!("Parsed {} metric: value={}, delta={}", metric_name, metric.value, metric.delta);
                                *result.lock().await = Some(metric);
                            }
                            Ok(metric) => {
                                trace!("Ignoring {} metric (waiting for {})", metric.name, metric_name);
                            }
                            Err(e) => {
                                warn!("Failed to parse metric JSON: {} - Error: {}", json_str, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the web-vitals JavaScript code to inject
    ///
    /// This is a simplified implementation of the web-vitals library that reports
    /// LCP, CLS, and INP via console.log with a special prefix.
    fn web_vitals_script() -> String {
        r#"
(function() {
    'use strict';

    // Prefix for all metric reports
    const METRIC_PREFIX = '__BENCHMARK_METRIC__:';

    // Report a metric via console.log
    function reportMetric(metric) {
        const payload = {
            name: metric.name,
            value: metric.value,
            delta: metric.delta || metric.value,
            id: metric.id || crypto.randomUUID(),
            timestamp: Date.now()
        };
        console.log(METRIC_PREFIX + JSON.stringify(payload));
    }

    // LCP Observer
    try {
        const lcpObserver = new PerformanceObserver((list) => {
            const entries = list.getEntries();
            const lastEntry = entries[entries.length - 1];
            reportMetric({
                name: 'LCP',
                value: lastEntry.renderTime || lastEntry.loadTime,
                id: lastEntry.id || crypto.randomUUID()
            });
        });
        lcpObserver.observe({ type: 'largest-contentful-paint', buffered: true });
    } catch (e) {
        console.warn('LCP observer not supported:', e);
    }

    // CLS Observer
    try {
        let clsValue = 0;
        let clsId = crypto.randomUUID();
        const clsObserver = new PerformanceObserver((list) => {
            for (const entry of list.getEntries()) {
                if (!entry.hadRecentInput) {
                    clsValue += entry.value;
                    reportMetric({
                        name: 'CLS',
                        value: clsValue,
                        delta: entry.value,
                        id: clsId
                    });
                }
            }
        });
        clsObserver.observe({ type: 'layout-shift', buffered: true });
    } catch (e) {
        console.warn('CLS observer not supported:', e);
    }

    // INP Observer (Interaction to Next Paint)
    try {
        const inpMap = new Map();
        const inpObserver = new PerformanceObserver((list) => {
            for (const entry of list.getEntries()) {
                // Track the interaction
                const interactionId = entry.interactionId;
                if (interactionId) {
                    const existing = inpMap.get(interactionId) || { duration: 0, entries: [] };
                    existing.duration = Math.max(existing.duration, entry.duration);
                    existing.entries.push(entry);
                    inpMap.set(interactionId, existing);

                    // Report the interaction
                    reportMetric({
                        name: 'INP',
                        value: existing.duration,
                        delta: entry.duration,
                        id: String(interactionId)
                    });
                }
            }
        });
        inpObserver.observe({ type: 'event', buffered: true, durationThreshold: 16 });
    } catch (e) {
        // Fallback for browsers without Event Timing API
        try {
            const firstInputObserver = new PerformanceObserver((list) => {
                for (const entry of list.getEntries()) {
                    reportMetric({
                        name: 'INP',
                        value: entry.processingStart - entry.startTime,
                        id: crypto.randomUUID()
                    });
                }
            });
            firstInputObserver.observe({ type: 'first-input', buffered: true });
        } catch (e2) {
            console.warn('INP/FID observer not supported:', e2);
        }
    }

    // Report final metrics before page unload
    addEventListener('visibilitychange', () => {
        if (document.visibilityState === 'hidden') {
            // Final CLS report
            if (typeof clsValue !== 'undefined') {
                reportMetric({
                    name: 'CLS',
                    value: clsValue,
                    delta: 0,
                    id: clsId
                });
            }
        }
    });
})();
"#.to_string()
    }
}

impl Default for WebVitalsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_vitals_script_not_empty() {
        let script = WebVitalsCollector::web_vitals_script();
        assert!(!script.is_empty());
        assert!(script.contains("__BENCHMARK_METRIC__:"));
        assert!(script.contains("LCP"));
        assert!(script.contains("CLS"));
        assert!(script.contains("INP"));
    }

    #[test]
    fn test_metric_serialization() {
        let metric = WebVitalMetric {
            name: "LCP".to_string(),
            value: 1234.5,
            delta: 100.0,
            id: "test-id".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&metric).unwrap();
        let parsed: WebVitalMetric = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "LCP");
        assert_eq!(parsed.value, 1234.5);
        assert_eq!(parsed.delta, 100.0);
        assert_eq!(parsed.id, "test-id");
        assert_eq!(parsed.timestamp, 1234567890);
    }

    #[test]
    fn test_metric_deserialization_from_js() {
        let json = r#"{"name":"LCP","value":2500.5,"delta":2500.5,"id":"abc123","timestamp":1700000000}"#;
        let metric: WebVitalMetric = serde_json::from_str(json).unwrap();

        assert_eq!(metric.name, "LCP");
        assert_eq!(metric.value, 2500.5);
        assert_eq!(metric.delta, 2500.5);
        assert_eq!(metric.id, "abc123");
        assert_eq!(metric.timestamp, 1700000000);
    }
}
