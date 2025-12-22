# Client-Side PDF Manipulation Architecture: Split & Merge for agentPDF.org

> Architectural Blueprint for High-Performance Rust/WASM PDF Operations

## Executive Summary

The integration of PDF Split and Merge capabilities into agentPDF.org represents a critical evolution from a "fill-and-dispatch" workflow into a comprehensive document lifecycle management hub. This feature set leverages Rust and WebAssembly (WASM) to achieve:

- **Absolute User Privacy** - Documents never leave the browser during split/merge operations
- **Zero Server Costs** - All computation occurs client-side
- **Near-Native Performance** - Sub-second operations with responsive UX

The implementation creates a new `pdfjoin-web` app under `apps/` with routes at:
- `agentpdf.org/split` - Split PDFs into individual pages or ranges
- `agentpdf.org/merge` - Combine multiple PDFs into a single document

### Strategic Positioning: Virtual Product Framework

The requirement to "track usage in isolation" introduces a sophisticated analytics pattern. This enables evaluation of the toolset as a potential spin-off product while residing within the parent ecosystem. The "Virtual Product" analytics framework decouples performance metrics from the broader platform.

---

## Part I: Core Technical Strategy

### 1.1 The Case for lopdf

The `lopdf` crate is the optimal choice for this implementation:

| Library | Type | Use Case | Binary Size |
|---------|------|----------|-------------|
| **lopdf** | Pure Rust | Low-level PDF object manipulation | ~200KB |
| pdf-writer | Pure Rust | Creating new documents from scratch | N/A for manipulation |
| pdfium-render | C++ wrapper | Full rendering pipeline | ~2MB+ |

**lopdf advantages:**

1. **Object Graph Traversal** - Walk the tree of objects to identify exactly which resources (fonts, images) are used by a specific page
2. **Object ID Manipulation** - Renumber every object during merge operations to prevent ID collisions
3. **Encryption Handling** - Native capabilities to detect and handle encrypted streams
4. **Pure Rust** - No FFI overhead, predictable WASM compilation

### 1.2 Web Worker Concurrency Model

The architecture splits processing between two execution contexts:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    BROWSER EXECUTION MODEL                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   MAIN THREAD (UI Layer)              WORKER THREAD (Compute)      │
│   ┌─────────────────────┐             ┌─────────────────────┐      │
│   │ - Render UI         │             │ - WASM Runtime      │      │
│   │ - Handle drag-drop  │  Transferable│ - lopdf Document   │      │
│   │ - File → ArrayBuffer│ ──────────► │ - Graph surgery     │      │
│   │ - PDF.js thumbnails │   Objects   │ - Serialize result  │      │
│   └─────────────────────┘             └─────────────────────┘      │
│                                                                     │
│   Zero-Copy Transfer: Ownership moves, no cloning                   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Critical Optimization: Transferable Objects**

Standard `postMessage` clones data, which is disastrous for large PDFs (copying 50MB takes time and doubles memory). Transferable objects transfer ownership of the underlying memory chunk instantly (zero-copy).

### 1.3 Memory Management Constraints

WASM memory limits in browsers:
- Desktop: 2-4GB theoretical, practical ~1GB
- Mobile: Often 500MB or less

**Memory Inflation Factor:** Loading a 100MB PDF via lopdf creates Rust structs for every object, resulting in 4x-10x memory usage.

**Mitigation Strategies:**

1. **Scope-Based Dropping** - Drop raw input `Vec<u8>` immediately after parsing
2. **Streaming Merge** - Process documents sequentially: Load A → Append → Drop A → Load B
3. **Lazy Loading** - WASM module fetched only when user clicks "Tools" button

---

## Part II: PDF Merge Engine

### 2.1 The ID Renumbering Algorithm

Merging PDFs requires preventing Object ID collisions between documents.

**Process Flow:**

```rust
// Pseudocode for merge algorithm
fn merge_documents(sources: Vec<Document>) -> Document {
    let mut dest = Document::new(PdfVersion::V1_7);

    for source in sources {
        // 1. Calculate offset
        let max_id = dest.max_object_id();

        // 2. Remap all object IDs in source
        for (old_id, object) in source.objects {
            let new_id = old_id + max_id;

            // 3. Rewrite internal references
            let remapped = rewrite_references(object, max_id);
            dest.insert(new_id, remapped);
        }

        // 4. Graft page tree
        dest.append_pages_from(&source, max_id);
    }

    // 5. Compress output
    dest.compress();
    dest
}
```

### 2.2 Resource Deduplication Strategy

| Resource Type | MVP Behavior | Future (v1.1) |
|--------------|--------------|---------------|
| Standard 14 Fonts | Deduplicate | Deduplicate |
| Embedded Fonts | Allow duplication | Hash-based dedup |
| Images | Allow duplication | Hash-based dedup |

**Rationale:** Deep deduplication requires hashing binary streams, which is CPU-intensive in WASM. Risk of subtle rendering bugs outweighs file size benefits for initial release.

### 2.3 Stream Compression

