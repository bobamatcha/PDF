//! Semantic threshold gating for dynamic routing of search results
//!
//! This module implements confidence-based routing logic for search results:
//! - High confidence (>= 0.85): Direct match returned
//! - Medium confidence (0.75-0.85): Similar matches with generation suggestion
//! - Low confidence (< 0.75): Weak matches with strong RAG suggestion
//!
//! The gating logic enables seamless transitions between:
//! 1. Direct template retrieval (high confidence)
//! 2. Template adaptation (medium confidence)
//! 3. RAG-based custom generation (low confidence)

use crate::search::{
    SearchResult, SearchResponse, SearchSuggestion,
    HIGH_CONFIDENCE_THRESHOLD, LOW_CONFIDENCE_THRESHOLD, MatchType
};

/// Semantic threshold gating for dynamic routing of search results
///
/// Routes queries based on confidence scores:
/// - High confidence (>= 0.85): Direct match returned
/// - Medium confidence (0.75-0.85): Similar matches with generation suggestion
/// - Low confidence (< 0.75): Weak matches with strong RAG suggestion
pub struct SemanticGate;

impl SemanticGate {
    /// Create a new SemanticGate instance
    pub fn new() -> Self {
        Self
    }

    /// Evaluate search results and return an appropriate suggestion
    ///
    /// This is a simplified version of `apply` that only returns the suggestion
    /// without wrapping in a SearchResponse.
    pub fn evaluate(&self, results: &[SearchResult]) -> Option<SearchSuggestion> {
        if results.is_empty() {
            return Some(SearchSuggestion::GenerateCustom {
                message: "No documents found. Would you like to generate a new one?".to_string(),
                anchor_documents: vec![],
            });
        }

        let top_score = results.first().map(|r| r.score).unwrap_or(0.0);

        if top_score >= HIGH_CONFIDENCE_THRESHOLD {
            // Direct match - no suggestion needed
            None
        } else if top_score >= LOW_CONFIDENCE_THRESHOLD {
            // Similar matches exist but not exact
            Some(SearchSuggestion::GenerateCustom {
                message: "Found similar documents that might serve as a starting point.".to_string(),
                anchor_documents: results
                    .iter()
                    .take(3)
                    .map(|r| r.document_id.clone())
                    .collect(),
            })
        } else {
            // Weak matches - strong RAG suggestion
            Some(SearchSuggestion::GenerateCustom {
                message: "No exact match found. Would you like to generate a new custom document?".to_string(),
                anchor_documents: results
                    .iter()
                    .take(3)
                    .map(|r| r.document_id.clone())
                    .collect(),
            })
        }
    }
}

impl Default for SemanticGate {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticGate {
    /// Apply threshold gating to search results
    ///
    /// # Arguments
    /// * `query` - The original search query
    /// * `results` - Vector search results sorted by score descending
    ///
    /// # Returns
    /// SearchResponse with appropriate suggestions based on confidence
    pub fn apply(query: &str, results: Vec<SearchResult>) -> SearchResponse {
        if results.is_empty() {
            return SearchResponse {
                query: query.to_string(),
                results: vec![],
                total_matches: 0,
                suggestion: Some(SearchSuggestion::GenerateCustom {
                    message: "No documents found. Would you like to generate a new one?".to_string(),
                    anchor_documents: vec![],
                }),
            };
        }

        let top_score = results.first().map(|r| r.score).unwrap_or(0.0);

        let suggestion = if top_score >= HIGH_CONFIDENCE_THRESHOLD {
            // Direct match - no suggestion needed
            None
        } else if top_score >= LOW_CONFIDENCE_THRESHOLD {
            // Similar matches exist but not exact
            Some(SearchSuggestion::GenerateCustom {
                message: format!(
                    "We couldn't find an exact template matching '{}'. \
                     However, we found similar documents that might serve as a starting point.",
                    query
                ),
                anchor_documents: results
                    .iter()
                    .take(3)
                    .map(|r| r.document_id.clone())
                    .collect(),
            })
        } else {
            // Weak matches - strong RAG suggestion
            Some(SearchSuggestion::GenerateCustom {
                message: format!(
                    "No exact match found for '{}'. \
                     Would you like to generate a new custom document based on similar templates?",
                    query
                ),
                anchor_documents: results
                    .iter()
                    .take(3)
                    .map(|r| r.document_id.clone())
                    .collect(),
            })
        };

        SearchResponse {
            query: query.to_string(),
            total_matches: results.len(),
            results,
            suggestion,
        }
    }

    /// Check if results meet high confidence threshold
    pub fn is_high_confidence(results: &[SearchResult]) -> bool {
        results
            .first()
            .map(|r| r.score >= HIGH_CONFIDENCE_THRESHOLD)
            .unwrap_or(false)
    }

    /// Check if results require RAG generation
    pub fn needs_generation(results: &[SearchResult]) -> bool {
        results
            .first()
            .map(|r| r.score < LOW_CONFIDENCE_THRESHOLD)
            .unwrap_or(true)
    }

    /// Get anchor documents for RAG context
    pub fn get_anchors(results: &[SearchResult], count: usize) -> Vec<String> {
        results
            .iter()
            .take(count)
            .map(|r| r.document_id.clone())
            .collect()
    }

