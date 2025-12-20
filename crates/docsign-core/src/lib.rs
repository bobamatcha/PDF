//! Document signing core logic
//!
//! This crate provides the core document signing functionality,
//! including PDF signing, audit trails, and session management.
//!
//! Most functionality is currently in the docsign-wasm crate and
//! will be migrated here incrementally.

// Re-export types from shared crates
pub use shared_crypto::{cert, cms, keys, tsa, EphemeralIdentity, SigningIdentity};
pub use shared_pdf::{dom_to_pdf, parser, pdf_to_dom, signer, PdfDocument};
