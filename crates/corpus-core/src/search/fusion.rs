//! Reciprocal Rank Fusion (RRF) implementation for hybrid search
//!
//! This module implements Reciprocal Rank Fusion, a technique for combining ranked
//! lists from different retrieval systems (vector and keyword search) without
//! requiring score normalization.
//!
//! # Algorithm
//!
//! RRF assigns a score to each document based on its rank position in each result list:
//!
//! ```text
//! score(d) = sum over all rankers: 1 / (k + rank(d))
//! ```
//!
//! Where:
//! - `k` is a constant (typically 60) that controls the impact of rank position
//! - `rank(d)` is the 1-based rank of document d in a particular result list
//!
//! # Benefits
//!
//! - No need to normalize scores from different systems
//! - Robust to outliers in individual rankers
//! - Simple and effective for combining diverse retrieval systems
//!
//! # References
//!
//! Cormack, G. V., Clarke, C. L., & Buettcher, S. (2009).
//! "Reciprocal rank fusion outperforms condorcet and individual rank learning methods."
//! Proceedings of the 32nd international ACM SIGIR conference.

use std::collections::HashMap;
use crate::search::SearchResult;

/// Standard RRF constant (k=60 is widely used in literature)
const RRF_K: f32 = 60.0;

/// Reciprocal Rank Fusion for combining vector and keyword search results
pub struct RankFusion;

impl RankFusion {
    /// Combine vector and keyword results using Reciprocal Rank Fusion
    ///
    /// This method merges results from semantic/vector search and keyword/BM25 search
    /// using the RRF algorithm. Documents appearing in both result sets get boosted
    /// scores, while maintaining the relative ranking from each individual system.
    ///
    /// # Arguments
    ///
    /// * `vector_results` - Results from vector/semantic search with embeddings
    /// * `keyword_results` - Results from keyword/BM25 search as (doc_id, score) tuples
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Combined and re-ranked results, sorted by RRF score in descending order
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let vector_results = vec![
    ///     SearchResult { document_id: "doc1".to_string(), score: 0.9, ... },
    ///     SearchResult { document_id: "doc2".to_string(), score: 0.8, ... },
    /// ];
    ///
    /// let keyword_results = vec![
    ///     ("doc2".to_string(), 10.0),
    ///     ("doc3".to_string(), 8.0),
    /// ];
    ///
    /// let fused = RankFusion::fuse(vector_results, keyword_results, 10);
    /// // doc2 will rank highly as it appears in both lists
    /// ```
    pub fn fuse(
        vector_results: Vec<SearchResult>,
        keyword_results: Vec<(String, f32)>,
        limit: usize,
    ) -> Vec<SearchResult> {
        let mut scores: HashMap<String, f32> = HashMap::new();
        let mut result_map: HashMap<String, SearchResult> = HashMap::new();

        // Score from vector search (rank-based, not score-based)
        for (rank, result) in vector_results.iter().enumerate() {
            let rrf_score = 1.0 / (RRF_K + rank as f32 + 1.0);
            *scores.entry(result.document_id.clone()).or_insert(0.0) += rrf_score;
            result_map.insert(result.document_id.clone(), result.clone());
        }

        // Score from keyword search (rank-based, ignoring original scores)
        for (rank, (doc_id, _)) in keyword_results.iter().enumerate() {
            let rrf_score = 1.0 / (RRF_K + rank as f32 + 1.0);
            *scores.entry(doc_id.clone()).or_insert(0.0) += rrf_score;
        }

        // Sort by combined RRF score
        let mut combined: Vec<(String, f32)> = scores.into_iter().collect();
        combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top results with updated scores
        combined
            .into_iter()
            .take(limit)
            .filter_map(|(id, score)| {
                result_map.get(&id).map(|r| SearchResult {
                    score,  // Use RRF combined score
                    ..r.clone()
                })
            })
            .collect()
    }

    /// Fuse with custom k parameter
    ///
    /// Allows fine-tuning the RRF algorithm by adjusting the k constant.
    /// Lower k values increase the importance of top-ranked documents,
    /// while higher k values flatten the distribution.
    ///
    /// # Arguments
    ///
    /// * `vector_results` - Results from vector/semantic search
    /// * `keyword_results` - Results from keyword/BM25 search
    /// * `limit` - Maximum number of results to return
    /// * `k` - RRF constant (typically between 10 and 100)
    ///
    /// # Returns
    ///
    /// Combined and re-ranked results
    pub fn fuse_with_k(
        vector_results: Vec<SearchResult>,
        keyword_results: Vec<(String, f32)>,
        limit: usize,
        k: f32,
    ) -> Vec<SearchResult> {
        let mut scores: HashMap<String, f32> = HashMap::new();
        let mut result_map: HashMap<String, SearchResult> = HashMap::new();

        for (rank, result) in vector_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f32 + 1.0);
            *scores.entry(result.document_id.clone()).or_insert(0.0) += rrf_score;
            result_map.insert(result.document_id.clone(), result.clone());
        }

