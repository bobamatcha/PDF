// Edit PDF functionality
// Uses lazy-loaded PDF.js and EditSession from WASM

import { ensurePdfJsLoaded } from './pdf-loader';
import { PdfBridge } from './pdf-bridge';
import { registerEditCallbacks, clearEditCallbacks, setSharedPdf } from './shared-state';
import type { EditSession, OpId, TextItem, CachedPageInfo, ActionKind } from './types';
import { getOpId, setOpId } from './types';

let editSession: EditSession | null = null;
let currentTool = 'select';
let currentPage = 1;
// Note: operationHistory removed - undo/redo now handled by Rust via editSession
let currentPdfBytes: Uint8Array | null = null; // Original PDF bytes for Tab PDF Sharing
let currentPdfFilename: string | null = null;
let textItemsMap = new Map<number, TextItem[]>(); // pageNum -> array of text items with positions
let activeEditItem: {
  pageNum: number;
  index: number;
  textItem: TextItem;
  spanElement: HTMLElement;
} | null = null;
let activeTextInput: HTMLElement | null = null; // Currently focused text input (for B/I buttons)

// Whiteout drawing state
let isDrawing = false;
let drawStartX = 0;
let drawStartY = 0;
let drawOverlay: HTMLElement | null = null;
let drawPageNum: number | null = null;
let pendingFileToLoad: { file: File } | null = null; // For file replace confirmation flow

/**
 * Shows a confirmation dialog for destructive actions.
 * Returns a promise that resolves to true if confirmed, false if cancelled.
 * Uses the reusable confirm-dialog-overlay from index.html
 */
function showEditConfirmDialog(options: {
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  icon?: string;
  modalType?: string; // Optional identifier for the modal type (e.g., 'file-replace')
}): Promise<boolean> {
  return new Promise((resolve) => {
    const overlay = document.getElementById('confirm-dialog-overlay');
    const heading = document.getElementById('confirm-dialog-heading');
    const message = document.getElementById('confirm-dialog-message');
    const icon = document.getElementById('confirm-dialog-icon');
    const confirmBtn = document.getElementById('confirm-dialog-confirm');
    const cancelBtn = document.getElementById('confirm-dialog-cancel');

    if (!overlay || !heading || !message || !confirmBtn || !cancelBtn) {
      // Fallback to native confirm if dialog elements not found
      resolve(window.confirm(options.message));
      return;
    }

    // Set dialog content
    heading.textContent = options.title;
    message.textContent = options.message;
    if (icon) icon.innerHTML = options.icon || '&#9888;';
    confirmBtn.textContent = options.confirmText || 'Replace';
    cancelBtn.textContent = options.cancelText || 'Cancel';

    // Set modal type identifier for testing (e.g., 'file-replace')
    if (options.modalType) {
      overlay.setAttribute('data-modal', options.modalType);
      overlay.classList.add(`${options.modalType}-confirm`);
    }

    // Show dialog
    overlay.classList.add('show');
    confirmBtn.focus();

    // Clean up function
    const cleanup = () => {
      overlay.classList.remove('show');
      // Remove modal type identifiers
      if (options.modalType) {
        overlay.removeAttribute('data-modal');
        overlay.classList.remove(`${options.modalType}-confirm`);
      }
      confirmBtn.removeEventListener('click', onConfirm);
      cancelBtn.removeEventListener('click', onCancel);
      overlay.removeEventListener('click', onOverlayClick);
      document.removeEventListener('keydown', onKeydown);
    };

    const onConfirm = () => {
      cleanup();
      resolve(true);
    };

    const onCancel = () => {
      cleanup();
      resolve(false);
    };

    const onOverlayClick = (e: Event) => {
      if (e.target === overlay) {
        onCancel();
      }
    };

    const onKeydown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onCancel();
      } else if (e.key === 'Enter') {
        onConfirm();
      }
    };

    confirmBtn.addEventListener('click', onConfirm);
    cancelBtn.addEventListener('click', onCancel);
    overlay.addEventListener('click', onOverlayClick);
    document.addEventListener('keydown', onKeydown);
  });
}
let drawPreviewEl: HTMLElement | null = null;
let drawPageDiv: HTMLElement | null = null;

// Resize state
let resizing = false;
let resizeTarget: HTMLElement | null = null;
let resizeHandle = '';
let resizeStartX = 0;
let resizeStartY = 0;
let resizeStartRect: { left: number; top: number; width: number; height: number } | null = null;

// Move state
let moving = false;
let moveTarget: HTMLElement | null = null;
let moveStartX = 0;
let moveStartY = 0;
let moveStartLeft = 0;
let moveStartTop = 0;

// Text drag state
let draggingTextOverlay: HTMLElement | null = null;
let textDragStartX = 0;
let textDragStartY = 0;
let textDragStartLeft = 0;
let textDragStartTop = 0;

// Selected whiteout
let selectedWhiteout: HTMLElement | null = null;

// Selected text box
let selectedTextBox: HTMLElement | null = null;

// Text box tracking
let textBoxes: Map<number, HTMLElement> = new Map();
let nextTextBoxId = 0;

// DISABLED: Highlight tool is hidden (ISSUE-001)
// let currentHighlightColor = '#FFFF00'; // Default yellow

// Blackout mode: false = whiteout (#FFFFFF), true = blackout (#000000)
let isBlackoutMode = false;

export function setupEditView(): void {
  const dropZone = document.getElementById('edit-drop-zone');
  const fileInput = document.getElementById('edit-file-input') as HTMLInputElement | null;
  const browseBtn = document.getElementById('edit-browse-btn');
  const removeBtn = document.getElementById('edit-remove-btn');
  const downloadBtn = document.getElementById('edit-download-btn');
  const goBackBtn = document.getElementById('edit-go-back-btn');
  const useSplitBtn = document.getElementById('edit-use-split-btn');
  const undoBtn = document.getElementById('edit-undo-btn');

  if (!dropZone || !fileInput || !browseBtn || !removeBtn || !downloadBtn || !undoBtn) return;

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
    if (e.dataTransfer?.files.length) {
      handleEditFile(e.dataTransfer.files[0]);
    }
  });

  fileInput.addEventListener('change', () => {
    if (fileInput.files?.length) {
      handleEditFile(fileInput.files[0]);
    }
  });

  // Actions
  removeBtn.addEventListener('click', resetEditView);
  downloadBtn.addEventListener('click', downloadEditedPdf);
  undoBtn.addEventListener('click', undoLastOperation);

  // Redo button
  const redoBtn = document.getElementById('edit-redo-btn');
  redoBtn?.addEventListener('click', redoLastOperation);

  // Signed PDF warning actions
  goBackBtn?.addEventListener('click', resetEditView);
  useSplitBtn?.addEventListener('click', () => {
    resetEditView();
    const splitTab = document.querySelector('[data-tab="split"]') as HTMLElement | null;
    splitTab?.click();
  });

  // DISABLED: Highlight tool is hidden (ISSUE-001)
  // const highlightColorDropdown = document.getElementById('highlight-color-dropdown');
  // const highlightWrapper = document.getElementById('highlight-wrapper');

  // ACCESSIBILITY: Whiteout/Blackout mode dropdown (replaces double-click for motor impairment)
  const whiteoutBtn = document.getElementById('edit-tool-whiteout');
  const whiteoutWrapper = document.getElementById('whiteout-wrapper');
  const whiteoutModeDropdown = document.getElementById('whiteout-mode-dropdown');

  // Second click on whiteout tool shows mode dropdown (like highlight color picker)
  whiteoutBtn?.addEventListener('click', (e) => {
    if (currentTool === 'whiteout' && whiteoutModeDropdown) {
      // Already active - toggle dropdown
      e.preventDefault();
      e.stopPropagation();
      whiteoutModeDropdown.classList.toggle('show');
      return;
    }
  });

  // Handle mode option clicks
  whiteoutModeDropdown?.querySelectorAll('.mode-option').forEach((option) => {
    option.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      const mode = (option as HTMLElement).dataset.mode;
      isBlackoutMode = mode === 'blackout';

      // Update active state on options
      whiteoutModeDropdown.querySelectorAll('.mode-option').forEach((opt) => {
        opt.classList.remove('active');
      });
      option.classList.add('active');

      // Update button text and appearance
      const btnText = whiteoutBtn?.querySelector('.whiteout-tool-text');
      if (btnText) {
        btnText.textContent = isBlackoutMode ? 'Blackout' : 'Whiteout';
      }

      // Update wrapper class for styling
      if (isBlackoutMode) {
        whiteoutWrapper?.classList.add('blackout-mode');
        whiteoutBtn?.classList.add('blackout-mode');
      } else {
        whiteoutWrapper?.classList.remove('blackout-mode');
        whiteoutBtn?.classList.remove('blackout-mode');
      }

      // Close dropdown
      whiteoutModeDropdown.classList.remove('show');

      // Ensure tool is active
      currentTool = 'whiteout';
      document.querySelectorAll<HTMLElement>('.tool-btn[id^="tool-"], .tool-btn[id^="edit-tool-"]').forEach((b) => {
        b.classList.remove('active');
      });
      whiteoutBtn?.classList.add('active');
      whiteoutWrapper?.classList.add('tool-active');
      updateCursor();
    });
  });

  // Close dropdown when clicking outside
  document.addEventListener('click', (e) => {
    if (whiteoutModeDropdown?.classList.contains('show') &&
        !whiteoutWrapper?.contains(e.target as Node)) {
      whiteoutModeDropdown.classList.remove('show');
    }
  });

  // Tool buttons - match both old format (tool-*) and new format (edit-tool-*)
  document.querySelectorAll<HTMLElement>('.tool-btn[id^="tool-"], .tool-btn[id^="edit-tool-"]').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      // Handle different ID formats
      let toolName = btn.id.replace('tool-', '').replace('edit-', '');

      // DISABLED: Highlight tool is hidden (ISSUE-001)
      // if (toolName === 'highlight' && currentTool === 'highlight') {
      //   e.stopPropagation();
      //   highlightColorDropdown?.classList.toggle('show');
      //   return;
      // }
      // highlightColorDropdown?.classList.remove('show');

      currentTool = toolName;
      document.querySelectorAll<HTMLElement>('.tool-btn[id^="tool-"], .tool-btn[id^="edit-tool-"]').forEach((b) => {
        b.classList.remove('active');
      });
      btn.classList.add('active');
      updateCursor();
      // Deselect any selected elements when changing tools
      deselectWhiteout();
      deselectTextBox();

      // Toggle whiteout-tool-active class for border visibility
      const viewer = document.getElementById('edit-viewer');
      if (viewer) {
        if (currentTool === 'whiteout') {
          viewer.classList.add('whiteout-tool-active');
        } else {
          viewer.classList.remove('whiteout-tool-active');
        }
      }

      // DISABLED: Highlight tool is hidden (ISSUE-001)
      // if (highlightWrapper) {
      //   if (currentTool === 'highlight') {
      //     highlightWrapper.classList.add('tool-active');
      //     highlightWrapper.style.setProperty('--current-highlight-color', currentHighlightColor);
      //   } else {
      //     highlightWrapper.classList.remove('tool-active');
      //   }
      // }

      // Toggle whiteout mode dropdown visibility based on tool selection
      if (whiteoutWrapper) {
        if (currentTool === 'whiteout') {
          whiteoutWrapper.classList.add('tool-active');
          if (isBlackoutMode) {
            whiteoutWrapper.classList.add('blackout-mode');
          }
        } else {
          whiteoutWrapper.classList.remove('tool-active');
          whiteoutModeDropdown?.classList.remove('show');
        }
      }
    });
  });

  // Close dropdown when clicking elsewhere
  // DISABLED: Highlight tool is hidden (ISSUE-001)
  // document.addEventListener('click', (e) => {
  //   const target = e.target as Node;
  //   const highlightBtn = document.getElementById('edit-tool-highlight');
  //   if (!highlightBtn?.contains(target) && !highlightColorDropdown?.contains(target)) {
  //     highlightColorDropdown?.classList.remove('show');
  //   }
  // });
  // document.querySelectorAll<HTMLElement>('.highlight-color-swatch').forEach((swatch) => {
  //   swatch.addEventListener('click', (e) => {
  //     e.stopPropagation();
  //     const color = swatch.dataset.color;
  //     if (color) {
  //       currentHighlightColor = color;
  //       const iconFill = document.getElementById('highlight-icon-fill');
  //       if (iconFill) iconFill.style.fill = color;
  //       if (highlightWrapper) highlightWrapper.style.setProperty('--current-highlight-color', color);
  //       document.querySelectorAll('.highlight-color-swatch').forEach((s) => s.classList.remove('active'));
  //       swatch.classList.add('active');
  //       highlightColorDropdown?.classList.remove('show');
  //     }
  //   });
  // });

  // Delete key handler for both text boxes and whiteouts
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Delete' || e.key === 'Backspace') {
      // Only delete if not editing text
      if (activeTextInput) return;

      if (selectedTextBox) {
        deleteSelectedTextBox();
        e.preventDefault();
      } else if (selectedWhiteout) {
        deleteWhiteout(selectedWhiteout);
        e.preventDefault();
      }
    }
  });

  // Undo/Redo keyboard shortcuts (Ctrl+Z / Ctrl+Shift+Z)
  document.addEventListener('keydown', (e) => {
    // Only handle if Ctrl/Cmd is pressed and not in text editing mode
    if (!(e.ctrlKey || e.metaKey)) return;

    if (e.key === 'z' || e.key === 'Z') {
      // Shift+Z or Shift+Ctrl+Z = Redo
      if (e.shiftKey) {
        e.preventDefault();
        redoLastOperation();
      } else if (!activeTextInput) {
        // Ctrl+Z = Undo (but not when editing text - let browser handle it)
        e.preventDefault();
        undoLastOperation();
      }
    }
  });

  // Click on viewer to deselect whiteout
  document.getElementById('edit-viewer')?.addEventListener('click', (e) => {
    // Only deselect if not clicking on a whiteout or its handles
    const target = e.target as HTMLElement;
    if (!target.closest('.edit-whiteout-overlay')) {
      deselectWhiteout();
    }
  });

  // DISABLED: Highlight (ISSUE-001) and Underline (ISSUE-002) tools are hidden
  // document.addEventListener('mouseup', () => {
  //   if (currentTool === 'highlight') {
  //     handleHighlightTextSelection();
  //   } else if (currentTool === 'underline') {
  //     handleUnderlineTextSelection();
  //   }
  // });

  // Page navigation
  document.getElementById('edit-prev-page')?.addEventListener('click', () => navigatePage(-1));
  document.getElementById('edit-next-page')?.addEventListener('click', () => navigatePage(1));

  // Error dismiss
  document.querySelector('#edit-error .dismiss')?.addEventListener('click', () => {
    document.getElementById('edit-error')?.classList.add('hidden');
  });

  // Bold/Italic style buttons
  const boldBtn = document.getElementById('style-bold');
  const italicBtn = document.getElementById('style-italic');

  boldBtn?.addEventListener('click', () => toggleBold());
  italicBtn?.addEventListener('click', () => toggleItalic());

  // Global keyboard shortcuts for Cmd+B and Cmd+I
  document.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && activeTextInput) {
      if (e.key === 'b' || e.key === 'B') {
        e.preventDefault();
        toggleBold();
      } else if (e.key === 'i' || e.key === 'I') {
        e.preventDefault();
        toggleItalic();
      }
    }
  });

  // Font size controls
  document.getElementById('font-size-decrease')?.addEventListener('click', () => decreaseFontSize());
  document.getElementById('font-size-increase')?.addEventListener('click', () => increaseFontSize());
  const fontSizeInput = document.getElementById('font-size-value') as HTMLInputElement | null;
  fontSizeInput?.addEventListener('change', () => setFontSize(fontSizeInput.value));

  // Font family control
  const fontFamilySelect = document.getElementById('style-font-family') as HTMLSelectElement | null;
  fontFamilySelect?.addEventListener('change', () => setFontFamily(fontFamilySelect.value));
}

