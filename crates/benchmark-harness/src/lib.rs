//! Benchmark harness for web performance testing
//!
//! This crate provides a comprehensive benchmarking framework for measuring
//! web application performance metrics including Core Web Vitals (LCP, INP, CLS).
//!
//! # Features
//!
//! - **Core Web Vitals**: Automated collection of LCP, CLS, and INP metrics
//! - **Parallel Execution**: Run multiple browser contexts simultaneously
//! - **Throttling**: Network and CPU throttling to simulate real-world conditions
//! - **Statistical Analysis**: Percentiles, outlier detection, and variance analysis
//! - **Multiple Output Formats**: JSON, Console, and Markdown reports
//!
//! # Example
//!
//! ```no_run
//! use benchmark_harness::{Config, runner::BenchmarkRunner, reporter::{Reporter, OutputFormat}};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Load configuration
//! let config = Config::from_file("benchmark.toml")?;
//!
//! // Create runner and execute benchmarks
//! let runner = BenchmarkRunner::new().await?;
//! let results = runner.run(&config).await?;
//!
//! // Report results
//! let reporter = Reporter::new(OutputFormat::Console);
//! reporter.report(&results)?;
//!
//! // Or save to file
//! Reporter::new(OutputFormat::Json).write_to_file(&results, "results.json")?;
//! # Ok(())
//! # }
//! ```
//!
//! # Configuration
//!
//! Benchmarks are configured using TOML files:
//!
//! ```toml
//! [benchmark]
//! name = "My App Performance"
//! base_url = "https://myapp.com"
//! iterations = 30
//! warmup = 3
//! parallel_contexts = 4
//!
//! [throttling]
//! network_profile = "Fast3G"
//! cpu_slowdown = 4.0
//!
//! [thresholds]
//! lcp_p95 = 2500.0
//! inp_p95 = 200.0
//! cls_p95 = 0.1
//!
//! [[scenarios]]
//! name = "Homepage Load"
//! steps = [
//!     { type = "navigate", url = "/" },
//!     { type = "wait", wait_for = "network_idle" },
//!     { type = "measure" }
//! ]
//! ```

pub mod config;
pub mod metrics;
pub mod reporter;
pub mod runner;
pub mod stats;
pub mod throttling;

// Re-export main types for convenience
pub use config::Config;
pub use reporter::{OutputFormat, Reporter};
pub use runner::{BenchmarkResults, BenchmarkRunner, MetricSummary, ScenarioResult};
