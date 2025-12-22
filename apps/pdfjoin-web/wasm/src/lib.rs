//! WASM bindings for PDF Split/Merge operations
//!
//! This module provides a stateful, session-based API for PDF operations.
//! All state is held in Rust, minimizing JavaScript complexity.
//!
//! ## Architecture
//!
//! The library follows the agentpdf-web pattern:
//! - State management in Rust via `PdfJoinSession`
//! - PDF validation and parsing in Rust
//! - Page info extraction in Rust
//! - JavaScript only handles DOM events and file I/O
//!
//! ## Usage (JavaScript)
//!
//! ```javascript
//! import init, { PdfJoinSession, SessionMode } from './pkg/pdfjoin_wasm.js';
//!
//! await init();
//!
//! // Split mode
//! const session = new PdfJoinSession(SessionMode.Split);
//! session.setProgressCallback((current, total, msg) => updateUI(current, total, msg));
//! const info = session.addDocument("file.pdf", bytes);
//! session.setPageSelection("1-3, 5");
//! const result = session.execute();
//! downloadBlob(result, "split.pdf");
//!
//! // Merge mode
//! const session = new PdfJoinSession(SessionMode.Merge);
//! session.addDocument("a.pdf", bytesA);
//! session.addDocument("b.pdf", bytesB);
//! session.reorderDocuments([1, 0]); // swap order
//! const result = session.execute();
//! ```

pub mod page_info;
pub mod session;
pub mod validation;

use wasm_bindgen::prelude::*;

// Re-export main types for JavaScript
pub use page_info::{PageInfo, PageOrientation};
pub use session::{PdfJoinSession, SessionMode};
pub use validation::PdfInfo;

/// Initialize the WASM module
/// Called automatically by wasm-bindgen
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Get the library version
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Quick validation check for a PDF file
/// Returns Ok(()) if valid, Err with message if not
#[wasm_bindgen]
pub fn quick_validate(bytes: &[u8]) -> Result<(), JsValue> {
    validation::quick_validate(bytes).map_err(|e| JsValue::from_str(&e))
}

/// Get detailed PDF info without creating a session
/// Useful for showing file info before user commits to an operation
#[wasm_bindgen]
pub fn get_pdf_info(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let info = validation::validate_pdf(bytes).map_err(|e| JsValue::from_str(&e))?;

    serde_wasm_bindgen::to_value(&info)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Get page count from PDF bytes (convenience function)
#[wasm_bindgen]
pub fn get_page_count(bytes: &[u8]) -> Result<u32, JsValue> {
    let info = validation::validate_pdf(bytes).map_err(|e| JsValue::from_str(&e))?;
    Ok(info.page_count)
}

/// Format bytes as human-readable string
#[wasm_bindgen]
pub fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(2621440), "2.5 MB");
    }
}