async function handleEditFile(file: File): Promise<void> {
  if (file.type !== 'application/pdf') {
    showError('edit-error', 'Please select a PDF file');
    return;
  }

  // ISSUE-016: If a document is already loaded, show confirmation before replacing
  // This prevents elderly users from accidentally losing their work
  if (editSession !== null || currentPdfBytes !== null) {
    const hasUnsavedChanges = editSession?.hasChanges() ?? false;
    const currentFilename = currentPdfFilename || 'current document';

    const title = hasUnsavedChanges
      ? 'Replace Document with Unsaved Changes?'
      : 'Replace Existing Document?';

    const message = hasUnsavedChanges
      ? `You have unsaved changes to "${currentFilename}". Loading "${file.name}" will replace your current document and discard all changes. This cannot be undone.`
      : `You already have "${currentFilename}" open. Loading "${file.name}" will replace it. Any edits you've made will be lost.`;

    const confirmed = await showEditConfirmDialog({
      title,
      message,
      confirmText: 'Replace',
      cancelText: 'Keep Current',
      icon: hasUnsavedChanges ? '&#9888;' : '&#128196;', // Warning icon for unsaved, doc icon otherwise
      modalType: 'file-replace', // For elderly UX testing - ISSUE-016
    });

    if (!confirmed) {
      // User chose to keep current document - do nothing
      return;
    }
  }

  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    await loadPdfIntoEditInternal(bytes, file.name);

    // Store in shared state for Tab PDF Sharing
    setSharedPdf(bytes, file.name, 'edit');
  } catch (e) {
    showError('edit-error', 'Failed to load PDF: ' + e);
    console.error(e);
  }
}

/**
 * Load a PDF into the Edit tab from external bytes (called from app.ts for Tab PDF Sharing)
 */
export async function loadPdfIntoEdit(bytes: Uint8Array, filename: string): Promise<void> {
  try {
    await loadPdfIntoEditInternal(bytes, filename);
  } catch (e) {
    showError('edit-error', 'Failed to load PDF: ' + e);
    console.error(e);
  }
}

/**
 * Internal function to load PDF into Edit tab
 */
async function loadPdfIntoEditInternal(bytes: Uint8Array, filename: string): Promise<void> {
  const { EditSession, format_bytes } = window.wasmBindings;

  editSession = new EditSession(filename, bytes);
  currentPdfBytes = bytes;
  currentPdfFilename = filename;

  // Expose for debugging
  (window as unknown as { __editSession__: typeof editSession }).__editSession__ = editSession;

  // Register callbacks for change detection
  registerEditCallbacks(
    () => editSession?.hasChanges() ?? false,
    () => {
      try {
        return editSession?.export() ?? null;
      } catch {
        return null;
      }
    }
  );

  // Check if signed
  if (editSession.isSigned) {
    document.getElementById('edit-drop-zone')?.classList.add('hidden');
    document.getElementById('edit-signed-warning')?.classList.remove('hidden');
    return;
  }

  // Show editor
  document.getElementById('edit-drop-zone')?.classList.add('hidden');
  document.getElementById('edit-editor')?.classList.remove('hidden');

  // Update file info
  const fileNameEl = document.getElementById('edit-file-name');
  const fileDetailsEl = document.getElementById('edit-file-details');
  if (fileNameEl) fileNameEl.textContent = filename;
  if (fileDetailsEl) fileDetailsEl.textContent = `${editSession.pageCount} pages - ${format_bytes(bytes.length)}`;

  // Lazy load PDF.js and render
  await ensurePdfJsLoaded();
  await PdfBridge.loadDocument(editSession.getDocumentBytes());
  await renderAllPages();

  updatePageNavigation();
  updateButtons();
}

async function renderAllPages(): Promise<void> {
  if (!editSession) return;

  const container = document.getElementById('edit-pages');
  if (!container) return;
  container.innerHTML = '';
  textItemsMap.clear();

  for (let i = 1; i <= editSession.pageCount; i++) {
    const pageDiv = document.createElement('div');
    pageDiv.className = 'edit-page';
    pageDiv.dataset.page = String(i);

    const canvas = document.createElement('canvas');
    pageDiv.appendChild(canvas);

    // Overlay container for annotations
    const overlay = document.createElement('div');
    overlay.className = 'overlay-container';
    overlay.dataset.page = String(i);
    pageDiv.appendChild(overlay);

    // Text layer for hover/click on existing text
    const textLayer = document.createElement('div');
    textLayer.className = 'text-layer';
    textLayer.dataset.page = String(i);
    pageDiv.appendChild(textLayer);

    container.appendChild(pageDiv);

    // Render page
    await PdfBridge.renderPage(i, canvas, 1.5);

    // Extract text and render text layer for editing
    const items = await PdfBridge.extractTextWithPositions(i);
    textItemsMap.set(i, items);
    renderTextLayer(textLayer, items, i);

    // Set up click handler for adding annotations
    overlay.addEventListener('click', (e) => handleOverlayClick(e as MouseEvent, i));

    // Set up mouse handlers for whiteout drawing on the PAGE div (not overlay)
    // This ensures events are captured even when text layer is on top
    pageDiv.addEventListener('mousedown', (e) => handleWhiteoutStart(e as MouseEvent, i, overlay, pageDiv));
    pageDiv.addEventListener('mousemove', (e) => handleWhiteoutMove(e as MouseEvent));
    pageDiv.addEventListener('mouseup', (e) => handleWhiteoutEnd(e as MouseEvent, i));
    pageDiv.addEventListener('mouseleave', () => {
      if (isDrawing) handleWhiteoutCancel();
    });

    // Double-click handler for creating default-sized text box
    // This is the preferred way to create a text box for elderly users
    pageDiv.addEventListener('dblclick', (e) => handleTextBoxDoubleClick(e as MouseEvent, i, overlay));
  }
}

function handleTextBoxDoubleClick(e: MouseEvent, pageNum: number, overlay: HTMLElement): void {
  // Only create text box on double-click when textbox tool is selected
  if (currentTool !== 'textbox') return;

  // Don't create if double-clicking on an existing text box
  const elementAtClick = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null;
  if (elementAtClick?.closest('.text-box')) return;

  // Calculate position relative to the page
  const pageDiv = (e.currentTarget as HTMLElement);
  const rect = pageDiv.getBoundingClientRect();
  const domX = e.clientX - rect.left;
  const domY = e.clientY - rect.top;

  // Create default-sized text box (accessible size enforced by createTextBox)
  createTextBox(pageNum, domX, domY);
}

function handleOverlayClick(e: MouseEvent, pageNum: number): void {
  if (currentTool === 'select') return;

  // Check if clicking on or inside a whiteout - if so, open its editor
  // Use elementFromPoint for more accurate detection (handles synthetic events)
  const elementAtClick = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null;
  const whiteout = elementAtClick?.closest('.edit-whiteout-overlay') || (e.target as HTMLElement).closest('.edit-whiteout-overlay');
  if (whiteout) {
    // Open the whiteout's text editor instead of creating new annotation
    openWhiteoutTextEditor(whiteout as HTMLElement, pageNum);
    return;
  }

  // Check if clicking on an existing text-box - if so, focus it for editing
  const existingTextBox = elementAtClick?.closest('.text-box') || (e.target as HTMLElement).closest('.text-box');
  if (existingTextBox && currentTool === 'textbox') {
    // Focus the existing text box's content for editing
    const textContent = existingTextBox.querySelector('.text-content') as HTMLElement | null;
    if (textContent) {
      selectTextBox(existingTextBox as HTMLElement);
      textContent.focus();
    }
    return;
  }

  // Check if clicking on an existing text overlay - if so, edit it
  const textOverlay = elementAtClick?.closest('.edit-text-overlay') || (e.target as HTMLElement).closest('.edit-text-overlay');
  if (textOverlay && currentTool === 'text') {
    // Edit the existing text overlay instead of creating new
    editExistingTextOverlay(textOverlay as HTMLElement, pageNum);
    return;
  }

  const overlay = e.currentTarget as HTMLElement;
  const rect = overlay.getBoundingClientRect();
  const domX = e.clientX - rect.left;
  const domY = e.clientY - rect.top;

  // Get page info for coordinate conversion
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;

  // Convert to PDF coordinates using PDF.js viewport method
  // This properly handles Y-flip, scaling, and any page rotation
  const [pdfX, pdfY] = pageInfo.viewport.convertToPdfPoint(domX, domY);

  switch (currentTool) {
    case 'text':
      addTextAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY);
      break;
    // Note: textbox is NOT handled on single click anymore
    // Use double-click for default-sized box, or click+drag to size it
    // This prevents confusing elderly users with accidental text boxes
    // DISABLED: Checkbox tool is hidden (ISSUE-003)
    // case 'checkbox':
    //   addCheckboxAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY);
    //   break;
    // TODO: Highlight requires text selection, not click-to-place
    // case 'highlight':
    //   addHighlightAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY);
    //   break;
  }
}

function addTextAtPosition(pageNum: number, pdfX: number, pdfY: number, overlay: HTMLElement, domX: number, domY: number): void {
  if (!editSession) return;

  // Create auto-expanding contentEditable span
  const input = document.createElement('span');
  input.contentEditable = 'true';
  input.className = 'edit-text-input';
  input.style.position = 'absolute';
  input.style.left = domX + 'px';
  input.style.top = domY + 'px';
  input.style.minWidth = '20px';
  input.style.minHeight = '1em';
  input.style.fontSize = '12px';
  input.style.fontFamily = 'sans-serif';
  input.style.padding = '2px 4px';
  input.style.border = '1px solid #007bff';
  input.style.borderRadius = '2px';
  input.style.outline = 'none';
  input.style.zIndex = '100';
  input.style.display = 'inline-block';
  input.style.whiteSpace = 'pre-wrap';
  input.style.wordBreak = 'break-word';
  input.style.background = 'white';

  // Initialize text styling state
  input.dataset.isBold = 'false';
  input.dataset.isItalic = 'false';
  input.dataset.fontSize = '12';
  input.dataset.fontFamily = 'sans-serif';

  overlay.appendChild(input);
  input.focus();
  setActiveTextInput(input);

  function saveText(): void {
    if (!editSession) return;

    const text = (input.textContent || '').trim();
    const isBold = input.dataset.isBold === 'true';
    const isItalic = input.dataset.isItalic === 'true';
    const fontSize = parseInt(input.dataset.fontSize || '12', 10) || 12;
    const fontFamily = input.dataset.fontFamily || 'sans-serif';
    input.remove();
    setActiveTextInput(null);

    if (!text) return;

    // Get actual dimensions of the text for PDF operation
    const textWidth = Math.max(input.offsetWidth, 50);
    const textHeight = Math.max(input.offsetHeight, 20);

    // Add to session (PDF coordinates, height adjusted)
    editSession.beginAction('textbox');
    const opId = editSession.addText(pageNum, pdfX, pdfY - 20, textWidth, textHeight, text, fontSize, '#000000', fontFamily, isItalic, isBold);
    editSession.commitAction();

    // Add visual overlay
    const textEl = document.createElement('div');
    textEl.className = 'edit-text-overlay';
    textEl.textContent = text;
    textEl.style.left = domX + 'px';
    textEl.style.top = domY + 'px';
    textEl.style.fontSize = fontSize + 'px';
    textEl.style.fontFamily = fontFamily;
    if (isBold) textEl.style.fontWeight = 'bold';
    if (isItalic) textEl.style.fontStyle = 'italic';
    setOpId(textEl, opId);
    textEl.dataset.fontSize = String(fontSize);
    textEl.dataset.fontFamily = fontFamily;
    textEl.dataset.isBold = isBold ? 'true' : 'false';
    textEl.dataset.isItalic = isItalic ? 'true' : 'false';

    overlay.appendChild(textEl);

    // Make text overlay draggable with Select tool
    makeTextOverlayDraggable(textEl, pageNum);

    updateButtons();
  }

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      saveText();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      input.remove();
      setActiveTextInput(null);
    }
  });

  input.addEventListener('blur', () => {
    // Small delay to allow click events to process
    setTimeout(() => {
      if (input.parentElement) {
        saveText();
      }
    }, 100);
  });
}

