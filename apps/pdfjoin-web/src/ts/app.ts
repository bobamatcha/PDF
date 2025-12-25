// PDFJoin - Single Page App
// Uses window.wasmBindings from Trunk-injected WASM loader

import { setupEditView, loadPdfIntoEdit } from './edit';
import { setSharedPdf, getSharedPdf, hasSharedPdf, editHasChanges, exportEditedPdf } from './shared-state';
import type { PdfJoinSession, PdfInfo, DocumentInfo } from './types';

// Size thresholds
const LARGE_FILE_WARNING_BYTES = 50 * 1024 * 1024; // 50 MB
const VERY_LARGE_FILE_WARNING_BYTES = 100 * 1024 * 1024; // 100 MB

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
  setupTabs();
  setupSplitView();
  setupMergeView();
  setupEditView(); // Initialize edit tab

  console.log('PDFJoin initialized (WASM-first architecture)');
}

// ============ Tab Navigation ============

function setupTabs(): void {
  const tabs = document.querySelectorAll<HTMLElement>('.tab');
  tabs.forEach((tab) => {
    tab.addEventListener('click', async () => {
      const tabName = tab.dataset.tab;
      const currentTab = document.querySelector('.tab.active')?.getAttribute('data-tab');

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
              loadPdfIntoSplit(shared.bytes, shared.filename);
            } else if (tabName === 'merge') {
              // ISSUE-009: Edit → Merge - add document to merge list
              loadPdfIntoMerge(shared.bytes, shared.filename);
            }
          }
        }
      }

      // Split → Merge: Add shared document to merge list (ISSUE-009)
      if (currentTab === 'split' && tabName === 'merge') {
        if (hasSharedPdf()) {
          const shared = getSharedPdf();
          if (shared.bytes && shared.filename) {
            loadPdfIntoMerge(shared.bytes, shared.filename);
          }
        }
      }

      // Merge → Split: Load first merged document into Split (ISSUE-009)
      if (currentTab === 'merge' && tabName === 'split') {
        if (mergeSession && mergeSession.getDocumentCount() > 0) {
          try {
            const bytes = mergeSession.getDocumentBytes(0);
            const name = mergeSession.getDocumentName(0);
            loadPdfIntoSplit(bytes, name);
            // Also update shared state
            setSharedPdf(bytes, name, 'merge');
          } catch (e) {
            console.error('Failed to load merge document into split:', e);
          }
        }
      }

      // Update active tab
      tabs.forEach((t) => t.classList.remove('active'));
      tab.classList.add('active');

      // Show corresponding view
      document.querySelectorAll<HTMLElement>('.view').forEach((v) => v.classList.add('hidden'));
      const view = document.getElementById(`${tabName}-view`);
      if (view) view.classList.remove('hidden');

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
 */
function loadPdfIntoMerge(bytes: Uint8Array, filename: string): void {
  if (!mergeSession) return;

  // Check if document is already in the merge list (by name)
  const infos = mergeSession.getDocumentInfos();
  const alreadyExists = infos.some((info) => info.name === filename);
  if (alreadyExists) {
    console.log(`Document "${filename}" already in merge list, skipping`);
    return;
  }

  try {
    mergeSession.addDocument(filename, bytes);
    updateMergeFileList();
    console.log(`Added "${filename}" to merge list from tab switch`);
  } catch (e) {
    console.error('Failed to add document to merge:', e);
  }
}

/**
 * Load PDF bytes directly into the Split tab (for Tab PDF Sharing)
 */
function loadPdfIntoSplit(bytes: Uint8Array, filename: string): void {
  if (!splitSession) return;
  const { format_bytes } = window.wasmBindings;

  try {
    // Clear any existing document first
    if (splitSession.getDocumentCount() > 0) {
      splitSession.removeDocument(0);
    }

    const info: PdfInfo = splitSession.addDocument(filename, bytes);

    // Store original filename for smart output naming
    splitOriginalFilename = filename.replace(/\.pdf$/i, '');

    // Update UI
    document.getElementById('split-drop-zone')?.classList.add('hidden');
    document.getElementById('split-editor')?.classList.remove('hidden');

    const fileNameEl = document.getElementById('split-file-name');
    const fileDetailsEl = document.getElementById('split-file-details');
    if (fileNameEl) fileNameEl.textContent = filename;
    if (fileDetailsEl) fileDetailsEl.textContent = `${info.page_count} pages - ${format_bytes(info.size_bytes)}`;

    // Update example chips with page count
    updateExampleChips(info.page_count);

    // Reset range input
    const rangeInput = document.getElementById('page-range') as HTMLInputElement | null;
    const splitBtn = document.getElementById('split-btn') as HTMLButtonElement | null;
    if (rangeInput) rangeInput.value = '';
    if (splitBtn) splitBtn.disabled = true;
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

  if (!dropZone || !fileInput || !browseBtn || !removeBtn || !splitBtn || !rangeInput) return;

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
  rangeInput.addEventListener('input', validateRange);
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
    const info: PdfInfo = splitSession.addDocument(file.name, bytes);

    // Store PDF bytes in shared state for Tab PDF Sharing (Phase 2)
    setSharedPdf(bytes, file.name, 'split');

    // Store original filename for smart output naming
    splitOriginalFilename = file.name.replace(/\.pdf$/i, '');

    // Update UI
    document.getElementById('split-drop-zone')?.classList.add('hidden');
    document.getElementById('split-editor')?.classList.remove('hidden');

    const fileNameEl = document.getElementById('split-file-name');
    const fileDetailsEl = document.getElementById('split-file-details');
    if (fileNameEl) fileNameEl.textContent = file.name;
    if (fileDetailsEl) fileDetailsEl.textContent = `${info.page_count} pages - ${format_bytes(info.size_bytes)}`;

    // Update example chips with page count
    updateExampleChips(info.page_count);

    // Don't auto-fill range - let placeholder show syntax examples
    const rangeInput = document.getElementById('page-range') as HTMLInputElement | null;
    const splitBtn = document.getElementById('split-btn') as HTMLButtonElement | null;
    if (rangeInput) rangeInput.value = '';
    if (splitBtn) splitBtn.disabled = true;

    // ACCESSIBILITY: Announce to screen readers
    announceToScreenReader(`${file.name} loaded. ${info.page_count} pages. Enter page range to split.`);
  } catch (e) {
    showError('split-error', String(e));
    announceToScreenReader(`Error loading file: ${e}`);
  }
}

function resetSplitView(): void {
  if (!splitSession) return;

  splitSession.removeDocument(0);
  splitOriginalFilename = null;

  document.getElementById('split-drop-zone')?.classList.remove('hidden');
  document.getElementById('split-editor')?.classList.add('hidden');

  const fileInput = document.getElementById('split-file-input') as HTMLInputElement | null;
  const rangeInput = document.getElementById('page-range') as HTMLInputElement | null;
  const splitBtn = document.getElementById('split-btn') as HTMLButtonElement | null;

  if (fileInput) fileInput.value = '';
  if (rangeInput) rangeInput.value = '';
  if (splitBtn) splitBtn.disabled = true;
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
  const rangeInput = document.getElementById('page-range') as HTMLInputElement | null;
  const multiFileCheckbox = document.getElementById('split-multiple-files') as HTMLInputElement | null;

  if (!splitBtn || !progress || !rangeInput) return;

  splitBtn.disabled = true;
  progress.classList.remove('hidden');

  try {
    const isMultiFile = multiFileCheckbox?.checked;
    const fullRange = rangeInput.value;

    if (isMultiFile && fullRange.includes(',')) {
      // Multi-file mode: split each comma-separated range into its own file
      const ranges = fullRange
        .split(',')
        .map((r) => r.trim())
        .filter((r) => r);

      for (let i = 0; i < ranges.length; i++) {
        const range = ranges[i];
        // Update progress
        const progressText = document.querySelector('#split-progress .progress-text');
        if (progressText) {
          progressText.textContent = `Processing range ${i + 1} of ${ranges.length}...`;
        }

        // Set selection to just this range and execute
        splitSession.setPageSelection(range);
        const result = splitSession.execute();

        // Download with range-specific filename
        const rangeLabel = range.replace(/\s+/g, '');
        const filename = `${splitOriginalFilename || 'split'}-pages-${rangeLabel}.pdf`;
        downloadBlob(result, filename);

        // Small delay between downloads to avoid browser issues
        if (i < ranges.length - 1) {
          await new Promise((r) => setTimeout(r, 100));
        }
      }

      // Restore original selection
      splitSession.setPageSelection(fullRange);
      announceToScreenReader(`Split complete. ${ranges.length} files are downloading.`);
    } else {
      // Single file mode (original behavior)
      const result = splitSession.execute();
      const range = fullRange.replace(/\s+/g, '').replace(/,/g, '_');
      const filename = `${splitOriginalFilename || 'split'}-pages-${range}.pdf`;
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

// ============ Merge View ============

function setupMergeView(): void {
  const dropZone = document.getElementById('merge-drop-zone');
  const fileInput = document.getElementById('merge-file-input') as HTMLInputElement | null;
  const browseBtn = document.getElementById('merge-browse-btn');
  const addBtn = document.getElementById('merge-add-btn');
  const mergeBtn = document.getElementById('merge-btn');
  const fileList = document.getElementById('merge-file-list');

  if (!dropZone || !fileInput || !browseBtn || !addBtn || !mergeBtn || !fileList) return;

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

  // Also allow drag-and-drop on the file list for adding more files
  fileList.addEventListener('dragover', (e) => {
    e.preventDefault();
    fileList.classList.add('drag-over');
  });
  fileList.addEventListener('dragleave', () => fileList.classList.remove('drag-over'));
  fileList.addEventListener('drop', (e) => {
    e.preventDefault();
    fileList.classList.remove('drag-over');
    if (e.dataTransfer?.files) {
      handleMergeFiles(e.dataTransfer.files);
    }
  });

  fileInput.addEventListener('change', () => {
    if (fileInput.files) {
      handleMergeFiles(fileInput.files);
      fileInput.value = ''; // Allow re-selecting same files
    }
  });

  addBtn.addEventListener('click', () => fileInput.click());
  mergeBtn.addEventListener('click', executeMerge);
}

async function handleMergeFiles(files: FileList): Promise<void> {
  if (!mergeSession) return;
  const { format_bytes } = window.wasmBindings;

  // Track if this is the first document being added (for Tab PDF Sharing)
  const wasEmpty = mergeSession.getDocumentCount() === 0;

  // Convert FileList to array to ensure proper iteration
  // FileList is a live collection and for...of may not iterate all items
  const fileArray = Array.from(files);
  for (const file of fileArray) {
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
      mergeSession.addDocument(file.name, bytes);

      // Tab PDF Sharing (ISSUE-009): Set shared PDF to first document added
      // This enables Merge → Split/Edit transitions
      if (wasEmpty && mergeSession.getDocumentCount() === 1) {
        setSharedPdf(bytes, file.name, 'merge');
      }
    } catch (e) {
      showError('merge-error', `${file.name}: ${e}`);
      announceToScreenReader(`Error loading ${file.name}: ${e}`);
    }
  }

  updateMergeFileList();

  // ACCESSIBILITY: Announce loaded files to screen readers
  if (mergeSession) {
    const count = mergeSession.getDocumentCount();
    const totalPages = mergeSession.getTotalPageCount();
    if (count > 0) {
      announceToScreenReader(`${count} files loaded with ${totalPages} total pages. Ready to merge.`);
    }
  }
}

/**
 * ACCESSIBILITY: Move a file in the merge list from one position to another.
 * Used by move up/down buttons and arrow key navigation.
 */
function moveFile(fromIndex: number, toIndex: number): void {
  if (!mergeSession) return;

  const count = mergeSession.getDocumentCount();
  if (fromIndex < 0 || fromIndex >= count || toIndex < 0 || toIndex >= count) return;

  // Build new order array
  const order = [...Array(count).keys()];
  order.splice(fromIndex, 1);
  order.splice(toIndex, 0, fromIndex);

  try {
    mergeSession.reorderDocuments(order);
    updateMergeFileList();
  } catch (e) {
    console.error('Reorder failed:', e);
  }
}

function updateMergeFileList(): void {
  if (!mergeSession) return;
  const { format_bytes } = window.wasmBindings;

  const infos: DocumentInfo[] = mergeSession.getDocumentInfos();
  const count = mergeSession.getDocumentCount();

  // Show/hide appropriate sections
  const hasFiles = count > 0;
  document.getElementById('merge-drop-zone')?.classList.toggle('hidden', hasFiles);
  document.getElementById('merge-file-list')?.classList.toggle('hidden', !hasFiles);

  // Update count and total size
  const totalSize = infos.reduce((sum, info) => sum + info.size_bytes, 0);
  const totalPages = infos.reduce((sum, info) => sum + info.page_count, 0);
  const countEl = document.getElementById('merge-count');
  if (countEl) {
    countEl.textContent = `(${count} files, ${totalPages} pages, ${format_bytes(totalSize)})`;
  }

  // Update file list
  const ul = document.getElementById('merge-files');
  if (!ul) return;
  ul.innerHTML = '';

  const totalFiles = infos.length;
  infos.forEach((info, idx) => {
    const li = document.createElement('li');
    li.draggable = true;
    li.dataset.index = String(idx);
    // ACCESSIBILITY: Make focusable for keyboard users
    li.tabIndex = 0;
    li.setAttribute('role', 'listitem');
    li.setAttribute('aria-label', `${info.name}, ${info.page_count} pages. Use arrow keys or buttons to reorder.`);
    li.innerHTML = `
            <span class="drag-handle" aria-hidden="true">☰</span>
            <span class="file-name">${info.name}</span>
            <span class="file-size">${info.page_count} pages - ${format_bytes(info.size_bytes)}</span>
            <div class="reorder-btns">
              <button class="move-btn move-up" title="Move up" aria-label="Move ${info.name} up" ${idx === 0 ? 'disabled' : ''}>↑</button>
              <button class="move-btn move-down" title="Move down" aria-label="Move ${info.name} down" ${idx === totalFiles - 1 ? 'disabled' : ''}>↓</button>
            </div>
            <button class="remove-btn" data-index="${idx}" aria-label="Remove ${info.name}">×</button>
        `;

    // ACCESSIBILITY: Move up/down buttons for keyboard/motor impairment accessibility
    const moveUpBtn = li.querySelector('.move-up') as HTMLButtonElement | null;
    const moveDownBtn = li.querySelector('.move-down') as HTMLButtonElement | null;

    moveUpBtn?.addEventListener('click', (e) => {
      e.stopPropagation();
      if (idx > 0) {
        moveFile(idx, idx - 1);
      }
    });

    moveDownBtn?.addEventListener('click', (e) => {
      e.stopPropagation();
      if (idx < totalFiles - 1) {
        moveFile(idx, idx + 1);
      }
    });

    // ACCESSIBILITY: Arrow key support for reordering
    li.addEventListener('keydown', (e) => {
      if (e.key === 'ArrowUp' && idx > 0) {
        e.preventDefault();
        moveFile(idx, idx - 1);
        // Focus the moved item after update
        setTimeout(() => {
          const newItem = ul.querySelector(`li[data-index="${idx - 1}"]`) as HTMLElement;
          newItem?.focus();
        }, 0);
      } else if (e.key === 'ArrowDown' && idx < totalFiles - 1) {
        e.preventDefault();
        moveFile(idx, idx + 1);
        // Focus the moved item after update
        setTimeout(() => {
          const newItem = ul.querySelector(`li[data-index="${idx + 1}"]`) as HTMLElement;
          newItem?.focus();
        }, 0);
      }
    });

    // Remove button - ACCESSIBILITY: Confirm before removing (WCAG 3.3.4)
    const removeBtn = li.querySelector('.remove-btn');
    removeBtn?.addEventListener('click', async () => {
      const confirmed = await showConfirmDialog({
        title: 'Remove File?',
        message: `Are you sure you want to remove "${info.name}" from the merge list? You can add it again later.`,
        confirmText: 'Remove',
        cancelText: 'Keep',
      });
      if (confirmed) {
        mergeSession?.removeDocument(idx);
        updateMergeFileList();
      }
    });

    // Drag events for reordering
    li.addEventListener('dragstart', onDragStart);
    li.addEventListener('dragover', onDragOver);
    li.addEventListener('drop', onDrop);
    li.addEventListener('dragend', onDragEnd);

    ul.appendChild(li);
  });

  // Update merge button state
  const mergeBtn = document.getElementById('merge-btn') as HTMLButtonElement | null;
  if (mergeBtn) mergeBtn.disabled = !mergeSession.canExecute();
}

// Drag and drop reordering
let draggedIndex: number | null = null;

function onDragStart(e: DragEvent): void {
  const target = e.target as HTMLElement;
  draggedIndex = parseInt(target.dataset.index || '0', 10);
  target.classList.add('dragging');
}

function onDragOver(e: DragEvent): void {
  e.preventDefault();
  const li = (e.target as HTMLElement).closest('li');
  if (li) li.classList.add('drag-over');
}

function onDrop(e: DragEvent): void {
  e.preventDefault();
  if (!mergeSession) return;

  const li = (e.target as HTMLElement).closest('li') as HTMLElement | null;
  if (!li) return;

  const dropIndex = parseInt(li.dataset.index || '0', 10);
  if (draggedIndex !== null && draggedIndex !== dropIndex) {
    // Build new order
    const count = mergeSession.getDocumentCount();
    const order = [...Array(count).keys()];
    order.splice(draggedIndex, 1);
    order.splice(dropIndex, 0, draggedIndex);

    try {
      mergeSession.reorderDocuments(order);
      updateMergeFileList();
    } catch (e) {
      console.error('Reorder failed:', e);
    }
  }
}

function onDragEnd(): void {
  draggedIndex = null;
  document.querySelectorAll('.dragging, .drag-over').forEach((el) => {
    el.classList.remove('dragging', 'drag-over');
  });
}

async function executeMerge(): Promise<void> {
  if (!mergeSession) return;

  const mergeBtn = document.getElementById('merge-btn') as HTMLButtonElement | null;
  const progress = document.getElementById('merge-progress');

  if (!mergeBtn || !progress) return;

  mergeBtn.disabled = true;
  progress.classList.remove('hidden');

  try {
    const result = mergeSession.execute();
    // Smart filename: merged-3-files.pdf
    const count = mergeSession.getDocumentCount();
    const filename = `merged-${count}-files.pdf`;
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
