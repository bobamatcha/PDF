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

/// A segment of styled text for mixed-style text blocks.
/// Each segment has its own text content and style flags (bold/italic).
/// Font size and color are inherited from the parent TextStyle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyledTextSegment {
    /// The text content for this segment
    pub text: String,
    /// Whether this segment is bold
    #[serde(default)]
    pub is_bold: bool,
    /// Whether this segment is italic
    #[serde(default)]
    pub is_italic: bool,
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
    /// Add text with mixed styling (bold/italic segments within one text block).
    /// Each segment can have its own bold/italic flags while sharing font_size and color.
    AddStyledText {
        id: OpId,
        page: u32,
        rect: PdfRect,
        /// Individual styled segments that make up the text
        segments: Vec<StyledTextSegment>,
        /// Base style (font_size, color, font_name) shared by all segments
        style: TextStyle,
    },
    AddHighlight {
        id: OpId,
        page: u32,
        rect: PdfRect,
        color: String,
        opacity: f64,
    },
    /// Add an underline annotation (thin line below text)
    /// Distinct from highlight - draws a line, not a filled rectangle
    AddUnderline {
        id: OpId,
        page: u32,
        rect: PdfRect,
        color: String,
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
    /// Add a colored rectangle to cover/redact content (white for whiteout, black for blackout/redaction)
    AddWhiteRect {
        id: OpId,
        page: u32,
        rect: PdfRect,
        /// RGB color as hex string (e.g., "#FFFFFF" for white, "#000000" for black)
        /// Defaults to white if not specified
        #[serde(default = "default_whiteout_color")]
        color: String,
    },
    /// Add an image (PNG/JPEG) to the PDF - used for signatures
    AddImage {
        id: OpId,
        page: u32,
        rect: PdfRect,
        /// Base64-encoded image data (PNG or JPEG)
        image_data: String,
        /// Image format: "png" or "jpeg"
        format: String,
    },
}

fn default_whiteout_color() -> String {
    "#FFFFFF".to_string()
}

/// The kind of user action that groups one or more operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    /// Adding a text box
    AddTextBox,
    /// Adding a whiteout (white rectangle, optionally with text)
    AddWhiteout,
    /// Adding a checkbox
    AddCheckbox,
    /// Adding a highlight
    AddHighlight,
    /// Adding an underline
    AddUnderline,
    /// Replacing existing PDF text
    ReplaceText,
    /// Adding an image (signature)
    AddImage,
    /// Moving an element
    Move,
    /// Resizing an element
    Resize,
    /// Deleting an element
    Delete,
}

/// An action groups one or more operations that should be undone/redone together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub kind: ActionKind,
    /// Operations that were added by this action
    pub added_ops: Vec<EditOperation>,
    /// Operations that were removed by this action (for Delete actions)
    pub removed_ops: Vec<EditOperation>,
}

impl Action {
    fn new(kind: ActionKind) -> Self {
        Self {
            kind,
            added_ops: Vec::new(),
            removed_ops: Vec::new(),
        }
    }
}

impl EditOperation {
    pub fn id(&self) -> OpId {
        match self {
            EditOperation::AddText { id, .. } => *id,
            EditOperation::AddStyledText { id, .. } => *id,
            EditOperation::AddHighlight { id, .. } => *id,
            EditOperation::AddUnderline { id, .. } => *id,
            EditOperation::AddCheckbox { id, .. } => *id,
            EditOperation::ReplaceText { id, .. } => *id,
            EditOperation::AddWhiteRect { id, .. } => *id,
            EditOperation::AddImage { id, .. } => *id,
        }
    }

