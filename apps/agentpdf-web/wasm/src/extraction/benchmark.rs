//! Benchmarking framework for comparing extraction strategies

use super::analyzer::PdfAnalysis;
use super::router::{ExtractionConfig, ExtractionRouter, ExtractionStrategy};
use super::types::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// PDF size categories for benchmarking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdfCategory {
    /// < 100KB, 1-5 pages
    Small,
    /// 100KB - 1MB, 5-20 pages
    Medium,
    /// 1MB - 10MB, 20-100 pages
    Large,
    /// > 10MB, 100+ pages
    XLarge,
}

impl PdfCategory {
    pub fn from_size(bytes: usize, pages: u32) -> Self {
        match (bytes, pages) {
            (0..=102400, 0..=5) => PdfCategory::Small,
            (0..=1048576, 0..=20) => PdfCategory::Medium,
            (0..=10485760, 0..=100) => PdfCategory::Large,
            _ => PdfCategory::XLarge,
        }
    }
}

/// Result from a single benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Name/identifier of the PDF tested
    pub pdf_name: String,
    /// Size category
    pub category: PdfCategory,
    /// Strategy used
    pub strategy: ExtractionStrategy,
    /// Which backend actually performed the extraction
    pub backend_used: String,
    /// Time to extract in milliseconds
    pub extraction_time_ms: f64,
    /// Peak memory usage (if measurable)
    pub memory_usage_mb: Option<f64>,
    /// Extraction accuracy (% of expected characters)
    pub accuracy_percent: Option<f64>,
    /// Total characters extracted
    pub characters_extracted: usize,
    /// Number of pages processed
    pub pages_processed: u32,
    /// Whether a fallback was triggered
    pub fallback_occurred: bool,
    /// Any errors encountered
    pub error: Option<String>,
    /// PDF analysis results
    pub analysis: Option<PdfAnalysis>,
}

impl BenchmarkResult {
    pub fn success(
        pdf_name: &str,
        category: PdfCategory,
        strategy: ExtractionStrategy,
        result: &ExtractionResult,
    ) -> Self {
        Self {
            pdf_name: pdf_name.to_string(),
            category,
            strategy,
            backend_used: result.backend_used.clone(),
            extraction_time_ms: result.extraction_time_ms,
            memory_usage_mb: None,
            accuracy_percent: None,
            characters_extracted: result.total_characters,
            pages_processed: result.pages.len() as u32,
            fallback_occurred: result.fallback_occurred,
            error: None,
            analysis: None,
        }
    }

    pub fn failure(
        pdf_name: &str,
        category: PdfCategory,
        strategy: ExtractionStrategy,
        error: &str,
        time_ms: f64,
    ) -> Self {
        Self {
            pdf_name: pdf_name.to_string(),
            category,
            strategy,
            backend_used: "none".to_string(),
            extraction_time_ms: time_ms,
            memory_usage_mb: None,
            accuracy_percent: None,
            characters_extracted: 0,
            pages_processed: 0,
            fallback_occurred: false,
            error: Some(error.to_string()),
            analysis: None,
        }
    }

    pub fn with_analysis(mut self, analysis: PdfAnalysis) -> Self {
        self.analysis = Some(analysis);
        self
    }

    pub fn with_accuracy(mut self, accuracy: f64) -> Self {
        self.accuracy_percent = Some(accuracy);
        self
    }
}

