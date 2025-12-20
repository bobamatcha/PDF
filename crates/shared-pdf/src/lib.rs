//! Shared PDF handling utilities
//!
//! This crate provides common PDF parsing, coordinate transformation,
//! and manipulation functionality used across the monolith.

pub mod audit;
pub mod coords;
pub mod parser;
pub mod signer;

pub use coords::{dom_to_pdf, pdf_to_dom};
pub use parser::PdfDocument;
