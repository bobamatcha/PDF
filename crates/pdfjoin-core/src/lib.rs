//! PDF Split and Merge operations
//!
//! This crate provides client-side PDF manipulation using lopdf.
//!
//! Two implementations are available:
//! - `split_document` / `merge_documents`: Full parse using lopdf (slower, more compatible)
//! - `streaming::split_streaming` / `streaming::merge_streaming`: Byte-level (faster, experimental)

pub mod apply_operations;
pub mod command;
pub mod error;
pub mod merge;
pub mod operations;
pub mod split;
pub mod streaming;

pub use command::{PdfCommand, ProcessMetrics, ProcessResult};
pub use error::PdfJoinError;
pub use merge::merge_documents;
pub use split::split_document;
pub use streaming::{merge_streaming, split_streaming};

/// Parse PDF bytes and return page count
pub fn get_page_count(bytes: &[u8]) -> Result<u32, PdfJoinError> {
    let doc =
        lopdf::Document::load_mem(bytes).map_err(|e| PdfJoinError::ParseError(e.to_string()))?;
    Ok(doc.get_pages().len() as u32)
}

/// Parse page range string like "1-3, 5, 8-10" into sorted unique page numbers
pub fn parse_ranges(input: &str) -> Result<Vec<u32>, PdfJoinError> {
    use std::collections::BTreeSet;

    let mut pages = BTreeSet::new();

    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((start, end)) = part.split_once('-') {
            // Range like "1-3"
            let start: u32 = start
                .trim()
                .parse()
                .map_err(|_| PdfJoinError::InvalidRange(format!("Invalid start: {}", start)))?;
            let end: u32 = end
                .trim()
                .parse()
                .map_err(|_| PdfJoinError::InvalidRange(format!("Invalid end: {}", end)))?;

            if start > end {
                return Err(PdfJoinError::InvalidRange(format!(
                    "Start {} > end {}",
                    start, end
                )));
            }

            for page in start..=end {
                pages.insert(page);
            }
        } else {
            // Single page like "5"
            let page: u32 = part
                .parse()
                .map_err(|_| PdfJoinError::InvalidRange(format!("Invalid page: {}", part)))?;
            pages.insert(page);
        }
    }

    Ok(pages.into_iter().collect())
}

/// Detect if a PDF contains digital signatures
///
/// A PDF is considered "signed" if:
/// 1. The document has `/AcroForm` with `/SigFlags` field, OR
/// 2. Any object has `/Type /Sig`
///
/// # Arguments
/// * `bytes` - The PDF file as a byte slice
///
/// # Returns
/// * `Ok(true)` if the PDF contains signatures
/// * `Ok(false)` if the PDF does not contain signatures
/// * `Err(PdfJoinError)` if the PDF cannot be parsed
///
/// # Example
/// ```
/// use pdfjoin_core::has_signatures;
///
/// // Check if PDF has signatures
/// let pdf_bytes: &[u8] = b"%PDF-1.4..."; // Your PDF bytes here
/// match has_signatures(&pdf_bytes) {
///     Ok(true) => println!("PDF is signed"),
///     Ok(false) => println!("PDF is not signed"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn has_signatures(bytes: &[u8]) -> Result<bool, PdfJoinError> {
    let doc =
        lopdf::Document::load_mem(bytes).map_err(|e| PdfJoinError::ParseError(e.to_string()))?;

    // Check 1: Look for /AcroForm with /SigFlags in the catalog
    if let Ok(catalog) = doc.catalog() {
        if let Ok(acroform) = catalog.get(b"AcroForm") {
            if let Ok(acroform_dict) = acroform.as_dict() {
                if acroform_dict.has(b"SigFlags") {
                    return Ok(true);
                }
            }
        }
    }

    // Check 2: Look for any object with /Type /Sig
    for (_object_id, object) in doc.objects.iter() {
        if let Ok(dict) = object.as_dict() {
            if let Ok(obj_type) = dict.get(b"Type") {
                if let Ok(type_name) = obj_type.as_name_str() {
                    if type_name == "Sig" {
                        return Ok(true);
                    }
                }
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_deserializes_merge() {
        let json = r#"{"type":"Merge","files":[]}"#;
        let cmd: PdfCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, PdfCommand::Merge { .. }));
    }

    #[test]
    fn test_command_deserializes_split() {
        let json = r#"{"type":"Split","file":[],"ranges":[[1,3],[5,5]]}"#;
        let cmd: PdfCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, PdfCommand::Split { .. }));
    }

    #[test]
    fn test_parse_ranges_single() {
        let result = parse_ranges("5").unwrap();
        assert_eq!(result, vec![5]);
    }

    #[test]
    fn test_parse_ranges_range() {
        let result = parse_ranges("1-3").unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_ranges_complex() {
        let result = parse_ranges("1-3, 5, 8-10").unwrap();
        assert_eq!(result, vec![1, 2, 3, 5, 8, 9, 10]);
    }

    #[test]
    fn test_parse_ranges_deduplicates() {
        let result = parse_ranges("1-3, 2-4").unwrap();
        assert_eq!(result, vec![1, 2, 3, 4]);
    }
}