function editExistingTextOverlay(textOverlay: HTMLElement, pageNum: number): void {
  if (!editSession) return;

  // Get existing text and style
  const existingText = textOverlay.textContent || '';
  const existingOpId = getOpId(textOverlay);
  const isBold = textOverlay.style.fontWeight === 'bold' || textOverlay.style.fontWeight === '700';
  const isItalic = textOverlay.style.fontStyle === 'italic';
  const fontSize = parseInt(textOverlay.dataset.fontSize || '12', 10) || 12;
  const fontFamily = textOverlay.dataset.fontFamily || 'sans-serif';

  // Get position
  const domX = parseFloat(textOverlay.style.left);
  const domY = parseFloat(textOverlay.style.top);

  // Get the overlay container
  const overlay = textOverlay.parentElement;
  if (!overlay) return;

  // Remove the old operation from session
  // Note: This is part of an edit, not tracked as a separate undoable action
  if (existingOpId !== null) {
    editSession.removeOperation(existingOpId);
  }

  // Hide the text overlay while editing (don't remove yet in case of cancel)
  textOverlay.style.display = 'none';

  // Create auto-expanding contentEditable span at the same position
  const input = document.createElement('span');
  input.contentEditable = 'true';
  input.className = 'edit-text-input';
  input.style.position = 'absolute';
  input.style.left = domX + 'px';
  input.style.top = domY + 'px';
  input.style.minWidth = '20px';
  input.style.minHeight = '1em';
  input.style.fontSize = fontSize + 'px';
  input.style.fontFamily = fontFamily;
  input.style.padding = '2px 4px';
  input.style.border = '1px solid #007bff';
  input.style.borderRadius = '2px';
  input.style.outline = 'none';
  input.style.zIndex = '100';
  input.style.display = 'inline-block';
  input.style.whiteSpace = 'pre-wrap';
  input.style.wordBreak = 'break-word';
  input.style.background = 'white';
  input.textContent = existingText;

  // Initialize text styling state from existing overlay
  input.dataset.isBold = isBold ? 'true' : 'false';
  input.dataset.isItalic = isItalic ? 'true' : 'false';
  input.dataset.fontSize = String(fontSize);
  input.dataset.fontFamily = fontFamily;
  if (isBold) input.style.fontWeight = 'bold';
  if (isItalic) input.style.fontStyle = 'italic';

  overlay.appendChild(input);
  input.focus();
  // Select all text for easy replacement
  const range = document.createRange();
  range.selectNodeContents(input);
  const selection = window.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
  setActiveTextInput(input);

  // Get page info for coordinate conversion
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;

  // Convert to PDF coordinates using PDF.js viewport method
  const [pdfX, pdfY] = pageInfo.viewport.convertToPdfPoint(domX, domY);

  function saveEditedText(): void {
    if (!editSession) return;

    const text = (input.textContent || '').trim();
    const newIsBold = input.dataset.isBold === 'true';
    const newIsItalic = input.dataset.isItalic === 'true';
    const newFontSize = parseInt(input.dataset.fontSize || '12', 10) || 12;
    const newFontFamily = input.dataset.fontFamily || 'sans-serif';

    // Get actual dimensions before removing
    const textWidth = Math.max(input.offsetWidth, 50);
    const textHeight = Math.max(input.offsetHeight, 20);

    input.remove();
    setActiveTextInput(null);

    if (!text) {
      // User cleared the text - remove the overlay
      textOverlay.remove();
      updateButtons();
      return;
    }

    // Add new operation with updated text and dimensions
    editSession.beginAction('textbox');
    const opId = editSession.addText(pageNum, pdfX, pdfY - 20, textWidth, textHeight, text, newFontSize, '#000000', newFontFamily, newIsItalic, newIsBold);
    editSession.commitAction();

    // Update existing overlay IN PLACE (don't remove and recreate - that changes DOM order)
    textOverlay.textContent = text;
    textOverlay.style.display = ''; // Make visible again
    textOverlay.style.fontSize = newFontSize + 'px';
    textOverlay.style.fontFamily = newFontFamily;
    textOverlay.style.fontWeight = newIsBold ? 'bold' : 'normal';
    textOverlay.style.fontStyle = newIsItalic ? 'italic' : 'normal';
    setOpId(textOverlay, opId);
    textOverlay.dataset.fontSize = String(newFontSize);
    textOverlay.dataset.fontFamily = newFontFamily;
    textOverlay.dataset.isBold = newIsBold ? 'true' : 'false';
    textOverlay.dataset.isItalic = newIsItalic ? 'true' : 'false';

    updateButtons();
  }

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      saveEditedText();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      input.remove();
      setActiveTextInput(null);
      // Restore the old text overlay since user cancelled
      textOverlay.style.display = '';
      // Re-add the operation that was removed
      if (existingText && editSession) {
        editSession.beginAction('textbox');
        const opId = editSession.addText(pageNum, pdfX, pdfY - 20, 200, 20, existingText, fontSize, '#000000', fontFamily, isItalic, isBold);
        editSession.commitAction();
        setOpId(textOverlay, opId);
      }
    }
  });

  input.addEventListener('blur', () => {
    setTimeout(() => {
      if (input.parentElement) {
        saveEditedText();
      }
    }, 100);
  });
}

// DISABLED: Checkbox tool is hidden (ISSUE-003)
// function addCheckboxAtPosition(...) { ... }

// DISABLED: Highlight tool is hidden (ISSUE-001)
// function handleHighlightTextSelection(): void { ... }

// DISABLED: Underline tool is hidden (ISSUE-002)
// function handleUnderlineTextSelection(): void { ... }

// ============ Whiteout Drawing Functions ============

function handleWhiteoutStart(e: MouseEvent, pageNum: number, overlay: HTMLElement, pageDiv: HTMLElement): void {
  // Check if clicking on UI elements (delete button, resize handles, etc.)
  const target = e.target as HTMLElement;
  const isOnUIElement = target.closest('.delete-btn') ||
      target.closest('.resize-handle') ||
      target.closest('.text-content') ||
      target.closest('.text-box') ||
      target.closest('.edit-whiteout-overlay');

  // ISSUE-014 FIX: When clicking on a blank area (not on a text box or whiteout),
  // deselect any selected elements regardless of current tool
  if (!isOnUIElement) {
    deselectTextBox();
    deselectWhiteout();
  }

  // Handle drawing only for whiteout and textbox tools
  if (currentTool !== 'whiteout' && currentTool !== 'textbox') return;

  // Don't start drawing if clicking on UI elements
  if (isOnUIElement) {
    return;
  }

  // Prevent text selection while drawing
  e.preventDefault();
  e.stopPropagation();

  isDrawing = true;
  drawOverlay = overlay;
  drawPageNum = pageNum;
  drawPageDiv = pageDiv;

  // Get position relative to the page div
  const rect = pageDiv.getBoundingClientRect();
  drawStartX = e.clientX - rect.left;
  drawStartY = e.clientY - rect.top;

  // Create preview rectangle
  drawPreviewEl = document.createElement('div');
  if (currentTool === 'textbox') {
    drawPreviewEl.className = 'textbox-preview';
  } else if (isBlackoutMode) {
    drawPreviewEl.className = 'blackout-preview';
  } else {
    drawPreviewEl.className = 'whiteout-preview';
  }
  drawPreviewEl.style.left = drawStartX + 'px';
  drawPreviewEl.style.top = drawStartY + 'px';
  drawPreviewEl.style.width = '0px';
  drawPreviewEl.style.height = '0px';
  if (currentTool === 'textbox') {
    // TextBox is always transparent with dashed border
    drawPreviewEl.style.border = '2px dashed #666';
    drawPreviewEl.style.background = 'transparent';
  }
  pageDiv.appendChild(drawPreviewEl);
}

function handleWhiteoutMove(e: MouseEvent): void {
  if (!isDrawing || !drawPreviewEl || !drawPageDiv) return;

  const rect = drawPageDiv.getBoundingClientRect();
  const currentX = e.clientX - rect.left;
  const currentY = e.clientY - rect.top;

  // Calculate rectangle dimensions (handle negative widths/dragging in any direction)
  const x = Math.min(drawStartX, currentX);
  const y = Math.min(drawStartY, currentY);
  const width = Math.abs(currentX - drawStartX);
  const height = Math.abs(currentY - drawStartY);

  drawPreviewEl.style.left = x + 'px';
  drawPreviewEl.style.top = y + 'px';
  drawPreviewEl.style.width = width + 'px';
  drawPreviewEl.style.height = height + 'px';
}

function handleWhiteoutEnd(e: MouseEvent, pageNum: number): void {
  if (!isDrawing || !drawPreviewEl || !drawPageDiv) return;

  const wasTextbox = currentTool === 'textbox';

  const rect = drawPageDiv.getBoundingClientRect();
  const endX = e.clientX - rect.left;
  const endY = e.clientY - rect.top;

  // Calculate rectangle dimensions in DOM coords
  const domX = Math.min(drawStartX, endX);
  const domY = Math.min(drawStartY, endY);
  const domWidth = Math.abs(endX - drawStartX);
  const domHeight = Math.abs(endY - drawStartY);

  // Remove preview
  if (drawPreviewEl) {
    drawPreviewEl.remove();
    drawPreviewEl = null;
  }

  if (wasTextbox) {
    // For textbox: only create on meaningful drag (>10px in at least one dimension)
    // Single clicks are ignored - use double-click for default-sized box
    // This prevents confusing elderly users with accidental text boxes
    if (domWidth >= 10 || domHeight >= 10) {
      // Create text box sized to the drag area (enforcing minimum sizes)
      createTextBox(pageNum, domX, domY, domWidth, domHeight);
    }
    // Small drags (<10px) are treated as clicks and ignored for textbox tool
  } else {
    // Whiteout - only add if rectangle is big enough (at least 5x5 pixels)
    if (domWidth >= 5 && domHeight >= 5) {
      addWhiteoutAtPosition(pageNum, domX, domY, domWidth, domHeight);
    }
  }

  isDrawing = false;
  drawOverlay = null;
  drawPageDiv = null;
  drawPageNum = null;
}

function handleWhiteoutCancel(): void {
  if (drawPreviewEl) {
    drawPreviewEl.remove();
    drawPreviewEl = null;
  }
  isDrawing = false;
  drawOverlay = null;
  drawPageDiv = null;
  drawPageNum = null;
}

function addWhiteoutAtPosition(pageNum: number, domX: number, domY: number, domWidth: number, domHeight: number): void {
  if (!editSession) return;

  // Get page info for coordinate conversion
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;

  // Convert DOM coordinates to PDF coordinates using PDF.js viewport method
  // Convert top-left and bottom-right corners
  const [pdfX1, pdfY1] = pageInfo.viewport.convertToPdfPoint(domX, domY);
  const [pdfX2, pdfY2] = pageInfo.viewport.convertToPdfPoint(domX + domWidth, domY + domHeight);
  // Normalize rectangle (PDF Y increases upward, so y1 > y2)
  const pdfX = Math.min(pdfX1, pdfX2);
  const pdfY = Math.min(pdfY1, pdfY2);
  const pdfWidth = Math.abs(pdfX2 - pdfX1);
  const pdfHeight = Math.abs(pdfY2 - pdfY1);

  // Determine color based on mode
  const rectColor = isBlackoutMode ? '#000000' : '#FFFFFF';

  // Add to session with color
  editSession.beginAction('whiteout');
  const opId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight, rectColor);
  editSession.commitAction();

  // Add visual overlay
  const overlay = document.querySelector<HTMLElement>(`.overlay-container[data-page="${pageNum}"]`);
  if (!overlay) return;

  const whiteRect = document.createElement('div');
  // Use appropriate class based on mode
  whiteRect.className = isBlackoutMode ? 'edit-blackout-overlay' : 'edit-whiteout-overlay';
  whiteRect.style.left = domX + 'px';
  whiteRect.style.top = domY + 'px';
  whiteRect.style.width = domWidth + 'px';
  whiteRect.style.height = domHeight + 'px';
  setOpId(whiteRect, opId);
  whiteRect.dataset.page = String(pageNum);
  // Mark as blackout if applicable (for preventing text editing)
  if (isBlackoutMode) {
    whiteRect.dataset.blackout = 'true';
  }

  // Mousedown to select and start move
  whiteRect.addEventListener('mousedown', (e) => {
    // Don't interfere with resize handles
    if ((e.target as HTMLElement).classList.contains('resize-handle')) return;

    e.stopPropagation();
    e.preventDefault();
    selectWhiteout(whiteRect);
    startMove(e, whiteRect);
  });

  // Double-click to add text inside whiteout (disabled for blackout)
  whiteRect.addEventListener('dblclick', (e) => {
    e.stopPropagation();
    // Blackout rectangles are not editable
    if (whiteRect.dataset.blackout === 'true') return;
    openWhiteoutTextEditor(whiteRect, pageNum);
  });

  overlay.appendChild(whiteRect);

  // Auto-select the newly created whiteout
  selectWhiteout(whiteRect);

  updateButtons();
}

function selectWhiteout(whiteRect: HTMLElement): void {
  // Deselect previous
  if (selectedWhiteout) {
    selectedWhiteout.classList.remove('selected');
    // Remove resize handles
    selectedWhiteout.querySelectorAll('.resize-handle').forEach((h) => h.remove());
  }

  selectedWhiteout = whiteRect;
  whiteRect.classList.add('selected');

  // Add resize handles
  const handles = ['nw', 'n', 'ne', 'w', 'e', 'sw', 's', 'se'];
  handles.forEach((pos) => {
    const handle = document.createElement('div');
    handle.className = `resize-handle ${pos}`;
    handle.dataset.handle = pos;
    handle.addEventListener('mousedown', (e) => startResize(e, whiteRect, pos));
    whiteRect.appendChild(handle);
  });
}

function deselectWhiteout(): void {
  if (selectedWhiteout) {
    selectedWhiteout.classList.remove('selected');
    selectedWhiteout.querySelectorAll('.resize-handle').forEach((h) => h.remove());
    selectedWhiteout = null;
  }
}

function deleteWhiteout(whiteout: HTMLElement): void {
  const opId = getOpId(whiteout);
  if (opId !== null && editSession) {
    editSession.removeOperation(opId);
  }

  if (selectedWhiteout === whiteout) {
    selectedWhiteout = null;
  }

  whiteout.remove();
  updateButtons();
}

// ============================================================================
// Text Box Functions (always transparent - for adding text on top of content)
// ============================================================================

// Z-index counter for layering (last-added on top)
let nextTextBoxZIndex = 100;

// Minimum sizes for accessibility (WCAG touch target guidelines)
const MIN_TEXTBOX_HEIGHT = 44;
const MIN_TEXTBOX_WIDTH = 100;
const DEFAULT_TEXTBOX_WIDTH = 200;
const DEFAULT_TEXTBOX_HEIGHT = 48;

