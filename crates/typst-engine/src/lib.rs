//! Typst document rendering engine
//!
//! This crate provides Typst compilation and document verification
//! functionality, including:
//! - Document rendering to PDF/SVG/PNG (sync and async)
//! - Florida lease verification
//! - Template management
//!
//! # Feature Flags
//!
//! - `server` (default): Enables async `compile_document` with timeout (requires tokio)
//! - `wasm`: For browser/WASM environments (use `compile_document_sync`)

pub mod compiler;
pub mod templates;
pub mod verifier;
pub mod world;

// Always export sync version (WASM-compatible)
pub use compiler::{compile_document_sync, RenderRequest, RenderResponse};

// Export async version only with server feature
#[cfg(feature = "server")]
pub use compiler::compile_document;
