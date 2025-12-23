# PDFJoin Edit Tab Implementation Plan

## Executive Summary

Add an "Edit PDF" tab to pdfjoin-web that allows users to add annotations, text, and simple edits to **unsigned PDFs only**. Signed PDFs are rejected with a clear, friendly error message.

### Architecture Follows Your Recommendations
1. **PDF.js for rendering** (copy from agentpdf-web)
2. **Operation log as structured data** (new Rust struct)
3. **Generate new PDF on export** (extend existing lopdf/streaming infrastructure)
4. **No incremental updates** (v1 scope - unsigned docs only)

---

## Current State Analysis

### What pdfjoin-web Has
- **Custom streaming parser** (`streaming.rs`) - byte-level PDF manipulation (fast)
- **lopdf fallback** (`merge.rs`, `split.rs`) - full parsing (more compatible)
- **Session architecture** (`session.rs`) - stateful WASM API
- **Clean tab UI** - easy to extend with third tab
- **No PDF.js** - renders nothing, just manipulates bytes

### What We Can Copy from agentpdf-web
- `www/js/pdf-bridge.js` - PDF.js wrapper
- `wasm/src/overlay.rs` - coordinate transforms, overlay management
- `wasm/src/coords.rs` - PDF↔DOM coordinate math

### What We Need to Build
1. Signature detection (reject signed PDFs)
2. Operation log data structure
3. Edit UI (toolbar, overlays)
4. PDF writer that applies operations

---

## PDF.js Best Practices: Lazy Loading

**IMPORTANT**: PDF.js should NOT be loaded in the main bundle. It should be:

1. **Lazy loaded** - Only load when user navigates to a route that needs PDF viewing
2. **Worker as separate asset** - The worker (`pdf.worker.min.js`) runs parsing/rendering in a separate thread
3. **Static assets** - Use prebuilt `pdf.min.js` and `pdf.worker.min.js` as static assets

### Why This Matters
- PDF.js is ~500KB+ minified
- Loading it upfront bloats initial page load
- Worker enables non-blocking PDF rendering
- Users who only split/merge never need PDF.js at all

### Implementation Pattern

```javascript
// www/js/pdf-loader.js - Lazy loader for PDF.js
let pdfJsLoaded = false;
let pdfJsLoadPromise = null;

export async function ensurePdfJsLoaded() {
    if (pdfJsLoaded) return;
    if (pdfJsLoadPromise) return pdfJsLoadPromise;

    pdfJsLoadPromise = new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = '/js/vendor/pdf.min.js';  // Static asset
        script.onload = () => {
            // Configure worker path
            pdfjsLib.GlobalWorkerOptions.workerSrc = '/js/vendor/pdf.worker.min.js';
            pdfJsLoaded = true;
            resolve();
        };
        script.onerror = reject;
        document.head.appendChild(script);
    });

    return pdfJsLoadPromise;
}
```

```javascript
// In edit tab handler - only load when needed
async function handleEditFile(file) {
    // Lazy load PDF.js only when user actually needs it
    await ensurePdfJsLoaded();

    // Now PDF.js is available
    const doc = await pdfjsLib.getDocument(bytes).promise;
    // ...
}
```

### Static Asset Setup (Trunk)

```html
<!-- In index.html, copy vendor files -->
<link data-trunk rel="copy-dir" href="js/vendor" />
```

Download and place in `www/js/vendor/`:
- `pdf.min.js` (v3.11.174)
- `pdf.worker.min.js` (v3.11.174)

### Benefits
- Main bundle stays small (~50KB)
- PDF.js only loads when user clicks "Edit PDF" tab
- Worker runs in separate thread (non-blocking UI)
- Split/Merge users never download PDF.js at all

---

## Phase 1: Signature Detection (Reject Signed PDFs)

### Why First?
- Gate the entire feature
- Clear error message before user wastes time
- Simple to implement with lopdf

### Implementation

Add to `crates/pdfjoin-core/src/lib.rs`:

```rust
/// Check if a PDF contains digital signatures
/// Returns true if /Type /Sig or /SigFlags exists
pub fn has_signatures(bytes: &[u8]) -> Result<bool, PdfJoinError> {
    let doc = lopdf::Document::load_mem(bytes)
        .map_err(|e| PdfJoinError::ParseError(e.to_string()))?;

    // Check AcroForm for SigFlags
    if let Ok(catalog) = doc.catalog() {
        if let Ok(acroform) = catalog.get(b"AcroForm") {
            if let Ok(acroform_dict) = doc.get_dictionary(acroform) {
                if acroform_dict.has(b"SigFlags") {
                    return Ok(true);
                }
            }
        }
    }

    // Check for /Type /Sig objects
    for (_, obj) in doc.objects.iter() {
        if let Ok(dict) = obj.as_dict() {
            if let Ok(type_name) = dict.get(b"Type") {
                if type_name.as_name().map(|n| n == b"Sig").unwrap_or(false) {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}
```

### User-Facing Error Message (Senior-Friendly)

```
┌──────────────────────────────────────────────────────────────┐
│  ⚠️  This PDF Has a Digital Signature                        │
│                                                              │
│  Editing this document would break its signature and         │
│  make it invalid.                                            │
│                                                              │
│  What you can do:                                            │
│  • Use Split or Merge (signatures will be removed)           │
│  • Contact the sender for an unsigned copy                   │
│  • [Coming soon: Edit signed docs at getsignatures.org]      │
│                                                              │
│  [← Go Back]                        [Use Split/Merge Anyway] │
└──────────────────────────────────────────────────────────────┘
```

