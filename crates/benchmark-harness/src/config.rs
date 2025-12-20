//! Configuration parsing for benchmark scenarios
//!
//! This module provides TOML-based configuration for defining benchmark
//! scenarios, including test steps, throttling parameters, and performance thresholds.

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Main configuration structure loaded from TOML files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Benchmark configuration
    pub benchmark: BenchmarkConfig,
    /// Network and CPU throttling settings
    #[serde(default)]
    pub throttling: ThrottlingConfig,
    /// Performance metric thresholds
    #[serde(default)]
    pub thresholds: ThresholdsConfig,
    /// Test scenarios to execute
    pub scenarios: Vec<Scenario>,
}

impl Config {
    /// Load configuration from a TOML file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML is malformed
    /// - Required fields are missing
    ///
    /// # Example
    ///
    /// ```no_run
    /// use benchmark_harness::config::Config;
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let config = Config::from_file("benchmark.toml")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        Self::from_str(&content)
    }

    /// Parse configuration from a TOML string
    ///
    /// # Arguments
    ///
    /// * `s` - TOML configuration as a string
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is malformed or required fields are missing
    ///
    /// # Example
    ///
    /// ```
    /// use benchmark_harness::config::Config;
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let toml = r#"
    ///     [benchmark]
    ///     name = "My Test"
    ///     base_url = "https://example.com"
    ///
    ///     [[scenarios]]
    ///     name = "Homepage"
    ///     steps = []
    /// "#;
    /// let config = Config::from_str(toml)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        toml::from_str(s).context("Failed to parse TOML configuration")
    }
}

/// Core benchmark configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Name of the benchmark suite
    pub name: String,
    /// Base URL for the application under test
    pub base_url: String,
    /// Number of iterations per scenario (default: 30)
    #[serde(default = "default_iterations")]
    pub iterations: u32,
    /// Number of warmup runs before measurement (default: 3)
    #[serde(default = "default_warmup")]
    pub warmup: u32,
    /// Number of parallel browser contexts (default: 4)
    #[serde(default = "default_parallel_contexts")]
    pub parallel_contexts: u32,
}

fn default_iterations() -> u32 {
    30
}

fn default_warmup() -> u32 {
    3
}

fn default_parallel_contexts() -> u32 {
    4
}

/// Network and CPU throttling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrottlingConfig {
    /// Network profile to simulate
    #[serde(default)]
    pub network_profile: NetworkProfile,
    /// CPU slowdown multiplier (1.0 = no slowdown, 4.0 = 4x slower)
    #[serde(default = "default_cpu_slowdown")]
    pub cpu_slowdown: f64,
}

impl Default for ThrottlingConfig {
    fn default() -> Self {
        Self {
            network_profile: NetworkProfile::default(),
            cpu_slowdown: 1.0,
        }
    }
}

fn default_cpu_slowdown() -> f64 {
    1.0
}

/// Predefined network throttling profiles
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum NetworkProfile {
    /// Fast 3G network (1.6 Mbps down, 750 Kbps up, 562.5ms RTT)
    Fast3G,
    /// Slow 4G network (4 Mbps down, 3 Mbps up, 20ms RTT)
    Slow4G,
    /// Offline mode (no network)
    Offline,
    /// No throttling
    #[default]
    None,
}

impl NetworkProfile {
    /// Get the download speed in bytes per second
    pub fn download_bps(&self) -> Option<u64> {
        match self {
            NetworkProfile::Fast3G => Some(1_600_000 / 8),
            NetworkProfile::Slow4G => Some(4_000_000 / 8),
            NetworkProfile::Offline => Some(0),
            NetworkProfile::None => None,
        }
    }

    /// Get the upload speed in bytes per second
    pub fn upload_bps(&self) -> Option<u64> {
        match self {
            NetworkProfile::Fast3G => Some(750_000 / 8),
            NetworkProfile::Slow4G => Some(3_000_000 / 8),
            NetworkProfile::Offline => Some(0),
            NetworkProfile::None => None,
        }
    }

    /// Get the round-trip time in milliseconds
    pub fn rtt_ms(&self) -> Option<u64> {
        match self {
            NetworkProfile::Fast3G => Some(562),
            NetworkProfile::Slow4G => Some(20),
            NetworkProfile::Offline => Some(0),
            NetworkProfile::None => None,
        }
    }
}

/// Performance metric thresholds for pass/fail determination
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThresholdsConfig {
    /// Largest Contentful Paint p95 threshold in milliseconds
    pub lcp_p95: Option<f64>,
    /// Interaction to Next Paint p95 threshold in milliseconds
    pub inp_p95: Option<f64>,
    /// Cumulative Layout Shift p95 threshold (unitless)
    pub cls_p95: Option<f64>,
}

/// A test scenario consisting of multiple steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// Name of the scenario
    pub name: String,
    /// Steps to execute in sequence
    pub steps: Vec<BenchmarkStep>,
}

/// Individual benchmark step in a scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BenchmarkStep {
    /// Navigate to a URL
    Navigate {
        /// URL to navigate to (relative to base_url or absolute)
        url: String,
    },
    /// Wait for a condition
    Wait {
        /// Condition to wait for
        #[serde(flatten)]
        condition: WaitCondition,
    },
    /// Click an element
    Click {
        /// CSS selector for the element
        selector: String,
    },
    /// Type text into an input field
    Type {
        /// CSS selector for the input
        selector: String,
        /// Text to type
        text: String,
    },
    /// Upload a file
    Upload {
        /// CSS selector for the file input
        selector: String,
        /// Path to the file to upload
        file_path: String,
    },
    /// Measure performance metrics
    Measure {
        /// Optional label for this measurement
        #[serde(default)]
        label: Option<String>,
    },
}