/// Benchmark runner for comparing extraction strategies
pub struct BenchmarkRunner {
    strategies: Vec<ExtractionStrategy>,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            strategies: vec![ExtractionStrategy::Legacy, ExtractionStrategy::Hybrid],
        }
    }

    pub fn with_strategies(strategies: Vec<ExtractionStrategy>) -> Self {
        Self { strategies }
    }

    /// Run benchmark on a single PDF with all configured strategies
    pub async fn benchmark_pdf(
        &self,
        pdf_name: &str,
        data: &[u8],
        expected_text: Option<&str>,
    ) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();

        // Analyze PDF first
        let analysis = PdfAnalysis::analyze(data);
        let category = PdfCategory::from_size(data.len(), analysis.page_count);

        for strategy in &self.strategies {
            let config = ExtractionConfig {
                strategy: *strategy,
                ..Default::default()
            };

            let router = ExtractionRouter::new(config);

            let result = match router.extract(data).await {
                Ok(extraction_result) => {
                    let mut bench_result =
                        BenchmarkResult::success(pdf_name, category, *strategy, &extraction_result);

                    // Calculate accuracy if expected text provided
                    if let Some(expected) = expected_text {
                        let extracted: String = extraction_result
                            .pages
                            .iter()
                            .map(|p| p.raw_text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");

                        let accuracy = Self::calculate_accuracy(&extracted, expected);
                        bench_result = bench_result.with_accuracy(accuracy);
                    }

                    bench_result.with_analysis(analysis.clone())
                }
                Err(e) => {
                    BenchmarkResult::failure(pdf_name, category, *strategy, &e.to_string(), 0.0)
                        .with_analysis(analysis.clone())
                }
            };

            results.push(result);
        }

        results
    }

    /// Calculate text extraction accuracy using normalized comparison
    fn calculate_accuracy(extracted: &str, expected: &str) -> f64 {
        if expected.is_empty() {
            return if extracted.is_empty() { 100.0 } else { 0.0 };
        }

        // Normalize both strings
        let extracted_normalized = Self::normalize_text(extracted);
        let expected_normalized = Self::normalize_text(expected);

        // Calculate Jaccard similarity on words
        let extracted_words: std::collections::HashSet<_> =
            extracted_normalized.split_whitespace().collect();
        let expected_words: std::collections::HashSet<_> =
            expected_normalized.split_whitespace().collect();

        if expected_words.is_empty() {
            return 100.0;
        }

        let intersection = extracted_words.intersection(&expected_words).count();
        let union = extracted_words.union(&expected_words).count();

        if union == 0 {
            return 100.0;
        }

        (intersection as f64 / union as f64) * 100.0
    }

    /// Normalize text for comparison
    fn normalize_text(text: &str) -> String {
        text.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Generate a summary report from benchmark results
    pub fn generate_report(results: &[BenchmarkResult]) -> BenchmarkReport {
        let mut report = BenchmarkReport::default();

        for result in results {
            report.total_tests += 1;

            if result.error.is_some() {
                report.failures += 1;
            } else {
                report.successes += 1;
            }

            if result.fallback_occurred {
                report.fallback_count += 1;
            }

            report.total_time_ms += result.extraction_time_ms;
            report.total_characters += result.characters_extracted;

            // Track by strategy
            let strategy_key = format!("{:?}", result.strategy);
            let entry = report
                .by_strategy
                .entry(strategy_key)
                .or_insert_with(StrategyStats::default);
            entry.tests += 1;
            entry.total_time_ms += result.extraction_time_ms;
            if result.error.is_some() {
                entry.failures += 1;
            }
            if let Some(acc) = result.accuracy_percent {
                entry.total_accuracy += acc;
                entry.accuracy_count += 1;
            }

            // Track by category
            let category_key = format!("{:?}", result.category);
            let cat_entry = report
                .by_category
                .entry(category_key)
                .or_insert_with(CategoryStats::default);
            cat_entry.tests += 1;
            cat_entry.total_time_ms += result.extraction_time_ms;
        }

        // Calculate averages
        for stats in report.by_strategy.values_mut() {
            if stats.tests > 0 {
                stats.avg_time_ms = stats.total_time_ms / stats.tests as f64;
            }
            if stats.accuracy_count > 0 {
                stats.avg_accuracy = stats.total_accuracy / stats.accuracy_count as f64;
            }
        }

        for stats in report.by_category.values_mut() {
            if stats.tests > 0 {
                stats.avg_time_ms = stats.total_time_ms / stats.tests as f64;
            }
        }

        report
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary report of benchmark results
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub total_tests: usize,
    pub successes: usize,
    pub failures: usize,
    pub fallback_count: usize,
    pub total_time_ms: f64,
    pub total_characters: usize,
    pub by_strategy: std::collections::HashMap<String, StrategyStats>,
    pub by_category: std::collections::HashMap<String, CategoryStats>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StrategyStats {
    pub tests: usize,
    pub failures: usize,
    pub total_time_ms: f64,
    pub avg_time_ms: f64,
    pub total_accuracy: f64,
    pub accuracy_count: usize,
    pub avg_accuracy: f64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub tests: usize,
    pub total_time_ms: f64,
    pub avg_time_ms: f64,
}

/// WASM-exposed benchmark function
#[wasm_bindgen]
pub async fn run_benchmark(
    pdf_data: &[u8],
    pdf_name: &str,
    expected_text: Option<String>,
) -> Result<JsValue, JsValue> {
    let runner = BenchmarkRunner::new();
    let results = runner
        .benchmark_pdf(pdf_name, pdf_data, expected_text.as_deref())
        .await;

    serde_wasm_bindgen::to_value(&results)
        .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))
}

/// WASM-exposed function to run benchmark and get summary report
#[wasm_bindgen]
pub async fn run_benchmark_with_report(
    pdf_data: &[u8],
    pdf_name: &str,
) -> Result<JsValue, JsValue> {
    let runner = BenchmarkRunner::new();
    let results = runner.benchmark_pdf(pdf_name, pdf_data, None).await;
    let report = BenchmarkRunner::generate_report(&results);

    let output = serde_json::json!({
        "results": results,
        "report": report,
    });

    serde_wasm_bindgen::to_value(&output)
        .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_category_small() {
        assert_eq!(PdfCategory::from_size(50000, 3), PdfCategory::Small);
    }

    #[test]
    fn test_pdf_category_medium() {
        assert_eq!(PdfCategory::from_size(500000, 10), PdfCategory::Medium);
    }

    #[test]
    fn test_pdf_category_large() {
        assert_eq!(PdfCategory::from_size(5000000, 50), PdfCategory::Large);
    }

    #[test]
    fn test_pdf_category_xlarge() {
        assert_eq!(PdfCategory::from_size(20000000, 200), PdfCategory::XLarge);
    }

    #[test]
    fn test_accuracy_calculation() {
        let extracted = "hello world this is a test";
        let expected = "hello world this is a test";
        let accuracy = BenchmarkRunner::calculate_accuracy(extracted, expected);
        assert!((accuracy - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_accuracy_partial_match() {
        let extracted = "hello world";
        let expected = "hello world this is a test";
        let accuracy = BenchmarkRunner::calculate_accuracy(extracted, expected);
        assert!(accuracy > 0.0 && accuracy < 100.0);
    }

    #[test]
    fn test_normalize_text() {
        let text = "  Hello,   WORLD!  123  ";
        let normalized = BenchmarkRunner::normalize_text(text);
        assert_eq!(normalized, "hello world 123");
    }
}
