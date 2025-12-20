//! Overlay management for interactive PDF elements
//!
//! This module provides the OverlayManager which manages HTML overlay positioning
//! on top of PDF canvases, handling coordinate transformations between PDF and DOM space.

use crate::coords::{dom_to_pdf, pdf_to_dom};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlElement, Window};

/// Page information for coordinate transformation
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// Page number (1-indexed)
    pub page_num: u32,
    /// PDF page width in points
    pub pdf_width: f64,
    /// PDF page height in points
    pub pdf_height: f64,
    /// Container width in pixels
    pub container_width: f64,
    /// Container height in pixels
    pub container_height: f64,
}

impl PageInfo {
    /// Create a new PageInfo
    pub fn new(
        page_num: u32,
        pdf_width: f64,
        pdf_height: f64,
        container_width: f64,
        container_height: f64,
    ) -> Self {
        Self {
            page_num,
            pdf_width,
            pdf_height,
            container_width,
            container_height,
        }
    }

    /// Get the media box for this page
    pub fn media_box(&self) -> [f64; 4] {
        [0.0, 0.0, self.pdf_width, self.pdf_height]
    }
}

/// Manages overlay divs for PDF pages
#[allow(dead_code)]
pub struct OverlayManager {
    pages: HashMap<u32, PageInfo>,
    window: Window,
    document: Document,
}

impl OverlayManager {
    /// Create a new OverlayManager
    ///
    /// # Errors
    /// Returns JsValue error if unable to access window or document
    pub fn new() -> Result<Self, JsValue> {
        let window =
            web_sys::window().ok_or_else(|| JsValue::from_str("No window object available"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("No document object available"))?;

        Ok(Self {
            pages: HashMap::new(),
            window,
            document,
        })
    }

    /// Get the number of registered pages
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Register a page with its dimensions
    pub fn register_page(
        &mut self,
        page_num: u32,
        pdf_width: f64,
        pdf_height: f64,
        container_width: f64,
        container_height: f64,
    ) {
        let page_info = PageInfo::new(
            page_num,
            pdf_width,
            pdf_height,
            container_width,
            container_height,
        );
        self.pages.insert(page_num, page_info);
    }

    /// Create an overlay div for a specific page
    ///
    /// # Arguments
    /// * `page_num` - The page number to create overlay for
    ///
    /// # Returns
    /// The created overlay Element
    ///
    /// # Errors
    /// Returns JsValue error if unable to create or configure the element
    pub fn create_overlay(&self, page_num: u32) -> Result<Element, JsValue> {
        let overlay = self.document.create_element("div")?;
        overlay.set_class_name("overlay-container");
        overlay.set_id(&format!("overlay-page-{}", page_num));

        // Set positioning styles
        if let Some(html_element) = overlay.dyn_ref::<HtmlElement>() {
            let style = html_element.style();
            style.set_property("position", "absolute")?;
            style.set_property("top", "0")?;
            style.set_property("left", "0")?;
            style.set_property("width", "100%")?;
            style.set_property("height", "100%")?;
            style.set_property("pointer-events", "none")?;
        }

        Ok(overlay)
    }

    /// Position an element on the overlay using PDF coordinates
    ///
    /// # Arguments
    /// * `element` - The HTML element to position
    /// * `page_num` - The page number this element belongs to
    /// * `pdf_x` - X coordinate in PDF space
    /// * `pdf_y` - Y coordinate in PDF space
    ///
    /// # Errors
    /// Returns JsValue error if page not found or positioning fails
    pub fn position_element(
        &self,
        element: &Element,
        page_num: u32,
        pdf_x: f64,
        pdf_y: f64,
    ) -> Result<(), JsValue> {
        let page_info = self
            .pages
            .get(&page_num)
            .ok_or_else(|| JsValue::from_str(&format!("Page {} not registered", page_num)))?;

        let (dom_x, dom_y) = pdf_to_dom(
            pdf_x,
            pdf_y,
            page_info.container_width,
            page_info.container_height,
            page_info.media_box(),
        );

        if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
            let style = html_element.style();
            style.set_property("position", "absolute")?;
            style.set_property("left", &format!("{}px", dom_x))?;
            style.set_property("top", &format!("{}px", dom_y))?;
            style.set_property("pointer-events", "auto")?;
        }

        Ok(())
    }