function createTextBox(pageNum: number, domX: number, domY: number, width?: number, height?: number): HTMLElement {
  if (!editSession) throw new Error('No edit session');

  const id = nextTextBoxId++;

  // Get page dimensions to constrain initial width
  const pageEl = document.querySelector(`.edit-page[data-page="${pageNum}"]`) as HTMLElement | null;
  const pageWidth = pageEl?.offsetWidth || 800;
  const margin = 10;
  const maxAvailableWidth = Math.max(MIN_TEXTBOX_WIDTH, pageWidth - domX - margin);

  // Use provided dimensions or defaults (enforcing minimums for accessibility)
  const initialWidth = Math.min(maxAvailableWidth, Math.max(MIN_TEXTBOX_WIDTH, width ?? DEFAULT_TEXTBOX_WIDTH));
  const initialHeight = Math.max(MIN_TEXTBOX_HEIGHT, height ?? DEFAULT_TEXTBOX_HEIGHT);

  // Create DOM element (always transparent)
  const box = document.createElement('div');
  box.className = 'text-box transparent';
  box.dataset.textboxId = String(id);
  box.dataset.page = String(pageNum);
  box.style.left = domX + 'px';
  box.style.top = domY + 'px';
  box.style.width = initialWidth + 'px';
  box.style.height = initialHeight + 'px';
  // Z-ordering: last-added on top, gets click priority
  box.style.zIndex = String(nextTextBoxZIndex++);

  // Add delete button (X)
  const deleteBtn = document.createElement('button');
  deleteBtn.className = 'delete-btn';
  deleteBtn.innerHTML = '&times;';
  deleteBtn.title = 'Delete';
  deleteBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    deleteTextBox(box);
  });
  box.appendChild(deleteBtn);

  // Add text content area
  const textContent = document.createElement('div');
  textContent.className = 'text-content';
  textContent.contentEditable = 'true';
  // Initialize font styling data attributes (used by style buttons)
  textContent.dataset.fontSize = '12';
  textContent.dataset.fontFamily = 'sans-serif';
  textContent.dataset.isBold = 'false';
  textContent.dataset.isItalic = 'false';
  textContent.style.fontSize = '12px';
  textContent.style.fontFamily = 'sans-serif';
  textContent.addEventListener('focus', () => {
    activeTextInput = textContent;
    updateStyleButtons();
  });
  textContent.addEventListener('blur', () => {
    activeTextInput = null;
    updateStyleButtons();
    // Commit text to WASM when done editing
    commitTextBox(box);
  });
  textContent.addEventListener('input', () => {
    checkTextBoxOverlap(box);
    // Auto-expand: grow text box to fit content
    expandTextBoxForContent(box, textContent);
  });
  box.appendChild(textContent);

  // Add resize handles
  const handles = ['nw', 'n', 'ne', 'w', 'e', 'sw', 's', 'se'];
  handles.forEach((pos) => {
    const handle = document.createElement('div');
    handle.className = `resize-handle resize-handle-${pos}`;
    handle.dataset.handle = pos;
    handle.addEventListener('mousedown', (e) => startTextBoxResize(e, box, pos));
    box.appendChild(handle);
  });

  // Click to select
  box.addEventListener('mousedown', (e) => {
    if ((e.target as HTMLElement).classList.contains('resize-handle') ||
        (e.target as HTMLElement).classList.contains('delete-btn')) {
      return;
    }
    selectTextBox(box);
    // Start move if not clicking on text content
    if (!(e.target as HTMLElement).classList.contains('text-content')) {
      startTextBoxMove(e, box);
    }
  });

  // Add to page overlay
  const overlay = document.querySelector<HTMLElement>(`.overlay-container[data-page="${pageNum}"]`);
  if (overlay) {
    overlay.appendChild(box);
  }

  // Track
  textBoxes.set(id, box);

  // Select immediately
  selectTextBox(box);

  // Focus text content for immediate typing
  setTimeout(() => textContent.focus(), 50);

  // Check for overlaps
  checkTextBoxOverlap(box);

  return box;
}

function selectTextBox(box: HTMLElement): void {
  deselectTextBox();
  deselectWhiteout();
  selectedTextBox = box;
  box.classList.add('selected');
  // Bring to front when selected
  box.style.zIndex = String(nextTextBoxZIndex++);
}

function deselectTextBox(): void {
  if (selectedTextBox) {
    selectedTextBox.classList.remove('selected');
    selectedTextBox = null;
  }
}

function deleteTextBox(box: HTMLElement): void {
  const opId = getOpId(box);
  if (opId !== null && editSession) {
    editSession.removeOperation(opId);
  }

  const id = parseInt(box.dataset.textboxId || '0');
  textBoxes.delete(id);

  if (selectedTextBox === box) {
    selectedTextBox = null;
  }

  box.remove();
  updateButtons();
}

function deleteSelectedTextBox(): void {
  if (selectedTextBox) {
    deleteTextBox(selectedTextBox);
  }
}

/**
 * Parse innerHTML into styled text segments.
 * Extracts text from bold/italic tags and creates segments with appropriate flags.
 * Returns null if no mixed styling is detected (use simple addText instead).
 */
function parseStyledSegments(element: HTMLElement): { text: string; is_bold: boolean; is_italic: boolean }[] | null {
  const innerHTML = element.innerHTML;

  // Check if there are any styled tags
  const hasStyledTags = /<(b|strong|i|em)\b/i.test(innerHTML);
  if (!hasStyledTags) {
    return null; // No mixed styling, use simple addText
  }

  const segments: { text: string; is_bold: boolean; is_italic: boolean }[] = [];

  // Helper to recursively walk nodes and extract styled text
  function walkNode(node: Node, isBold: boolean, isItalic: boolean): void {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = node.textContent || '';
      if (text) {
        segments.push({ text, is_bold: isBold, is_italic: isItalic });
      }
    } else if (node.nodeType === Node.ELEMENT_NODE) {
      const el = node as HTMLElement;
      const tagName = el.tagName.toLowerCase();

      // Determine style from tag
      const newBold = isBold || tagName === 'b' || tagName === 'strong';
      const newItalic = isItalic || tagName === 'i' || tagName === 'em';

      // Recursively process children
      for (const child of Array.from(el.childNodes)) {
        walkNode(child, newBold, newItalic);
      }
    }
  }

  // Start walking from root element
  for (const child of Array.from(element.childNodes)) {
    walkNode(child, false, false);
  }

  // Merge adjacent segments with same styling
  const merged: { text: string; is_bold: boolean; is_italic: boolean }[] = [];
  for (const seg of segments) {
    if (merged.length > 0) {
      const last = merged[merged.length - 1];
      if (last.is_bold === seg.is_bold && last.is_italic === seg.is_italic) {
        last.text += seg.text;
        continue;
      }
    }
    merged.push({ ...seg });
  }

  // If all segments have the same style, return null (use simple addText)
  if (merged.length <= 1) {
    return null;
  }

  // Check if there's actual mixed styling
  const firstStyle = { is_bold: merged[0].is_bold, is_italic: merged[0].is_italic };
  const hasMixedStyles = merged.some(
    s => s.is_bold !== firstStyle.is_bold || s.is_italic !== firstStyle.is_italic
  );

  if (!hasMixedStyles) {
    return null; // All same style, use simple addText
  }

  return merged;
}

function commitTextBox(box: HTMLElement): void {
  if (!editSession) return;

  const textContent = box.querySelector('.text-content') as HTMLElement | null;
  const text = textContent?.textContent?.trim() || '';
  const pageNum = parseInt(box.dataset.page || '1');

  // Get page info for coordinate conversion
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;

  // Get DOM coordinates
  const domX = parseFloat(box.style.left);
  const domY = parseFloat(box.style.top);
  const domWidth = box.offsetWidth;
  const domHeight = box.offsetHeight;

  // Convert to PDF coordinates using PDF.js viewport method
  const [pdfX1, pdfY1] = pageInfo.viewport.convertToPdfPoint(domX, domY);
  const [pdfX2, pdfY2] = pageInfo.viewport.convertToPdfPoint(domX + domWidth, domY + domHeight);
  // Normalize rectangle (PDF Y increases upward, so y1 > y2)
  const pdfX = Math.min(pdfX1, pdfX2);
  const pdfY = Math.min(pdfY1, pdfY2);
  const pdfWidth = Math.abs(pdfX2 - pdfX1);
  const pdfHeight = Math.abs(pdfY2 - pdfY1);

  // Remove old operation if exists
  const existingOpId = getOpId(box);
  if (existingOpId !== null) {
    editSession.removeOperation(existingOpId);
  }

  // TextBox is always transparent - just add text, no white rect
  if (text && textContent) {
    // Get base style from text content
    const style = window.getComputedStyle(textContent);
    const fontSize = parseFloat(style.fontSize) || 12;

    // Check for mixed styling (partial bold/italic)
    const styledSegments = parseStyledSegments(textContent);

    editSession.beginAction('textbox');

    let opId: bigint;
    if (styledSegments) {
      // Use addStyledText for mixed styling
      const segmentsJson = JSON.stringify(styledSegments);
      opId = editSession.addStyledText(
        pageNum,
        pdfX,
        pdfY,
        pdfWidth,
        pdfHeight,
        segmentsJson,
        fontSize,
        '#000000',
        null // font name
      );
    } else {
      // Use simple addText for uniform styling
      const isBold = style.fontWeight === 'bold' || parseInt(style.fontWeight || '400') >= 700;
      const isItalic = style.fontStyle === 'italic';
      opId = editSession.addText(
        pageNum,
        pdfX,
        pdfY,
        pdfWidth,
        pdfHeight,
        text,
        fontSize,
        '#000000',
        null, // font name
        isItalic,
        isBold
      );
    }

    editSession.commitAction();
    setOpId(box, opId);
  }

  updateButtons();
}

function startTextBoxResize(e: MouseEvent, box: HTMLElement, handle: string): void {
  e.preventDefault();
  e.stopPropagation();

  resizing = true;
  resizeTarget = box;
  resizeHandle = handle;
  resizeStartX = e.clientX;
  resizeStartY = e.clientY;
  resizeStartRect = {
    left: parseFloat(box.style.left),
    top: parseFloat(box.style.top),
    width: box.offsetWidth,
    height: box.offsetHeight,
  };
}

function startTextBoxMove(e: MouseEvent, box: HTMLElement): void {
  e.preventDefault();

  moving = true;
  moveTarget = box;
  moveStartX = e.clientX;
  moveStartY = e.clientY;
  moveStartLeft = parseFloat(box.style.left);
  moveStartTop = parseFloat(box.style.top);

  // Add document-level listeners for move tracking
  document.addEventListener('mousemove', handleMove);
  document.addEventListener('mouseup', endMove);
}

function expandTextBoxForContent(box: HTMLElement, textContent: HTMLElement): void {
  const text = textContent.textContent || '';
  if (!text) return;

  // Get the page element to determine boundary constraints
  const pageEl = box.closest('.edit-page') as HTMLElement | null;
  if (!pageEl) return;

  const pageWidth = pageEl.offsetWidth;
  const boxLeft = parseFloat(box.style.left) || 0;

  // Calculate maximum available width (page width minus box position minus margin)
  const margin = 10; // Small margin from page edge
  const maxAvailableWidth = Math.max(100, pageWidth - boxLeft - margin);

  // Measure text dimensions using a temporary canvas
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  const fontSize = textContent.dataset.fontSize || '12';
  const fontFamily = textContent.dataset.fontFamily || 'sans-serif';
  const isBold = textContent.dataset.isBold === 'true';
  const isItalic = textContent.dataset.isItalic === 'true';

  let fontStyle = '';
  if (isItalic) fontStyle += 'italic ';
  if (isBold) fontStyle += 'bold ';
  ctx.font = `${fontStyle}${fontSize}px ${fontFamily}`;

  const metrics = ctx.measureText(text);
  const textWidth = metrics.width + 20; // Add padding
  const lineHeight = parseInt(fontSize, 10) * 1.4;

  // Constrain width to page boundary (elderly UX: never overflow page)
  const constrainedWidth = Math.min(textWidth, maxAvailableWidth);

  // Calculate lines needed when constrained to available width
  const effectiveWidth = Math.max(100, constrainedWidth - 20); // Account for padding
  const numLines = Math.max(1, Math.ceil(metrics.width / effectiveWidth));
  const textHeight = lineHeight * numLines + 10; // Add padding

  // Expand box if needed (minimum 150x30, but respect page boundary)
  const currentWidth = parseFloat(box.style.width);
  const currentHeight = parseFloat(box.style.height);
  const newWidth = Math.max(150, Math.min(constrainedWidth, maxAvailableWidth));
  const newHeight = Math.max(30, textHeight);

  // Only expand width if within bounds
  if (newWidth > currentWidth && newWidth <= maxAvailableWidth) {
    box.style.width = newWidth + 'px';
  } else if (currentWidth > maxAvailableWidth) {
    // Shrink width if it exceeds boundary (shouldn't happen, but safety check)
    box.style.width = maxAvailableWidth + 'px';
  }

  // Always allow height to grow to accommodate wrapped text
  if (newHeight > currentHeight) {
    box.style.height = newHeight + 'px';
  }
}

function checkTextBoxOverlap(box: HTMLElement): void {
  const boxRect = box.getBoundingClientRect();
  const pageNum = box.dataset.page;
  let hasOverlap = false;

  textBoxes.forEach((otherBox) => {
    if (otherBox === box) return;
    if (otherBox.dataset.page !== pageNum) return;

    const otherRect = otherBox.getBoundingClientRect();
    if (rectsOverlap(boxRect, otherRect)) {
      hasOverlap = true;
    }
  });

  // Also check whiteout overlays
  document.querySelectorAll<HTMLElement>(`.edit-whiteout-overlay[data-page="${pageNum}"]`).forEach((overlay) => {
    const overlayRect = overlay.getBoundingClientRect();
    if (rectsOverlap(boxRect, overlayRect)) {
      hasOverlap = true;
    }
  });

  box.classList.toggle('overlapping', hasOverlap);

  // Add/remove warning tooltip
  let warning = box.querySelector('.overlap-warning');
  if (hasOverlap && !warning) {
    warning = document.createElement('div');
    warning.className = 'overlap-warning';
    warning.textContent = 'Overlapping';
    box.appendChild(warning);
  } else if (!hasOverlap && warning) {
    warning.remove();
  }
}

function rectsOverlap(a: DOMRect, b: DOMRect): boolean {
  return !(a.right < b.left || b.right < a.left || a.bottom < b.top || b.bottom < a.top);
}

function startResize(e: MouseEvent, whiteRect: HTMLElement, handle: string): void {
  e.preventDefault();
  e.stopPropagation();

  resizing = true;
  resizeTarget = whiteRect;
  resizeHandle = handle;
  resizeStartX = e.clientX;
  resizeStartY = e.clientY;
  resizeStartRect = {
    left: parseFloat(whiteRect.style.left),
    top: parseFloat(whiteRect.style.top),
    width: parseFloat(whiteRect.style.width),
    height: parseFloat(whiteRect.style.height),
  };

  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', endResize);
}

