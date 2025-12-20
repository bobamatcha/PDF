//! Network and CPU throttling via Chrome DevTools Protocol
//!
//! This module provides utilities to throttle network and CPU performance
//! using Chrome DevTools Protocol (CDP) commands. This is useful for
//! benchmarking under constrained conditions that simulate slower devices
//! or poor network connectivity.

pub mod cpu;
pub mod network;

pub use cpu::CpuThrottler;
pub use network::NetworkThrottler;

// Re-export NetworkProfile from config for convenience
pub use crate::config::NetworkProfile;
