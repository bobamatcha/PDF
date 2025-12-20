use benchmark_harness::config::Config;
use std::env;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("crates/benchmark-harness/scenarios/getsignatures.toml")
    };

    println!("Validating config file: {}", config_path.display());

    let config = Config::from_file(&config_path)?;

    println!("\n✓ Successfully parsed configuration!");
    println!("\nBenchmark: {}", config.benchmark.name);
    println!("Base URL: {}", config.benchmark.base_url);
    println!("Iterations: {}", config.benchmark.iterations);
    println!("Warmup: {}", config.benchmark.warmup);
    println!("Parallel contexts: {}", config.benchmark.parallel_contexts);

    println!("\nThrottling:");
    println!("  Network profile: {:?}", config.throttling.network_profile);
    println!("  CPU slowdown: {}x", config.throttling.cpu_slowdown);

    println!("\nThresholds:");
    println!("  LCP p95: {:?}ms", config.thresholds.lcp_p95);
    println!("  INP p95: {:?}ms", config.thresholds.inp_p95);
    println!("  CLS p95: {:?}", config.thresholds.cls_p95);

    println!("\nScenarios ({}):", config.scenarios.len());
    for (i, scenario) in config.scenarios.iter().enumerate() {
        println!("  {}. {} ({} steps)", i + 1, scenario.name, scenario.steps.len());
    }

    println!("\n✓ All validations passed!");

    Ok(())
}
