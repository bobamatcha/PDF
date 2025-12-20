//! PDF rendering utilities for WASM
//!
//! Provides page metadata extraction and rendering support.
//! Uses a hybrid approach: Rust extracts metadata, JavaScript handles canvas rendering.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Page metadata for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct PageMetadata {
    /// Page number (1-indexed)
    pub page_num: u32,
    /// Page width in PDF points (1 point = 1/72 inch)
    pub width: f64,
    /// Page height in PDF points
    pub height: f64,
    /// X offset (usually 0)
    pub x: f64,
    /// Y offset (usually 0)
    pub y: f64,
}

#[wasm_bindgen]
impl PageMetadata {
    /// Get aspect ratio (width / height)
    #[wasm_bindgen]
    pub fn aspect_ratio(&self) -> f64 {
        self.width / self.height
    }

    /// Calculate scaled dimensions given a target width
    #[wasm_bindgen]
    pub fn scale_to_width(&self, target_width: f64) -> ScaledDimensions {
        let scale = target_width / self.width;
        ScaledDimensions {
            width: target_width,
            height: self.height * scale,
            scale,
        }
    }

    /// Calculate scaled dimensions given a target height
    #[wasm_bindgen]
    pub fn scale_to_height(&self, target_height: f64) -> ScaledDimensions {
        let scale = target_height / self.height;
        ScaledDimensions {
            width: self.width * scale,
            height: target_height,
            scale,
        }
    }
}

/// Scaled dimensions with scale factor
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct ScaledDimensions {
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

/// PDF Renderer for extracting page metadata
#[wasm_bindgen]
pub struct PdfRenderer {
    doc: lopdf::Document,
}

#[wasm_bindgen]
impl PdfRenderer {
    /// Create a new PDF renderer from raw bytes
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8]) -> Result<PdfRenderer, JsValue> {
        let doc = lopdf::Document::load_mem(data)
            .map_err(|e| JsValue::from_str(&format!("Failed to load PDF: {}", e)))?;

        Ok(PdfRenderer { doc })
    }

    /// Get the total number of pages
    #[wasm_bindgen]
    pub fn page_count(&self) -> Result<u32, JsValue> {
        Ok(self.doc.get_pages().len() as u32)
    }

    /// Get metadata for a specific page (1-indexed)
    #[wasm_bindgen]
    pub fn get_page_metadata(&self, page_num: u32) -> Result<PageMetadata, JsValue> {
        let page_id = self
            .doc
            .get_pages()
            .get(&page_num)
            .copied()
            .ok_or_else(|| JsValue::from_str(&format!("Page {} not found", page_num)))?;

        let page = self
            .doc
            .get_object(page_id)
            .map_err(|e| JsValue::from_str(&format!("Failed to get page object: {}", e)))?;

        let page_dict = page
            .as_dict()
            .map_err(|_| JsValue::from_str("Page is not a dictionary"))?;

        // Get MediaBox (page dimensions)
        let media_box = self.get_media_box(page_dict, page_id)?;

        Ok(PageMetadata {
            page_num,
            x: media_box[0],
            y: media_box[1],
            width: media_box[2],
            height: media_box[3],
        })
    }

    /// Get metadata for all pages
    #[wasm_bindgen]
    pub fn get_all_page_metadata(&self) -> Result<JsValue, JsValue> {
        let page_count = self.doc.get_pages().len() as u32;
        let mut metadata_list = Vec::new();

        for page_num in 1..=page_count {
            let metadata = self.get_page_metadata(page_num)?;
            metadata_list.push(metadata);
        }

        serde_wasm_bindgen::to_value(&metadata_list)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize metadata: {}", e)))
    }
}

// Internal implementation (not exposed to WASM)
impl PdfRenderer {
    /// Extract MediaBox from page dictionary, traversing parent if needed
    /// Based on docsign's parser.rs implementation
    fn get_media_box(
        &self,
        page_dict: &lopdf::Dictionary,
        _page_id: lopdf::ObjectId,
    ) -> Result<[f64; 4], JsValue> {
        // First try direct MediaBox
        if let Ok(media_box) = page_dict.get(b"MediaBox") {
            return self.parse_rect(media_box);
        }

        // Try to get from Parent
        if let Ok(parent_ref) = page_dict.get(b"Parent") {
            if let Ok(parent_id) = parent_ref.as_reference() {
                if let Ok(parent) = self.doc.get_object(parent_id) {
                    if let Ok(parent_dict) = parent.as_dict() {
                        if let Ok(media_box) = parent_dict.get(b"MediaBox") {
                            return self.parse_rect(media_box);
                        }
                    }
                }
            }
        }

        // Default to US Letter size
        Ok([0.0, 0.0, 612.0, 792.0])
    }

