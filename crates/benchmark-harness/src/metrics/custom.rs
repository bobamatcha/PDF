//! Custom timing measurements via the User Timing API
//!
//! This module provides the [`CustomTimingCollector`] which bridges to the browser's
//! User Timing API (performance.mark() and performance.measure()) for custom timing
//! measurements within benchmark scenarios.
//!
//! # How it works
//!
//! 1. Use [`mark_start`](CustomTimingCollector::mark_start) to create a performance mark at the beginning of an operation
//! 2. Use [`mark_end`](CustomTimingCollector::mark_end) to create a mark at the end
//! 3. Use [`measure`](CustomTimingCollector::measure) to calculate the duration between two marks
//!
//! # Example
//!
//! ```no_run
//! use benchmark_harness::metrics::custom::CustomTimingCollector;
//! use chromiumoxide::Browser;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let browser = Browser::default().await?;
//! let page = browser.new_page("https://example.com").await?;
//!
//! let collector = CustomTimingCollector::new();
//!
//! // Mark the start of an operation
//! collector.mark_start(&page, "button-click").await?;
//!
//! // Perform the operation
//! page.click("button#submit").await?;
//!
//! // Mark the end
//! collector.mark_end(&page, "button-click").await?;
//!
//! // Measure the duration
//! let measurement = collector.measure(
//!     &page,
//!     "button-click-duration",
//!     "button-click-start",
//!     "button-click-end"
//! ).await?;
//!
//! println!("Button click took {}ms", measurement.duration);
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use chromiumoxide::Page;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

/// A custom timing measurement from the User Timing API
///
/// Represents a measurement created via `performance.measure()`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimingMeasurement {
    /// Name of the measurement
    pub name: String,
    /// Duration in milliseconds
    pub duration: f64,
    /// Start time relative to navigation start (milliseconds)
    pub start_time: f64,
    /// Name of the start mark (if any)
    pub start_mark: Option<String>,
    /// Name of the end mark (if any)
    pub end_mark: Option<String>,
}

/// Custom timing collector for User Timing API measurements
///
/// This collector provides methods to create performance marks and measures
/// using the browser's Performance API. Useful for tracking custom operations
/// within benchmark scenarios.
#[derive(Debug, Clone)]
pub struct CustomTimingCollector {
    _private: (),
}

impl CustomTimingCollector {
    /// Create a new custom timing collector
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Create a performance mark at the start of an operation
    ///
    /// This is equivalent to calling `performance.mark(name + '-start')` in the browser.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to create the mark in
    /// * `name` - Base name for the operation (will append '-start')
    ///
    /// # Errors
    ///
    /// Returns an error if the mark cannot be created
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use benchmark_harness::metrics::custom::CustomTimingCollector;
    /// # use chromiumoxide::Page;
    /// # async fn example(page: &Page) -> anyhow::Result<()> {
    /// let collector = CustomTimingCollector::new();
    /// collector.mark_start(page, "render-component").await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, page))]
    pub async fn mark_start(&self, page: &Page, name: &str) -> Result<()> {
        let mark_name = format!("{}-start", name);
        debug!("Creating start mark: {}", mark_name);

        self.create_mark(page, &mark_name).await?;
        Ok(())
    }

    /// Create a performance mark at the end of an operation
    ///
    /// This is equivalent to calling `performance.mark(name + '-end')` in the browser.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to create the mark in
    /// * `name` - Base name for the operation (will append '-end')
    ///
    /// # Errors
    ///
    /// Returns an error if the mark cannot be created
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use benchmark_harness::metrics::custom::CustomTimingCollector;
    /// # use chromiumoxide::Page;
    /// # async fn example(page: &Page) -> anyhow::Result<()> {
    /// let collector = CustomTimingCollector::new();
    /// collector.mark_end(page, "render-component").await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, page))]
    pub async fn mark_end(&self, page: &Page, name: &str) -> Result<()> {
        let mark_name = format!("{}-end", name);
        debug!("Creating end mark: {}", mark_name);

        self.create_mark(page, &mark_name).await?;
        Ok(())
    }