    pub fn page(&self) -> u32 {
        match self {
            EditOperation::AddText { page, .. } => *page,
            EditOperation::AddStyledText { page, .. } => *page,
            EditOperation::AddHighlight { page, .. } => *page,
            EditOperation::AddUnderline { page, .. } => *page,
            EditOperation::AddCheckbox { page, .. } => *page,
            EditOperation::ReplaceText { page, .. } => *page,
            EditOperation::AddWhiteRect { page, .. } => *page,
            EditOperation::AddImage { page, .. } => *page,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationLog {
    next_id: OpId,
    operations: Vec<EditOperation>,
    /// Stack of committed actions (for undo)
    #[serde(default)]
    undo_stack: Vec<Action>,
    /// Stack of undone actions (for redo)
    #[serde(default)]
    redo_stack: Vec<Action>,
    /// Action currently being built (between begin_action and commit_action)
    #[serde(skip)]
    pending_action: Option<Action>,
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
            EditOperation::AddStyledText { id: op_id, .. } => *op_id = id,
            EditOperation::AddHighlight { id: op_id, .. } => *op_id = id,
            EditOperation::AddUnderline { id: op_id, .. } => *op_id = id,
            EditOperation::AddCheckbox { id: op_id, .. } => *op_id = id,
            EditOperation::ReplaceText { id: op_id, .. } => *op_id = id,
            EditOperation::AddWhiteRect { id: op_id, .. } => *op_id = id,
            EditOperation::AddImage { id: op_id, .. } => *op_id = id,
        }

        // Track in pending action if one is being built
        if let Some(ref mut action) = self.pending_action {
            action.added_ops.push(op.clone());
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

    /// Get an operation by its ID
    pub fn get_operation(&self, id: OpId) -> Option<&EditOperation> {
        self.operations.iter().find(|op| op.id() == id)
    }

    /// Get a mutable reference to an operation by its ID
    fn get_operation_mut(&mut self, id: OpId) -> Option<&mut EditOperation> {
        self.operations.iter_mut().find(|op| op.id() == id)
    }

    /// Update the checked state of a checkbox operation
    /// Returns false if the operation is not found or is not a checkbox
    pub fn set_checkbox(&mut self, id: OpId, checked: bool) -> bool {
        if let Some(EditOperation::AddCheckbox {
            checked: ref mut c, ..
        }) = self.get_operation_mut(id)
        {
            *c = checked;
            return true;
        }
        false
    }

    /// Update the rect of an operation
    /// Works for AddText, AddHighlight, AddUnderline, AddCheckbox, AddWhiteRect
    /// Returns false if the operation is not found
    pub fn update_rect(&mut self, id: OpId, new_rect: PdfRect) -> bool {
        if let Some(op) = self.get_operation_mut(id) {
            match op {
                EditOperation::AddText { ref mut rect, .. } => {
                    *rect = new_rect;
                    true
                }
                EditOperation::AddStyledText { ref mut rect, .. } => {
                    *rect = new_rect;
                    true
                }
                EditOperation::AddHighlight { ref mut rect, .. } => {
                    *rect = new_rect;
                    true
                }
                EditOperation::AddUnderline { ref mut rect, .. } => {
                    *rect = new_rect;
                    true
                }
                EditOperation::AddCheckbox { ref mut rect, .. } => {
                    *rect = new_rect;
                    true
                }
                EditOperation::AddWhiteRect { ref mut rect, .. } => {
                    *rect = new_rect;
                    true
                }
                EditOperation::AddImage { ref mut rect, .. } => {
                    *rect = new_rect;
                    true
                }
                EditOperation::ReplaceText {
                    ref mut replacement_rect,
                    ..
                } => {
                    *replacement_rect = new_rect;
                    true
                }
            }
        } else {
            false
        }
    }

    /// Update the text and optionally the style of a text operation
    /// Returns false if the operation is not found or is not a text operation
    pub fn update_text(&mut self, id: OpId, new_text: &str, new_style: Option<&TextStyle>) -> bool {
        if let Some(op) = self.get_operation_mut(id) {
            match op {
                EditOperation::AddText {
                    ref mut text,
                    ref mut style,
                    ..
                } => {
                    *text = new_text.to_string();
                    if let Some(s) = new_style {
                        *style = s.clone();
                    }
                    true
                }
                EditOperation::ReplaceText {
                    new_text: ref mut existing_text,
                    ref mut style,
                    ..
                } => {
                    *existing_text = new_text.to_string();
                    if let Some(s) = new_style {
                        *style = s.clone();
                    }
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    // ============ Action-Based Undo/Redo (Phase 4) ============

    /// Start building a new action. Operations added after this will be grouped.
    pub fn begin_action(&mut self, kind: ActionKind) {
        // If there's already a pending action, commit it first
        if self.pending_action.is_some() {
            self.commit_action();
        }
        self.pending_action = Some(Action::new(kind));
        // Clear redo stack when starting a new action
        self.redo_stack.clear();
    }

    /// Commit the pending action to the undo stack.
    /// Returns true if an action was committed.
    pub fn commit_action(&mut self) -> bool {
        if let Some(action) = self.pending_action.take() {
            // Only commit if the action has operations
            if !action.added_ops.is_empty() || !action.removed_ops.is_empty() {
                self.undo_stack.push(action);
                return true;
            }
        }
        false
    }

    /// Abort the pending action and remove any operations that were added.
    pub fn abort_action(&mut self) {
        if let Some(action) = self.pending_action.take() {
            // Remove all operations that were added as part of this action
            for op in &action.added_ops {
                self.remove(op.id());
            }
            // Restore any operations that were removed as part of this action
            for op in action.removed_ops {
                self.operations.push(op);
            }
        }
    }

    /// Record an operation that was removed (for Delete actions).
    /// This allows undo to restore the deleted operation.
    pub fn record_removed_op(&mut self, op: EditOperation) {
        if let Some(ref mut action) = self.pending_action {
            action.removed_ops.push(op);
        }
    }

    /// Undo the last action.
    /// Returns the OpIds that were removed (for DOM cleanup by TS).
    pub fn undo(&mut self) -> Option<Vec<OpId>> {
        let action = self.undo_stack.pop()?;

        // Collect the op_ids that will be removed
        let op_ids: Vec<OpId> = action.added_ops.iter().map(|op| op.id()).collect();

        // Remove added operations
        for op in &action.added_ops {
            self.remove(op.id());
        }

        // Restore removed operations
        for op in &action.removed_ops {
            self.operations.push(op.clone());
        }

        // Push to redo stack
        self.redo_stack.push(action);

        Some(op_ids)
    }

    /// Redo the last undone action.
    /// Returns the OpIds that were restored (for DOM recreation by TS).
    pub fn redo(&mut self) -> Option<Vec<OpId>> {
        let action = self.redo_stack.pop()?;

        // Remove the operations that were restored by undo (the removed_ops)
        for op in &action.removed_ops {
            self.remove(op.id());
        }

        // Restore the added operations
        for op in &action.added_ops {
            self.operations.push(op.clone());
        }

        // Collect the op_ids that were restored
        let op_ids: Vec<OpId> = action.added_ops.iter().map(|op| op.id()).collect();

        // Push back to undo stack
        self.undo_stack.push(action);

        Some(op_ids)
    }

    /// Check if there are actions that can be undone.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if there are actions that can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get an operation by ID for redo purposes (returns a clone)
    pub fn get_operation_for_redo(&self, id: OpId) -> Option<EditOperation> {
        // First check in current operations
        if let Some(op) = self.get_operation(id) {
            return Some(op.clone());
        }
        // Then check in redo stack
        for action in &self.redo_stack {
            for op in &action.added_ops {
                if op.id() == id {
                    return Some(op.clone());
                }
            }
        }
        None
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

    // ============ Update Method Tests (Phase 1) ============

    #[test]
    fn test_get_operation_returns_operation_by_id() {
        let mut log = OperationLog::new();
        let id = log.add(EditOperation::AddCheckbox {
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

        let op = log.get_operation(id);
        assert!(op.is_some());
        assert_eq!(op.unwrap().id(), id);
    }

    #[test]
    fn test_get_operation_returns_none_for_invalid_id() {
        let log = OperationLog::new();
        assert!(log.get_operation(999).is_none());
    }

    #[test]
    fn test_set_checkbox_updates_checked_state() {
        let mut log = OperationLog::new();
        let id = log.add(EditOperation::AddCheckbox {
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

        // Initially unchecked
        if let Some(EditOperation::AddCheckbox { checked, .. }) = log.get_operation(id) {
            assert!(!checked);
        }

        // Update to checked
        assert!(log.set_checkbox(id, true));

        // Verify it's now checked
        if let Some(EditOperation::AddCheckbox { checked, .. }) = log.get_operation(id) {
            assert!(*checked);
        } else {
            panic!("Expected AddCheckbox operation");
        }
    }

    #[test]
    fn test_set_checkbox_returns_false_for_non_checkbox() {
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
            text: "Hello".to_string(),
            style: TextStyle::default(),
        });

        // Should return false for non-checkbox operations
        assert!(!log.set_checkbox(id, true));
    }

    #[test]
    fn test_update_rect_changes_position_and_size() {
        let mut log = OperationLog::new();
        let id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 100.0,
                width: 200.0,
                height: 30.0,
            },
            text: "Test".to_string(),
            style: TextStyle::default(),
        });

        let new_rect = PdfRect {
            x: 150.0,
            y: 200.0,
            width: 250.0,
            height: 40.0,
        };

        assert!(log.update_rect(id, new_rect.clone()));

        // Verify rect was updated
        if let Some(EditOperation::AddText { rect, text, .. }) = log.get_operation(id) {
            assert_eq!(rect.x, 150.0);
            assert_eq!(rect.y, 200.0);
            assert_eq!(rect.width, 250.0);
            assert_eq!(rect.height, 40.0);
            // Text should be preserved
            assert_eq!(text, "Test");
        } else {
            panic!("Expected AddText operation");
        }
    }

    #[test]
    fn test_update_rect_works_for_whiteout() {
        let mut log = OperationLog::new();
        let id = log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            color: "#FFFFFF".to_string(),
        });

        let new_rect = PdfRect {
            x: 10.0,
            y: 20.0,
            width: 150.0,
            height: 75.0,
        };

        assert!(log.update_rect(id, new_rect));

        if let Some(EditOperation::AddWhiteRect { rect, .. }) = log.get_operation(id) {
            assert_eq!(rect.x, 10.0);
            assert_eq!(rect.y, 20.0);
            assert_eq!(rect.width, 150.0);
            assert_eq!(rect.height, 75.0);
        } else {
            panic!("Expected AddWhiteRect operation");
        }
    }

    #[test]
    fn test_update_text_changes_text_content() {
        let mut log = OperationLog::new();
        let id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 100.0,
                width: 200.0,
                height: 30.0,
            },
            text: "Original".to_string(),
            style: TextStyle::default(),
        });

        assert!(log.update_text(id, "Updated", None));

        if let Some(EditOperation::AddText { text, rect, .. }) = log.get_operation(id) {
            assert_eq!(text, "Updated");
            // Rect should be preserved
            assert_eq!(rect.x, 100.0);
        } else {
            panic!("Expected AddText operation");
        }
    }

    #[test]
    fn test_update_text_with_new_style() {
        let mut log = OperationLog::new();
        let id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 100.0,
                width: 200.0,
                height: 30.0,
            },
            text: "Original".to_string(),
            style: TextStyle::default(),
        });

