//! Error types for compilation and server operations

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Status of a render operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderStatus {
    Success,
    Error,
}

/// A compilation error with location information (LLM-friendly)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileError {
    /// Human-readable error message
    pub message: String,
    /// Line number (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Column number (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// Span start byte offset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_start: Option<usize>,
    /// Span end byte offset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_end: Option<usize>,
    /// Helpful hint for fixing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Severity level
    pub severity: ErrorSeverity,
}

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Error,
    Warning,
}

/// Server-side errors
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Font loading error: {0}")]
    FontError(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Compilation failed")]
    CompileError(Vec<CompileError>),

    #[error("Compilation timeout after {0}ms")]
    Timeout(u64),

    #[error("Invalid asset encoding for '{0}': {1}")]
    AssetError(String, String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Unknown tool: {0}")]
    UnknownTool(String),

    #[error("Prompt not found: {0}")]
    PromptNotFound(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Typst source error: {0}")]
    SourceError(String),

    #[error("Path security violation: {0}")]
    PathSecurityViolation(String),
}

impl CompileError {
    /// Create a new compile error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
            span_start: None,
            span_end: None,
            hint: None,
            severity: ErrorSeverity::Error,
        }
    }

    /// Set the location
    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    /// Set the span
    pub fn with_span(mut self, start: usize, end: usize) -> Self {
        self.span_start = Some(start);
        self.span_end = Some(end);
        self
    }

    /// Set a hint
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Set as warning
    pub fn as_warning(mut self) -> Self {
        self.severity = ErrorSeverity::Warning;
        self
    }
}
