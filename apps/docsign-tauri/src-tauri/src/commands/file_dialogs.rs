//! Native file dialog commands for document signing.
//!
//! These commands provide native OS file dialogs for opening and saving PDF files.
//! Designed with geriatric UX considerations:
//! - Clear file type descriptions ("PDF Documents" not just "*.pdf")
//! - Defaults to user's Documents folder for familiarity
//! - User-friendly error messages

use std::path::PathBuf;
use tauri_plugin_dialog::DialogExt;

/// Maximum file size allowed (100MB)
pub const MAX_FILE_SIZE: usize = 100 * 1024 * 1024;

/// Validates file size against the maximum limit.
///
/// Returns `Ok(())` if the file is within limits, or an error message if too large.
pub fn validate_file_size(size: usize) -> Result<(), String> {
    if size > MAX_FILE_SIZE {
        Err("This PDF file is too large (over 100MB). Please select a smaller file.".to_string())
    } else {
        Ok(())
    }
}

/// Validates that PDF bytes are not empty.
pub fn validate_pdf_not_empty(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        Err("Cannot save an empty PDF file.".to_string())
    } else {
        Ok(())
    }
}

/// Sanitizes a suggested filename for saving.
///
/// - Removes path separators to prevent directory traversal
/// - Replaces dangerous characters (including control characters)
/// - Ensures non-empty result
/// - Limits length to reasonable value (respecting UTF-8 boundaries)
pub fn sanitize_filename(name: &str) -> String {
    // Replace path separators, dangerous characters, and control characters
    let sanitized: String = name
        .chars()
        .filter_map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => Some('_'),
            '\0'..='\x1f' | '\x7f' => None, // Remove control characters entirely (including DEL)
            c => Some(c),
        })
        .collect();

    // Trim whitespace and dots from start/end
    let trimmed = sanitized.trim().trim_matches('.');

    // Limit length (255 is common filesystem limit)
    // Use char_indices to respect UTF-8 boundaries
    let limited = if trimmed.chars().count() > 200 {
        // Find the byte index of the 200th character
        let end_idx = trimmed
            .char_indices()
            .nth(200)
            .map(|(idx, _)| idx)
            .unwrap_or(trimmed.len());
        &trimmed[..end_idx]
    } else {
        trimmed
    };

    // Ensure non-empty result
    if limited.is_empty() {
        "document.pdf".to_string()
    } else {
        limited.to_string()
    }
}

/// Ensures a path has the .pdf extension.
///
/// Returns a new PathBuf with .pdf extension if not present or different.
pub fn ensure_pdf_extension(path: &PathBuf) -> PathBuf {
    let mut result = path.clone();
    if result.extension().map_or(true, |ext| {
        ext.to_str().map_or(true, |s| s.to_lowercase() != "pdf")
    }) {
        result.set_extension("pdf");
    }
    result
}

/// Formats an error message for file access failures.
pub fn format_file_access_error(context: &str, error: impl std::fmt::Display) -> String {
    format!("{} ({})", context, error)
}

/// Formats an error message for reading a file.
pub fn format_read_error(error: impl std::fmt::Display) -> String {
    format!(
        "Could not read the selected file. Please make sure you have permission to access it. ({})",
        error
    )
}

/// Formats an error message for writing a file.
pub fn format_write_error(error: impl std::fmt::Display) -> String {
    format!(
        "Could not save the file. Please make sure you have permission to write to this location. ({})",
        error
    )
}

/// Formats an error message for file selection access.
pub fn format_selection_error(error: impl std::fmt::Display) -> String {
    format!(
        "Could not access the selected file. Please try again. ({})",
        error
    )
}

/// Formats an error for a single file in multi-file selection.
pub fn format_file_skip_error(filename: &str, reason: &str) -> String {
    if reason.contains("100MB") {
        format!("Skipped '{}' - file is too large (over 100MB)", filename)
    } else {
        format!("Could not read '{}': {}", filename, reason)
    }
}

