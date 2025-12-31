/**
 * Template Completion Engine for agentPDF
 *
 * This is a LIMITED editor for completing generated templates.
 * It is NOT a general-purpose PDF editor.
 *
 * ALLOWED:
 * - Text fields (fill in blanks)
 * - Signature fields (mark where to sign)
 * - Initials fields (contract revision marks)
 * - Checkbox fields (yes/no selections)
 * - Date fields (auto-formatted)
 * - Font controls (size, type, bold/italic)
 * - Page split/merge operations
 *
 * EXPLICITLY NOT ALLOWED (prevents contract drafting/UPL concerns):
 * - Whiteout/blackout tools
 * - Text replacement on existing content
 * - Adding paragraphs/sections
 * - Highlight/underline tools
 */

import { PdfBridge } from './pdf-bridge';
import { domRectToPdf, pdfRectToDom, getPageRenderInfo, type Rect } from './coord-utils';
import type { PDFJSViewport, CachedPageInfo } from './types/pdf-types';

// ============================================================================
// FIELD TYPES - Limited set for template completion only
// ============================================================================

/**
 * Allowed field types for template completion
 * This enum is intentionally limited to prevent contract drafting
 */
export enum FieldType {
  Text = 'text',           // Fill in names, dates, amounts
  Signature = 'signature', // Mark where to sign
  Initials = 'initials',   // Contract revision acknowledgment
  Checkbox = 'checkbox',   // Yes/No selections
  Date = 'date',           // Auto-formatted date entry
}

/**
 * Field style options
 */
export interface FieldStyle {
  fontSize: number;      // Font size in pixels
  fontFamily: 'serif' | 'sans-serif' | 'monospace';
  isBold: boolean;
  isItalic: boolean;
  color: string;         // Hex color
}

/**
 * Placed field data
 */
export interface PlacedField {
  id: string;
  type: FieldType;
  pageNum: number;
  // DOM coordinates (relative to page container)
  domX: number;
  domY: number;
  domWidth: number;
  domHeight: number;
  // PDF coordinates (for export)
  pdfX: number;
  pdfY: number;
  pdfWidth: number;
  pdfHeight: number;
  // Content and style
  value: string;
  style: FieldStyle;
  // For checkbox
  checked?: boolean;
}

// ============================================================================
// CONSTANTS
// ============================================================================

const MIN_FIELD_WIDTH = 100;   // WCAG minimum touch target
const MIN_FIELD_HEIGHT = 44;   // WCAG minimum touch target
const DEFAULT_TEXT_WIDTH = 200;
const DEFAULT_TEXT_HEIGHT = 48;
const DEFAULT_SIGNATURE_WIDTH = 200;
const DEFAULT_SIGNATURE_HEIGHT = 60;
const DEFAULT_INITIALS_WIDTH = 80;
const DEFAULT_INITIALS_HEIGHT = 44;
const DEFAULT_CHECKBOX_SIZE = 24;
const DEFAULT_DATE_WIDTH = 150;
const DEFAULT_DATE_HEIGHT = 44;

const DEFAULT_STYLE: FieldStyle = {
  fontSize: 14,
  fontFamily: 'sans-serif',
  isBold: false,
  isItalic: false,
  color: '#000000',
};

// ============================================================================
// TEMPLATE EDITOR STATE
// ============================================================================

interface EditorState {
  currentTool: FieldType | 'select' | null;
  fields: Map<string, PlacedField>;
  selectedFieldId: string | null;
  currentStyle: FieldStyle;
  pdfBytes: Uint8Array | null;
  pageCount: number;
}

const state: EditorState = {
  currentTool: null,
  fields: new Map(),
  selectedFieldId: null,
  currentStyle: { ...DEFAULT_STYLE },
  pdfBytes: null,
  pageCount: 0,
};

// ============================================================================
// FIELD CREATION
// ============================================================================

/**
 * Generate unique field ID
 */
