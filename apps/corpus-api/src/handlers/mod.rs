//! HTTP request handlers for the Corpus Server API
//!
//! Provides handlers for:
//! - Health checks
//! - Hybrid, vector, and keyword search
//! - Document retrieval
//! - RAG-based generation
//! - Corpus synchronization

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use corpus_core::search::{
    fusion::RankFusion, gating::SemanticGate, MatchType, SearchFilters,
    SearchResponse, SearchResult,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::state::AppState;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub filters: Option<SearchFilters>,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    pub query: String,
    #[serde(default)]
    pub anchor_documents: Vec<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_max_tokens() -> usize {
    1024
}

#[derive(Debug, Serialize)]
pub struct GenerateResponse {
    pub content: String,
    pub sources: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Serialize)]
pub struct DocumentResponse {
    pub id: String,
    pub content: String,
    pub title: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub document_count: usize,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub storage: String,
    pub embeddings: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Health check endpoint
pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let storage_status = match state.storage.health_check().await {
        Ok(_) => "healthy".to_string(),
        Err(e) => format!("unhealthy: {}", e),
    };

    Json(HealthResponse {
        status: "ok".to_string(),
        storage: storage_status,
        embeddings: format!("loaded (dim={})", state.embeddings.dimension()),
    })
}

/// Hybrid search endpoint (vector + keyword with RRF)
pub async fn search(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    info!("Hybrid search: query='{}', limit={}", request.query, request.limit);

    // Generate query embedding
    let query_embedding = state
        .embeddings
        .embed(&request.query)
        .map_err(|e| {
            error!("Failed to embed query: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Embedding failed: {}", e))
        })?;

    // Perform vector search
    let vector_results = state
        .vector_search
        .search(&query_embedding, request.limit * 2, request.filters.as_ref())
        .await
        .map_err(|e| {
            error!("Vector search failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Vector search failed: {}", e))
        })?;

    // Perform keyword search
    let keyword_index = state.keyword_index.read().await;
    let keyword_results = keyword_index
        .search(&request.query, request.limit * 2)
        .map_err(|e| {
            error!("Keyword search failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Keyword search failed: {}", e))
        })?;
    drop(keyword_index);

    // Fuse results using RRF
    let fused_results = RankFusion::fuse(
        vector_results,
        keyword_results,
        request.limit,
    );

    // Apply semantic gating
    let gate = SemanticGate::new();
    let suggestion = gate.evaluate(&fused_results);

    let response = SearchResponse {
        query: request.query,
        total_matches: fused_results.len(),
        results: fused_results,
        suggestion,
    };

    Ok(Json(response))
}

/// Vector-only search endpoint
pub async fn vector_search(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    info!("Vector search: query='{}', limit={}", request.query, request.limit);

    // Generate query embedding
    let query_embedding = state
        .embeddings
        .embed(&request.query)
        .map_err(|e| {
            error!("Failed to embed query: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Embedding failed: {}", e))
        })?;

    // Perform vector search
    let results = state
        .vector_search
        .search(&query_embedding, request.limit, request.filters.as_ref())
        .await
        .map_err(|e| {
            error!("Vector search failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Vector search failed: {}", e))
        })?;

    let response = SearchResponse {
        query: request.query,
        total_matches: results.len(),
        results,
        suggestion: None,
    };

    Ok(Json(response))
}

/// Keyword-only search endpoint
pub async fn keyword_search(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    info!("Keyword search: query='{}', limit={}", request.query, request.limit);

    let keyword_index = state.keyword_index.read().await;
    let raw_results = keyword_index
        .search(&request.query, request.limit)
        .map_err(|e| {
            error!("Keyword search failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Keyword search failed: {}", e))
        })?;
    drop(keyword_index);

    // Convert (String, f32) tuples to SearchResult
    let results: Vec<SearchResult> = raw_results
        .into_iter()
        .map(|(doc_id, score)| SearchResult {
            document_id: doc_id,
            score,
            match_type: MatchType::from(score),
            snippet: String::new(), // Keyword search doesn't provide snippets
        })
        .collect();

    let response = SearchResponse {
        query: request.query,
        total_matches: results.len(),
        results,
        suggestion: None,
    };

    Ok(Json(response))
}

/// Get document by ID
pub async fn get_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DocumentResponse>, (StatusCode, String)> {
    info!("Get document: id='{}'", id);

    let document = state
        .storage
        .get_document(&id)
        .await
        .map_err(|e| {
            error!("Failed to retrieve document: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, format!("Document not found: {}", id))
        })?;

    Ok(Json(DocumentResponse {
        id: document.id,
        content: document.content,
        title: document.metadata.title,
        author: document.metadata.author,
        tags: document.metadata.tags,
        created_at: document.metadata.created_at,
        updated_at: document.metadata.updated_at,
    }))
}

/// RAG-based document generation endpoint
pub async fn generate(
    State(state): State<Arc<AppState>>,
    Json(request): Json<GenerateRequest>,
) -> Result<Json<GenerateResponse>, (StatusCode, String)> {
    info!("Generate: query='{}', anchors={:?}", request.query, request.anchor_documents);

    // Get anchor documents or search for relevant ones
    let anchor_docs = if request.anchor_documents.is_empty() {
        // Search for relevant documents
        let query_embedding = state
            .embeddings
            .embed(&request.query)
            .map_err(|e| {
                error!("Failed to embed query: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Embedding failed: {}", e))
            })?;

        let results = state
            .vector_search
            .search(&query_embedding, 3, None)
            .await
            .map_err(|e| {
                error!("Vector search failed: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Search failed: {}", e))
            })?;

        results.iter().map(|r| r.document_id.clone()).collect()
    } else {
        request.anchor_documents
    };

    // Retrieve anchor document contents
    let mut context_parts = Vec::new();
    for doc_id in &anchor_docs {
        if let Ok(Some(doc)) = state.storage.get_document(doc_id).await {
            context_parts.push(format!("--- {} ---\n{}", doc.metadata.title, doc.content));
        }
    }

    // For now, return a placeholder response
    // In production, this would call an LLM API with the context
    let response = GenerateResponse {
        content: format!(
            "Generated response based on {} anchor documents for query: '{}'.\n\n\
            Note: Full RAG generation requires LLM integration (Phase 5).",
            anchor_docs.len(),
            request.query
        ),
        sources: anchor_docs,
        confidence: 0.75,
    };

    Ok(Json(response))
}

/// Get corpus version and stats
pub async fn get_corpus_version(
    State(state): State<Arc<AppState>>,
) -> Result<Json<VersionResponse>, (StatusCode, String)> {
    let version = state.corpus_version.read().await.clone();

    let document_count = state
        .storage
        .count_documents()
        .await
        .map_err(|e| {
            error!("Failed to count documents: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e))
        })?;

    Ok(Json(VersionResponse {
        version,
        document_count,
    }))
}