#[cfg(test)]
mod signature_tests {
    use super::*;

    /// Helper to create a minimal unsigned PDF
    fn create_minimal_unsigned_pdf() -> Vec<u8> {
        use lopdf::dictionary;
        use lopdf::{Document, Object};

        let mut doc = Document::with_version("1.4");

        // Create page object
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });

        // Create pages object
        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        });

        // Update page to reference parent
        if let Ok(page) = doc.get_object_mut(page_id) {
            if let Ok(dict) = page.as_dict_mut() {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }

        // Create catalog
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        });

        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    /// Helper to create a PDF with SigFlags in AcroForm
    fn create_pdf_with_sigflags() -> Vec<u8> {
        use lopdf::dictionary;
        use lopdf::{Document, Object};

        let mut doc = Document::with_version("1.4");

        // Create page object
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });

        // Create pages object
        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        });

        // Update page to reference parent
        if let Ok(page) = doc.get_object_mut(page_id) {
            if let Ok(dict) = page.as_dict_mut() {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }

        // Create AcroForm with SigFlags
        let acroform = dictionary! {
            "SigFlags" => 3,
            "Fields" => Object::Array(vec![]),
        };

        // Create catalog with AcroForm
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
            "AcroForm" => acroform,
        });

        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    /// Helper to create a PDF with a signature field object
    fn create_pdf_with_sig_object() -> Vec<u8> {
        use lopdf::dictionary;
        use lopdf::{Document, Object};

        let mut doc = Document::with_version("1.4");

        // Create page object
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });

        // Create pages object
        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        });

        // Update page to reference parent
        if let Ok(page) = doc.get_object_mut(page_id) {
            if let Ok(dict) = page.as_dict_mut() {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }

        // Create a signature object with /Type /Sig
        doc.add_object(dictionary! {
            "Type" => "Sig",
            "Filter" => "Adobe.PPKLite",
            "SubFilter" => "adbe.pkcs7.detached",
        });

        // Create catalog
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        });

        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn test_unsigned_pdf_returns_false() {
        let pdf = create_minimal_unsigned_pdf();
        let result = has_signatures(&pdf);
        if let Err(e) = &result {
            eprintln!("Error parsing unsigned PDF: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Should successfully parse unsigned PDF: {:?}",
            result
        );
        assert!(!result.unwrap(), "Unsigned PDF should return false");
    }

    #[test]
    fn test_pdf_with_sigflags_returns_true() {
        let pdf = create_pdf_with_sigflags();
        let result = has_signatures(&pdf);
        assert!(
            result.is_ok(),
            "Should successfully parse PDF with SigFlags"
        );
        assert!(result.unwrap(), "PDF with SigFlags should return true");
    }

    #[test]
    fn test_pdf_with_sig_object_returns_true() {
        let pdf = create_pdf_with_sig_object();
        let result = has_signatures(&pdf);
        assert!(
            result.is_ok(),
            "Should successfully parse PDF with /Type /Sig"
        );
        assert!(result.unwrap(), "PDF with /Type /Sig should return true");
    }

    #[test]
    fn test_invalid_pdf_returns_error() {
        let result = has_signatures(b"not a pdf");
        assert!(result.is_err(), "Invalid PDF should return error");
    }

    #[test]
    fn test_empty_bytes_returns_error() {
        let result = has_signatures(&[]);
        assert!(result.is_err(), "Empty bytes should return error");
    }
}