        for (rank, (doc_id, _)) in keyword_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f32 + 1.0);
            *scores.entry(doc_id.clone()).or_insert(0.0) += rrf_score;
        }

        let mut combined: Vec<(String, f32)> = scores.into_iter().collect();
        combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        combined
            .into_iter()
            .take(limit)
            .filter_map(|(id, score)| {
                result_map.get(&id).map(|r| SearchResult {
                    score,
                    ..r.clone()
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::MatchType;

    #[test]
    fn test_rrf_fusion() {
        let vector_results = vec![
            SearchResult {
                document_id: "doc1".to_string(),
                score: 0.9,
                match_type: MatchType::DirectMatch,
                snippet: "test".to_string(),
            },
            SearchResult {
                document_id: "doc2".to_string(),
                score: 0.8,
                match_type: MatchType::SimilarMatch,
                snippet: "test".to_string(),
            },
        ];

        let keyword_results = vec![
            ("doc2".to_string(), 10.0),
            ("doc1".to_string(), 8.0),
            ("doc3".to_string(), 5.0),
        ];

        let fused = RankFusion::fuse(vector_results, keyword_results, 10);

        // doc2 should rank higher because it appears high in both lists
        assert!(!fused.is_empty());
        assert_eq!(fused[0].document_id, "doc2");
    }

    #[test]
    fn test_rrf_score_calculation() {
        // Test with single document in both lists
        let vector_results = vec![
            SearchResult {
                document_id: "doc1".to_string(),
                score: 0.95,
                match_type: MatchType::DirectMatch,
                snippet: "test".to_string(),
            },
        ];

        let keyword_results = vec![
            ("doc1".to_string(), 15.0),
        ];

        let fused = RankFusion::fuse(vector_results, keyword_results, 10);

        assert_eq!(fused.len(), 1);
        // RRF score for doc1: 1/(60+1) + 1/(60+1) = 2/61 ≈ 0.0328
        assert!((fused[0].score - (2.0 / 61.0)).abs() < 0.001);
    }

    #[test]
    fn test_rrf_limit() {
        let vector_results = vec![
            SearchResult {
                document_id: "doc1".to_string(),
                score: 0.9,
                match_type: MatchType::DirectMatch,
                snippet: "test".to_string(),
            },
            SearchResult {
                document_id: "doc2".to_string(),
                score: 0.8,
                match_type: MatchType::SimilarMatch,
                snippet: "test".to_string(),
            },
            SearchResult {
                document_id: "doc3".to_string(),
                score: 0.7,
                match_type: MatchType::WeakMatch,
                snippet: "test".to_string(),
            },
        ];

        let keyword_results = vec![
            ("doc4".to_string(), 10.0),
            ("doc5".to_string(), 8.0),
        ];

        let fused = RankFusion::fuse(vector_results, keyword_results, 2);

        // Should only return top 2 results
        assert_eq!(fused.len(), 2);
    }

    #[test]
    fn test_rrf_with_custom_k() {
        let vector_results = vec![
            SearchResult {
                document_id: "doc1".to_string(),
                score: 0.9,
                match_type: MatchType::DirectMatch,
                snippet: "test".to_string(),
            },
        ];

        let keyword_results = vec![
            ("doc1".to_string(), 10.0),
        ];

        // Test with k=10 (more emphasis on rank)
        let fused = RankFusion::fuse_with_k(
            vector_results.clone(),
            keyword_results.clone(),
            10,
            10.0
        );

        assert_eq!(fused.len(), 1);
        // RRF score: 1/(10+1) + 1/(10+1) = 2/11 ≈ 0.1818
        assert!((fused[0].score - (2.0 / 11.0)).abs() < 0.001);
    }

    #[test]
    fn test_rrf_no_overlap() {
        // Test when vector and keyword results have no overlap
        let vector_results = vec![
            SearchResult {
                document_id: "doc1".to_string(),
                score: 0.9,
                match_type: MatchType::DirectMatch,
                snippet: "test".to_string(),
            },
            SearchResult {
                document_id: "doc2".to_string(),
                score: 0.8,
                match_type: MatchType::SimilarMatch,
                snippet: "test".to_string(),
            },
        ];

        let keyword_results = vec![
            ("doc3".to_string(), 10.0),
            ("doc4".to_string(), 8.0),
        ];

        let fused = RankFusion::fuse(vector_results, keyword_results, 10);

        // All 4 documents should appear in results
        assert_eq!(fused.len(), 4);
    }

    #[test]
    fn test_rrf_empty_keyword() {
        // Test with empty keyword results
        let vector_results = vec![
            SearchResult {
                document_id: "doc1".to_string(),
                score: 0.9,
                match_type: MatchType::DirectMatch,
                snippet: "test".to_string(),
            },
        ];

        let keyword_results = vec![];

        let fused = RankFusion::fuse(vector_results, keyword_results, 10);

        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].document_id, "doc1");
    }

    #[test]
    fn test_rrf_empty_vector() {
        // Test with empty vector results
        let vector_results = vec![];

        let keyword_results = vec![
            ("doc1".to_string(), 10.0),
        ];

        let fused = RankFusion::fuse(vector_results, keyword_results, 10);

        // Should have no results since we can't create SearchResult without template
        assert_eq!(fused.len(), 0);
    }
}
