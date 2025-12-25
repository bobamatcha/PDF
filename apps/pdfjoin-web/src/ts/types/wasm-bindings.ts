// WASM bindings type definitions for pdfjoin-wasm
// These types match the Rust structs exposed via wasm-bindgen

/**
 * Operation ID type - standardized as bigint
 * Rust returns u64, wasm-bindgen converts to BigInt
 */
export type OpId = bigint;

/**
 * Session modes for PdfJoinSession
 */
export enum SessionMode {
  Split = 0,
  Merge = 1,
}

/**
 * PDF document validation info returned from WASM
 */
export interface PdfInfo {
  page_count: number;
  version: string;
  encrypted: boolean;
  size_bytes: number;
  valid: boolean;
  title?: string;
  author?: string;
}

/**
 * Document info in merged list
 */
export interface DocumentInfo {
  name: string;
  page_count: number;
  size_bytes: number;
  version: string;
  encrypted: boolean;
}

/**
 * Page orientation
 */
export type PageOrientation = 'Portrait' | 'Landscape' | 'Square';

/**
 * Page info from WASM
 */
export interface WasmPageInfo {
  page_num: number;
  width: number;
  height: number;
  rotation: number;
  has_content: boolean;
  orientation: PageOrientation;
}

/**
 * Progress callback signature
 */
export type ProgressCallback = (current: number, total: number, message: string) => void;

/**
 * PdfJoinSession class interface (split/merge operations)
 */
export interface PdfJoinSession {
  readonly mode: SessionMode;

  setProgressCallback(callback: ProgressCallback): void;
  addDocument(name: string, bytes: Uint8Array): PdfInfo;
  removeDocument(index: number): void;
  reorderDocuments(newOrder: number[]): void;
  setPageSelection(rangeStr: string): void;
  getSelectedPages(): number[];
  getPageInfo(docIndex: number, pageNum: number): WasmPageInfo;
  getDocumentInfos(): DocumentInfo[];
  getTotalPageCount(): number;
  getDocumentCount(): number;
  canExecute(): boolean;
  execute(): Uint8Array;
}

/**
 * PdfJoinSession constructor
 */
export interface PdfJoinSessionConstructor {
  new (mode: SessionMode): PdfJoinSession;
}

/**
 * EditSession class interface (PDF editing operations)
 */
export interface EditSession {
  readonly isSigned: boolean;
  readonly pageCount: number;
  readonly documentName: string;

  getDocumentBytes(): Uint8Array;

  addText(
    page: number,
    x: number,
    y: number,
    width: number,
    height: number,
    text: string,
    fontSize: number,
    color: string,
    fontName: string | null,
    isItalic: boolean,
    isBold: boolean
  ): OpId;

  addHighlight(
    page: number,
    x: number,
    y: number,
    width: number,
    height: number,
    color: string,
    opacity: number
  ): OpId;

  addCheckbox(
    page: number,
    x: number,
    y: number,
    width: number,
    height: number,
    checked: boolean
  ): OpId;

  replaceText(
    page: number,
    origX: number,
    origY: number,
    origWidth: number,
    origHeight: number,
    newX: number,
    newY: number,
    newWidth: number,
    newHeight: number,
    originalText: string,
    newText: string,
    fontSize: number,
    color: string,
    fontName: string | null,
    isItalic: boolean,
    isBold: boolean
  ): OpId;

  addWhiteRect(
    page: number,
    x: number,
    y: number,
    width: number,
    height: number
  ): OpId;

  removeOperation(id: OpId): boolean;
  hasChanges(): boolean;
  getOperationCount(): number;
  getOperationsJson(): string;
  export(): Uint8Array;

  // Action-based undo/redo (Phase 4)
  beginAction(kind: ActionKind): void;
  commitAction(): boolean;
  abortAction(): void;
  undo(): BigInt64Array | null;
  redo(): BigInt64Array | null;
  canUndo(): boolean;
  canRedo(): boolean;
  getOperationJson(id: OpId): string | null;
  recordRemovedOp(id: OpId): boolean;
  setCheckbox(id: OpId, checked: boolean): boolean;
}

/**
 * Action kinds for grouping operations
 */
export type ActionKind =
  | 'textbox'
  | 'whiteout'
  | 'checkbox'
  | 'highlight'
  | 'replacetext'
  | 'move'
  | 'resize'
  | 'delete';

/**
 * EditSession constructor
 */
export interface EditSessionConstructor {
  new (name: string, bytes: Uint8Array): EditSession;
}

/**
 * Complete WASM bindings interface
 */
export interface WasmBindings {
  PdfJoinSession: PdfJoinSessionConstructor;
  EditSession: EditSessionConstructor;
  SessionMode: typeof SessionMode;
  format_bytes: (bytes: number) => string;
  get_version: () => string;
  quick_validate: (bytes: Uint8Array) => void;
  get_pdf_info: (bytes: Uint8Array) => PdfInfo;
  get_page_count: (bytes: Uint8Array) => number;
}

/**
 * Helper function to get OpId from DOM element dataset
 * Always returns BigInt for consistency
 */
export function getOpId(element: HTMLElement): OpId | null {
  const id = element.dataset.opId;
  if (!id) return null;
  try {
    return BigInt(id);
  } catch {
    return null;
  }
}

/**
 * Helper function to set OpId on DOM element dataset
 * Converts BigInt to string for dataset storage
 */
export function setOpId(element: HTMLElement, opId: OpId): void {
  element.dataset.opId = opId.toString();
}
