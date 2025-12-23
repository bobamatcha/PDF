//! Operation log for tracking PDF edit operations
//!
//! This module provides data structures for tracking PDF editing operations
//! such as adding text, highlights, and checkboxes to PDF pages.

use serde::{Deserialize, Serialize};

pub type OpId = u64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PdfRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextStyle {
    pub font_size: f64,
    pub color: String,
    /// Font name for text rendering. Will be mapped to PDF standard fonts.
    #[serde(default)]
    pub font_name: Option<String>,
    /// Whether the text should be italic
    #[serde(default)]
    pub is_italic: bool,
    /// Whether the text should be bold
    #[serde(default)]
    pub is_bold: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            color: "#000000".to_string(),
            font_name: None,
            is_italic: false,
            is_bold: false,
        }
    }
}

impl TextStyle {
    /// Map a PDF.js font name to a PDF standard font name.
    /// PDF.js returns names like "g_d0_f1", "Times-Roman", "BCDEEE+ArialMT", etc.
    /// We map these to the PDF standard 14 fonts for maximum compatibility.
    /// Also considers is_italic and is_bold flags for proper font variant selection.
    pub fn pdf_font_name(&self) -> &'static str {
        let base_font = match &self.font_name {
            Some(name) => {
                // First check if the name already specifies a style
                let lower = name.to_lowercase();
                if lower.contains("italic") || lower.contains("bold") || lower.contains("oblique") {
                    // Font name already includes style info, use existing logic
                    return map_to_standard_font(name);
                }
                // Get base font family
                map_font_family_to_base(name)
            }
            None => "Helvetica",
        };

        // Apply italic/bold based on flags
        match base_font {
            "Times-Roman" | "Times" => match (self.is_bold, self.is_italic) {
                (true, true) => "Times-BoldItalic",
                (true, false) => "Times-Bold",
                (false, true) => "Times-Italic",
                (false, false) => "Times-Roman",
            },
            "Helvetica" => match (self.is_bold, self.is_italic) {
                (true, true) => "Helvetica-BoldOblique",
                (true, false) => "Helvetica-Bold",
                (false, true) => "Helvetica-Oblique",
                (false, false) => "Helvetica",
            },
            "Courier" => match (self.is_bold, self.is_italic) {
                (true, true) => "Courier-BoldOblique",
                (true, false) => "Courier-Bold",
                (false, true) => "Courier-Oblique",
                (false, false) => "Courier",
            },
            _ => base_font,
        }
    }
}

/// Map font family name to base PDF font (without style variants)
fn map_font_family_to_base(name: &str) -> &'static str {
    let lower = name.to_lowercase();

    // Handle CSS generic font families
    match lower.as_str() {
        "serif" => return "Times-Roman",
        "sans-serif" => return "Helvetica",
        "monospace" => return "Courier",
        "cursive" | "fantasy" => return "Helvetica",
        _ => {}
    }

    // Check for Times/serif
    if lower.contains("times") || lower.contains("georgia") || lower.contains("garamond") {
        return "Times-Roman";
    }

    // Check for Courier/monospace
    if lower.contains("courier")
        || lower.contains("mono")
        || lower.contains("consolas")
        || lower.contains("monaco")
    {
        return "Courier";
    }

    // Check for Helvetica/sans-serif
    if lower.contains("arial")
        || lower.contains("helvetica")
        || lower.contains("sans")
        || lower.contains("gothic")
    {
        return "Helvetica";
    }

    // Default
    "Helvetica"
}

