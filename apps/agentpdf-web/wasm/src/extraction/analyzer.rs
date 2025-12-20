//! PDF pre-flight analyzer
//!
//! Analyzes PDF structure to predict extraction difficulties
//! and select the optimal backend.
//!
//! ## Optimization
//!
//! Uses a tiered analysis approach to minimize overhead:
//! 1. Size check - tiny files skip analysis entirely
//! 2. Quick byte scan - detects problematic patterns without parsing
//! 3. Full analysis - only when needed, Document can be reused

pub use lopdf::Document;
use serde::{Deserialize, Serialize};

/// Analysis result for a PDF document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfAnalysis {
    /// PDF is valid and parseable
    pub is_valid: bool,
    /// PDF version string
    pub version: String,
    /// Number of pages
    pub page_count: u32,
    /// File size in bytes
    pub file_size: usize,
    /// Whether Identity-H encoding is detected
    pub has_identity_h: bool,
    /// Whether ToUnicode CMaps are present
    pub has_tounicode: bool,
    /// Whether embedded fonts are present
    pub has_embedded_fonts: bool,
    /// Whether the PDF appears to be scanned/image-only
    pub is_scanned: bool,
    /// Encryption status
    pub is_encrypted: bool,
    /// Predicted extraction difficulty
    pub difficulty: ExtractionDifficulty,
    /// Recommended backend
    pub recommended_backend: String,
    /// Analysis warnings
    pub warnings: Vec<String>,
}

/// Extraction difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionDifficulty {
    /// Simple PDF with standard encodings
    Easy,
    /// Some complexity but should work with native
    Medium,
    /// Complex encodings, may need fallback
    Hard,
    /// Very problematic, likely needs browser or OCR
    VeryHard,
    /// Scanned document, requires OCR
    RequiresOcr,
}

/// Result of quick analysis (no full parse)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickAnalysis {
    /// Small/simple file, use Legacy directly (no analysis needed)
    UseLegacy,
    /// Likely simple, but do quick validation with Legacy
    ProbablyLegacy,
    /// Detected problematic patterns, needs full analysis
    NeedsFullAnalysis,
    /// Invalid PDF header
    Invalid,
}

/// Size thresholds for routing decisions (in KB)
pub const TINY_PDF_KB: usize = 50;
pub const SMALL_PDF_KB: usize = 100;
pub const LARGE_PDF_KB: usize = 500;

/// Perform quick analysis without full PDF parsing
///
/// This is O(n) byte scan vs O(n log n) full parse - much faster for large files
pub fn quick_analyze(data: &[u8]) -> QuickAnalysis {
    // Check PDF header
    if data.len() < 8 || &data[0..4] != b"%PDF" {
        return QuickAnalysis::Invalid;
    }

    let size_kb = data.len() / 1024;

    // Tiny files: skip all analysis, Legacy is fastest
    if size_kb < TINY_PDF_KB {
        return QuickAnalysis::UseLegacy;
    }

    // Quick byte scan for problematic patterns
    // Look for Identity-H/Identity-V encoding markers
    let has_identity_encoding = data
        .windows(10)
        .any(|w| w.starts_with(b"Identity-H") || w.starts_with(b"Identity-V"));

    if !has_identity_encoding {
        // No Identity-H detected, Legacy should work fine
        if size_kb < SMALL_PDF_KB {
            return QuickAnalysis::UseLegacy;
        } else {
            // Larger file without Identity-H - probably fine but validate
            return QuickAnalysis::ProbablyLegacy;
        }
    }

    // Identity-H detected - need full analysis to check for ToUnicode
    QuickAnalysis::NeedsFullAnalysis
}

impl PdfAnalysis {
    /// Full analysis from raw bytes (parses internally)
    pub fn analyze(data: &[u8]) -> Self {
        let file_size = data.len();

        // Check PDF header
        if data.len() < 8 || &data[0..4] != b"%PDF" {
            return Self::invalid("Not a PDF file");
        }

        let version = Self::extract_version(data);

        // Try to load with lopdf
        match Document::load_mem(data) {
            Ok(doc) => Self::analyze_from_document(&doc, file_size, version),
            Err(e) => Self {
                is_valid: false,
                version,
                page_count: 0,
                file_size,
                has_identity_h: false,
                has_tounicode: false,
                has_embedded_fonts: false,
                is_scanned: false,
                is_encrypted: false,
                difficulty: ExtractionDifficulty::VeryHard,
                recommended_backend: "browser".to_string(),
                warnings: vec![format!("Parse error: {}", e)],
            },
        }
    }

