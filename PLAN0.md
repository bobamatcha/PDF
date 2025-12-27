# Monolith Integration Plan - Part 1 of 4

> **This is Part 1 of 4. See also:** [PLAN1.md](./PLAN1.md), [PLAN2.md](./PLAN2.md), [PLAN3.md](./PLAN3.md)

> **Development Guidelines**: See [CLAUDE.md](./CLAUDE.md) for test-first development practices.
>
> **Known Issues**: See [KNOWN_ISSUES.md](./KNOWN_ISSUES.md) for tracked bugs and investigation items.

## Table of Contents (All Parts)

**Part 1 (This File):**
- UX Principle: Design for Clarity
- PDFJoin Editor Architecture Refactoring
- PDF Split/Merge Tools
- Tampa Demo Priority
- Existing Template Gap Analysis
- Four Demo Structure

**Part 2 ([PLAN1.md](./PLAN1.md)):**
- Implementation Roadmap
- 16-State Compliance Engine
- Quick Reference
- Florida Compliance Features
- Local-First Template Generation
- Current Progress
- Phases 4-6 (Template Expansion, Tax Platform, Estate Planning)

**Part 3 ([PLAN2.md](./PLAN2.md)):**
- Phase 7: Web Performance Benchmarking
- Executive Summary
- Architecture Overview
- Existing Assets Inventory
- Shared Components Strategy
- Directory Structure
- Migration Plan
- Dual-Site Deployment Strategy
- Phase 0: ASAP Deployment

**Part 4 ([PLAN3.md](./PLAN3.md)):**
- Phases 1-3: Foundation, Compliance Engine, Full Integration
- Test Coverage Strategy
- Demo Functionality Preservation
- Future Considerations
- Summary
- Appendices

---

## UX Principle: Design for Clarity

> **The interface must work FOR users, not make users work.**
>
> Design for clarity over flexibility. Elderly users should never need to learn workarounds—if they must, the UI is broken. Every interaction should be obvious and forgiving.

---

## PDFJoin Editor Architecture Refactoring

**Goal**: Move editor state from TypeScript to Rust ("thin TS / thick Rust")

### Progress Tracker

