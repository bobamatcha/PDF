//! Vector search implementation using semantic embeddings
//!
//! This module provides semantic vector search capabilities using BGE-M3 embeddings
//! stored in LanceDB. Vector search enables finding semantically similar documents
//! even when exact keyword matches don't exist.
//!
//! # Architecture
//!
//! The vector search pipeline:
//! 1. Generate query embedding using the BGE-M3 model
//! 2. Perform cosine similarity search in LanceDB
//! 3. Classify match quality based on similarity scores
//! 4. Return ranked results with snippets
//!
//! # Match Type Classification
//!
//! Results are classified into three categories:
//! - **DirectMatch** (score >= 0.85): High-confidence semantic match
//! - **SimilarMatch** (0.75 <= score < 0.85): Related but not exact
//! - **WeakMatch** (score < 0.75): Low confidence, may trigger RAG generation
//!
//! # Example
//!
//! ```rust,no_run
//! use corpus_core::search::vector::VectorSearch;
//! use corpus_core::storage::CorpusStorage;
//! use corpus_core::embeddings::EmbeddingModel;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let storage = Arc::new(CorpusStorage::connect("s3://bucket", "documents").await?);
//! let model = Arc::new(EmbeddingModel::load(&std::path::Path::new("./models/bge-m3")).await?);
//!
//! let search = VectorSearch::new(storage, model);
//! let results = search.search("contract template for freelancer", 10).await?;
//!
//! for result in results {
//!     println!("Doc: {} (score: {:.3}, type: {:?})",
//!              result.document_id, result.score, result.match_type);
//! }
//! # Ok(())
//! # }
//! ```

use crate::storage::CorpusStorage;
use crate::search::{SearchResult, SearchFilters};
use anyhow::Result;
use std::sync::Arc;

/// Vector search engine using semantic embeddings
///
/// This struct provides the core vector search functionality using
/// LanceDB storage for efficient similarity search.
///
/// Vector search is particularly useful for:
/// - Finding semantically similar documents
/// - Cross-lingual search (BGE-M3 supports multiple languages)
/// - Fuzzy matching when exact keywords aren't known
/// - Discovering related documents by concept
pub struct VectorSearch {
    /// Reference to the LanceDB storage layer
    storage: Arc<CorpusStorage>,
}

impl VectorSearch {
    /// Create a new vector search instance
    ///
    /// # Arguments
    ///
    /// * `storage` - Shared reference to corpus storage (LanceDB)
    ///
    /// # Returns
    ///
    /// A new `VectorSearch` instance ready to process queries
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use corpus_core::search::vector::VectorSearch;
    /// # use corpus_core::storage::CorpusStorage;
    /// # use std::sync::Arc;
    /// # async fn example() -> anyhow::Result<()> {
    /// let storage = Arc::new(CorpusStorage::connect("s3://bucket", "docs").await?);
    /// let search = VectorSearch::new(storage);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(storage: Arc<CorpusStorage>) -> Self {
        Self { storage }
    }

    /// Perform semantic vector search using pre-computed embeddings
    ///
    /// This method:
    /// 1. Searches LanceDB for the most similar document vectors
    /// 2. Classifies results by match quality (Direct/Similar/Weak)
    /// 3. Extracts content snippets for preview
    ///
    /// # Arguments
    ///
    /// * `query_embedding` - Pre-computed 1024-dimensional embedding vector
    /// * `limit` - Maximum number of results to return
    /// * `filters` - Optional search filters (tags, author, date range)
    ///
    /// # Returns
    ///
    /// A vector of `SearchResult` objects, sorted by descending similarity score.
    /// Each result includes:
    /// - Document ID for retrieval
    /// - Similarity score (0.0 to 1.0)
    /// - Match type classification
    /// - Content snippet (first 200 characters)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - LanceDB search operation fails
    /// - Storage layer is unavailable
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use corpus_core::search::vector::VectorSearch;
    /// # use corpus_core::storage::CorpusStorage;
    /// # use std::sync::Arc;
    /// # async fn example() -> anyhow::Result<()> {
    /// # let storage = Arc::new(CorpusStorage::connect("s3://bucket", "docs").await?);
    /// # let search = VectorSearch::new(storage);
    /// let query_embedding = vec![0.0f32; 1024]; // Pre-computed embedding
    /// let results = search.search(&query_embedding, 5, None).await?;
    ///
    /// for result in results {
    ///     match result.match_type {
    ///         corpus_core::search::MatchType::DirectMatch => {
    ///             println!("Exact match found: {}", result.document_id);
    ///         }
    ///         corpus_core::search::MatchType::SimilarMatch => {
    ///             println!("Similar document: {}", result.document_id);
    ///         }
    ///         corpus_core::search::MatchType::WeakMatch => {
    ///             println!("Weak match: {}", result.document_id);
    ///         }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        filters: Option<&SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        // Use filtered vector search from storage
        self.storage.vector_search_filtered(query_embedding, limit, filters).await
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_search_construction() {
        // Test that VectorSearch can be constructed with mock components
        // Actual functionality tests would require integration testing
        // with a live LanceDB instance and loaded embedding model
    }

    #[test]
    fn test_embedding_dimension() {
        // BGE-M3 produces 1024-dimensional embeddings
        // This is verified in the embeddings module tests
    }
}
