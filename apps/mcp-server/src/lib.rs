//! Generic Typst MCP Server
//!
//! A Model Context Protocol server that provides Typst document rendering
//! capabilities to MCP-compliant AI agents.

pub mod mcp;
pub mod transport;

// Re-export from typst-engine
pub use typst_engine::compiler;
pub use typst_engine::templates;
pub use typst_engine::verifier;
pub use typst_engine::world;

pub use mcp::server::TypstMcpServer;
pub use typst_engine::compiler::{RenderRequest, RenderResponse};