The `doc.compress()` method in lopdf enables "Object Streams" where multiple small objects are compressed together. This reduces file size by **11-61%** and must be called before serialization.

---

## Part III: PDF Split Engine

### 3.1 The "Construction by Whitelist" Algorithm

A naive split (delete unwanted pages) leaves orphan resources in the file. The SOTA approach constructs a new document containing only required resources.

**Algorithm:**

```rust
fn split_document(source: Document, pages: Vec<u32>) -> Document {
    // 1. Identify target page ObjectIds
    let target_pages: HashSet<ObjectId> = pages
        .iter()
        .map(|p| source.page_object_id(*p))
        .collect();

    // 2. Build dependency graph via recursive traversal
    let mut keep_set: HashSet<ObjectId> = HashSet::new();
    for page_id in &target_pages {
        traverse_dependencies(source, *page_id, &mut keep_set);
    }

    // 3. Construct pristine document with only keep_set objects
    let mut dest = Document::new(source.version);
    for id in keep_set {
        dest.insert(id, source.get(id).clone());
    }

    // 4. Rebuild page tree
    dest.rebuild_page_tree(&target_pages);

    dest
}

fn traverse_dependencies(doc: &Document, id: ObjectId, keep: &mut HashSet<ObjectId>) {
    if keep.contains(&id) { return; }
    keep.insert(id);

    // Recursively add referenced objects
    for ref_id in doc.get_references(id) {
        traverse_dependencies(doc, ref_id, keep);
    }
}
```

**Result:** New PDF contains exactly the requested pages and exactly the resources they need. No zombie objects. No bloat.

### 3.2 Complex Range Handling

The frontend sends a structured command; parsing happens in Rust:

```rust
#[derive(Deserialize)]
pub struct SplitCommand {
    /// Ranges like [(1,3), (5,5), (8,10)] for "pages 1-3, 5, and 8-10"
    pub ranges: Vec<(u32, u32)>,
    pub file_bytes: Vec<u8>,
}

impl SplitCommand {
    pub fn flatten_pages(&self) -> Vec<u32> {
        self.ranges
            .iter()
            .flat_map(|(start, end)| *start..=*end)
            .sorted()
            .dedup()
            .collect()
    }
}
```

---

## Part IV: Generic Command Pattern

### 4.1 The Extensible Interface

To support future operations (Rotate, Watermark, Compress, OCR), implement a generic command processor:

```rust
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum PdfCommand {
    Merge { files: Vec<Vec<u8>> },
    Split { file: Vec<u8>, ranges: Vec<(u32, u32)> },
    // Future extensions:
    // Rotate { file: Vec<u8>, pages: Vec<u32>, angle: i32 },
    // Watermark { file: Vec<u8>, text: String, position: Position },
    // Compress { file: Vec<u8>, level: CompressionLevel },
}

#[derive(Serialize)]
pub struct ProcessResult {
    pub success: bool,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
    pub metrics: Option<ProcessMetrics>,
}

#[derive(Serialize)]
pub struct ProcessMetrics {
    pub input_size_bytes: usize,
    pub output_size_bytes: usize,
    pub page_count: u32,
    pub processing_time_ms: u64,
}

#[wasm_bindgen]
pub fn process_pdf(command_json: &str) -> JsValue {
    console_error_panic_hook::set_once();

    let command: PdfCommand = match serde_json::from_str(command_json) {
        Ok(c) => c,
        Err(e) => return error_result(&e.to_string()),
    };

    let start = web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now());

    let result = match command {
        PdfCommand::Merge { files } => execute_merge(files),
        PdfCommand::Split { file, ranges } => execute_split(file, ranges),
    };

    serde_wasm_bindgen::to_value(&result).unwrap()
}
```

### 4.2 Binary Size Optimization

**Cargo.toml Configuration:**

```toml
[dependencies]
lopdf = { version = "0.32", default-features = false }
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
console_error_panic_hook = "0.1.7"

[profile.release]
lto = true
opt-level = "z"      # Optimize for size
codegen-units = 1
panic = "abort"
```

**Expected Binary Size:** ~200-400KB (uncompressed), ~80-150KB (gzipped)

---

## Part V: User Experience Design

### 5.1 Integration Pattern: Quick Tools Dashboard

The new tools integrate as secondary actions, not replacing the primary signing workflow:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    agentPDF.org HOME SCREEN                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   PRIMARY ACTION                                                    │
│   ┌─────────────────────────────────────────────────────────────┐  │
│   │              [ Start New Agreement ]                        │  │
│   └─────────────────────────────────────────────────────────────┘  │
│                                                                     │
│   ───────────────────────────────────────────────────────────────  │
│                                                                     │
│   DOCUMENT UTILITIES                                                │
│   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐   │
│   │   Split PDF     │  │   Merge PDFs    │  │ Organize Pages  │   │
│   │  Extract pages  │  │ Combine files   │  │ Reorder/Delete  │   │
│   └─────────────────┘  └─────────────────┘  └─────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 5.2 Unified Drop Zone Logic

