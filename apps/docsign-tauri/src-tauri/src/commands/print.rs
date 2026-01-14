//! Native printing commands for Tauri docsign app.
//!
//! This module provides platform-specific printing functionality for PDFs.
//! On macOS, it uses the `lpr` command and system printing infrastructure.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Characters that are dangerous in printer names (command injection risk)
pub const DANGEROUS_PRINTER_CHARS: [char; 10] =
    ['\'', '"', ';', '&', '|', '`', '$', '\\', '\n', '\r'];

/// Information about an available printer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrinterInfo {
    /// The printer name (as known to the system)
    pub name: String,
    /// Whether this is the default printer
    pub is_default: bool,
}

/// Validates a printer name for security.
///
/// Returns `Ok(())` if the printer name is safe to use in a command,
/// or an error if it contains dangerous characters that could enable
/// command injection attacks.
pub fn validate_printer_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Printer name cannot be empty".to_string());
    }

    if name.contains(DANGEROUS_PRINTER_CHARS) {
        return Err("Invalid printer name".to_string());
    }

    // Also reject names that are suspiciously long
    if name.len() > 256 {
        return Err("Printer name is too long".to_string());
    }

    Ok(())
}

/// Checks if a character is dangerous for printer names.
pub fn is_dangerous_printer_char(c: char) -> bool {
    DANGEROUS_PRINTER_CHARS.contains(&c)
}

/// Sanitizes a printer name by removing dangerous characters.
///
/// Note: This is provided for reference, but we prefer to reject
/// rather than sanitize to maintain security guarantees.
pub fn sanitize_printer_name(name: &str) -> String {
    name.chars()
        .filter(|c| !is_dangerous_printer_char(*c))
        .collect()
}

/// Formats an error message for print failures.
pub fn format_print_error(context: &str, error: impl std::fmt::Display) -> String {
    format!("{}: {}", context, error)
}

/// Formats an error message for printer not found.
pub fn format_printer_not_found(printer_name: &str) -> String {
    format!("Printer '{}' not found", printer_name)
}

/// Formats a generic print failure message.
pub fn format_print_failure_generic() -> String {
    "Failed to print - please check your printer connection".to_string()
}

/// Formats an error message for command execution failure.
pub fn format_command_error(error: impl std::fmt::Display) -> String {
    format!("Failed to execute print command: {}", error)
}

/// Parses lpstat output to extract printer information.
///
/// Expected format: "printer PrinterName is idle..." or "printer PrinterName disabled..."
pub fn parse_lpstat_printer_line(line: &str, default_printer: Option<&str>) -> Option<PrinterInfo> {
    if line.starts_with("printer ") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name = parts[1].to_string();
            let is_default = default_printer == Some(&name);
            return Some(PrinterInfo { name, is_default });
        }
    }
    None
}

/// Parses the default printer from lpstat -d output.
///
/// Expected format: "system default destination: PrinterName"
pub fn parse_default_printer(output: &str) -> Option<String> {
    output
        .trim()
        .split(':')
        .last()
        .map(|s| s.trim().to_string())
}