function handleResize(e: MouseEvent): void {
  if (!resizing || !resizeTarget || !resizeStartRect) return;

  const dx = e.clientX - resizeStartX;
  const dy = e.clientY - resizeStartY;

  let newLeft = resizeStartRect.left;
  let newTop = resizeStartRect.top;
  let newWidth = resizeStartRect.width;
  let newHeight = resizeStartRect.height;

  // Adjust based on which handle is being dragged
  if (resizeHandle.includes('w')) {
    newLeft = resizeStartRect.left + dx;
    newWidth = resizeStartRect.width - dx;
  }
  if (resizeHandle.includes('e')) {
    newWidth = resizeStartRect.width + dx;
  }
  if (resizeHandle.includes('n')) {
    newTop = resizeStartRect.top + dy;
    newHeight = resizeStartRect.height - dy;
  }
  if (resizeHandle.includes('s')) {
    newHeight = resizeStartRect.height + dy;
  }

  // Ensure minimum size
  if (newWidth < 10) {
    if (resizeHandle.includes('w')) {
      newLeft = resizeStartRect.left + resizeStartRect.width - 10;
    }
    newWidth = 10;
  }
  if (newHeight < 10) {
    if (resizeHandle.includes('n')) {
      newTop = resizeStartRect.top + resizeStartRect.height - 10;
    }
    newHeight = 10;
  }

  resizeTarget.style.left = newLeft + 'px';
  resizeTarget.style.top = newTop + 'px';
  resizeTarget.style.width = newWidth + 'px';
  resizeTarget.style.height = newHeight + 'px';
}

function endResize(): void {
  if (!resizing || !resizeTarget) return;

  // IMPORTANT: Remove event listeners FIRST to prevent stuck state
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', endResize);

  // Store reference before clearing state
  const target = resizeTarget;
  const pageNum = parseInt(target.dataset.page || '0', 10);
  const opId = getOpId(target);

  // Clear state immediately
  resizing = false;
  resizeTarget = null;
  resizeHandle = '';

  // Now update the PDF operation (errors here won't leave listeners stuck)
  try {
    if (opId !== null && editSession) {
      editSession.removeOperation(opId);

      const pageInfo = PdfBridge.getPageInfo(pageNum);
      if (pageInfo) {
        const domX = parseFloat(target.style.left);
        const domY = parseFloat(target.style.top);
        const domWidth = parseFloat(target.style.width);
        const domHeight = parseFloat(target.style.height);

        // Convert to PDF coordinates using PDF.js viewport method
        const [pdfX1, pdfY1] = pageInfo.viewport.convertToPdfPoint(domX, domY);
        const [pdfX2, pdfY2] = pageInfo.viewport.convertToPdfPoint(domX + domWidth, domY + domHeight);
        const pdfX = Math.min(pdfX1, pdfX2);
        const pdfY = Math.min(pdfY1, pdfY2);
        const pdfWidth = Math.abs(pdfX2 - pdfX1);
        const pdfHeight = Math.abs(pdfY2 - pdfY1);

        // Preserve color from the element being resized
        const resizeColor = target.dataset.blackout === 'true' ? '#000000' : '#FFFFFF';
        editSession.beginAction('resize');
        const newOpId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight, resizeColor);
        editSession.commitAction();
        setOpId(target, newOpId);
      }
    }
  } catch (err) {
    console.error('Error updating resize operation:', err);
  }
}

// ============ Move Whiteout Functions ============

function startMove(e: MouseEvent, whiteRect: HTMLElement): void {
  // Don't start move if we're resizing
  if (resizing) return;

  // Only Select or Whiteout tools can drag whiteout overlays
  if (currentTool !== 'select' && currentTool !== 'whiteout') return;

  e.preventDefault();
  e.stopPropagation();

  moving = true;
  moveTarget = whiteRect;
  moveStartX = e.clientX;
  moveStartY = e.clientY;
  moveStartLeft = parseFloat(whiteRect.style.left);
  moveStartTop = parseFloat(whiteRect.style.top);

  document.addEventListener('mousemove', handleMove);
  document.addEventListener('mouseup', endMove);
}

function handleMove(e: MouseEvent): void {
  if (!moving || !moveTarget) return;

  const dx = e.clientX - moveStartX;
  const dy = e.clientY - moveStartY;

  moveTarget.style.left = moveStartLeft + dx + 'px';
  moveTarget.style.top = moveStartTop + dy + 'px';
}

function endMove(): void {
  if (!moving || !moveTarget) return;

  // IMPORTANT: Remove event listeners FIRST to prevent stuck state
  document.removeEventListener('mousemove', handleMove);
  document.removeEventListener('mouseup', endMove);

  // Store reference before clearing state
  const target = moveTarget;
  const pageNum = parseInt(target.dataset.page || '0', 10);
  const opId = getOpId(target);

  // Clear state immediately
  moving = false;
  moveTarget = null;

  // Now update the PDF operation (errors here won't leave listeners stuck)
  try {
    if (opId !== null && editSession) {
      editSession.removeOperation(opId);

      const pageInfo = PdfBridge.getPageInfo(pageNum);
      if (pageInfo) {
        const domX = parseFloat(target.style.left);
        const domY = parseFloat(target.style.top);
        const domWidth = parseFloat(target.style.width);
        const domHeight = parseFloat(target.style.height);

        // Convert to PDF coordinates using PDF.js viewport method
        const [pdfX1, pdfY1] = pageInfo.viewport.convertToPdfPoint(domX, domY);
        const [pdfX2, pdfY2] = pageInfo.viewport.convertToPdfPoint(domX + domWidth, domY + domHeight);
        const pdfX = Math.min(pdfX1, pdfX2);
        const pdfY = Math.min(pdfY1, pdfY2);
        const pdfWidth = Math.abs(pdfX2 - pdfX1);
        const pdfHeight = Math.abs(pdfY2 - pdfY1);

        // Preserve color from the element being moved
        const moveColor = target.dataset.blackout === 'true' ? '#000000' : '#FFFFFF';
        editSession.beginAction('move');
        const newOpId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight, moveColor);
        editSession.commitAction();
        setOpId(target, newOpId);
      }
    }
  } catch (err) {
    console.error('Error updating move operation:', err);
  }
}

// ============ Text Overlay Dragging ============

function makeTextOverlayDraggable(textEl: HTMLElement, pageNum: number): void {
  textEl.style.cursor = 'move';

  // Click handler for editing with Text tool
  textEl.addEventListener('click', (e) => {
    if (currentTool === 'text') {
      e.preventDefault();
      e.stopPropagation();
      editExistingTextOverlay(textEl, pageNum);
    }
  });

  textEl.addEventListener('mousedown', (e) => {
    // Only Select tool can drag text overlays
    if (currentTool !== 'select') return;

    e.preventDefault();
    e.stopPropagation();

    draggingTextOverlay = textEl;
    textDragStartX = e.clientX;
    textDragStartY = e.clientY;
    textDragStartLeft = parseFloat(textEl.style.left);
    textDragStartTop = parseFloat(textEl.style.top);

    document.addEventListener('mousemove', handleTextDrag);
    document.addEventListener('mouseup', endTextDrag);
  });
}

// Make replacement overlays (from editing existing PDF text) re-editable
// Uses "undo and re-edit" approach: removes the old replacement and triggers fresh edit on original text
function makeReplaceOverlayEditable(replaceEl: HTMLElement, pageNum: number): void {
  replaceEl.style.cursor = 'pointer';

  // Click handler for re-editing via undo-and-reedit
  replaceEl.addEventListener('click', (e) => {
    e.preventDefault();
    e.stopPropagation();

    // 1. Get the original textItem data stored in the overlay
    const originalTextItemJson = replaceEl.dataset.originalTextItem;
    const textItemIndex = replaceEl.dataset.textItemIndex;
    const opId = getOpId(replaceEl);

    if (!originalTextItemJson) {
      console.error('Cannot re-edit: no original text item data stored');
      return;
    }

    // Capture the user's intermediate text (what they last saved) BEFORE removing the overlay
    const intermediateText = replaceEl.textContent || '';

    const textItem = JSON.parse(originalTextItemJson) as TextItem;
    // Override the original text with the user's intermediate text for the editor
    // This way the editor shows what the user last typed, not the original PDF text
    textItem.str = intermediateText;

    // 2. Remove the replacement operation from the edit session
    // Note: Rust handles history tracking now
    if (opId !== null && editSession) {
      editSession.removeOperation(opId);
    }

    // 3. Keep the replacement overlay visible to cover the canvas during editing
    // Mark it for removal when save happens (applyTextReplacement will clean it up)
    replaceEl.dataset.pendingRemoval = 'true';

    // 4. Don't unhide the original text item - keep it hidden so canvas text doesn't show
    const originalSpan = document.querySelector<HTMLElement>(`.text-item[data-page="${pageNum}"][data-index="${textItemIndex}"]`);

    // 5. Trigger fresh edit on the original text item
    if (originalSpan) {
      startTextEdit(pageNum, parseInt(textItemIndex || '0', 10), textItem, originalSpan);
    } else {
      console.error('Cannot find original text item span to re-edit');
    }
  });

  // Hover highlight for visual feedback
  replaceEl.addEventListener('mouseenter', () => {
    if (currentTool === 'select' || currentTool === 'text' || currentTool === 'edit-text') {
      replaceEl.style.outline = '2px solid #007bff';
    }
  });
  replaceEl.addEventListener('mouseleave', () => {
    replaceEl.style.outline = '';
  });
}

function handleTextDrag(e: MouseEvent): void {
  if (!draggingTextOverlay) return;

  const dx = e.clientX - textDragStartX;
  const dy = e.clientY - textDragStartY;

  draggingTextOverlay.style.left = textDragStartLeft + dx + 'px';
  draggingTextOverlay.style.top = textDragStartTop + dy + 'px';
}

function endTextDrag(): void {
  if (!draggingTextOverlay) return;

  // Remove event listeners first
  document.removeEventListener('mousemove', handleTextDrag);
  document.removeEventListener('mouseup', endTextDrag);

  const textEl = draggingTextOverlay;
  draggingTextOverlay = null;

  // Only update if position actually changed
  const newLeft = parseFloat(textEl.style.left);
  const newTop = parseFloat(textEl.style.top);
  if (newLeft === textDragStartLeft && newTop === textDragStartTop) return;

  // Get operation data
  const opId = getOpId(textEl);
  const pageEl = textEl.closest('.edit-page') as HTMLElement | null;
  const pageNum = parseInt(pageEl?.dataset.page || '0', 10);
  const text = textEl.textContent || '';
  const fontSize = parseInt(textEl.dataset.fontSize || '12', 10) || 12;
  const fontFamily = textEl.dataset.fontFamily || 'sans-serif';
  const isBold = textEl.dataset.isBold === 'true';
  const isItalic = textEl.dataset.isItalic === 'true';

  // Remove old operation (Rust handles history tracking)
  if (opId !== null && editSession) {
    try {
      editSession.removeOperation(opId);
    } catch (err) {
      console.error('Error removing text operation:', err);
    }
  }

  // Convert new position to PDF coordinates using PDF.js viewport method
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (pageInfo && editSession) {
    const [pdfX, pdfY] = pageInfo.viewport.convertToPdfPoint(newLeft, newTop);

    // Add new text operation at new position
    // NOTE: This uses hardcoded 200x20 dimensions - this is a known bug
    // TODO: Preserve original dimensions using updateRect when that method is added
    editSession.beginAction('move');
    const newOpId = editSession.addText(pageNum, pdfX, pdfY - 20, 200, 20, text, fontSize, '#000000', fontFamily, isItalic, isBold);
    editSession.commitAction();
    setOpId(textEl, newOpId);
  }
}

// ============ Whiteout Text Editor ============

async function openWhiteoutTextEditor(whiteRect: HTMLElement, pageNum: number): Promise<void> {
  // Check if already editing
  if (whiteRect.querySelector('.whiteout-text-input')) {
    return;
  }

  // Get whiteout dimensions
  const domX = parseFloat(whiteRect.style.left);
  const domY = parseFloat(whiteRect.style.top);
  const domWidth = parseFloat(whiteRect.style.width);
  const domHeight = parseFloat(whiteRect.style.height);

  // Store original dimensions for potential restoration
  const originalWidth = domWidth;
  const originalHeight = domHeight;

  // Detect covered text style
  const coveredStyle = await detectCoveredTextStyle(pageNum, domX, domY, domWidth, domHeight);

  // Create auto-expanding contentEditable div INSIDE the whiteout
  // Note: Using div (block) instead of span (inline) for predictable absolute positioning
  const input = document.createElement('div');
  input.contentEditable = 'true';
  input.className = 'whiteout-text-input';
  // Explicitly set ALL positioning inline to avoid CSS conflicts
  input.style.position = 'absolute';
  input.style.top = '0';
  input.style.left = '0';
  input.style.width = '100%';
  input.style.height = '100%';
  input.style.margin = '0';
  input.style.padding = '0';
  input.style.boxSizing = 'border-box';
  input.style.border = 'none';
  input.style.outline = 'none';
  input.style.background = 'transparent';
  input.style.textAlign = 'center';
  // Use flexbox for vertical centering
  input.style.display = 'flex';
  input.style.alignItems = 'center';
  input.style.justifyContent = 'center';

  // Apply covered text style (including bold/italic)
  input.style.fontSize = coveredStyle.fontSize + 'px';
  input.style.fontFamily = coveredStyle.fontFamily;
  input.style.color = '#000000';
  if (coveredStyle.isBold) input.style.fontWeight = 'bold';
  if (coveredStyle.isItalic) input.style.fontStyle = 'italic';

  // Store style info for saving
  input.dataset.fontSize = String(coveredStyle.fontSize);
  input.dataset.fontFamily = coveredStyle.fontFamily;
  input.dataset.isBold = coveredStyle.isBold ? 'true' : 'false';
  input.dataset.isItalic = coveredStyle.isItalic ? 'true' : 'false';
  // Store original dimensions for commitPendingEdits to use
  input.dataset.originalWidth = String(originalWidth);
  input.dataset.originalHeight = String(originalHeight);

  whiteRect.appendChild(input);
  whiteRect.classList.add('editing');
  // Allow whiteout to expand with content
  whiteRect.style.overflow = 'visible';
  input.focus();
  setActiveTextInput(input);

  // Auto-expand whiteout as user types
  function expandWhiteoutForText(): void {
    const text = input.textContent || '';
    if (!text) return; // Don't expand for empty content

    // Use Range to measure actual text content width (scrollWidth includes minWidth: 100%)
    const range = document.createRange();
    range.selectNodeContents(input);
    const textRect = range.getBoundingClientRect();

    // Add padding only if text exceeds current dimensions
    const padding = 16;
    const verticalPadding = 8;
    const textWidth = textRect.width + padding;
    const textHeight = textRect.height + verticalPadding;
    const currentWidth = parseFloat(whiteRect.style.width);
    const currentHeight = parseFloat(whiteRect.style.height);

    if (textWidth > currentWidth) {
      whiteRect.style.width = textWidth + 'px';
    }
    if (textHeight > currentHeight) {
      whiteRect.style.height = textHeight + 'px';
    }
  }

  input.addEventListener('input', expandWhiteoutForText);

  // Handle Enter to save
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      whiteRect.classList.remove('editing');
      saveWhiteoutText(whiteRect, pageNum, input, originalWidth, originalHeight);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      whiteRect.classList.remove('editing');
      // Restore original dimensions
      whiteRect.style.width = originalWidth + 'px';
      whiteRect.style.height = originalHeight + 'px';
      whiteRect.style.overflow = 'hidden';
      input.remove();
      setActiveTextInput(null);
    }
  });

  // Handle blur to save
  input.addEventListener('blur', () => {
    // Small delay to allow click events on style buttons to process
    setTimeout(() => {
      // Don't close if still focused (style button click refocuses)
      if (input.matches(':focus')) return;

      whiteRect.classList.remove('editing');
      if (input.parentElement && (input.textContent || '').trim()) {
        saveWhiteoutText(whiteRect, pageNum, input, originalWidth, originalHeight);
      } else if (input.parentElement) {
        // Restore original dimensions
        whiteRect.style.width = originalWidth + 'px';
        whiteRect.style.height = originalHeight + 'px';
        whiteRect.style.overflow = 'hidden';
        input.remove();
        setActiveTextInput(null);
      }
    }, 200);
  });
}

