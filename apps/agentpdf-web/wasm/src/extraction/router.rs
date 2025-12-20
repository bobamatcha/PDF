//! Extraction router - coordinates between backends with intelligent fallback
//!
//! ## Optimized Routing Flow
//!
//! 1. Quick byte scan (no parse) - determines if full analysis needed
//! 2. Small/simple files → Legacy directly (fastest path)
//! 3. Complex files → Parse once, share Document between analysis + extraction

use super::analyzer::{quick_analyze, ExtractionDifficulty, PdfAnalysis, QuickAnalysis};
use super::browser::BrowserExtractor;
use super::legacy::LegacyExtractor;
use super::native::NativeExtractor;
use super::types::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use web_sys::window;

/// Extraction strategy configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionStrategy {
    /// Use legacy pdf-extract/lopdf only
    Legacy,
    /// Use hybrid approach with intelligent fallback
    Hybrid,
    /// Use native pdf_oxide only (no fallback)
    NativeOnly,
    /// Use browser pdf.js only
    BrowserOnly,
    /// Automatically select based on PDF analysis
    Auto,
}

#[allow(clippy::derivable_impls)]
impl Default for ExtractionStrategy {
    fn default() -> Self {
        // Auto strategy intelligently routes based on PDF characteristics:
        // - Small, simple PDFs → Legacy (fastest)
        // - Large or complex PDFs → Hybrid with fallback
        ExtractionStrategy::Auto
    }
}

/// Configuration for extraction behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Primary strategy to use
    pub strategy: ExtractionStrategy,
    /// Enable fallback to browser on native failure
    pub enable_browser_fallback: bool,
    /// Enable OCR fallback for scanned documents
    pub enable_ocr_fallback: bool,
    /// Validate output quality and trigger fallback if garbage detected
    pub validate_output: bool,
    /// Maximum retries before giving up
    pub max_retries: u32,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            // Use Auto strategy by default for intelligent routing
            strategy: ExtractionStrategy::default(),
            enable_browser_fallback: true,
            enable_ocr_fallback: false, // OCR not implemented yet
            validate_output: true,
            max_retries: 2,
        }
    }
}

/// Main extraction router
pub struct ExtractionRouter {
    config: ExtractionConfig,
    legacy: LegacyExtractor,
    native: NativeExtractor,
    browser: BrowserExtractor,
}

impl ExtractionRouter {
    pub fn new(config: ExtractionConfig) -> Self {
        Self {
            config,
            legacy: LegacyExtractor::new(),
            native: NativeExtractor::new(),
            browser: BrowserExtractor::new(),
        }
    }

    /// Main extraction entry point
    pub async fn extract(&self, data: &[u8]) -> Result<ExtractionResult, ExtractionError> {
        // Get timing
        let start_time = Self::get_time_ms();

        // Determine which backend to use
        // Note: Auto strategy does its own optimized analysis (quick byte scan)
        // Only do full analysis for strategies that need it
        let result = match self.config.strategy {
            ExtractionStrategy::Legacy => self.extract_legacy(data).await,
            ExtractionStrategy::NativeOnly => self.extract_native_only(data).await,
            ExtractionStrategy::BrowserOnly => self.extract_browser_only(data).await,
            ExtractionStrategy::Hybrid => {
                let analysis = PdfAnalysis::analyze(data);
                self.extract_hybrid(data, &analysis).await
            }
            ExtractionStrategy::Auto => {
                // Auto does its own quick analysis - no upfront full parse!
                self.extract_auto_optimized(data).await
            }
        };

        // Add timing information
        let elapsed = Self::get_time_ms() - start_time;
        result.map(|mut r| {
            r.extraction_time_ms = elapsed;
            r
        })
    }

    /// Optimized auto extraction entry point (no upfront analysis)
    async fn extract_auto_optimized(
        &self,
        data: &[u8],
    ) -> Result<ExtractionResult, ExtractionError> {
        // Quick analysis (byte scan only, no parsing)
        let quick = quick_analyze(data);

        match quick {
            QuickAnalysis::Invalid => Err(ExtractionError::ParseError(
                "Invalid PDF header".to_string(),
            )),

            QuickAnalysis::UseLegacy => {
                // Fast path: small/simple file, go straight to Legacy
                // No lopdf parsing overhead!
                self.extract_legacy(data).await
            }

            QuickAnalysis::ProbablyLegacy => {
                // Likely simple, try Legacy first with validation
                match self.extract_legacy(data).await {
                    Ok(result) => {
                        if self.config.validate_output {
                            let validation = self.legacy.validate_output(&result.pages);
                            if !validation.is_valid {
                                // Legacy produced garbage, need full analysis
                                return self.extract_with_shared_document(data).await;
                            }
                        }
                        Ok(result)
                    }
                    Err(_) => {
                        // Legacy failed, try with full analysis
                        self.extract_with_shared_document(data).await
                    }
                }
            }

            QuickAnalysis::NeedsFullAnalysis => {
                // Complex file detected (Identity-H), need full analysis
                self.extract_with_shared_document(data).await
            }
        }
    }

