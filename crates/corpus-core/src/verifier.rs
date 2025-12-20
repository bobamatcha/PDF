use crate::document::Template;
use thiserror::Error;

/// Errors that can occur during template verification
#[derive(Debug, Error, Clone, PartialEq)]
pub enum VerificationError {
    /// A required variable is missing
    #[error("Missing required variable: {0}")]
    MissingVariable(String),

    /// Invalid variable syntax detected
    #[error("Invalid variable syntax at position {0}: {1}")]
    InvalidSyntax(usize, String),

    /// Circular reference detected in variables
    #[error("Circular reference detected: {0}")]
    CircularReference(String),

    /// Unknown variable reference
    #[error("Unknown variable reference: {0}")]
    UnknownReference(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Result of template verification
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// The ID of the template that was verified
    pub template_id: String,

    /// Whether the template is valid
    pub is_valid: bool,

    /// List of errors found during verification
    pub errors: Vec<VerificationError>,

    /// List of warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

/// Trait for template verifiers
pub trait Verifier: Send + Sync {
    /// Verify a template and return the verification result
    fn verify(&self, template: &Template) -> VerificationResult;

    /// Get the name of this verifier
    fn name(&self) -> &str;
}

/// A composite verifier that runs multiple verifiers
#[derive(Default)]
pub struct CompositeVerifier {
    verifiers: Vec<Box<dyn Verifier>>,
}

impl CompositeVerifier {
    /// Create a new composite verifier
    pub fn new() -> Self {
        Self {
            verifiers: Vec::new(),
        }
    }

    /// Add a verifier to the composite
    pub fn add_verifier(mut self, verifier: Box<dyn Verifier>) -> Self {
        self.verifiers.push(verifier);
        self
    }

    /// Verify a template using all registered verifiers
    pub fn verify_all(&self, template: &Template) -> Vec<VerificationResult> {
        self.verifiers
            .iter()
            .map(|verifier| verifier.verify(template))
            .collect()
    }
}