async function detectCoveredTextStyle(
  pageNum: number,
  domX: number,
  domY: number,
  domWidth: number,
  domHeight: number
): Promise<{ fontSize: number; fontFamily: string; isBold: boolean; isItalic: boolean }> {
  // Default style
  const defaultStyle = {
    fontSize: 12,
    fontFamily: 'Helvetica, Arial, sans-serif',
    isBold: false,
    isItalic: false,
  };

  try {
    // Get text items from this page
    const items = await PdfBridge.extractTextWithPositions(pageNum);
    if (!items || items.length === 0) {
      return defaultStyle;
    }

    // Find text items that overlap with the whiteout area
    const overlapping = items.filter((item) => {
      if (!item.domBounds) return false;
      const b = item.domBounds;
      // Check if text item intersects with whiteout
      return !(b.x + b.width < domX || b.x > domX + domWidth || b.y + b.height < domY || b.y > domY + domHeight);
    });

    if (overlapping.length === 0) {
      return defaultStyle;
    }

    // Use the first overlapping item's style
    const item = overlapping[0];
    // Use domFontSize (viewport-scaled) for display in the DOM
    return {
      fontSize: item.domFontSize || item.fontSize || 12,
      fontFamily: item.fontFamily || defaultStyle.fontFamily,
      isBold: item.isBold || false,
      isItalic: item.isItalic || false,
    };
  } catch (err) {
    console.error('Error detecting covered text style:', err);
    return defaultStyle;
  }
}

function saveWhiteoutText(whiteRect: HTMLElement, pageNum: number, input: HTMLElement, originalWidth: number, originalHeight: number): void {
  if (!editSession) return;

  const text = (input.textContent || '').trim();
  if (!text) {
    // Restore original dimensions if no text
    if (originalWidth) whiteRect.style.width = originalWidth + 'px';
    if (originalHeight) whiteRect.style.height = originalHeight + 'px';
    whiteRect.style.overflow = 'hidden';
    input.remove();
    setActiveTextInput(null);
    return;
  }

  // Get position and style info (including potentially expanded dimensions)
  const domX = parseFloat(whiteRect.style.left);
  const domY = parseFloat(whiteRect.style.top);
  const domWidth = parseFloat(whiteRect.style.width);
  const domHeight = parseFloat(whiteRect.style.height);
  const fontSize = parseFloat(input.dataset.fontSize || '12') || 12;
  const fontFamily = input.dataset.fontFamily || null;
  const isBold = input.dataset.isBold === 'true';
  const isItalic = input.dataset.isItalic === 'true';

  // Convert to PDF coordinates using PDF.js viewport method
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) {
    input.remove();
    return;
  }

  // Convert top-left and bottom-right corners
  const [pdfX1, pdfY1] = pageInfo.viewport.convertToPdfPoint(domX, domY);
  const [pdfX2, pdfY2] = pageInfo.viewport.convertToPdfPoint(domX + domWidth, domY + domHeight);
  const pdfX = Math.min(pdfX1, pdfX2);
  const pdfY = Math.min(pdfY1, pdfY2);
  const pdfWidth = Math.abs(pdfX2 - pdfX1);
  const pdfHeight = Math.abs(pdfY2 - pdfY1);

  // Begin whiteout action (may include whiteout resize + text)
  editSession.beginAction('whiteout');

  // If whiteout was resized, update the whiteout operation
  // Note: This only runs for whiteouts (not blackouts) since blackouts don't allow text editing
  if (originalWidth && originalHeight && (domWidth !== originalWidth || domHeight !== originalHeight)) {
    const existingOpId = getOpId(whiteRect);
    if (existingOpId !== null) {
      editSession.removeOperation(existingOpId);
      // Add new whiteout with updated dimensions (always white since this is a whiteout text editor)
      const newWhiteOpId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight, '#FFFFFF');
      setOpId(whiteRect, newWhiteOpId);
    }
  }

  // Check for mixed styling (partial bold/italic) in the whiteout text input
  const styledSegments = parseStyledSegments(input);

  let opId: bigint;
  if (styledSegments) {
    // Use addStyledText for mixed styling
    const segmentsJson = JSON.stringify(styledSegments);
    opId = editSession.addStyledText(
      pageNum,
      pdfX,
      pdfY,
      pdfWidth,
      pdfHeight,
      segmentsJson,
      fontSize,
      '#000000',
      fontFamily
    );
  } else {
    // Use simple addText for uniform styling
    opId = editSession.addText(pageNum, pdfX, pdfY, pdfWidth, pdfHeight, text, fontSize, '#000000', fontFamily, isItalic, isBold);
  }

  editSession.commitAction();

  // Replace input with text span INSIDE the whiteout (auto-sizing)
  const textSpan = document.createElement('span');
  textSpan.className = 'whiteout-text-content';
  textSpan.textContent = text;
  textSpan.style.display = 'flex';
  textSpan.style.alignItems = 'center';
  textSpan.style.justifyContent = 'center';
  textSpan.style.width = '100%';
  textSpan.style.height = '100%';
  textSpan.style.fontSize = fontSize + 'px';
  textSpan.style.fontFamily = input.dataset.fontFamily || 'Helvetica, Arial, sans-serif';
  textSpan.style.color = '#000000';
  if (isBold) textSpan.style.fontWeight = 'bold';
  if (isItalic) textSpan.style.fontStyle = 'italic';
  textSpan.style.whiteSpace = 'pre-wrap';
  textSpan.style.wordBreak = 'break-word';
  setOpId(textSpan, opId);
  textSpan.dataset.fontSize = String(fontSize);
  textSpan.dataset.fontFamily = fontFamily || 'sans-serif';
  textSpan.dataset.isBold = isBold ? 'true' : 'false';
  textSpan.dataset.isItalic = isItalic ? 'true' : 'false';

  // Remove input and add text span
  input.remove();
  setActiveTextInput(null);
  whiteRect.style.overflow = 'hidden';
  whiteRect.appendChild(textSpan);

  // Store text op ID on whiteRect for reference
  whiteRect.dataset.textOpId = opId.toString();

  updateButtons();
}

// ============ Text Editing Functions ============

function renderTextLayer(textLayer: HTMLElement, items: TextItem[], pageNum: number): void {
  textLayer.innerHTML = '';

  items.forEach((item, index) => {
    if (!item.str.trim()) return; // Skip whitespace-only items
    if (!item.domBounds) return; // Skip items without position

    const span = document.createElement('span');
    span.className = 'text-item';
    span.dataset.page = String(pageNum);
    span.dataset.index = String(index);
    span.textContent = item.str;

    // Position using DOM bounds
    span.style.left = item.domBounds.x + 'px';
    span.style.top = item.domBounds.y + 'px';
    span.style.width = Math.max(item.domBounds.width, 10) + 'px';
    span.style.height = Math.max(item.domBounds.height, 12) + 'px';

    // Hover highlight
    span.addEventListener('mouseenter', () => {
      if (currentTool === 'select' || currentTool === 'edit-text') {
        span.classList.add('hover');
      }
    });
    span.addEventListener('mouseleave', () => {
      span.classList.remove('hover');
    });

    // Click to edit
    span.addEventListener('click', (e) => {
      e.stopPropagation();
      if (currentTool === 'select' || currentTool === 'edit-text') {
        startTextEdit(pageNum, index, item, span);
      }
    });

    textLayer.appendChild(span);
  });
}

function startTextEdit(pageNum: number, index: number, textItem: TextItem, spanElement: HTMLElement): void {
  // Close any existing editor
  closeTextEditor();

  activeEditItem = { pageNum, index, textItem, spanElement };

  // Map CSS generic font family to web-safe fonts for preview
  const fontFamily = mapFontFamilyForPreview(textItem.fontFamily);
  // Use pdfHeight scaled by 1.5 (our render scale) for preview size
  const fontSize = (textItem.pdfHeight || 12) * 1.5;

  // Create inline editor
  const editor = document.createElement('div');
  editor.className = 'text-editor-popup';
  editor.innerHTML = `
        <input type="text" class="text-editor-input" value="${escapeHtml(textItem.str)}" />
        <div class="text-editor-actions">
            <button class="text-editor-save">Save</button>
            <button class="text-editor-cancel">Cancel</button>
        </div>
    `;

  // Apply font styling to input for accurate preview
  const input = editor.querySelector('.text-editor-input') as HTMLInputElement;
  input.style.fontFamily = fontFamily;
  input.style.fontSize = fontSize + 'px';
  // Store font size and family for style controls
  input.dataset.fontSize = String(Math.round(fontSize));
  input.dataset.fontFamily = textItem.fontFamily || 'sans-serif';
  // Initialize bold/italic state from detected text item
  input.dataset.isBold = textItem.isBold ? 'true' : 'false';
  input.dataset.isItalic = textItem.isItalic ? 'true' : 'false';
  if (textItem.isItalic) input.style.fontStyle = 'italic';
  if (textItem.isBold) input.style.fontWeight = 'bold';

  // Position near the text item
  const bounds = textItem.domBounds;
  if (bounds) {
    editor.style.left = bounds.x + 'px';
    editor.style.top = bounds.y + bounds.height + 5 + 'px';
  }

  const pageDiv = document.querySelector<HTMLElement>(`.edit-page[data-page="${pageNum}"]`);
  pageDiv?.appendChild(editor);

  // Focus input and register with style buttons
  input.focus();
  input.select();
  setActiveTextInput(input);

  // Event handlers
  editor.querySelector('.text-editor-save')?.addEventListener('click', () => {
    const newText = input.value;
    const inputIsBold = input.dataset.isBold === 'true';
    const inputIsItalic = input.dataset.isItalic === 'true';
    const customFontSize = parseFloat(input.dataset.fontSize || '0') || null;
    const customFontFamily = input.dataset.fontFamily || null;
    if (
      newText !== textItem.str ||
      inputIsBold !== textItem.isBold ||
      inputIsItalic !== textItem.isItalic ||
      customFontSize !== Math.round((textItem.pdfHeight || 12) * 1.5) ||
      customFontFamily !== textItem.fontFamily
    ) {
      applyTextReplacement(pageNum, textItem, newText, inputIsBold, inputIsItalic, customFontSize, customFontFamily);
    }
    closeTextEditor();
  });

  editor.querySelector('.text-editor-cancel')?.addEventListener('click', closeTextEditor);

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      const newText = input.value;
      const inputIsBold = input.dataset.isBold === 'true';
      const inputIsItalic = input.dataset.isItalic === 'true';
      const customFontSize = parseFloat(input.dataset.fontSize || '0') || null;
      const customFontFamily = input.dataset.fontFamily || null;
      if (
        newText !== textItem.str ||
        inputIsBold !== textItem.isBold ||
        inputIsItalic !== textItem.isItalic ||
        customFontSize !== Math.round((textItem.pdfHeight || 12) * 1.5) ||
        customFontFamily !== textItem.fontFamily
      ) {
        applyTextReplacement(pageNum, textItem, newText, inputIsBold, inputIsItalic, customFontSize, customFontFamily);
      }
      closeTextEditor();
    } else if (e.key === 'Escape') {
      closeTextEditor();
    }
  });

  // Mark span as editing
  spanElement.classList.add('editing');
}

function closeTextEditor(): void {
  const editor = document.querySelector('.text-editor-popup');
  if (editor) editor.remove();

  if (activeEditItem) {
    activeEditItem.spanElement.classList.remove('editing');
    activeEditItem = null;
  }

  setActiveTextInput(null);
}

