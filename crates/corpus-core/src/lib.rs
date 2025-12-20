//! Corpus Core - Domain types and core functionality for the semantic document corpus
//!
//! This crate provides:
//! - Document and Template types
//! - Verification traits and error types
//! - Search types and scoring logic
//! - Storage abstractions (LanceDB)
//! - Embedding model integration (Candle)
//! - Configuration management

pub mod document;
pub mod verifier;
pub mod search;
pub mod storage;
pub mod config;
pub mod embeddings;

// Re-export commonly used types
pub use document::{Document, DocumentMetadata, Template, TemplateVariable, VariableType};
pub use verifier::{Verifier, VerificationResult, VerificationError, CompositeVerifier};
pub use search::{SearchQuery, SearchResult, SearchResponse, MatchType, SearchSuggestion};
pub use config::StorageConfig;