/// Opens the system print dialog with the provided PDF.
///
/// This creates a temporary file with the PDF contents and opens it
/// with the system's default PDF viewer, which typically provides
/// a print option.
#[tauri::command]
pub async fn print_pdf(pdf_bytes: Vec<u8>) -> Result<bool, String> {
    // Create a temporary file with .pdf extension
    let temp_file = create_temp_pdf(&pdf_bytes)?;
    let temp_path = temp_file.path();

    #[cfg(target_os = "macos")]
    {
        // On macOS, use the `open` command with the print flag
        // This opens Preview with the print dialog
        let result = Command::new("open")
            .arg("-a")
            .arg("Preview")
            .arg(temp_path)
            .spawn();

        match result {
            Ok(mut child) => {
                // Wait a bit for Preview to open, then send print command via AppleScript
                std::thread::sleep(std::time::Duration::from_millis(1000));

                let print_script = format!(
                    r#"tell application "Preview"
                        activate
                        tell application "System Events"
                            keystroke "p" using command down
                        end tell
                    end tell"#
                );

                let _ = Command::new("osascript")
                    .arg("-e")
                    .arg(&print_script)
                    .output();

                // Keep the temp file alive for a while so Preview can use it
                // In a real implementation, we'd track when the file is no longer needed
                let temp_path_owned = temp_path.to_path_buf();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    let _ = std::fs::remove_file(temp_path_owned);
                });

                // Forget about the temp file so it's not deleted immediately
                std::mem::forget(temp_file);

                let _ = child.wait();
                Ok(true)
            }
            Err(e) => Err(format!("Failed to open print dialog: {}", e)),
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use ShellExecute with "print" verb
        let result = Command::new("cmd")
            .args(["/C", "start", "/wait", "print"])
            .arg(temp_path)
            .spawn();

        match result {
            Ok(mut child) => {
                let _ = child.wait();
                Ok(true)
            }
            Err(e) => Err(format!("Failed to print: {}", e)),
        }
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, use lpr or xdg-open
        let result = Command::new("lpr").arg(temp_path).spawn();

        match result {
            Ok(mut child) => {
                let _ = child.wait();
                Ok(true)
            }
            Err(_) => {
                // Fallback to xdg-open
                let result = Command::new("xdg-open").arg(temp_path).spawn();

                match result {
                    Ok(mut child) => {
                        let _ = child.wait();
                        Ok(true)
                    }
                    Err(e) => Err(format!("Failed to print: {}", e)),
                }
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Printing not supported on this platform".to_string())
    }
}

/// Lists all available printers on the system.
#[tauri::command]
pub async fn get_available_printers() -> Result<Vec<PrinterInfo>, String> {
    #[cfg(target_os = "macos")]
    {
        get_macos_printers()
    }

    #[cfg(target_os = "windows")]
    {
        get_windows_printers()
    }

    #[cfg(target_os = "linux")]
    {
        get_linux_printers()
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Printer listing not supported on this platform".to_string())
    }
}

/// Prints a PDF directly to a specified printer.
#[tauri::command]
pub async fn print_to_printer(pdf_bytes: Vec<u8>, printer_name: String) -> Result<bool, String> {
    // Validate printer name to prevent command injection
    validate_printer_name(&printer_name)?;

    let temp_file = create_temp_pdf(&pdf_bytes)?;
    let temp_path = temp_file.path();

    #[cfg(target_os = "macos")]
    {
        // Use lpr with printer name
        let result = Command::new("lpr")
            .arg("-P")
            .arg(&printer_name)
            .arg(temp_path)
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(true)
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if stderr.contains("Unknown printer") || stderr.contains("not exist") {
                        Err(format!("Printer '{}' not found", printer_name))
                    } else if stderr.is_empty() {
                        Err("Failed to print - please check your printer connection".to_string())
                    } else {
                        Err(format!("Print failed: {}", stderr.trim()))
                    }
                }
            }
            Err(e) => Err(format!("Failed to execute print command: {}", e)),
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use print command with printer selection
        let result = Command::new("print")
            .arg(format!("/D:{}", printer_name))
            .arg(temp_path)
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(true)
                } else {
                    Err("Failed to print - please check your printer connection".to_string())
                }
            }
            Err(e) => Err(format!("Failed to execute print command: {}", e)),
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Use lpr with printer name
        let result = Command::new("lpr")
            .arg("-P")
            .arg(&printer_name)
            .arg(temp_path)
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(true)
                } else {
                    Err("Failed to print - please check your printer connection".to_string())
                }
            }
            Err(e) => Err(format!("Failed to execute print command: {}", e)),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Direct printing not supported on this platform".to_string())
    }
}

/// Creates a temporary PDF file from the given bytes.
fn create_temp_pdf(pdf_bytes: &[u8]) -> Result<NamedTempFile, String> {
    let mut temp_file = tempfile::Builder::new()
        .suffix(".pdf")
        .tempfile()
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;

    temp_file
        .write_all(pdf_bytes)
        .map_err(|e| format!("Failed to write PDF data: {}", e))?;

    temp_file
        .flush()
        .map_err(|e| format!("Failed to flush PDF data: {}", e))?;

    Ok(temp_file)
}