**Design notes for seniors:**
- Large, high-contrast text
- Simple language (no jargon)
- Clear action buttons
- No tiny dismiss X

---

## Phase 2: PDF.js Rendering Integration

### Files to Add/Copy

```
apps/pdfjoin-web/
├── www/
│   └── js/
│       └── pdf-bridge.js    # Copy from agentpdf-web, simplify
└── wasm/
    └── src/
        ├── overlay.rs       # Copy from agentpdf-web
        └── coords.rs        # Copy from agentpdf-web
```

### Add PDF.js CDN to index.html

```html
<!-- Add before </head> -->
<script src="https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.min.js"></script>
<script>
    pdfjsLib.GlobalWorkerOptions.workerSrc =
        'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';
</script>
```

### Simplified pdf-bridge.js for Edit Tab

```javascript
window.EditPdfBridge = {
    currentDoc: null,
    pageCanvases: new Map(),

    async loadDocument(data) {
        const typedArray = new Uint8Array(data);
        this.currentDoc = await pdfjsLib.getDocument(typedArray).promise;
        return this.currentDoc.numPages;
    },

    async renderPage(pageNum, canvas, scale = 1.5) {
        const page = await this.currentDoc.getPage(pageNum);
        const viewport = page.getViewport({ scale });

        canvas.width = viewport.width;
        canvas.height = viewport.height;

        await page.render({
            canvasContext: canvas.getContext('2d'),
            viewport
        }).promise;

        this.pageCanvases.set(pageNum, { canvas, viewport, page });

        return {
            width: viewport.width,
            height: viewport.height,
            pdfWidth: page.view[2],   // Original PDF width in points
            pdfHeight: page.view[3]   // Original PDF height in points
        };
    },

    getPageInfo(pageNum) {
        return this.pageCanvases.get(pageNum);
    },

    cleanup() {
        if (this.currentDoc) {
            this.currentDoc.destroy();
            this.currentDoc = null;
        }
        this.pageCanvases.clear();
    }
};
```

---

## Phase 3: Operation Log Data Structure

### Core Types (Rust)

Add `crates/pdfjoin-core/src/operations.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Unique ID for tracking operations
pub type OpId = u64;

/// A rectangle in PDF coordinates (origin bottom-left)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Text styling options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_size: f64,
    pub color: String,      // "#000000" format
    pub bold: bool,
    pub italic: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            color: "#000000".to_string(),
            bold: false,
            italic: false,
        }
    }
}

/// A single edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EditOperation {
    /// Add text at a position
    AddText {
        id: OpId,
        page: u32,
        rect: PdfRect,
        text: String,
        style: TextStyle,
    },

    /// Add a highlight rectangle
    AddHighlight {
        id: OpId,
        page: u32,
        rect: PdfRect,
        color: String,  // "#FFFF00" for yellow
        opacity: f64,   // 0.0 - 1.0
    },

    /// Add a freehand drawing/signature
    AddDrawing {
        id: OpId,
        page: u32,
        paths: Vec<Vec<[f64; 2]>>,  // List of paths, each path is points
        color: String,
        stroke_width: f64,
    },

    /// Add an image (stamp, signature image, etc.)
    AddImage {
        id: OpId,
        page: u32,
        rect: PdfRect,
        image_data: Vec<u8>,  // PNG bytes
    },

    /// Add a checkbox (visual only)
    AddCheckbox {
        id: OpId,
        page: u32,
        rect: PdfRect,
        checked: bool,
    },

    /// Delete a page
    DeletePage {
        id: OpId,
        page: u32,
    },

    /// Rotate a page
    RotatePage {
        id: OpId,
        page: u32,
        degrees: i32,  // 90, 180, 270
    },
}

impl EditOperation {
    pub fn id(&self) -> OpId {
        match self {
            Self::AddText { id, .. } => *id,
            Self::AddHighlight { id, .. } => *id,
            Self::AddDrawing { id, .. } => *id,
            Self::AddImage { id, .. } => *id,
            Self::AddCheckbox { id, .. } => *id,
            Self::DeletePage { id, .. } => *id,
            Self::RotatePage { id, .. } => *id,
        }
    }

    pub fn page(&self) -> u32 {
        match self {
            Self::AddText { page, .. } => *page,
            Self::AddHighlight { page, .. } => *page,
            Self::AddDrawing { page, .. } => *page,
            Self::AddImage { page, .. } => *page,
            Self::AddCheckbox { page, .. } => *page,
            Self::DeletePage { page, .. } => *page,
            Self::RotatePage { page, .. } => *page,
        }
    }
}

/// The complete operation log for a document
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationLog {
    next_id: OpId,
    operations: Vec<EditOperation>,
}

impl OperationLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an operation and return its ID
    pub fn add(&mut self, mut op: EditOperation) -> OpId {
        let id = self.next_id;
        self.next_id += 1;

        // Set the ID on the operation
        match &mut op {
            EditOperation::AddText { id: ref mut op_id, .. } => *op_id = id,
            EditOperation::AddHighlight { id: ref mut op_id, .. } => *op_id = id,
            EditOperation::AddDrawing { id: ref mut op_id, .. } => *op_id = id,
            EditOperation::AddImage { id: ref mut op_id, .. } => *op_id = id,
            EditOperation::AddCheckbox { id: ref mut op_id, .. } => *op_id = id,
            EditOperation::DeletePage { id: ref mut op_id, .. } => *op_id = id,
            EditOperation::RotatePage { id: ref mut op_id, .. } => *op_id = id,
        }

        self.operations.push(op);
        id
    }

    /// Remove an operation by ID
    pub fn remove(&mut self, id: OpId) -> bool {
        let len_before = self.operations.len();
        self.operations.retain(|op| op.id() != id);
        self.operations.len() < len_before
    }

    /// Get all operations
    pub fn operations(&self) -> &[EditOperation] {
        &self.operations
    }

    /// Get operations for a specific page
    pub fn operations_for_page(&self, page: u32) -> Vec<&EditOperation> {
        self.operations.iter().filter(|op| op.page() == page).collect()
    }

    /// Check if there are any changes
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Serialize to JSON for storage/debugging
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
```

