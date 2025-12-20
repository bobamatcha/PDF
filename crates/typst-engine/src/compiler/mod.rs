//! Typst compilation wrapper with timeout and error handling

pub mod errors;
pub mod output;
pub mod render;

pub use errors::{CompileError, RenderStatus, ServerError};
pub use output::OutputFormat;
pub use render::{compile_document, validate_syntax};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input source for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceInput {
    /// Raw Typst source code
    Raw(String),
}

/// Request to render a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderRequest {
    /// Raw Typst source code or template URI
    pub source: String,
    /// Variables injected into sys.inputs
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,
    /// Binary assets as base64 strings
    #[serde(default)]
    pub assets: HashMap<String, String>,
    /// Output format (pdf, svg, png)
    #[serde(default)]
    pub format: OutputFormat,
    /// Pixels per inch for PNG output
    pub ppi: Option<u32>,
}

/// Response from rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderResponse {
    pub status: RenderStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<RenderArtifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<CompileError>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<CompileError>,
}

/// Rendered artifact with base64-encoded data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderArtifact {
    pub data_base64: String,
    pub mime_type: String,
    pub page_count: usize,
}
