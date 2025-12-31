//! agentPDF Proxy Server
//!
//! A high-performance server for rendering Florida real estate contracts
//! using the Typst templating engine. Provides REST API endpoints for:
//!
//! - Template rendering (PDF/SVG/PNG)
//! - Compliance validation
//! - Template listing
//!
//! ## Architecture
//!
//! This server acts as a proxy between the slim agentPDF frontend and the
//! typst-engine, providing:
//!
//! - Rate limiting via tower-governor
//! - Request caching (future)
//! - Authentication (future)
//! - Horizontal scaling capability

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod api;
mod error;
#[cfg(test)]
mod tests;

use api::{
    handle_check_compliance, handle_health, handle_list_document_types, handle_list_templates,
    handle_render_template,
};

/// Command-line arguments for the agentPDF server
#[derive(Parser, Debug)]
#[command(name = "agentpdf-server")]
#[command(about = "agentPDF proxy server for Typst template rendering")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Host address to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Render timeout in milliseconds
    #[arg(long, default_value = "10000")]
    timeout_ms: u64,

    /// Rate limit: requests per second per IP
    #[arg(long, default_value = "10")]
    rate_limit: u32,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Render timeout in milliseconds
    pub timeout_ms: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(log_level.into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting agentPDF server on {}:{}", args.host, args.port);

    // Create rate limiter configuration
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(args.rate_limit.into())
            .burst_size(args.rate_limit * 2)
            .finish()
            .expect("Failed to create rate limiter config"),
    );

    // Create shared state
    let state = AppState {
        timeout_ms: args.timeout_ms,
    };

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(handle_health))
        // API endpoints
        .route("/api/templates", get(handle_list_templates))
        .route("/api/document-types", get(handle_list_document_types))
        .route("/api/render", post(handle_render_template))
        .route("/api/compliance", post(handle_check_compliance))
        // Apply middleware
        .layer(GovernorLayer {
            config: governor_conf,
        })
        .layer(cors)
        .with_state(state);

    // Start server
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Server listening on http://{}", addr);
    info!("Rate limit: {} requests/second per IP", args.rate_limit);
    info!("Render timeout: {}ms", args.timeout_ms);

    axum::serve(listener, app).await?;

    Ok(())
}
