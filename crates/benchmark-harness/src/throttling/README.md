# Throttling Module

Network and CPU throttling via Chrome DevTools Protocol for benchmark testing.

## Overview

This module provides utilities to throttle network and CPU performance using Chrome DevTools Protocol (CDP) commands. This is useful for benchmarking under constrained conditions that simulate slower devices or poor network connectivity.

## Components

### Network Throttling (`network.rs`)

Simulates different network conditions using the `Network.emulateNetworkConditions` CDP command.

**Supported Profiles:**
- `NetworkProfile::None` - No throttling (full speed)
- `NetworkProfile::Fast3G` - 1.6 Mbps down, 750 Kbps up, 562ms latency
- `NetworkProfile::Slow4G` - 4.0 Mbps down, 3.0 Mbps up, 20ms latency
- `NetworkProfile::Offline` - No network connectivity

**API:**
```rust
use benchmark_harness::throttling::{NetworkThrottler, NetworkProfile};

// Apply Fast 3G throttling
NetworkThrottler::apply(&page, NetworkProfile::Fast3G).await?;

// Clear throttling
NetworkThrottler::clear(&page).await?;
```

### CPU Throttling (`cpu.rs`)

Simulates slower CPU performance using the `Emulation.setCPUThrottlingRate` CDP command.

**Throttling Rates:**
- `1.0` - No throttling (full speed)
- `2.0` - 2x slowdown
- `4.0` - 4x slowdown (typical mid-tier mobile device)
- `6.0` - 6x slowdown (low-end mobile device)

**API:**
```rust
use benchmark_harness::throttling::CpuThrottler;

// Apply 4x CPU slowdown
CpuThrottler::apply(&page, 4.0).await?;

// Clear throttling
CpuThrottler::clear(&page).await?;
```

## Usage Example

```rust
use benchmark_harness::throttling::{CpuThrottler, NetworkProfile, NetworkThrottler};
use chromiumoxide::Page;

async fn benchmark_with_throttling(page: &Page) -> anyhow::Result<()> {
    // Apply combined throttling to simulate a mid-tier mobile device on 3G
    NetworkThrottler::apply(page, NetworkProfile::Fast3G).await?;
    CpuThrottler::apply(page, 4.0).await?;

    // Run your benchmark tests here
    page.goto("https://example.com").await?;

    // Measure performance metrics...

    // Clear throttling when done
    NetworkThrottler::clear(page).await?;
    CpuThrottler::clear(page).await?;

    Ok(())
}
```

## Configuration Integration

The throttling module integrates with the benchmark configuration system:

```toml
[throttling]
network_profile = "Fast3G"
cpu_slowdown = 4.0
```

The `NetworkProfile` enum is defined in `config.rs` and re-exported by the throttling module for convenience.

## Implementation Details

### Chrome DevTools Protocol Commands

- **Network Throttling:** Uses `Network.emulateNetworkConditions`
  - Parameters: `offline`, `latency`, `downloadThroughput`, `uploadThroughput`
  - Throughput in bytes/second (-1 means no limit)
  - Latency in milliseconds

- **CPU Throttling:** Uses `Emulation.setCPUThrottlingRate`
  - Parameter: `rate` (1.0 = no throttling, higher = more throttling)

### Chromiumoxide Integration

The module uses `chromiumoxide`'s CDP builder API to send protocol commands:

```rust
use chromiumoxide::cdp::browser_protocol::network::EmulateNetworkConditionsParams;

let params = EmulateNetworkConditionsParams::builder()
    .offline(false)
    .latency(150.0)
    .download_throughput(200_000.0)
    .upload_throughput(93_750.0)
    .build();

page.execute(params).await?;
```

## Testing

Unit tests verify the profile calculations and value conversions. See:
- `src/throttling/network.rs` - Network profile tests
- `src/throttling/cpu.rs` - CPU throttling validation tests
- `tests/throttling_integration.rs` - Integration tests

Run tests with:
```bash
cargo test --package benchmark-harness
```

## Examples

See `examples/throttling_example.rs` for a complete demonstration:

```bash
cargo run --example throttling_example
```

## Error Handling

Both throttlers return `anyhow::Result<()>` and will propagate CDP errors. Common error scenarios:

- Browser not connected
- Invalid throttling parameters
- CDP command execution failures

All operations include tracing spans for debugging.
