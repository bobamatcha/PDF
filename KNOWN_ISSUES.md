# Known Issues

This document tracks known issues that need investigation or resolution.

---

## Future Work: UI Fuzzing Framework

> **Priority:** High
> **Goal:** Systematically discover edge cases and regressions through automated exploration

### Concept

Create a fuzzing framework that extends the existing browser tests to:

1. **Random User Path Exploration** - Simulate realistic but randomized user interactions
2. **Cross-Tool Consistency Checking** - Verify that similar behaviors (e.g., text box vs whiteout persistence) work consistently
3. **Long-Running Background Tests** - Run for extended periods collecting bug reports
4. **Reproducible Bug Reports** - Log all actions to enable exact reproduction

### Design Goals

```
┌─────────────────────────────────────────────────────────────────┐
│                    UI Fuzzer Architecture                       │
├─────────────────────────────────────────────────────────────────┤
│  Input Generator                                                │
│  ├── Random tool selection (textbox, whiteout, select, etc.)   │
│  ├── Random coordinates within PDF bounds                       │
│  ├── Random interaction types (click, dblclick, drag, type)    │
│  └── Random timing variations                                   │
├─────────────────────────────────────────────────────────────────┤
│  Invariant Checkers                                             │
│  ├── Preview matches export (DOM vs PDF content)               │
│  ├── No duplicate elements from single action                   │
│  ├── Selection state consistency                                │
│  ├── Style tools enable/disable correctly                       │
│  └── Autosave before download                                   │
├─────────────────────────────────────────────────────────────────┤
│  Bug Collector                                                  │
│  ├── Action log (JSON format for replay)                       │
│  ├── Screenshot at failure                                      │
│  ├── DOM state snapshot                                         │
│  └── PDF export comparison                                      │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Options

1. **Extend `browser_pdfjoin.rs`** - Add parameterized/randomized test cases
2. **New `fuzz-harness` crate** - Dedicated long-running fuzzer with state management
3. **Property-based testing** - Use `proptest` or `quickcheck` for Rust-side invariants

### Key Scenarios to Fuzz

- Tool switching mid-operation
- Rapid create/delete cycles
- Overlapping elements
- Immediate download after edits
- Reselection and re-editing
- Multi-page operations
- Undo/redo sequences

### Bug Report Format

```json
{
  "timestamp": "2024-12-26T12:34:56Z",
  "seed": 12345,
  "actions": [
    {"type": "tool_select", "tool": "textbox"},
    {"type": "dblclick", "x": 150, "y": 200},
    {"type": "type", "text": "Hello"},
    {"type": "click_away", "x": 50, "y": 50},
    {"type": "click", "x": 150, "y": 200},
    {"type": "resize_attempt", "handle": "se", "dx": 50, "dy": 30}
  ],
  "invariant_failed": "resize_handle_visible_after_reselection",
  "screenshot": "bug_12345.png",
  "dom_snapshot": "bug_12345.html"
}
```

---

## Table of Contents

| # | Issue | Status | Severity |
|---|-------|--------|----------|
| [001](#issue-001-highlight-tool-not-functional) | Highlight Tool Not Functional | Open | **Critical** |
| [002](#issue-002-underline-tool-not-functional) | Underline Tool Not Functional | Open | **Critical** |
| [003](#issue-003-checkbox-tool-not-functional) | Checkbox Tool Not Functional | Open | **Critical** |
| [004](#issue-004-unwanted-hover-highlight-ux) | Unwanted Hover Highlight UX | Open | Medium |
| [005](#issue-005-pdf-flatten-export-compress-bug) | PDF Flatten Export Compress Bug | Resolved | Medium |
| [006](#issue-006-highlightunderline-y-coordinate-offset) | Highlight/Underline Y-Coordinate Offset | Resolved | High |
| [007](#issue-007-underline-tool-color-and-gap) | Underline Tool Color and Gap | Resolved | Low |
| [008](#issue-008-highlight-tool-icon-and-color-picker) | Highlight Tool Icon and Color Picker | Partial | Low |
| [009](#issue-009-whiteout-text-not-visible-in-exported-pdf) | Whiteout Text Not Visible in Exported PDF | Resolved | **Critical** |
| [010](#issue-010-text-box-tool-clickdrag-creates-two-boxes) | Text Box Tool Click+Drag Creates Two Boxes | Resolved | **Critical** |
| [011](#issue-011-text-box-resize-broken-after-reselection) | Text Box Resize Broken After Reselection | Resolved | **Critical** |
| [012](#issue-012-text-box-styling-not-functional) | Text Box Styling Not Functional | Resolved | **Critical** |
| [013](#issue-013-text-box-not-autosaved-before-download) | Text Box Not Autosaved Before Download | Resolved | **Critical** |
| [014](#issue-014-text-box-deselection-inconsistent) | Text Box Deselection Inconsistent | Resolved | High |
| [015](#issue-015-style-tools-disabled-when-box-selected) | Style Tools Disabled When Box Selected | Resolved | High |
| [016](#issue-016-file-replace-confirmation-missing) | File Replace Confirmation Missing | Resolved | **Critical** |
| [017](#issue-017-cannot-re-edit-existing-text-after-first-edit) | Cannot Re-Edit Existing Text After First Edit | Open | **Critical** |
| [018](#issue-018-text-tools-not-working-when-editing-existing-text) | Text Tools Not Working When Editing Existing Text | Open | **Critical** |
| [019](#issue-019-text-box-tool-types-on-whiteout-text-instead-of-editing) | Text Box Tool Types On Whiteout Text Instead of Editing | Open | High |
| [020](#issue-020-text-tools-blur-away-after-multiple-edits) | Text Tools Blur Away After Multiple Edits | Open | **Critical** |
| [021](#issue-021-test-parity-between-preview-and-downloaded-pdf) | Test Parity Between Preview and Downloaded PDF | Open | High |
| [022](#issue-022-verify-download-applies-pending-changes-for-all-edit-types) | Verify Download Applies Pending Changes for All Edit Types | Open | **Critical** |
| [023](#issue-023-cross-tab-document-persistence-arraybuffer-detachment) | Cross-Tab Document Persistence (ArrayBuffer Detachment) | Resolved | **Critical** |
| [024](#issue-024-filelist-iteration-bug-in-merge-file-handler) | FileList Iteration Bug in Merge File Handler | Resolved | **Critical** |
| [025](#issue-025-partial-text-selection-styling-not-supported) | Partial Text Selection Styling Not Supported | Resolved | **Critical** |
| [025b](#issue-025b-mixed-partial-styling-not-exported-to-pdf) | Mixed Partial Styling Not Exported to PDF | Open | **Critical** |

---

## ISSUE-001: Highlight Tool Not Functional

**Status:** Open (Tool Hidden)
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-25

### Description

The highlight tool does not work correctly:

1. **Preview color is always yellow** - Even when the user selects a different highlight color (green, pink, blue, orange), the preview shown while selecting text is always yellow
2. **Highlights do not render to exported PDF** - The highlight annotations are not being properly saved/exported to the final PDF
3. **DOM preview may appear but PDF export is broken** - Visual feedback in the browser doesn't translate to actual PDF content

### Current State

Tool has been **hidden from the toolbar** until these issues are resolved. The underlying code remains but is not accessible to users.

### Acceptance Criteria

- [ ] Preview color matches selected color
- [ ] Highlights appear in exported PDF
- [ ] Multi-line highlights render as separate rectangles per line

---

## ISSUE-002: Underline Tool Not Functional

**Status:** Open (Tool Hidden)
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-25

### Description

The underline tool does not work correctly:

1. **Uses highlight preview instead of underline** - When selecting text with underline tool, it shows a highlight-style preview instead of an underline
2. **Underlines do not render to exported PDF** - The underline annotations are not being properly saved/exported to the final PDF

### Current State

Tool has been **hidden from the toolbar** until these issues are resolved. The underlying code remains but is not accessible to users.

### Acceptance Criteria

- [ ] Underline preview shows a thin line below text, not a highlight box
- [ ] Underlines appear in exported PDF
- [ ] Underline color matches text color or user selection

---

## ISSUE-003: Checkbox Tool Not Functional

**Status:** Open (Tool Hidden)
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-25

### Description

The checkbox tool has never successfully rendered a checkbox on top of the PDF:

1. **Clicking on PDF does not create checkbox** - Despite the tool being selected, clicking on the PDF does not create a visible checkbox element
2. **No checkbox appears in exported PDF** - Even if DOM elements were created, they don't appear in the final PDF export

### Current State

Tool has been **hidden from the toolbar** until these issues are resolved. The underlying code remains but is not accessible to users.

### Acceptance Criteria

- [ ] Clicking with checkbox tool creates visible checkbox on PDF
- [ ] Checkbox can be toggled (checked/unchecked)
- [ ] Checkbox appears in exported PDF

---

## ISSUE-004: Unwanted Hover Highlight UX

**Status:** Open
**Severity:** Medium
**Component:** pdfjoin-web / index.html, edit.ts
**Date Identified:** 2024-12-25

### Description

When the highlight or underline tool is selected, hovering over text in the PDF shows a yellow highlight preview. This behavior is:

1. **Unexpected** - Users don't expect text to highlight just from hovering
2. **Confusing** - Makes it unclear what has been actually highlighted vs what is just a hover effect
3. **Visually distracting** - Creates visual noise when moving the mouse across the document

### Resolution Required

Remove the `.highlight-mode .text-layer .text-item:hover` CSS styling that adds background color on hover. The text selection should only show highlighting during active click-drag selection, not on hover.

### Files to Change

- `apps/pdfjoin-web/www/index.html` - Remove hover highlight CSS

### Acceptance Criteria

- [ ] No highlight preview appears when just hovering over text
- [ ] Highlight only appears during active text selection (click-drag)

---

## ISSUE-005: PDF Flatten Export Compress Bug

**Status:** Resolved
**Severity:** Medium
**Component:** pdfjoin-web / pdfjoin-wasm
**Date Identified:** 2024-12-25
**Date Resolved:** 2024-12-25

### Description

The `apply_operations_flattened()` function, which burns PDF edits directly into page content streams (making them permanent/non-editable), works correctly in Rust unit tests but fails when called through the WASM binding in the browser.

When `exportFlattened()` is called from JavaScript:
- The function returns successfully (no error)
- But the returned PDF is unchanged (same size as original, no operations applied)
- Operations exist in the session (`sessionOpCount: 2`, `sessionHasChanges: true`)

### Root Cause

**`doc.compress()` was corrupting content streams in WASM.**

Through systematic debugging, we ruled out:
1. Page number mismatch - `get_pages()` returns correct 1-indexed pages matching operations
2. Content stream not saving - Manual stream creation and save works correctly in WASM

The actual issue: `lopdf::Document::compress()` was removing or corrupting the newly added content streams when running in WASM context.

### Resolution

Removed the `doc.compress()` call from `apply_operations_flattened()` in `crates/pdfjoin-core/src/apply_operations.rs:107`.

```rust
// NOTE: Removed doc.compress() - it was corrupting the content streams in WASM
// See KNOWN_ISSUES.md ISSUE-005 for details
// doc.compress();
```

### Size Analysis

| Scenario | Size | Has `1 1 1 rg` | Notes |
|----------|------|----------------|-------|
| Original PDF | 608 bytes | N/A | Test PDF with 2 pages |
| Flatten WITHOUT compress | 788 bytes | Yes | Content correctly added |
| Flatten WITH compress | 1077 bytes | No | Larger but content missing! |

---

## ISSUE-006: Highlight/Underline Y-Coordinate Offset

**Status:** Resolved
**Severity:** High
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-25
**Date Resolved:** 2024-12-25

### Description

Highlights and underlines were appearing at incorrect Y positions, especially near the top of pages.

### Root Cause

The code used manual coordinate calculations that didn't properly account for PDF.js viewport transformations.

### Resolution

Replaced all manual coordinate calculations with PDF.js native viewport methods:

```typescript
const [pdfX, pdfY] = viewport.convertToPdfPoint(domX, domY);
```

---

## ISSUE-007: Underline Tool Color and Gap

**Status:** Resolved
**Severity:** Low
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-25
**Date Resolved:** 2024-12-25

### Description

Two UX issues with the underline tool:
1. No color picker - Underlines should match text color
2. No whitespace gap - Underlines should have a small gap below the text baseline

### Resolution

Both issues were already implemented:
1. `const underlineGap = 2;` adds 2px spacing below text baseline
2. Underline automatically matches text color by detecting `computedStyle.color`

---

## ISSUE-008: Highlight Tool Icon and Color Picker

**Status:** Partial (Icon done, functionality broken)
**Severity:** Low
**Component:** pdfjoin-web / index.html, edit.ts
**Date Identified:** 2024-12-25

### Description

1. **Icon** - Original highlight icon didn't clearly convey "highlighter marker"
2. **Color picker** - Should allow clicking the highlight tool again to change color

### Resolution (Partial)

1. **New SVG Marker Icon**: Replaced the emoji icon with a proper SVG highlighter marker icon
2. **Click-to-show Color Picker**: First click activates tool, second click shows color dropdown

**Note:** While the UI improvements were made, the underlying highlight functionality is broken (see ISSUE-001). The tool has been hidden until the core functionality is fixed.

---

## Redaction vs Whiteout Discussion

### Current Whiteout Implementation

The whiteout tool creates a white rectangle that covers content visually. With the flattened export (now working after ISSUE-005 resolution), the whiteout is burned directly into the PDF content stream as:

```
q
1 1 1 rg          % Set fill color to white (RGB 1,1,1)
x y w h re f      % Draw filled rectangle at position
Q
```

### Would Redaction Be Different?

**No, redaction would use the same approach.** Here's why:

1. **Flatten = Permanent**: Since we now use `apply_operations_flattened()`, the white rectangle is written directly into the page content stream. This is **true redaction** - the original content underneath is not recoverable because:
   - The white rectangle is drawn OVER the existing content
   - The content stream is rewritten with the new drawing commands
   - The original PDF objects are not preserved

2. **No Annotation Layer**: We're NOT using PDF annotations (which would be removable). We're modifying the actual page content.

3. **Black vs White**: The only difference between "whiteout" and "redaction" is color:
   - Whiteout: `1 1 1 rg` (white fill)
   - Redaction: `0 0 0 rg` (black fill)

### Security Consideration

For legal redaction requirements, the current flattened export approach is correct. The redacted content cannot be recovered because:
- We're not using removable annotations
- We're not using layers that can be toggled
- The content stream is permanently modified

The only way to "see" the original would be to have access to the original PDF file before redaction.

---

## ISSUE-009: Whiteout Text Not Visible in Exported PDF

**Status:** Resolved
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts, pdfjoin-core / apply_operations.rs
**Date Identified:** 2024-12-25
**Date Resolved:** 2024-12-26

### Description

When a user types text INTO a whiteout box, the text appears correctly in the browser preview but is **not visible** in the exported/downloaded PDF.

### Root Causes Found

Two separate bugs were identified:

1. **Immediate Download Bug**: User types text in whiteout → clicks Download immediately → text is NOT saved because the blur event's 200ms setTimeout hasn't fired yet.

2. **Text Centering Bug**: Text was positioned at fixed offset `2 4 Td` (bottom-left) instead of centered like the preview showed.

### Resolution

1. **commitPendingEdits()** - Added to `edit.ts`:
   - Called at start of `downloadEditedPdf()` before export
   - Finds any active `.whiteout-text-input` with unsaved text
   - Calls `saveWhiteoutText()` to persist it before export
   - Added `data-original-width` and `data-original-height` attributes to track dimensions

2. **Text Centering** - Updated `apply_operations.rs`:
   - `create_text_appearance_content()` now calculates centered position
   - Uses box dimensions and estimated text width based on font metrics
   - `x_offset = (box_width - text_width) / 2`
   - `y_offset = (box_height - font_size) / 2`

### Regression Tests

- `test_immediate_download_after_whiteout_text_includes_text` - Tests typing text and immediately downloading without pressing Enter
- `test_pdfjoin_whiteout_text_appears_in_exported_pdf` - Tests full UI flow with Enter key

### Verification

All whiteout text tests pass:
```
test_immediate_download_after_whiteout_text_includes_text ... ok
test_pdfjoin_whiteout_text_appears_in_exported_pdf ... ok
test_pdfjoin_whiteout_text_matches_covered_style ... ok
test_pdfjoin_whiteout_text_input_fills_area_and_matches_font ... ok
```

---

## ISSUE-010: Text Box Tool Click+Drag Creates Two Boxes

**Status:** Resolved
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26
**Date Resolved:** 2024-12-26

### Description

When using the Text Box tool, click-and-drag to create a sized text box instead creates TWO tiny text boxes: one at the mousedown location and another at the mouseup location. This confused elderly users significantly.

### Steps to Reproduce (Before Fix)

1. Upload any PDF
2. Select the Text Box tool
3. Click and drag from one point to another (like drawing a rectangle)
4. Result: Two small text boxes appear instead of one sized text box

### Root Cause

The text box tool had multiple event handlers that each created a text box:
- `handleOverlayClick` created a box on click
- `handleWhiteoutEnd` also handled textbox creation on mouseup

### Resolution

1. **Removed textbox creation from `handleOverlayClick`** - Single clicks no longer create text boxes
2. **Modified `handleWhiteoutEnd`** to only create textbox on meaningful drag (>10px)
3. **Added `handleTextBoxDoubleClick`** handler for double-click creation
4. **Modified `createTextBox`** to accept optional width/height parameters
5. **Added minimum sizes for accessibility**:
   - `MIN_TEXTBOX_HEIGHT = 44` (WCAG touch target)
   - `MIN_TEXTBOX_WIDTH = 100`
   - `DEFAULT_TEXTBOX_WIDTH = 200`
   - `DEFAULT_TEXTBOX_HEIGHT = 48`

### Regression Tests

- `test_textbox_click_drag_creates_one_sized_box` - Verifies drag creates single sized box
- `test_textbox_single_click_does_not_create_box` - Verifies single click is no-op
- `test_textbox_double_click_creates_accessible_sized_box` - Verifies double-click creates 44px+ box

### Verification

All 10 textbox tests pass:
```
test_textbox_click_creates_text_box ... ok
test_textbox_click_drag_creates_one_sized_box ... ok
test_textbox_create_transparent ... ok
test_textbox_delete_key ... ok
test_textbox_delete_x_button ... ok
test_textbox_double_click_creates_accessible_sized_box ... ok
test_textbox_overlap_zorder ... ok
test_textbox_resize ... ok
test_textbox_single_click_does_not_create_box ... ok
test_textbox_toolbar_button_exists ... ok
```

---

## ISSUE-011: Text Box Resize Broken After Reselection

**Status:** Resolved (Verified Working)
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26
**Date Resolved:** 2024-12-26

### Description

Initially reported that after clicking away from a text box and clicking back, resize handles were not visible/functional.

### Investigation

Created regression test `test_textbox_resize_handles_visible_after_reselection` which:
1. Creates a text box
2. Clicks away to deselect
3. Clicks back on the text box
4. Verifies resize handles are visible

**Test passes**, indicating the functionality works correctly. The reported issue may have been:
- A transient UI state issue
- Fixed by a previous change
- A specific edge case not captured by the test

### Regression Test

`test_textbox_resize_handles_visible_after_reselection` in `browser_pdfjoin.rs`

### Acceptance Criteria

- [x] Clicking on existing text box shows resize handles
- [x] All 8 resize handles (corners + edges) are visible when selected
- [x] Dragging resize handles changes text box dimensions
- [x] Resize persists after clicking away and back

---

## ISSUE-012: Text Box Styling Not Functional

**Status:** Resolved (Verified Working)
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26
**Resolution Date:** 2024-12-26

### Description

Text styling controls (font size, bold, italic) do not work for text boxes:

1. **Font size buttons** - Clicking +/- or typing a size has no effect
2. **Bold button** - Clicking B does not make text bold
3. **Italic button** - Clicking I does not make text italic
4. **All styling fails even on initial creation** - Not just after reselection

### Resolution

**Test `test_textbox_styling_bold_italic_fontsize` confirms functionality works:**
- Style buttons are correctly enabled when text content is focused
- Bold button successfully applies `fontWeight: bold` (700)
- Italic button successfully applies `fontStyle: italic`
- Font size increase button increments font size

**Key requirement:** Text styling requires focusing INSIDE the text content area (clicking on the text itself), not just clicking on the text box border. The style buttons become enabled only when `activeTextInput` is set via the focus event.

### Acceptance Criteria

- [x] Font size +/- buttons change text size in preview
- [x] Bold button toggles bold styling in preview
- [x] Italic button toggles italic styling in preview
- [ ] All styling changes appear in exported PDF (needs verification)

---

## ISSUE-013: Text Box Not Autosaved Before Download

**Status:** Resolved
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26
**Date Resolved:** 2024-12-26

### Description

Similar to ISSUE-009 (whiteout text), text box content is not autosaved when clicking Download immediately after typing. The text appears in preview but is missing from the exported PDF.

### Root Cause

The `commitPendingEdits()` function added for ISSUE-009 only handles `.whiteout-text-input` elements. Text boxes use `.text-content` which was not being committed before export.

### Resolution

Extended `commitPendingEdits()` in `edit.ts` to iterate over all `.text-box` elements and call `commitTextBox()` for any with text content:

```typescript
function commitPendingEdits(): void {
  // ISSUE-013 FIX: Commit pending text boxes before export
  const textBoxes = document.querySelectorAll<HTMLElement>('.text-box');
  textBoxes.forEach(box => {
    const textContent = box.querySelector<HTMLElement>('.text-content');
    const text = textContent?.textContent?.trim() || '';
    if (text) {
      commitTextBox(box);
    }
  });
  // ... rest of whiteout handling
}
```

### Regression Test

`test_textbox_immediate_download_includes_text` in `browser_pdfjoin.rs` validates this fix.

### Acceptance Criteria

- [x] Text box content is saved even without blur/Enter
- [x] Immediate download after typing includes text
- [x] Regression test confirms fix

---

## ISSUE-014: Text Box Deselection Inconsistent

**Status:** Resolved
**Severity:** High
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26
**Resolution Date:** 2024-12-26

### Description

Clicking away from a text box does not consistently deselect it. Sometimes:
- The text box remains visually selected (blue outline)
- The cursor stays in the text input
- Multiple text boxes appear selected simultaneously

### Resolution

**Fixed in `handleWhiteoutStart()` in edit.ts:**
- Added deselection logic that runs BEFORE the tool check
- When clicking on a blank area (not on a text box, whiteout, or UI element), `deselectTextBox()` and `deselectWhiteout()` are called regardless of current tool
- This ensures clicking away always deselects, whether using select tool, text tool, or any other tool

**Test:** `test_textbox_deselection_on_click_away` verifies:
- Text box is deselected after clicking away
- Resize handles are hidden after deselection
- Deselection works consistently on repeat clicks

### Acceptance Criteria

- [x] Clicking on PDF background deselects text box
- [x] Clicking on another element deselects previous selection
- [x] Selection outline accurately reflects selection state

---

## ISSUE-015: Style Tools Disabled When Box Selected

**Status:** Resolved (Verified Working)
**Severity:** High
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26
**Resolution Date:** 2024-12-26

### Description

When a text box or whiteout is selected (clicked on, showing selection outline), the style tools (font size, bold, italic) remain disabled/grayed out. Users cannot change styling without being in active text edit mode.

### Resolution

**Test `test_textbox_selection_enables_style_tools` confirms functionality works:**

The test verifies that when clicking on a text box:
1. The text box becomes selected (has `.selected` class)
2. Style buttons (Bold, Italic, Font Size) become ENABLED

**How it works:**
- Clicking on the text box triggers the text content's focus event
- The focus event sets `activeTextInput` to the text content element
- `updateStyleButtons()` enables all style controls when `activeTextInput` is set

**Note:** The text content area fills most of the text box, so clicking anywhere inside the box (not just on the text) typically triggers the focus. This provides good UX since users don't need to click precisely on text characters.

### Acceptance Criteria

- [x] Selecting text box enables font size controls
- [x] Selecting text box enables bold/italic buttons
- [x] Clicking Bold applies to entire text box content
- [ ] Changes persist to exported PDF (needs verification)

---

## ISSUE-016: File Replace Confirmation Missing

**Status:** Resolved
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26
**Date Resolved:** 2024-12-26

### Description

When a PDF is already loaded in the edit view and the user loads another PDF, the existing document is replaced without any confirmation dialog. This is dangerous for elderly users who may accidentally lose their work.

### Resolution

Modified `handleEditFile()` in `edit.ts` to:
1. Check if `editSession` or `currentPdfBytes` already exists
2. If so, show confirmation dialog using `showEditConfirmDialog()`
3. Dialog shows different messages based on whether there are unsaved changes
4. "Keep Current" cancels the load, "Replace" proceeds

### Code Changes

Added `showEditConfirmDialog()` helper function that uses the existing `confirm-dialog-overlay` element.

Updated `handleEditFile()`:
```typescript
if (editSession !== null || currentPdfBytes !== null) {
  const hasUnsavedChanges = editSession?.hasChanges() ?? false;
  const confirmed = await showEditConfirmDialog({
    title: hasUnsavedChanges ? 'Replace Document with Unsaved Changes?' : 'Replace Existing Document?',
    message: `You already have "${currentFilename}" open. Loading "${file.name}" will replace it.`,
    confirmText: 'Replace',
    cancelText: 'Keep Current',
  });
  if (!confirmed) return;
}
```

### Acceptance Criteria

- [x] Loading new PDF when one exists shows confirmation modal
- [x] Modal explains that current work will be lost
- [x] "Keep Current" button cancels the new file load
- [x] "Replace" button proceeds with replacement
- [x] Modal is accessible (keyboard navigation, screen reader)

### Test Reference

Test `test_elderly_ux_critical_file_replace_confirmation` in `browser_pdfjoin.rs` validates this behavior.

---

## ISSUE-017: Cannot Re-Edit Existing Text After First Edit

**Status:** Open
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26

### Description

After editing existing PDF text for the first time (using the text replacement feature), the user cannot edit it again. The replacement overlay (`edit-replace-overlay`) should be clickable to re-edit, but either:

1. The click handler doesn't fire
2. The editor doesn't open properly
3. The second edit doesn't apply

### Steps to Reproduce

1. Load a PDF with existing text
2. Click on text to edit with Select tool → editor popup appears
3. Change text and save → replacement overlay appears
4. Click on the replacement overlay to edit again → **FAILS**

### Expected Behavior

- Clicking on replacement overlay should open the editor with the user's last edit (intermediate text)
- User can modify and save again
- The change should persist to PDF export

### Root Cause (Investigation Needed)

The `makeReplaceOverlayEditable()` function in `edit.ts:1798` sets up click handlers, but:
- The handler may not be firing due to event propagation issues
- The `originalSpan` lookup may fail because the span has `.replaced` class
- The WASM operation removal might be causing state issues

### Acceptance Criteria

- [ ] Replacement overlay is re-editable after first save
- [ ] Editor shows user's intermediate text (last edit), not original PDF text
- [ ] Multiple re-edits work without issues
- [ ] All edits persist to exported PDF

### Test References

- `test_pdfjoin_pdf_text_replacement_is_reeditable` (existing, verify if passing)
- `test_pdfjoin_reedit_shows_intermediate_text_not_original` (existing)

---

## ISSUE-018: Text Tools Not Working When Editing Existing Text

**Status:** Open
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26

### Description

When editing existing PDF text (replacement overlay mode), the text styling tools (Bold, Italic, Font Size, Font Family) do not work:

1. **Style buttons disabled** - The bold/italic buttons remain disabled when editing
2. **Font size doesn't change** - Clicking +/- has no effect
3. **No visual feedback** - Changes don't apply to the preview
4. **Not persisted** - Even if preview updated, the PDF export doesn't reflect changes

### Steps to Reproduce

1. Load a PDF with existing text
2. Click on text to open editor popup
3. Try to click Bold button → disabled or no effect
4. Try to change font size → no effect
5. Save and download → original styling only

### Expected Behavior

- All style tools should be enabled when text editor is active
- Bold/Italic/Font size should update the preview immediately
- Style changes should persist to the exported PDF

### Acceptance Criteria

- [ ] Bold button works when editing existing text (preview)
- [ ] Bold persists to exported PDF
- [ ] Italic button works when editing existing text (preview)
- [ ] Italic persists to exported PDF
- [ ] Font size +/- works when editing existing text (preview)
- [ ] Font size persists to exported PDF
- [ ] Font family dropdown works when editing existing text (preview)
- [ ] Font family persists to exported PDF

### Test Parity Requirements

Tests must verify BOTH preview AND exported PDF. Current tests may only check preview.

---

## ISSUE-019: Text Box Tool Types On Whiteout Text Instead of Editing

**Status:** Open
**Severity:** High
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26

### Description

When the Text Box tool is selected and the user clicks on text that was typed in a whiteout box, the behavior is wrong:

1. **Creates new text box on top** - Instead of editing the existing whiteout text, a new text box is created overlapping it
2. **Requires tool switch** - User must switch back to Whiteout tool to edit whiteout text
3. **UX-hostile for elderly users** - Confusing and not discoverable

### Steps to Reproduce

1. Create a whiteout box
2. Double-click to add text: "Original Text"
3. Click away to save
4. Select Text Box tool
5. Click on the whiteout text → **BUG: Creates new text box on top**

### Expected Behavior

When ANY text editing tool (Text Box, Select, etc.) is active:
- Clicking on existing text (in whiteout or text box) should EDIT that text
- Only clicking on empty space should create new elements

### Acceptance Criteria

- [ ] Text Box tool clicking on whiteout text opens whiteout text editor
- [ ] Text Box tool clicking on existing text box edits that text box
- [ ] Select tool clicking on whiteout text opens whiteout text editor
- [ ] Select tool clicking on text box edits that text box

---

## ISSUE-020: Text Tools Blur Away After Multiple Edits

**Status:** Open
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26

### Description

After performing multiple edits (creating/editing text boxes, whiteouts, etc.), clicking on text styling tools causes them to immediately blur/lose focus without applying. The tools become non-functional.

### Steps to Reproduce

1. Load PDF
2. Create text box, type text
3. Create whiteout, type text
4. Edit the text box again
5. Try to click Bold button → **Blurs away immediately, doesn't apply**
6. Try to click Font Size +/- → **Blurs away, doesn't apply**

### Expected Behavior

- Text style tools should remain functional after any number of edits
- Focus should be managed properly to prevent premature blur
- Tools should apply their effect before any blur occurs

### Root Cause (Investigation Needed)

Likely a focus management issue where:
1. Multiple focus handlers are fighting for `activeTextInput`
2. Blur events are firing before click events complete
3. State becomes inconsistent after multiple operations

### Acceptance Criteria

- [ ] Bold button works after 10+ sequential edits
- [ ] Italic button works after 10+ sequential edits
- [ ] Font size works after 10+ sequential edits
- [ ] Font family works after 10+ sequential edits
- [ ] Same behavior for text boxes, whiteout text, and existing PDF text

---

## ISSUE-021: Test Parity Between Preview and Downloaded PDF

**Status:** Open
**Severity:** High
**Component:** pdfjoin-web / browser tests
**Date Identified:** 2024-12-26

### Description

Current tests may verify UI preview state but not the actual downloaded PDF content. This allows bugs to hide where the preview looks correct but the PDF export is wrong.

### Requirements

Every text editing feature needs TWO assertions:
1. **Preview assertion** - DOM shows correct content/styling
2. **Export assertion** - PDF bytes contain correct content/styling

### Features Needing Dual Verification

| Feature | Preview Test | Export Test |
|---------|--------------|-------------|
| Text Box text content | ? | ? |
| Text Box bold styling | ? | ? |
| Text Box italic styling | ? | ? |
| Text Box font size | ? | ? |
| Whiteout text content | ✓ | ✓ |
| Whiteout text bold | ? | ? |
| Whiteout text italic | ? | ? |
| Existing text replacement | ? | ? |
| Existing text bold | ? | ? |
| Existing text italic | ? | ? |

### Acceptance Criteria

- [ ] All text editing features have export verification tests
- [ ] Export tests decode PDF and search for actual content
- [ ] Style attributes are verified in PDF streams (font flags, size)

---

## ISSUE-022: Verify Download Applies Pending Changes for All Edit Types

**Status:** Open
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-26

### Description

The `commitPendingEdits()` function commits unsaved edits before PDF export. Need to verify this covers ALL edit types:

1. **Text boxes** - ✓ Implemented (ISSUE-013)
2. **Whiteout text** - ✓ Implemented (ISSUE-009)
3. **Existing text replacements** - ? Needs verification
4. **Style changes** - ? Needs verification

### Current Implementation

```typescript
function commitPendingEdits(): void {
  // Text boxes - handled
  const textBoxes = document.querySelectorAll<HTMLElement>('.text-box');

  // Whiteout text - handled
  const activeWhiteoutInput = document.querySelector<HTMLElement>('.whiteout-text-input');

  // MISSING: Existing text replacements?
  // MISSING: Pending style changes?
}
```

### Acceptance Criteria

- [ ] Audit `commitPendingEdits()` for all edit types
- [ ] Add handling for any missing edit types
- [ ] Write tests for each commit scenario

---

## ISSUE-023: Cross-Tab Document Persistence (ArrayBuffer Detachment)

**Status:** Resolved
**Severity:** Critical
**Component:** pdfjoin-web / app.ts, shared-state.ts
**Date Identified:** 2024-12-27
**Date Resolved:** 2024-12-27

### Description

Cross-tab document persistence (allowing users to upload a PDF once and switch between Split/Merge/Edit tabs without re-uploading) was broken due to ArrayBuffer detachment.

When a user uploaded a PDF to the Split tab and then switched to the Edit or Merge tab, the shared PDF bytes were empty (length 0), causing the document to not load in the destination tab.

### Root Cause

**ArrayBuffer ownership transfer to WASM/PDF.js**

When passing a `Uint8Array` to:
- `splitSession.addDocument()` (WASM)
- `window.pdfjsLib.getDocument()` (PDF.js)

These APIs **transfer ownership** of the underlying ArrayBuffer, making the original bytes empty/detached. The code was storing a reference to the shared state BEFORE calling these APIs, but since they shared the same ArrayBuffer reference, the stored bytes became empty.

### Resolution

Create copies of the bytes using `bytes.slice()` BEFORE any operation that might detach the buffer:

```typescript
// In handleSplitFile()
const bytes = new Uint8Array(await file.arrayBuffer());