/// Map font family/name to PDF standard 14 fonts
/// Handles both CSS generic families ("serif", "sans-serif", "monospace")
/// and specific font names ("Times-Roman", "Arial", etc.)
fn map_to_standard_font(name: &str) -> &'static str {
    let lower = name.to_lowercase();

    // Handle CSS generic font families (from PDF.js styles)
    match lower.as_str() {
        "serif" => return "Times-Roman",
        "sans-serif" => return "Helvetica",
        "monospace" => return "Courier",
        "cursive" | "fantasy" => return "Helvetica", // Fallback
        _ => {}
    }

    // Check for Times variants
    if lower.contains("times") {
        if lower.contains("bold") && lower.contains("italic") {
            return "Times-BoldItalic";
        } else if lower.contains("bold") {
            return "Times-Bold";
        } else if lower.contains("italic") || lower.contains("oblique") {
            return "Times-Italic";
        }
        return "Times-Roman";
    }

    // Check for Courier/monospace variants
    if lower.contains("courier")
        || lower.contains("mono")
        || lower.contains("consolas")
        || lower.contains("monaco")
    {
        if lower.contains("bold") && lower.contains("italic") {
            return "Courier-BoldOblique";
        } else if lower.contains("bold") {
            return "Courier-Bold";
        } else if lower.contains("italic") || lower.contains("oblique") {
            return "Courier-Oblique";
        }
        return "Courier";
    }

    // Check for Arial/Helvetica/sans-serif variants
    if lower.contains("arial")
        || lower.contains("helvetica")
        || lower.contains("sans")
        || lower.contains("gothic")
    {
        if lower.contains("bold") && lower.contains("italic") {
            return "Helvetica-BoldOblique";
        } else if lower.contains("bold") {
            return "Helvetica-Bold";
        } else if lower.contains("italic") || lower.contains("oblique") {
            return "Helvetica-Oblique";
        }
        return "Helvetica";
    }

    // Symbol fonts
    if lower.contains("symbol") {
        return "Symbol";
    }
    if lower.contains("zapf") || lower.contains("dingbat") {
        return "ZapfDingbats";
    }

    // Default to Helvetica (most compatible sans-serif)
    "Helvetica"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum EditOperation {
    AddText {
        id: OpId,
        page: u32,
        rect: PdfRect,
        text: String,
        style: TextStyle,
    },
    AddHighlight {
        id: OpId,
        page: u32,
        rect: PdfRect,
        color: String,
        opacity: f64,
    },
    AddCheckbox {
        id: OpId,
        page: u32,
        rect: PdfRect,
        checked: bool,
    },
    ReplaceText {
        id: OpId,
        page: u32,
        original_rect: PdfRect,
        replacement_rect: PdfRect,
        original_text: String,
        new_text: String,
        style: TextStyle,
    },
    /// Add a white rectangle to cover/redact content
    AddWhiteRect { id: OpId, page: u32, rect: PdfRect },
}

impl EditOperation {
    pub fn id(&self) -> OpId {
        match self {
            EditOperation::AddText { id, .. } => *id,
            EditOperation::AddHighlight { id, .. } => *id,
            EditOperation::AddCheckbox { id, .. } => *id,
            EditOperation::ReplaceText { id, .. } => *id,
            EditOperation::AddWhiteRect { id, .. } => *id,
        }
    }

    pub fn page(&self) -> u32 {
        match self {
            EditOperation::AddText { page, .. } => *page,
            EditOperation::AddHighlight { page, .. } => *page,
            EditOperation::AddCheckbox { page, .. } => *page,
            EditOperation::ReplaceText { page, .. } => *page,
            EditOperation::AddWhiteRect { page, .. } => *page,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationLog {
    next_id: OpId,
    operations: Vec<EditOperation>,
}

impl OperationLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, mut op: EditOperation) -> OpId {
        let id = self.next_id;
        self.next_id += 1;

        // Update the operation's id
        match &mut op {
            EditOperation::AddText { id: op_id, .. } => *op_id = id,
            EditOperation::AddHighlight { id: op_id, .. } => *op_id = id,
            EditOperation::AddCheckbox { id: op_id, .. } => *op_id = id,
            EditOperation::ReplaceText { id: op_id, .. } => *op_id = id,
            EditOperation::AddWhiteRect { id: op_id, .. } => *op_id = id,
        }

        self.operations.push(op);
        id
    }

    pub fn remove(&mut self, id: OpId) -> bool {
        if let Some(pos) = self.operations.iter().position(|op| op.id() == id) {
            self.operations.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn operations(&self) -> &[EditOperation] {
        &self.operations
    }

    pub fn operations_for_page(&self, page: u32) -> Vec<&EditOperation> {
        self.operations
            .iter()
            .filter(|op| op.page() == page)
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_log_new_is_empty() {
        let log = OperationLog::new();
        assert!(log.is_empty());
        assert_eq!(log.operations().len(), 0);
    }

    #[test]
    fn test_add_operation_returns_unique_id() {
        let mut log = OperationLog::new();
        let id1 = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            text: "Hello".to_string(),
            style: TextStyle::default(),
        });
        let id2 = log.add(EditOperation::AddCheckbox {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 50.0,
                y: 50.0,
                width: 20.0,
                height: 20.0,
            },
            checked: true,
        });
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_remove_operation() {
        let mut log = OperationLog::new();
        let id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            text: "Test".to_string(),
            style: TextStyle::default(),
        });
        assert!(!log.is_empty());
        assert!(log.remove(id));
        assert!(log.is_empty());
    }