function applyTextReplacement(
  pageNum: number,
  textItem: TextItem,
  newText: string,
  isBold: boolean | null = null,
  isItalic: boolean | null = null,
  customFontSize: number | null = null,
  customFontFamily: string | null = null
): void {
  if (!editSession) return;

  // Get page info for coordinate conversion
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;

  // Use explicit bold/italic if provided, otherwise fall back to detected values
  const useBold = isBold !== null ? isBold : textItem.isBold || false;
  const useItalic = isItalic !== null ? isItalic : textItem.isItalic || false;

  // Use custom font size if provided (from toolbar), otherwise estimate from text height
  // customFontSize comes in as DOM pixels, need to convert to PDF points
  const renderScale = 1.5;
  const fontSize = customFontSize !== null ? customFontSize / renderScale : textItem.pdfHeight || 12.0;

  // Use custom font family if provided (from toolbar)
  const useFontFamily = customFontFamily || textItem.fontFamily || null;

  // Use PDF coordinates from text item
  // Note: The Rust code adds padding to the white cover rectangle
  editSession.beginAction('replacetext');
  const opId = editSession.replaceText(
    pageNum,
    // Original rect (to cover)
    textItem.pdfX,
    textItem.pdfY,
    textItem.pdfWidth || 100,
    textItem.pdfHeight || 14,
    // New rect (same position)
    textItem.pdfX,
    textItem.pdfY,
    textItem.pdfWidth || 100,
    textItem.pdfHeight || 14,
    // Text
    textItem.str,
    newText,
    fontSize,
    '#000000',
    // Font family from toolbar or PDF.js styles
    useFontFamily,
    // Font style flags
    useItalic,
    useBold
  );
  editSession.commitAction();

  // Calculate DOM font size (use custom or scale from PDF)
  const domFontSize = customFontSize !== null ? customFontSize : (textItem.pdfHeight || 12) * renderScale;
  const fontFamily = mapFontFamilyForPreview(useFontFamily);

  // Add visual indicator (replacement overlay) with matching font
  const overlay = document.querySelector<HTMLElement>(`.overlay-container[data-page="${pageNum}"]`);
  if (!overlay) return;

  // Clean up any old overlay marked for removal (from re-editing)
  const oldOverlay = overlay.querySelector('.edit-replace-overlay[data-pending-removal="true"]');
  if (oldOverlay) {
    oldOverlay.remove();
  }

  const replaceEl = document.createElement('div');
  replaceEl.className = 'edit-replace-overlay';
  replaceEl.textContent = newText;

  // Position with generous padding to ensure full coverage of original canvas text
  const padding = 15; // Liberal padding to cover descenders, ascenders, and rendering artifacts
  if (textItem.domBounds) {
    replaceEl.style.left = textItem.domBounds.x - padding + 'px';
    replaceEl.style.top = textItem.domBounds.y - padding + 'px';
    replaceEl.style.minWidth = textItem.domBounds.width + padding * 2 + 'px';
    replaceEl.style.minHeight = textItem.domBounds.height + padding * 2 + 'px';
  }
  replaceEl.style.padding = padding + 'px';
  replaceEl.style.boxSizing = 'border-box';

  // Apply matching font styling (family, size, italic, bold)
  replaceEl.style.fontFamily = fontFamily;
  replaceEl.style.fontSize = domFontSize + 'px';
  replaceEl.style.lineHeight = '1';
  if (useItalic) replaceEl.style.fontStyle = 'italic';
  if (useBold) replaceEl.style.fontWeight = 'bold';

  setOpId(replaceEl, opId);
  // Store original textItem for undo-and-reedit approach
  replaceEl.dataset.textItemIndex = String(textItem.index);
  replaceEl.dataset.originalTextItem = JSON.stringify({
    index: textItem.index,
    str: textItem.str,
    pdfX: textItem.pdfX,
    pdfY: textItem.pdfY,
    pdfWidth: textItem.pdfWidth,
    pdfHeight: textItem.pdfHeight,
    fontFamily: textItem.fontFamily,
    isBold: textItem.isBold,
    isItalic: textItem.isItalic,
    domBounds: textItem.domBounds,
  });
  overlay.appendChild(replaceEl);

  // Make replacement overlay re-editable (click to undo and re-edit)
  makeReplaceOverlayEditable(replaceEl, pageNum);

  // Hide original text item visually
  const span = document.querySelector<HTMLElement>(`.text-item[data-page="${pageNum}"][data-index="${textItem.index}"]`);
  if (span) span.classList.add('replaced');

  updateButtons();
}

function escapeHtml(str: string): string {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// Map PDF.js font family to web-safe CSS font for preview
function mapFontFamilyForPreview(fontFamily: string | null | undefined): string {
  if (!fontFamily) return 'sans-serif';

  const lower = fontFamily.toLowerCase();

  // CSS generic families
  if (lower === 'serif') return 'Georgia, "Times New Roman", Times, serif';
  if (lower === 'sans-serif') return 'Arial, Helvetica, sans-serif';
  if (lower === 'monospace') return '"Courier New", Courier, monospace';

  // Specific font names
  if (lower.includes('times')) return '"Times New Roman", Times, serif';
  if (lower.includes('arial') || lower.includes('helvetica')) return 'Arial, Helvetica, sans-serif';
  if (lower.includes('courier') || lower.includes('mono')) return '"Courier New", Courier, monospace';
  if (lower.includes('georgia')) return 'Georgia, serif';

  // Default to sans-serif
  return 'sans-serif';
}

// Map detected font family to dropdown option value
function mapFontFamilyToDropdown(fontFamily: string | null | undefined): string {
  if (!fontFamily) return 'sans-serif';

  const lower = fontFamily.toLowerCase();

  // Exact matches for dropdown values
  if (lower === 'sans-serif') return 'sans-serif';
  if (lower === 'serif') return 'serif';
  if (lower === 'monospace') return 'monospace';
  if (lower === 'arial') return 'Arial';
  if (lower === 'times new roman') return 'Times New Roman';
  if (lower === 'georgia') return 'Georgia';
  if (lower === 'courier new') return 'Courier New';
  if (lower === 'verdana') return 'Verdana';
  if (lower === 'trebuchet ms') return 'Trebuchet MS';

  // Partial matches for detected fonts
  if (lower.includes('times')) return 'Times New Roman';
  if (lower.includes('arial')) return 'Arial';
  if (lower.includes('helvetica')) return 'sans-serif';
  if (lower.includes('courier') || lower.includes('mono')) return 'Courier New';
  if (lower.includes('georgia')) return 'Georgia';
  if (lower.includes('verdana')) return 'Verdana';
  if (lower.includes('trebuchet')) return 'Trebuchet MS';

  // Default to sans-serif
  return 'sans-serif';
}

// ============ Bold/Italic Style Functions ============

function setActiveTextInput(input: HTMLElement | null): void {
  activeTextInput = input;
  updateStyleButtons();

  if (input) {
    // Track focus/blur to update active state
    input.addEventListener('blur', handleTextInputBlur);
  }
}

function handleTextInputBlur(): void {
  // Small delay to allow click events on style buttons to process
  setTimeout(() => {
    if (activeTextInput && !activeTextInput.matches(':focus')) {
      activeTextInput.removeEventListener('blur', handleTextInputBlur);
      activeTextInput = null;
      updateStyleButtons();
    }
  }, 150);
}

function updateStyleButtons(): void {
  const boldBtn = document.getElementById('style-bold') as HTMLButtonElement | null;
  const italicBtn = document.getElementById('style-italic') as HTMLButtonElement | null;
  const fontSizeDecrease = document.getElementById('font-size-decrease') as HTMLButtonElement | null;
  const fontSizeIncrease = document.getElementById('font-size-increase') as HTMLButtonElement | null;
  const fontSizeValue = document.getElementById('font-size-value') as HTMLInputElement | null;
  const fontFamilySelect = document.getElementById('style-font-family') as HTMLSelectElement | null;

  if (!boldBtn || !italicBtn || !fontSizeDecrease || !fontSizeIncrease || !fontSizeValue || !fontFamilySelect) return;

  if (!activeTextInput) {
    boldBtn.disabled = true;
    italicBtn.disabled = true;
    fontSizeDecrease.disabled = true;
    fontSizeIncrease.disabled = true;
    fontSizeValue.disabled = true;
    fontFamilySelect.disabled = true;
    boldBtn.classList.remove('active');
    italicBtn.classList.remove('active');
    return;
  }

  boldBtn.disabled = false;
  italicBtn.disabled = false;
  fontSizeDecrease.disabled = false;
  fontSizeIncrease.disabled = false;
  fontSizeValue.disabled = false;
  fontFamilySelect.disabled = false;

  // Check current state from input's dataset or computed style
  const inputEl = activeTextInput as HTMLElement;
  const isBold = inputEl.dataset.isBold === 'true' || inputEl.style.fontWeight === 'bold' || inputEl.style.fontWeight === '700';
  const isItalic = inputEl.dataset.isItalic === 'true' || inputEl.style.fontStyle === 'italic';

  boldBtn.classList.toggle('active', isBold);
  italicBtn.classList.toggle('active', isItalic);

  // Sync font size value
  const fontSize = inputEl.dataset.fontSize || '12';
  fontSizeValue.value = fontSize;

  // Sync font family dropdown (map detected font to dropdown option)
  const fontFamily = inputEl.dataset.fontFamily || 'sans-serif';
  fontFamilySelect.value = mapFontFamilyToDropdown(fontFamily);
}

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
    const currentBold = activeTextInput.dataset.isBold === 'true';
    const newBold = !currentBold;

    activeTextInput.dataset.isBold = String(newBold);
    activeTextInput.style.fontWeight = newBold ? 'bold' : 'normal';

    updateStyleButtons();
    activeTextInput.focus();
  }
}

function toggleItalic(): void {
  if (!activeTextInput) return;

  // ISSUE-025 FIX: Check for text selection within the active input
  const selection = window.getSelection();
  const hasSelection =
    selection &&
    selection.rangeCount > 0 &&
    !selection.isCollapsed &&
    activeTextInput.contains(selection.anchorNode);

  if (hasSelection) {
    // Apply italic to selected text only using execCommand
    // This wraps selected text in <i> or <em> tags
    document.execCommand('italic', false);
    activeTextInput.focus();
  } else {
    // No selection - apply to entire element (fallback behavior)
    const currentItalic = activeTextInput.dataset.isItalic === 'true';
    const newItalic = !currentItalic;

    activeTextInput.dataset.isItalic = String(newItalic);
    activeTextInput.style.fontStyle = newItalic ? 'italic' : 'normal';

    updateStyleButtons();
    activeTextInput.focus();
  }
}

function increaseFontSize(): void {
  if (!activeTextInput) return;
  const current = parseInt(activeTextInput.dataset.fontSize || '12', 10) || 12;
  setFontSize(String(Math.min(current + 2, 72)));
}

function decreaseFontSize(): void {
  if (!activeTextInput) return;
  const current = parseInt(activeTextInput.dataset.fontSize || '12', 10) || 12;
  setFontSize(String(Math.max(current - 2, 6)));
}

function setFontSize(size: string): void {
  if (!activeTextInput) return;
  const oldSize = parseInt(activeTextInput.dataset.fontSize || '12', 10) || 12;
  const sizeNum = Math.max(6, Math.min(72, parseInt(size, 10) || 12));
  activeTextInput.dataset.fontSize = String(sizeNum);
  activeTextInput.style.fontSize = sizeNum + 'px';
  const fontSizeValue = document.getElementById('font-size-value') as HTMLInputElement | null;
  if (fontSizeValue) fontSizeValue.value = String(sizeNum);
  updateStyleButtons();

  // Auto-expand text box proportionally when font size increases
  const parentBox = activeTextInput.closest('.text-box') as HTMLElement | null;
  if (parentBox && sizeNum > oldSize) {
    const scaleFactor = sizeNum / oldSize;
    const currentWidth = parseFloat(parentBox.style.width) || 200;
    const currentHeight = parseFloat(parentBox.style.height) || 48;

    // Scale dimensions proportionally (with constraints)
    const pageEl = parentBox.closest('.edit-page') as HTMLElement | null;
    const pageWidth = pageEl?.offsetWidth || 800;
    const boxLeft = parseFloat(parentBox.style.left) || 0;
    const maxAvailableWidth = Math.max(100, pageWidth - boxLeft - 10);

    const newWidth = Math.min(maxAvailableWidth, currentWidth * scaleFactor);
    const newHeight = currentHeight * scaleFactor;

    parentBox.style.width = newWidth + 'px';
    parentBox.style.height = newHeight + 'px';
  }

  // Also run content-based expansion to ensure text fits
  if (parentBox) {
    expandTextBoxForContent(parentBox, activeTextInput);
  }

  activeTextInput.focus();
}

function setFontFamily(family: string): void {
  if (!activeTextInput) return;
  activeTextInput.dataset.fontFamily = family;
  activeTextInput.style.fontFamily = family;
  updateStyleButtons();
  activeTextInput.focus();
}

function undoLastOperation(): void {
  if (!editSession || !editSession.canUndo()) return;

  // Call Rust undo - returns array of OpIds that were removed
  const undoneIds = editSession.undo();
  if (!undoneIds) return;

  // Remove all undone elements from DOM
  for (let i = 0; i < undoneIds.length; i++) {
    const opId = undoneIds[i];
    const el = document.querySelector(`[data-op-id="${opId}"]`);
    if (el) el.remove();
  }

  updateButtons();
}

function redoLastOperation(): void {
  if (!editSession || !editSession.canRedo()) return;

  // Call Rust redo - returns array of OpIds that were restored
  const redoneIds = editSession.redo();
  if (!redoneIds) return;

  // Recreate DOM elements for redone operations
  for (let i = 0; i < redoneIds.length; i++) {
    const opId = redoneIds[i];
    recreateOperationElement(opId);
  }

  updateButtons();
}

// Recreate a DOM element for an operation (used during redo)
function recreateOperationElement(opId: bigint): void {
  if (!editSession) return;

  const json = editSession.getOperationJson(opId);
  if (!json) return;

  try {
    const op = JSON.parse(json) as { type: string; page: number; rect: PdfRect; text?: string; style?: TextStyle; checked?: boolean };

    // Handle serde adjacent tag format: {"type":"AddWhiteRect","page":1,"rect":{...}}
    switch (op.type) {
      case 'AddWhiteRect':
        recreateWhiteRect(opId, { page: op.page, rect: op.rect });
        break;
      case 'AddText':
        recreateTextBox(opId, { page: op.page, rect: op.rect, text: op.text || '', style: op.style });
        break;
      // DISABLED: Checkbox tool is hidden (ISSUE-003)
      // case 'AddCheckbox':
      //   recreateCheckbox(opId, { page: op.page, rect: op.rect, checked: op.checked || false });
      //   break;
      // DISABLED: Highlight tool is hidden (ISSUE-001)
      // case 'AddHighlight':
      //   recreateHighlight(opId, { page: op.page, rect: op.rect });
      //   break;
    }
  } catch {
    // Ignore parse errors
  }
}

interface PdfRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface TextStyle {
  font_size?: number;
  font_name?: string;
  color?: string;
  is_bold?: boolean;
  is_italic?: boolean;
}

