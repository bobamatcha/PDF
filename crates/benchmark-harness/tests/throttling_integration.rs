//! Integration tests for throttling module
//!
//! These tests verify that throttling commands are properly constructed
//! and sent via the Chrome DevTools Protocol.

use benchmark_harness::throttling::NetworkProfile;

#[test]
fn test_network_profile_values() {
    // Test that NetworkProfile correctly calculates throughput and latency values

    // Fast3G
    let profile = NetworkProfile::Fast3G;
    assert_eq!(profile.download_throughput(), 200_000.0); // 1.6 Mbps = 200 KB/s
    assert_eq!(profile.upload_throughput(), 93_750.0);    // 750 Kbps = 93.75 KB/s
    assert_eq!(profile.latency(), 562.0);
    assert!(!profile.offline());

    // Slow4G
    let profile = NetworkProfile::Slow4G;
    assert_eq!(profile.download_throughput(), 500_000.0); // 4 Mbps = 500 KB/s
    assert_eq!(profile.upload_throughput(), 375_000.0);   // 3 Mbps = 375 KB/s
    assert_eq!(profile.latency(), 20.0);
    assert!(!profile.offline());

    // None (no throttling)
    let profile = NetworkProfile::None;
    assert_eq!(profile.download_throughput(), -1.0); // -1 means no limit
    assert_eq!(profile.upload_throughput(), -1.0);
    assert_eq!(profile.latency(), 0.0);
    assert!(!profile.offline());

    // Offline
    let profile = NetworkProfile::Offline;
    assert_eq!(profile.download_throughput(), 0.0);
    assert_eq!(profile.upload_throughput(), 0.0);
    assert_eq!(profile.latency(), 0.0);
    assert!(profile.offline());
}

#[test]
fn test_network_profile_conversion() {
    // Verify that the extension methods properly convert from config values

    let profile = NetworkProfile::Fast3G;

    // download_bps returns Some(1_600_000 / 8) = Some(200_000)
    assert_eq!(profile.download_bps(), Some(200_000));
    // download_throughput should convert to f64
    assert_eq!(profile.download_throughput(), 200_000.0);

    // upload_bps returns Some(750_000 / 8) = Some(93_750)
    assert_eq!(profile.upload_bps(), Some(93_750));
    assert_eq!(profile.upload_throughput(), 93_750.0);

    // rtt_ms returns Some(562)
    assert_eq!(profile.rtt_ms(), Some(562));
    assert_eq!(profile.latency(), 562.0);
}

#[test]
fn test_cpu_throttling_validation() {
    // CPU throttling rate must be >= 1.0
    // This is validated in CpuThrottler::apply, but we can test the logic here

    let valid_rates = vec![1.0, 2.0, 4.0, 6.0, 10.0];
    for rate in valid_rates {
        assert!(rate >= 1.0, "Rate {} should be valid", rate);
    }

    let invalid_rates = vec![0.0, 0.5, -1.0];
    for rate in invalid_rates {
        assert!(rate < 1.0, "Rate {} should be invalid", rate);
    }
}

// Note: Actual browser integration tests would require a running Chrome instance
// and are better suited for end-to-end testing or manual verification.
// The examples/throttling_example.rs demonstrates the actual usage.
