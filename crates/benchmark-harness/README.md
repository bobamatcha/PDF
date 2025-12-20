# Benchmark Harness

A comprehensive benchmarking framework for measuring web application performance metrics, including Core Web Vitals (LCP, INP, CLS).

## Features

- **TOML-based Configuration**: Define benchmark scenarios declaratively
- **Core Web Vitals**: Automated collection of LCP, INP, and CLS metrics
- **Network Throttling**: Simulate Fast 3G, Slow 4G, and offline conditions
- **CPU Throttling**: Test performance on slower devices
- **Parallel Execution**: Run multiple browser contexts concurrently
- **Statistical Analysis**: P50, P95, P99 percentiles with outlier detection
- **Flexible Reporting**: JSON, text, and HTML output formats

## Quick Start

### 1. Create a benchmark configuration

Create a `benchmark.toml` file:

```toml
[benchmark]
name = "My App Performance Tests"
base_url = "https://myapp.com"
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
name = "Homepage Load"
steps = [
    { type = "navigate", url = "/" },
    { type = "wait", wait_for = "network_idle" },
    { type = "measure" }
]
```

### 2. Load and parse the configuration

```rust
use benchmark_harness::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_file("benchmark.toml")?;
    println!("Loaded benchmark: {}", config.benchmark.name);
    Ok(())
}
```

## Configuration Reference

### Benchmark Settings

```toml
[benchmark]
name = "Test Suite Name"          # Required
base_url = "https://example.com"   # Required
iterations = 30                    # Optional (default: 30)
warmup = 3                         # Optional (default: 3)
parallel_contexts = 4              # Optional (default: 4)
```

### Throttling Settings

```toml
[throttling]
network_profile = "Fast3G"  # "Fast3G" | "Slow4G" | "Offline" | "None"
cpu_slowdown = 4.0          # 1.0 = no slowdown, 4.0 = 4x slower
```

Network profiles:
- **Fast3G**: 1.6 Mbps down, 750 Kbps up, 562.5ms RTT
- **Slow4G**: 4 Mbps down, 3 Mbps up, 20ms RTT
- **Offline**: No network access
- **None**: No throttling (default)

### Performance Thresholds

```toml
[thresholds]
lcp_p95 = 2500.0  # Largest Contentful Paint (ms)
inp_p95 = 200.0   # Interaction to Next Paint (ms)
cls_p95 = 0.1     # Cumulative Layout Shift (score)
```

All thresholds are optional. If specified, they're used for pass/fail determination.

### Scenarios and Steps

#### Navigate

Navigate to a URL (relative to `base_url` or absolute):

```toml
{ type = "navigate", url = "/login" }
```

#### Wait

Wait for various conditions:

```toml
# Wait for network to be idle
{ type = "wait", wait_for = "network_idle" }

# Wait for a CSS selector
{ type = "wait", wait_for = "selector", selector = "#content" }

# Wait for a specific duration (milliseconds)
{ type = "wait", wait_for = "timeout", duration = 5000 }
```

#### Click

Click an element:

```toml
{ type = "click", selector = "#submit-button" }
```

#### Type

Type text into an input field:

```toml
{ type = "type", selector = "#username", text = "test@example.com" }
```

#### Upload

Upload a file:

```toml
{ type = "upload", selector = "#file-input", file_path = "/path/to/file.pdf" }
```

#### Measure

Capture performance metrics at a specific point:

```toml
# Basic measurement
{ type = "measure" }

# Labeled measurement
{ type = "measure", label = "post-login" }
```

## Example Scenarios

### PDF Upload Flow

```toml
[[scenarios]]
name = "PDF Upload"
steps = [
    { type = "navigate", url = "/" },
    { type = "wait", wait_for = "selector", selector = "#upload-button" },
    { type = "click", selector = "#upload-button" },
    { type = "upload", selector = "#file-input", file_path = "/tmp/test.pdf" },
    { type = "wait", wait_for = "selector", selector = "#processing-complete" },
    { type = "measure", label = "upload-complete" }
]
```

### Login and Search

```toml
[[scenarios]]
name = "Login and Search"
steps = [
    { type = "navigate", url = "/login" },
    { type = "type", selector = "#username", text = "user@example.com" },
    { type = "type", selector = "#password", text = "password123" },
    { type = "click", selector = "#submit" },
    { type = "wait", wait_for = "network_idle" },
    { type = "navigate", url = "/search" },
    { type = "type", selector = "#query", text = "test query" },
    { type = "click", selector = "#search-button" },
    { type = "wait", wait_for = "selector", selector = ".results" },
    { type = "measure", label = "search-results" }
]
```

## Module Structure

- **config**: TOML configuration parsing and validation
- **metrics**: Core Web Vitals and custom timing collection
- **throttling**: Network and CPU throttling via Chrome DevTools Protocol
- **stats**: Statistical analysis (percentiles, outliers, confidence intervals)
- **runner**: Benchmark execution orchestration
- **reporter**: Result formatting and output

## License

See the workspace LICENSE file for details.
