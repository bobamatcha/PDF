//! DocSign API Server - Backend for document signing
//!
//! Provides REST endpoints for:
//! - Signature sync from offline clients
//! - Session management
//! - Document delivery

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

mod error;
mod handlers;
mod models;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("docsign_api=info".parse()?)
                .add_directive("tower_http=debug".parse()?),
        )
        .init();

    // Initialize application state
    info!("Initializing DocSign API...");
    let state = AppState::new().await?;
    let state = Arc::new(state);

    // CORS configuration for web clients
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // Session endpoints
        .route("/api/session", post(handlers::create_session))
        .route("/api/session/:id", get(handlers::get_session))
        // Signature sync endpoint
        .route("/api/signatures/sync", post(handlers::sync_signatures))
        // Document delivery
        .route("/api/session/:id/document", get(handlers::get_document))
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    // Parse bind address
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting DocSign API on http://{}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