#[cfg(target_os = "macos")]
fn get_macos_printers() -> Result<Vec<PrinterInfo>, String> {
    // Get default printer
    let default_printer = Command::new("lpstat")
        .arg("-d")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Output format: "system default destination: PrinterName"
                stdout
                    .trim()
                    .split(':')
                    .last()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        });

    // Get all printers
    let output = Command::new("lpstat")
        .arg("-p")
        .output()
        .map_err(|e| format!("Failed to list printers: {}", e))?;

    if !output.status.success() {
        // No printers configured
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No destinations") || stderr.contains("lpstat: No") {
            return Ok(vec![]);
        }
        return Err("No printers available".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let printers: Vec<PrinterInfo> = stdout
        .lines()
        .filter_map(|line| {
            // Output format: "printer PrinterName is idle..."
            // or "printer PrinterName disabled..."
            if line.starts_with("printer ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[1].to_string();
                    let is_default = default_printer.as_ref() == Some(&name);
                    return Some(PrinterInfo { name, is_default });
                }
            }
            None
        })
        .collect();

    if printers.is_empty() {
        Ok(vec![])
    } else {
        Ok(printers)
    }
}

#[cfg(target_os = "windows")]
fn get_windows_printers() -> Result<Vec<PrinterInfo>, String> {
    // Use wmic to list printers
    let output = Command::new("wmic")
        .args(["printer", "get", "name,default"])
        .output()
        .map_err(|e| format!("Failed to list printers: {}", e))?;

    if !output.status.success() {
        return Err("No printers available".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let printers: Vec<PrinterInfo> = stdout
        .lines()
        .skip(1) // Skip header
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let is_default = parts[0].to_lowercase() == "true";
                let name = parts[1..].join(" ");
                if !name.is_empty() {
                    return Some(PrinterInfo { name, is_default });
                }
            }
            None
        })
        .collect();

    if printers.is_empty() {
        Ok(vec![])
    } else {
        Ok(printers)
    }
}

