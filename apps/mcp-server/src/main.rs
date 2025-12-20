//! Typst MCP Server Binary
//!
//! Entry point for the MCP server supporting multiple transports.

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "typst-mcp-server")]
#[command(
    version,
    about = "Generic Typst rendering server via Model Context Protocol"
)]
struct Args {
    /// Transport mode: stdio or http
    #[arg(short, long, default_value = "stdio")]
    transport: String,

    /// HTTP server address (only used with http transport)
    #[arg(long, default_value = "127.0.0.1:3000")]
    http_addr: String,

    /// Compilation timeout in milliseconds
    #[arg(long, default_value = "5000")]
    timeout_ms: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize tracing - CRITICAL: use stderr for stdio transport
    let use_stderr = args.transport == "stdio";

    if use_stderr {
        // For stdio transport, ALL output must go to stderr
        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    tracing::info!("Starting Typst MCP Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Transport: {}", args.transport);

    match args.transport.as_str() {
        "stdio" => {
            mcp_server::transport::stdio::run_stdio_server(args.timeout_ms).await?;
        }
        #[cfg(feature = "http")]
        "http" => {
            mcp_server::transport::http::run_http_server(&args.http_addr, args.timeout_ms).await?;
        }
        #[cfg(not(feature = "http"))]
        "http" => {
            eprintln!("HTTP transport not enabled. Rebuild with --features http");
            std::process::exit(1);
        }
        other => {
            eprintln!("Unknown transport: {}. Use 'stdio' or 'http'", other);
            std::process::exit(1);
        }
    }

    Ok(())
}