    #[test]
    fn test_operations_for_page() {
        let mut log = OperationLog::new();
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            text: "Page 1".to_string(),
            style: TextStyle::default(),
        });
        log.add(EditOperation::AddText {
            id: 0,
            page: 2,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            text: "Page 2".to_string(),
            style: TextStyle::default(),
        });
        log.add(EditOperation::AddCheckbox {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 50.0,
                y: 50.0,
                width: 20.0,
                height: 20.0,
            },
            checked: false,
        });

        let page1_ops = log.operations_for_page(1);
        assert_eq!(page1_ops.len(), 2);

        let page2_ops = log.operations_for_page(2);
        assert_eq!(page2_ops.len(), 1);
    }

    #[test]
    fn test_json_roundtrip() {
        let mut log = OperationLog::new();
        log.add(EditOperation::AddHighlight {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 15.0,
            },
            color: "#FFFF00".to_string(),
            opacity: 0.5,
        });

        let json = log.to_json().unwrap();
        let restored = OperationLog::from_json(&json).unwrap();

        assert_eq!(log.operations().len(), restored.operations().len());
    }

    // ============ Font Preservation Tests ============

    #[test]
    fn test_font_mapping_css_generic_families() {
        // CSS generic font families from PDF.js styles
        assert_eq!(map_to_standard_font("serif"), "Times-Roman");
        assert_eq!(map_to_standard_font("sans-serif"), "Helvetica");
        assert_eq!(map_to_standard_font("monospace"), "Courier");
        assert_eq!(map_to_standard_font("cursive"), "Helvetica"); // fallback
        assert_eq!(map_to_standard_font("fantasy"), "Helvetica"); // fallback
    }

    #[test]
    fn test_font_mapping_times_variants() {
        assert_eq!(map_to_standard_font("Times-Roman"), "Times-Roman");
        assert_eq!(map_to_standard_font("Times-Bold"), "Times-Bold");
        assert_eq!(map_to_standard_font("Times-Italic"), "Times-Italic");
        assert_eq!(map_to_standard_font("Times-BoldItalic"), "Times-BoldItalic");
        assert_eq!(map_to_standard_font("TimesNewRoman"), "Times-Roman");
        assert_eq!(
            map_to_standard_font("BCDEEE+TimesNewRomanPSMT"),
            "Times-Roman"
        );
    }

    #[test]
    fn test_font_mapping_helvetica_variants() {
        assert_eq!(map_to_standard_font("Helvetica"), "Helvetica");
        assert_eq!(map_to_standard_font("Helvetica-Bold"), "Helvetica-Bold");
        assert_eq!(
            map_to_standard_font("Helvetica-Oblique"),
            "Helvetica-Oblique"
        );
        assert_eq!(map_to_standard_font("Arial"), "Helvetica");
        assert_eq!(map_to_standard_font("ArialMT"), "Helvetica");
        assert_eq!(map_to_standard_font("BCDEEE+ArialMT"), "Helvetica");
        assert_eq!(map_to_standard_font("Arial-BoldMT"), "Helvetica-Bold");
    }

    #[test]
    fn test_font_mapping_courier_variants() {
        assert_eq!(map_to_standard_font("Courier"), "Courier");
        assert_eq!(map_to_standard_font("Courier-Bold"), "Courier-Bold");
        assert_eq!(map_to_standard_font("CourierNew"), "Courier");
        assert_eq!(map_to_standard_font("Consolas"), "Courier");
        assert_eq!(map_to_standard_font("Monaco"), "Courier");
    }

    #[test]
    fn test_font_mapping_case_insensitive() {
        assert_eq!(map_to_standard_font("SERIF"), "Times-Roman");
        assert_eq!(map_to_standard_font("Sans-Serif"), "Helvetica");
        assert_eq!(map_to_standard_font("TIMES-ROMAN"), "Times-Roman");
        assert_eq!(map_to_standard_font("arial"), "Helvetica");
    }

    #[test]
    fn test_font_mapping_unknown_defaults_to_helvetica() {
        assert_eq!(map_to_standard_font("g_d0_f1"), "Helvetica"); // PDF.js internal name
        assert_eq!(map_to_standard_font("UnknownFont"), "Helvetica");
        assert_eq!(map_to_standard_font(""), "Helvetica");
    }

    #[test]
    fn test_text_style_pdf_font_name() {
        // No font specified -> Helvetica
        let style_default = TextStyle::default();
        assert_eq!(style_default.pdf_font_name(), "Helvetica");

        // Serif font (regular)
        let style_serif = TextStyle {
            font_size: 12.0,
            color: "#000000".to_string(),
            font_name: Some("serif".to_string()),
            is_italic: false,
            is_bold: false,
        };
        assert_eq!(style_serif.pdf_font_name(), "Times-Roman");

        // Sans-serif font (regular)
        let style_sans = TextStyle {
            font_size: 14.0,
            color: "#333333".to_string(),
            font_name: Some("sans-serif".to_string()),
            is_italic: false,
            is_bold: false,
        };
        assert_eq!(style_sans.pdf_font_name(), "Helvetica");

        // Monospace font (regular)
        let style_mono = TextStyle {
            font_size: 10.0,
            color: "#000000".to_string(),
            font_name: Some("monospace".to_string()),
            is_italic: false,
            is_bold: false,
        };
        assert_eq!(style_mono.pdf_font_name(), "Courier");
    }

    #[test]
    fn test_text_style_italic_bold() {
        // Serif + italic -> Times-Italic
        let style_italic = TextStyle {
            font_size: 12.0,
            color: "#000000".to_string(),
            font_name: Some("serif".to_string()),
            is_italic: true,
            is_bold: false,
        };
        assert_eq!(style_italic.pdf_font_name(), "Times-Italic");

        // Serif + bold -> Times-Bold
        let style_bold = TextStyle {
            font_size: 12.0,
            color: "#000000".to_string(),
            font_name: Some("serif".to_string()),
            is_italic: false,
            is_bold: true,
        };
        assert_eq!(style_bold.pdf_font_name(), "Times-Bold");

        // Serif + bold + italic -> Times-BoldItalic
        let style_bold_italic = TextStyle {
            font_size: 12.0,
            color: "#000000".to_string(),
            font_name: Some("serif".to_string()),
            is_italic: true,
            is_bold: true,
        };
        assert_eq!(style_bold_italic.pdf_font_name(), "Times-BoldItalic");

        // Sans-serif + italic -> Helvetica-Oblique
        let style_sans_italic = TextStyle {
            font_size: 12.0,
            color: "#000000".to_string(),
            font_name: Some("sans-serif".to_string()),
            is_italic: true,
            is_bold: false,
        };
        assert_eq!(style_sans_italic.pdf_font_name(), "Helvetica-Oblique");
    }

    #[test]
    fn test_replace_text_operation_with_font() {
        let mut log = OperationLog::new();
        let id = log.add(EditOperation::ReplaceText {
            id: 0,
            page: 1,
            original_rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 150.0,
                height: 14.0,
            },
            replacement_rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 150.0,
                height: 14.0,
            },
            original_text: "Miami, FL 33101".to_string(),
            new_text: "Orlando, FL 32801".to_string(),
            style: TextStyle {
                font_size: 11.0,
                color: "#000000".to_string(),
                font_name: Some("serif".to_string()),
                is_italic: false,
                is_bold: false,
            },
        });

        assert!(id == 0);
        assert!(!log.is_empty());

        // Verify the operation was stored correctly
        if let EditOperation::ReplaceText { style, .. } = &log.operations()[0] {
            assert_eq!(style.font_size, 11.0);
            assert_eq!(style.font_name, Some("serif".to_string()));
            assert_eq!(style.pdf_font_name(), "Times-Roman");
        } else {
            panic!("Expected ReplaceText operation");
        }
    }
}