    /// Convert DOM coordinates (e.g., from a click event) to PDF coordinates
    ///
    /// # Arguments
    /// * `page_num` - The page number
    /// * `dom_x` - X coordinate in DOM space
    /// * `dom_y` - Y coordinate in DOM space
    ///
    /// # Returns
    /// Tuple of (pdf_x, pdf_y) coordinates
    ///
    /// # Errors
    /// Returns JsValue error if page not found
    pub fn get_pdf_coordinates(
        &self,
        page_num: u32,
        dom_x: f64,
        dom_y: f64,
    ) -> Result<(f64, f64), JsValue> {
        let page_info = self
            .pages
            .get(&page_num)
            .ok_or_else(|| JsValue::from_str(&format!("Page {} not registered", page_num)))?;

        let coords = dom_to_pdf(
            dom_x,
            dom_y,
            page_info.container_width,
            page_info.container_height,
            page_info.media_box(),
        );

        Ok(coords)
    }

    /// Get page info for a specific page
    pub fn get_page_info(&self, page_num: u32) -> Option<&PageInfo> {
        self.pages.get(&page_num)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_info_creation() {
        let page = PageInfo::new(1, 612.0, 792.0, 612.0, 792.0);
        assert_eq!(page.page_num, 1);
        assert_eq!(page.pdf_width, 612.0);
        assert_eq!(page.pdf_height, 792.0);
        assert_eq!(page.container_width, 612.0);
        assert_eq!(page.container_height, 792.0);
    }

    #[test]
    fn test_page_info_media_box() {
        let page = PageInfo::new(1, 612.0, 792.0, 612.0, 792.0);
        let media_box = page.media_box();
        assert_eq!(media_box, [0.0, 0.0, 612.0, 792.0]);
    }

    // Note: OverlayManager tests that require DOM APIs are skipped in non-WASM environment
    // These would need to be run with wasm-bindgen-test
}

// WASM-specific tests that run in a browser environment
#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_overlay_manager_creation() {
        let manager = OverlayManager::new();
        assert!(manager.is_ok());
        let manager = manager.unwrap();
        assert_eq!(manager.page_count(), 0);
    }

    #[wasm_bindgen_test]
    fn test_register_page() {
        let mut manager = OverlayManager::new().unwrap();
        manager.register_page(1, 612.0, 792.0, 612.0, 792.0); // Letter size
        assert_eq!(manager.page_count(), 1);

        let page_info = manager.get_page_info(1);
        assert!(page_info.is_some());
        let page_info = page_info.unwrap();
        assert_eq!(page_info.pdf_width, 612.0);
        assert_eq!(page_info.pdf_height, 792.0);
    }

    #[wasm_bindgen_test]
    fn test_create_overlay() {
        let manager = OverlayManager::new().unwrap();
        let overlay = manager.create_overlay(1);
        assert!(overlay.is_ok());
        let overlay = overlay.unwrap();
        assert_eq!(overlay.id(), "overlay-page-1");
        assert_eq!(overlay.class_name(), "overlay-container");
    }

    #[wasm_bindgen_test]
    fn test_get_pdf_coordinates() {
        let mut manager = OverlayManager::new().unwrap();
        manager.register_page(1, 612.0, 792.0, 612.0, 792.0);

        // Test center point
        let result = manager.get_pdf_coordinates(1, 306.0, 396.0);
        assert!(result.is_ok());
        let (pdf_x, pdf_y) = result.unwrap();
        assert!((pdf_x - 306.0).abs() < 0.1);
        assert!((pdf_y - 396.0).abs() < 0.1);
    }
}
