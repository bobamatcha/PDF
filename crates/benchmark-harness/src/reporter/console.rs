//! Console reporter for benchmark results
//!
//! Provides human-readable output with ASCII tables and status indicators.

use anyhow::Result;
use std::fmt::Write;

use crate::runner::{BenchmarkResults, MetricSummary, ScenarioResult};

/// Console format reporter
pub struct ConsoleReporter;

impl ConsoleReporter {
    /// Format benchmark results for console output
    pub fn format(results: &BenchmarkResults) -> Result<String> {
        let mut output = String::new();

        // Header
        writeln!(output)?;
        writeln!(output, "╔══════════════════════════════════════════════════════════════╗")?;
        writeln!(output, "║                    BENCHMARK RESULTS                          ║")?;
        writeln!(output, "╚══════════════════════════════════════════════════════════════╝")?;
        writeln!(output)?;

        // Suite info
        writeln!(output, "Suite:     {}", results.suite_name)?;
        writeln!(output, "Base URL:  {}", results.base_url)?;
        writeln!(output, "Started:   {}", results.started_at)?;
        writeln!(output, "Duration:  {}ms", results.total_duration_ms)?;
        writeln!(output)?;

        // Configuration
        writeln!(output, "Configuration:")?;
        writeln!(output, "  Iterations:         {}", results.config_summary.iterations)?;
        writeln!(output, "  Warmup:             {}", results.config_summary.warmup)?;
        writeln!(output, "  Parallel Contexts:  {}", results.config_summary.parallel_contexts)?;
        writeln!(output, "  Network Profile:    {}", results.config_summary.network_profile)?;
        writeln!(output, "  CPU Slowdown:       {}x", results.config_summary.cpu_slowdown)?;
        writeln!(output)?;

        // Scenario results
        for scenario in &results.scenario_results {
            Self::format_scenario(&mut output, scenario)?;
        }

        // Summary
        writeln!(output)?;
        writeln!(output, "────────────────────────────────────────────────────────────────")?;
        let status = if results.passed { "PASSED" } else { "FAILED" };
        let status_symbol = if results.passed { "✓" } else { "✗" };
        writeln!(output, "Overall Status: {} {}", status_symbol, status)?;

        if !results.failures.is_empty() {
            writeln!(output)?;
            writeln!(output, "Failures:")?;
            for failure in &results.failures {
                writeln!(output, "  • {}", failure)?;
            }
        }

        writeln!(output)?;
        Ok(output)
    }

    fn format_scenario(output: &mut String, scenario: &ScenarioResult) -> Result<()> {
        let status = if scenario.passed { "✓" } else { "✗" };

        writeln!(output, "────────────────────────────────────────────────────────────────")?;
        writeln!(output, "Scenario: {} {}", scenario.scenario_name, status)?;
        writeln!(output, "────────────────────────────────────────────────────────────────")?;
        writeln!(output)?;

        // Iterations info
        writeln!(
            output,
            "  Iterations: {} successful, {} failed",
            scenario.successful_iterations, scenario.failed_iterations
        )?;
        writeln!(output, "  Duration:   {}ms", scenario.duration_ms)?;
        writeln!(output)?;

        // Metrics table
        writeln!(output, "  ┌─────────┬─────────┬─────────┬─────────┬─────────┬─────────┐")?;
        writeln!(output, "  │ Metric  │   Min   │   P50   │   P95   │   P99   │   Max   │")?;
        writeln!(output, "  ├─────────┼─────────┼─────────┼─────────┼─────────┼─────────┤")?;

        Self::format_metric_row(output, "LCP", &scenario.lcp_summary, "ms")?;
        Self::format_metric_row(output, "CLS", &scenario.cls_summary, "")?;

        if let Some(ref inp) = scenario.inp_summary {
            Self::format_metric_row(output, "INP", inp, "ms")?;
        }

        writeln!(output, "  └─────────┴─────────┴─────────┴─────────┴─────────┴─────────┘")?;
        writeln!(output)?;

        // Additional stats
        writeln!(output, "  LCP Stats:")?;
        writeln!(
            output,
            "    Mean: {:.1}ms, StdDev: {:.1}ms, CV: {:.2}%, Outliers Removed: {}",
            scenario.lcp_summary.mean,
            scenario.lcp_summary.std_dev,
            scenario.lcp_summary.cv * 100.0,
            scenario.lcp_summary.outliers_removed
        )?;

        writeln!(output, "  CLS Stats:")?;
        writeln!(
            output,
            "    Mean: {:.4}, StdDev: {:.4}, CV: {:.2}%, Outliers Removed: {}",
            scenario.cls_summary.mean,
            scenario.cls_summary.std_dev,
            scenario.cls_summary.cv * 100.0,
            scenario.cls_summary.outliers_removed
        )?;

        if !scenario.failures.is_empty() {
            writeln!(output)?;
            writeln!(output, "  Threshold Violations:")?;
            for failure in &scenario.failures {
                writeln!(output, "    ✗ {}", failure)?;
            }
        }

        writeln!(output)?;
        Ok(())
    }

    fn format_metric_row(
        output: &mut String,
        name: &str,
        summary: &MetricSummary,
        unit: &str,
    ) -> Result<()> {
        let format_value = |v: f64| -> String {
            if unit == "ms" {
                format!("{:.0}", v)
            } else if v < 1.0 {
                format!("{:.4}", v)
            } else {
                format!("{:.2}", v)
            }
        };

        writeln!(
            output,
            "  │ {:^7} │ {:>7} │ {:>7} │ {:>7} │ {:>7} │ {:>7} │",
            name,
            format_value(summary.min),
            format_value(summary.p50),
            format_value(summary.p95),
            format_value(summary.p99),
            format_value(summary.max)
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::ConfigSummary;

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
                inp_summary: Some(MetricSummary {
                    min: 50.0,
                    p25: 75.0,
                    p50: 100.0,
                    p75: 125.0,
                    p95: 150.0,
                    p99: 175.0,
                    max: 200.0,
                    mean: 100.0,
                    std_dev: 30.0,
                    count: 30,
                    cv: 0.3,
                    outliers_removed: 1,
                }),
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
    fn test_console_format_contains_suite_name() {
        let results = create_test_results();
        let output = ConsoleReporter::format(&results).unwrap();

        assert!(output.contains("Test Suite"));
    }

    #[test]
    fn test_console_format_contains_metrics_table() {
        let results = create_test_results();
        let output = ConsoleReporter::format(&results).unwrap();

        assert!(output.contains("LCP"));
        assert!(output.contains("CLS"));
        assert!(output.contains("INP"));
        assert!(output.contains("P50"));
        assert!(output.contains("P95"));
    }

    #[test]
    fn test_console_format_shows_pass_status() {
        let results = create_test_results();
        let output = ConsoleReporter::format(&results).unwrap();

        assert!(output.contains("PASSED"));
        assert!(output.contains("✓"));
    }

    #[test]
    fn test_console_format_shows_failures() {
        let mut results = create_test_results();
        results.passed = false;
        results.failures = vec!["LCP p95 exceeded threshold".to_string()];

        let output = ConsoleReporter::format(&results).unwrap();

        assert!(output.contains("FAILED"));
        assert!(output.contains("LCP p95 exceeded threshold"));
    }

    #[test]
    fn test_console_format_shows_config() {
        let results = create_test_results();
        let output = ConsoleReporter::format(&results).unwrap();

        assert!(output.contains("Iterations:"));
        assert!(output.contains("30"));
        assert!(output.contains("Parallel Contexts:"));
        assert!(output.contains("4"));
    }
}
