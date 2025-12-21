//! JSON reporter for benchmark results

use crate::runner::BenchmarkResults;
use anyhow::Result;

/// JSON format reporter
pub struct JsonReporter;

impl JsonReporter {
    /// Format benchmark results as JSON
    ///
    /// # Arguments
    ///
    /// * `results` - The benchmark results to format
    /// * `pretty` - Whether to pretty-print the JSON
    ///
    /// # Returns
    ///
    /// JSON string representation of the results
    pub fn format(results: &BenchmarkResults, pretty: bool) -> Result<String> {
        let output = if pretty {
            serde_json::to_string_pretty(results)?
        } else {
            serde_json::to_string(results)?
        };
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::ConfigSummary;

    fn create_test_results() -> BenchmarkResults {
        BenchmarkResults {
            suite_name: "Test".to_string(),
            base_url: "https://example.com".to_string(),
            config_summary: ConfigSummary {
                iterations: 30,
                warmup: 3,
                parallel_contexts: 4,
                network_profile: "None".to_string(),
                cpu_slowdown: 1.0,
            },
            scenario_results: vec![],
            total_duration_ms: 1000,
            passed: true,
            failures: vec![],
            started_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_json_format_compact() {
        let results = create_test_results();
        let output = JsonReporter::format(&results, false).unwrap();

        // Compact JSON should not have newlines
        assert!(!output.contains('\n'));
        assert!(output.contains("\"suite_name\":\"Test\""));
    }

    #[test]
    fn test_json_format_pretty() {
        let results = create_test_results();
        let output = JsonReporter::format(&results, true).unwrap();

        // Pretty JSON should have indentation
        assert!(output.contains('\n'));
        assert!(output.contains("  "));
    }

    #[test]
    fn test_json_roundtrip() {
        let results = create_test_results();
        let json = JsonReporter::format(&results, false).unwrap();
        let parsed: BenchmarkResults = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.suite_name, results.suite_name);
        assert_eq!(parsed.base_url, results.base_url);
        assert_eq!(parsed.passed, results.passed);
    }
}
