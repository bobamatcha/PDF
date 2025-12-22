//! PDF Split and Merge operations
//!
//! This crate provides client-side PDF manipulation using lopdf.
//!
//! Two implementations are available:
//! - `split_document` / `merge_documents`: Full parse using lopdf (slower, more compatible)
//! - `streaming::split_streaming` / `streaming::merge_streaming`: Byte-level (faster, experimental)

pub mod command;
pub mod error;
pub mod merge;
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