    /// Create a named performance mark
    ///
    /// This is equivalent to calling `performance.mark(name)` in the browser.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to create the mark in
    /// * `name` - Name of the mark
    ///
    /// # Errors
    ///
    /// Returns an error if the mark cannot be created
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use benchmark_harness::metrics::custom::CustomTimingCollector;
    /// # use chromiumoxide::Page;
    /// # async fn example(page: &Page) -> anyhow::Result<()> {
    /// let collector = CustomTimingCollector::new();
    /// collector.create_mark(page, "custom-event").await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, page))]
    pub async fn create_mark(&self, page: &Page, name: &str) -> Result<()> {
        let script = format!("performance.mark('{}');", Self::escape_js_string(name));

        page.evaluate(script)
            .await
            .context("Failed to create performance mark")?;

        debug!("Performance mark '{}' created", name);
        Ok(())
    }

    /// Measure the duration between two marks
    ///
    /// This is equivalent to calling `performance.measure(name, startMark, endMark)` in the browser.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to create the measurement in
    /// * `name` - Name for this measurement
    /// * `start_mark` - Name of the start mark
    /// * `end_mark` - Name of the end mark
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Either mark doesn't exist
    /// - The measurement cannot be created
    /// - The measurement data cannot be retrieved
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use benchmark_harness::metrics::custom::CustomTimingCollector;
    /// # use chromiumoxide::Page;
    /// # async fn example(page: &Page) -> anyhow::Result<()> {
    /// let collector = CustomTimingCollector::new();
    ///
    /// collector.create_mark(page, "start").await?;
    /// // ... perform operation ...
    /// collector.create_mark(page, "end").await?;
    ///
    /// let measurement = collector.measure(page, "operation-time", "start", "end").await?;
    /// println!("Duration: {}ms", measurement.duration);
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, page))]
    pub async fn measure(
        &self,
        page: &Page,
        name: &str,
        start_mark: &str,
        end_mark: &str,
    ) -> Result<TimingMeasurement> {
        debug!(
            "Creating measurement '{}' from '{}' to '{}'",
            name, start_mark, end_mark
        );

        // Create the measure and retrieve its data
        let script = format!(
            r#"
            (function() {{
                performance.measure('{}', '{}', '{}');
                const entries = performance.getEntriesByName('{}', 'measure');
                if (entries.length === 0) {{
                    throw new Error('Measurement not found: {}');
                }}
                const entry = entries[entries.length - 1];
                return {{
                    name: entry.name,
                    duration: entry.duration,
                    startTime: entry.startTime
                }};
            }})()
            "#,
            Self::escape_js_string(name),
            Self::escape_js_string(start_mark),
            Self::escape_js_string(end_mark),
            Self::escape_js_string(name),
            Self::escape_js_string(name),
        );

        let result = page
            .evaluate(script)
            .await
            .context("Failed to create or retrieve measurement")?;

        // Parse the result
        let value: serde_json::Value = result
            .into_value()
            .context("Failed to get measurement value")?;

        let measurement = TimingMeasurement {
            name: value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_string(),
            duration: value
                .get("duration")
                .and_then(|v| v.as_f64())
                .context("Missing or invalid duration")?,
            start_time: value
                .get("startTime")
                .and_then(|v| v.as_f64())
                .context("Missing or invalid startTime")?,
            start_mark: Some(start_mark.to_string()),
            end_mark: Some(end_mark.to_string()),
        };

        debug!(
            "Measurement '{}' created: {}ms",
            measurement.name, measurement.duration
        );

        Ok(measurement)
    }

    /// Measure the duration from a mark to the current time
    ///
    /// This is equivalent to calling `performance.measure(name, startMark)` in the browser,
    /// which measures from the start mark to now.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to create the measurement in
    /// * `name` - Name for this measurement
    /// * `start_mark` - Name of the start mark
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The start mark doesn't exist
    /// - The measurement cannot be created
    /// - The measurement data cannot be retrieved
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use benchmark_harness::metrics::custom::CustomTimingCollector;
    /// # use chromiumoxide::Page;
    /// # async fn example(page: &Page) -> anyhow::Result<()> {
    /// let collector = CustomTimingCollector::new();
    ///
    /// collector.create_mark(page, "start").await?;
    /// // ... perform operation ...
    ///
    /// let measurement = collector.measure_from(page, "time-elapsed", "start").await?;
    /// println!("Time elapsed: {}ms", measurement.duration);
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, page))]
    pub async fn measure_from(
        &self,
        page: &Page,
        name: &str,
        start_mark: &str,
    ) -> Result<TimingMeasurement> {
        debug!("Creating measurement '{}' from '{}'", name, start_mark);

        let script = format!(
            r#"
            (function() {{
                performance.measure('{}', '{}');
                const entries = performance.getEntriesByName('{}', 'measure');
                if (entries.length === 0) {{
                    throw new Error('Measurement not found: {}');
                }}
                const entry = entries[entries.length - 1];
                return {{
                    name: entry.name,
                    duration: entry.duration,
                    startTime: entry.startTime
                }};
            }})()
            "#,
            Self::escape_js_string(name),
            Self::escape_js_string(start_mark),
            Self::escape_js_string(name),
            Self::escape_js_string(name),
        );

        let result = page
            .evaluate(script)
            .await
            .context("Failed to create or retrieve measurement")?;

        let value: serde_json::Value = result
            .into_value()
            .context("Failed to get measurement value")?;

        let measurement = TimingMeasurement {
            name: value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_string(),
            duration: value
                .get("duration")
                .and_then(|v| v.as_f64())
                .context("Missing or invalid duration")?,
            start_time: value
                .get("startTime")
                .and_then(|v| v.as_f64())
                .context("Missing or invalid startTime")?,
            start_mark: Some(start_mark.to_string()),
            end_mark: None,
        };

        debug!(
            "Measurement '{}' created: {}ms",
            measurement.name, measurement.duration
        );

        Ok(measurement)
    }

    /// Clear all performance marks and measures
    ///
    /// This is equivalent to calling `performance.clearMarks()` and `performance.clearMeasures()`
    /// in the browser.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to clear marks and measures from
    ///
    /// # Errors
    ///
    /// Returns an error if the clear operation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use benchmark_harness::metrics::custom::CustomTimingCollector;
    /// # use chromiumoxide::Page;
    /// # async fn example(page: &Page) -> anyhow::Result<()> {
    /// let collector = CustomTimingCollector::new();
    /// collector.clear_all(page).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, page))]
    pub async fn clear_all(&self, page: &Page) -> Result<()> {
        debug!("Clearing all performance marks and measures");

        let script = r#"
            performance.clearMarks();
            performance.clearMeasures();
        "#;

        page.evaluate(script)
            .await
            .context("Failed to clear performance marks and measures")?;

        debug!("All performance marks and measures cleared");
        Ok(())
    }

    /// Escape a string for safe use in JavaScript code
    fn escape_js_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    }
}