---

## Phase 4: PDF Writer (Apply Operations)

### Add to `crates/pdfjoin-core/src/apply_operations.rs`:

```rust
use crate::operations::{EditOperation, OperationLog, PdfRect};
use crate::error::PdfJoinError;
use lopdf::{Document, Object, Dictionary, Stream, ObjectId};
use std::collections::BTreeMap;

/// Apply all operations from the log to a PDF document
pub fn apply_operations(
    pdf_bytes: &[u8],
    log: &OperationLog,
) -> Result<Vec<u8>, PdfJoinError> {
    let mut doc = Document::load_mem(pdf_bytes)
        .map_err(|e| PdfJoinError::ParseError(e.to_string()))?;

    // Group operations by page for efficiency
    let pages = doc.get_pages();
    let page_ids: Vec<(u32, ObjectId)> = pages.into_iter().collect();

    for (page_num, page_id) in &page_ids {
        let page_ops = log.operations_for_page(*page_num);
        if page_ops.is_empty() {
            continue;
        }

        for op in page_ops {
            apply_single_operation(&mut doc, *page_id, op)?;
        }
    }

    // Handle page deletions (do these last, in reverse order)
    let mut deletions: Vec<u32> = log.operations()
        .iter()
        .filter_map(|op| {
            if let EditOperation::DeletePage { page, .. } = op {
                Some(*page)
            } else {
                None
            }
        })
        .collect();
    deletions.sort_by(|a, b| b.cmp(a));  // Reverse order

    for page_num in deletions {
        doc.delete_pages(&[page_num]);
    }

    // Save to bytes
    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| PdfJoinError::OperationError(e.to_string()))?;

    Ok(output)
}

fn apply_single_operation(
    doc: &mut Document,
    page_id: ObjectId,
    op: &EditOperation,
) -> Result<(), PdfJoinError> {
    match op {
        EditOperation::AddText { rect, text, style, .. } => {
            add_text_annotation(doc, page_id, rect, text, style)
        }
        EditOperation::AddHighlight { rect, color, opacity, .. } => {
            add_highlight_annotation(doc, page_id, rect, color, *opacity)
        }
        EditOperation::AddDrawing { paths, color, stroke_width, .. } => {
            add_ink_annotation(doc, page_id, paths, color, *stroke_width)
        }
        EditOperation::AddCheckbox { rect, checked, .. } => {
            add_checkbox_annotation(doc, page_id, rect, *checked)
        }
        // Image and page ops handled separately
        _ => Ok(())
    }
}

fn add_text_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
    text: &str,
    style: &crate::operations::TextStyle,
) -> Result<(), PdfJoinError> {
    // Create FreeText annotation
    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"FreeText".to_vec()));
    annot.set("Rect", Object::Array(vec![
        Object::Real(rect.x),
        Object::Real(rect.y),
        Object::Real(rect.x + rect.width),
        Object::Real(rect.y + rect.height),
    ]));
    annot.set("Contents", Object::String(
        text.as_bytes().to_vec(),
        lopdf::StringFormat::Literal,
    ));

    // Default appearance string
    let da = format!(
        "/Helvetica {} Tf {} {} {} rg",
        style.font_size,
        0.0, 0.0, 0.0  // Black color (would parse style.color)
    );
    annot.set("DA", Object::String(da.into_bytes(), lopdf::StringFormat::Literal));

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)?;

    Ok(())
}

fn add_highlight_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
    _color: &str,
    opacity: f64,
) -> Result<(), PdfJoinError> {
    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"Highlight".to_vec()));
    annot.set("Rect", Object::Array(vec![
        Object::Real(rect.x),
        Object::Real(rect.y),
        Object::Real(rect.x + rect.width),
        Object::Real(rect.y + rect.height),
    ]));
    // QuadPoints for highlight shape
    annot.set("QuadPoints", Object::Array(vec![
        Object::Real(rect.x), Object::Real(rect.y + rect.height),
        Object::Real(rect.x + rect.width), Object::Real(rect.y + rect.height),
        Object::Real(rect.x), Object::Real(rect.y),
        Object::Real(rect.x + rect.width), Object::Real(rect.y),
    ]));
    annot.set("CA", Object::Real(opacity));  // Opacity
    annot.set("C", Object::Array(vec![  // Yellow
        Object::Real(1.0), Object::Real(1.0), Object::Real(0.0)
    ]));

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)?;

    Ok(())
}

fn add_ink_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    paths: &[Vec<[f64; 2]>],
    _color: &str,
    stroke_width: f64,
) -> Result<(), PdfJoinError> {
    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"Ink".to_vec()));

    // Calculate bounding box
    let (min_x, min_y, max_x, max_y) = calculate_paths_bounds(paths);
    annot.set("Rect", Object::Array(vec![
        Object::Real(min_x), Object::Real(min_y),
        Object::Real(max_x), Object::Real(max_y),
    ]));

    // InkList: array of arrays of coordinates
    let ink_list: Vec<Object> = paths.iter().map(|path| {
        Object::Array(path.iter().flat_map(|[x, y]| {
            vec![Object::Real(*x), Object::Real(*y)]
        }).collect())
    }).collect();
    annot.set("InkList", Object::Array(ink_list));

    // Border style
    let mut bs = Dictionary::new();
    bs.set("W", Object::Real(stroke_width));
    annot.set("BS", Object::Dictionary(bs));

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)?;

    Ok(())
}

fn add_checkbox_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
    checked: bool,
) -> Result<(), PdfJoinError> {
    // Use a stamp annotation with checkmark appearance
    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"Square".to_vec()));
    annot.set("Rect", Object::Array(vec![
        Object::Real(rect.x),
        Object::Real(rect.y),
        Object::Real(rect.x + rect.width),
        Object::Real(rect.y + rect.height),
    ]));

    if checked {
        // Green fill for checked
        annot.set("IC", Object::Array(vec![
            Object::Real(0.2), Object::Real(0.8), Object::Real(0.2)
        ]));
    }

    annot.set("C", Object::Array(vec![  // Black border
        Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)
    ]));

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)?;

    Ok(())
}

fn add_annotation_to_page(
    doc: &mut Document,
    page_id: ObjectId,
    annot_id: ObjectId,
) -> Result<(), PdfJoinError> {
    let page = doc.get_object_mut(page_id)
        .map_err(|e| PdfJoinError::OperationError(e.to_string()))?;

    if let Object::Dictionary(ref mut page_dict) = page {
        let annots = page_dict.get_mut(b"Annots");
        match annots {
            Some(Object::Array(ref mut arr)) => {
                arr.push(Object::Reference(annot_id));
            }
            _ => {
                page_dict.set("Annots", Object::Array(vec![
                    Object::Reference(annot_id)
                ]));
            }
        }
    }

    Ok(())
}

fn calculate_paths_bounds(paths: &[Vec<[f64; 2]>]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for path in paths {
        for [x, y] in path {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
    }

    (min_x, min_y, max_x, max_y)
}
```