```javascript
const handleDrop = (files) => {
    if (files.length === 1) {
        // Single file → Split/Organize mode
        enterSplitMode(files[0]);
    } else {
        // Multiple files → Merge mode
        enterMergeMode(files);
    }
};
```

### 5.3 Split UI: Visual Page Grid

**Rendering Strategy:**
- Use PDF.js (browser's main thread) to render page thumbnails
- lopdf handles structural manipulation only (no rasterization)
- User clicks thumbnails to select/deselect pages
- Selected indices sent to WASM worker for processing

### 5.4 Operation Lifecycle

```
┌─────────────────────────────────────────────────────────────────────┐
│                    OPERATION HEARTBEAT                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   1. INITIATION (The Handshake)                                     │
│      User clicks "Process" → UI locks → postMessage to Worker      │
│                                                                     │
│   2. EXECUTION (The Black Box)                                      │
│      Worker: foreach doc { process → Msg::Progress { n, total } }  │
│                                                                     │
│   3. FEEDBACK (The Pulse)                                           │
│      Main Thread receives progress → Updates progress bar           │
│                                                                     │
│   4. COMPLETION (The Result)                                        │
│      Worker: Msg::Success { blob } → Download triggered             │
│      Analytics event fired with metrics                             │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Part VI: Isolated Analytics Framework

### 6.1 Virtual Product Namespace

All tool events use a distinct namespace, never sharing names with core platform events:

| Core Platform Events | Tool Events |
|---------------------|-------------|
| `sign_flow_start` | `tool_split_init` |
| `template_selected` | `tool_merge_add_file` |
| `signature_complete` | `tool_process_complete` |

### 6.2 Context Dimension

```javascript
// User property switches based on active context
const trackEvent = (name, data) => {
    const context = isInToolsMode() ? 'lab_tools' : 'core';
    analytics.track(name, { ...data, product_context: context });
};
```

### 6.3 Spin-off Viability KPIs

| Metric | Description | Spin-off Signal |
|--------|-------------|-----------------|
| **Tool-Only Cohort** | Users who only use tools, never sign | >20% = separate audience |
| **Processing Intensity** | `input_size_mb`, `page_count`, `processing_time_ms` | 50MB+ = professional users |
| **Bailout Rate** | `tool_error_oom` (Out of Memory) events | High = needs server backend |
| **Feature Depth** | Merge 10+ files, Split complex ranges | Advanced = willingness to pay |

---

## Part VII: Implementation Architecture

### 7.1 New Crate Structure

```
apps/
├── agentpdf-web/          # Existing
├── docsign-web/           # Existing
└── pdfjoin-web/           # NEW
    ├── Cargo.toml
    ├── Trunk.toml
    ├── wasm/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs      # WASM entry point
    │       ├── merge.rs    # Merge algorithm
    │       ├── split.rs    # Split algorithm
    │       └── worker.rs   # Web Worker bindings
    └── www/
        ├── index.html      # /split route
        ├── merge.html      # /merge route
        ├── js/
        │   ├── worker.js   # Web Worker host
        │   └── ui.js       # UI logic
        └── css/
            └── styles.css
```

### 7.2 Routing Integration

The pdfjoin app deploys as sub-paths of agentPDF.org:

| Route | Handler | Purpose |
|-------|---------|---------|
| `agentpdf.org/` | agentpdf-web | Existing functionality |
| `agentpdf.org/split` | pdfjoin-web | Split tool |
| `agentpdf.org/merge` | pdfjoin-web | Merge tool |

**Deployment:** Configure Cloudflare Pages to serve pdfjoin-web's `www/dist` at the `/split` and `/merge` paths.

---

## Part VIII: Dependencies and Versions

| Crate | Version | Purpose |
|-------|---------|---------|
| `lopdf` | 0.32.0 | PDF object manipulation |
| `wasm-bindgen` | 0.2 | JS interop |
| `js-sys` | 0.3 | JS type interaction |
| `web-sys` | 0.3 | Web API bindings |
| `serde` | 1.0 | Serialization |
| `serde_json` | 1.0 | JSON handling |
| `console_error_panic_hook` | 0.1.7 | Debug panics in browser |

---

## References

1. [lopdf GitHub Repository](https://github.com/J-F-Liu/lopdf)
2. [lopdf Docs.rs Documentation](https://docs.rs/lopdf)
3. [Transferable Objects - MDN](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects)
4. [WebAssembly for Document Processing - Nutrient](https://www.nutrient.io/blog/webassembly-javascript-document-processing/)
5. [PDF Manipulation with Rust - Dario Cancelliere](https://www.dariocancelliere.it/blog/2020/09/29/pdf-manipulation-with-rust-and-considerations/)
6. [pdf-lib Discussion on Unused Resources](https://github.com/Hopding/pdf-lib/discussions/1661)
7. [Navigation UX Patterns - Userpilot](https://userpilot.com/blog/navigation-ux/)
8. [Google Analytics Setup Best Practices - Philip Walton](https://philipwalton.com/articles/the-ga-setup-i-use-on-every-site-i-build/)