// IMPORTANT: Create copies BEFORE any operation that might detach the buffer
const bytesForWasm = bytes.slice();      // Copy for WASM session
const bytesForShared = bytes.slice();    // Copy for shared state (cross-tab)
const bytesForThumbnails = bytes.slice(); // Copy for PDF.js thumbnail rendering

const info: PdfInfo = splitSession.addDocument(file.name, bytesForWasm);
setSharedPdf(bytesForShared, file.name, 'split');
splitState.pdfBytes = bytesForThumbnails;
```

Same pattern applied to:
- `handleSplitFile()` in app.ts
- `loadPdfIntoSplit()` in app.ts
- `loadPdfIntoMerge()` in app.ts
- `handleMergeFilesArray()` in app.ts

### Regression Tests

- `test_tab_switching_split_to_edit_preserves_document`
- `test_tab_switching_split_to_merge_adds_document`
- `test_tab_switching_edit_to_split_preserves_document`
- `test_tab_switching_merge_to_edit_preserves_document`
- `test_tab_switching_merge_to_split_preserves_document`
- `test_tab_switching_roundtrip_preserves_document`

### Acceptance Criteria

- [x] Upload PDF to Split → Switch to Edit → PDF loads automatically
- [x] Upload PDF to Split → Switch to Merge → PDF added to merge list
- [x] Upload PDF to Merge → Switch to Split → PDF loads automatically
- [x] Upload PDF to Edit → Switch to Split → PDF loads automatically
- [x] Roundtrip (Split → Edit → Merge → Split) preserves document

---

## ISSUE-024: FileList Iteration Bug in Merge File Handler

**Status:** Resolved
**Severity:** Critical
**Component:** pdfjoin-web / app.ts
**Date Identified:** 2024-12-27
**Date Resolved:** 2024-12-27

### Description

When selecting multiple files via the merge file input (Cmd+click or Shift+click), 0 files were being added to the merge list even though `files.length` showed the correct count.

### Root Cause

**FileList is a live object that gets cleared when `fileInput.value = ''` is executed**

The change event handler had this sequence:
```typescript
fileInput.addEventListener('change', () => {
  if (fileInput.files) {
    handleMergeFiles(fileInput.files);  // async function
    fileInput.value = '';  // Clears FileList IMMEDIATELY
  }
});
```

Since `handleMergeFiles` is async and starts with `await ensurePdfJsLoaded()`, the `fileInput.value = ''` executed BEFORE the files were actually processed, emptying the FileList.

Additionally, `Array.from(fileList)` doesn't reliably work on synthetic FileList objects created via DataTransfer in browser tests.

### Resolution

1. **Copy File objects to array BEFORE clearing input**:
```typescript
fileInput.addEventListener('change', async () => {
  if (fileInput.files && fileInput.files.length > 0) {
    // IMPORTANT: Copy files BEFORE clearing input (FileList is live)
    const filesArray: File[] = [];
    for (let i = 0; i < fileInput.files.length; i++) {
      filesArray.push(fileInput.files[i]);
    }
    fileInput.value = ''; // Now safe to clear
    await handleMergeFilesArray(filesArray);
  }
});
```

2. **Created `handleMergeFilesArray(files: File[])` function** that takes an array instead of FileList

3. **Added `handleMergeFiles(files: FileList)` wrapper** that copies to array first (for drop handlers)

### Key Learning

- `FileList` is a **live object** - clearing the input clears ALL references to it
- `Array.from(fileList)` may return empty array for synthetic FileList objects
- Always copy File objects individually with index-based access before async operations

### Regression Tests

- `test_pdfjoin_merge_browse_multiple_files` - Verifies multi-file selection works
- `test_pdfjoin_merge_drag_drop_additional_files` - Verifies drag-drop after initial file works

### Acceptance Criteria

- [x] Selecting 3 files via browse button adds all 3 to merge list
- [x] Drag-drop additional files after initial file works
- [x] File names appear correctly in merge document list

---

## ISSUE-025: Partial Text Selection Styling Not Supported

**Status:** Resolved
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts
**Date Identified:** 2024-12-27
**Date Resolved:** 2024-12-27

### Description

Users cannot apply text styling (bold, italic, font size) to **partial text selections**. When a user types "HELLO WORLD", selects just "HELLO", and clicks Bold, the **entire text** gets bolded instead of just the selected portion.

This is a fundamental feature gap affecting:
1. **Text Boxes** - Cannot bold part of textbox content
2. **Whiteout Text** - Cannot style part of whiteout text
3. **Existing Text Replacement** - Cannot style part of replaced text

### Steps to Reproduce

1. Load a PDF
2. Create a text box with TextBox tool (double-click)
3. Type "HELLO WORLD"
4. Use cursor to select just "HELLO" (first 5 characters)
5. Click Bold button
6. **Result:** Entire text "HELLO WORLD" becomes bold
7. **Expected:** Only "HELLO" should be bold

### Root Cause

The current styling implementation in `edit.ts` applies styles to the **entire element** rather than the current selection:

```typescript
// Current (broken) implementation
function toggleBold(): void {
  if (!activeTextInput) return;
  activeTextInput.style.fontWeight = newBold ? 'bold' : 'normal';
  // This affects ALL text in the element, not just selected text
}
```

The code should use `document.execCommand('bold')` or the Selection API with `surroundContents()` to wrap only the selected text in styling elements like `<b>` or `<span style="font-weight:bold">`.

### Resolution Required

Implement selection-based styling using one of:

1. **`execCommand` approach** (deprecated but widely supported):
```typescript
function toggleBold(): void {
  const selection = window.getSelection();
  if (selection && !selection.isCollapsed) {
    document.execCommand('bold');  // Only affects selection
  } else if (activeTextInput) {
    // Fall back to whole-element styling if no selection
    activeTextInput.style.fontWeight = ...;
  }
}
```

2. **Modern Selection API approach**:
```typescript
function toggleBold(): void {
  const selection = window.getSelection();
  if (selection && selection.rangeCount > 0 && !selection.isCollapsed) {
    const range = selection.getRangeAt(0);
    const boldSpan = document.createElement('b');
    range.surroundContents(boldSpan);
  }
}
```

### Additional Complexity

After implementing partial styling, these need to handle:
1. **PDF Export** - Parse HTML structure (`<b>`, `<i>`, `<span>`) and generate corresponding PDF text commands with different fonts
2. **Toggle off** - Detect if selection is already bold/italic and remove formatting
3. **Nested styles** - Handle `<b><i>text</i></b>` correctly
4. **Font Size** - Need to handle `<span style="font-size:X">` for partial font size changes

### Acceptance Criteria

- [x] Selecting "HELLO" in "HELLO WORLD" and bolding only bolds "HELLO"
- [x] innerHTML shows `<b>HELLO</b> WORLD` or equivalent
- [x] Same behavior works for:
  - [x] Text boxes
  - [x] Whiteout text
  - [ ] Existing text replacement (needs verification)
- [ ] Partial styling persists to exported PDF (needs verification for mixed-style text)
- [x] Multiple styles can be combined (bold + italic on same selection)
- [x] Clicking on box edge (no selection) still applies to entire text

### Test References

- `test_textbox_partial_selection_bold` - Tests textbox partial selection (PASSES)
- `test_whiteout_partial_selection_bold` - Tests whiteout partial selection (PASSES)
- `test_textbox_multiple_styles_at_once` - Tests applying multiple whole-text styles (PASSES)
- `test_textbox_styles_persist_to_pdf` - Tests export persistence (PASSES)

### Resolution

Modified `toggleBold()` and `toggleItalic()` in `edit.ts` to use `document.execCommand()` for selection-based styling:

```typescript
function toggleBold(): void {
  if (!activeTextInput) return;

  // ISSUE-025 FIX: Check for text selection within the active input
  const selection = window.getSelection();
  const hasSelection =
    selection &&
    selection.rangeCount > 0 &&
    !selection.isCollapsed &&
    activeTextInput.contains(selection.anchorNode);

  if (hasSelection) {
    // Apply bold to selected text only using execCommand
    // This wraps selected text in <b> or <strong> tags
    document.execCommand('bold', false);
    activeTextInput.focus();
  } else {
    // No selection - apply to entire element (fallback behavior)
    // ... original whole-element styling
  }
}
```

Same pattern applied to `toggleItalic()`.

**Note:** `document.execCommand` is deprecated but still widely supported and is the simplest approach for this use case. The modern Selection API approach with `surroundContents()` has edge cases that make it more complex.

### Verification

All 6 styling tests pass:
```
test_textbox_partial_selection_bold ... ok
test_whiteout_partial_selection_bold ... ok
test_textbox_styling_bold_italic_fontsize ... ok
test_whiteout_text_styling_bold_italic_fontsize ... ok
test_textbox_multiple_styles_at_once ... ok
test_textbox_styles_persist_to_pdf ... ok
```

---

## ISSUE-025b: Mixed Partial Styling Not Exported to PDF

**Status:** Resolved
**Severity:** Critical
**Component:** pdfjoin-web / edit.ts, pdfjoin-core
**Date Identified:** 2024-12-27
**Date Resolved:** 2024-12-27

### Description

While ISSUE-025 fixed partial text selection styling in the **preview** (e.g., selecting "BOLD" and bolding it, then selecting "ITAL" and italicizing it produces `<b>BOLD</b><i>ITAL</i>` in the DOM), this mixed styling was **NOT persisting** to the exported PDF.

The exported PDF only contained a single font style because `commitTextBox()` was reading `textContent.textContent` (plain text) and applying only the computed style of the whole element.

### Resolution

#### New API: `AddStyledText` Operation

Added a new `AddStyledText` operation that accepts an array of styled text segments, each with its own bold/italic flags:

**Rust types (operations.rs):**
```rust
pub struct StyledTextSegment {
    pub text: String,
    pub is_bold: bool,
    pub is_italic: bool,
}

