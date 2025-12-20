//! Smart PDF text extraction with intelligent routing
//!
//! Provides multiple extraction backends with automatic routing:
//! - Legacy: Uses pdf-extract/lopdf (fastest for simple PDFs)
//! - Native: Uses enhanced lopdf with better encoding support
//! - Browser: Falls back to pdf.js via JS bridge (best for complex encodings)
//!
//! ## Routing Logic (Auto Strategy - Default)
//!
//! | PDF Characteristics | Backend Used | Reason |
//! |---------------------|--------------|--------|
//! | Small (<100KB), Easy | Legacy | Fastest |
//! | Medium size, Easy | Legacy | Fast, reliable |
//! | Large (>500KB), Medium | Hybrid | Better structure handling |
//! | Identity-H with ToUnicode | Native | Handles encoding properly |
//! | Identity-H without ToUnicode | Browser | Only option that works |
//! | Encrypted | Browser | Best compatibility |
//!
//! ## Usage
//!
//! ```javascript
//! // Auto routing (recommended) - picks best backend automatically
//! const text = await extract_text_hybrid(pdfData);
//!
//! // Force specific strategy
//! const text = await extract_text_with_strategy(pdfData, "legacy");
//! const text = await extract_text_with_strategy(pdfData, "auto");
//! ```

pub mod analyzer;
pub mod benchmark;
pub mod browser;
pub mod legacy;
pub mod native;
pub mod router;
pub mod types;

pub use analyzer::PdfAnalysis;
pub use benchmark::{BenchmarkResult, BenchmarkRunner, PdfCategory};
pub use router::{ExtractionConfig, ExtractionRouter, ExtractionStrategy};
pub use types::*;

use wasm_bindgen::prelude::*;

/// Smart extraction with automatic backend routing
///
/// Uses the Auto strategy by default which intelligently selects:
/// - Legacy for small, simple PDFs (fastest)
/// - Hybrid for larger or medium-complexity PDFs
/// - Browser for complex encodings (Identity-H without ToUnicode)
#[wasm_bindgen]
pub async fn extract_text_hybrid(data: &[u8]) -> Result<String, JsValue> {
    let config = ExtractionConfig::default();
    let router = ExtractionRouter::new(config);

    let result = router
        .extract(data)
        .await
        .map_err(|e| JsValue::from_str(&format!("Extraction failed: {}", e)))?;

    // Combine all pages into single text
    let text: String = result
        .pages
        .iter()
        .map(|p| p.raw_text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n--- Page Break ---\n\n");

    Ok(text)
}

/// Extract with specific strategy selection
#[wasm_bindgen]
pub async fn extract_text_with_strategy(data: &[u8], strategy: &str) -> Result<String, JsValue> {
    let strategy = match strategy {
        "legacy" => ExtractionStrategy::Legacy,
        "hybrid" => ExtractionStrategy::Hybrid,
        "native" => ExtractionStrategy::NativeOnly,
        "browser" => ExtractionStrategy::BrowserOnly,
        "auto" => ExtractionStrategy::Auto,
        _ => {
            return Err(JsValue::from_str(
                "Invalid strategy. Use: legacy, hybrid, native, browser, auto",
            ))
        }
    };

    let config = ExtractionConfig {
        strategy,
        ..Default::default()
    };

    let router = ExtractionRouter::new(config);
    let result = router
        .extract(data)
        .await
        .map_err(|e| JsValue::from_str(&format!("Extraction failed: {}", e)))?;

    let text: String = result
        .pages
        .iter()
        .map(|p| p.raw_text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n--- Page Break ---\n\n");

    Ok(text)
}

/// Get extraction metadata (backend used, timing, etc.)
#[wasm_bindgen]
pub async fn extract_with_metadata(data: &[u8], strategy: &str) -> Result<JsValue, JsValue> {
    let strategy = match strategy {
        "legacy" => ExtractionStrategy::Legacy,
        "hybrid" => ExtractionStrategy::Hybrid,
        "native" => ExtractionStrategy::NativeOnly,
        "browser" => ExtractionStrategy::BrowserOnly,
        "auto" => ExtractionStrategy::Auto,
        _ => return Err(JsValue::from_str("Invalid strategy")),
    };

    let config = ExtractionConfig {
        strategy,
        ..Default::default()
    };

    let router = ExtractionRouter::new(config);
    let result = router
        .extract(data)
        .await
        .map_err(|e| JsValue::from_str(&format!("Extraction failed: {}", e)))?;

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))
}
