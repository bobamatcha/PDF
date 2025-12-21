//! Metrics collection module for Core Web Vitals and custom timing measurements
//!
//! This module provides collectors for:
//! - **Core Web Vitals**: LCP, CLS, INP via the web-vitals library
//! - **Custom Timing**: User Timing API measurements via performance.mark()/measure()
//!
//! # Example
//!
//! ```no_run
//! use benchmark_harness::metrics::web_vitals::WebVitalsCollector;
//! # use chromiumoxide::Page;
//!
//! # async fn example(page: &Page) -> anyhow::Result<()> {
//! let collector = WebVitalsCollector::new();
//! collector.inject_into_page(page).await?;
//!
//! // Navigate and interact with the page...
//!
//! let lcp = collector.wait_for_lcp(page, std::time::Duration::from_secs(30)).await?;
//! println!("LCP: {}ms", lcp.value);
//! # Ok(())
//! # }
//! ```

pub mod custom;
pub mod web_vitals;

// Re-export commonly used types
pub use custom::{CustomTimingCollector, TimingMeasurement};
pub use web_vitals::{WebVitalMetric, WebVitalsCollector};
