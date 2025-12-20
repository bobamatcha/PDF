//! Typst document rendering engine
//!
//! This crate provides Typst compilation and document verification
//! functionality, including:
//! - Document rendering to PDF/SVG/PNG
//! - Florida lease verification
//! - Template management

pub mod compiler;
pub mod templates;
pub mod verifier;
pub mod world;

pub use compiler::{RenderRequest, RenderResponse};