    /// Full analysis from already-parsed Document (avoids double-parse)
    pub fn analyze_from_document(doc: &Document, file_size: usize, version: String) -> Self {
        let page_count = doc.get_pages().len() as u32;
        let is_encrypted = doc.is_encrypted();

        // Analyze fonts and encodings
        let font_analysis = Self::analyze_fonts(doc);

        // Check for image-only pages
        let is_scanned = Self::detect_scanned(doc);

        // Determine difficulty and recommendation
        let (difficulty, recommended_backend, warnings) =
            Self::determine_difficulty(&font_analysis, is_scanned, is_encrypted);

        Self {
            is_valid: true,
            version,
            page_count,
            file_size,
            has_identity_h: font_analysis.has_identity_h,
            has_tounicode: font_analysis.has_tounicode,
            has_embedded_fonts: font_analysis.has_embedded_fonts,
            is_scanned,
            is_encrypted,
            difficulty,
            recommended_backend,
            warnings,
        }
    }

    /// Parse document and return both Document and Analysis (for reuse)
    pub fn parse_and_analyze(data: &[u8]) -> Result<(Document, Self), String> {
        let file_size = data.len();

        if data.len() < 8 || &data[0..4] != b"%PDF" {
            return Err("Not a PDF file".to_string());
        }

        let version = Self::extract_version(data);

        let doc = Document::load_mem(data).map_err(|e| format!("Parse error: {}", e))?;

        let analysis = Self::analyze_from_document(&doc, file_size, version);

        Ok((doc, analysis))
    }

    fn invalid(reason: &str) -> Self {
        Self {
            is_valid: false,
            version: String::new(),
            page_count: 0,
            file_size: 0,
            has_identity_h: false,
            has_tounicode: false,
            has_embedded_fonts: false,
            is_scanned: false,
            is_encrypted: false,
            difficulty: ExtractionDifficulty::VeryHard,
            recommended_backend: "browser".to_string(),
            warnings: vec![reason.to_string()],
        }
    }

    fn extract_version(data: &[u8]) -> String {
        // Look for %PDF-X.Y
        if data.len() >= 8 {
            if let Ok(header) = std::str::from_utf8(&data[0..8]) {
                if let Some(version) = header.strip_prefix("%PDF-") {
                    return version.trim().to_string();
                }
            }
        }
        "unknown".to_string()
    }

    fn analyze_fonts(doc: &Document) -> FontAnalysis {
        let mut analysis = FontAnalysis::default();

        // Iterate through document objects looking for font dictionaries
        for (_id, object) in doc.objects.iter() {
            if let Ok(dict) = object.as_dict() {
                // Check if this is a font dictionary
                if let Ok(obj_type) = dict.get(b"Type") {
                    if let Ok(name) = obj_type.as_name_str() {
                        if name == "Font" {
                            analysis.font_count += 1;

                            // Check subtype
                            if let Ok(subtype) = dict.get(b"Subtype") {
                                if let Ok(subtype_name) = subtype.as_name_str() {
                                    if subtype_name == "Type0" {
                                        analysis.has_composite_fonts = true;
                                    }
                                }
                            }

                            // Check for embedded font data
                            if dict.has(b"FontDescriptor") {
                                analysis.has_embedded_fonts = true;
                            }

                            // Check encoding
                            if let Ok(encoding) = dict.get(b"Encoding") {
                                if let Ok(enc_name) = encoding.as_name_str() {
                                    if enc_name == "Identity-H" || enc_name == "Identity-V" {
                                        analysis.has_identity_h = true;
                                    }
                                }
                            }

                            // Check for ToUnicode
                            if dict.has(b"ToUnicode") {
                                analysis.has_tounicode = true;
                            }
                        }
                    }
                }
            }
        }

        analysis
    }

    fn detect_scanned(doc: &Document) -> bool {
        let mut image_count = 0;
        let mut text_op_count = 0;

        // Check content streams for text operations vs image operations
        for page_id in doc.get_pages().values() {
            if let Ok(content) = doc.get_page_content(*page_id) {
                if let Ok(operations) = lopdf::content::Content::decode(&content) {
                    for op in operations.operations {
                        match op.operator.as_str() {
                            "Tj" | "TJ" | "'" | "\"" => text_op_count += 1,
                            "Do" => image_count += 1, // XObject reference (often images)
                            _ => {}
                        }
                    }
                }
            }
        }

        // If mostly images and very few text operations, likely scanned
        image_count > 0 && text_op_count < 5
    }

