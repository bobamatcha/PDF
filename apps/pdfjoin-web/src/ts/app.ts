// PDFJoin - Single Page App
// Uses window.wasmBindings from Trunk-injected WASM loader

import { setupEditView, loadPdfIntoEdit } from './edit';
import { setSharedPdf, getSharedPdf, hasSharedPdf, editHasChanges, exportEditedPdf } from './shared-state';
import { ensurePdfJsLoaded } from './pdf-loader';
import type { PdfJoinSession, PdfInfo, DocumentInfo, PDFJSDocument } from './types';

// DEBUG: Expose shared state functions on window for testing
(window as any).__sharedState__ = {
  hasSharedPdf,
  getSharedPdf,
  setSharedPdf,
};

// Size thresholds
const LARGE_FILE_WARNING_BYTES = 50 * 1024 * 1024; // 50 MB
const VERY_LARGE_FILE_WARNING_BYTES = 100 * 1024 * 1024; // 100 MB

// ============ Split Visual Page Selection (Phase 2 UX) ============

interface PageThumbnail {
  pageNumber: number;
  thumbnailDataUrl: string;
  isSelected: boolean;
}

interface SplitState {
  pages: PageThumbnail[];
  selectedPageNumbers: Set<number>;
  separateFiles: boolean;
  lastSelectedIndex: number | null; // For shift+click range selection
  pdfDoc: PDFJSDocument | null;
  pdfBytes: Uint8Array | null;
}

const splitState: SplitState = {
  pages: [],
  selectedPageNumbers: new Set(),
  separateFiles: false,
  lastSelectedIndex: null,
  pdfDoc: null,
  pdfBytes: null,
};

// ============ Merge Multi-Document Page Picker (Phase 3 UX) ============

interface MergePageRef {
  documentId: string;
  pageNumber: number;
}

interface MergeDocument {
  id: string;
  filename: string;
  pdfBytes: Uint8Array;
  pageCount: number;
  pages: PageThumbnail[];
  isExpanded: boolean;
  shortCode: string;
  pdfDoc: PDFJSDocument | null;
}

interface MergeState {
  documents: MergeDocument[];
  mergeOrder: MergePageRef[]; // Final order for merging
  shortCodeMap: Map<string, number>; // Track used short codes for conflict resolution
}

const mergeState: MergeState = {
  documents: [],
  mergeOrder: [],
  shortCodeMap: new Map(),
};

/**
 * Generate a short code for a document (e.g., "C" for "contract.pdf")
 * If the first letter conflicts, use "C1", "C2", etc.
 */
function generateShortCode(filename: string): string {
  // Get first letter uppercase
  const baseName = filename.replace(/\.pdf$/i, '');
  const firstLetter = baseName.charAt(0).toUpperCase() || 'D';

  // Check if this letter is already used
  const count = mergeState.shortCodeMap.get(firstLetter) || 0;
  mergeState.shortCodeMap.set(firstLetter, count + 1);

  // If first use, just return the letter; otherwise add number
  if (count === 0) {
    return firstLetter;
  }
  return `${firstLetter}${count + 1}`;
}

/**
 * Render a page thumbnail for a merge document
 */
async function renderMergePageThumbnail(
  pdfDoc: PDFJSDocument,
  pageNum: number,
  scale: number = 0.3
): Promise<string> {
  const page = await pdfDoc.getPage(pageNum);
  const viewport = page.getViewport({ scale });

  const canvas = document.createElement('canvas');
  canvas.width = viewport.width;
  canvas.height = viewport.height;

  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('Could not get 2d context');

  await page.render({
    canvasContext: ctx,
    viewport,
  }).promise;

  return canvas.toDataURL('image/png', 0.8);
}

// ============ ACCESSIBILITY: Screen Reader Announcements (WCAG 4.1.3) ============

/**
 * Announce a message to screen readers via the aria-live region.
 * Use for important state changes like loading, success, and errors.
 */
function announceToScreenReader(message: string): void {
  const liveRegion = document.getElementById('aria-live-region');
  if (liveRegion) {
    // Clear first to ensure re-announcement of same message
    liveRegion.textContent = '';
    // Use setTimeout to ensure the DOM update triggers the announcement
    setTimeout(() => {
      liveRegion.textContent = message;
    }, 50);
  }
}

// ============ ACCESSIBILITY: Confirmation Dialog (WCAG 3.3.4) ============

interface ConfirmDialogOptions {
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  icon?: string;
}

/**
 * Shows a confirmation dialog for destructive actions.
 * Returns a promise that resolves to true if confirmed, false if cancelled.
 */
function showConfirmDialog(options: ConfirmDialogOptions): Promise<boolean> {
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
    confirmBtn.textContent = options.confirmText || 'Remove';
    cancelBtn.textContent = options.cancelText || 'Cancel';

    // Show dialog
    overlay.classList.add('show');
    confirmBtn.focus();

    // Clean up function
    const cleanup = () => {
      overlay.classList.remove('show');
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

    const onOverlayClick = (e: MouseEvent) => {
      if (e.target === overlay) {
        cleanup();
        resolve(false);
      }
    };

    const onKeydown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        cleanup();
        resolve(false);
      }
    };

    confirmBtn.addEventListener('click', onConfirm);
    cancelBtn.addEventListener('click', onCancel);
    overlay.addEventListener('click', onOverlayClick);
    document.addEventListener('keydown', onKeydown);
  });
}

let splitSession: PdfJoinSession | null = null;
let mergeSession: PdfJoinSession | null = null;
let splitOriginalFilename: string | null = null; // Track original filename for smart naming

export function init(): void {
  const { PdfJoinSession, SessionMode } = window.wasmBindings;

  // Initialize WASM sessions
  splitSession = new PdfJoinSession(SessionMode.Split);
  mergeSession = new PdfJoinSession(SessionMode.Merge);

  // Set up progress callbacks
  splitSession.setProgressCallback(onSplitProgress);
  mergeSession.setProgressCallback(onMergeProgress);

  // Set up UI
  setupToolPicker(); // Phase 1 UX: Tool card homepage
  setupTabs(); // Keep for screen reader fallback
  setupBackToToolsButtons(); // Phase 1 UX: Back navigation
  setupSplitView();
  setupMergeView();
  setupEditView(); // Initialize edit tab

  // Signal that initialization is complete (used by tests)
  (window as unknown as { pdfjoinInitialized: boolean }).pdfjoinInitialized = true;

  console.log('PDFJoin initialized (WASM-first architecture)');
}

// ============ Tool Picker Homepage (Phase 1 UX) ============

/**
 * Set up the tool picker cards on the homepage.
 * Clicking a card shows the corresponding tool view.
 */
function setupToolPicker(): void {
  const toolCards = document.querySelectorAll<HTMLElement>('.tool-card');

  toolCards.forEach((card) => {
    card.addEventListener('click', async () => {
      const tool = card.dataset.tool;
      if (tool) {
        await navigateToTool(tool);
      }
    });
  });
}

/**
 * Set up "Back to Tools" buttons in each tool view.
 * Clicking returns to the tool picker homepage.
 */
function setupBackToToolsButtons(): void {
  const backButtons = document.querySelectorAll<HTMLElement>('.back-to-tools');

  backButtons.forEach((btn) => {
    btn.addEventListener('click', async () => {
      // Check for unsaved changes when leaving Edit view
      const editView = document.getElementById('edit-view');
      const isEditActive = editView && !editView.classList.contains('hidden');

      if (isEditActive && editHasChanges()) {
        const action = await showUnsavedChangesModalForBack();
        if (action === 'cancel') return; // User cancelled
        // action === 'download' means user downloaded, action === 'continue' means proceed without saving
      }

      navigateToToolPicker();
    });
  });
}

/**
 * Navigate to a specific tool view.
 * Hides the tool picker and shows the tool view.
 * Also auto-loads shared PDF if available (cross-tab document persistence).
 */
async function navigateToTool(toolName: string): Promise<void> {
  // Hide tool picker
  const toolPicker = document.getElementById('tool-picker');
  if (toolPicker) toolPicker.classList.add('hidden');

  // Hide all views first
  document.querySelectorAll<HTMLElement>('.view').forEach((v) => v.classList.add('hidden'));

  // Show the selected view
  const view = document.getElementById(`${toolName}-view`);
  if (view) view.classList.remove('hidden');

  // Update tabs for screen reader users (keep in sync)
  const tabs = document.querySelectorAll<HTMLElement>('.tab');
  tabs.forEach((t) => t.classList.remove('active'));
  const activeTab = document.querySelector(`.tab[data-tab="${toolName}"]`);
  if (activeTab) activeTab.classList.add('active');

  // Announce to screen readers
  const toolNames: Record<string, string> = {
    split: 'Split PDF',
    merge: 'Merge PDFs',
    edit: 'Edit PDF'
  };
  announceToScreenReader(`Now viewing ${toolNames[toolName] || toolName} tool`);

  // Cross-tab document persistence: Auto-load shared PDF if available
  if (hasSharedPdf()) {
    const shared = getSharedPdf();
    if (shared.bytes && shared.filename) {
      // Don't reload into the source tool (avoid duplicate)
      if (shared.source !== toolName) {
        if (toolName === 'edit') {
          // Check if Edit already has a document loaded
          const editEditor = document.getElementById('edit-editor');
          const editAlreadyLoaded = editEditor && !editEditor.classList.contains('hidden');
          if (!editAlreadyLoaded) {
            await loadPdfIntoEdit(shared.bytes, shared.filename);
          }
        } else if (toolName === 'split') {
          // Check if Split already has a document loaded
          const splitEditor = document.getElementById('split-editor');
          const splitAlreadyLoaded = splitEditor && !splitEditor.classList.contains('hidden');
          if (!splitAlreadyLoaded) {
            await loadPdfIntoSplit(shared.bytes, shared.filename);
          }
        } else if (toolName === 'merge') {
          // Add to merge list if not already there
          await loadPdfIntoMerge(shared.bytes, shared.filename);
        }
      }
    }
  }
}

