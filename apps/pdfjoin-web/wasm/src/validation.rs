//! PDF validation and info extraction
//!
//! Validates PDF files and extracts metadata without full parsing.

use lopdf::Document;
use serde::Serialize;

/// PDF file information extracted during validation
#[derive(Debug, Clone, Serialize, Default)]
pub struct PdfInfo {
    /// Number of pages in the document
    pub page_count: u32,
    /// PDF version string (e.g., "1.7")
    pub version: String,
    /// Whether the document is encrypted
    pub encrypted: bool,
    /// File size in bytes
    pub size_bytes: usize,
    /// Whether the document appears valid
    pub valid: bool,
    /// Document title from metadata (if available)
    pub title: Option<String>,
    /// Document author from metadata (if available)
    pub author: Option<String>,
}

/// Validate a PDF file and extract basic info
pub fn validate_pdf(bytes: &[u8]) -> Result<PdfInfo, String> {
    // Check minimum size
    if bytes.len() < 8 {
        return Err("File too small to be a valid PDF".to_string());
    }

    // Check PDF magic bytes
    if !bytes.starts_with(b"%PDF-") {
        return Err("Not a valid PDF file (missing %PDF- header)".to_string());
    }

    // Extract version from header
    let version = extract_version(bytes);

    // Try to parse the document
    let document = Document::load_mem(bytes).map_err(|e| format!("Failed to parse PDF: {}", e))?;

    // Check for encryption
    let encrypted = document.is_encrypted();
    if encrypted {
        // We can still report info but operations may fail
    }

    // Get page count
    let page_count = document.get_pages().len() as u32;
    if page_count == 0 {
        return Err("PDF has no pages".to_string());
    }

    // Extract metadata if available
    let (title, author) = extract_metadata(&document);

    Ok(PdfInfo {
        page_count,
        version,
        encrypted,
        size_bytes: bytes.len(),
        valid: true,
        title,
        author,
    })
}

/// Extract PDF version from header
fn extract_version(bytes: &[u8]) -> String {
    // Header format: %PDF-1.7
    if bytes.len() >= 8 && bytes.starts_with(b"%PDF-") {
        let version_bytes = &bytes[5..8];
        if let Ok(version) = std::str::from_utf8(version_bytes) {
            return version.trim().to_string();
        }
    }
    "1.4".to_string() // Default version
}

/// Extract title and author from document metadata
fn extract_metadata(document: &Document) -> (Option<String>, Option<String>) {
    let mut title = None;
    let mut author = None;

    // Try to get Info dictionary from trailer
    if let Ok(info_ref) = document.trailer.get(b"Info") {
        if let Ok(info_id) = info_ref.as_reference() {
            if let Some(info_obj) = document.objects.get(&info_id) {
                if let Ok(info_dict) = info_obj.as_dict() {
                    // Extract Title
                    if let Ok(title_obj) = info_dict.get(b"Title") {
                        if let Ok(title_bytes) = title_obj.as_str() {
                            let decoded = String::from_utf8_lossy(title_bytes);
                            if !decoded.is_empty() {
                                title = Some(decoded.into_owned());
                            }
                        }
                    }

                    // Extract Author
                    if let Ok(author_obj) = info_dict.get(b"Author") {
                        if let Ok(author_bytes) = author_obj.as_str() {
                            let decoded = String::from_utf8_lossy(author_bytes);
                            if !decoded.is_empty() {
                                author = Some(decoded.into_owned());
                            }
                        }
                    }
                }
            }
        }
    }

    (title, author)
}

/// Quick validation without full parsing (for large files)
pub fn quick_validate(bytes: &[u8]) -> Result<(), String> {
    // Check minimum size
    if bytes.len() < 8 {
        return Err("File too small to be a valid PDF".to_string());
    }

    // Check PDF magic bytes
    if !bytes.starts_with(b"%PDF-") {
        return Err("Not a valid PDF file (missing %PDF- header)".to_string());
    }

    // Check for EOF marker (should be near the end)
    let tail = if bytes.len() > 1024 {
        &bytes[bytes.len() - 1024..]
    } else {
        bytes
    };

    if !tail.windows(5).any(|w| w == b"%%EOF") {
        return Err("PDF appears truncated (missing %%EOF marker)".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, content::Operation, Dictionary, Object, Stream};

    /// Create a valid test PDF with the specified number of pages
    /// Uses the same pattern as pdfjoin-core tests
    fn create_test_pdf(num_pages: u32) -> Vec<u8> {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();

        let mut page_ids = Vec::new();

        for i in 0..num_pages {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new(
                        "Tf",
                        vec![Object::Name(b"F1".to_vec()), Object::Integer(12)],
                    ),
                    Operation::new("Td", vec![Object::Integer(100), Object::Integer(700)]),
                    Operation::new(
                        "Tj",
                        vec![Object::String(
                            format!("Page {}", i + 1).into_bytes(),
                            lopdf::StringFormat::Literal,
                        )],
                    ),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id =
                doc.add_object(Stream::new(Dictionary::new(), content.encode().unwrap()));

            let page = Dictionary::from_iter(vec![
                ("Type", Object::Name(b"Page".to_vec())),
                ("Parent", Object::Reference(pages_id)),
                (
                    "MediaBox",
                    Object::Array(vec![
                        Object::Integer(0),
                        Object::Integer(0),
                        Object::Integer(612),
                        Object::Integer(792),
                    ]),
                ),
                ("Contents", Object::Reference(content_id)),
            ]);
            let page_id = doc.add_object(page);
            page_ids.push(page_id);
        }

        let pages = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Pages".to_vec())),
            ("Count", Object::Integer(num_pages as i64)),
            (
                "Kids",
                Object::Array(page_ids.iter().map(|id| Object::Reference(*id)).collect()),
            ),
        ]);
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Catalog".to_vec())),
            ("Pages", Object::Reference(pages_id)),
        ]);
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn test_quick_validate_rejects_non_pdf() {
        let result = quick_validate(b"not a pdf file");
        assert!(result.is_err());
    }

    #[test]
    fn test_quick_validate_rejects_small_file() {
        let result = quick_validate(b"tiny");
        assert!(result.is_err());
    }

    #[test]
    fn test_quick_validate_accepts_valid_pdf() {
        let pdf = create_test_pdf(1);
        let result = quick_validate(&pdf);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_pdf_returns_correct_page_count() {
        let pdf = create_test_pdf(5);
        let info = validate_pdf(&pdf).unwrap();
        assert_eq!(info.page_count, 5);
        assert!(info.valid);
    }

    #[test]
    fn test_validate_pdf_single_page() {
        let pdf = create_test_pdf(1);
        let info = validate_pdf(&pdf).unwrap();
        assert_eq!(info.page_count, 1);
        assert_eq!(info.version, "1.7");
        assert!(!info.encrypted);
    }

    #[test]
    fn test_validate_pdf_rejects_invalid_data() {
        let result = validate_pdf(b"not a valid pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_version() {
        assert_eq!(extract_version(b"%PDF-1.7\n"), "1.7");
        assert_eq!(extract_version(b"%PDF-1.4\n"), "1.4");
        assert_eq!(extract_version(b"%PDF-2.0\n"), "2.0");
    }
}