    /// Legacy-only extraction
    async fn extract_legacy(&self, data: &[u8]) -> Result<ExtractionResult, ExtractionError> {
        let pages = self.legacy.extract_sync(data)?;

        if self.config.validate_output {
            let validation = self.legacy.validate_output(&pages);
            if !validation.is_valid {
                return Err(ExtractionError::GarbageOutput {
                    sample: pages
                        .first()
                        .map(|p| p.raw_text.chars().take(50).collect())
                        .unwrap_or_default(),
                    confidence: validation.garbage_ratio,
                });
            }
        }

        Ok(ExtractionResult::new("legacy").with_pages(pages))
    }

    /// Native-only extraction (no fallback)
    async fn extract_native_only(&self, data: &[u8]) -> Result<ExtractionResult, ExtractionError> {
        let pages = self.native.extract_sync(data)?;

        if self.config.validate_output {
            let validation = self.native.validate_output(&pages);
            if !validation.is_valid {
                return Err(ExtractionError::GarbageOutput {
                    sample: pages
                        .first()
                        .map(|p| p.raw_text.chars().take(50).collect())
                        .unwrap_or_default(),
                    confidence: validation.garbage_ratio,
                });
            }
        }

        Ok(ExtractionResult::new("native").with_pages(pages))
    }

    /// Browser-only extraction
    async fn extract_browser_only(&self, data: &[u8]) -> Result<ExtractionResult, ExtractionError> {
        if !BrowserExtractor::is_available() {
            return Err(ExtractionError::BackendUnavailable(
                "pdf.js not available in this environment".to_string(),
            ));
        }

        let pages = self.browser.extract_with_pdfjs(data).await?;
        Ok(ExtractionResult::new("browser").with_pages(pages))
    }

    /// Hybrid extraction with intelligent fallback
    async fn extract_hybrid(
        &self,
        data: &[u8],
        _analysis: &PdfAnalysis,
    ) -> Result<ExtractionResult, ExtractionError> {
        // First, try native extraction
        match self.native.extract_sync(data) {
            Ok(pages) => {
                // Validate output quality
                if self.config.validate_output {
                    let validation = self.native.validate_output(&pages);
                    if !validation.is_valid {
                        // Native produced garbage, fall back to browser
                        if self.config.enable_browser_fallback && BrowserExtractor::is_available() {
                            let browser_pages = self.browser.extract_with_pdfjs(data).await?;
                            return Ok(ExtractionResult::new("browser")
                                .with_pages(browser_pages)
                                .with_fallback(true));
                        }
                        // No browser available, try legacy as last resort
                        return self.try_legacy_fallback(data, true).await;
                    }
                }
                Ok(ExtractionResult::new("native").with_pages(pages))
            }
            Err(e) => {
                // Native failed, check error type
                match &e {
                    ExtractionError::EncodingFailure { recoverable, .. } if *recoverable => {
                        // Encoding issue - browser should handle this better
                        if self.config.enable_browser_fallback && BrowserExtractor::is_available() {
                            let pages = self.browser.extract_with_pdfjs(data).await?;
                            return Ok(ExtractionResult::new("browser")
                                .with_pages(pages)
                                .with_fallback(true));
                        }
                    }
                    ExtractionError::BackendUnavailable(_) => {
                        // Native not available, go straight to legacy or browser
                        if BrowserExtractor::is_available() {
                            let pages = self.browser.extract_with_pdfjs(data).await?;
                            return Ok(ExtractionResult::new("browser")
                                .with_pages(pages)
                                .with_fallback(true));
                        }
                        return self.try_legacy_fallback(data, true).await;
                    }
                    _ => {}
                }

                // Try browser fallback
                if self.config.enable_browser_fallback && BrowserExtractor::is_available() {
                    match self.browser.extract_with_pdfjs(data).await {
                        Ok(pages) => {
                            return Ok(ExtractionResult::new("browser")
                                .with_pages(pages)
                                .with_fallback(true));
                        }
                        Err(_) => {
                            // Browser also failed, try legacy as last resort
                            return self.try_legacy_fallback(data, true).await;
                        }
                    }
                }

                // No fallbacks available
                Err(e)
            }
        }
    }