---

## Phase 5: Edit Session (WASM Bindings)

### Add `apps/pdfjoin-web/wasm/src/edit_session.rs`:

```rust
use pdfjoin_core::operations::{OperationLog, EditOperation, PdfRect, TextStyle};
use pdfjoin_core::apply_operations::apply_operations;
use pdfjoin_core::has_signatures;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct EditSession {
    document_bytes: Vec<u8>,
    document_name: String,
    page_count: u32,
    operations: OperationLog,
    is_signed: bool,
}

#[wasm_bindgen]
impl EditSession {
    /// Create a new edit session with the given PDF
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str, bytes: &[u8]) -> Result<EditSession, JsValue> {
        // Check for signatures first
        let is_signed = has_signatures(bytes)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Get page count
        let page_count = pdfjoin_core::get_page_count(bytes)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(EditSession {
            document_bytes: bytes.to_vec(),
            document_name: name.to_string(),
            page_count,
            operations: OperationLog::new(),
            is_signed,
        })
    }

    /// Check if the document is signed
    #[wasm_bindgen(getter, js_name = isSigned)]
    pub fn is_signed(&self) -> bool {
        self.is_signed
    }

    /// Get page count
    #[wasm_bindgen(getter, js_name = pageCount)]
    pub fn page_count(&self) -> u32 {
        self.page_count
    }

    /// Get document name
    #[wasm_bindgen(getter, js_name = documentName)]
    pub fn document_name(&self) -> String {
        self.document_name.clone()
    }

    /// Get document bytes for PDF.js rendering
    #[wasm_bindgen(js_name = getDocumentBytes)]
    pub fn get_document_bytes(&self) -> js_sys::Uint8Array {
        let array = js_sys::Uint8Array::new_with_length(self.document_bytes.len() as u32);
        array.copy_from(&self.document_bytes);
        array
    }

    /// Add a text annotation
    #[wasm_bindgen(js_name = addText)]
    pub fn add_text(
        &mut self,
        page: u32,
        x: f64, y: f64, width: f64, height: f64,
        text: &str,
        font_size: f64,
        color: &str,
    ) -> u64 {
        let op = EditOperation::AddText {
            id: 0,
            page,
            rect: PdfRect { x, y, width, height },
            text: text.to_string(),
            style: TextStyle {
                font_size,
                color: color.to_string(),
                bold: false,
                italic: false,
            },
        };
        self.operations.add(op)
    }

    /// Add a highlight
    #[wasm_bindgen(js_name = addHighlight)]
    pub fn add_highlight(
        &mut self,
        page: u32,
        x: f64, y: f64, width: f64, height: f64,
        color: &str,
        opacity: f64,
    ) -> u64 {
        let op = EditOperation::AddHighlight {
            id: 0,
            page,
            rect: PdfRect { x, y, width, height },
            color: color.to_string(),
            opacity,
        };
        self.operations.add(op)
    }

    /// Add an ink/drawing annotation
    #[wasm_bindgen(js_name = addDrawing)]
    pub fn add_drawing(
        &mut self,
        page: u32,
        paths_json: &str,  // JSON array of paths
        color: &str,
        stroke_width: f64,
    ) -> Result<u64, JsValue> {
        let paths: Vec<Vec<[f64; 2]>> = serde_json::from_str(paths_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid paths JSON: {}", e)))?;

        let op = EditOperation::AddDrawing {
            id: 0,
            page,
            paths,
            color: color.to_string(),
            stroke_width,
        };
        Ok(self.operations.add(op))
    }

    /// Add a checkbox
    #[wasm_bindgen(js_name = addCheckbox)]
    pub fn add_checkbox(
        &mut self,
        page: u32,
        x: f64, y: f64, width: f64, height: f64,
        checked: bool,
    ) -> u64 {
        let op = EditOperation::AddCheckbox {
            id: 0,
            page,
            rect: PdfRect { x, y, width, height },
            checked,
        };
        self.operations.add(op)
    }

    /// Remove an operation by ID
    #[wasm_bindgen(js_name = removeOperation)]
    pub fn remove_operation(&mut self, id: u64) -> bool {
        self.operations.remove(id)
    }

    /// Check if there are unsaved changes
    #[wasm_bindgen(js_name = hasChanges)]
    pub fn has_changes(&self) -> bool {
        !self.operations.is_empty()
    }

    /// Get operations as JSON (for debugging/persistence)
    #[wasm_bindgen(js_name = getOperationsJson)]
    pub fn get_operations_json(&self) -> Result<String, JsValue> {
        self.operations.to_json()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Apply all operations and return the modified PDF
    pub fn export(&self) -> Result<js_sys::Uint8Array, JsValue> {
        if self.is_signed {
            return Err(JsValue::from_str(
                "Cannot export: Document is signed. Editing would invalidate the signature."
            ));
        }

        let result = apply_operations(&self.document_bytes, &self.operations)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let array = js_sys::Uint8Array::new_with_length(result.len() as u32);
        array.copy_from(&result);
        Ok(array)
    }
}
```