impl Default for CustomTimingCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_js_string() {
        assert_eq!(
            CustomTimingCollector::escape_js_string("simple"),
            "simple"
        );
        assert_eq!(
            CustomTimingCollector::escape_js_string("with'quote"),
            "with\\'quote"
        );
        assert_eq!(
            CustomTimingCollector::escape_js_string("with\"doublequote"),
            "with\\\"doublequote"
        );
        assert_eq!(
            CustomTimingCollector::escape_js_string("with\\backslash"),
            "with\\\\backslash"
        );
        assert_eq!(
            CustomTimingCollector::escape_js_string("with\nnewline"),
            "with\\nnewline"
        );
    }

    #[test]
    fn test_timing_measurement_serialization() {
        let measurement = TimingMeasurement {
            name: "test-measure".to_string(),
            duration: 123.45,
            start_time: 1000.0,
            start_mark: Some("start".to_string()),
            end_mark: Some("end".to_string()),
        };

        let json = serde_json::to_string(&measurement).unwrap();
        let parsed: TimingMeasurement = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "test-measure");
        assert_eq!(parsed.duration, 123.45);
        assert_eq!(parsed.start_time, 1000.0);
        assert_eq!(parsed.start_mark, Some("start".to_string()));
        assert_eq!(parsed.end_mark, Some("end".to_string()));
    }

    #[test]
    fn test_timing_measurement_without_marks() {
        let measurement = TimingMeasurement {
            name: "test-measure".to_string(),
            duration: 123.45,
            start_time: 1000.0,
            start_mark: None,
            end_mark: None,
        };

        let json = serde_json::to_string(&measurement).unwrap();
        let parsed: TimingMeasurement = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.start_mark, None);
        assert_eq!(parsed.end_mark, None);
    }
}