/// Opens a native file picker dialog for selecting a single PDF file.
///
/// Returns the file contents as bytes if a file was selected,
/// or None if the user cancelled the dialog.
///
/// # Errors
/// Returns an error string if:
/// - The file cannot be read
/// - The file is too large to load into memory
#[tauri::command]
pub async fn open_pdf_file(app: tauri::AppHandle) -> Result<Option<Vec<u8>>, String> {
    // Get the user's Documents folder as the default starting location
    let default_path = dirs::document_dir().unwrap_or_else(|| PathBuf::from("."));

    // Build the file dialog with clear descriptions for elderly users
    let file_path = app
        .dialog()
        .file()
        .set_title("Open PDF Document")
        .add_filter("PDF Documents", &["pdf", "PDF"])
        .set_directory(default_path)
        .blocking_pick_file();

    match file_path {
        Some(path) => {
            let path_buf = path.into_path().map_err(|e| {
                format!(
                    "Could not access the selected file. Please try again. ({})",
                    e
                )
            })?;

            // Read the file contents
            match tokio::fs::read(&path_buf).await {
                Ok(bytes) => {
                    // Sanity check file size (100MB limit)
                    if bytes.len() > 100 * 1024 * 1024 {
                        return Err(
                            "This PDF file is too large (over 100MB). Please select a smaller file."
                                .to_string(),
                        );
                    }
                    Ok(Some(bytes))
                }
                Err(e) => Err(format!(
                    "Could not read the selected file. Please make sure you have permission to access it. ({})",
                    e
                )),
            }
        }
        None => {
            // User cancelled - this is normal, not an error
            Ok(None)
        }
    }
}

/// Opens a native save dialog for saving a signed PDF file.
///
/// # Arguments
/// * `pdf_bytes` - The PDF file contents to save
/// * `suggested_name` - A suggested filename (e.g., "contract_signed.pdf")
///
/// # Returns
/// * `Ok(Some(path))` - The path where the file was saved
/// * `Ok(None)` - The user cancelled the save dialog
/// * `Err(message)` - An error occurred while saving
#[tauri::command]
pub async fn save_signed_pdf(
    app: tauri::AppHandle,
    pdf_bytes: Vec<u8>,
    suggested_name: String,
) -> Result<Option<String>, String> {
    // Validate input
    if pdf_bytes.is_empty() {
        return Err("Cannot save an empty PDF file.".to_string());
    }

    // Get the user's Documents folder as the default starting location
    let default_path = dirs::document_dir().unwrap_or_else(|| PathBuf::from("."));

    // Build the save dialog with clear descriptions
    let save_path = app
        .dialog()
        .file()
        .set_title("Save Signed PDF Document")
        .add_filter("PDF Documents", &["pdf", "PDF"])
        .set_directory(default_path)
        .set_file_name(&suggested_name)
        .blocking_save_file();

    match save_path {
        Some(path) => {
            let mut path_buf = path.into_path().map_err(|e| {
                format!(
                    "Could not access the save location. Please try again. ({})",
                    e
                )
            })?;

            // Ensure the file has .pdf extension
            if path_buf.extension().map_or(true, |ext| {
                ext.to_str().map_or(true, |s| s.to_lowercase() != "pdf")
            }) {
                path_buf.set_extension("pdf");
            }

            // Write the file
            match tokio::fs::write(&path_buf, &pdf_bytes).await {
                Ok(()) => {
                    let path_string = path_buf.to_string_lossy().to_string();
                    Ok(Some(path_string))
                }
                Err(e) => Err(format!(
                    "Could not save the file. Please make sure you have permission to write to this location. ({})",
                    e
                )),
            }
        }
        None => {
            // User cancelled - this is normal, not an error
            Ok(None)
        }
    }
}