---

## Phase 6: Edit Tab UI

### Add to index.html tabs:

```html
<nav class="tabs">
    <button class="tab active" data-tab="split">Split PDF</button>
    <button class="tab" data-tab="merge">Merge PDFs</button>
    <button class="tab" data-tab="edit">Edit PDF</button>  <!-- NEW -->
</nav>
```

### Edit View HTML:

```html
<!-- Edit View -->
<section id="edit-view" class="view hidden">
    <!-- Drop zone (initial state) -->
    <div id="edit-drop-zone" class="drop-zone">
        <input type="file" id="edit-file-input" accept=".pdf" hidden>
        <div class="drop-content">
            <div class="drop-icon">✏️</div>
            <p>Drag & drop a PDF here or <button id="edit-browse-btn">browse</button></p>
            <p class="drop-hint">Add text, highlights, and annotations</p>
        </div>
    </div>

    <!-- Signature warning (shown for signed PDFs) -->
    <div id="edit-signed-warning" class="signed-warning hidden">
        <div class="warning-icon">⚠️</div>
        <h2>This PDF Has a Digital Signature</h2>
        <p class="warning-text">
            Editing this document would break its signature and make it invalid.
        </p>
        <div class="warning-options">
            <h3>What you can do:</h3>
            <ul>
                <li>Use <strong>Split</strong> or <strong>Merge</strong> (signatures will be removed)</li>
                <li>Contact the sender for an unsigned copy</li>
                <li>Coming soon: Edit signed docs at getsignatures.org</li>
            </ul>
        </div>
        <div class="warning-actions">
            <button id="edit-go-back-btn" class="primary-btn">← Go Back</button>
            <button id="edit-use-split-btn" class="secondary-btn">Use Split/Merge Instead</button>
        </div>
    </div>

    <!-- Editor (shown for unsigned PDFs) -->
    <div id="edit-editor" class="edit-editor hidden">
        <!-- Toolbar -->
        <div class="edit-toolbar">
            <div class="toolbar-section">
                <button id="tool-select" class="tool-btn active" title="Select">
                    <span class="tool-icon">👆</span>
                    <span class="tool-label">Select</span>
                </button>
                <button id="tool-text" class="tool-btn" title="Add Text">
                    <span class="tool-icon">T</span>
                    <span class="tool-label">Text</span>
                </button>
                <button id="tool-highlight" class="tool-btn" title="Highlight">
                    <span class="tool-icon">🖍️</span>
                    <span class="tool-label">Highlight</span>
                </button>
                <button id="tool-draw" class="tool-btn" title="Draw/Sign">
                    <span class="tool-icon">✍️</span>
                    <span class="tool-label">Draw</span>
                </button>
                <button id="tool-checkbox" class="tool-btn" title="Checkbox">
                    <span class="tool-icon">☑️</span>
                    <span class="tool-label">Check</span>
                </button>
            </div>

            <div class="toolbar-section toolbar-right">
                <button id="edit-undo-btn" class="tool-btn" title="Undo" disabled>
                    ↶ Undo
                </button>
                <button id="edit-download-btn" class="primary-btn" disabled>
                    💾 Download Edited PDF
                </button>
            </div>
        </div>

        <!-- File info bar -->
        <div class="edit-file-info">
            <span id="edit-file-name" class="file-name"></span>
            <span id="edit-file-details" class="file-details"></span>
            <button id="edit-remove-btn" class="remove-btn" title="Remove file">×</button>
        </div>

        <!-- PDF viewer with overlays -->
        <div id="edit-viewer" class="edit-viewer">
            <div id="edit-pages" class="edit-pages">
                <!-- Pages will be rendered here -->
            </div>
        </div>

        <!-- Page navigation -->
        <div class="edit-nav">
            <button id="edit-prev-page" class="nav-btn" disabled>← Previous</button>
            <span id="edit-page-indicator">Page 1 of 1</span>
            <button id="edit-next-page" class="nav-btn" disabled>Next →</button>
        </div>
    </div>

    <div id="edit-error" class="error-msg hidden">
        <span class="error-text"></span>
        <button class="dismiss">×</button>
    </div>
</section>
```