    /// Extract with shared Document (parse once, use for both analysis and extraction)
    async fn extract_with_shared_document(
        &self,
        data: &[u8],
    ) -> Result<ExtractionResult, ExtractionError> {
        // Parse once and get both Document and Analysis
        let (doc, analysis) =
            PdfAnalysis::parse_and_analyze(data).map_err(ExtractionError::ParseError)?;

        match analysis.difficulty {
            ExtractionDifficulty::Easy | ExtractionDifficulty::Medium => {
                // Try native extraction with shared Document (no re-parse!)
                match self.native.extract_from_document(&doc) {
                    Ok(pages) => {
                        if self.config.validate_output {
                            let validation = self.native.validate_output(&pages);
                            if !validation.is_valid {
                                // Native produced garbage, try browser
                                if self.config.enable_browser_fallback
                                    && BrowserExtractor::is_available()
                                {
                                    let browser_pages =
                                        self.browser.extract_with_pdfjs(data).await?;
                                    return Ok(ExtractionResult::new("browser")
                                        .with_pages(browser_pages)
                                        .with_fallback(true));
                                }
                            }
                        }
                        Ok(ExtractionResult::new("native").with_pages(pages))
                    }
                    Err(_) => {
                        // Native failed, try browser
                        if self.config.enable_browser_fallback && BrowserExtractor::is_available() {
                            let pages = self.browser.extract_with_pdfjs(data).await?;
                            return Ok(ExtractionResult::new("browser")
                                .with_pages(pages)
                                .with_fallback(true));
                        }
                        // Last resort: legacy
                        self.try_legacy_fallback(data, true).await
                    }
                }
            }

            ExtractionDifficulty::Hard | ExtractionDifficulty::VeryHard => {
                // Hard PDFs - go straight to browser if available
                if BrowserExtractor::is_available() {
                    let pages = self.browser.extract_with_pdfjs(data).await?;
                    Ok(ExtractionResult::new("browser").with_pages(pages))
                } else {
                    // Try native with shared Document as fallback
                    match self.native.extract_from_document(&doc) {
                        Ok(pages) => Ok(ExtractionResult::new("native")
                            .with_pages(pages)
                            .with_fallback(true)),
                        Err(_) => self.try_legacy_fallback(data, true).await,
                    }
                }
            }

            ExtractionDifficulty::RequiresOcr => Err(ExtractionError::Other(
                "Document appears to be scanned. OCR support not yet implemented.".to_string(),
            )),
        }
    }

    /// Try legacy extraction as a fallback
    async fn try_legacy_fallback(
        &self,
        data: &[u8],
        is_fallback: bool,
    ) -> Result<ExtractionResult, ExtractionError> {
        let pages = self.legacy.extract_sync(data)?;
        Ok(ExtractionResult::new("legacy")
            .with_pages(pages)
            .with_fallback(is_fallback))
    }

    /// Get current time in milliseconds
    fn get_time_ms() -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = window() {
                if let Some(performance) = window.performance() {
                    return performance.now();
                }
            }
            0.0
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ExtractionConfig::default();
        // Default should be Auto for intelligent routing
        assert_eq!(config.strategy, ExtractionStrategy::Auto);
        assert!(config.enable_browser_fallback);
        assert!(config.validate_output);
    }

    #[test]
    fn test_router_creation() {
        let config = ExtractionConfig::default();
        let router = ExtractionRouter::new(config);
        assert!(router.config.enable_browser_fallback);
        assert_eq!(router.config.strategy, ExtractionStrategy::Auto);
    }

    #[test]
    fn test_strategy_serialization() {
        let strategy = ExtractionStrategy::Auto;
        let json = serde_json::to_string(&strategy).unwrap();
        assert_eq!(json, "\"Auto\"");

        let strategy = ExtractionStrategy::Legacy;
        let json = serde_json::to_string(&strategy).unwrap();
        assert_eq!(json, "\"Legacy\"");
    }

    #[test]
    fn test_all_strategies() {
        // Verify all strategies can be created
        let strategies = [
            ExtractionStrategy::Legacy,
            ExtractionStrategy::Hybrid,
            ExtractionStrategy::NativeOnly,
            ExtractionStrategy::BrowserOnly,
            ExtractionStrategy::Auto,
        ];

        for strategy in strategies {
            let config = ExtractionConfig {
                strategy,
                ..Default::default()
            };
            let _router = ExtractionRouter::new(config);
        }
    }
}
