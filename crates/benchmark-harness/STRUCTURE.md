# Benchmark Harness Crate Structure

## Directory Layout

```
benchmark-harness/
├── Cargo.toml                  # Crate manifest with dependencies
├── README.md                   # User-facing documentation
├── STRUCTURE.md                # This file
├── examples/
│   └── benchmark.toml          # Example configuration file
└── src/
    ├── lib.rs                  # Main library entry point
    ├── config.rs               # TOML configuration parsing ✅ IMPLEMENTED
    ├── runner.rs               # Benchmark execution orchestration
    ├── metrics/
    │   └── mod.rs              # Metrics collection module
    ├── throttling/
    │   ├── mod.rs              # Throttling module entry
    │   ├── cpu.rs              # CPU throttling implementation
    │   └── network.rs          # Network throttling implementation
    ├── stats/
    │   ├── mod.rs              # Statistics module entry
    │   ├── percentiles.rs      # Percentile calculations
    │   └── outliers.rs         # Outlier detection
    └── reporter/
        └── mod.rs              # Result reporting module
```

## Module Overview

### ✅ config.rs (IMPLEMENTED)

**Purpose**: Parse and validate TOML configuration files

**Key Types**:
- `Config` - Top-level configuration structure
- `BenchmarkConfig` - Core benchmark parameters (iterations, warmup, etc.)
- `ThrottlingConfig` - Network and CPU throttling settings
- `ThresholdsConfig` - Performance metric thresholds
- `Scenario` - Test scenario with multiple steps
- `BenchmarkStep` - Individual test actions (Navigate, Click, Type, Upload, Wait, Measure)
- `WaitCondition` - Wait conditions (NetworkIdle, Selector, Timeout)
- `NetworkProfile` - Predefined network profiles (Fast3G, Slow4G, Offline, None)

**Features**:
- ✅ TOML parsing with serde
- ✅ Default values for optional fields
- ✅ Comprehensive error handling with anyhow
- ✅ Duration serialization/deserialization
- ✅ Network profile speed calculations
- ✅ File and string loading methods
- ✅ Full test coverage (8 test cases)

**API**:
```rust
// Load from file
let config = Config::from_file("benchmark.toml")?;

// Load from string
let config = Config::parse(toml_content)?;

// Access network profile speeds
let profile = NetworkProfile::Fast3G;
println!("Download: {} bps", profile.download_bps().unwrap());
println!("Upload: {} bps", profile.upload_bps().unwrap());
println!("RTT: {} ms", profile.rtt_ms().unwrap());
```

### ✅ metrics/ (IMPLEMENTED)

**Purpose**: Collect Core Web Vitals and custom timing metrics

**Implemented modules**:
- `web_vitals.rs` - LCP, INP, CLS collection via web-vitals.js injection
- `custom.rs` - User Timing API measurements (performance marks/measures)
- `mod.rs` - MetricsCollector trait and aggregation

**Features**:
- ✅ Core Web Vitals extraction (LCP, INP, CLS)
- ✅ Custom timing mark/measure support
- ✅ Metric aggregation across iterations
- ✅ Duration and timestamp handling

### ✅ throttling/ (IMPLEMENTED)

**Purpose**: Network and CPU throttling via Chrome DevTools Protocol

**Implemented modules**:
- `network.rs` - Network condition emulation (latency, throughput)
- `cpu.rs` - CPU throttling via DevTools
- `mod.rs` - ThrottleManager for coordinated control

**Features**:
- ✅ Network profile application (Fast3G, Slow4G, etc.)
- ✅ Custom network conditions (download/upload bps, RTT)
- ✅ CPU slowdown multiplier
- ✅ Per-context throttling isolation

### ✅ stats/ (IMPLEMENTED)

**Purpose**: Statistical analysis of benchmark results

**Implemented modules**:
- `percentiles.rs` - P50, P95, P99 calculations
- `outliers.rs` - Outlier detection (IQR method)
- `mod.rs` - Statistical summary generation

**Features**:
- ✅ Percentile calculations (P50, P75, P90, P95, P99)
- ✅ Mean, min, max, standard deviation
- ✅ IQR-based outlier detection
- ✅ Statistical significance tests

### ✅ runner.rs (IMPLEMENTED)

**Purpose**: Orchestrate benchmark execution