### CSS for Edit View (add to <style>):

```css
/* Signed PDF Warning */
.signed-warning {
    background: #fef3c7;
    border: 2px solid #f59e0b;
    border-radius: 12px;
    padding: 2rem;
    text-align: center;
    max-width: 600px;
    margin: 0 auto;
}
.signed-warning .warning-icon {
    font-size: 4rem;
    margin-bottom: 1rem;
}
.signed-warning h2 {
    font-size: 1.5rem;
    margin-bottom: 1rem;
    color: #92400e;
}
.signed-warning .warning-text {
    font-size: 1.1rem;
    color: #78350f;
    margin-bottom: 1.5rem;
}
.signed-warning .warning-options {
    text-align: left;
    background: white;
    padding: 1rem;
    border-radius: 8px;
    margin-bottom: 1.5rem;
}
.signed-warning .warning-options h3 {
    font-size: 1rem;
    margin-bottom: 0.5rem;
}
.signed-warning .warning-options ul {
    margin-left: 1.5rem;
}
.signed-warning .warning-options li {
    margin-bottom: 0.5rem;
    font-size: 1rem;
}
.signed-warning .warning-actions {
    display: flex;
    gap: 1rem;
    justify-content: center;
    flex-wrap: wrap;
}

/* Edit Toolbar */
.edit-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background: var(--card-bg);
    border: 1px solid var(--border);
    border-radius: 12px 12px 0 0;
    flex-wrap: wrap;
    gap: 0.5rem;
}
.toolbar-section {
    display: flex;
    gap: 0.25rem;
}
.tool-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    padding: 0.5rem 0.75rem;
    background: transparent;
    border: 2px solid transparent;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;
    min-width: 60px;
}
.tool-btn:hover {
    background: rgba(37, 99, 235, 0.1);
}
.tool-btn.active {
    background: rgba(37, 99, 235, 0.15);
    border-color: var(--primary);
}
.tool-icon {
    font-size: 1.5rem;
}
.tool-label {
    font-size: 0.75rem;
    color: var(--text-muted);
}

/* Edit Viewer */
.edit-editor {
    background: var(--card-bg);
    border: 1px solid var(--border);
    border-radius: 12px;
}
.edit-file-info {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1rem;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
}
.edit-viewer {
    padding: 1rem;
    min-height: 500px;
    max-height: 70vh;
    overflow: auto;
    background: #f0f0f0;
}
.edit-pages {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    align-items: center;
}
.edit-page {
    position: relative;
    background: white;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
}
.edit-page canvas {
    display: block;
}
.edit-page .overlay-container {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
}
.edit-page .overlay-container > * {
    pointer-events: auto;
}

/* Page Navigation */
.edit-nav {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    padding: 1rem;
    border-top: 1px solid var(--border);
}
.nav-btn {
    padding: 0.5rem 1rem;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
    font-size: 1rem;
}
.nav-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

/* Overlay Elements */
.edit-text-overlay {
    background: rgba(255, 255, 200, 0.9);
    border: 1px solid #ccc;
    padding: 4px 8px;
    cursor: move;
    min-width: 100px;
    min-height: 20px;
}
.edit-highlight-overlay {
    background: rgba(255, 255, 0, 0.3);
    cursor: move;
}
.edit-drawing-overlay {
    pointer-events: none;
}
.edit-checkbox-overlay {
    width: 20px;
    height: 20px;
    border: 2px solid #333;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
}
.edit-checkbox-overlay.checked {
    background: #4ade80;
}

/* Responsive */
@media (max-width: 600px) {
    .edit-toolbar {
        flex-direction: column;
    }
    .tool-btn {
        min-width: 50px;
        padding: 0.25rem 0.5rem;
    }
    .tool-label {
        display: none;
    }
}
```

---

## Phase 7: JavaScript Integration

### Add `www/js/edit.js`:

```javascript
// Edit PDF functionality
const { EditSession, format_bytes } = window.wasmBindings;

let editSession = null;
let currentTool = 'select';
let currentPage = 1;
let operationHistory = [];  // For undo

export function setupEditView() {
    const dropZone = document.getElementById('edit-drop-zone');
    const fileInput = document.getElementById('edit-file-input');
    const browseBtn = document.getElementById('edit-browse-btn');
    const removeBtn = document.getElementById('edit-remove-btn');
    const downloadBtn = document.getElementById('edit-download-btn');
    const goBackBtn = document.getElementById('edit-go-back-btn');
    const useSplitBtn = document.getElementById('edit-use-split-btn');

    // File input
    browseBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        fileInput.click();
    });
    dropZone.addEventListener('click', () => fileInput.click());

    // Drag and drop
    dropZone.addEventListener('dragover', (e) => {
        e.preventDefault();
        dropZone.classList.add('drag-over');
    });
    dropZone.addEventListener('dragleave', () => dropZone.classList.remove('drag-over'));
    dropZone.addEventListener('drop', (e) => {
        e.preventDefault();
        dropZone.classList.remove('drag-over');
        if (e.dataTransfer.files.length > 0) {
            handleEditFile(e.dataTransfer.files[0]);
        }
    });

    fileInput.addEventListener('change', (e) => {
        if (e.target.files.length > 0) {
            handleEditFile(e.target.files[0]);
        }
    });

    // Actions
    removeBtn.addEventListener('click', resetEditView);
    downloadBtn.addEventListener('click', downloadEditedPdf);

    // Signed PDF warning actions
    goBackBtn.addEventListener('click', resetEditView);
    useSplitBtn.addEventListener('click', () => {
        resetEditView();
        document.querySelector('[data-tab="split"]').click();
    });

    // Tool buttons
    document.querySelectorAll('.tool-btn[id^="tool-"]').forEach(btn => {
        btn.addEventListener('click', () => {
            currentTool = btn.id.replace('tool-', '');
            document.querySelectorAll('.tool-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            updateCursor();
        });
    });

    // Page navigation
    document.getElementById('edit-prev-page').addEventListener('click', () => navigatePage(-1));
    document.getElementById('edit-next-page').addEventListener('click', () => navigatePage(1));

    // Undo
    document.getElementById('edit-undo-btn').addEventListener('click', undoLastOperation);
}

async function handleEditFile(file) {
    if (file.type !== 'application/pdf') {
        showError('edit-error', 'Please select a PDF file');
        return;
    }

    try {
        const bytes = new Uint8Array(await file.arrayBuffer());
        editSession = new EditSession(file.name, bytes);

        // Check if signed
        if (editSession.isSigned) {
            document.getElementById('edit-drop-zone').classList.add('hidden');
            document.getElementById('edit-signed-warning').classList.remove('hidden');
            return;
        }

        // Show editor
        document.getElementById('edit-drop-zone').classList.add('hidden');
        document.getElementById('edit-editor').classList.remove('hidden');

        // Update file info
        document.getElementById('edit-file-name').textContent = file.name;
        document.getElementById('edit-file-details').textContent =
            `${editSession.pageCount} pages - ${format_bytes(bytes.length)}`;

        // Load PDF.js and render
        await loadPdfJs();
        await EditPdfBridge.loadDocument(editSession.getDocumentBytes());
        await renderAllPages();

        updatePageNavigation();
        updateDownloadButton();

    } catch (e) {
        showError('edit-error', 'Failed to load PDF: ' + e);
        console.error(e);
    }
}

async function loadPdfJs() {
    // PDF.js should already be loaded via CDN
    if (!window.pdfjsLib) {
        throw new Error('PDF.js not loaded');
    }
}

async function renderAllPages() {
    const container = document.getElementById('edit-pages');
    container.innerHTML = '';

    for (let i = 1; i <= editSession.pageCount; i++) {
        const pageDiv = document.createElement('div');
        pageDiv.className = 'edit-page';
        pageDiv.dataset.page = i;

        const canvas = document.createElement('canvas');
        pageDiv.appendChild(canvas);

        // Overlay container for annotations
        const overlay = document.createElement('div');
        overlay.className = 'overlay-container';
        overlay.dataset.page = i;
        pageDiv.appendChild(overlay);

        container.appendChild(pageDiv);

        // Render page
        const info = await EditPdfBridge.renderPage(i, canvas, 1.5);

        // Set up click handler for adding annotations
        overlay.addEventListener('click', (e) => handleOverlayClick(e, i));
        overlay.addEventListener('mousedown', (e) => handleOverlayMouseDown(e, i));
    }
}

function handleOverlayClick(e, pageNum) {
    if (currentTool === 'select') return;

    const overlay = e.currentTarget;
    const rect = overlay.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Convert to PDF coordinates
    const pageInfo = EditPdfBridge.getPageInfo(pageNum);
    const pdfX = (x / pageInfo.viewport.width) * pageInfo.page.view[2];
    const pdfY = pageInfo.page.view[3] - (y / pageInfo.viewport.height) * pageInfo.page.view[3];

    switch (currentTool) {
        case 'text':
            addTextAtPosition(pageNum, pdfX, pdfY, overlay, x, y);
            break;
        case 'checkbox':
            addCheckboxAtPosition(pageNum, pdfX, pdfY, overlay, x, y);
            break;
        case 'highlight':
            // Highlight needs drag selection
            break;
    }
}

function addTextAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY) {
    const text = prompt('Enter text:');
    if (!text) return;

    const opId = editSession.addText(pageNum, pdfX, pdfY - 20, 200, 20, text, 12, '#000000');
    operationHistory.push(opId);

    // Add visual overlay
    const textEl = document.createElement('div');
    textEl.className = 'edit-text-overlay';
    textEl.textContent = text;
    textEl.style.left = domX + 'px';
    textEl.style.top = domY + 'px';
    textEl.dataset.opId = opId;

    overlay.appendChild(textEl);
    updateDownloadButton();
}

function addCheckboxAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY) {
    const opId = editSession.addCheckbox(pageNum, pdfX, pdfY - 20, 20, 20, true);
    operationHistory.push(opId);

    const checkbox = document.createElement('div');
    checkbox.className = 'edit-checkbox-overlay checked';
    checkbox.textContent = '✓';
    checkbox.style.left = domX + 'px';
    checkbox.style.top = domY + 'px';
    checkbox.dataset.opId = opId;

    checkbox.addEventListener('click', (e) => {
        e.stopPropagation();
        checkbox.classList.toggle('checked');
        checkbox.textContent = checkbox.classList.contains('checked') ? '✓' : '';
    });

    overlay.appendChild(checkbox);
    updateDownloadButton();
}

function undoLastOperation() {
    if (operationHistory.length === 0) return;

    const opId = operationHistory.pop();
    editSession.removeOperation(opId);

    // Remove from DOM
    const el = document.querySelector(`[data-op-id="${opId}"]`);
    if (el) el.remove();

    updateDownloadButton();
}

function updateDownloadButton() {
    const btn = document.getElementById('edit-download-btn');
    const undoBtn = document.getElementById('edit-undo-btn');

    const hasChanges = editSession && editSession.hasChanges();
    btn.disabled = !hasChanges;
    undoBtn.disabled = operationHistory.length === 0;
}

async function downloadEditedPdf() {
    try {
        const result = editSession.export();
        const blob = new Blob([result], { type: 'application/pdf' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = editSession.documentName.replace(/\.pdf$/i, '-edited.pdf');
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

    } catch (e) {
        showError('edit-error', 'Export failed: ' + e);
    }
}

function resetEditView() {
    editSession = null;
    currentPage = 1;
    operationHistory = [];

    document.getElementById('edit-drop-zone').classList.remove('hidden');
    document.getElementById('edit-signed-warning').classList.add('hidden');
    document.getElementById('edit-editor').classList.add('hidden');
    document.getElementById('edit-file-input').value = '';
    document.getElementById('edit-pages').innerHTML = '';

    if (window.EditPdfBridge) {
        EditPdfBridge.cleanup();
    }
}

function navigatePage(delta) {
    const newPage = currentPage + delta;
    if (newPage < 1 || newPage > editSession.pageCount) return;

    currentPage = newPage;
    updatePageNavigation();

    // Scroll to page
    const pageEl = document.querySelector(`.edit-page[data-page="${currentPage}"]`);
    if (pageEl) {
        pageEl.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
}

function updatePageNavigation() {
    const indicator = document.getElementById('edit-page-indicator');
    const prevBtn = document.getElementById('edit-prev-page');
    const nextBtn = document.getElementById('edit-next-page');

    indicator.textContent = `Page ${currentPage} of ${editSession.pageCount}`;
    prevBtn.disabled = currentPage <= 1;
    nextBtn.disabled = currentPage >= editSession.pageCount;
}

function updateCursor() {
    const viewer = document.getElementById('edit-viewer');
    switch (currentTool) {
        case 'select': viewer.style.cursor = 'default'; break;
        case 'text': viewer.style.cursor = 'text'; break;
        case 'highlight': viewer.style.cursor = 'crosshair'; break;
        case 'draw': viewer.style.cursor = 'crosshair'; break;
        case 'checkbox': viewer.style.cursor = 'pointer'; break;
    }
}

function showError(containerId, message) {
    const container = document.getElementById(containerId);
    container.querySelector('.error-text').textContent = message;
    container.classList.remove('hidden');
    setTimeout(() => container.classList.add('hidden'), 8000);
}
```