    /// Parse a PDF rectangle array into [x, y, width, height]
    fn parse_rect(&self, obj: &lopdf::Object) -> Result<[f64; 4], JsValue> {
        let arr = match obj {
            lopdf::Object::Array(a) => a,
            lopdf::Object::Reference(id) => {
                let resolved = self.doc.get_object(*id).map_err(|e| {
                    JsValue::from_str(&format!("Failed to resolve reference: {}", e))
                })?;
                resolved
                    .as_array()
                    .map_err(|_| JsValue::from_str("MediaBox reference is not an array"))?
            }
            _ => return Err(JsValue::from_str("MediaBox is not an array")),
        };

        if arr.len() != 4 {
            return Err(JsValue::from_str(&format!(
                "MediaBox has {} elements, expected 4",
                arr.len()
            )));
        }

        let mut values = [0.0f64; 4];
        for (i, obj) in arr.iter().enumerate() {
            values[i] = self.extract_number(obj)?;
        }

        // Convert from [x1, y1, x2, y2] to [x, y, width, height]
        Ok([
            values[0],
            values[1],
            values[2] - values[0], // width
            values[3] - values[1], // height
        ])
    }

    /// Extract a number from a PDF object
    fn extract_number(&self, obj: &lopdf::Object) -> Result<f64, JsValue> {
        match obj {
            lopdf::Object::Integer(i) => Ok(*i as f64),
            lopdf::Object::Real(r) => Ok(*r as f64),
            lopdf::Object::Reference(id) => {
                let resolved = self
                    .doc
                    .get_object(*id)
                    .map_err(|e| JsValue::from_str(&format!("Failed to resolve: {}", e)))?;
                self.extract_number(resolved)
            }
            _ => Err(JsValue::from_str("Expected number in rectangle")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    // Helper function to create a minimal valid PDF for testing
    fn create_test_pdf() -> Vec<u8> {
        // Create a minimal PDF with 2 pages using lopdf
        let mut doc = lopdf::Document::with_version("1.5");

        // Create two pages with different dimensions
        let pages_id = doc.new_object_id();
        let page1_id = doc.new_object_id();
        let page2_id = doc.new_object_id();

        // Page 1: US Letter (612 x 792 points)
        let page1 = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        };
        doc.objects
            .insert(page1_id, lopdf::Object::Dictionary(page1));

        // Page 2: A4 (595 x 842 points)
        let page2 = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects
            .insert(page2_id, lopdf::Object::Dictionary(page2));

        // Pages collection
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page1_id.into(), page2_id.into()],
            "Count" => 2,
        };
        doc.objects
            .insert(pages_id, lopdf::Object::Dictionary(pages));

        // Catalog
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });

        // Trailer
        doc.trailer.set("Root", catalog_id);

        // Save to bytes
        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn test_parse_rect_array() {
        let pdf_data = create_test_pdf();
        let doc = lopdf::Document::load_mem(&pdf_data).unwrap();
        let renderer = PdfRenderer { doc };

        // Test with a valid array
        let arr = lopdf::Object::Array(vec![
            lopdf::Object::Integer(0),
            lopdf::Object::Integer(0),
            lopdf::Object::Integer(612),
            lopdf::Object::Integer(792),
        ]);

        let result = renderer.parse_rect(&arr);
        assert!(result.is_ok());
        let dims = result.unwrap();
        assert_eq!(dims[0], 0.0);
        assert_eq!(dims[1], 0.0);
        assert_eq!(dims[2], 612.0); // width
        assert_eq!(dims[3], 792.0); // height
    }

    #[test]
    fn test_extract_number() {
        let pdf_data = create_test_pdf();
        let doc = lopdf::Document::load_mem(&pdf_data).unwrap();
        let renderer = PdfRenderer { doc };

        // Test integer extraction
        let int_obj = lopdf::Object::Integer(42);
        assert_eq!(renderer.extract_number(&int_obj).unwrap(), 42.0);

        // Test real extraction
        let real_obj = lopdf::Object::Real(1.234);
        assert!((renderer.extract_number(&real_obj).unwrap() - 1.234).abs() < 0.001);
    }

    #[test]
    fn test_page_metadata_aspect_ratio() {
        // Test the helper methods that should work
        let metadata = PageMetadata {
            page_num: 1,
            width: 612.0,
            height: 792.0,
            x: 0.0,
            y: 0.0,
        };

        let aspect_ratio = metadata.aspect_ratio();
        assert!((aspect_ratio - 0.7727).abs() < 0.001); // 612/792 = 0.7727
    }

    #[test]
    fn test_scale_to_width() {
        let metadata = PageMetadata {
            page_num: 1,
            width: 612.0,
            height: 792.0,
            x: 0.0,
            y: 0.0,
        };

        let scaled = metadata.scale_to_width(800.0);
        assert_eq!(scaled.width, 800.0);
        assert!((scaled.height - 1035.29).abs() < 0.01);
        assert!((scaled.scale - 1.307).abs() < 0.001);
    }

    #[test]
    fn test_scale_to_height() {
        let metadata = PageMetadata {
            page_num: 1,
            width: 612.0,
            height: 792.0,
            x: 0.0,
            y: 0.0,
        };

        let scaled = metadata.scale_to_height(1000.0);
        assert!((scaled.width - 772.73).abs() < 0.01);
        assert_eq!(scaled.height, 1000.0);
        assert!((scaled.scale - 1.262).abs() < 0.001);
    }
}