function generateFieldId(): string {
  return `field-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

/**
 * Get default dimensions for a field type
 */
function getDefaultDimensions(type: FieldType): { width: number; height: number } {
  switch (type) {
    case FieldType.Text:
      return { width: DEFAULT_TEXT_WIDTH, height: DEFAULT_TEXT_HEIGHT };
    case FieldType.Signature:
      return { width: DEFAULT_SIGNATURE_WIDTH, height: DEFAULT_SIGNATURE_HEIGHT };
    case FieldType.Initials:
      return { width: DEFAULT_INITIALS_WIDTH, height: DEFAULT_INITIALS_HEIGHT };
    case FieldType.Checkbox:
      return { width: DEFAULT_CHECKBOX_SIZE, height: DEFAULT_CHECKBOX_SIZE };
    case FieldType.Date:
      return { width: DEFAULT_DATE_WIDTH, height: DEFAULT_DATE_HEIGHT };
  }
}

/**
 * Create a field element in the DOM
 */
function createFieldElement(field: PlacedField, pageContainer: HTMLElement): HTMLElement {
  const el = document.createElement('div');
  el.id = field.id;
  el.className = `template-field template-field-${field.type}`;
  el.dataset.fieldId = field.id;
  el.dataset.fieldType = field.type;
  el.dataset.pageNum = field.pageNum.toString();

  // Position
  el.style.position = 'absolute';
  el.style.left = `${field.domX}px`;
  el.style.top = `${field.domY}px`;
  el.style.width = `${field.domWidth}px`;
  el.style.height = `${field.domHeight}px`;

  // Common styling
  el.style.border = '2px solid #0066cc';
  el.style.borderRadius = '4px';
  el.style.backgroundColor = 'rgba(255, 255, 255, 0.9)';
  el.style.cursor = 'move';
  el.style.boxSizing = 'border-box';

  // Type-specific content
  switch (field.type) {
    case FieldType.Text:
    case FieldType.Date:
      createTextFieldContent(el, field);
      break;
    case FieldType.Signature:
      createSignatureFieldContent(el, field);
      break;
    case FieldType.Initials:
      createInitialsFieldContent(el, field);
      break;
    case FieldType.Checkbox:
      createCheckboxFieldContent(el, field);
      break;
  }

  // Add resize handles
  addResizeHandles(el, field);

  // Add delete button
  addDeleteButton(el, field);

  // Add to page container
  const overlay = pageContainer.querySelector('.field-overlay');
  if (overlay) {
    overlay.appendChild(el);
  }

  return el;
}

/**
 * Create text field content (editable input)
 */
function createTextFieldContent(el: HTMLElement, field: PlacedField): void {
  const input = document.createElement('input');
  input.type = field.type === FieldType.Date ? 'date' : 'text';
  input.className = 'field-input';
  input.value = field.value;
  input.placeholder = field.type === FieldType.Date ? 'Select date' : 'Enter text...';

  // Apply font styles
  input.style.fontSize = `${field.style.fontSize}px`;
  input.style.fontFamily = field.style.fontFamily;
  input.style.fontWeight = field.style.isBold ? 'bold' : 'normal';
  input.style.fontStyle = field.style.isItalic ? 'italic' : 'normal';
  input.style.color = field.style.color;
  input.style.border = 'none';
  input.style.outline = 'none';
  input.style.width = '100%';
  input.style.height = '100%';
  input.style.padding = '4px 8px';
  input.style.boxSizing = 'border-box';
  input.style.backgroundColor = 'transparent';

  input.addEventListener('input', () => {
    field.value = input.value;
  });

  input.addEventListener('focus', () => {
    selectField(field.id);
  });

  el.appendChild(input);
}

/**
 * Create signature field content (placeholder box)
 */
function createSignatureFieldContent(el: HTMLElement, field: PlacedField): void {
  el.style.backgroundColor = 'rgba(255, 245, 230, 0.9)';
  el.style.borderColor = '#cc6600';

  const label = document.createElement('div');
  label.className = 'field-label';
  label.textContent = 'Signature';
  label.style.textAlign = 'center';
  label.style.color = '#cc6600';
  label.style.fontSize = '12px';
  label.style.fontStyle = 'italic';
  label.style.lineHeight = `${field.domHeight}px`;

  el.appendChild(label);
}

/**
 * Create initials field content (small editable box)
 */
function createInitialsFieldContent(el: HTMLElement, field: PlacedField): void {
  el.style.backgroundColor = 'rgba(230, 245, 255, 0.9)';
  el.style.borderColor = '#0099cc';

  const input = document.createElement('input');
  input.type = 'text';
  input.className = 'field-input';
  input.value = field.value;
  input.placeholder = 'AB';
  input.maxLength = 4; // Initials are short

  input.style.fontSize = `${field.style.fontSize}px`;
  input.style.fontFamily = field.style.fontFamily;
  input.style.fontWeight = 'bold';
  input.style.textAlign = 'center';
  input.style.border = 'none';
  input.style.outline = 'none';
  input.style.width = '100%';
  input.style.height = '100%';
  input.style.padding = '4px';
  input.style.boxSizing = 'border-box';
  input.style.backgroundColor = 'transparent';

  input.addEventListener('input', () => {
    field.value = input.value.toUpperCase();
    input.value = field.value;
  });

  input.addEventListener('focus', () => {
    selectField(field.id);
  });

  el.appendChild(input);
}

/**
 * Create checkbox field content
 */
function createCheckboxFieldContent(el: HTMLElement, field: PlacedField): void {
  el.style.backgroundColor = 'rgba(240, 255, 240, 0.9)';
  el.style.borderColor = '#00cc66';
  el.style.display = 'flex';
  el.style.alignItems = 'center';
  el.style.justifyContent = 'center';

  const checkbox = document.createElement('input');
  checkbox.type = 'checkbox';
  checkbox.className = 'field-checkbox';
  checkbox.checked = field.checked || false;
  checkbox.style.width = '18px';
  checkbox.style.height = '18px';
  checkbox.style.cursor = 'pointer';

  checkbox.addEventListener('change', () => {
    field.checked = checkbox.checked;
    field.value = checkbox.checked ? 'Yes' : 'No';
  });

  checkbox.addEventListener('focus', () => {
    selectField(field.id);
  });

  el.appendChild(checkbox);
}

/**
 * Add resize handles to a field element
 */
function addResizeHandles(el: HTMLElement, field: PlacedField): void {
  const handles = ['nw', 'n', 'ne', 'w', 'e', 'sw', 's', 'se'];

  handles.forEach((position) => {
    const handle = document.createElement('div');
    handle.className = `resize-handle resize-${position}`;
    handle.style.position = 'absolute';
    handle.style.width = '8px';
    handle.style.height = '8px';
    handle.style.backgroundColor = '#0066cc';
    handle.style.borderRadius = '50%';
    handle.style.display = 'none'; // Show only when selected

    // Position the handle
    switch (position) {
      case 'nw': handle.style.top = '-4px'; handle.style.left = '-4px'; handle.style.cursor = 'nw-resize'; break;
      case 'n': handle.style.top = '-4px'; handle.style.left = '50%'; handle.style.transform = 'translateX(-50%)'; handle.style.cursor = 'n-resize'; break;
      case 'ne': handle.style.top = '-4px'; handle.style.right = '-4px'; handle.style.cursor = 'ne-resize'; break;
      case 'w': handle.style.top = '50%'; handle.style.left = '-4px'; handle.style.transform = 'translateY(-50%)'; handle.style.cursor = 'w-resize'; break;
      case 'e': handle.style.top = '50%'; handle.style.right = '-4px'; handle.style.transform = 'translateY(-50%)'; handle.style.cursor = 'e-resize'; break;
      case 'sw': handle.style.bottom = '-4px'; handle.style.left = '-4px'; handle.style.cursor = 'sw-resize'; break;
      case 's': handle.style.bottom = '-4px'; handle.style.left = '50%'; handle.style.transform = 'translateX(-50%)'; handle.style.cursor = 's-resize'; break;
      case 'se': handle.style.bottom = '-4px'; handle.style.right = '-4px'; handle.style.cursor = 'se-resize'; break;
    }

    handle.addEventListener('mousedown', (e) => {
      e.stopPropagation();
      startResize(field.id, position, e);
    });

    el.appendChild(handle);
  });
}

/**
 * Add delete button to a field element
 */
function addDeleteButton(el: HTMLElement, field: PlacedField): void {
  const btn = document.createElement('button');
  btn.className = 'field-delete-btn';
  btn.textContent = '×';
  btn.style.position = 'absolute';
  btn.style.top = '-12px';
  btn.style.right = '-12px';
  btn.style.width = '24px';
  btn.style.height = '24px';
  btn.style.borderRadius = '50%';
  btn.style.border = 'none';
  btn.style.backgroundColor = '#cc0000';
  btn.style.color = 'white';
  btn.style.cursor = 'pointer';
  btn.style.fontSize = '16px';
  btn.style.lineHeight = '1';
  btn.style.display = 'none'; // Show only when selected

  btn.addEventListener('click', (e) => {
    e.stopPropagation();
    deleteField(field.id);
  });

  el.appendChild(btn);
}

// ============================================================================
// FIELD OPERATIONS
// ============================================================================

/**
 * Place a new field at the given position
 */
export function placeField(
  type: FieldType,
  pageNum: number,
  domX: number,
  domY: number,
  viewport: PDFJSViewport
): PlacedField {
  const dimensions = getDefaultDimensions(type);
  const id = generateFieldId();

  // Convert to PDF coordinates
  const pdfRect = domRectToPdf(viewport, domX, domY, dimensions.width, dimensions.height);

  const field: PlacedField = {
    id,
    type,
    pageNum,
    domX,
    domY,
    domWidth: dimensions.width,
    domHeight: dimensions.height,
    pdfX: pdfRect.x,
    pdfY: pdfRect.y,
    pdfWidth: pdfRect.width,
    pdfHeight: pdfRect.height,
    value: '',
    style: { ...state.currentStyle },
    checked: type === FieldType.Checkbox ? false : undefined,
  };

  state.fields.set(id, field);

  // Create DOM element
  const pageContainer = document.querySelector(`[data-page="${pageNum}"]`);
  if (pageContainer) {
    createFieldElement(field, pageContainer as HTMLElement);
  }

  selectField(id);
  return field;
}

/**
 * Select a field
 */
export function selectField(id: string): void {
  // Deselect previous
  if (state.selectedFieldId) {
    const prevEl = document.getElementById(state.selectedFieldId);
    if (prevEl) {
      prevEl.classList.remove('selected');
      prevEl.querySelectorAll('.resize-handle, .field-delete-btn').forEach((h) => {
        (h as HTMLElement).style.display = 'none';
      });
    }
  }

  state.selectedFieldId = id;
  const el = document.getElementById(id);
  if (el) {
    el.classList.add('selected');
    el.querySelectorAll('.resize-handle, .field-delete-btn').forEach((h) => {
      (h as HTMLElement).style.display = 'block';
    });
  }
}

/**
 * Delete a field
 */
export function deleteField(id: string): void {
  const el = document.getElementById(id);
  if (el) {
    el.remove();
  }
  state.fields.delete(id);
  if (state.selectedFieldId === id) {
    state.selectedFieldId = null;
  }
}

/**
 * Update field style
 */
export function updateFieldStyle(id: string, style: Partial<FieldStyle>): void {
  const field = state.fields.get(id);
  if (!field) return;

  Object.assign(field.style, style);

  // Update DOM element
  const el = document.getElementById(id);
  if (!el) return;

  const input = el.querySelector('input') as HTMLInputElement | null;
  if (input) {
    if (style.fontSize !== undefined) input.style.fontSize = `${style.fontSize}px`;
    if (style.fontFamily !== undefined) input.style.fontFamily = style.fontFamily;
    if (style.isBold !== undefined) input.style.fontWeight = style.isBold ? 'bold' : 'normal';
    if (style.isItalic !== undefined) input.style.fontStyle = style.isItalic ? 'italic' : 'normal';
    if (style.color !== undefined) input.style.color = style.color;
  }
}

// ============================================================================
// DRAG & RESIZE
// ============================================================================

let dragState: {
  fieldId: string;
  startX: number;
  startY: number;
  startFieldX: number;
  startFieldY: number;
} | null = null;

let resizeState: {
  fieldId: string;
  handle: string;
  startX: number;
  startY: number;
  startRect: { x: number; y: number; width: number; height: number };
} | null = null;

/**
 * Start dragging a field
 */
export function startDrag(id: string, e: MouseEvent): void {
  const field = state.fields.get(id);
  if (!field) return;

  dragState = {
    fieldId: id,
    startX: e.clientX,
    startY: e.clientY,
    startFieldX: field.domX,
    startFieldY: field.domY,
  };

  selectField(id);
  document.addEventListener('mousemove', onDragMove);
  document.addEventListener('mouseup', onDragEnd);
}

function onDragMove(e: MouseEvent): void {
  if (!dragState) return;

  const field = state.fields.get(dragState.fieldId);
  if (!field) return;

  const dx = e.clientX - dragState.startX;
  const dy = e.clientY - dragState.startY;

  field.domX = dragState.startFieldX + dx;
  field.domY = dragState.startFieldY + dy;

  const el = document.getElementById(dragState.fieldId);
  if (el) {
    el.style.left = `${field.domX}px`;
    el.style.top = `${field.domY}px`;
  }

  // Update PDF coordinates
  const pageInfo = PdfBridge.getPageInfo(field.pageNum);
  if (pageInfo) {
    const pdfRect = domRectToPdf(pageInfo.viewport, field.domX, field.domY, field.domWidth, field.domHeight);
    field.pdfX = pdfRect.x;
    field.pdfY = pdfRect.y;
  }
}

function onDragEnd(): void {
  dragState = null;
  document.removeEventListener('mousemove', onDragMove);
  document.removeEventListener('mouseup', onDragEnd);
}

/**
 * Start resizing a field
 */
function startResize(id: string, handle: string, e: MouseEvent): void {
  const field = state.fields.get(id);
  if (!field) return;

  resizeState = {
    fieldId: id,
    handle,
    startX: e.clientX,
    startY: e.clientY,
    startRect: {
      x: field.domX,
      y: field.domY,
      width: field.domWidth,
      height: field.domHeight,
    },
  };

  document.addEventListener('mousemove', onResizeMove);
  document.addEventListener('mouseup', onResizeEnd);
}

function onResizeMove(e: MouseEvent): void {
  if (!resizeState) return;

  const field = state.fields.get(resizeState.fieldId);
  if (!field) return;

  const dx = e.clientX - resizeState.startX;
  const dy = e.clientY - resizeState.startY;
  const { handle, startRect } = resizeState;

  let newX = startRect.x;
  let newY = startRect.y;
  let newWidth = startRect.width;
  let newHeight = startRect.height;

  // Calculate new dimensions based on handle
  if (handle.includes('w')) {
    newX = startRect.x + dx;
    newWidth = startRect.width - dx;
  }
  if (handle.includes('e')) {
    newWidth = startRect.width + dx;
  }
  if (handle.includes('n')) {
    newY = startRect.y + dy;
    newHeight = startRect.height - dy;
  }
  if (handle.includes('s')) {
    newHeight = startRect.height + dy;
  }

  // Enforce minimum size
  if (newWidth < MIN_FIELD_WIDTH) {
    if (handle.includes('w')) {
      newX = startRect.x + startRect.width - MIN_FIELD_WIDTH;
    }
    newWidth = MIN_FIELD_WIDTH;
  }
  if (newHeight < MIN_FIELD_HEIGHT) {
    if (handle.includes('n')) {
      newY = startRect.y + startRect.height - MIN_FIELD_HEIGHT;
    }
    newHeight = MIN_FIELD_HEIGHT;
  }

  // Update field
  field.domX = newX;
  field.domY = newY;
  field.domWidth = newWidth;
  field.domHeight = newHeight;

  // Update DOM
  const el = document.getElementById(resizeState.fieldId);
  if (el) {
    el.style.left = `${newX}px`;
    el.style.top = `${newY}px`;
    el.style.width = `${newWidth}px`;
    el.style.height = `${newHeight}px`;
  }

  // Update PDF coordinates
  const pageInfo = PdfBridge.getPageInfo(field.pageNum);
  if (pageInfo) {
    const pdfRect = domRectToPdf(pageInfo.viewport, field.domX, field.domY, field.domWidth, field.domHeight);
    field.pdfX = pdfRect.x;
    field.pdfY = pdfRect.y;
    field.pdfWidth = pdfRect.width;
    field.pdfHeight = pdfRect.height;
  }
}

function onResizeEnd(): void {
  resizeState = null;
  document.removeEventListener('mousemove', onResizeMove);
  document.removeEventListener('mouseup', onResizeEnd);
}

// ============================================================================
// TOOL MANAGEMENT
// ============================================================================

/**
 * Set the current tool
 */
export function setTool(tool: FieldType | 'select' | null): void {
  state.currentTool = tool;

  // Update cursor style on editor container
  const container = document.querySelector('.template-editor-container');
  if (container) {
    if (tool === 'select' || tool === null) {
      (container as HTMLElement).style.cursor = 'default';
    } else {
      (container as HTMLElement).style.cursor = 'crosshair';
    }
  }
}

/**
 * Get current tool
 */
export function getCurrentTool(): FieldType | 'select' | null {
  return state.currentTool;
}

/**
 * Set current style (applies to new fields)
 */
export function setCurrentStyle(style: Partial<FieldStyle>): void {
  Object.assign(state.currentStyle, style);

  // Also update selected field if any
  if (state.selectedFieldId) {
    updateFieldStyle(state.selectedFieldId, style);
  }
}

// ============================================================================
// PDF RENDERING
// ============================================================================

/**
 * Load a PDF for editing
 */
export async function loadPdf(bytes: Uint8Array): Promise<number> {
  state.pdfBytes = bytes;
  state.pageCount = await PdfBridge.loadDocument(bytes);
  return state.pageCount;
}

/**
 * Render a page to a container
 */
export async function renderPage(pageNum: number, container: HTMLElement): Promise<void> {
  // Create page structure
  container.innerHTML = '';
  container.className = 'template-editor-page';
  container.dataset.page = pageNum.toString();

  const canvas = document.createElement('canvas');
  canvas.className = 'page-canvas';
  container.appendChild(canvas);

  // Field overlay (positioned over canvas)
  const overlay = document.createElement('div');
  overlay.className = 'field-overlay';
  overlay.style.position = 'absolute';
  overlay.style.top = '0';
  overlay.style.left = '0';
  overlay.style.width = '100%';
  overlay.style.height = '100%';
  overlay.style.pointerEvents = 'none'; // Pass through clicks to fields
  container.appendChild(overlay);

  // Render PDF page
  const dims = await PdfBridge.renderPage(pageNum, canvas);
  container.style.width = `${dims.width}px`;
  container.style.height = `${dims.height}px`;
  container.style.position = 'relative';

  // Set up click handler for field placement
  canvas.addEventListener('click', (e) => {
    if (state.currentTool && state.currentTool !== 'select') {
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      const pageInfo = PdfBridge.getPageInfo(pageNum);
      if (pageInfo) {
        placeField(state.currentTool as FieldType, pageNum, x, y, pageInfo.viewport);
        setTool('select'); // Switch back to select after placing
      }
    }
  });
}

/**
 * Render all pages
 */
export async function renderAllPages(container: HTMLElement): Promise<void> {
  container.innerHTML = '';

  for (let i = 1; i <= state.pageCount; i++) {
    const pageDiv = document.createElement('div');
    pageDiv.className = 'page-wrapper';
    pageDiv.style.marginBottom = '20px';
    container.appendChild(pageDiv);
    await renderPage(i, pageDiv);
  }
}

// ============================================================================
// EXPORT
// ============================================================================

/**
 * Get all placed fields
 */
export function getAllFields(): PlacedField[] {
  return Array.from(state.fields.values());
}

/**
 * Get fields as JSON for export
 */
export function exportFieldsAsJson(): string {
  return JSON.stringify(getAllFields(), null, 2);
}

/**
 * Clear all fields
 */
export function clearAllFields(): void {
  state.fields.forEach((_, id) => {
    const el = document.getElementById(id);
    if (el) el.remove();
  });
  state.fields.clear();
  state.selectedFieldId = null;
}

// ============================================================================
// EXPOSE ON WINDOW
// ============================================================================

export const TemplateEditor = {
  // Field types
  FieldType,

  // Field operations
  placeField,
  selectField,
  deleteField,
  updateFieldStyle,
  getAllFields,
  exportFieldsAsJson,
  clearAllFields,

  // Tool management
  setTool,
  getCurrentTool,
  setCurrentStyle,

  // PDF operations
  loadPdf,
  renderPage,
  renderAllPages,

  // Drag operations
  startDrag,
};

// Expose on window for use from HTML
(window as unknown as { TemplateEditor: typeof TemplateEditor }).TemplateEditor = TemplateEditor;