### Update app.js:

```javascript
// Add to init()
import { setupEditView } from './edit.js';

export function init() {
    // ... existing code ...
    setupEditView();  // Add this
}
```

---

## Implementation Order

### Week 1: Foundation
1. ✅ Signature detection (`has_signatures()`)
2. ✅ Operation log data structures
3. ✅ Basic apply_operations (text, highlight)
4. ✅ Unit tests for all above

### Week 2: WASM Integration
5. ✅ EditSession WASM bindings
6. ✅ Copy pdf-bridge.js and coords.rs
7. ✅ Basic rendering pipeline
8. ✅ Integration tests

### Week 3: UI
9. ✅ Edit tab HTML/CSS
10. ✅ Signed PDF warning (senior-friendly)
11. ✅ Toolbar and tool switching
12. ✅ Text and checkbox tools

### Week 4: Polish
13. ✅ Highlight tool with drag selection
14. ✅ Drawing/signature tool
15. ✅ Undo functionality
16. ✅ Mobile responsive
17. ✅ E2E testing with Puppeteer

---

## Testing Strategy

### Unit Tests (Rust)
- `has_signatures()` detects various signature types
- `OperationLog` serialization/deserialization
- `apply_operations()` produces valid PDFs
- Coordinate transforms

### Integration Tests (WASM)
- EditSession lifecycle
- Operations added and removed correctly
- Export produces viewable PDF

### E2E Tests (Puppeteer)
- Signed PDF shows warning
- Unsigned PDF enters editor
- Add text, download, verify text exists
- Undo removes operation

---

## Questions for You

1. **Tool priority**: Which editing tools are most important for v1?
   - Text ✓
   - Highlight ✓
   - Checkbox ✓
   - Drawing/signature ?
   - Image stamps ?

2. **Persistence**: Should we save edit sessions to IndexedDB (like agentpdf)?

3. **Drawing complexity**: Full freehand signature, or simplified (start with stamps)?

4. **docsign-web link**: What URL should the "coming soon" link point to?