/// Conditions to wait for during test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "wait_for", rename_all = "snake_case")]
pub enum WaitCondition {
    /// Wait for network to be idle
    NetworkIdle,
    /// Wait for a CSS selector to appear
    Selector {
        /// CSS selector to wait for
        selector: String,
    },
    /// Wait for a specific duration
    Timeout {
        /// Duration in milliseconds
        #[serde(with = "duration_ms")]
        duration: Duration,
    },
}

/// Serde module for serializing/deserializing Duration as milliseconds
mod duration_ms {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ms = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            [benchmark]
            name = "Test Suite"
            base_url = "https://example.com"

            [[scenarios]]
            name = "Homepage"
            steps = []
        "#;

        let config = Config::from_str(toml).unwrap();
        assert_eq!(config.benchmark.name, "Test Suite");
        assert_eq!(config.benchmark.base_url, "https://example.com");
        assert_eq!(config.benchmark.iterations, 30);
        assert_eq!(config.benchmark.warmup, 3);
        assert_eq!(config.benchmark.parallel_contexts, 4);
        assert_eq!(config.scenarios.len(), 1);
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r##"
            [benchmark]
            name = "Full Test"
            base_url = "https://example.com"
            iterations = 50
            warmup = 5
            parallel_contexts = 8

            [throttling]
            network_profile = "Fast3G"
            cpu_slowdown = 4.0

            [thresholds]
            lcp_p95 = 2500.0
            inp_p95 = 200.0
            cls_p95 = 0.1

            [[scenarios]]
            name = "Login Flow"
            steps = [
                { type = "navigate", url = "/login" },
                { type = "wait", wait_for = "selector", selector = "#username" },
                { type = "type", selector = "#username", text = "test@example.com" },
                { type = "type", selector = "#password", text = "password123" },
                { type = "click", selector = "#submit" },
                { type = "wait", wait_for = "network_idle" },
                { type = "measure", label = "post-login" }
            ]
        "##;

        let config = Config::from_str(toml).unwrap();
        assert_eq!(config.benchmark.iterations, 50);
        assert_eq!(config.benchmark.warmup, 5);
        assert_eq!(config.benchmark.parallel_contexts, 8);
        assert_eq!(config.throttling.network_profile, NetworkProfile::Fast3G);
        assert_eq!(config.throttling.cpu_slowdown, 4.0);
        assert_eq!(config.thresholds.lcp_p95, Some(2500.0));
        assert_eq!(config.thresholds.inp_p95, Some(200.0));
        assert_eq!(config.thresholds.cls_p95, Some(0.1));
        assert_eq!(config.scenarios.len(), 1);
        assert_eq!(config.scenarios[0].steps.len(), 7);
    }

    #[test]
    fn test_network_profile_speeds() {
        assert_eq!(
            NetworkProfile::Fast3G.download_bps(),
            Some(1_600_000 / 8)
        );
        assert_eq!(NetworkProfile::Fast3G.upload_bps(), Some(750_000 / 8));
        assert_eq!(NetworkProfile::Fast3G.rtt_ms(), Some(562));

        assert_eq!(
            NetworkProfile::Slow4G.download_bps(),
            Some(4_000_000 / 8)
        );
        assert_eq!(NetworkProfile::Slow4G.upload_bps(), Some(3_000_000 / 8));
        assert_eq!(NetworkProfile::Slow4G.rtt_ms(), Some(20));

        assert_eq!(NetworkProfile::None.download_bps(), None);
        assert_eq!(NetworkProfile::None.upload_bps(), None);
        assert_eq!(NetworkProfile::None.rtt_ms(), None);
    }

    #[test]
    fn test_parse_wait_timeout() {
        let toml = r#"
            [benchmark]
            name = "Test"
            base_url = "https://example.com"

            [[scenarios]]
            name = "Timeout Test"
            steps = [
                { type = "wait", wait_for = "timeout", duration = 5000 }
            ]
        "#;

        let config = Config::from_str(toml).unwrap();
        match &config.scenarios[0].steps[0] {
            BenchmarkStep::Wait {
                condition: WaitCondition::Timeout { duration },
            } => {
                assert_eq!(*duration, Duration::from_millis(5000));
            }
            _ => panic!("Expected Wait step with Timeout condition"),
        }
    }

    #[test]
    fn test_parse_upload_step() {
        let toml = r##"
            [benchmark]
            name = "Test"
            base_url = "https://example.com"

            [[scenarios]]
            name = "Upload Test"
            steps = [
                { type = "upload", selector = "#file-input", file_path = "/tmp/test.pdf" }
            ]
        "##;

        let config = Config::from_str(toml).unwrap();
        match &config.scenarios[0].steps[0] {
            BenchmarkStep::Upload {
                selector,
                file_path,
            } => {
                assert_eq!(selector, "#file-input");
                assert_eq!(file_path, "/tmp/test.pdf");
            }
            _ => panic!("Expected Upload step"),
        }
    }

    #[test]
    fn test_default_values() {
        let toml = r#"
            [benchmark]
            name = "Test"
            base_url = "https://example.com"

            [[scenarios]]
            name = "Test"
            steps = []
        "#;

        let config = Config::from_str(toml).unwrap();
        assert_eq!(config.throttling.network_profile, NetworkProfile::None);
        assert_eq!(config.throttling.cpu_slowdown, 1.0);
        assert_eq!(config.thresholds.lcp_p95, None);
        assert_eq!(config.thresholds.inp_p95, None);
        assert_eq!(config.thresholds.cls_p95, None);
    }
}
