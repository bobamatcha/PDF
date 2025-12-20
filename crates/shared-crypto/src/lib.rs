//! Shared cryptography utilities
//!
//! This crate provides cryptographic primitives for digital signatures,
//! certificates, and timestamping.

pub mod cert;
pub mod cms;
pub mod keys;
pub mod tsa;

pub use keys::{EphemeralIdentity, SigningIdentity};