    /// Classify the overall result quality
    pub fn classify(results: &[SearchResult]) -> MatchType {
        results
            .first()
            .map(|r| MatchType::from(r.score))
            .unwrap_or(MatchType::WeakMatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(id: &str, score: f32) -> SearchResult {
        SearchResult {
            document_id: id.to_string(),
            score,
            match_type: MatchType::from(score),
            snippet: "test snippet".to_string(),
        }
    }

    #[test]
    fn test_high_confidence_no_suggestion() {
        let results = vec![make_result("doc1", 0.90)];
        let response = SemanticGate::apply("test query", results);
        assert!(response.suggestion.is_none());
    }

    #[test]
    fn test_medium_confidence_has_suggestion() {
        let results = vec![make_result("doc1", 0.80)];
        let response = SemanticGate::apply("test query", results);
        assert!(response.suggestion.is_some());
    }

    #[test]
    fn test_low_confidence_has_suggestion() {
        let results = vec![make_result("doc1", 0.50)];
        let response = SemanticGate::apply("test query", results);
        assert!(response.suggestion.is_some());
    }

    #[test]
    fn test_empty_results() {
        let response = SemanticGate::apply("test query", vec![]);
        assert!(response.suggestion.is_some());
        assert_eq!(response.total_matches, 0);
    }

    #[test]
    fn test_is_high_confidence_true() {
        let results = vec![make_result("doc1", 0.90)];
        assert!(SemanticGate::is_high_confidence(&results));
    }

    #[test]
    fn test_is_high_confidence_false() {
        let results = vec![make_result("doc1", 0.80)];
        assert!(!SemanticGate::is_high_confidence(&results));
    }

    #[test]
    fn test_needs_generation_true() {
        let results = vec![make_result("doc1", 0.70)];
        assert!(SemanticGate::needs_generation(&results));
    }

    #[test]
    fn test_needs_generation_false() {
        let results = vec![make_result("doc1", 0.80)];
        assert!(!SemanticGate::needs_generation(&results));
    }

    #[test]
    fn test_get_anchors() {
        let results = vec![
            make_result("doc1", 0.90),
            make_result("doc2", 0.85),
            make_result("doc3", 0.80),
            make_result("doc4", 0.75),
        ];
        let anchors = SemanticGate::get_anchors(&results, 3);
        assert_eq!(anchors.len(), 3);
        assert_eq!(anchors, vec!["doc1", "doc2", "doc3"]);
    }

    #[test]
    fn test_classify_direct_match() {
        let results = vec![make_result("doc1", 0.90)];
        match SemanticGate::classify(&results) {
            MatchType::DirectMatch => {}
            _ => panic!("Expected DirectMatch"),
        }
    }

    #[test]
    fn test_classify_similar_match() {
        let results = vec![make_result("doc1", 0.80)];
        match SemanticGate::classify(&results) {
            MatchType::SimilarMatch => {}
            _ => panic!("Expected SimilarMatch"),
        }
    }

    #[test]
    fn test_classify_weak_match() {
        let results = vec![make_result("doc1", 0.70)];
        match SemanticGate::classify(&results) {
            MatchType::WeakMatch => {}
            _ => panic!("Expected WeakMatch"),
        }
    }

    #[test]
    fn test_anchor_documents_in_response() {
        let results = vec![
            make_result("doc1", 0.80),
            make_result("doc2", 0.78),
            make_result("doc3", 0.76),
        ];
        let response = SemanticGate::apply("test query", results);

        match response.suggestion {
            Some(SearchSuggestion::GenerateCustom { anchor_documents, .. }) => {
                assert_eq!(anchor_documents.len(), 3);
                assert_eq!(anchor_documents, vec!["doc1", "doc2", "doc3"]);
            }
            _ => panic!("Expected GenerateCustom suggestion"),
        }
    }

    #[test]
    fn test_message_content_medium_confidence() {
        let results = vec![make_result("doc1", 0.80)];
        let response = SemanticGate::apply("contract template", results);

        match response.suggestion {
            Some(SearchSuggestion::GenerateCustom { message, .. }) => {
                assert!(message.contains("couldn't find an exact template"));
                assert!(message.contains("contract template"));
                assert!(message.contains("similar documents"));
            }
            _ => panic!("Expected GenerateCustom suggestion"),
        }
    }

    #[test]
    fn test_message_content_low_confidence() {
        let results = vec![make_result("doc1", 0.50)];
        let response = SemanticGate::apply("rare document", results);

        match response.suggestion {
            Some(SearchSuggestion::GenerateCustom { message, .. }) => {
                assert!(message.contains("No exact match found"));
                assert!(message.contains("rare document"));
                assert!(message.contains("generate a new custom document"));
            }
            _ => panic!("Expected GenerateCustom suggestion"),
        }
    }

    #[test]
    fn test_query_preserved_in_response() {
        let results = vec![make_result("doc1", 0.90)];
        let response = SemanticGate::apply("my test query", results);
        assert_eq!(response.query, "my test query");
    }

    #[test]
    fn test_total_matches_count() {
        let results = vec![
            make_result("doc1", 0.90),
            make_result("doc2", 0.85),
            make_result("doc3", 0.80),
        ];
        let response = SemanticGate::apply("test", results);
        assert_eq!(response.total_matches, 3);
    }
}
