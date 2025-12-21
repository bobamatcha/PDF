//! Network throttling via Chrome DevTools Protocol
//!
//! Applies network throttling to simulate different network conditions
//! using the Network.emulateNetworkConditions CDP command.

#![allow(deprecated)] // EmulateNetworkConditionsParams is deprecated but still functional

use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network::{
    ConnectionType, EmulateNetworkConditionsParams,
};
use chromiumoxide::Page;
use tracing::{debug, instrument};

// Re-export NetworkProfile from config
pub use crate::config::NetworkProfile;

impl NetworkProfile {
    /// Download throughput in bytes per second (-1 means no throttling)
    pub fn download_throughput(&self) -> f64 {
        match self.download_bps() {
            Some(bps) => bps as f64,
            None => -1.0, // -1 means no throttling in Chrome DevTools Protocol
        }
    }

    /// Upload throughput in bytes per second (-1 means no throttling)
    pub fn upload_throughput(&self) -> f64 {
        match self.upload_bps() {
            Some(bps) => bps as f64,
            None => -1.0, // -1 means no throttling in Chrome DevTools Protocol
        }
    }

    /// Network latency in milliseconds
    pub fn latency(&self) -> f64 {
        match self.rtt_ms() {
            Some(rtt) => rtt as f64,
            None => 0.0,
        }
    }

    /// Whether the network is completely offline
    pub fn offline(&self) -> bool {
        matches!(self, NetworkProfile::Offline)
    }
}

/// Network throttling controller
pub struct NetworkThrottler;

impl NetworkThrottler {
    /// Apply network throttling to a page using Network.emulateNetworkConditions
    ///
    /// # Arguments
    ///
    /// * `page` - The chromiumoxide Page to apply throttling to
    /// * `profile` - The network profile to apply
    ///
    /// # Example
    ///
    /// ```no_run
    /// use benchmark_harness::throttling::{NetworkThrottler, NetworkProfile};
    /// # use chromiumoxide::Page;
    ///
    /// # async fn example(page: &Page) -> anyhow::Result<()> {
    /// NetworkThrottler::apply(page, NetworkProfile::Fast3G).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(page), fields(profile = ?profile))]
    pub async fn apply(page: &Page, profile: NetworkProfile) -> Result<()> {
        debug!(
            "Applying network throttling: offline={}, latency={}ms, down={:.2} KB/s, up={:.2} KB/s",
            profile.offline(),
            profile.latency(),
            profile.download_throughput() / 1024.0,
            profile.upload_throughput() / 1024.0
        );

        let params = EmulateNetworkConditionsParams::builder()
            .offline(profile.offline())
            .latency(profile.latency())
            .download_throughput(profile.download_throughput())
            .upload_throughput(profile.upload_throughput())
            .connection_type(ConnectionType::Cellular4g) // Generic type for throttled connections
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build network params: {}", e))?;

        page.execute(params).await?;

        debug!("Network throttling applied successfully");
        Ok(())
    }

    /// Remove network throttling by setting all values to no-throttling defaults
    ///
    /// # Arguments
    ///
    /// * `page` - The chromiumoxide Page to clear throttling from
    ///
    /// # Example
    ///
    /// ```no_run
    /// use benchmark_harness::throttling::NetworkThrottler;
    /// # use chromiumoxide::Page;
    ///
    /// # async fn example(page: &Page) -> anyhow::Result<()> {
    /// NetworkThrottler::clear(page).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(page))]
    pub async fn clear(page: &Page) -> Result<()> {
        debug!("Clearing network throttling");

        let params = EmulateNetworkConditionsParams::builder()
            .offline(false)
            .latency(0.0)
            .download_throughput(-1.0) // -1 means no throttling
            .upload_throughput(-1.0) // -1 means no throttling
            .connection_type(ConnectionType::None)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build network params: {}", e))?;

        page.execute(params).await?;

        debug!("Network throttling cleared successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_profile_none() {
        let profile = NetworkProfile::None;
        assert_eq!(profile.download_throughput(), -1.0);
        assert_eq!(profile.upload_throughput(), -1.0);
        assert_eq!(profile.latency(), 0.0);
        assert!(!profile.offline());
    }

    #[test]
    fn test_network_profile_fast3g() {
        let profile = NetworkProfile::Fast3G;
        // From config.rs: 1_600_000 / 8 = 200_000 bytes/sec
        assert_eq!(profile.download_throughput(), 200_000.0);
        // From config.rs: 750_000 / 8 = 93_750 bytes/sec
        assert_eq!(profile.upload_throughput(), 93_750.0);
        // From config.rs: 562ms RTT
        assert_eq!(profile.latency(), 562.0);
        assert!(!profile.offline());
    }

    #[test]
    fn test_network_profile_slow4g() {
        let profile = NetworkProfile::Slow4G;
        // From config.rs: 4_000_000 / 8 = 500_000 bytes/sec
        assert_eq!(profile.download_throughput(), 500_000.0);
        // From config.rs: 3_000_000 / 8 = 375_000 bytes/sec
        assert_eq!(profile.upload_throughput(), 375_000.0);
        // From config.rs: 20ms RTT
        assert_eq!(profile.latency(), 20.0);
        assert!(!profile.offline());
    }

    #[test]
    fn test_network_profile_offline() {
        let profile = NetworkProfile::Offline;
        assert_eq!(profile.download_throughput(), 0.0);
        assert_eq!(profile.upload_throughput(), 0.0);
        assert_eq!(profile.latency(), 0.0);
        assert!(profile.offline());
    }
}
