//! Common types for PDF extraction

use serde::{Deserialize, Serialize};
use std::fmt;

/// A single text item with spatial information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextItem {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub font_name: Option<String>,
    pub font_size: Option<f64>,
}

/// Content extracted from a single page
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageContent {
    pub page_number: u32,
    pub text_items: Vec<TextItem>,
    pub raw_text: String,
    pub width: f64,
    pub height: f64,
}

/// Complete extraction result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub pages: Vec<PageContent>,
    pub backend_used: String,
    pub fallback_occurred: bool,
    pub extraction_time_ms: f64,
    pub total_characters: usize,
    pub warnings: Vec<String>,
}

impl ExtractionResult {
    pub fn new(backend: &str) -> Self {
        Self {
            pages: Vec::new(),
            backend_used: backend.to_string(),
            fallback_occurred: false,
            extraction_time_ms: 0.0,
            total_characters: 0,
            warnings: Vec::new(),
        }
    }

    pub fn with_pages(mut self, pages: Vec<PageContent>) -> Self {
        self.total_characters = pages.iter().map(|p| p.raw_text.len()).sum();
        self.pages = pages;
        self
    }

    pub fn with_time(mut self, ms: f64) -> Self {
        self.extraction_time_ms = ms;
        self
    }

    pub fn with_fallback(mut self, occurred: bool) -> Self {
        self.fallback_occurred = occurred;
        self
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Extraction errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionError {
    /// PDF parsing failed
    ParseError(String),
    /// Encoding issues (Identity-H without ToUnicode, etc.)
    EncodingFailure { details: String, recoverable: bool },
    /// Text extraction produced garbage output
    GarbageOutput { sample: String, confidence: f64 },
    /// Backend not available
    BackendUnavailable(String),
    /// JavaScript bridge error
    JsBridgeError(String),
    /// Generic error
    Other(String),
}

impl fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractionError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ExtractionError::EncodingFailure { details, .. } => {
                write!(f, "Encoding failure: {}", details)
            }
            ExtractionError::GarbageOutput { sample, confidence } => {
                write!(
                    f,
                    "Garbage output detected (confidence: {:.1}%): '{}'",
                    confidence * 100.0,
                    sample
                )
            }
            ExtractionError::BackendUnavailable(name) => {
                write!(f, "Backend '{}' not available", name)
            }
            ExtractionError::JsBridgeError(msg) => write!(f, "JS bridge error: {}", msg),
            ExtractionError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ExtractionError {}

/// Trait for PDF extraction backends
pub trait PdfBackend {
    /// Backend identifier
    fn name(&self) -> &'static str;

    /// Check if this backend can likely handle the PDF
    fn can_handle(&self, data: &[u8]) -> bool;

    /// Attempt extraction (synchronous)
    fn extract_sync(&self, data: &[u8]) -> Result<Vec<PageContent>, ExtractionError>;

    /// Validate extraction quality
    fn validate_output(&self, pages: &[PageContent]) -> ValidationResult;
}

/// Result of output validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub garbage_ratio: f64,
    pub encoding_issues_detected: bool,
    pub private_use_area_ratio: f64,
    pub details: String,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            garbage_ratio: 0.0,
            encoding_issues_detected: false,
            private_use_area_ratio: 0.0,
            details: "Output appears valid".to_string(),
        }
    }

    pub fn invalid(reason: &str, garbage_ratio: f64) -> Self {
        Self {
            is_valid: false,
            garbage_ratio,
            encoding_issues_detected: garbage_ratio > 0.1,
            private_use_area_ratio: 0.0,
            details: reason.to_string(),
        }
    }
}

/// Analyze text for quality issues
pub fn analyze_text_quality(text: &str) -> ValidationResult {
    if text.is_empty() {
        return ValidationResult::invalid("Empty text", 1.0);
    }

    let total_chars = text.chars().count();
    if total_chars == 0 {
        return ValidationResult::invalid("No characters", 1.0);
    }

    // Count problematic characters
    let mut replacement_chars = 0;
    let mut private_use_chars = 0;
    let mut control_chars = 0;
    let mut printable_chars = 0;

    for c in text.chars() {
        match c {
            '\u{FFFD}' => replacement_chars += 1, // Unicode replacement character
            '\u{E000}'..='\u{F8FF}' => private_use_chars += 1, // Private Use Area
            '\u{0000}'..='\u{001F}' if c != '\n' && c != '\r' && c != '\t' => {
                control_chars += 1;
            }
            c if c.is_alphanumeric() || c.is_whitespace() || c.is_ascii_punctuation() => {
                printable_chars += 1;
            }
            _ => {}
        }
    }

    let garbage_count = replacement_chars + private_use_chars + control_chars;
    let garbage_ratio = garbage_count as f64 / total_chars as f64;
    let pua_ratio = private_use_chars as f64 / total_chars as f64;

    // Thresholds for considering output as garbage
    const GARBAGE_THRESHOLD: f64 = 0.15; // 15% garbage = failure
    const PUA_THRESHOLD: f64 = 0.10; // 10% PUA = encoding issue

    if garbage_ratio > GARBAGE_THRESHOLD {
        return ValidationResult {
            is_valid: false,
            garbage_ratio,
            encoding_issues_detected: true,
            private_use_area_ratio: pua_ratio,
            details: format!(
                "High garbage ratio: {:.1}% (replacement: {}, PUA: {}, control: {})",
                garbage_ratio * 100.0,
                replacement_chars,
                private_use_chars,
                control_chars
            ),
        };
    }

    if pua_ratio > PUA_THRESHOLD {
        return ValidationResult {
            is_valid: false,
            garbage_ratio,
            encoding_issues_detected: true,
            private_use_area_ratio: pua_ratio,
            details: format!(
                "High Private Use Area ratio: {:.1}% - likely encoding failure",
                pua_ratio * 100.0
            ),
        };
    }

    ValidationResult {
        is_valid: true,
        garbage_ratio,
        encoding_issues_detected: false,
        private_use_area_ratio: pua_ratio,
        details: format!(
            "Valid output: {:.1}% printable, {:.2}% garbage",
            (printable_chars as f64 / total_chars as f64) * 100.0,
            garbage_ratio * 100.0
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_clean_text() {
        let text = "This is a clean text with normal characters.";
        let result = analyze_text_quality(text);
        assert!(result.is_valid);
        assert!(result.garbage_ratio < 0.01);
    }

    #[test]
    fn test_analyze_garbage_text() {
        let text = "Normal \u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}";
        let result = analyze_text_quality(text);
        // With 5 replacement chars out of ~12, ratio is ~40%
        assert!(result.garbage_ratio > 0.1);
    }

    #[test]
    fn test_analyze_pua_text() {
        // Simulate Identity-H failure with PUA characters
        let text = "Text\u{E001}\u{E002}\u{E003}\u{E004}\u{E005}more";
        let result = analyze_text_quality(text);
        assert!(result.private_use_area_ratio > 0.1);
    }

    #[test]
    fn test_empty_text_invalid() {
        let result = analyze_text_quality("");
        assert!(!result.is_valid);
        assert_eq!(result.garbage_ratio, 1.0);
    }
}