| Phase | Status | Description |
|-------|--------|-------------|
| TS Migration | ✅ Done | Converted JS → TypeScript with esbuild + Trunk |
| Whitebox Bug Fix | ✅ Done | Fixed expansion bug using Range API |
| Tool Removal | ✅ Done | Checkbox/highlight tools commented out pending testing |
| Rust Update Methods | ✅ Done | Add `set_checkbox`, `update_rect`, `update_text` to Rust |
| UX Improvements | ✅ Done | Updated messaging, linked to Boba Matcha blog |
| Tab PDF Sharing | ✅ Done | Share PDF between Split/Edit tabs without re-upload |
| Edit→Split Flow | ✅ Done | Prompt to save changes when switching from Edit to Split |
| Text Tool Refactor | ✅ Done | Separate tools: TextBox (transparent) + Whiteout (covers content) |
| Text Tool Tests | ✅ Done | Browser tests for textbox/whiteout behavior |
| Text Sizing UX | ✅ Done | Expansion on font size increase, content growth, page boundaries |
| Action-Based Undo | ✅ Done | Transaction model in Rust, removed `operationHistory` from TS |
| Re-enable Checkbox | ✅ Done | Checkbox tool with action system for undo/redo |
| Highlight Tool | ✅ Done | Text selection-based highlight with action system |
| Underline Tool | ✅ Done | Text selection-based underline with action system |
| Export Compression | ✅ Done | Added `doc.compress()` to reduce file size |
| Coordinate Fix | ✅ Done | Fixed Y-offset bug using PDF.js viewport methods - see [ISSUE-002](./KNOWN_ISSUES.md#issue-002-highlightunderline-y-coordinate-offset-bug) |
| Flatten Export | ⚠️ Blocked | Rust works, WASM fails - see [ISSUE-001](./KNOWN_ISSUES.md#issue-001-pdf-flatten-export-not-working-in-wasm) |

### Phase 1: Rust Update Methods ✅ Complete

Added update-in-place methods to avoid fragile remove+add pattern:

```rust
// In pdfjoin-core/src/operations.rs - OperationLog
pub fn get_operation(&self, id: OpId) -> Option<&EditOperation>
pub fn set_checkbox(&mut self, id: OpId, checked: bool) -> bool
pub fn update_rect(&mut self, id: OpId, new_rect: PdfRect) -> bool
pub fn update_text(&mut self, id: OpId, text: &str, style: Option<TextStyle>) -> bool

// In pdfjoin-wasm/src/edit_session.rs - Expose to JS
#[wasm_bindgen]
pub fn set_checkbox(&mut self, op_id: u64, checked: bool) -> bool
pub fn update_rect(&mut self, op_id: u64, x: f64, y: f64, w: f64, h: f64) -> bool
pub fn update_text(&mut self, op_id: u64, text: &str) -> bool
pub fn get_operation_rect(&self, op_id: u64) -> Option<js_sys::Float64Array>
```

**Tests written and passing:**
- ✅ `test_set_checkbox_updates_operation`
- ✅ `test_update_rect_preserves_other_fields`
- ✅ `test_update_text_preserves_rect`
- ✅ `test_get_operation_returns_correct_op`
- ✅ `test_update_text_with_style`

### Phase 2: Tab PDF Sharing ✅ Complete

**Feature 1**: Split ↔ Edit bidirectional auto-load
- Store PDF bytes in shared state when loaded in any tab
- Switching tabs auto-loads the PDF (no re-upload needed)

**Feature 2**: Edit → Split with change detection
- If no changes: auto-load PDF into Split (no modal)
- If changes exist: show simple modal asking to download first
  - "Yes, Download My PDF" → downloads edited PDF, then continues
  - "No, Continue Without Saving" → continues without saving
  - "Go Back" → stay in Edit

**Implementation**:
- `apps/pdfjoin-web/src/ts/shared-state.ts` - Shared PDF state module
- `apps/pdfjoin-web/src/ts/app.ts` - Tab switching, `loadPdfIntoSplit()`, modal
- `apps/pdfjoin-web/src/ts/edit.ts` - `loadPdfIntoEdit()` export, change callbacks

**Tests** (in `crates/benchmark-harness/tests/browser_pdfjoin.rs`):
- ✅ `test_tab_sharing_split_to_edit_autoloads_pdf`
- ✅ `test_tab_sharing_edit_to_split_autoloads_without_changes`
- ✅ `test_tab_sharing_edit_to_split_shows_modal_with_changes`

Run tests: `cargo test -p benchmark-harness --test browser_pdfjoin test_tab_sharing`

### Phase 3: Text Tool Refactoring ✅ Complete

**Implementation Complete:**

| Tool | Icon | Purpose | Status |
|------|------|---------|--------|
| Select | ☝️ | Select & edit existing elements | ✅ Done |
| Text Box | T | Always transparent, dashed border | ✅ Done |
| Whiteout | ⬜ | White rectangle to cover/redact | ✅ Done |

**Completed Features:**
- ✅ TextBox always transparent (no mode toggle), resizable, movable
- ✅ Whiteout has white background, resizable
- ✅ Z-ordering: last-added gets click priority (`nextTextBoxZIndex++`)
- ✅ Delete: X button on selection + Delete key (no Trash tool)
- ✅ Delete button bug fixed: `handleWhiteoutStart` checks for UI elements before starting draw

**Existing Tests:**
- ✅ `test_textbox_toolbar_button_exists`
- ✅ `test_textbox_click_creates_text_box`

### Phase 3b: Text Tool Browser Tests ✅ Complete

**All tests passing** (in `crates/benchmark-harness/tests/browser_pdfjoin.rs`):
- ✅ `test_textbox_create_transparent` - verify CSS class and background
- ✅ `test_textbox_resize` - resize handle exists (note: simulated events limited in headless)
- ✅ `test_textbox_delete_x_button` - click X deletes box
- ✅ `test_textbox_delete_key` - Delete key removes selected box
- ✅ `test_textbox_overlap_zorder` - newer boxes on top
- ✅ `test_whiteout_covers_content` - white background
- ✅ `test_whiteout_with_text` - type in whiteout

Run tests: `cargo test -p benchmark-harness --test browser_pdfjoin test_textbox`

### Phase 3c: Text Sizing UX Tests ✅ Complete

**All tests passing** - validates elderly-friendly UX behavior:
- ✅ `test_ux_whiteout_matches_covered_text_size` - Whiteout detects covered text font size (36px from PDF)
- ✅ `test_ux_textbox_expands_on_font_size_increase` - Box grows when font size increases (12→22px, 150x30→180x41)
- ✅ `test_ux_textbox_expands_with_content` - Box expands for long text (150→500px)
- ✅ `test_ux_whiteout_expands_with_content` - Whiteout expands for long text (100x30→109x346)
- ✅ `test_ux_textbox_respects_page_boundary_right` - TextBox width constrained to page (never overflows)
- ✅ `test_ux_textbox_grows_height_at_boundary` - TextBox height grows when width is constrained (30→161px)
- ✅ `test_ux_whiteout_respects_page_boundary_right` - Whiteout respects right page edge
- ✅ `test_ux_whiteout_respects_page_boundary_bottom` - Whiteout respects bottom page edge

**Elderly UX**: Text boxes never overflow page boundaries. When text would exceed the right edge, width is constrained and height grows to wrap text instead.

Run tests: `cargo test -p benchmark-harness --test browser_pdfjoin test_ux`

### Phase 4: Action-Based Undo/Redo ✅ Complete

Replaced JS `operationHistory` with Rust transaction model ("thin TS / thick Rust"):

**Rust Implementation** (`pdfjoin-core/src/operations.rs`):
```rust
pub enum ActionKind {
    AddTextBox, AddWhiteout, AddCheckbox, AddHighlight,
    ReplaceText, Move, Resize, Delete,
}

pub struct Action {
    pub kind: ActionKind,
    pub added_ops: Vec<EditOperation>,
    pub removed_ops: Vec<EditOperation>,  // For delete undo
}

// OperationLog methods
pub fn begin_action(&mut self, kind: ActionKind)
pub fn commit_action(&mut self) -> bool
pub fn abort_action(&mut self)
pub fn undo(&mut self) -> Option<Vec<OpId>>
pub fn redo(&mut self) -> Option<Vec<OpId>>
pub fn can_undo(&self) -> bool
pub fn can_redo(&self) -> bool
pub fn record_removed_op(&mut self, op: EditOperation)
```

**WASM Bindings** (`edit_session.rs`):
- `beginAction(kind: string)` - Start action ("textbox", "whiteout", etc.)
- `commitAction() -> bool` - Finalize and push to undo stack
- `abortAction()` - Cancel pending action
- `undo() -> BigInt64Array | null` - Returns removed OpIds for DOM cleanup
- `redo() -> BigInt64Array | null` - Returns restored OpIds
- `canUndo() / canRedo()` - State queries

**TypeScript Changes** (`edit.ts`):
- Removed `operationHistory: OpId[]` array
- All operation adds wrapped with `beginAction`/`commitAction`
- `undoLastOperation()` now calls Rust `undo()` and removes DOM elements
- Added `redoLastOperation()` function
- `updateButtons()` uses `canUndo()`/`canRedo()` from Rust

**Tests passing**: 10 new tests in `pdfjoin-core/src/operations.rs`:
- ✅ `test_initial_state_cannot_undo_or_redo`
- ✅ `test_begin_and_commit_action_enables_undo`
- ✅ `test_undo_returns_op_ids_for_dom_removal`
- ✅ `test_redo_returns_op_ids_for_dom_recreation`
- ✅ `test_action_with_multiple_operations`
- ✅ `test_new_action_clears_redo_stack`
- ✅ `test_undo_multiple_actions`
- ✅ `test_undo_without_commit_does_nothing`
- ✅ `test_abort_action_removes_uncommitted_ops`
- ✅ `test_delete_action_type`

**Browser Tests** (`benchmark-harness/tests/browser_pdfjoin.rs`):
- ✅ `test_undo_redo_buttons_exist` - Both Undo and Redo buttons in toolbar
- ✅ `test_undo_keyboard_shortcut` - Ctrl+Z removes whiteout
- ✅ `test_redo_keyboard_shortcut` - Ctrl+Shift+Z restores whiteout
- ✅ `test_undo_redo_button_clicks` - Full undo/redo cycle with DOM recreation

**UI Features Added**:
- Redo button (`#edit-redo-btn`) in HTML toolbar
- Keyboard shortcuts: Ctrl+Z (undo), Ctrl+Shift+Z (redo)
- DOM recreation on redo via `recreateOperationElement()` parsing Rust JSON

Run tests: `cargo test -p benchmark-harness --test browser_pdfjoin test_undo`

### Phase 5: UX Improvements

- Remove "Free, private" messaging
- Replace with: "All Files Stay On Your Computer"
- Add: "Certified and Verified by Boba Matcha Solutions LLC"
- Link to: https://bobamatchasolutions.com/#/blog/announcing-pdfjoin

### Phase 6: Re-enable Disabled Tools

**Checkbox Tool - Done**
- Uncommented `addCheckboxAtPosition()` in `edit.ts` with action system
- Uncommented `#edit-tool-checkbox` button in `index.html`
- Uncommented browser test `test_pdfjoin_checkbox_tool_creates_annotation_on_click`
- Updated `overlayNeedsClicks` to include checkbox tool
- Rust backing complete with `set_checkbox` method

Run test: `cargo test -p benchmark-harness --test browser_pdfjoin test_pdfjoin_checkbox -- --nocapture`

**Highlight Tool - Done**
- Implemented text selection-based highlighting (not click-to-place)
- User selects text in PDF, highlight is created on mouseup
- Handles multi-line selections (creates highlight for each rect)
- Proper PDF coordinate conversion
- Uses action system for undo/redo

Run test: `cargo test -p benchmark-harness --test browser_pdfjoin test_pdfjoin_highlight_tool -- --nocapture`

**Phase 6 Complete** - Both checkbox and highlight tools working with action system support.

---

> Consolidating agentPDF-server, agentPDF-web, corpus-server, and docsign-web into a unified workspace with two deployable web applications.

---

## PDF SPLIT/MERGE TOOLS (Priority Feature)

> **Research Foundation**: See [PDFJOIN_RESEARCH.md](./PDFJOIN_RESEARCH.md) for full architectural details.

### Overview

Add client-side PDF Split and Merge capabilities as a new `pdfjoin-web` app, exposed at:
- `agentpdf.org/split` - Extract pages from PDFs
- `agentpdf.org/merge` - Combine multiple PDFs

**Key Principles:**
- All processing in-browser via Rust/WASM (zero server costs, total privacy)
- Web Worker architecture (non-blocking UI)
- Generic Command Pattern (extensible for future tools: rotate, watermark, compress)
- Isolated analytics for spin-off evaluation

### Test-First Implementation Flow

Following [CLAUDE.md](./CLAUDE.md) guidelines, each feature follows the test-first approach:

1. **Write Failing Tests** - Prove the feature is missing
2. **Confirm Failure** - Run `cargo test --all-features --workspace`
3. **Implement** - Minimal code to pass tests
4. **Confirm Pass** - All tests green
5. **Verify with Puppeteer** - Browser validation
6. **Fix Tests if Needed** - If Puppeteer reveals bugs, tests were wrong

### Implementation Phases (Parallelizable Tasks Marked with ⚡)

#### Phase 1: Foundation (Parallel Subagents Possible)

| Task | Type | Parallelizable | Test First |
|------|------|----------------|------------|
| ⚡ Create `apps/pdfjoin-web/` scaffold | Structure | Yes | N/A |
| ⚡ Create `crates/pdfjoin-core/` with lopdf integration | Rust | Yes | Yes |
| ⚡ Design command protocol types | Types | Yes | Yes |
| Wire up Trunk.toml and WASM build | Build | No (depends on scaffold) | N/A |

**Phase 1 Tests (Write First):**
```rust
// crates/pdfjoin-core/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pdf_returns_page_count() {
        let bytes = include_bytes!("../fixtures/sample-3page.pdf");
        let doc = parse_pdf(bytes).unwrap();
        assert_eq!(doc.page_count(), 3);
    }

    #[test]
    fn test_command_deserializes_merge() {
        let json = r#"{"type":"Merge","files":[]}"#;
        let cmd: PdfCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, PdfCommand::Merge { .. }));
    }
}
```

#### Phase 2: Merge Engine

| Task | Type | Parallelizable | Test First |
|------|------|----------------|------------|
| Implement ID renumbering algorithm | Core | No | Yes |
| Implement page tree grafting | Core | No | Yes |
| ⚡ Add stream compression | Optimization | Yes | Yes |
| ⚡ Add standard font deduplication | Optimization | Yes | Yes |

**Phase 2 Tests (Write First):**
```rust
#[test]
fn test_merge_two_documents_combines_pages() {
    let doc_a = load_fixture("2page.pdf"); // 2 pages
    let doc_b = load_fixture("3page.pdf"); // 3 pages
    let merged = merge_documents(vec![doc_a, doc_b]).unwrap();
    assert_eq!(merged.page_count(), 5);
}

#[test]
fn test_merge_preserves_content() {
    let doc_a = load_fixture("text-hello.pdf");
    let doc_b = load_fixture("text-world.pdf");
    let merged = merge_documents(vec![doc_a, doc_b]).unwrap();
    let text = extract_text(&merged);
    assert!(text.contains("Hello"));
    assert!(text.contains("World"));
}

#[test]
fn test_merge_no_id_collisions() {
    let doc_a = load_fixture("complex-a.pdf");
    let doc_b = load_fixture("complex-b.pdf");
    let merged = merge_documents(vec![doc_a, doc_b]).unwrap();
    // Verify no duplicate ObjectIds
    assert!(merged.validate_object_ids().is_ok());
}
```

#### Phase 3: Split Engine

| Task | Type | Parallelizable | Test First |
|------|------|----------------|------------|
| Implement dependency graph traversal | Core | No | Yes |
| Implement "Construction by Whitelist" | Core | No | Yes |
| ⚡ Range parsing (e.g., "1-3, 5, 8-10") | Utility | Yes | Yes |
| ⚡ Page tree rebuilding | Core | Yes | Yes |

**Phase 3 Tests (Write First):**
```rust
#[test]
fn test_split_extracts_single_page() {
    let doc = load_fixture("5page.pdf");
    let split = split_document(doc, vec![3]).unwrap();
    assert_eq!(split.page_count(), 1);
}

#[test]
fn test_split_extracts_range() {
    let doc = load_fixture("10page.pdf");
    let split = split_document(doc, vec![2, 3, 4, 5]).unwrap();
    assert_eq!(split.page_count(), 4);
}

#[test]
fn test_split_removes_unused_resources() {
    let doc = load_fixture("large-with-fonts.pdf");
    let original_size = doc.to_bytes().len();
    let split = split_document(doc, vec![1]).unwrap(); // First page only
    let split_size = split.to_bytes().len();
    // Split should be significantly smaller
    assert!(split_size < original_size / 2);
}

#[test]
fn test_range_parsing() {
    let ranges = parse_ranges("1-3, 5, 8-10").unwrap();
    assert_eq!(ranges, vec![1, 2, 3, 5, 8, 9, 10]);
}
```

#### Phase 4: WASM & Web Worker Integration

| Task | Type | Parallelizable | Test First |
|------|------|----------------|------------|
| WASM bindings for `process_pdf` | WASM | No | Yes |
| ⚡ Web Worker message protocol | JS | Yes | Yes |
| ⚡ Progress event emission | JS | Yes | Yes |
| Transferable Objects optimization | JS | No | Manual test |

**Phase 4 Tests:**
```rust
// WASM-level tests
#[wasm_bindgen_test]
fn test_process_pdf_merge_command() {
    let cmd = r#"{"type":"Merge","files":["base64...", "base64..."]}"#;
    let result = process_pdf(cmd);
    let parsed: ProcessResult = serde_wasm_bindgen::from_value(result).unwrap();
    assert!(parsed.success);
}
```

#### Phase 5: UI Implementation (Parallel Subagents Possible)

| Task | Type | Parallelizable | Test First |
|------|------|----------------|------------|
| ⚡ Split UI with PDF.js thumbnails | Frontend | Yes | Puppeteer |
| ⚡ Merge UI with drag-drop reorder | Frontend | Yes | Puppeteer |
| ⚡ Progress bar component | Frontend | Yes | Puppeteer |
| ⚡ Cross-links between /split and /merge | Frontend | Yes | Puppeteer |
| Unified drop zone logic | Frontend | No | Puppeteer |

#### Phase 6: Analytics & Polish

| Task | Type | Parallelizable | Test First |
|------|------|----------------|------------|
| ⚡ Implement isolated analytics namespace | Analytics | Yes | Unit test |
| ⚡ Add processing metrics collection | Analytics | Yes | Unit test |
| ⚡ Add error tracking (OOM detection) | Analytics | Yes | Unit test |
| Integration with agentpdf.org routing | Deploy | No | E2E |

### Crate Structure

```
apps/
├── agentpdf-web/          # Existing
├── docsign-web/           # Existing
└── pdfjoin-web/           # NEW
    ├── Cargo.toml
    ├── Trunk.toml
    ├── wasm/
    │   ├── Cargo.toml
    │   └── src/lib.rs
    └── www/
        ├── split.html     # /split route
        ├── merge.html     # /merge route
        └── js/worker.js

crates/
├── ...existing...
└── pdfjoin-core/          # NEW - lopdf-based manipulation
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── merge.rs
        ├── split.rs
        └── command.rs
```

### Dependencies

```toml
# crates/pdfjoin-core/Cargo.toml
[dependencies]
lopdf = { version = "0.32", default-features = false }
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"

# apps/pdfjoin-web/wasm/Cargo.toml
[dependencies]
pdfjoin-core = { path = "../../../crates/pdfjoin-core" }
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde-wasm-bindgen = "0.6"
console_error_panic_hook = "0.1.7"
```

### Subagent Delegation Guide

When implementing with Claude Code, delegate these task groups to parallel subagents:

**Subagent Group A: Core Library**
- Create `crates/pdfjoin-core/`
- Implement merge algorithm with tests
- Implement split algorithm with tests

**Subagent Group B: WASM Bindings**
- Create `apps/pdfjoin-web/wasm/`
- Implement `process_pdf` entry point
- Add WASM-specific tests

**Subagent Group C: Web UI**
- Create `apps/pdfjoin-web/www/`
- Implement Split UI with PDF.js
- Implement Merge UI with drag-drop

**Subagent Group D: Integration**
- Configure Trunk.toml
- Set up routing
- Add analytics hooks

### Quick Reference Commands

```bash
# Run all pdfjoin tests
cargo test -p pdfjoin-core --all-features

# Build WASM
cd apps/pdfjoin-web && trunk build

# Serve for development
cd apps/pdfjoin-web && trunk serve --port 8082

# Full workspace check
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --workspace
```

---

## Tampa Demo Priority (January 2026)

> **Goal:** Ship a comprehensive, featureful demo for Tampa Bay real estate meetups covering all three contract types: **Lease**, **Purchase**, and **Listing**.

### Research Foundation

Three comprehensive research documents have been created to guide implementation:

| Document | Coverage | Key Statutes |
|----------|----------|--------------|
| [FL_PURCHASE.md](./FL_PURCHASE.md) | Purchase contracts, condos, mobile homes, maritime | FAR/BAR, SB 264, F.S. 718.503, F.S. 319.261 |
| [FL_LEASE.md](./FL_LEASE.md) | Residential, commercial, mobile home park leases | Ch. 83 Pt I/II, Ch. 723, HB 1015, HB 621 |
| [FL_LIST.md](./FL_LIST.md) | Listing agreements, brokerage compliance | Ch. 475, NAR Settlement, F.S. 689.302 |

### Contract Type Priority Matrix

| Contract Type | Template | Status | Demo Priority |
|--------------|----------|--------|---------------|
| **Residential Lease** | `florida_lease.typ` | ✅ Complete (all gaps filled) | P0 - Showcase |
| **Purchase - As-Is** | `florida_purchase_as_is.typ` | ✅ Complete | P0 - Most common transaction |
| **Commercial Lease** | `florida_commercial_lease.typ` | ✅ Complete | P1 - Commercial properties |
| **Listing Agreement** | `florida_listing.typ` | 🔴 Needs creation | P0 - Brokers are the audience |

### Tampa Bay Metro Local Considerations

| Area | County | Key Local Issues | Implementation |
|------|--------|------------------|----------------|
| **Tampa** | Hillsborough | MacDill AFB (SB 264 10-mile zone), CDDs (South Tampa, Brandon), Bayshore flood zones | Military proximity check, CDD addendum trigger |
| **St. Petersburg** | Pinellas | CCCL disclosures, aging condo stock (SIRS critical), barrier islands | Coastal property rider, enhanced condo safety |
| **Clearwater** | Pinellas | Beach regulations, height restrictions, short-term rental rules | Zoning disclosure addendum |
| **Wesley Chapel** | Pasco | Extensive CDDs, agricultural transitions, Scrub Jay habitat | CDD detection, environmental disclosure |

### Metro Detection Implementation

```typescript
// Efficient zip code → metro mapping (~15KB JSON)
const FL_ZIP_METRO = {
  // Tampa Bay (~600 zips)
  "33601": { metro: "Tampa", county: "Hillsborough" },
  "33701": { metro: "St. Petersburg", county: "Pinellas" },
  // Orlando (~400 zips)
  "32801": { metro: "Orlando", county: "Orange" },
  // Miami (~500 zips)
  "33101": { metro: "Miami", county: "Miami-Dade" },
  // Jacksonville (~300 zips)
  "32201": { metro: "Jacksonville", county: "Duval" },
};

// Critical infrastructure zones for SB 264
const MILITARY_BASES = [
  { name: "MacDill AFB", lat: 27.8492, lng: -82.5213, radius_miles: 10 },
  { name: "NAS Jacksonville", lat: 30.3867, lng: -81.6800, radius_miles: 10 },
  { name: "NS Mayport", lat: 30.3936, lng: -81.4183, radius_miles: 10 },
];
```

**Detection Priority:**
1. Zip code lookup (fastest, ~1ms)
2. Parcel ID prefix (Hillsborough = 19-XXXXX)
3. Geocode address (async, ~200ms, requires API)
4. Manual selection (fallback)

---

## Existing Template Gap Analysis

### `florida_lease.typ` - Current State Assessment

**What's Implemented Well:**

| Aspect | Status | Notes |
|--------|--------|-------|
| Modular Addenda Architecture | ✅ Excellent | Uses `#if get_bool()` for conditional sections |
| Radon Gas (§ 404.056) | ✅ Complete | Exact statutory text |
| Lead-Based Paint (pre-1978) | ✅ Complete | Conditional on year built |
| Security Deposit (§ 83.49) | ✅ Complete | Bank details, method, statutory rights |
| HB 615 Email Consent | ✅ Complete | Both checkboxes blank—tenant must choose |
| Flood Disclosure (§ 83.512) | ✅ Complete | Tristate wizard, scrivener compliant |
| Scrivener Doctrine | ✅ Followed | No "we recommend" language |

**Gaps to Fill:** ✅ ALL COMPLETE

| Gap | Research Source | Priority | Status |
|-----|-----------------|----------|--------|
| **HOA/Condo Association Addendum** | FL_LEASE.md §3.1 | P0 | ✅ Addendum I - Association Supremacy, indemnity, approval contingency |
| **CDD Disclosure (§ 190.048)** | FL_LEASE.md, FL_LIST.md | P0 | ✅ Addendum J - Boldfaced warning, assessment amounts |
| **Liquidated Damages (§ 83.595)** | FL_LEASE.md §6.2 | P1 | ✅ Addendum K - Safe harbor, max 2 months, separate signature |
| **30-Day Notice Explicit Reference** | FL_LEASE.md (HB 1417) | P1 | ✅ Section 2.7 - Explicit § 83.57(3) citation |
| **Jury Trial Waiver** | FL_LEASE.md §6.3 | P2 | ✅ Section 2.16 - Bold, all-caps KNOWINGLY AND VOLUNTARILY |
| **Mold Prevention Addendum** | FL_LEASE.md §6.4 | P2 | ✅ Addendum L - AC operation, humidity <60%, leak reporting |
| **HB 621 Squatter Language** | FL_LEASE.md §6.1 | P2 | ✅ Section 2.17 - Unauthorized occupants = transient/trespassers |
| **Service Member Rights (§ 83.682)** | FL_LEASE.md §3.2 | P2 | ✅ Section 2.18 - 35-mile radius termination right |
| **ESA Fraud Prevention (SB 1084)** | FL_LEASE.md §6.5 | P3 | ✅ Section 2.19 - § 817.265 citation, personal knowledge req |
| **Standalone Flood Disclosure** | FL_LEASE.md §1.3 | P2 | ✅ `florida_flood_disclosure.typ` - SB 948/HB 1015 compliant |

### New Templates Required

#### `florida_purchase_as_is.typ` - Structure

Based on FL_PURCHASE.md research, modeled on FAR/BAR "As-Is" Residential Contract:

```
SECTIONS:
1. PARTIES AND PROPERTY
   - Buyer/Seller identification
   - Property legal description (not just address)
   - Parcel ID requirement

2. PURCHASE PRICE AND DEPOSITS
   - Initial deposit, additional deposit
   - Escrow agent details

3. FINANCING (Conditional)
   - Cash, Conventional, FHA, VA options
   - Appraisal contingency
   - Appraisal Gap Clause (configurable cap)

4. INSPECTION PERIOD (Key "As-Is" Feature)
   - Sole discretion termination right
   - Default 15 days (negotiable to 7-10 in competitive markets)
   - No repair obligation language

5. TITLE AND SURVEY
   - Title insurance commitment
   - Survey requirements
   - Marketable title definition

6. CLOSING
   - Closing date
   - Prorations (taxes, HOA, etc.)
   - Closing costs allocation

7. DISCLOSURES (Mandatory Addenda)
   - Flood Disclosure (§ 689.302)
   - Foreign Ownership Affidavit (SB 264)
   - Lead-Based Paint (pre-1978)
   - Radon Gas (§ 404.056)

CONDITIONAL ADDENDA:
A. Condo Rider (if condo)
   - SIRS/Milestone Inspection acknowledgment
   - 7-day document review period
   - Association approval contingency

B. HOA Rider (if in HOA)
   - HOA disclosure summary (§ 720.401)
   - Assessment disclosure

C. CDD Rider (if in CDD)
   - § 190.048 boldfaced disclosure

D. SB 264 Critical Infrastructure Warning (if near military base)
   - MacDill, NAS Jax, etc.
   - Foreign principal prohibition notice

E. Appraisal Gap Guarantee (optional)
   - Configurable cap amount

F. Kick-Out Clause / Rider X (optional)
   - For home-sale contingencies
   - 24-72 hour response window

G. Post-Closing Occupancy Agreement (optional)
   - Seller as "licensee" not "tenant"
   - Holdback escrow provision
```

#### `florida_listing.typ` - Structure

Based on FL_LIST.md research, Ch. 475 compliant:

```
SECTIONS:
1. BROKER AND SELLER INFORMATION
   - License numbers
   - Brokerage details

2. PROPERTY INFORMATION
   - Legal description (not just address)
   - Parcel ID
   - Year built (Lead Paint trigger)

3. LISTING TERMS (Ch. 475 "Four Pillars")
   - Definite expiration date (hard-coded, no auto-renewal)
   - Property description
   - Price and terms
   - Commission structure

4. COMPENSATION (NAR Settlement Compliant)
   - Listing broker fee (explicit, separate)
   - Buyer concession authorization (optional, separate section)
   - Fee negotiability disclosure (bold, initialed)
   - NO aggregated commission language

5. SELLER OBLIGATIONS
   - Access for showings
   - Disclosure duties
   - Cooperation requirements

6. BROKER OBLIGATIONS
   - Marketing commitments
   - MLS participation
   - Fiduciary duties

7. TERMINATION
   - Protection period clause
   - Liquidated damages (not penalty)
   - Procuring cause protection

8. SIGNATURES
   - 24-hour delivery acknowledgment
   - Electronic consent (Ch. 668)

MANDATORY DISCLOSURES (Pre-filled at Listing):
A. Flood Disclosure Questionnaire (§ 689.302)
   - Completed by seller at listing time
   - Ready for buyer at contract

B. Radon Gas Notification (§ 404.056)
   - Statutory text

CONDITIONAL ADDENDA:
C. Lead-Based Paint (pre-1978)
   - Federal requirement

D. CDD Disclosure (if applicable)
   - § 190.048 boldfaced text

E. HOA Disclosure Summary (§ 720.401)
   - Voidability warning if not provided

F. Condo Safety Rider (if condo)
   - SIRS/Milestone status
   - 7-day review period disclosure

G. Coastal Property Rider (if seaward of CCCL)
   - § 161.57 disclosure

H. Scrub Jay/HCP Disclosure (if in habitat zone)
   - Charlotte, Sarasota, Brevard, Palm Bay

I. SB 264 Foreign Interest Notice
   - Warning for properties near critical infrastructure
```

---

## Four Demo Structure

### Demo 1: High-Level Overview (5-7 min)
**"The Florida Real Estate Compliance Shield"**

```
FLOW:
1. Open agentPDF.org → Show three contract types available
   - "Lease, Purchase, Listing—all Florida-compliant"

2. Metro detection demo
   - Enter zip 33629 (South Tampa)
   - System detects: "Tampa Bay Metro, Hillsborough County"
   - Warning appears: "Property within 10 miles of MacDill AFB - SB 264 restrictions apply"

3. Quick generation of each type (30 seconds each)
   - Generate a lease → Show flood disclosure included
   - Generate a purchase contract → Show As-Is inspection clause
   - Generate a listing → Show NAR-compliant commission structure

4. Cross-site handoff demonstration
   - Click "Send for Signature"
   - Seamless redirect to getsignatures.org
   - Document pre-loaded, ready to sign

5. Show "Coming Soon: 16 States" selector
   - FL highlighted as available
   - TX, CA, NY, GA, IL, etc. shown with lock icons
   - "Q2 2026" tooltip on hover
```

### Demo 2: Lease Deep Dive (10-15 min)
**"The Chapter 83 Compliance Engine"**

```
FLOW:
1. Start new lease template
   - Enter property address in Tampa
   - System auto-detects metro, shows applicable disclosures

2. Flood Disclosure Wizard walkthrough (§ 83.512)
   - Step through tristate questions
   - Show "I don't know" as neutral default
   - Generate compliant disclosure

3. Basic lease terms
   - Rent, deposit, dates
   - Show security deposit bank disclosure auto-generated

4. Conditional addenda demonstration
   - Toggle "Property in HOA" → HOA Addendum appears
   - Toggle "Property in CDD" → CDD Disclosure appears
   - Toggle "Military tenant" → § 83.682 rights notice appears

5. HB 615 Email Consent ceremony
   - Show both checkboxes blank
   - Explain tenant must actively choose during signing
   - "This saves landlords $800/month in certified mail"

6. Generate PDF → Send to GetSignatures
   - Show complete document with all addenda
   - Demonstrate signature flow
```

### Demo 3: Purchase Contract Deep Dive (10-15 min)
**"The Tiered Contractual Defense Protocol"**

```
FLOW:
1. Explain As-Is vs Standard contract selection
   - "As-Is gives buyer sole discretion exit during inspection"
   - "Standard locks buyer in once seller agrees to repairs"
   - Show why As-Is is recommended for buyers

2. Property identification
   - Enter address near MacDill AFB
   - System triggers SB 264 warning
   - "This property is within Critical Infrastructure Zone"
   - Foreign Ownership Affidavit requirement shown

3. Flood Disclosure integration
   - Same wizard as lease
   - "Disclosure must be ready at contract time"

4. Condo scenario walkthrough
   - Select "Property is a condominium"
   - SIRS/Milestone Inspection disclosure appears
   - Explain 7-day document review period
   - "Buyer can void until closing if paperwork is flawed"

5. Financing contingencies
   - Show Appraisal Gap clause configuration
   - "Pay up to $10,000 over appraisal, exit if gap exceeds"
   - Demonstrate Kick-Out clause for contingent offers

6. Generate and explain signature blocks
   - Multiple signature points for different addenda
   - Clear separation of disclosures
```

### Demo 4: Listing Agreement Deep Dive (10-15 min)
**"The NAR Settlement-Ready Listing System"**

```
FLOW:
1. Chapter 475 "Four Pillars" validation
   - Definite expiration date (no auto-renewal allowed by law)
   - Legal description requirement
   - Price and terms
   - Commission structure

2. NAR Settlement compliance demonstration
   - Show decoupled commission structure
   - "Listing broker fee: X%" (explicit, separate)
   - "Buyer concession authorization" (optional, separate section)
   - Fee negotiability disclosure with required initial box
   - "No more aggregated 6% language"

3. Pre-listing disclosure collection
   - Flood disclosure completed by seller NOW
   - "Ready to provide to buyers immediately"
   - Explain voidability risk if disclosures missing

4. Property-specific triggers
   - Enter pre-1978 property → Lead Paint Rider attached
   - Enter CDD property → § 190.048 disclosure attached
   - Enter coastal property → CCCL Rider attached

5. Protection period and termination
   - Explain procuring cause protection
   - Show liquidated damages vs penalty distinction
   - "Exception voids if seller relists with another broker"

6. Electronic consent and 24-hour delivery
   - Ch. 668 compliance
   - Audit trail generation
   - "Proof of delivery protects your license"
```

---

**Continue to [PLAN1.md](./PLAN1.md) for Implementation Roadmap and Phases 4-6.**
