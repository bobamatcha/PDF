# Known Issues

This document tracks known issues that need investigation or resolution.

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

