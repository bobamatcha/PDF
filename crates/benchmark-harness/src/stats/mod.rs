//! Statistical analysis of benchmark results
//!
//! This module provides statistical functions for analyzing benchmark data,
//! including percentile calculations, outlier detection, and summary statistics.
//!
//! # Examples
//!
//! ```
//! use benchmark_harness::stats::{PercentileSummary, OutlierResult, remove_warmup};
//!
//! // Sample data with warmup period
//! let samples = vec![100.0, 95.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 150.0];
//!
//! // Remove warmup iterations
//! let clean = remove_warmup(&samples, 2);
//!
//! // Detect outliers
//! let outliers = OutlierResult::detect(clean).unwrap();
//! let filtered = outliers.clean_samples(clean);
//!
//! // Calculate summary statistics
//! let summary = PercentileSummary::from_samples(&filtered).unwrap();
//! println!("Median: {}, p95: {}", summary.p50, summary.p95);
//! ```

pub mod outliers;
pub mod percentiles;

// Re-export main types and functions
pub use outliers::{remove_warmup, OutlierResult};
pub use percentiles::{percentile, PercentileSummary};