#[cfg(target_os = "linux")]
fn get_linux_printers() -> Result<Vec<PrinterInfo>, String> {
    // Same as macOS - uses CUPS
    get_macos_printers()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================
    // Unit Tests
    // ============================================

    #[test]
    fn test_printer_info_serialization() {
        let printer = PrinterInfo {
            name: "Test Printer".to_string(),
            is_default: true,
        };

        let json = serde_json::to_string(&printer).unwrap();
        assert!(json.contains("Test Printer"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_printer_info_deserialization() {
        let json = r#"{"name":"HP LaserJet","is_default":false}"#;
        let printer: PrinterInfo = serde_json::from_str(json).unwrap();
        assert_eq!(printer.name, "HP LaserJet");
        assert!(!printer.is_default);
    }

    #[test]
    fn test_create_temp_pdf() {
        let pdf_bytes = b"%PDF-1.4\n%%EOF";
        let result = create_temp_pdf(pdf_bytes);
        assert!(result.is_ok());

        let temp_file = result.unwrap();
        let path = temp_file.path();
        assert!(path.exists());
        assert!(path.extension().unwrap() == "pdf");
    }

    #[test]
    fn test_printer_name_validation_rejects_semicolon() {
        assert!(validate_printer_name("printer; rm -rf /").is_err());
    }

    #[test]
    fn test_printer_name_validation_rejects_quotes() {
        assert!(validate_printer_name("printer' OR '1'='1").is_err());
        assert!(validate_printer_name("printer\" OR \"1\"=\"1").is_err());
    }

    #[test]
    fn test_printer_name_validation_rejects_backticks() {
        assert!(validate_printer_name("printer`whoami`").is_err());
    }

    #[test]
    fn test_printer_name_validation_rejects_dollar() {
        assert!(validate_printer_name("printer$(id)").is_err());
    }

    #[test]
    fn test_printer_name_validation_rejects_pipe() {
        assert!(validate_printer_name("printer|cat /etc/passwd").is_err());
    }

    #[test]
    fn test_printer_name_validation_rejects_ampersand() {
        assert!(validate_printer_name("printer && echo pwned").is_err());
    }

    #[test]
    fn test_printer_name_validation_rejects_backslash() {
        assert!(validate_printer_name("printer\\ncmd").is_err());
    }

    #[test]
    fn test_printer_name_validation_rejects_newlines() {
        assert!(validate_printer_name("printer\ncmd").is_err());
        assert!(validate_printer_name("printer\rcmd").is_err());
    }

    #[test]
    fn test_printer_name_validation_rejects_empty() {
        assert!(validate_printer_name("").is_err());
    }

    #[test]
    fn test_printer_name_validation_accepts_valid() {
        assert!(validate_printer_name("HP LaserJet Pro").is_ok());
        assert!(validate_printer_name("Brother_HL-2270DW").is_ok());
        assert!(validate_printer_name("Canon PIXMA MG3620").is_ok());
    }

    #[test]
    fn test_parse_lpstat_printer_line_valid() {
        let result = parse_lpstat_printer_line("printer HP_LaserJet is idle", None);
        assert_eq!(
            result,
            Some(PrinterInfo {
                name: "HP_LaserJet".to_string(),
                is_default: false
            })
        );
    }

    #[test]
    fn test_parse_lpstat_printer_line_default() {
        let result = parse_lpstat_printer_line("printer HP_LaserJet is idle", Some("HP_LaserJet"));
        assert_eq!(
            result,
            Some(PrinterInfo {
                name: "HP_LaserJet".to_string(),
                is_default: true
            })
        );
    }

    #[test]
    fn test_parse_lpstat_printer_line_invalid() {
        assert!(parse_lpstat_printer_line("not a printer line", None).is_none());
        assert!(parse_lpstat_printer_line("", None).is_none());
        assert!(parse_lpstat_printer_line("printer", None).is_none());
    }

    #[test]
    fn test_parse_default_printer() {
        let output = "system default destination: HP_LaserJet";
        assert_eq!(
            parse_default_printer(output),
            Some("HP_LaserJet".to_string())
        );
    }

    #[test]
    fn test_error_message_formatting() {
        let error = format_printer_not_found("TestPrinter");
        assert!(error.contains("TestPrinter"));
        assert!(error.contains("not found"));
    }

    // ============================================
    // Property Tests - Printer Name Validation
    // ============================================

    proptest! {
        /// Names with dangerous characters should always be rejected
        #[test]
        fn prop_dangerous_chars_rejected(
            prefix in "[a-zA-Z0-9 ]{0,10}",
            dangerous in proptest::sample::select(DANGEROUS_PRINTER_CHARS.as_slice()),
            suffix in "[a-zA-Z0-9 ]{0,10}"
        ) {
            let name = format!("{}{}{}", prefix, dangerous, suffix);
            prop_assert!(validate_printer_name(&name).is_err());
        }

        /// Safe printer names should be accepted
        #[test]
        fn prop_safe_names_accepted(name in "[a-zA-Z0-9_ -]{1,100}") {
            // Filter out names that might accidentally contain dangerous chars
            if !name.contains(DANGEROUS_PRINTER_CHARS) {
                prop_assert!(validate_printer_name(&name).is_ok());
            }
        }

        /// Empty names should always be rejected
        #[test]
        fn prop_empty_name_rejected(_dummy in Just(())) {
            prop_assert!(validate_printer_name("").is_err());
        }

        /// Very long names should be rejected
        #[test]
        fn prop_long_names_rejected(length in 257usize..1000) {
            let name: String = (0..length).map(|_| 'a').collect();
            prop_assert!(validate_printer_name(&name).is_err());
        }

        /// Names at the length limit should be accepted
        #[test]
        fn prop_max_length_names_accepted(length in 1usize..=256) {
            let name: String = (0..length).map(|_| 'a').collect();
            prop_assert!(validate_printer_name(&name).is_ok());
        }

        /// Sanitized names should never contain dangerous characters
        #[test]
        fn prop_sanitized_no_dangerous_chars(name in ".*") {
            let sanitized = sanitize_printer_name(&name);
            for c in DANGEROUS_PRINTER_CHARS {
                prop_assert!(!sanitized.contains(c), "Sanitized name should not contain '{}'", c);
            }
        }

        /// is_dangerous_printer_char should match DANGEROUS_PRINTER_CHARS
        #[test]
        fn prop_dangerous_char_check_consistent(c in any::<char>()) {
            let expected = DANGEROUS_PRINTER_CHARS.contains(&c);
            prop_assert_eq!(is_dangerous_printer_char(c), expected);
        }
    }

    // ============================================
    // Property Tests - PrinterInfo Serialization
    // ============================================

    proptest! {
        /// Serialization should be reversible
        #[test]
        fn prop_serialization_roundtrip(name in "[a-zA-Z0-9_ ]{1,50}", is_default in any::<bool>()) {
            let printer = PrinterInfo {
                name: name.clone(),
                is_default,
            };

            let json = serde_json::to_string(&printer).unwrap();
            let deserialized: PrinterInfo = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(deserialized.name, name);
            prop_assert_eq!(deserialized.is_default, is_default);
        }

        /// JSON should contain the printer name
        #[test]
        fn prop_json_contains_name(name in "[a-zA-Z0-9]{1,20}") {
            let printer = PrinterInfo {
                name: name.clone(),
                is_default: false,
            };

            let json = serde_json::to_string(&printer).unwrap();
            prop_assert!(json.contains(&name));
        }

        /// JSON should contain is_default field
        #[test]
        fn prop_json_contains_is_default(is_default in any::<bool>()) {
            let printer = PrinterInfo {
                name: "Test".to_string(),
                is_default,
            };

            let json = serde_json::to_string(&printer).unwrap();
            prop_assert!(json.contains("is_default"));
            prop_assert!(json.contains(&is_default.to_string()));
        }
    }

    // ============================================
    // Property Tests - Error Message Generation
    // ============================================

    proptest! {
        /// Printer not found error should include the printer name
        #[test]
        fn prop_not_found_includes_name(name in "[a-zA-Z0-9_ ]{1,50}") {
            let error = format_printer_not_found(&name);
            prop_assert!(error.contains(&name));
            prop_assert!(error.to_lowercase().contains("not found"));
        }

        /// Command error should include the original error
        #[test]
        fn prop_command_error_includes_original(msg in "[a-zA-Z0-9 ]{1,50}") {
            let error = format_command_error(&msg);
            prop_assert!(error.contains(&msg));
        }

        /// Print error should include context and error
        #[test]
        fn prop_print_error_format(context in "[a-zA-Z ]{1,30}", msg in "[a-zA-Z0-9 ]{1,30}") {
            let error = format_print_error(&context, &msg);
            prop_assert!(error.contains(&context));
            prop_assert!(error.contains(&msg));
        }
    }

    // ============================================
    // Property Tests - lpstat Parsing
    // ============================================

    proptest! {
        /// Valid lpstat lines should parse correctly
        #[test]
        fn prop_parse_lpstat_valid(name in "[a-zA-Z0-9_-]{1,30}", status in "(idle|disabled|printing)") {
            let line = format!("printer {} is {}", name, status);
            let result = parse_lpstat_printer_line(&line, None);
            prop_assert!(result.is_some());
            prop_assert_eq!(result.unwrap().name, name);
        }

        /// Default printer detection should work
        #[test]
        fn prop_parse_lpstat_default_detection(name in "[a-zA-Z0-9_-]{1,30}") {
            let line = format!("printer {} is idle", name);

            // When printer matches default
            let result = parse_lpstat_printer_line(&line, Some(&name));
            prop_assert!(result.is_some());
            prop_assert!(result.unwrap().is_default);

            // When printer doesn't match default
            let result = parse_lpstat_printer_line(&line, Some("OtherPrinter"));
            prop_assert!(result.is_some());
            prop_assert!(!result.unwrap().is_default);
        }

        /// Invalid lines should return None
        #[test]
        fn prop_parse_lpstat_invalid(line in "[^p].*") {
            // Lines not starting with "printer " should return None
            let result = parse_lpstat_printer_line(&line, None);
            prop_assert!(result.is_none() || line.starts_with("printer "));
        }

        /// Default printer parsing should extract name after colon
        #[test]
        fn prop_parse_default_printer(name in "[a-zA-Z0-9_-]{1,30}") {
            let output = format!("system default destination: {}", name);
            let result = parse_default_printer(&output);
            prop_assert_eq!(result, Some(name));
        }
    }

    // ============================================
    // Property Tests - Temp File Creation
    // ============================================

    proptest! {
        /// Temp files should always have .pdf extension
        #[test]
        fn prop_temp_file_has_pdf_extension(bytes in proptest::collection::vec(any::<u8>(), 1..100)) {
            let result = create_temp_pdf(&bytes);
            prop_assert!(result.is_ok());
            let temp_file = result.unwrap();
            prop_assert_eq!(temp_file.path().extension().unwrap(), "pdf");
        }

        /// Temp files should exist after creation
        #[test]
        fn prop_temp_file_exists(bytes in proptest::collection::vec(any::<u8>(), 1..100)) {
            let result = create_temp_pdf(&bytes);
            prop_assert!(result.is_ok());
            let temp_file = result.unwrap();
            prop_assert!(temp_file.path().exists());
        }
    }
}