        let new_style = TextStyle {
            font_size: 18.0,
            color: "#FF0000".to_string(),
            font_name: Some("serif".to_string()),
            is_bold: true,
            is_italic: false,
        };

        assert!(log.update_text(id, "Bold Red", Some(&new_style)));

        if let Some(EditOperation::AddText { text, style, .. }) = log.get_operation(id) {
            assert_eq!(text, "Bold Red");
            assert_eq!(style.font_size, 18.0);
            assert_eq!(style.color, "#FF0000");
            assert!(style.is_bold);
        } else {
            panic!("Expected AddText operation");
        }
    }

    #[test]
    fn test_update_text_returns_false_for_non_text() {
        let mut log = OperationLog::new();
        let id = log.add(EditOperation::AddCheckbox {
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

        // Should return false for non-text operations
        assert!(!log.update_text(id, "Test", None));
    }

    // ============ Action-Based Undo/Redo Tests (Phase 4) ============

    #[test]
    fn test_initial_state_cannot_undo_or_redo() {
        let log = OperationLog::new();
        assert!(!log.can_undo());
        assert!(!log.can_redo());
    }

    #[test]
    fn test_begin_and_commit_action_enables_undo() {
        let mut log = OperationLog::new();

        log.begin_action(ActionKind::AddTextBox);
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 100.0,
                width: 200.0,
                height: 30.0,
            },
            text: "Test".to_string(),
            style: TextStyle::default(),
        });
        log.commit_action();

        assert!(log.can_undo());
        assert!(!log.can_redo());
    }

    #[test]
    fn test_undo_returns_op_ids_for_dom_removal() {
        let mut log = OperationLog::new();

        log.begin_action(ActionKind::AddTextBox);
        let id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 100.0,
                width: 200.0,
                height: 30.0,
            },
            text: "Test".to_string(),
            style: TextStyle::default(),
        });
        log.commit_action();

        let undone = log.undo();
        assert!(undone.is_some());
        let op_ids = undone.unwrap();
        assert_eq!(op_ids.len(), 1);
        assert_eq!(op_ids[0], id);

        // Operation should be removed from the log
        assert!(log.get_operation(id).is_none());
        assert!(!log.can_undo());
        assert!(log.can_redo());
    }

    #[test]
    fn test_redo_returns_op_ids_for_dom_recreation() {
        let mut log = OperationLog::new();

        log.begin_action(ActionKind::AddTextBox);
        let id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 100.0,
                width: 200.0,
                height: 30.0,
            },
            text: "Redo test".to_string(),
            style: TextStyle::default(),
        });
        log.commit_action();

        log.undo();
        assert!(!log.can_undo());
        assert!(log.can_redo());

        let redone = log.redo();
        assert!(redone.is_some());
        let op_ids = redone.unwrap();
        assert_eq!(op_ids.len(), 1);
        // ID should be the same as original
        assert_eq!(op_ids[0], id);

        // Operation should be back in the log
        assert!(log.get_operation(id).is_some());
        assert!(log.can_undo());
        assert!(!log.can_redo());
    }

    #[test]
    fn test_action_with_multiple_operations() {
        let mut log = OperationLog::new();

        // A whiteout with text creates 2 operations in one action
        log.begin_action(ActionKind::AddWhiteout);
        let white_id = log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 50.0,
                y: 50.0,
                width: 100.0,
                height: 30.0,
            },
            color: "#FFFFFF".to_string(),
        });
        let text_id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 50.0,
                y: 50.0,
                width: 100.0,
                height: 30.0,
            },
            text: "Over whiteout".to_string(),
            style: TextStyle::default(),
        });
        log.commit_action();

        assert_eq!(log.operations().len(), 2);

        // Undo should remove BOTH operations
        let undone = log.undo();
        assert!(undone.is_some());
        let op_ids = undone.unwrap();
        assert_eq!(op_ids.len(), 2);
        assert!(op_ids.contains(&white_id));
        assert!(op_ids.contains(&text_id));

        // Both should be gone
        assert!(log.operations().is_empty());
    }

    #[test]
    fn test_new_action_clears_redo_stack() {
        let mut log = OperationLog::new();

        log.begin_action(ActionKind::AddTextBox);
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            text: "First".to_string(),
            style: TextStyle::default(),
        });
        log.commit_action();

        log.undo();
        assert!(log.can_redo());

        // Adding a new action should clear redo stack
        log.begin_action(ActionKind::AddTextBox);
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 50.0,
                y: 50.0,
                width: 100.0,
                height: 20.0,
            },
            text: "Second".to_string(),
            style: TextStyle::default(),
        });
        log.commit_action();

        assert!(!log.can_redo());
    }

    #[test]
    fn test_undo_multiple_actions() {
        let mut log = OperationLog::new();

        // Action 1
        log.begin_action(ActionKind::AddTextBox);
        let id1 = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            text: "First".to_string(),
            style: TextStyle::default(),
        });
        log.commit_action();

        // Action 2
        log.begin_action(ActionKind::AddTextBox);
        let id2 = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 50.0,
                y: 50.0,
                width: 100.0,
                height: 20.0,
            },
            text: "Second".to_string(),
            style: TextStyle::default(),
        });
        log.commit_action();

        assert_eq!(log.operations().len(), 2);

        // Undo second action
        let undone1 = log.undo().unwrap();
        assert_eq!(undone1, vec![id2]);
        assert_eq!(log.operations().len(), 1);

        // Undo first action
        let undone2 = log.undo().unwrap();
        assert_eq!(undone2, vec![id1]);
        assert!(log.operations().is_empty());
    }

    #[test]
    fn test_undo_without_commit_does_nothing() {
        let mut log = OperationLog::new();

        log.begin_action(ActionKind::AddTextBox);
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            text: "Uncommitted".to_string(),
            style: TextStyle::default(),
        });
        // NOT committing

        assert!(!log.can_undo()); // Can't undo uncommitted action
    }

    #[test]
    fn test_abort_action_removes_uncommitted_ops() {
        let mut log = OperationLog::new();

        log.begin_action(ActionKind::AddTextBox);
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            text: "Will be aborted".to_string(),
            style: TextStyle::default(),
        });

        log.abort_action();

        // Operation should be removed
        assert!(log.operations().is_empty());
        assert!(!log.can_undo());
    }

    #[test]
    fn test_delete_action_type() {
        let mut log = OperationLog::new();

        // First add something
        log.begin_action(ActionKind::AddTextBox);
        let id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            text: "To delete".to_string(),
            style: TextStyle::default(),
        });
        log.commit_action();

        // Now delete it via action
        log.begin_action(ActionKind::Delete);
        let removed_op = log.get_operation(id).cloned();
        log.remove(id);
        log.record_removed_op(removed_op.unwrap());
        log.commit_action();

        // Undo should restore the deleted operation
        log.undo();
        assert!(log.get_operation(id).is_some());
    }

    // ============ Underline Operation Tests (Bug 5 fix) ============

    #[test]
    fn test_add_underline_operation_exists() {
        // Bug 5: Underline should be a separate operation from highlight
        let mut log = OperationLog::new();
        let id = log.add(EditOperation::AddUnderline {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 2.0, // Underline is thin
            },
            color: "#000000".to_string(),
        });

        assert!(id == 0);
        assert!(!log.is_empty());

        // Verify it's stored as AddUnderline, not AddHighlight
        if let EditOperation::AddUnderline { color, rect, .. } = &log.operations()[0] {
            assert_eq!(color, "#000000");
            assert_eq!(rect.height, 2.0); // Underlines should be thin
        } else {
            panic!("Expected AddUnderline operation, got something else");
        }
    }

    #[test]
    fn test_underline_operation_id_getter() {
        let op = EditOperation::AddUnderline {
            id: 42,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 2.0,
            },
            color: "#000000".to_string(),
        };
        assert_eq!(op.id(), 42);
    }

    #[test]
    fn test_underline_operation_page_getter() {
        let op = EditOperation::AddUnderline {
            id: 0,
            page: 5,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 2.0,
            },
            color: "#FF0000".to_string(),
        };
        assert_eq!(op.page(), 5);
    }

    #[test]
    fn test_underline_json_roundtrip() {
        let mut log = OperationLog::new();
        log.add(EditOperation::AddUnderline {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 50.0,
                y: 100.0,
                width: 150.0,
                height: 2.0,
            },
            color: "#0000FF".to_string(),
        });

        let json = log.to_json().unwrap();
        let restored = OperationLog::from_json(&json).unwrap();

        assert_eq!(log.operations().len(), restored.operations().len());

        // Verify it deserializes as AddUnderline, not AddHighlight
        if let EditOperation::AddUnderline { color, .. } = &restored.operations()[0] {
            assert_eq!(color, "#0000FF");
        } else {
            panic!("Expected AddUnderline after JSON roundtrip");
        }
    }

    #[test]
    fn test_action_kind_add_underline_exists() {
        // ActionKind should have AddUnderline variant for undo/redo
        let mut log = OperationLog::new();
        log.begin_action(ActionKind::AddUnderline);
        log.add(EditOperation::AddUnderline {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 2.0,
            },
            color: "#000000".to_string(),
        });
        log.commit_action();

        assert!(log.can_undo());
    }
}