    fn determine_difficulty(
        font_analysis: &FontAnalysis,
        is_scanned: bool,
        is_encrypted: bool,
    ) -> (ExtractionDifficulty, String, Vec<String>) {
        let mut warnings = Vec::new();

        if is_scanned {
            warnings.push("Document appears to be scanned/image-only".to_string());
            return (
                ExtractionDifficulty::RequiresOcr,
                "ocr".to_string(),
                warnings,
            );
        }

        if is_encrypted {
            warnings.push("Document is encrypted".to_string());
            return (
                ExtractionDifficulty::VeryHard,
                "browser".to_string(),
                warnings,
            );
        }

        // Identity-H without ToUnicode is the problematic case
        if font_analysis.has_identity_h && !font_analysis.has_tounicode {
            warnings.push("Identity-H encoding without ToUnicode CMap detected".to_string());
            warnings.push("Native extraction may produce garbage output".to_string());
            return (
                ExtractionDifficulty::VeryHard,
                "browser".to_string(),
                warnings,
            );
        }

        // Identity-H with ToUnicode should work
        if font_analysis.has_identity_h && font_analysis.has_tounicode {
            warnings.push("Identity-H encoding with ToUnicode CMap present".to_string());
            return (ExtractionDifficulty::Medium, "native".to_string(), warnings);
        }

        // Composite fonts can be tricky
        if font_analysis.has_composite_fonts {
            return (ExtractionDifficulty::Medium, "native".to_string(), warnings);
        }

        // Standard fonts should be easy
        (ExtractionDifficulty::Easy, "legacy".to_string(), warnings)
    }
}

#[derive(Debug, Default)]
struct FontAnalysis {
    font_count: usize,
    has_composite_fonts: bool,
    has_embedded_fonts: bool,
    has_identity_h: bool,
    has_tounicode: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_pdf() {
        let data = b"Not a PDF file at all";
        let analysis = PdfAnalysis::analyze(data);
        assert!(!analysis.is_valid);
        assert_eq!(analysis.recommended_backend, "browser");
    }

    #[test]
    fn test_extract_version() {
        let data = b"%PDF-1.7 more content...";
        let version = PdfAnalysis::extract_version(data);
        assert!(version.starts_with("1.7"));
    }

    #[test]
    fn test_difficulty_levels() {
        // Test that difficulty levels are ordered correctly
        assert!(ExtractionDifficulty::Easy != ExtractionDifficulty::Hard);
        assert!(ExtractionDifficulty::VeryHard != ExtractionDifficulty::RequiresOcr);
    }

    #[test]
    fn test_quick_analyze_invalid() {
        let data = b"Not a PDF";
        assert_eq!(quick_analyze(data), QuickAnalysis::Invalid);
    }

    #[test]
    fn test_quick_analyze_tiny_pdf() {
        // Tiny valid PDF header (under 50KB)
        let mut data = b"%PDF-1.4 ".to_vec();
        data.extend(vec![0u8; 10_000]); // ~10KB
        assert_eq!(quick_analyze(&data), QuickAnalysis::UseLegacy);
    }

    #[test]
    fn test_quick_analyze_small_no_identity_h() {
        // Small file (50-100KB) without Identity-H
        let mut data = b"%PDF-1.4 some content without identity encoding".to_vec();
        data.extend(vec![0u8; 60_000]); // ~60KB
        assert_eq!(quick_analyze(&data), QuickAnalysis::UseLegacy);
    }

    #[test]
    fn test_quick_analyze_large_no_identity_h() {
        // Larger file (>100KB) without Identity-H
        let mut data = b"%PDF-1.4 some content without identity encoding".to_vec();
        data.extend(vec![0u8; 150_000]); // ~150KB
        assert_eq!(quick_analyze(&data), QuickAnalysis::ProbablyLegacy);
    }

    #[test]
    fn test_quick_analyze_with_identity_h() {
        // File with Identity-H encoding marker
        let mut data = b"%PDF-1.4 /Encoding /Identity-H more content".to_vec();
        data.extend(vec![0u8; 60_000]); // ~60KB
        assert_eq!(quick_analyze(&data), QuickAnalysis::NeedsFullAnalysis);
    }

    #[test]
    fn test_quick_analyze_with_identity_v() {
        // File with Identity-V encoding marker
        let mut data = b"%PDF-1.4 /Encoding /Identity-V more content".to_vec();
        data.extend(vec![0u8; 60_000]); // ~60KB
        assert_eq!(quick_analyze(&data), QuickAnalysis::NeedsFullAnalysis);
    }
}
