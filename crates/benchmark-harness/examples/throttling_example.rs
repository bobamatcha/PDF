//! Example demonstrating network and CPU throttling
//!
//! This example shows how to use the throttling module to simulate
//! different network conditions and CPU performance levels.
//!
//! Run with:
//! ```bash
//! cargo run --example throttling_example
//! ```

use anyhow::Result;
use benchmark_harness::throttling::{CpuThrottler, NetworkProfile, NetworkThrottler};
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Launch browser
    let config = BrowserConfig::builder()
        .build()
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let (mut browser, mut handler) = Browser::launch(config).await?;

    // Spawn a task to handle browser events
    let _handle = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(e) = event {
                eprintln!("Browser handler error: {}", e);
            }
        }
    });

    // Create a new page
    let page = browser.new_page("about:blank").await?;

    println!("=== Network Throttling Demo ===\n");

    // Example 1: Apply Fast 3G throttling
    println!("1. Applying Fast 3G throttling...");
    NetworkThrottler::apply(&page, NetworkProfile::Fast3G).await?;
    println!("   Fast 3G applied: 1.6 Mbps down, 750 Kbps up, 562ms latency\n");

    // Example 2: Apply Slow 4G throttling
    println!("2. Applying Slow 4G throttling...");
    NetworkThrottler::apply(&page, NetworkProfile::Slow4G).await?;
    println!("   Slow 4G applied: 4.0 Mbps down, 3.0 Mbps up, 20ms latency\n");

    // Example 3: Simulate offline mode
    println!("3. Applying Offline mode...");
    NetworkThrottler::apply(&page, NetworkProfile::Offline).await?;
    println!("   Offline mode: No network connectivity\n");

    // Example 4: Clear network throttling
    println!("4. Clearing network throttling...");
    NetworkThrottler::clear(&page).await?;
    println!("   Network throttling cleared: Full speed\n");

    println!("=== CPU Throttling Demo ===\n");

    // Example 5: Apply 4x CPU slowdown (mid-tier mobile)
    println!("5. Applying 4x CPU slowdown (mid-tier mobile)...");
    CpuThrottler::apply(&page, 4.0).await?;
    println!("   CPU throttled: 4x slower than normal\n");

    // Example 6: Apply 6x CPU slowdown (low-end mobile)
    println!("6. Applying 6x CPU slowdown (low-end mobile)...");
    CpuThrottler::apply(&page, 6.0).await?;
    println!("   CPU throttled: 6x slower than normal\n");

    // Example 7: Clear CPU throttling
    println!("7. Clearing CPU throttling...");
    CpuThrottler::clear(&page).await?;
    println!("   CPU throttling cleared: Full speed\n");

    println!("=== Combined Throttling Demo ===\n");

    // Example 8: Apply both network and CPU throttling together
    println!("8. Applying combined throttling (Fast 3G + 4x CPU)...");
    NetworkThrottler::apply(&page, NetworkProfile::Fast3G).await?;
    CpuThrottler::apply(&page, 4.0).await?;
    println!("   Combined throttling applied:");
    println!("   - Network: Fast 3G (1.6 Mbps down, 750 Kbps up, 562ms latency)");
    println!("   - CPU: 4x slowdown\n");

    // Navigate to a test page with throttling enabled
    println!("9. Navigating to example.com with throttling...");
    page.goto("https://example.com").await?;
    println!("   Page loaded (with throttling applied)\n");

    // Clear all throttling
    println!("10. Clearing all throttling...");
    NetworkThrottler::clear(&page).await?;
    CpuThrottler::clear(&page).await?;
    println!("    All throttling cleared\n");

    println!("=== Demo Complete ===");

    // Close browser
    browser.close().await?;

    Ok(())
}