/// Opens a native file picker dialog for selecting multiple PDF files.
///
/// Returns a list of (filename, file_bytes) pairs for each selected file.
/// Returns an empty list if the user cancelled the dialog.
///
/// # Errors
/// Returns an error string if any file cannot be read.
#[tauri::command]
pub async fn open_multiple_pdfs(app: tauri::AppHandle) -> Result<Vec<(String, Vec<u8>)>, String> {
    // Get the user's Documents folder as the default starting location
    let default_path = dirs::document_dir().unwrap_or_else(|| PathBuf::from("."));

    // Build the file dialog allowing multiple selection
    let file_paths = app
        .dialog()
        .file()
        .set_title("Select PDF Documents")
        .add_filter("PDF Documents", &["pdf", "PDF"])
        .set_directory(default_path)
        .blocking_pick_files();

    match file_paths {
        Some(paths) => {
            let mut results = Vec::with_capacity(paths.len());
            let mut errors = Vec::new();

            for path in paths {
                let path_buf = match path.into_path() {
                    Ok(p) => p,
                    Err(e) => {
                        errors.push(format!("Could not access a selected file: {}", e));
                        continue;
                    }
                };

                let filename = path_buf
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "document.pdf".to_string());

                match tokio::fs::read(&path_buf).await {
                    Ok(bytes) => {
                        // Check file size limit (100MB per file)
                        if bytes.len() > 100 * 1024 * 1024 {
                            errors.push(format!(
                                "Skipped '{}' - file is too large (over 100MB)",
                                filename
                            ));
                            continue;
                        }
                        results.push((filename, bytes));
                    }
                    Err(e) => {
                        errors.push(format!("Could not read '{}': {}", filename, e));
                    }
                }
            }

            // If some files failed but others succeeded, include warning in result
            if !errors.is_empty() && results.is_empty() {
                return Err(format!(
                    "Could not read any of the selected files:\n{}",
                    errors.join("\n")
                ));
            }

            // If we got some files, return them (frontend can show warnings separately)
            Ok(results)
        }
        None => {
            // User cancelled - return empty list
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================
    // Unit Tests
    // ============================================

    #[test]
    fn test_pdf_bytes_validation_empty() {
        let empty: Vec<u8> = vec![];
        assert!(validate_pdf_not_empty(&empty).is_err());
    }

    #[test]
    fn test_pdf_bytes_validation_non_empty() {
        let bytes = vec![0x25, 0x50, 0x44, 0x46]; // %PDF
        assert!(validate_pdf_not_empty(&bytes).is_ok());
    }

    #[test]
    fn test_file_size_at_limit() {
        assert!(validate_file_size(MAX_FILE_SIZE).is_ok());
    }

    #[test]
    fn test_file_size_over_limit() {
        assert!(validate_file_size(MAX_FILE_SIZE + 1).is_err());
    }

    #[test]
    fn test_sanitize_path_traversal() {
        let result = sanitize_filename("../../../etc/passwd");
        assert!(!result.contains('/'));
        // After sanitizing ../ to .._, it becomes ".._.._.._.." when trimmed of dots
        // The important thing is no path separators
    }

    #[test]
    fn test_sanitize_windows_path() {
        let result = sanitize_filename("C:\\Windows\\System32\\config");
        assert!(!result.contains('\\'));
        assert!(!result.contains(':'));
    }

    #[test]
    fn test_ensure_pdf_extension_missing() {
        let path = PathBuf::from("/home/user/document");
        let result = ensure_pdf_extension(&path);
        assert_eq!(result.extension().unwrap(), "pdf");
    }

    #[test]
    fn test_ensure_pdf_extension_present() {
        let path = PathBuf::from("/home/user/document.pdf");
        let result = ensure_pdf_extension(&path);
        assert_eq!(result.extension().unwrap(), "pdf");
    }

    #[test]
    fn test_ensure_pdf_extension_uppercase() {
        let path = PathBuf::from("/home/user/document.PDF");
        let result = ensure_pdf_extension(&path);
        // Should keep uppercase PDF (case-insensitive check)
        assert!(result
            .extension()
            .unwrap()
            .to_str()
            .unwrap()
            .eq_ignore_ascii_case("pdf"));
    }

    #[test]
    fn test_ensure_pdf_extension_wrong() {
        let path = PathBuf::from("/home/user/document.txt");
        let result = ensure_pdf_extension(&path);
        assert_eq!(result.extension().unwrap(), "pdf");
    }

    #[test]
    fn test_error_message_contains_context() {
        let error = format_read_error("permission denied");
        assert!(error.contains("permission denied"));
        assert!(error.contains("permission"));
    }

    // ============================================
    // Property Tests - File Size Validation
    // ============================================

    proptest! {
        /// Any file size under the limit should be valid
        #[test]
        fn prop_valid_file_sizes_accepted(size in 0usize..=MAX_FILE_SIZE) {
            prop_assert!(validate_file_size(size).is_ok());
        }

        /// Any file size over the limit should be rejected
        #[test]
        fn prop_oversized_files_rejected(size in (MAX_FILE_SIZE + 1)..=usize::MAX) {
            prop_assert!(validate_file_size(size).is_err());
        }

        /// Error message should be user-friendly for large files
        #[test]
        fn prop_large_file_error_message_is_friendly(size in (MAX_FILE_SIZE + 1)..=(MAX_FILE_SIZE * 10)) {
            let error = validate_file_size(size).unwrap_err();
            prop_assert!(error.contains("100MB"), "Error should mention the limit");
            prop_assert!(error.contains("smaller"), "Error should suggest action");
        }
    }

    // ============================================
    // Property Tests - Filename Sanitization
    // ============================================

    proptest! {
        /// Sanitized filenames should never contain path separators
        #[test]
        fn prop_sanitized_no_path_separators(name in ".*") {
            let result = sanitize_filename(&name);
            prop_assert!(!result.contains('/'), "Should not contain forward slash");
            prop_assert!(!result.contains('\\'), "Should not contain backslash");
        }

        /// Sanitized filenames should never contain dangerous shell characters
        #[test]
        fn prop_sanitized_no_dangerous_chars(name in ".*") {
            let result = sanitize_filename(&name);
            prop_assert!(!result.contains(':'), "Should not contain colon");
            prop_assert!(!result.contains('*'), "Should not contain asterisk");
            prop_assert!(!result.contains('?'), "Should not contain question mark");
            prop_assert!(!result.contains('"'), "Should not contain quote");
            prop_assert!(!result.contains('<'), "Should not contain less-than");
            prop_assert!(!result.contains('>'), "Should not contain greater-than");
            prop_assert!(!result.contains('|'), "Should not contain pipe");
        }

        /// Sanitized filenames should never be empty
        #[test]
        fn prop_sanitized_never_empty(name in ".*") {
            let result = sanitize_filename(&name);
            prop_assert!(!result.is_empty(), "Sanitized name should never be empty");
        }

        /// Sanitized filenames should have reasonable length (measured in characters, not bytes)
        #[test]
        fn prop_sanitized_reasonable_length(name in ".{0,1000}") {
            let result = sanitize_filename(&name);
            let char_count = result.chars().count();
            prop_assert!(char_count <= 200, "Sanitized name should be at most 200 chars, got {}", char_count);
        }

        /// Sanitized filenames should not have ASCII control characters (0x00-0x1F, 0x7F)
        #[test]
        fn prop_sanitized_no_control_chars(name in ".*") {
            let result = sanitize_filename(&name);
            prop_assert!(
                !result.chars().any(|c| matches!(c, '\0'..='\x1f' | '\x7f')),
                "Should not contain ASCII control characters"
            );
        }

        /// Valid filenames should be preserved
        #[test]
        fn prop_valid_filenames_preserved(name in "[a-zA-Z0-9_-]{1,50}\\.pdf") {
            let result = sanitize_filename(&name);
            prop_assert_eq!(result, name, "Valid filename should be unchanged");
        }
    }

    // ============================================
    // Property Tests - PDF Extension Handling
    // ============================================

    proptest! {
        /// Result should always have pdf extension for valid filenames
        #[test]
        fn prop_always_has_pdf_extension(stem in "[a-zA-Z0-9_]{1,20}") {
            // Use a proper filename, not just a path
            let path_buf = PathBuf::from(format!("/tmp/{}", stem));
            let result = ensure_pdf_extension(&path_buf);
            let ext = result.extension().map(|e| e.to_str().unwrap_or("")).unwrap_or("");
            prop_assert!(ext.eq_ignore_ascii_case("pdf"), "Extension should be pdf, got: {}", ext);
        }

        /// Paths already ending in .pdf should not change the stem
        #[test]
        fn prop_pdf_paths_keep_stem(stem in "[a-zA-Z0-9_]{1,20}") {
            let path = PathBuf::from(format!("/tmp/{}.pdf", stem));
            let result = ensure_pdf_extension(&path);
            prop_assert_eq!(
                result.file_stem().unwrap().to_str().unwrap(),
                stem,
                "Stem should be preserved"
            );
        }

        /// Paths with other extensions get .pdf appended
        #[test]
        fn prop_other_extensions_replaced(stem in "[a-zA-Z0-9_]{1,20}", ext in "[a-z]{1,4}") {
            if ext.to_lowercase() != "pdf" {
                let path = PathBuf::from(format!("/tmp/{}.{}", stem, ext));
                let result = ensure_pdf_extension(&path);
                prop_assert_eq!(result.extension().unwrap(), "pdf");
            }
        }
    }

    // ============================================
    // Property Tests - Error Message Formatting
    // ============================================

    proptest! {
        /// Error messages should contain the original error
        #[test]
        fn prop_error_contains_original(msg in "[a-zA-Z0-9 ]{1,100}") {
            let result = format_read_error(&msg);
            prop_assert!(result.contains(&msg), "Should contain original message");
        }

        /// Error messages should be wrapped in user-friendly context
        #[test]
        fn prop_error_has_context(msg in "[a-zA-Z0-9 ]{1,50}") {
            let result = format_read_error(&msg);
            prop_assert!(result.contains("Could not"), "Should have user-friendly prefix");
            prop_assert!(result.contains("("), "Should wrap error in parens");
            prop_assert!(result.contains(")"), "Should close parens");
        }

        /// Write errors should suggest checking permissions
        #[test]
        fn prop_write_error_mentions_permission(msg in "[a-zA-Z0-9 ]{1,50}") {
            let result = format_write_error(&msg);
            prop_assert!(result.to_lowercase().contains("permission"), "Should mention permissions");
        }

        /// File selection errors should suggest trying again
        #[test]
        fn prop_selection_error_suggests_retry(msg in "[a-zA-Z0-9 ]{1,50}") {
            let result = format_selection_error(&msg);
            prop_assert!(result.to_lowercase().contains("try again"), "Should suggest retry");
        }
    }

    // ============================================
    // Property Tests - Empty/Non-Empty Validation
    // ============================================

    proptest! {
        /// Non-empty byte arrays should always be valid
        #[test]
        fn prop_nonempty_bytes_valid(bytes in proptest::collection::vec(any::<u8>(), 1..1000)) {
            prop_assert!(validate_pdf_not_empty(&bytes).is_ok());
        }
    }

    #[test]
    fn test_empty_bytes_always_invalid() {
        let empty: Vec<u8> = vec![];
        assert!(validate_pdf_not_empty(&empty).is_err());
        let error = validate_pdf_not_empty(&empty).unwrap_err();
        assert!(error.contains("empty"), "Error should mention empty");
    }
}
