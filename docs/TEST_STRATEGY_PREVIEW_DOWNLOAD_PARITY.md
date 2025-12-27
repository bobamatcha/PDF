# Test Strategy: Preview/Download Parity

This document outlines the testing strategy to ensure what users see in the preview exactly matches what gets downloaded.

## Problem Statement

The preview (DOM rendering) and download (PDF export) use different code paths:
- **Preview**: DOM elements rendered by CSS/browser
- **Download**: PDF annotations or flattened content via `apply_operations()`

Any mismatch causes user confusion and data loss.

## Critical Test Scenarios

### 1. Text Box Operations

| Scenario | Test Status | Test Name |
|----------|-------------|-----------|
| Create text box with text | ✅ Covered | `test_pdfjoin_textbox_ui_edit_persisted_to_export` |
| Edit existing text box | ✅ Covered | `test_pdfjoin_textbox_edit_persisted_to_export` |
| Empty text box (no text) | ⚠️ Should verify no operation | TODO |
| Text with special chars (parentheses, backslash) | ⚠️ Unit test only | `escape_pdf_string` tests |
| Bold/italic text styling | ⚠️ Partial coverage | Need visual verification |
| Font size changes | ⚠️ Partial coverage | `test_pdfjoin_font_size_preservation` |
| Text box resize then edit | ❌ Not covered | TODO |
| Delete text box | ❌ Not covered | TODO |

### 2. Whiteout Operations

| Scenario | Test Status | Test Name |
|----------|-------------|-----------|
| Whiteout without text | ⚠️ Partial | TODO: verify export |
| Whiteout with text | ✅ Covered | `test_pdfjoin_whiteout_text_appears_in_exported_pdf` |
| Edit whiteout text | ❌ Not covered | TODO |
| Blackout mode | ❌ Not covered | TODO |
| Whiteout resize | ❌ Not covered | TODO |

### 3. Other Annotations

| Scenario | Test Status | Test Name |
|----------|-------------|-----------|
| Highlight annotation | ⚠️ Partial | Need export verification |
| Underline annotation | ⚠️ Partial | Need export verification |
| Checkbox annotation | ⚠️ Partial | Need export verification |
| Replace text operation | ⚠️ Partial | `test_pdfjoin_export_preserves_font_in_pdf` |

### 4. Edge Cases

| Scenario | Test Status | Priority |
|----------|-------------|----------|
| Multi-page operations | ❌ Not covered | HIGH |
| Page navigation then edit | ❌ Not covered | HIGH |
| Undo/redo then export | ❌ Not covered | MEDIUM |
| Operations on signed PDF | ⚠️ Partial (rejection only) | LOW |
| Very large text content | ❌ Not covered | LOW |
| Unicode/emoji in text | ❌ Not covered | MEDIUM |

## Recommended Test Additions

### Priority 1: Critical Coverage Gaps

```rust
// In browser_pdfjoin.rs

/// Test multi-page: add text to page 2, verify export
#[tokio::test]
async fn test_pdfjoin_multipage_text_persisted() { ... }

/// Test page navigation: add text on page 1, navigate to page 2 and back, verify export
#[tokio::test]
async fn test_pdfjoin_page_navigation_preserves_operations() { ... }

/// Test edit then immediate download (race condition check)
#[tokio::test]
async fn test_pdfjoin_edit_then_immediate_download() { ... }
```

### Priority 2: Undo/Redo Integration

```rust
/// Test undo text addition then export (should have no text)
#[tokio::test]
async fn test_pdfjoin_undo_removes_from_export() { ... }

/// Test redo text addition then export (should have text)
#[tokio::test]
async fn test_pdfjoin_redo_restores_to_export() { ... }
```

### Priority 3: Visual Verification (Optional)

For font/styling verification, consider screenshot comparison:
1. Render exported PDF back via PDF.js
2. Compare specific region with original preview
3. Flag if visual difference exceeds threshold

## Test Pattern: Preview/Download Parity

All parity tests should follow this pattern:

```javascript
// 1. Create operation via UI
// 2. Capture what's visible in DOM (preview)
// 3. Export via editSession.export()
// 4. Verify exported PDF contains the same content

async function testPreviewDownloadParity(operation) {
    // Setup
    await loadPdf();

    // Create operation
    await createOperation(operation);

    // Capture preview state
    const previewState = capturePreviewState();

    // Export
    const exported = editSession.export();

    // Verify parity
    assertParity(previewState, exported);
}
```

## Key Invariants to Test

1. **Text Content Invariant**: If DOM shows text T at position P, exported PDF must render T at P
2. **Operation Count Invariant**: `editSession.getOperationCount()` must equal number of visible annotations
3. **Style Invariant**: Font size, color, bold/italic must match between preview and export
4. **Position Invariant**: DOM coordinates converted to PDF coordinates must be consistent

## Monitoring

Add console warnings in production for potential parity issues:

```typescript
// In downloadEditedPdf()
const opCount = editSession.getOperationCount();
const domAnnotations = document.querySelectorAll('.text-box, .edit-whiteout-overlay').length;
if (opCount !== domAnnotations) {
    console.warn(`Parity warning: ${opCount} operations vs ${domAnnotations} DOM elements`);
}
```

## Running Tests

```bash
# Run all parity-related tests
cargo test -p benchmark-harness --test browser_pdfjoin -- --nocapture 2>&1 | grep -E "(persist|parity|export)"

# Run specific test
cargo test -p benchmark-harness --test browser_pdfjoin test_pdfjoin_textbox_ui_edit_persisted_to_export -- --nocapture
```