/**
 * Navigate back to the tool picker homepage.
 * Hides all tool views and shows the tool picker.
 */
function navigateToToolPicker(): void {
  // Hide all views
  document.querySelectorAll<HTMLElement>('.view').forEach((v) => v.classList.add('hidden'));

  // Show tool picker
  const toolPicker = document.getElementById('tool-picker');
  if (toolPicker) toolPicker.classList.remove('hidden');

  // Focus the first tool card for keyboard users
  const firstCard = document.querySelector<HTMLElement>('.tool-card');
  if (firstCard) firstCard.focus();

  // Announce to screen readers
  announceToScreenReader('Returned to tool selection. Choose Split, Merge, or Edit.');
}

/**
 * Modal for unsaved changes when clicking "Back to Tools"
 * Similar to tab switching but with different messaging
 */
async function showUnsavedChangesModalForBack(): Promise<'download' | 'continue' | 'cancel'> {
  return new Promise((resolve) => {
    let modal = document.getElementById('unsaved-changes-modal');
    if (!modal) {
      modal = document.createElement('div');
      modal.id = 'unsaved-changes-modal';
      modal.className = 'unsaved-changes-modal';
      modal.innerHTML = `
        <div class="modal-backdrop"></div>
        <div class="modal-content">
          <h2>You Made Edits</h2>
          <p>Would you like to download your edited PDF before going back?</p>
          <div class="modal-actions">
            <button class="primary-btn" data-action="download">Yes, Download My PDF</button>
            <button class="secondary-btn" data-action="continue">No, Go Back Without Saving</button>
            <button class="text-btn" data-action="cancel">Stay Here</button>
          </div>
        </div>
      `;
      document.body.appendChild(modal);

      // Add modal styles if not already present
      if (!document.getElementById('modal-styles')) {
        const style = document.createElement('style');
        style.id = 'modal-styles';
        style.textContent = `
          .unsaved-changes-modal { position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 1000; display: flex; align-items: center; justify-content: center; }
          .unsaved-changes-modal.hidden { display: none; }
          .modal-backdrop { position: absolute; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); }
          .modal-content { position: relative; background: white; padding: 2rem; border-radius: 12px; max-width: 420px; text-align: center; box-shadow: 0 4px 20px rgba(0,0,0,0.2); }
          .modal-content h2 { margin-bottom: 0.75rem; font-size: 1.5rem; }
          .modal-content p { margin-bottom: 1.5rem; color: #64748b; font-size: 1.1rem; line-height: 1.5; }
          .modal-actions { display: flex; flex-direction: column; gap: 0.75rem; }
          .modal-actions button { padding: 1rem 1.5rem; border-radius: 8px; font-size: 1.1rem; cursor: pointer; border: none; }
          .modal-actions .primary-btn { background: #2563eb; color: white; font-weight: 600; }
          .modal-actions .primary-btn:hover { background: #1d4ed8; }
          .modal-actions .secondary-btn { background: #f1f5f9; color: #334155; }
          .modal-actions .secondary-btn:hover { background: #e2e8f0; }
          .modal-actions .text-btn { background: transparent; color: #64748b; font-size: 1rem; }
          .modal-actions .text-btn:hover { color: #334155; }
        `;
        document.head.appendChild(style);
      }
    } else {
      // Update existing modal text for "back" context
      const content = modal.querySelector('.modal-content');
      if (content) {
        content.innerHTML = `
          <h2>You Made Edits</h2>
          <p>Would you like to download your edited PDF before going back?</p>
          <div class="modal-actions">
            <button class="primary-btn" data-action="download">Yes, Download My PDF</button>
            <button class="secondary-btn" data-action="continue">No, Go Back Without Saving</button>
            <button class="text-btn" data-action="cancel">Stay Here</button>
          </div>
        `;
      }
    }

    modal.classList.remove('hidden');

    const cleanup = (): void => {
      modal?.classList.add('hidden');
      modal?.querySelectorAll('button').forEach(btn => {
        btn.replaceWith(btn.cloneNode(true));
      });
    };

    modal.querySelector('[data-action="download"]')?.addEventListener('click', () => {
      const editedBytes = exportEditedPdf();
      if (editedBytes) {
        const shared = getSharedPdf();
        const filename = (shared.filename || 'document.pdf').replace(/\.pdf$/i, '-edited.pdf');
        downloadBlob(editedBytes, filename);
        setSharedPdf(editedBytes, filename, 'edit');
      }
      cleanup();
      resolve('download');
    }, { once: true });

    modal.querySelector('[data-action="continue"]')?.addEventListener('click', () => {
      cleanup();
      resolve('continue');
    }, { once: true });

    modal.querySelector('[data-action="cancel"]')?.addEventListener('click', () => {
      cleanup();
      resolve('cancel');
    }, { once: true });

    modal.querySelector('.modal-backdrop')?.addEventListener('click', () => {
      cleanup();
      resolve('cancel');
    }, { once: true });
  });
}

// ============ Tab Navigation (Screen Reader Fallback) ============

/**
 * Set up tab navigation for screen reader users.
 * Tabs are visually hidden but remain accessible for keyboard navigation.
 * They call the same navigation functions as the tool picker cards.
 */
function setupTabs(): void {
  const tabs = document.querySelectorAll<HTMLElement>('.tab');
  tabs.forEach((tab) => {
    tab.addEventListener('click', async () => {
      const tabName = tab.dataset.tab;
      const currentTab = document.querySelector('.tab.active')?.getAttribute('data-tab');

      // Determine what view is currently active (could be tool picker or a view)
      const toolPicker = document.getElementById('tool-picker');
      const isOnToolPicker = toolPicker && !toolPicker.classList.contains('hidden');

      // If on tool picker, just navigate to the tool
      if (isOnToolPicker && tabName) {
        await navigateToTool(tabName);
        return;
      }

      // Edit → Split/Merge: Check for unsaved changes
      if (currentTab === 'edit' && tabName !== 'edit') {
        if (editHasChanges()) {
          // Has changes - show simple modal
          const action = await showUnsavedChangesModal();
          if (action === 'cancel') return; // User cancelled
          // action === 'download' means user downloaded, action === 'continue' means proceed without saving
        }
        // Auto-load PDF into target tab (Split or Merge)
        if (hasSharedPdf()) {
          const shared = getSharedPdf();
          if (shared.bytes && shared.filename) {
            if (tabName === 'split') {
              await loadPdfIntoSplit(shared.bytes, shared.filename);
            } else if (tabName === 'merge') {
              // ISSUE-009: Edit → Merge - add document to merge list
              await loadPdfIntoMerge(shared.bytes, shared.filename);
            }
          }
        }
      }

      // Split → Merge: Add shared document to merge list (ISSUE-009)
      if (currentTab === 'split' && tabName === 'merge') {
        if (hasSharedPdf()) {
          const shared = getSharedPdf();
          if (shared.bytes && shared.filename) {
            await loadPdfIntoMerge(shared.bytes, shared.filename);
          }
        }
      }

      // Merge → Split: Load first merged document into Split (ISSUE-009)
      if (currentTab === 'merge' && tabName === 'split') {
        if (mergeSession && mergeSession.getDocumentCount() > 0) {
          try {
            const bytes = mergeSession.getDocumentBytes(0);
            const name = mergeSession.getDocumentName(0);
            await loadPdfIntoSplit(bytes, name);
            // Also update shared state
            setSharedPdf(bytes, name, 'merge');
          } catch (e) {
            console.error('Failed to load merge document into split:', e);
          }
        }
      }

      // Use centralized navigation (handles tool picker hiding, view showing, tab syncing, and shared PDF loading)
      if (tabName) {
        await navigateToTool(tabName);
      }

      // Split/Merge → Edit: Auto-load PDF (ISSUE-009 extended to include Merge)
      if (tabName === 'edit') {
        const editEditor = document.getElementById('edit-editor');
        const editAlreadyLoaded = editEditor && !editEditor.classList.contains('hidden');

        if (!editAlreadyLoaded) {
          // Try shared state first
          if (hasSharedPdf()) {
            const shared = getSharedPdf();
            if (shared.bytes && shared.filename) {
              await loadPdfIntoEdit(shared.bytes, shared.filename);
            }
          }
          // If coming from Merge and no shared state, load first merge document
          else if (currentTab === 'merge' && mergeSession && mergeSession.getDocumentCount() > 0) {
            try {
              const bytes = mergeSession.getDocumentBytes(0);
              const name = mergeSession.getDocumentName(0);
              await loadPdfIntoEdit(bytes, name);
              setSharedPdf(bytes, name, 'merge');
            } catch (e) {
              console.error('Failed to load merge document into edit:', e);
            }
          }
        }
      }
    });
  });
}

/**
 * Load PDF bytes directly into the Merge tab (for Tab PDF Sharing - ISSUE-009)
 * Adds the document to the merge list if not already present
 * This is an async function because it needs to load PDF.js for thumbnails
 */
