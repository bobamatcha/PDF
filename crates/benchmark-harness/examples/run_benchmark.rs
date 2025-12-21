//! Run benchmarks from a TOML config file
//!
//! Usage: cargo run -p benchmark-harness --example run_benchmark -- <config.toml>

use anyhow::Result;
use benchmark_harness::config::Config;
use benchmark_harness::reporter::{OutputFormat, Reporter};
use benchmark_harness::runner::BenchmarkRunner;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).expect("Usage: run_benchmark <config.toml>");

    println!("Loading config from: {}", config_path);
    let config = Config::from_file(config_path)?;

    println!("Starting benchmark: {}", config.benchmark.name);
    println!("  Base URL: {}", config.benchmark.base_url);
    println!("  Iterations: {}", config.benchmark.iterations);
    println!("  Warmup: {}", config.benchmark.warmup);
    println!("  Scenarios: {}", config.scenarios.len());
    println!();

    let runner = BenchmarkRunner::new().await?;
    let results = runner.run(&config).await?;

    // Print results
    let reporter = Reporter::new(OutputFormat::Console);
    reporter.report(&results)?;

    Ok(())
}
