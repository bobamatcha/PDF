//! Search module - Vector search, keyword search, and hybrid search implementation
//!
//! This module provides:
//! - Vector search using embeddings
//! - Keyword search using Tantivy (BM25)
//! - Reciprocal Rank Fusion for hybrid search
//! - Semantic threshold gating for confidence routing

pub mod vector;
pub mod keyword;
pub mod fusion;
pub mod gating;

pub use vector::VectorSearch;
pub use keyword::KeywordIndex;
pub use fusion::RankFusion;
pub use gating::SemanticGate;

use serde::{Deserialize, Serialize};

// Search confidence thresholds
pub const HIGH_CONFIDENCE_THRESHOLD: f32 = 0.85;
pub const LOW_CONFIDENCE_THRESHOLD: f32 = 0.75;

/// Search query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub filters: Option<SearchFilters>,
    pub limit: usize,
}

/// Optional search filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub date_range: Option<(i64, i64)>,
}

/// Individual search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document_id: String,
    pub score: f32,
    pub match_type: MatchType,
    pub snippet: String,
}

/// Type of match based on confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchType {
    DirectMatch,   // score >= 0.85
    SimilarMatch,  // 0.75 <= score < 0.85
    WeakMatch,     // score < 0.75
}

impl From<f32> for MatchType {
    fn from(score: f32) -> Self {
        if score >= HIGH_CONFIDENCE_THRESHOLD {
            MatchType::DirectMatch
        } else if score >= LOW_CONFIDENCE_THRESHOLD {
            MatchType::SimilarMatch
        } else {
            MatchType::WeakMatch
        }
    }
}

/// Complete search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total_matches: usize,
    pub suggestion: Option<SearchSuggestion>,
}

/// Search suggestion for handling low-confidence queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchSuggestion {
    None,  // high confidence match found
    GenerateCustom {
        message: String,
        anchor_documents: Vec<String>,
    },  // suggest RAG generation
}