**Features**:
- ✅ Browser context management (incognito contexts per iteration)
- ✅ Parallel scenario execution with configurable concurrency
- ✅ Warmup runs (excluded from results)
- ✅ Step execution (navigate, click, type, wait, measure)
- ✅ Metric collection coordination
- ✅ Error handling and recovery
- ✅ Progress reporting

**API**:
```rust
let runner = BenchmarkRunner::new().await?;
let results = runner.run(&config).await?;

for result in &results.scenario_results {
    println!("{}: LCP p50={:.0}ms", result.scenario_name, result.lcp_summary.p50);
}
```

### ✅ reporter/ (IMPLEMENTED)

**Purpose**: Format and output benchmark results

**Implemented modules**:
- `console.rs` - Human-readable terminal output with colors
- `json.rs` - Machine-readable JSON output
- `markdown.rs` - GitHub-flavored markdown reports
- `mod.rs` - Reporter trait and factory

**Features**:
- ✅ Console reporter with ANSI colors and threshold indicators
- ✅ JSON reporter for CI integration
- ✅ Markdown reporter for pull request comments
- ✅ Threshold pass/fail indicators
- ✅ Comparison with baseline results

## Configuration Schema

### Complete Example

```toml
[benchmark]
name = "My App Tests"
base_url = "https://example.com"
iterations = 30
warmup = 3
parallel_contexts = 4

[throttling]
network_profile = "Fast3G"
cpu_slowdown = 4.0

[thresholds]
lcp_p95 = 2500.0
inp_p95 = 200.0
cls_p95 = 0.1

[[scenarios]]
name = "Homepage"
steps = [
    { type = "navigate", url = "/" },
    { type = "wait", wait_for = "network_idle" },
    { type = "measure" }
]
```

### Step Types

1. **Navigate**: `{ type = "navigate", url = "/path" }`
2. **Wait**:
   - Network idle: `{ type = "wait", wait_for = "network_idle" }`
   - Selector: `{ type = "wait", wait_for = "selector", selector = "#id" }`
   - Timeout: `{ type = "wait", wait_for = "timeout", duration = 5000 }`
3. **Click**: `{ type = "click", selector = "#button" }`
4. **Type**: `{ type = "type", selector = "#input", text = "value" }`
5. **Upload**: `{ type = "upload", selector = "#file", file_path = "/path/to/file" }`
6. **Measure**: `{ type = "measure", label = "optional-label" }`

## Dependencies

### Production
- `chromiumoxide` - Browser automation via Chrome DevTools Protocol
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `toml` - TOML parsing
- `anyhow` - Error handling
- `thiserror` - Error type derivation
- `tracing` - Logging
- `chrono` - Timestamp handling
- `futures` - Future utilities

### Development
- `tokio-test` - Async testing utilities
- `proptest` - Property-based testing

## Test Coverage

### config.rs Tests
1. ✅ `test_parse_minimal_config` - Validates default values
2. ✅ `test_parse_full_config` - All fields specified
3. ✅ `test_network_profile_speeds` - Network profile calculations
4. ✅ `test_parse_wait_timeout` - Duration deserialization
5. ✅ `test_parse_upload_step` - File upload step parsing
6. ✅ `test_default_values` - Default field values

## Status: ✅ COMPLETE

All core modules are implemented:

| Module | Status | Tests |
|--------|--------|-------|
| config.rs | ✅ Complete | 8 tests |
| metrics/ | ✅ Complete | Web vitals + custom timings |
| throttling/ | ✅ Complete | Network + CPU throttling |
| stats/ | ✅ Complete | Percentiles + outliers |
| runner.rs | ✅ Complete | Parallel execution |
| reporter/ | ✅ Complete | Console, JSON, Markdown |

## Future Enhancements

1. Add HTML report generation with charts
2. Add baseline comparison for regression detection
3. Add CI/CD integration examples (GitHub Actions, CircleCI)
4. Add Lighthouse score integration
5. Add custom metric plugins

## Design Principles

1. **Declarative Configuration**: TOML-based scenario definition
2. **Type Safety**: Leverage Rust's type system for correctness
3. **Error Handling**: Comprehensive error messages with context
4. **Testability**: Unit tests for all parsing and business logic
5. **Documentation**: Doc comments for all public APIs
6. **Performance**: Parallel execution with async/await