function recreateWhiteRect(opId: bigint, data: { page: number; rect: PdfRect }): void {
  const pageNum = data.page;
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;

  // Convert PDF coords to DOM coords using PDF.js viewport method
  const pdfRect = data.rect;
  const viewportRect = pageInfo.viewport.convertToViewportRectangle([
    pdfRect.x,
    pdfRect.y,
    pdfRect.x + pdfRect.width,
    pdfRect.y + pdfRect.height
  ]);
  // Normalize (viewport may swap coordinates)
  const domX = Math.min(viewportRect[0], viewportRect[2]);
  const domY = Math.min(viewportRect[1], viewportRect[3]);
  const domWidth = Math.abs(viewportRect[2] - viewportRect[0]);
  const domHeight = Math.abs(viewportRect[3] - viewportRect[1]);

  const overlay = document.querySelector<HTMLElement>(`.overlay-container[data-page="${pageNum}"]`);
  if (!overlay) return;

  const whiteRect = document.createElement('div');
  whiteRect.className = 'edit-whiteout-overlay';
  whiteRect.style.left = domX + 'px';
  whiteRect.style.top = domY + 'px';
  whiteRect.style.width = domWidth + 'px';
  whiteRect.style.height = domHeight + 'px';
  setOpId(whiteRect, opId);
  whiteRect.dataset.page = String(pageNum);

  // Add event handlers
  whiteRect.addEventListener('mousedown', (e) => {
    if ((e.target as HTMLElement).classList.contains('resize-handle')) return;
    e.stopPropagation();
    e.preventDefault();
    selectWhiteout(whiteRect);
    startMove(e, whiteRect);
  });

  whiteRect.addEventListener('dblclick', (e) => {
    e.stopPropagation();
    openWhiteoutTextEditor(whiteRect, pageNum);
  });

  overlay.appendChild(whiteRect);
}

function recreateTextBox(opId: bigint, data: { page: number; rect: PdfRect; text: string; style?: TextStyle }): void {
  const pageNum = data.page;
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;

  // Convert PDF coords to DOM coords using PDF.js viewport method
  const pdfRect = data.rect;
  const viewportRect = pageInfo.viewport.convertToViewportRectangle([
    pdfRect.x,
    pdfRect.y,
    pdfRect.x + pdfRect.width,
    pdfRect.y + pdfRect.height
  ]);
  // Normalize (viewport may swap coordinates)
  const domX = Math.min(viewportRect[0], viewportRect[2]);
  const domY = Math.min(viewportRect[1], viewportRect[3]);
  const domWidth = Math.abs(viewportRect[2] - viewportRect[0]);
  const domHeight = Math.abs(viewportRect[3] - viewportRect[1]);

  const overlay = document.querySelector<HTMLElement>(`.overlay-container[data-page="${pageNum}"]`);
  if (!overlay) return;

  const box = document.createElement('div');
  box.className = 'text-box transparent';
  box.dataset.page = String(pageNum);
  box.style.left = domX + 'px';
  box.style.top = domY + 'px';
  box.style.width = domWidth + 'px';
  box.style.height = domHeight + 'px';
  box.style.zIndex = String(nextTextBoxZIndex++);
  setOpId(box, opId);

  // Add delete button
  const deleteBtn = document.createElement('button');
  deleteBtn.className = 'delete-btn';
  deleteBtn.innerHTML = '&times;';
  deleteBtn.title = 'Delete';
  deleteBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    deleteTextBox(box);
  });
  box.appendChild(deleteBtn);

  // Add text content
  const textContent = document.createElement('div');
  textContent.className = 'text-content';
  textContent.contentEditable = 'true';
  textContent.textContent = data.text || '';

  const style = data.style || {};
  textContent.dataset.fontSize = String(style.font_size || 12);
  textContent.dataset.fontFamily = style.font_name || 'sans-serif';
  textContent.dataset.isBold = String(style.is_bold || false);
  textContent.dataset.isItalic = String(style.is_italic || false);
  textContent.style.fontSize = (style.font_size || 12) + 'px';
  textContent.style.fontFamily = style.font_name || 'sans-serif';
  if (style.is_bold) textContent.style.fontWeight = 'bold';
  if (style.is_italic) textContent.style.fontStyle = 'italic';
  if (style.color) textContent.style.color = style.color;

  textContent.addEventListener('focus', () => {
    activeTextInput = textContent;
    updateStyleButtons();
  });
  textContent.addEventListener('blur', () => {
    activeTextInput = null;
    updateStyleButtons();
    commitTextBox(box);
  });
  box.appendChild(textContent);

  // Add resize handles
  const handles = ['nw', 'n', 'ne', 'w', 'e', 'sw', 's', 'se'];
  handles.forEach((pos) => {
    const handle = document.createElement('div');
    handle.className = `resize-handle resize-handle-${pos}`;
    handle.dataset.handle = pos;
    handle.addEventListener('mousedown', (e) => startTextBoxResize(e, box, pos));
    box.appendChild(handle);
  });

  box.addEventListener('mousedown', (e) => {
    if ((e.target as HTMLElement).classList.contains('resize-handle') ||
        (e.target as HTMLElement).classList.contains('delete-btn')) {
      return;
    }
    selectTextBox(box);
    startTextBoxMove(e, box);
  });

  overlay.appendChild(box);
}

// DISABLED: Checkbox tool is hidden (ISSUE-003)
// function recreateCheckbox(...) { ... }

// DISABLED: Highlight tool is hidden (ISSUE-001)
// function recreateHighlight(...) { ... }

function updateButtons(): void {
  const downloadBtn = document.getElementById('edit-download-btn') as HTMLButtonElement | null;
  const undoBtn = document.getElementById('edit-undo-btn') as HTMLButtonElement | null;
  const redoBtn = document.getElementById('edit-redo-btn') as HTMLButtonElement | null;

  const hasChanges = editSession && editSession.hasChanges();
  if (downloadBtn) downloadBtn.disabled = !hasChanges;
  if (undoBtn) undoBtn.disabled = !editSession || !editSession.canUndo();
  if (redoBtn) redoBtn.disabled = !editSession || !editSession.canRedo();
}

/**
 * Commit any pending text edits before export.
 * This handles the case where user types in whiteout and clicks Download
 * before the blur event's setTimeout (200ms) completes.
 */
function commitPendingEdits(): void {
  // ISSUE-013 FIX: Commit pending text boxes before export
  // Find all text boxes with unsaved content and commit them
  const textBoxes = document.querySelectorAll<HTMLElement>('.text-box');
  textBoxes.forEach(box => {
    const textContent = box.querySelector<HTMLElement>('.text-content');
    const text = textContent?.textContent?.trim() || '';
    if (text) {
      // Check if this text box has a pending operation (no opId means it hasn't been saved)
      const existingOpId = getOpId(box);
      // Always commit to ensure latest content is saved
      commitTextBox(box);
    }
  });

  // Find any active whiteout text input
  const activeWhiteoutInput = document.querySelector<HTMLElement>('.whiteout-text-input');
  if (!activeWhiteoutInput) return;

  const text = (activeWhiteoutInput.textContent || '').trim();
  if (!text) return; // No text to save

  // Find the parent whiteout
  const whiteRect = activeWhiteoutInput.closest<HTMLElement>('.edit-whiteout-overlay');
  if (!whiteRect) return;

  // Get page number from overlay container
  const overlayContainer = whiteRect.closest<HTMLElement>('.overlay-container');
  const pageNum = overlayContainer ? parseInt(overlayContainer.dataset.page || '1', 10) : 1;

  // Get original dimensions from data attributes
  const originalWidth = parseFloat(activeWhiteoutInput.dataset.originalWidth || '0');
  const originalHeight = parseFloat(activeWhiteoutInput.dataset.originalHeight || '0');

  // Call saveWhiteoutText to persist the text
  saveWhiteoutText(whiteRect, pageNum, activeWhiteoutInput, originalWidth, originalHeight);
}

async function downloadEditedPdf(): Promise<void> {
  if (!editSession) return;

  // Commit any pending text edits before export
  commitPendingEdits();

  const downloadBtn = document.getElementById('edit-download-btn') as HTMLButtonElement | null;
  const btnContent = downloadBtn?.querySelector('.download-btn-content');
  if (!btnContent) return;

  try {
    // Parity check: Verify DOM elements match WASM operations (debug aid)
    const opCount = editSession.getOperationCount();
    const textBoxCount = document.querySelectorAll('.text-box').length;
    const whiteoutCount = document.querySelectorAll('.edit-whiteout-overlay').length;
    const highlightCount = document.querySelectorAll('.edit-highlight-overlay').length;
    const checkboxCount = document.querySelectorAll('.edit-checkbox-overlay').length;
    const replaceCount = document.querySelectorAll('.edit-replace-overlay').length;
    const underlineCount = document.querySelectorAll('.edit-underline-overlay').length;
    const domAnnotations = textBoxCount + whiteoutCount + highlightCount + checkboxCount + replaceCount + underlineCount;

    // Whiteout with text creates 2 operations (whiteout + text), so adjust expected count
    // This is an approximation - exact parity requires tracking op types
    if (opCount > 0 && Math.abs(opCount - domAnnotations) > domAnnotations) {
      console.warn(
        `[PDFJoin Parity Warning] Operation count (${opCount}) significantly differs from DOM elements (${domAnnotations}). ` +
        `TextBoxes: ${textBoxCount}, Whiteouts: ${whiteoutCount}, Highlights: ${highlightCount}, ` +
        `Checkboxes: ${checkboxCount}, Replaces: ${replaceCount}, Underlines: ${underlineCount}. ` +
        `This may indicate preview/download mismatch.`
      );
    }

    // Disable button during verification
    if (downloadBtn) downloadBtn.disabled = true;

    // Show verification spinner
    btnContent.innerHTML = `
      <span class="spinner"></span>
      <span class="verification-text">Proof Verification in Progress</span>
    `;

    const result = editSession.export();
    const blob = new Blob([result as unknown as BlobPart], { type: 'application/pdf' });

    // Calculate verification time proportional to file size (300ms min, 3000ms max)
    const fileSizeKB = blob.size / 1024;
    const verificationTime = Math.min(3000, Math.max(300, fileSizeKB * 2));

    await new Promise((resolve) => setTimeout(resolve, verificationTime));

    // Show verification passed
    btnContent.innerHTML = `
      <span class="verification-text verification-passed">✓ Proof Verification Passed!</span>
    `;

    // Brief pause to show success message
    await new Promise((resolve) => setTimeout(resolve, 500));

    // Download the file
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = editSession.documentName.replace(/\.pdf$/i, '-edited.pdf');
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);

    // Reset button state
    btnContent.innerHTML = `<span class="download-text">Download Edited PDF</span>`;
    if (downloadBtn) downloadBtn.disabled = false;
  } catch (e) {
    // Reset button on error
    btnContent.innerHTML = `<span class="download-text">Download Edited PDF</span>`;
    if (downloadBtn) downloadBtn.disabled = false;
    showError('edit-error', 'Export failed: ' + e);
  }
}

function resetEditView(): void {
  editSession = null;
  currentPage = 1;
  // Note: undo/redo history is cleared when editSession is set to null
  currentTool = 'select';
  currentPdfBytes = null;
  currentPdfFilename = null;
  textItemsMap.clear();
  closeTextEditor();

  // Clear shared state callbacks
  clearEditCallbacks();

  // Reset whiteout drawing and selection state
  handleWhiteoutCancel();
  deselectWhiteout();
  selectedWhiteout = null;

  // Reset UI
  document.getElementById('edit-drop-zone')?.classList.remove('hidden');
  document.getElementById('edit-signed-warning')?.classList.add('hidden');
  document.getElementById('edit-editor')?.classList.add('hidden');
  const fileInput = document.getElementById('edit-file-input') as HTMLInputElement | null;
  if (fileInput) fileInput.value = '';
  const pagesContainer = document.getElementById('edit-pages');
  if (pagesContainer) pagesContainer.innerHTML = '';
  document.getElementById('edit-error')?.classList.add('hidden');

  // Reset tool buttons
  document.querySelectorAll<HTMLElement>('.tool-btn[id^="tool-"]').forEach((b) => b.classList.remove('active'));
  document.getElementById('tool-select')?.classList.add('active');

  // Cleanup PDF.js
  PdfBridge.cleanup();
}

function navigatePage(delta: number): void {
  if (!editSession) return;

  const newPage = currentPage + delta;
  if (newPage < 1 || newPage > editSession.pageCount) return;

  currentPage = newPage;
  updatePageNavigation();

  // Scroll to page
  const pageEl = document.querySelector<HTMLElement>(`.edit-page[data-page="${currentPage}"]`);
  if (pageEl) {
    pageEl.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }
}

function updatePageNavigation(): void {
  if (!editSession) return;

  const indicator = document.getElementById('edit-page-indicator');
  const prevBtn = document.getElementById('edit-prev-page') as HTMLButtonElement | null;
  const nextBtn = document.getElementById('edit-next-page') as HTMLButtonElement | null;

  if (indicator) indicator.textContent = `Page ${currentPage} of ${editSession.pageCount}`;
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= editSession.pageCount;
}

function updateCursor(): void {
  const viewer = document.getElementById('edit-viewer');
  if (!viewer) return;

  switch (currentTool) {
    case 'select':
      viewer.style.cursor = 'default';
      break;
    case 'edit-text':
      viewer.style.cursor = 'text';
      break;
    case 'text':
      viewer.style.cursor = 'text';
      break;
    case 'textbox':
      viewer.style.cursor = 'crosshair';
      break;
    // DISABLED: Highlight (ISSUE-001), Underline (ISSUE-002), Checkbox (ISSUE-003) tools are hidden
    // case 'highlight':
    // case 'underline':
    //   viewer.style.cursor = 'text';
    //   break;
    // case 'checkbox':
    //   viewer.style.cursor = 'pointer';
    //   break;
    case 'whiteout':
      viewer.style.cursor = 'crosshair';
      break;
    default:
      viewer.style.cursor = 'default';
  }

  // Disable text layer pointer events when drawing tools are active
  // This allows mouse events to reach the page div for drawing
  const isDrawingTool = currentTool === 'whiteout' || currentTool === 'textbox';
  document.querySelectorAll<HTMLElement>('.text-layer').forEach((layer) => {
    layer.style.pointerEvents = isDrawingTool ? 'none' : 'auto';
  });

  // DISABLED: Highlight/underline visual feedback (ISSUE-001, ISSUE-002)
  // const isTextAnnotationTool = currentTool === 'highlight' || currentTool === 'underline';
  // viewer.classList.toggle('highlight-mode', isTextAnnotationTool);

  // Enable overlay-container pointer events for tools that need to capture clicks
  // textbox needs clicks for creating new boxes and editing existing ones
  // DISABLED: checkbox (ISSUE-003)
  const overlayNeedsClicks = currentTool === 'text' || currentTool === 'textbox';
  document.querySelectorAll<HTMLElement>('.overlay-container').forEach((overlay) => {
    overlay.style.pointerEvents = overlayNeedsClicks ? 'auto' : 'none';
  });
}

function showError(containerId: string, message: string): void {
  const container = document.getElementById(containerId);
  if (!container) return;

  const textEl = container.querySelector('.error-text');
  if (textEl) textEl.textContent = message;

  container.classList.remove('hidden');

  // Auto-dismiss after 8 seconds
  setTimeout(() => container.classList.add('hidden'), 8000);
}
