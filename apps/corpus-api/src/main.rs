//! Corpus Server - HTTP API for the semantic document corpus
//!
//! Provides REST endpoints for:
//! - Hybrid search (vector + keyword)
//! - Document retrieval
//! - RAG-based document generation
//! - Corpus synchronization

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::info;

mod handlers;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("corpus_server=info".parse()?)
                .add_directive("tower_http=debug".parse()?),
        )
        .init();

    // Initialize application state
    info!("Initializing application state...");
    let state = AppState::new().await?;
    let state = Arc::new(state);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // Search endpoints
        .route("/search", post(handlers::search))
        .route("/search/vector", post(handlers::vector_search))
        .route("/search/keyword", post(handlers::keyword_search))
        // Document endpoints
        .route("/documents/:id", get(handlers::get_document))
        // Generation endpoint
        .route("/generate", post(handlers::generate))
        // Sync endpoints
        .route("/sync/version", get(handlers::get_corpus_version))
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Parse bind address
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting server on http://{}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
