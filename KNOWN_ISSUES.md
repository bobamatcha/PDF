# Known Issues

This document tracks known issues that need investigation or resolution.

---

## ISSUE-001: PDF Flatten Export Not Working in WASM

**Status:** Open
**Severity:** Medium
**Component:** pdfjoin-web / pdfjoin-wasm
**Date Identified:** 2024-12-25

### Description

The `apply_operations_flattened()` function, which burns PDF edits directly into page content streams (making them permanent/non-editable), works correctly in Rust unit tests but fails when called through the WASM binding in the browser.

When `exportFlattened()` is called from JavaScript:
- The function returns successfully (no error)
- But the returned PDF is unchanged (same size as original, no operations applied)
- Operations exist in the session (`sessionOpCount: 2`, `sessionHasChanges: true`)

### Current Workaround

The export path was reverted to use the annotation-based `export()` function, which creates PDF annotations (Square, FreeText, etc.) that remain editable in PDF editors.

### Technical Details

**Files involved:**
- `crates/pdfjoin-core/src/apply_operations.rs` - `apply_operations_flattened()` function
- `apps/pdfjoin-web/wasm/src/edit_session.rs` - `export_flattened()` WASM binding
- `apps/pdfjoin-web/src/ts/edit.ts` - JavaScript export calls

**Unit tests that PASS (proving Rust logic works):**
- `test_flattened_output_has_no_annotations`
- `test_flattened_text_is_in_content_stream`
- `test_flattened_whiteout_produces_white_color`
- `test_flattened_with_multipage_pdf`

**Browser test that FAILS:**
- `test_pdfjoin_whiteout_text_appears_in_exported_pdf` (when using flattened export)

### Suspected Causes

1. **lopdf behavior in WASM** - `get_pages()` or `set_object()` may behave differently in WASM vs native
2. **Object serialization** - The OperationLog may not serialize correctly across the WASM boundary
3. **Page numbering mismatch** - Possible off-by-one or indexing issue specific to WASM context

### Acceptance Criteria for Resolution

- [ ] `exportFlattened()` called from browser JavaScript returns a PDF with operations applied
- [ ] Browser test `test_pdfjoin_whiteout_text_appears_in_exported_pdf` passes with flattened export
- [ ] Exported PDF contains content stream operators (e.g., `1 1 1 rg` for white rectangles)
- [ ] Exported PDF does NOT contain annotation objects (`/Square`, `/FreeText`, etc.)
- [ ] File size of exported PDF is larger than original (content was added)
- [ ] Visual verification: whiteout rectangles and text appear correctly in exported PDF

### Investigation Steps

1. Add `web_sys::console::log` debugging to `apply_operations_flattened` in WASM context
2. Check what `doc.get_pages()` returns in WASM vs native for the same PDF
3. Verify `operations_for_page()` finds operations in WASM context
4. Check if `doc.set_object()` actually modifies the document in WASM
5. Test with a minimal PDF to isolate the issue

### Related Files

- Test: `crates/benchmark-harness/tests/browser_pdfjoin.rs:5043` - `test_pdfjoin_whiteout_text_appears_in_exported_pdf`
- Unit tests: `crates/pdfjoin-core/src/apply_operations.rs` (search for `test_flattened`)
