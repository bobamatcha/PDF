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
let config = Config::from_str(toml_content)?;

// Access network profile speeds
let profile = NetworkProfile::Fast3G;
println!("Download: {} bps", profile.download_bps().unwrap());
println!("Upload: {} bps", profile.upload_bps().unwrap());
println!("RTT: {} ms", profile.rtt_ms().unwrap());
```

### metrics/ (TODO)

**Purpose**: Collect Core Web Vitals and custom timing metrics

**Planned modules**:
- `web_vitals.rs` - LCP, INP, CLS collection via web-vitals.js
- `custom.rs` - User Timing API measurements

### throttling/ (PARTIALLY IMPLEMENTED)

**Purpose**: Network and CPU throttling via Chrome DevTools Protocol

**Status**: Module structure exists with placeholder implementations

### stats/ (PARTIALLY IMPLEMENTED)

**Purpose**: Statistical analysis of benchmark results

**Existing modules**:
- `percentiles.rs` - P50, P95, P99 calculations
- `outliers.rs` - Outlier detection

### runner.rs (TODO)

**Purpose**: Orchestrate benchmark execution

**Planned features**:
- Browser context management
- Parallel scenario execution
- Warmup runs
- Metric collection coordination

### reporter/ (TODO)

**Purpose**: Format and output benchmark results

**Planned formats**:
- JSON output
- Human-readable text
- HTML reports

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

## Next Steps

1. Implement `metrics/` module for Core Web Vitals collection
2. Implement `runner.rs` for benchmark orchestration
3. Implement `stats/` statistical analysis functions
4. Implement `reporter/` for result formatting
5. Add integration tests with real browser automation
6. Add property-based tests for configuration validation

## Design Principles

1. **Declarative Configuration**: TOML-based scenario definition
2. **Type Safety**: Leverage Rust's type system for correctness
3. **Error Handling**: Comprehensive error messages with context
4. **Testability**: Unit tests for all parsing and business logic
5. **Documentation**: Doc comments for all public APIs
6. **Performance**: Parallel execution with async/await
