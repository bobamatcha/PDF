//! Benchmark result reporting
//!
//! This module handles formatting and outputting benchmark results in various formats
//! including JSON, human-readable text, and HTML reports.
//!
//! # Output Formats
//!
//! - **JSON**: Machine-readable format for CI/CD integration
//! - **Console**: Human-readable format with colors and tables
//! - **Markdown**: Documentation-friendly format for reports
//!
//! # Example
//!
//! ```no_run
//! use benchmark_harness::reporter::{Reporter, OutputFormat};
//! use benchmark_harness::runner::BenchmarkResults;
//!
//! # fn example(results: BenchmarkResults) -> anyhow::Result<()> {
//! let reporter = Reporter::new(OutputFormat::Console);
//! reporter.report(&results)?;
//!
//! // Or write to a file
//! Reporter::new(OutputFormat::Json)
//!     .write_to_file(&results, "results.json")?;
//! # Ok(())
//! # }
//! ```

mod console;
mod json;
mod markdown;

use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::runner::BenchmarkResults;

pub use console::ConsoleReporter;
pub use json::JsonReporter;
pub use markdown::MarkdownReporter;

/// Output format for benchmark results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// JSON format for machine parsing
    Json,
    /// Pretty-printed JSON
    JsonPretty,
    /// Console output with colors and formatting
    Console,
    /// Markdown format for documentation
    Markdown,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Console
    }
}

/// Reporter for benchmark results
pub struct Reporter {
    format: OutputFormat,
}

impl Reporter {
    /// Create a new reporter with the specified output format
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    /// Report results to stdout
    pub fn report(&self, results: &BenchmarkResults) -> Result<()> {
        let output = self.format_results(results)?;
        print!("{}", output);
        io::stdout().flush()?;
        Ok(())
    }

    /// Write results to a file
    pub fn write_to_file<P: AsRef<Path>>(
        &self,
        results: &BenchmarkResults,
        path: P,
    ) -> Result<()> {
        let output = self.format_results(results)?;
        fs::write(path, output)?;
        Ok(())
    }

    /// Format results as a string
    pub fn format_results(&self, results: &BenchmarkResults) -> Result<String> {
        match self.format {
            OutputFormat::Json => JsonReporter::format(results, false),
            OutputFormat::JsonPretty => JsonReporter::format(results, true),
            OutputFormat::Console => ConsoleReporter::format(results),
            OutputFormat::Markdown => MarkdownReporter::format(results),
        }
    }
}

impl Default for Reporter {
    fn default() -> Self {
        Self::new(OutputFormat::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{ConfigSummary, MetricSummary, ScenarioResult};

    fn create_test_results() -> BenchmarkResults {
        BenchmarkResults {
            suite_name: "Test Suite".to_string(),
            base_url: "https://example.com".to_string(),
            config_summary: ConfigSummary {
                iterations: 30,
                warmup: 3,
                parallel_contexts: 4,
                network_profile: "None".to_string(),
                cpu_slowdown: 1.0,
            },
            scenario_results: vec![ScenarioResult {
                scenario_name: "Homepage".to_string(),
                lcp_summary: MetricSummary {
                    min: 100.0,
                    p25: 150.0,
                    p50: 200.0,
                    p75: 250.0,
                    p95: 300.0,
                    p99: 350.0,
                    max: 400.0,
                    mean: 210.0,
                    std_dev: 50.0,
                    count: 30,
                    cv: 0.24,
                    outliers_removed: 2,
                },
                cls_summary: MetricSummary {
                    min: 0.0,
                    p25: 0.01,
                    p50: 0.02,
                    p75: 0.03,
                    p95: 0.05,
                    p99: 0.08,
                    max: 0.1,
                    mean: 0.025,
                    std_dev: 0.02,
                    count: 30,
                    cv: 0.8,
                    outliers_removed: 0,
                },
                inp_summary: None,
                lcp_samples: vec![100.0, 150.0, 200.0],
                cls_samples: vec![0.01, 0.02, 0.03],
                successful_iterations: 30,
                failed_iterations: 0,
                duration_ms: 5000,
                passed: true,
                failures: Vec::new(),
            }],
            total_duration_ms: 5000,
            passed: true,
            failures: Vec::new(),
            started_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_reporter_json_format() {
        let results = create_test_results();
        let reporter = Reporter::new(OutputFormat::Json);
        let output = reporter.format_results(&results).unwrap();

        assert!(output.contains("Test Suite"));
        assert!(output.contains("https://example.com"));
    }

    #[test]
    fn test_reporter_console_format() {
        let results = create_test_results();
        let reporter = Reporter::new(OutputFormat::Console);
        let output = reporter.format_results(&results).unwrap();

        assert!(output.contains("Test Suite"));
        assert!(output.contains("Homepage"));
    }

    #[test]
    fn test_reporter_markdown_format() {
        let results = create_test_results();
        let reporter = Reporter::new(OutputFormat::Markdown);
        let output = reporter.format_results(&results).unwrap();

        assert!(output.contains("# "));
        assert!(output.contains("Test Suite"));
    }

    #[test]
    fn test_default_format() {
        let reporter = Reporter::default();
        assert_eq!(reporter.format, OutputFormat::Console);
    }
}
