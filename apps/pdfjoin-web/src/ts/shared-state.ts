// Shared state for PDF bytes across tabs
// Enables Tab PDF Sharing feature (Phase 2)

export interface SharedPdfState {
  bytes: Uint8Array | null;
  filename: string | null;
  source: 'split' | 'merge' | 'edit' | null;
}

// Module-level state
let sharedPdf: SharedPdfState = {
  bytes: null,
  filename: null,
  source: null,
};

/**
 * Store PDF bytes in shared state
 */
export function setSharedPdf(bytes: Uint8Array, filename: string, source: 'split' | 'merge' | 'edit'): void {
  sharedPdf = { bytes, filename, source };
}

/**
 * Get the shared PDF state
 */
export function getSharedPdf(): SharedPdfState {
  return sharedPdf;
}

/**
 * Check if there's a shared PDF available
 */
export function hasSharedPdf(): boolean {
  return sharedPdf.bytes !== null && sharedPdf.bytes.length > 0;
}

/**
 * Clear the shared PDF state
 */
export function clearSharedPdf(): void {
  sharedPdf = { bytes: null, filename: null, source: null };
}

// ============================================================================
// Edit Session Change Detection (for Edit → Split modal)
// ============================================================================

// Callback type for checking if edit session has changes
type HasChangesCallback = () => boolean;
type ExportCallback = () => Uint8Array | null;

let hasChangesCallback: HasChangesCallback | null = null;
let exportCallback: ExportCallback | null = null;

/**
 * Register callbacks from edit.ts for change detection
 */
export function registerEditCallbacks(hasChanges: HasChangesCallback, exportFn: ExportCallback): void {
  hasChangesCallback = hasChanges;
  exportCallback = exportFn;
}

/**
 * Check if the edit session has unsaved changes
 */
export function editHasChanges(): boolean {
  return hasChangesCallback ? hasChangesCallback() : false;
}

/**
 * Export the edited PDF (returns null if no session or export fails)
 */
export function exportEditedPdf(): Uint8Array | null {
  return exportCallback ? exportCallback() : null;
}

/**
 * Clear edit callbacks (called when edit session is reset)
 */
export function clearEditCallbacks(): void {
  hasChangesCallback = null;
  exportCallback = null;
}
