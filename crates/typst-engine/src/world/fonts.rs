//! Font loading and caching
//!
//! This module manages font loading and provides a shared font cache
//! that can be reused across compilation requests.

use std::sync::OnceLock;

use typst::foundations::Bytes;
use typst::text::{Font, FontBook, FontInfo};

/// Global font cache singleton
static FONT_CACHE: OnceLock<FontCache> = OnceLock::new();

/// Get the global font cache, initializing it if necessary
pub fn global_font_cache() -> &'static FontCache {
    FONT_CACHE.get_or_init(FontCache::new)
}

/// A cache of fonts available for compilation
#[derive(Debug)]
pub struct FontCache {
    /// The font book containing metadata about available fonts
    book: FontBook,
    /// The actual font data
    fonts: Vec<Font>,
}

impl FontCache {
    /// Create a new font cache with embedded fonts
    pub fn new() -> Self {
        let mut book = FontBook::new();
        let mut fonts = Vec::new();

        // Load embedded fonts
        Self::load_embedded_fonts(&mut book, &mut fonts);

        // Optionally load system fonts (disabled by default for reproducibility)
        // Self::load_system_fonts(&mut book, &mut fonts);

        tracing::info!("Font cache initialized with {} fonts", fonts.len());

        Self { book, fonts }
    }

    /// Load fonts embedded in the binary
    fn load_embedded_fonts(book: &mut FontBook, fonts: &mut Vec<Font>) {
        // Load from typst-assets crate (provides Liberation fonts, DejaVu, New Computer Modern, etc.)
        for data in typst_assets::fonts() {
            let buffer = Bytes::from_static(data);
            for font in Font::iter(buffer) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }

        // To add custom fonts from assets/fonts directory, uncomment and add:
        // Example for embedding custom fonts at compile time:
        //
        // const CUSTOM_FONT: &[u8] = include_bytes!("../../assets/fonts/MyFont-Regular.ttf");
        // let buffer = Bytes::from_static(CUSTOM_FONT);
        // for font in Font::iter(buffer) {
        //     book.push(font.info().clone());
        //     fonts.push(font);
        // }
    }

    /// Load system fonts (optional, can break reproducibility)
    #[allow(dead_code)]
    fn load_system_fonts(book: &mut FontBook, fonts: &mut Vec<Font>) {
        // Search common font directories
        let font_dirs = [
            "/usr/share/fonts",
            "/usr/local/share/fonts",
            "/Library/Fonts",
            "/System/Library/Fonts",
        ];

        for dir in font_dirs {
            let path = std::path::Path::new(dir);
            if path.exists() {
                Self::scan_font_dir(path, book, fonts);
            }
        }
    }

    /// Recursively scan a directory for font files
    #[allow(dead_code)]
    fn scan_font_dir(dir: &std::path::Path, book: &mut FontBook, fonts: &mut Vec<Font>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                Self::scan_font_dir(&path, book, fonts);
            } else if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "otc") {
                    Self::load_font_file(&path, book, fonts);
                }
            }
        }
    }

    /// Load a single font file
    #[allow(dead_code)]
    fn load_font_file(path: &std::path::Path, book: &mut FontBook, fonts: &mut Vec<Font>) {
        let Ok(data) = std::fs::read(path) else {
            return;
        };

        let buffer = Bytes::from(data);
        for font in Font::iter(buffer) {
            book.push(font.info().clone());
            fonts.push(font);
        }
    }

    /// Get the font book
    pub fn book(&self) -> &FontBook {
        &self.book
    }

    /// Get a font by index
    pub fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    /// Get the number of fonts
    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    /// List all font families
    pub fn list_font_families(&self) -> Vec<String> {
        let mut families: Vec<String> = self
            .book
            .families()
            .map(|(name, _)| name.to_string())
            .collect();

        families.sort();
        families.dedup();
        families
    }

    /// Search for fonts by family name
    pub fn find_by_family(&self, family: &str) -> Vec<FontInfo> {
        self.fonts
            .iter()
            .filter(|font| font.info().family.eq_ignore_ascii_case(family))
            .map(|font| font.info().clone())
            .collect()
    }

    /// Get detailed information about all available fonts
    pub fn list_all_fonts(&self) -> Vec<FontDetail> {
        self.fonts
            .iter()
            .map(|font| {
                let info = font.info();
                FontDetail {
                    family: info.family.clone(),
                    variant: info.variant,
                    style: format!("{:?}", info.variant.style),
                    weight: info.variant.weight.to_number(),
                    stretch: format!("{:?}", info.variant.stretch),
                }
            })
            .collect()
    }
}

/// Detailed font information for listing
#[derive(Debug, Clone)]
pub struct FontDetail {
    /// Font family name
    pub family: String,
    /// Font variant (style, weight, stretch)
    pub variant: typst::text::FontVariant,
    /// Style (e.g., Normal, Italic, Oblique)
    pub style: String,
    /// Weight (100-900)
    pub weight: u16,
    /// Stretch (e.g., Normal, Condensed, Expanded)
    pub stretch: String,
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_cache_creation() {
        let cache = FontCache::new();
        // Should have at least some fonts from typst-assets
        assert!(!cache.is_empty(), "Font cache should not be empty");
    }

    #[test]
    fn test_list_families() {
        let cache = FontCache::new();
        let families = cache.list_font_families();

        // typst-assets provides several font families
        assert!(!families.is_empty(), "Should have font families");
    }

    #[test]
    fn test_global_cache_singleton() {
        let cache1 = global_font_cache();
        let cache2 = global_font_cache();

        // Should be the same instance
        assert!(std::ptr::eq(cache1, cache2));
    }
}