async function loadPdfIntoMerge(bytes: Uint8Array, filename: string): Promise<void> {
  if (!mergeSession) return;

  // Check if document is already in the merge list (by name in mergeState)
  const alreadyExists = mergeState.documents.some((doc) => doc.filename === filename);
  if (alreadyExists) return;

  try {
    // Ensure PDF.js is loaded for thumbnail rendering
    await ensurePdfJsLoaded();

    // IMPORTANT: Make copies of bytes to prevent detachment issues
    const bytesForWasm = bytes.slice();
    const bytesForPdfJs = bytes.slice();
    const bytesForStorage = bytes.slice();

    // Add to WASM session for actual merging
    const info: PdfInfo = mergeSession.addDocument(filename, bytesForWasm);

    // Load PDF.js document for thumbnail rendering
    const pdfDoc = await window.pdfjsLib!.getDocument(bytesForPdfJs).promise;

    // Generate unique ID for this document
    const docId = `doc-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

    // Generate short code (e.g., "C" for contract.pdf)
    const shortCode = generateShortCode(filename);

    // Create merge document entry (same structure as handleMergeFiles)
    const mergeDoc: MergeDocument = {
      id: docId,
      filename: filename,
      pdfBytes: bytesForStorage,
      pageCount: info.page_count,
      pages: [],
      isExpanded: mergeState.documents.length === 0, // First doc expanded by default
      shortCode,
      pdfDoc,
    };

    // Add all pages to merge order by default (selected)
    for (let i = 1; i <= info.page_count; i++) {
      mergeDoc.pages.push({
        pageNumber: i,
        thumbnailDataUrl: '', // Render lazily
        isSelected: true,
      });
      mergeState.mergeOrder.push({
        documentId: docId,
        pageNumber: i,
      });
    }

    mergeState.documents.push(mergeDoc);

    // Update the UI
    updateMergePagePicker();

    // Pre-render thumbnails
    preRenderAllMergeThumbnails();

    console.log(`[loadPdfIntoMerge] Added "${filename}" to merge list from tab switch (${info.page_count} pages)`);
  } catch (e) {
    console.error('[loadPdfIntoMerge] Failed to add document to merge:', e);
  }
}

/**
 * Load PDF bytes directly into the Split tab (for Tab PDF Sharing)
 */
async function loadPdfIntoSplit(bytes: Uint8Array, filename: string): Promise<void> {
  if (!splitSession) return;
  const { format_bytes } = window.wasmBindings;

  try {
    // Clear any existing document first
    if (splitSession.getDocumentCount() > 0) {
      splitSession.removeDocument(0);
    }

    // IMPORTANT: Create copies BEFORE any operation that might detach the buffer
    // WASM and PDF.js both may transfer ownership of ArrayBuffer
    const bytesForWasm = bytes.slice();
    const bytesForThumbnails = bytes.slice();
    const bytesForState = bytes.slice();

    const info: PdfInfo = splitSession.addDocument(filename, bytesForWasm);

    // Store bytes in split state (use copy to prevent detachment issues)
    splitState.pdfBytes = bytesForState;

    // Store original filename for smart output naming
    splitOriginalFilename = filename.replace(/\.pdf$/i, '');

    // Update UI
    document.getElementById('split-drop-zone')?.classList.add('hidden');
    document.getElementById('split-editor')?.classList.remove('hidden');

    const fileNameEl = document.getElementById('split-file-name');
    const fileDetailsEl = document.getElementById('split-file-details');
    if (fileNameEl) fileNameEl.textContent = filename;
    if (fileDetailsEl) fileDetailsEl.textContent = `${info.page_count} pages - ${format_bytes(info.size_bytes)}`;

    // Update quick select button visibility
    updateQuickSelectButtons(info.page_count);

    // Reset split state for new document
    splitState.pages = [];
    splitState.selectedPageNumbers.clear();
    splitState.separateFiles = false;
    splitState.lastSelectedIndex = null;

    // Initialize page thumbnails (visual selection - Phase 2 UX)
    await renderPageThumbnails(bytesForThumbnails, info.page_count);

    // Reset UI state
    const splitBtn = document.getElementById('split-btn') as HTMLButtonElement | null;
    if (splitBtn) splitBtn.disabled = true;
    updateSelectionSummary();
  } catch (e) {
    showError('split-error', String(e));
  }
}

// Simple modal for unsaved changes - designed to be clear for all users
type ModalAction = 'download' | 'continue' | 'cancel';

async function showUnsavedChangesModal(): Promise<ModalAction> {
  return new Promise((resolve) => {
    // Create modal if it doesn't exist
    let modal = document.getElementById('unsaved-changes-modal');
    if (!modal) {
      modal = document.createElement('div');
      modal.id = 'unsaved-changes-modal';
      modal.className = 'unsaved-changes-modal';
      modal.innerHTML = `
        <div class="modal-backdrop"></div>
        <div class="modal-content">
          <h2>You Made Edits</h2>
          <p>Would you like to download your edited PDF before continuing?</p>
          <div class="modal-actions">
            <button class="primary-btn" data-action="download">Yes, Download My PDF</button>
            <button class="secondary-btn" data-action="continue">No, Continue Without Saving</button>
            <button class="text-btn" data-action="cancel">Go Back</button>
          </div>
        </div>
      `;
      document.body.appendChild(modal);

      // Add modal styles if not already present
      if (!document.getElementById('modal-styles')) {
        const style = document.createElement('style');
        style.id = 'modal-styles';
        style.textContent = `
          .unsaved-changes-modal { position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 1000; display: flex; align-items: center; justify-content: center; }
          .unsaved-changes-modal.hidden { display: none; }
          .modal-backdrop { position: absolute; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); }
          .modal-content { position: relative; background: white; padding: 2rem; border-radius: 12px; max-width: 420px; text-align: center; box-shadow: 0 4px 20px rgba(0,0,0,0.2); }
          .modal-content h2 { margin-bottom: 0.75rem; font-size: 1.5rem; }
          .modal-content p { margin-bottom: 1.5rem; color: #64748b; font-size: 1.1rem; line-height: 1.5; }
          .modal-actions { display: flex; flex-direction: column; gap: 0.75rem; }
          .modal-actions button { padding: 1rem 1.5rem; border-radius: 8px; font-size: 1.1rem; cursor: pointer; border: none; }
          .modal-actions .primary-btn { background: #2563eb; color: white; font-weight: 600; }
          .modal-actions .primary-btn:hover { background: #1d4ed8; }
          .modal-actions .secondary-btn { background: #f1f5f9; color: #334155; }
          .modal-actions .secondary-btn:hover { background: #e2e8f0; }
          .modal-actions .text-btn { background: transparent; color: #64748b; font-size: 1rem; }
          .modal-actions .text-btn:hover { color: #334155; }
        `;
        document.head.appendChild(style);
      }
    }

    modal.classList.remove('hidden');

    const cleanup = (): void => {
      modal?.classList.add('hidden');
      // Remove old listeners
      modal?.querySelectorAll('button').forEach(btn => {
        btn.replaceWith(btn.cloneNode(true));
      });
    };

    // Handle button clicks - use fresh listeners each time
    modal.querySelector('[data-action="download"]')?.addEventListener('click', () => {
      // Export and download the edited PDF
      const editedBytes = exportEditedPdf();
      if (editedBytes) {
        const shared = getSharedPdf();
        const filename = (shared.filename || 'document.pdf').replace(/\.pdf$/i, '-edited.pdf');
        downloadBlob(editedBytes, filename);
        // Also update shared state with edited version
        setSharedPdf(editedBytes, filename, 'edit');
      }
      cleanup();
      resolve('download');
    }, { once: true });

    modal.querySelector('[data-action="continue"]')?.addEventListener('click', () => {
      cleanup();
      resolve('continue');
    }, { once: true });

    modal.querySelector('[data-action="cancel"]')?.addEventListener('click', () => {
      cleanup();
      resolve('cancel');
    }, { once: true });

    // Close on backdrop click
    modal.querySelector('.modal-backdrop')?.addEventListener('click', () => {
      cleanup();
      resolve('cancel');
    }, { once: true });
  });
}

// ============ Split View ============

function setupSplitView(): void {
  const dropZone = document.getElementById('split-drop-zone');
  const fileInput = document.getElementById('split-file-input') as HTMLInputElement | null;
  const browseBtn = document.getElementById('split-browse-btn');
  const removeBtn = document.getElementById('split-remove-btn');
  const splitBtn = document.getElementById('split-btn');
  const rangeInput = document.getElementById('page-range');

  if (!dropZone || !fileInput || !browseBtn || !removeBtn || !splitBtn) return;

  browseBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    fileInput.click();
  });

  dropZone.addEventListener('click', () => fileInput.click());
  dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropZone.classList.add('drag-over');
  });
  dropZone.addEventListener('dragleave', () => dropZone.classList.remove('drag-over'));
  dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    dropZone.classList.remove('drag-over');
    const files = e.dataTransfer?.files;
    if (files && files.length > 0 && files[0].type === 'application/pdf') {
      handleSplitFile(files[0]);
    }
  });

  fileInput.addEventListener('change', () => {
    if (fileInput.files && fileInput.files.length > 0) handleSplitFile(fileInput.files[0]);
  });

  // ACCESSIBILITY: Confirm before removing file (WCAG 3.3.4 Error Prevention)
  removeBtn.addEventListener('click', async () => {
    const fileName = document.getElementById('split-file-name')?.textContent || 'this file';
    const confirmed = await showConfirmDialog({
      title: 'Remove File?',
      message: `Are you sure you want to remove "${fileName}"? You can add it again later.`,
      confirmText: 'Remove',
      cancelText: 'Keep',
    });
    if (confirmed) {
      resetSplitView();
    }
  });
  splitBtn.addEventListener('click', executeSplit);

  // Keep fallback text input functional for accessibility
  if (rangeInput) {
    rangeInput.addEventListener('input', validateRange);
  }

  // Setup quick select buttons (Phase 2 UX)
  setupQuickSelectButtons();

  // Setup separate files checkbox
  const separateFilesCheckbox = document.getElementById('split-separate-files') as HTMLInputElement | null;
  if (separateFilesCheckbox) {
    separateFilesCheckbox.addEventListener('change', () => {
      splitState.separateFiles = separateFilesCheckbox.checked;
    });
  }

  // Setup fallback text input toggle
  const showTextInputBtn = document.getElementById('split-show-text-input');
  const textInputContainer = document.getElementById('split-text-input-container');
  if (showTextInputBtn && textInputContainer) {
    showTextInputBtn.addEventListener('click', () => {
      const isVisible = textInputContainer.style.display !== 'none';
      textInputContainer.style.display = isVisible ? 'none' : 'block';
      showTextInputBtn.textContent = isVisible ? 'Or enter page numbers manually' : 'Hide manual input';
    });
  }

  // Setup keyboard navigation on thumbnail grid
  const pageGrid = document.getElementById('split-page-grid');
  if (pageGrid) {
    pageGrid.addEventListener('keydown', handleThumbnailKeyNavigation);
  }
}

/**
 * Setup quick select buttons for page selection
 */
function setupQuickSelectButtons(): void {
  const quickSelectRow = document.getElementById('split-quick-select');
  if (!quickSelectRow) return;

  quickSelectRow.addEventListener('click', (e) => {
    const target = e.target as HTMLElement;
    const btn = target.closest('.quick-select-btn') as HTMLButtonElement | null;
    if (!btn) return;

    const action = btn.dataset.action;
    if (!action) return;

    const totalPages = splitState.pages.length;
    let newSelection: number[] = [];

    switch (action) {
      case 'all':
        newSelection = splitState.pages.map(p => p.pageNumber);
        break;
      case 'none':
        newSelection = [];
        break;
      case 'odd':
        newSelection = splitState.pages.filter(p => p.pageNumber % 2 === 1).map(p => p.pageNumber);
        break;
      case 'even':
        newSelection = splitState.pages.filter(p => p.pageNumber % 2 === 0).map(p => p.pageNumber);
        break;
      case 'first5':
        newSelection = splitState.pages.slice(0, Math.min(5, totalPages)).map(p => p.pageNumber);
        break;
      case 'last5':
        newSelection = splitState.pages.slice(Math.max(0, totalPages - 5)).map(p => p.pageNumber);
        break;
    }

    // Clear current selection and set new
    splitState.selectedPageNumbers.clear();
    newSelection.forEach(n => splitState.selectedPageNumbers.add(n));
    splitState.pages.forEach(p => {
      p.isSelected = splitState.selectedPageNumbers.has(p.pageNumber);
    });

    updateThumbnailSelectionUI();
    updateSelectionSummary();
    updateSplitButtonState();

    announceToScreenReader(`${action === 'none' ? 'Cleared selection' : `Selected ${newSelection.length} pages`}`);
  });
}

/**
 * Handle keyboard navigation on thumbnail grid
 */
function handleThumbnailKeyNavigation(e: KeyboardEvent): void {
  const target = e.target as HTMLElement;
  const thumbnail = target.closest('.page-thumbnail') as HTMLElement | null;
  if (!thumbnail) return;

  const pageNum = parseInt(thumbnail.dataset.page || '0', 10);
  const thumbnails = Array.from(document.querySelectorAll('.page-thumbnail')) as HTMLElement[];
  const currentIndex = thumbnails.indexOf(thumbnail);
  const gridColumns = Math.floor((document.getElementById('split-page-grid')?.offsetWidth || 600) / 140);

  let newIndex = currentIndex;

  switch (e.key) {
    case 'ArrowRight':
      e.preventDefault();
      newIndex = Math.min(currentIndex + 1, thumbnails.length - 1);
      break;
    case 'ArrowLeft':
      e.preventDefault();
      newIndex = Math.max(currentIndex - 1, 0);
      break;
    case 'ArrowDown':
      e.preventDefault();
      newIndex = Math.min(currentIndex + gridColumns, thumbnails.length - 1);
      break;
    case 'ArrowUp':
      e.preventDefault();
      newIndex = Math.max(currentIndex - gridColumns, 0);
      break;
    case ' ':
    case 'Enter':
      e.preventDefault();
      togglePageSelection(pageNum, e.shiftKey);
      return;
  }

  if (newIndex !== currentIndex) {
    thumbnails[newIndex]?.focus();
  }
}

/**
 * Toggle selection of a page, with optional shift+click range selection
 */
function togglePageSelection(pageNumber: number, shiftKey: boolean): void {
  const pageIndex = pageNumber - 1;

  if (shiftKey && splitState.lastSelectedIndex !== null) {
    // Range selection: select all pages between last selected and current
    const start = Math.min(splitState.lastSelectedIndex, pageIndex);
    const end = Math.max(splitState.lastSelectedIndex, pageIndex);

    for (let i = start; i <= end; i++) {
      const page = splitState.pages[i];
      if (page) {
        page.isSelected = true;
        splitState.selectedPageNumbers.add(page.pageNumber);
      }
    }
  } else {
    // Single toggle
    const page = splitState.pages[pageIndex];
    if (page) {
      page.isSelected = !page.isSelected;
      if (page.isSelected) {
        splitState.selectedPageNumbers.add(pageNumber);
      } else {
        splitState.selectedPageNumbers.delete(pageNumber);
      }
    }
  }

  splitState.lastSelectedIndex = pageIndex;

  updateThumbnailSelectionUI();
  updateSelectionSummary();
  updateSplitButtonState();
}

/**
 * Update thumbnail visual selection state
 */
function updateThumbnailSelectionUI(): void {
  const thumbnails = document.querySelectorAll('.page-thumbnail');
  thumbnails.forEach((thumb) => {
    const pageNum = parseInt((thumb as HTMLElement).dataset.page || '0', 10);
    const isSelected = splitState.selectedPageNumbers.has(pageNum);
    thumb.classList.toggle('selected', isSelected);
    thumb.setAttribute('aria-pressed', String(isSelected));
  });
}

/**
 * Update selection summary display
 */
function updateSelectionSummary(): void {
  const countEl = document.getElementById('split-selected-count');
  const pagesEl = document.getElementById('split-selected-pages');

  if (!countEl || !pagesEl) return;

  const count = splitState.selectedPageNumbers.size;
  countEl.textContent = String(count);

  // Build page list string (e.g., "1, 2, 5-7, 10")
  const pageList = formatPageRanges(Array.from(splitState.selectedPageNumbers).sort((a, b) => a - b));
  pagesEl.textContent = count > 0 ? `(${pageList})` : '';
}

/**
 * Format array of page numbers into condensed range string
 * e.g., [1, 2, 3, 5, 7, 8, 9] -> "1-3, 5, 7-9"
 */
function formatPageRanges(pages: number[]): string {
  if (pages.length === 0) return '';

  const ranges: string[] = [];
  let rangeStart = pages[0];
  let rangeEnd = pages[0];

  for (let i = 1; i <= pages.length; i++) {
    if (i < pages.length && pages[i] === rangeEnd + 1) {
      rangeEnd = pages[i];
    } else {
      if (rangeStart === rangeEnd) {
        ranges.push(String(rangeStart));
      } else if (rangeEnd === rangeStart + 1) {
        ranges.push(`${rangeStart}, ${rangeEnd}`);
      } else {
        ranges.push(`${rangeStart}-${rangeEnd}`);
      }
      if (i < pages.length) {
        rangeStart = pages[i];
        rangeEnd = pages[i];
      }
    }
  }

  return ranges.join(', ');
}

/**
 * Update split button enabled/disabled state
 */
function updateSplitButtonState(): void {
  const splitBtn = document.getElementById('split-btn') as HTMLButtonElement | null;
  if (splitBtn) {
    splitBtn.disabled = splitState.selectedPageNumbers.size === 0;
  }
}

async function handleSplitFile(file: File): Promise<void> {
  if (!splitSession) return;
  const { format_bytes } = window.wasmBindings;

  try {
    // Check file size and warn if large
    if (file.size > VERY_LARGE_FILE_WARNING_BYTES) {
      if (
        !confirm(
          `This file is ${format_bytes(file.size)} which is very large. Processing may be slow or fail on some devices. Continue?`
        )
      ) {
        return;
      }
    } else if (file.size > LARGE_FILE_WARNING_BYTES) {
      console.warn(`Large file: ${format_bytes(file.size)} - processing may take longer`);
    }

    const bytes = new Uint8Array(await file.arrayBuffer());

    // IMPORTANT: Create copies BEFORE any operation that might detach the buffer
    // PDF.js getDocument() transfers ownership of ArrayBuffer, making original empty
    const bytesForWasm = bytes.slice();      // Copy for WASM session
    const bytesForShared = bytes.slice();    // Copy for shared state (cross-tab persistence)
    const bytesForThumbnails = bytes.slice(); // Copy for PDF.js thumbnail rendering

    const info: PdfInfo = splitSession.addDocument(file.name, bytesForWasm);

    // Store PDF bytes in shared state for Tab PDF Sharing (Phase 2)
    setSharedPdf(bytesForShared, file.name, 'split');

    // Store bytes in split state for thumbnail rendering
    splitState.pdfBytes = bytesForThumbnails;

    // Store original filename for smart output naming
    splitOriginalFilename = file.name.replace(/\.pdf$/i, '');

    // Update UI
    document.getElementById('split-drop-zone')?.classList.add('hidden');
    document.getElementById('split-editor')?.classList.remove('hidden');

    const fileNameEl = document.getElementById('split-file-name');
    const fileDetailsEl = document.getElementById('split-file-details');
    if (fileNameEl) fileNameEl.textContent = file.name;
    if (fileDetailsEl) fileDetailsEl.textContent = `${info.page_count} pages - ${format_bytes(info.size_bytes)}`;

    // Update quick select button visibility based on page count
    updateQuickSelectButtons(info.page_count);

    // Reset split state for new document
    splitState.pages = [];
    splitState.selectedPageNumbers.clear();
    splitState.separateFiles = false;
    splitState.lastSelectedIndex = null;

    // Initialize page thumbnails (visual selection - Phase 2 UX)
    // Use bytesForThumbnails since original bytes may be detached by WASM
    await renderPageThumbnails(bytesForThumbnails, info.page_count);

    // Reset UI state
    const splitBtn = document.getElementById('split-btn') as HTMLButtonElement | null;
    if (splitBtn) splitBtn.disabled = true;
    updateSelectionSummary();

    // ACCESSIBILITY: Announce to screen readers
    announceToScreenReader(`${file.name} loaded. ${info.page_count} pages. Click pages to select them for extraction.`);
  } catch (e) {
    showError('split-error', String(e));
    announceToScreenReader(`Error loading file: ${e}`);
  }
}

/**
 * Update quick select buttons based on page count
 */
function updateQuickSelectButtons(pageCount: number): void {
  const first5Btn = document.getElementById('split-first5-btn');
  const last5Btn = document.getElementById('split-last5-btn');

  // Hide First 5/Last 5 buttons if document has fewer than 5 pages
  if (first5Btn) first5Btn.style.display = pageCount >= 5 ? '' : 'none';
  if (last5Btn) last5Btn.style.display = pageCount >= 5 ? '' : 'none';
}

/**
 * Render page thumbnails using PDF.js
 */
async function renderPageThumbnails(bytes: Uint8Array, pageCount: number): Promise<void> {
  const pageGrid = document.getElementById('split-page-grid');
  const progressContainer = document.getElementById('split-thumbnail-progress');
  const progressFill = document.getElementById('split-thumbnail-progress-fill');
  const progressText = document.getElementById('split-thumbnail-progress-text');

  if (!pageGrid) return;

  // Show progress, hide grid temporarily
  if (progressContainer) progressContainer.classList.remove('hidden');
  pageGrid.innerHTML = '';

  try {
    // Lazy load PDF.js
    await ensurePdfJsLoaded();

    if (!window.pdfjsLib) {
      throw new Error('PDF.js not available');
    }

    // Load the PDF document
    splitState.pdfDoc = await window.pdfjsLib.getDocument(bytes).promise;

    // Render thumbnails one by one
    for (let i = 1; i <= pageCount; i++) {
      // Update progress
      if (progressFill) progressFill.style.width = `${(i / pageCount) * 100}%`;
      if (progressText) progressText.textContent = `${i} of ${pageCount} pages`;

      // Render page thumbnail
      const thumbnailDataUrl = await renderSinglePageThumbnail(splitState.pdfDoc, i);

      // Create thumbnail element
      const thumbnail: PageThumbnail = {
        pageNumber: i,
        thumbnailDataUrl,
        isSelected: false,
      };
      splitState.pages.push(thumbnail);

      // Add thumbnail to grid
      const thumbEl = createThumbnailElement(thumbnail);
      pageGrid.appendChild(thumbEl);
    }

    // Hide progress
    if (progressContainer) progressContainer.classList.add('hidden');

  } catch (e) {
    console.error('Failed to render thumbnails:', e);
    if (progressContainer) progressContainer.classList.add('hidden');

    // Show fallback message in grid
    pageGrid.innerHTML = `
      <div style="grid-column: 1 / -1; text-align: center; padding: 2rem; color: var(--text-muted);">
        <p>Could not load page previews.</p>
        <p>Use the quick select buttons or manual input below.</p>
      </div>
    `;

    // Create placeholder thumbnails without images
    for (let i = 1; i <= pageCount; i++) {
      splitState.pages.push({
        pageNumber: i,
        thumbnailDataUrl: '',
        isSelected: false,
      });
    }
  }
}

/**
 * Render a single page to a thumbnail data URL
 */
async function renderSinglePageThumbnail(pdfDoc: PDFJSDocument, pageNum: number): Promise<string> {
  const page = await pdfDoc.getPage(pageNum);
  const scale = 0.3; // Small preview scale
  const viewport = page.getViewport({ scale });

  const canvas = document.createElement('canvas');
  canvas.width = viewport.width;
  canvas.height = viewport.height;

  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('Could not get canvas context');

  await page.render({
    canvasContext: ctx,
    viewport,
  }).promise;

  return canvas.toDataURL('image/jpeg', 0.7); // Use JPEG for smaller size
}

/**
 * Create thumbnail DOM element
 */
function createThumbnailElement(thumbnail: PageThumbnail): HTMLButtonElement {
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = 'page-thumbnail';
  btn.dataset.page = String(thumbnail.pageNumber);
  btn.setAttribute('role', 'button');
  btn.setAttribute('aria-pressed', 'false');
  btn.setAttribute('aria-label', `Page ${thumbnail.pageNumber}, not selected`);
  btn.tabIndex = 0;

  btn.innerHTML = `
    <div class="page-thumbnail-preview">
      ${thumbnail.thumbnailDataUrl
        ? `<img src="${thumbnail.thumbnailDataUrl}" alt="Preview of page ${thumbnail.pageNumber}" loading="lazy" />`
        : `<span style="color: var(--text-muted);">${thumbnail.pageNumber}</span>`
      }
    </div>
    <span class="page-thumbnail-check" aria-hidden="true">&#10003;</span>
    <span class="page-thumbnail-number">${thumbnail.pageNumber}</span>
  `;

  // Click handler with shift+click support
  btn.addEventListener('click', (e) => {
    togglePageSelection(thumbnail.pageNumber, e.shiftKey);

    // Update aria-label
    const isSelected = splitState.selectedPageNumbers.has(thumbnail.pageNumber);
    btn.setAttribute('aria-label', `Page ${thumbnail.pageNumber}, ${isSelected ? 'selected' : 'not selected'}`);
  });

  return btn;
}

function resetSplitView(): void {
  if (!splitSession) return;

  splitSession.removeDocument(0);
  splitOriginalFilename = null;

  // Reset split state
  splitState.pages = [];
  splitState.selectedPageNumbers.clear();
  splitState.separateFiles = false;
  splitState.lastSelectedIndex = null;
  splitState.pdfBytes = null;
  if (splitState.pdfDoc) {
    splitState.pdfDoc.destroy();
    splitState.pdfDoc = null;
  }

  document.getElementById('split-drop-zone')?.classList.remove('hidden');
  document.getElementById('split-editor')?.classList.add('hidden');

  // Clear thumbnail grid
  const pageGrid = document.getElementById('split-page-grid');
  if (pageGrid) pageGrid.innerHTML = '';

  const fileInput = document.getElementById('split-file-input') as HTMLInputElement | null;
  const rangeInput = document.getElementById('page-range') as HTMLInputElement | null;
  const splitBtn = document.getElementById('split-btn') as HTMLButtonElement | null;
  const separateFilesCheckbox = document.getElementById('split-separate-files') as HTMLInputElement | null;

  if (fileInput) fileInput.value = '';
  if (rangeInput) rangeInput.value = '';
  if (splitBtn) splitBtn.disabled = true;
  if (separateFilesCheckbox) separateFilesCheckbox.checked = false;
}

function validateRange(): void {
  if (!splitSession) return;

  const rangeInput = document.getElementById('page-range') as HTMLInputElement | null;
  const splitBtn = document.getElementById('split-btn') as HTMLButtonElement | null;
  if (!rangeInput || !splitBtn) return;

  try {
    splitSession.setPageSelection(rangeInput.value);
    rangeInput.classList.remove('invalid');
    splitBtn.disabled = !splitSession.canExecute();
  } catch {
    rangeInput.classList.add('invalid');
    splitBtn.disabled = true;
  }
}

async function executeSplit(): Promise<void> {
  if (!splitSession) return;

  const splitBtn = document.getElementById('split-btn') as HTMLButtonElement | null;
  const progress = document.getElementById('split-progress');

  if (!splitBtn || !progress) return;

  splitBtn.disabled = true;
  progress.classList.remove('hidden');

  try {
    // Use visual selection if available, otherwise fall back to text input
    const selectedPages = Array.from(splitState.selectedPageNumbers).sort((a, b) => a - b);
    const separateFiles = splitState.separateFiles;

    if (selectedPages.length === 0) {
      // Fallback to legacy text input
      const rangeInput = document.getElementById('page-range') as HTMLInputElement | null;
      const multiFileCheckbox = document.getElementById('split-multiple-files') as HTMLInputElement | null;

      if (!rangeInput?.value) {
        showError('split-error', 'No pages selected. Click pages to select them.');
        return;
      }

      const fullRange = rangeInput.value;
      const isMultiFile = multiFileCheckbox?.checked;

      if (isMultiFile && fullRange.includes(',')) {
        const ranges = fullRange.split(',').map((r) => r.trim()).filter((r) => r);
        for (let i = 0; i < ranges.length; i++) {
          splitSession.setPageSelection(ranges[i]);
          const result = splitSession.execute();
          const rangeLabel = ranges[i].replace(/\s+/g, '');
          downloadBlob(result, `${splitOriginalFilename || 'split'}-pages-${rangeLabel}.pdf`);
          if (i < ranges.length - 1) await new Promise((r) => setTimeout(r, 100));
        }
        announceToScreenReader(`Split complete. ${ranges.length} files are downloading.`);
      } else {
        splitSession.setPageSelection(fullRange);
        const result = splitSession.execute();
        const range = fullRange.replace(/\s+/g, '').replace(/,/g, '_');
        downloadBlob(result, `${splitOriginalFilename || 'split'}-pages-${range}.pdf`);
        announceToScreenReader(`Split complete. File is downloading.`);
      }
    } else if (separateFiles) {
      // Separate files mode: one PDF per selected page
      const progressText = document.querySelector('#split-progress .progress-text');

      for (let i = 0; i < selectedPages.length; i++) {
        const pageNum = selectedPages[i];
        if (progressText) {
          progressText.textContent = `Processing page ${i + 1} of ${selectedPages.length}...`;
        }

        splitSession.setPageSelection(String(pageNum));
        const result = splitSession.execute();
        downloadBlob(result, `${splitOriginalFilename || 'split'}-page-${pageNum}.pdf`);

        if (i < selectedPages.length - 1) {
          await new Promise((r) => setTimeout(r, 100));
        }
      }
      announceToScreenReader(`Split complete. ${selectedPages.length} files are downloading.`);
    } else {
      // Single file with all selected pages
      const pageRange = formatPageRanges(selectedPages);
      splitSession.setPageSelection(pageRange);
      const result = splitSession.execute();

      const rangeLabel = pageRange.replace(/\s+/g, '').replace(/,/g, '_');
      const filename = `${splitOriginalFilename || 'split'}-pages-${rangeLabel}.pdf`;
      downloadBlob(result, filename);
      announceToScreenReader(`Split complete. ${filename} is downloading.`);
    }
  } catch (e) {
    showError('split-error', 'Split failed: ' + e);
    announceToScreenReader(`Split failed: ${e}`);
  } finally {
    splitBtn.disabled = false;
    setTimeout(() => progress.classList.add('hidden'), 500);
  }
}

function onSplitProgress(current: number, total: number, message: string): void {
  const progressFill = document.querySelector<HTMLElement>('#split-progress .progress-fill');
  const progressText = document.querySelector<HTMLElement>('#split-progress .progress-text');
  if (progressFill) progressFill.style.width = `${(current / total) * 100}%`;
  if (progressText) progressText.textContent = message;
}

function updateExampleChips(pageCount: number): void {
  const container = document.getElementById('range-chips');
  if (!container) return;

  container.innerHTML = '';

  // Generate dynamic chips based on page count
  const chips: Array<{ label: string; range: string }> = [];

  if (pageCount >= 1) {
    chips.push({ label: 'First page', range: '1' });
  }
  if (pageCount >= 5) {
    chips.push({ label: 'First 5', range: '1-5' });
  }
  if (pageCount >= 3) {
    const last3Start = pageCount - 2;
    chips.push({ label: 'Last 3', range: `${last3Start}-${pageCount}` });
  }
  if (pageCount >= 1) {
    chips.push({ label: 'All pages', range: `1-${pageCount}` });
  }

  chips.forEach(({ label, range }) => {
    const chip = document.createElement('button');
    chip.className = 'chip';
    chip.type = 'button';
    chip.textContent = label;
    chip.dataset.range = range;
    chip.addEventListener('click', () => {
      const rangeInput = document.getElementById('page-range') as HTMLInputElement | null;
      if (rangeInput) {
        rangeInput.value = range;
        validateRange();
      }
    });
    container.appendChild(chip);
  });
}

// ============ Merge View (Phase 3: Multi-Document Page Picker) ============

function setupMergeView(): void {
  const dropZone = document.getElementById('merge-drop-zone');
  const fileInput = document.getElementById('merge-file-input') as HTMLInputElement | null;
  const browseBtn = document.getElementById('merge-browse-btn');
  const addBtn = document.getElementById('merge-add-btn');
  const mergeBtn = document.getElementById('merge-btn');
  const pagePicker = document.getElementById('merge-page-picker');

  if (!dropZone || !fileInput || !browseBtn || !mergeBtn) return;

  browseBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    fileInput.click();
  });

  dropZone.addEventListener('click', () => fileInput.click());
  dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropZone.classList.add('drag-over');
  });
  dropZone.addEventListener('dragleave', () => dropZone.classList.remove('drag-over'));
  dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    dropZone.classList.remove('drag-over');
    if (e.dataTransfer?.files) {
      handleMergeFiles(e.dataTransfer.files);
    }
  });

  // Also allow drag-and-drop on the page picker for adding more files
  if (pagePicker) {
    pagePicker.addEventListener('dragover', (e) => {
      // Only accept file drops, not preview reorder drags
      if (e.dataTransfer?.types.includes('Files')) {
        e.preventDefault();
        pagePicker.classList.add('drag-over');
      }
    });
    pagePicker.addEventListener('dragleave', () => pagePicker.classList.remove('drag-over'));
    pagePicker.addEventListener('drop', (e) => {
      if (e.dataTransfer?.types.includes('Files')) {
        e.preventDefault();
        pagePicker.classList.remove('drag-over');
        if (e.dataTransfer?.files) {
          handleMergeFiles(e.dataTransfer.files);
        }
      }
    });
  }

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

  if (addBtn) addBtn.addEventListener('click', () => fileInput.click());
  mergeBtn.addEventListener('click', executeMerge);
}

async function handleMergeFiles(files: FileList): Promise<void> {
  // Convert FileList to array (FileList is live and may be cleared before async operations complete)
  const filesArray: File[] = [];
  for (let i = 0; i < files.length; i++) {
    filesArray.push(files[i]);
  }
  await handleMergeFilesArray(filesArray);
}

async function handleMergeFilesArray(files: File[]): Promise<void> {
  if (!mergeSession) return;
  const { format_bytes } = window.wasmBindings;

  // Track if this is the first document being added (for Tab PDF Sharing)
  const wasEmpty = mergeSession.getDocumentCount() === 0;

  // Lazy load PDF.js for thumbnail rendering
  await ensurePdfJsLoaded();

  // Process files
  for (const file of files) {
    if (file.type !== 'application/pdf') continue;

    // Check file size and warn if large
    if (file.size > VERY_LARGE_FILE_WARNING_BYTES) {
      if (
        !confirm(
          `"${file.name}" is ${format_bytes(file.size)} which is very large. Processing may be slow. Continue?`
        )
      ) {
        continue;
      }
    }

    try {
      const bytes = new Uint8Array(await file.arrayBuffer());

      // Keep copies for later use since WASM and PDF.js may detach buffers
      const bytesForWasm = bytes.slice();
      const bytesForPdfJs = bytes.slice();
      const bytesForStorage = bytes.slice();

      // Add to WASM session for actual merging (this detaches the buffer)
      const info = mergeSession.addDocument(file.name, bytesForWasm);

      // Load PDF.js document for thumbnail rendering (may also detach)
      const pdfDoc = await window.pdfjsLib!.getDocument(bytesForPdfJs).promise;

      // Generate unique ID for this document
      const docId = `doc-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

      // Generate short code (e.g., "C" for contract.pdf)
      const shortCode = generateShortCode(file.name);

      // Create merge document entry
      const mergeDoc: MergeDocument = {
        id: docId,
        filename: file.name,
        pdfBytes: bytesForStorage,
        pageCount: info.page_count,
        pages: [],
        isExpanded: mergeState.documents.length === 0, // First doc expanded by default
        shortCode,
        pdfDoc,
      };

      // Add all pages to merge order by default (selected)
      for (let i = 1; i <= info.page_count; i++) {
        mergeDoc.pages.push({
          pageNumber: i,
          thumbnailDataUrl: '', // Render lazily
          isSelected: true,
        });
        mergeState.mergeOrder.push({
          documentId: docId,
          pageNumber: i,
        });
      }

      mergeState.documents.push(mergeDoc);

      // Tab PDF Sharing (ISSUE-009): Set shared PDF to first document added
      if (wasEmpty && mergeSession.getDocumentCount() === 1) {
        setSharedPdf(bytesForStorage, file.name, 'merge');
      }
    } catch (e) {
      showError('merge-error', `${file.name}: ${e}`);
      announceToScreenReader(`Error loading ${file.name}: ${e}`);
    }
  }

  // Update the UI
  updateMergePagePicker();

  // Pre-render thumbnails for ALL documents (so preview strip works)
  preRenderAllMergeThumbnails();

  // ACCESSIBILITY: Announce loaded files to screen readers
  if (mergeSession) {
    const count = mergeSession.getDocumentCount();
    const totalPages = mergeSession.getTotalPageCount();
    if (count > 0) {
      announceToScreenReader(`${count} files loaded with ${totalPages} total pages. Select pages and drag to reorder.`);
    }
  }
}

/**
 * Pre-render thumbnails for all documents in background
 * This ensures preview strip has thumbnails even for collapsed documents
 */
async function preRenderAllMergeThumbnails(): Promise<void> {
  for (const doc of mergeState.documents) {
    if (!doc.pdfDoc) continue;
    for (const page of doc.pages) {
      if (!page.thumbnailDataUrl) {
        page.thumbnailDataUrl = await renderMergePageThumbnail(doc.pdfDoc, page.pageNumber);
      }
    }
  }
  // Update preview strip with rendered thumbnails
  renderMergePreviewStrip();
}

/**
 * Update the merge page picker UI (Phase 3)
 * Renders document accordions and merge preview strip
 */
function updateMergePagePicker(): void {
  const hasFiles = mergeState.documents.length > 0;

  // Show/hide appropriate sections
  document.getElementById('merge-drop-zone')?.classList.toggle('hidden', hasFiles);
  document.getElementById('merge-page-picker')?.classList.toggle('hidden', !hasFiles);

  if (!hasFiles) return;

  // Render document accordions
  renderMergeDocuments();

  // Render merge preview strip
  renderMergePreviewStrip();

  // Update merge button state
  updateMergeButtonState();
}

/**
 * Render the document accordion panels
 */
function renderMergeDocuments(): void {
  const container = document.getElementById('merge-documents');
  if (!container) return;

  container.innerHTML = '';

  mergeState.documents.forEach((doc, docIdx) => {
    const docEl = document.createElement('div');
    docEl.className = `merge-document${doc.isExpanded ? ' expanded' : ''}`;
    docEl.dataset.docId = doc.id;

    // Header
    const header = document.createElement('div');
    header.className = 'merge-document-header';
    header.innerHTML = `
      <span class="merge-document-icon" aria-hidden="true">&#128196;</span>
      <span class="merge-document-shortcode">${doc.shortCode}</span>
      <span class="merge-document-name" title="${doc.filename}">${doc.filename}</span>
      <span class="merge-document-pages">(${doc.pageCount} pages)</span>
      <button class="merge-document-toggle" aria-expanded="${doc.isExpanded}" aria-label="${doc.isExpanded ? 'Collapse' : 'Expand'} ${doc.filename}">
        &#9660;
      </button>
      <button class="merge-document-remove" aria-label="Remove ${doc.filename}">&#10005;</button>
    `;

    // Header click to toggle expand/collapse
    header.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      if (target.closest('.merge-document-remove')) return;

      doc.isExpanded = !doc.isExpanded;
      docEl.classList.toggle('expanded', doc.isExpanded);
      const toggleBtn = header.querySelector('.merge-document-toggle') as HTMLButtonElement;
      if (toggleBtn) {
        toggleBtn.setAttribute('aria-expanded', String(doc.isExpanded));
        toggleBtn.setAttribute('aria-label', `${doc.isExpanded ? 'Collapse' : 'Expand'} ${doc.filename}`);
      }

      if (doc.isExpanded) {
        renderDocumentThumbnails(doc, docEl.querySelector('.merge-document-content'));
      }
    });

    // Remove button
    const removeBtn = header.querySelector('.merge-document-remove');
    removeBtn?.addEventListener('click', async (e) => {
      e.stopPropagation();
      const confirmed = await showConfirmDialog({
        title: 'Remove Document?',
        message: `Are you sure you want to remove "${doc.filename}" from the merge list?`,
        confirmText: 'Remove',
        cancelText: 'Keep',
      });
      if (confirmed) {
        removeMergeDocument(docIdx);
      }
    });

    docEl.appendChild(header);

    // Content (page grid)
    const content = document.createElement('div');
    content.className = 'merge-document-content';

    const pageGrid = document.createElement('div');
    pageGrid.className = 'merge-page-grid';
    pageGrid.id = `merge-page-grid-${doc.id}`;

    // Quick select buttons
    const actions = document.createElement('div');
    actions.className = 'merge-page-actions';
    actions.innerHTML = `
      <button class="merge-quick-select" data-action="all">All</button>
      <button class="merge-quick-select" data-action="none">None</button>
    `;

    actions.querySelector('[data-action="all"]')?.addEventListener('click', () => {
      selectAllMergePages(doc, true);
    });
    actions.querySelector('[data-action="none"]')?.addEventListener('click', () => {
      selectAllMergePages(doc, false);
    });

    content.appendChild(pageGrid);
    content.appendChild(actions);
    docEl.appendChild(content);

    container.appendChild(docEl);

    if (doc.isExpanded) {
      renderDocumentThumbnails(doc, content);
    }
  });
}

/**
 * Render page thumbnails for a document (lazy loaded when accordion expands)
 */
async function renderDocumentThumbnails(doc: MergeDocument, content: Element | null): Promise<void> {
  if (!content) return;

  const grid = content.querySelector('.merge-page-grid');
  if (!grid) return;

  if (grid.children.length > 0) return;

  grid.innerHTML = '<div class="merge-page-loading">Loading thumbnails...</div>';

  try {
    for (let i = 0; i < doc.pages.length; i++) {
      const page = doc.pages[i];
      if (!page.thumbnailDataUrl && doc.pdfDoc) {
        page.thumbnailDataUrl = await renderMergePageThumbnail(doc.pdfDoc, page.pageNumber);
      }
    }

    grid.innerHTML = '';

    doc.pages.forEach((page) => {
      const thumb = document.createElement('button');
      thumb.className = `merge-page-thumb${page.isSelected ? ' selected' : ''}`;
      thumb.type = 'button';
      thumb.setAttribute('role', 'checkbox');
      thumb.setAttribute('aria-checked', String(page.isSelected));
      thumb.setAttribute('aria-label', `Page ${page.pageNumber}${page.isSelected ? ', selected' : ''}`);

      thumb.innerHTML = `
        <img class="merge-page-canvas" src="${page.thumbnailDataUrl}" alt="Page ${page.pageNumber}" />
        <span class="merge-page-num">${page.pageNumber}</span>
      `;

      thumb.addEventListener('click', () => {
        toggleMergePageSelection(doc, page);
        thumb.classList.toggle('selected', page.isSelected);
        thumb.setAttribute('aria-checked', String(page.isSelected));
        thumb.setAttribute('aria-label', `Page ${page.pageNumber}${page.isSelected ? ', selected' : ''}`);
      });

      grid.appendChild(thumb);
    });

    // Re-render preview strip now that thumbnails are available
    renderMergePreviewStrip();
  } catch (e) {
    grid.innerHTML = '<div class="merge-page-loading" style="color: var(--error)">Failed to load thumbnails</div>';
    console.error('Failed to render thumbnails:', e);
  }
}

/**
 * Toggle page selection for a document page in merge view
 */
function toggleMergePageSelection(doc: MergeDocument, page: PageThumbnail): void {
  page.isSelected = !page.isSelected;

  if (page.isSelected) {
    mergeState.mergeOrder.push({
      documentId: doc.id,
      pageNumber: page.pageNumber,
    });
  } else {
    mergeState.mergeOrder = mergeState.mergeOrder.filter(
      (ref) => !(ref.documentId === doc.id && ref.pageNumber === page.pageNumber)
    );
  }

  renderMergePreviewStrip();
  updateMergeButtonState();
}

/**
 * Select or deselect all pages in a document
 */
function selectAllMergePages(doc: MergeDocument, select: boolean): void {
  doc.pages.forEach((page) => {
    if (page.isSelected !== select) {
      page.isSelected = select;

      if (select) {
        const exists = mergeState.mergeOrder.some(
          (ref) => ref.documentId === doc.id && ref.pageNumber === page.pageNumber
        );
        if (!exists) {
          mergeState.mergeOrder.push({
            documentId: doc.id,
            pageNumber: page.pageNumber,
          });
        }
      } else {
        mergeState.mergeOrder = mergeState.mergeOrder.filter(
          (ref) => !(ref.documentId === doc.id && ref.pageNumber === page.pageNumber)
        );
      }
    }
  });

  const grid = document.getElementById(`merge-page-grid-${doc.id}`);
  if (grid) {
    const thumbs = grid.querySelectorAll('.merge-page-thumb');
    thumbs.forEach((thumb, idx) => {
      const page = doc.pages[idx];
      thumb.classList.toggle('selected', page.isSelected);
      thumb.setAttribute('aria-checked', String(page.isSelected));
    });
  }

  renderMergePreviewStrip();
  updateMergeButtonState();
}

/**
 * Remove a document from the merge state
 */
function removeMergeDocument(docIdx: number): void {
  const doc = mergeState.documents[docIdx];
  if (!doc) return;

  mergeState.mergeOrder = mergeState.mergeOrder.filter((ref) => ref.documentId !== doc.id);
  mergeState.documents.splice(docIdx, 1);
  mergeSession?.removeDocument(docIdx);

  if (doc.pdfDoc) {
    doc.pdfDoc.destroy();
  }

  updateMergePagePicker();
}

/**
 * Render the merge preview strip (bottom panel)
 */
function renderMergePreviewStrip(): void {
  const strip = document.getElementById('merge-preview-strip');
  const summaryText = document.getElementById('merge-summary-text');
  if (!strip) return;

  strip.innerHTML = '';

  if (mergeState.mergeOrder.length === 0) {
    if (summaryText) summaryText.textContent = 'No pages selected';
    return;
  }

  mergeState.mergeOrder.forEach((ref, idx) => {
    const doc = mergeState.documents.find((d) => d.id === ref.documentId);
    if (!doc) return;

    const page = doc.pages.find((p) => p.pageNumber === ref.pageNumber);
    if (!page) return;

    const previewPage = document.createElement('div');
    previewPage.className = 'merge-preview-page';
    previewPage.draggable = true;
    previewPage.dataset.orderIdx = String(idx);
    previewPage.setAttribute('role', 'listitem');
    previewPage.tabIndex = 0;

    const label = `${doc.shortCode}${ref.pageNumber}`;
    previewPage.setAttribute('aria-label', `${doc.filename} page ${ref.pageNumber}. Drag to reorder.`);

    // Use placeholder if thumbnail not yet rendered
    const thumbContent = page.thumbnailDataUrl
      ? `<img class="merge-preview-page-img" src="${page.thumbnailDataUrl}" alt="${label}" />`
      : `<div class="merge-preview-page-placeholder">${label}</div>`;

    previewPage.innerHTML = `
      ${thumbContent}
      <span class="merge-preview-label">${label}</span>
      <button class="merge-preview-remove" aria-label="Remove ${label}">&times;</button>
    `;

    previewPage.querySelector('.merge-preview-remove')?.addEventListener('click', (e) => {
      e.stopPropagation();
      removeFromMergeOrder(idx);
    });

    previewPage.addEventListener('dragstart', onPreviewDragStart);
    previewPage.addEventListener('dragover', onPreviewDragOver);
    previewPage.addEventListener('drop', onPreviewDrop);
    previewPage.addEventListener('dragend', onPreviewDragEnd);

    strip.appendChild(previewPage);
  });

  const uniqueDocs = new Set(mergeState.mergeOrder.map((ref) => ref.documentId));
  if (summaryText) {
    summaryText.textContent = `Total: ${mergeState.mergeOrder.length} pages from ${uniqueDocs.size} document${uniqueDocs.size !== 1 ? 's' : ''}`;
  }
}

/**
 * Remove a page from the merge order (deselects it)
 */
function removeFromMergeOrder(orderIdx: number): void {
  const ref = mergeState.mergeOrder[orderIdx];
  if (!ref) return;

  const doc = mergeState.documents.find((d) => d.id === ref.documentId);
  if (doc) {
    const page = doc.pages.find((p) => p.pageNumber === ref.pageNumber);
    if (page) {
      page.isSelected = false;

      const grid = document.getElementById(`merge-page-grid-${doc.id}`);
      if (grid) {
        const thumbs = grid.querySelectorAll('.merge-page-thumb');
        thumbs.forEach((thumb, idx) => {
          if (doc.pages[idx]?.pageNumber === ref.pageNumber) {
            thumb.classList.remove('selected');
            thumb.setAttribute('aria-checked', 'false');
          }
        });
      }
    }
  }

  mergeState.mergeOrder.splice(orderIdx, 1);
  renderMergePreviewStrip();
  updateMergeButtonState();
}

// Preview strip drag and drop
let previewDraggedIdx: number | null = null;

function onPreviewDragStart(e: DragEvent): void {
  const target = e.target as HTMLElement;
  previewDraggedIdx = parseInt(target.dataset.orderIdx || '0', 10);
  target.classList.add('dragging');
  e.dataTransfer?.setData('text/plain', String(previewDraggedIdx));
}

function onPreviewDragOver(e: DragEvent): void {
  e.preventDefault();
  const page = (e.target as HTMLElement).closest('.merge-preview-page');
  document.querySelectorAll('.merge-preview-page.drop-target').forEach((el) => {
    el.classList.remove('drop-target');
  });
  if (page) page.classList.add('drop-target');
}

function onPreviewDrop(e: DragEvent): void {
  e.preventDefault();
  const page = (e.target as HTMLElement).closest('.merge-preview-page') as HTMLElement | null;
  if (!page) return;

  const dropIdx = parseInt(page.dataset.orderIdx || '0', 10);
  if (previewDraggedIdx !== null && previewDraggedIdx !== dropIdx) {
    const [moved] = mergeState.mergeOrder.splice(previewDraggedIdx, 1);
    mergeState.mergeOrder.splice(dropIdx, 0, moved);
    renderMergePreviewStrip();
  }
}

function onPreviewDragEnd(): void {
  previewDraggedIdx = null;
  document.querySelectorAll('.merge-preview-page.dragging, .merge-preview-page.drop-target').forEach((el) => {
    el.classList.remove('dragging', 'drop-target');
  });
}

/**
 * Update merge button state based on selection
 */
function updateMergeButtonState(): void {
  const mergeBtn = document.getElementById('merge-btn') as HTMLButtonElement | null;
  if (mergeBtn) {
    mergeBtn.disabled = mergeState.mergeOrder.length === 0;
  }
}

/**
 * Legacy function for backwards compatibility with tab switching
 */
function updateMergeFileList(): void {
  updateMergePagePicker();
}

async function executeMerge(): Promise<void> {
  if (!mergeSession) return;
  const { PdfJoinSession, SessionMode } = window.wasmBindings;

  const mergeBtn = document.getElementById('merge-btn') as HTMLButtonElement | null;
  const progress = document.getElementById('merge-progress');

  if (!mergeBtn || !progress) return;

  mergeBtn.disabled = true;
  progress.classList.remove('hidden');

  try {
    // Phase 3: Page-level merge using mergeState.mergeOrder
    // Since WASM doesn't support page selection in merge, we extract pages using split first

    // Group merge order by document
    const docPages = new Map<string, number[]>();
    mergeState.mergeOrder.forEach((ref) => {
      if (!docPages.has(ref.documentId)) {
        docPages.set(ref.documentId, []);
      }
      docPages.get(ref.documentId)!.push(ref.pageNumber);
    });

    // Create a new merge session for the final result
    const finalMerge = new PdfJoinSession(SessionMode.Merge);

    // Process documents in the order they appear in mergeOrder
    const processedDocs = new Set<string>();
    for (const ref of mergeState.mergeOrder) {
      if (processedDocs.has(ref.documentId)) continue;
      processedDocs.add(ref.documentId);

      const doc = mergeState.documents.find((d) => d.id === ref.documentId);
      if (!doc) continue;

      const pages = docPages.get(ref.documentId)!;
      const allPages = pages.length === doc.pageCount && pages.every((p, i) => p === i + 1);

      if (allPages) {
        // All pages selected in order - add whole document
        // pdfBytes is already a dedicated copy, but slice again for WASM ownership
        finalMerge.addDocument(doc.filename, doc.pdfBytes.slice());
      } else {
        // Extract only selected pages using split session
        const splitSession = new PdfJoinSession(SessionMode.Split);
        // pdfBytes is already a dedicated copy, but slice again for WASM ownership
        splitSession.addDocument(doc.filename, doc.pdfBytes.slice());
        const pageRange = pages.join(',');
        splitSession.setPageSelection(pageRange);
        const extractedBytes = splitSession.execute();
        finalMerge.addDocument(`${doc.filename}-pages`, extractedBytes);
      }
    }

    let result: Uint8Array;
    let filename: string;
    const totalPages = mergeState.mergeOrder.length;

    if (finalMerge.getDocumentCount() === 1) {
      // Single document - no actual merge needed, just get the extracted bytes directly
      // Re-extract the pages since finalMerge can't execute with 1 doc
      const ref = mergeState.mergeOrder[0];
      const doc = mergeState.documents.find((d) => d.id === ref.documentId);
      if (doc) {
        const pages = docPages.get(ref.documentId)!;
        const allPages = pages.length === doc.pageCount && pages.every((p, i) => p === i + 1);
        if (allPages) {
          result = doc.pdfBytes.slice();
        } else {
          const splitSession = new PdfJoinSession(SessionMode.Split);
          splitSession.addDocument(doc.filename, doc.pdfBytes.slice());
          splitSession.setPageSelection(pages.join(','));
          result = splitSession.execute();
        }
        filename = `${doc.filename.replace(/\.pdf$/i, '')}-${totalPages}-pages.pdf`;
      } else {
        throw new Error('Document not found');
      }
    } else {
      result = finalMerge.execute();
      filename = `merged-${totalPages}-pages.pdf`;
    }
    downloadBlob(result, filename);
    announceToScreenReader(`Merge complete. ${filename} is downloading.`);
  } catch (e) {
    showError('merge-error', 'Merge failed: ' + e);
    announceToScreenReader(`Merge failed: ${e}`);
  } finally {
    mergeBtn.disabled = false;
    setTimeout(() => progress.classList.add('hidden'), 500);
  }
}

function onMergeProgress(current: number, total: number, message: string): void {
  const progressFill = document.querySelector<HTMLElement>('#merge-progress .progress-fill');
  const progressText = document.querySelector<HTMLElement>('#merge-progress .progress-text');
  if (progressFill) progressFill.style.width = `${(current / total) * 100}%`;
  if (progressText) progressText.textContent = message;
}

// ============ Utilities ============

/**
 * ACCESSIBILITY: Convert raw error messages into user-friendly messages
 * with clear recovery guidance (WCAG 3.3.3 Error Suggestion).
 */
function getUserFriendlyError(rawMessage: string): string {
  const lowerMsg = rawMessage.toLowerCase();

  // Password/encryption errors
  if (lowerMsg.includes('password') || lowerMsg.includes('encrypted')) {
    return 'This PDF is password-protected. Please remove the password using Adobe Acrobat or a PDF unlocker, then try again.';
  }

  // Invalid PDF format
  if (lowerMsg.includes('invalid pdf') || lowerMsg.includes('not a pdf') || lowerMsg.includes('magic bytes')) {
    return 'This file is not a valid PDF. Please check that you selected the correct file. If the file is a Word document, save it as PDF first.';
  }

  // File size issues
  if (lowerMsg.includes('too large') || lowerMsg.includes('memory')) {
    return 'This file is too large to process. Try splitting it into smaller parts using Adobe Acrobat, then try again.';
  }

  // Page range errors
  if (lowerMsg.includes('page') && (lowerMsg.includes('invalid') || lowerMsg.includes('range'))) {
    return 'Invalid page range. Use format like "1-3, 5, 8-10". Pages must exist in the document.';
  }

  // Corrupted PDF
  if (lowerMsg.includes('corrupt') || lowerMsg.includes('damaged') || lowerMsg.includes('parse')) {
    return 'This PDF appears to be corrupted. Try downloading it again or opening and re-saving it in Adobe Acrobat.';
  }

  // Network errors (if applicable)
  if (lowerMsg.includes('network') || lowerMsg.includes('fetch')) {
    return 'A network error occurred. Please check your internet connection and try again.';
  }

  // Generic fallback with guidance
  return `${rawMessage}. If this keeps happening, try refreshing the page or using a different browser.`;
}

function showError(containerId: string, message: string): void {
  const container = document.getElementById(containerId);
  if (!container) return;

  const textEl = container.querySelector('.error-text');
  const dismissBtn = container.querySelector('.dismiss');

  // ACCESSIBILITY: Convert to user-friendly error with recovery guidance
  const friendlyMessage = getUserFriendlyError(message);
  if (textEl) textEl.textContent = friendlyMessage;
  container.classList.remove('hidden');

  // ACCESSIBILITY: Extended auto-dismiss (20 seconds) for elderly users to read
  const timer = setTimeout(() => container.classList.add('hidden'), 20000);

  // Manual dismiss
  if (dismissBtn) {
    (dismissBtn as HTMLElement).onclick = (): void => {
      clearTimeout(timer);
      container.classList.add('hidden');
    };
  }
}

function downloadBlob(data: Uint8Array, filename: string): void {
  const blob = new Blob([data as unknown as BlobPart], { type: 'application/pdf' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
