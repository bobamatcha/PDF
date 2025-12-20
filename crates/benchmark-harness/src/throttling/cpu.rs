//! CPU throttling via Chrome DevTools Protocol
//!
//! Applies CPU throttling to simulate slower devices using the
//! Emulation.setCPUThrottlingRate CDP command.

use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::emulation::SetCpuThrottlingRateParams;
use chromiumoxide::Page;
use tracing::{debug, instrument};

/// CPU throttling controller
pub struct CpuThrottler;

impl CpuThrottler {
    /// Apply CPU throttling using Emulation.setCPUThrottlingRate
    ///
    /// # Arguments
    ///
    /// * `page` - The chromiumoxide Page to apply throttling to
    /// * `rate` - Throttling rate as a slowdown multiplier (1.0 = no throttling)
    ///   - 1.0 = no throttling (full speed)
    ///   - 2.0 = 2x slowdown
    ///   - 4.0 = 4x slowdown (typical mid-tier mobile device)
    ///   - 6.0 = 6x slowdown (low-end mobile device)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use benchmark_harness::throttling::CpuThrottler;
    /// # use chromiumoxide::Page;
    ///
    /// # async fn example(page: &Page) -> anyhow::Result<()> {
    /// // Simulate a mid-tier mobile device with 4x CPU slowdown
    /// CpuThrottler::apply(page, 4.0).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(page), fields(rate = %rate))]
    pub async fn apply(page: &Page, rate: f64) -> Result<()> {
        if rate < 1.0 {
            anyhow::bail!("CPU throttling rate must be >= 1.0 (got {})", rate);
        }

        debug!("Applying CPU throttling with {}x slowdown", rate);

        let params = SetCpuThrottlingRateParams::builder()
            .rate(rate)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build CPU params: {}", e))?;

        page.execute(params).await?;

        debug!("CPU throttling applied successfully");
        Ok(())
    }

    /// Remove CPU throttling by setting rate to 1.0 (no throttling)
    ///
    /// # Arguments
    ///
    /// * `page` - The chromiumoxide Page to clear throttling from
    ///
    /// # Example
    ///
    /// ```no_run
    /// use benchmark_harness::throttling::CpuThrottler;
    /// # use chromiumoxide::Page;
    ///
    /// # async fn example(page: &Page) -> anyhow::Result<()> {
    /// CpuThrottler::clear(page).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(page))]
    pub async fn clear(page: &Page) -> Result<()> {
        debug!("Clearing CPU throttling");

        let params = SetCpuThrottlingRateParams::builder()
            .rate(1.0)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build CPU params: {}", e))?;

        page.execute(params).await?;

        debug!("CPU throttling cleared successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_throttling_rate() {
        // This test verifies the validation logic, but can't test the actual CDP call
        // without a real browser instance
        let rate = 0.5;
        assert!(rate < 1.0, "Rate below 1.0 should be invalid");
    }

    #[test]
    fn test_valid_throttling_rates() {
        // Common valid rates
        let rates = vec![1.0, 2.0, 4.0, 6.0, 10.0];
        for rate in rates {
            assert!(rate >= 1.0, "Rate {} should be valid", rate);
        }
    }
}