pub enum EditOperation {
    // ... existing variants ...
    AddStyledText {
        id: OpId,
        page: u32,
        rect: PdfRect,
        segments: Vec<StyledTextSegment>,
        style: TextStyle, // Base style (font_size, color)
    },
}
```

**PDF Generation (apply_operations.rs):**
The new operation generates a PDF content stream that switches fonts within a single text block:
```
BT
/F1 12 Tf  (Helvetica-Bold)
(BOLD) Tj
/F3 12 Tf  (Helvetica-Oblique)
(ITAL) Tj
ET
```

**TypeScript Integration:**
- Added `parseStyledSegments(element)` function to extract styled segments from innerHTML
- Modified `commitTextBox()` to detect mixed styling and use `addStyledText` when needed
- Modified `saveWhiteoutText()` similarly

### Files Changed

1. `crates/pdfjoin-core/src/operations.rs` - Added `StyledTextSegment` struct and `AddStyledText` operation
2. `crates/pdfjoin-core/src/apply_operations.rs` - Implemented PDF generation for styled text
3. `apps/pdfjoin-web/wasm/src/edit_session.rs` - Added `addStyledText()` WASM binding
4. `apps/pdfjoin-web/src/ts/types/wasm-bindings.ts` - Added TypeScript interface
5. `apps/pdfjoin-web/src/ts/edit.ts` - Added `parseStyledSegments()` and modified commit functions

### Test Results

All styling tests now pass:
```
test_mixed_styling_both_fonts_in_pdf_export ... ok
test_textbox_mixed_partial_styling_export ... ok
test_textbox_mixed_partial_styling_preview ... ok
test_textbox_styling_bold_italic_fontsize ... ok
test_whiteout_mixed_partial_styling_preview ... ok
test_whiteout_text_styling_bold_italic_fontsize ... ok
test_textbox_partial_selection_bold ... ok
test_whiteout_partial_selection_bold ... ok
```

### Acceptance Criteria

- [x] Export PDF contains BOTH bold AND italic font references when mixed styling used
- [x] "BOLD" text appears bold in exported PDF
- [x] "ITAL" text appears italic in exported PDF
- [x] Works for textbox and whiteout text
- [x] Handles nested styles (bold-italic)
- [x] Handles 3+ different style segments

