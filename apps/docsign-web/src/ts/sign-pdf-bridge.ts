/**
 * Sign PDF Bridge - Bridge between sign.js and PdfPreviewBridge
 *
 * This module exposes PDF loading and rendering functions to the window.DocSign
 * namespace for use by sign.js. It uses PdfPreviewBridge internally.
 *
 * Architecture:
 * - sign.js handles signing workflow, UI, and state
 * - This bridge handles PDF loading and canvas rendering
 * - PdfPreviewBridge handles low-level PDF.js operations
 */

import { PdfPreviewBridge } from "./pdf-preview";
import type { PageDimensions } from "./types/pdf-types";

// Import error handling modules
import {
  getUserFriendlyError,
  categorizeError,
  createUserError,
  getOfflineError,
  getFileTooLargeError,
  getUnsupportedFileError,
} from "./error-messages";
import type { UserError, ErrorIcon, ErrorCategory } from "./error-messages";

import {
  showErrorModal,
  hideErrorModal,
  showErrorToast,
  hideErrorToast,
  showConfirmDialog,
} from "./error-ui";
import type { ToastType } from "./error-ui";

// Import sync manager
import {
  SyncManager,
  getSyncManager,
  initSyncManager,
  type SyncStatus,
  type SyncError,
  type SyncManagerConfig,
} from "./sync-manager";

import {
  SYNC_EVENTS,
  onSyncStarted,
  onSyncCompleted,
  onSyncFailed,
  onSyncProgress,
  onOnlineStatusChanged,
} from "./sync-events";
import { createLogger } from "./logger";

const log = createLogger('DocSign');

// Re-export error types for external use
export type { UserError, ErrorIcon, ErrorCategory, ToastType, SyncStatus, SyncError, SyncManagerConfig };

// Re-export error functions for ES module consumers
export {
  getUserFriendlyError,
  categorizeError,
  createUserError,
  getOfflineError,
  getFileTooLargeError,
  getUnsupportedFileError,
  showErrorModal,
  hideErrorModal,
  showErrorToast,
  hideErrorToast,
  showConfirmDialog,
};

// Re-export sync manager
export {
  SyncManager,
  getSyncManager,
  initSyncManager,
  SYNC_EVENTS,
  onSyncStarted,
  onSyncCompleted,
  onSyncFailed,
  onSyncProgress,
  onOnlineStatusChanged,
};

/**
 * Result from loading a PDF
 */
export interface LoadPdfResult {
  numPages: number;
  /** Whether the load was successful */
  success: boolean;
  /** Error message if load failed */
  error?: string;
}

/**
 * Result from rendering a page
 */
export interface RenderPageResult {
  pageNum: number;
  dimensions: PageDimensions;
  canvas: HTMLCanvasElement;
  success: boolean;
  error?: string;
}

/**
 * Configuration for rendering pages
 */
export interface RenderConfig {
  /** Container element to append pages to */
  container: HTMLElement;
  /** Scale factor for rendering (default: 1.5) */
  scale?: number;
  /** CSS class for page wrapper divs */
  pageWrapperClass?: string;
}

/**
 * DocSign PDF Bridge namespace - exposed on window.DocSign
 */
export interface DocSignPdfBridge {
  /**
   * Load a PDF document from binary data
   * @param data PDF bytes as Uint8Array, ArrayBuffer, or base64 string
   * @returns Promise with load result including page count
   */
  loadPdf(data: Uint8Array | ArrayBuffer | string): Promise<LoadPdfResult>;

  /**
   * Render all pages from the currently loaded PDF
   * @param config Render configuration
   * @returns Promise with array of render results
   */
  renderAllPages(config: RenderConfig): Promise<RenderPageResult[]>;

  /**
   * Render a single page to a canvas
   * @param pageNum 1-indexed page number
   * @param canvas Canvas element to render to
   * @param scale Render scale (default: 1.5)
   * @returns Promise with render result including dimensions
   */
  renderPage(pageNum: number, canvas: HTMLCanvasElement, scale?: number): Promise<RenderPageResult>;

  /**
   * Get the number of pages in the currently loaded document
   */
  getPageCount(): number;

  /**
   * Get dimensions for a rendered page
   * @param pageNum 1-indexed page number
   */
  getPageDimensions(pageNum: number): { width: number; height: number } | null;

  /**
   * Cleanup resources - call when done with document
   */
  cleanup(): void;

  /**
   * Check if a document is currently loaded
   */
  isDocumentLoaded(): boolean;
}

/**
 * Convert base64 string to Uint8Array
 */
function base64ToUint8Array(base64: string): Uint8Array {
  // Remove data URL prefix if present
  const cleanBase64 = base64.replace(/^data:[^;]+;base64,/, "");
  const binaryString = atob(cleanBase64);
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes;
}

/**
 * Create the DocSign PDF bridge implementation
 */
function createDocSignPdfBridge(): DocSignPdfBridge {
  let pageCount = 0;

  return {
    async loadPdf(data: Uint8Array | ArrayBuffer | string): Promise<LoadPdfResult> {
      try {
        // Clean up any previous document
        PdfPreviewBridge.cleanup();

        // Convert data to Uint8Array if needed
        let bytes: Uint8Array | ArrayBuffer;
        if (typeof data === "string") {
          bytes = base64ToUint8Array(data);
        } else {
          bytes = data;
        }

        // Load document using PdfPreviewBridge
        pageCount = await PdfPreviewBridge.loadDocument(bytes);

        log.info("PDF loaded:", pageCount, "pages");

        return {
          numPages: pageCount,
          success: true,
        };
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        log.error("Failed to load PDF:", errorMessage);
        pageCount = 0;
        return {
          numPages: 0,
          success: false,
          error: errorMessage,
        };
      }
    },

    async renderAllPages(config: RenderConfig): Promise<RenderPageResult[]> {
      const { container, scale = 1.5, pageWrapperClass = "pdf-page-wrapper" } = config;
      const results: RenderPageResult[] = [];

      if (!PdfPreviewBridge.currentDoc) {
        log.error("No document loaded");
        return results;
      }

      // Clear container
      container.innerHTML = "";

      for (let pageNum = 1; pageNum <= pageCount; pageNum++) {
        // Create page wrapper
        const pageWrapper = document.createElement("div");
        pageWrapper.className = pageWrapperClass;
        pageWrapper.dataset.pageNumber = String(pageNum);

        // Create canvas
        const canvas = document.createElement("canvas");

        // Add canvas to wrapper, wrapper to container
        pageWrapper.appendChild(canvas);
        container.appendChild(pageWrapper);

        // Render page
        const result = await this.renderPage(pageNum, canvas, scale);
        results.push(result);
      }

      log.debug("Rendered", results.length, "pages");
      return results;
    },

    async renderPage(
      pageNum: number,
      canvas: HTMLCanvasElement,
      scale = 1.5
    ): Promise<RenderPageResult> {
      try {
        const dimensions = await PdfPreviewBridge.renderPage(pageNum, canvas, scale);

        return {
          pageNum,
          dimensions,
          canvas,
          success: true,
        };
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        log.error(`Failed to render page ${pageNum}:`, errorMessage);
        return {
          pageNum,
          dimensions: {
            width: 0,
            height: 0,
            originalWidth: 0,
            originalHeight: 0,
            pdfWidth: 0,
            pdfHeight: 0,
          },
          canvas,
          success: false,
          error: errorMessage,
        };
      }
    },

    getPageCount(): number {
      return pageCount;
    },

    getPageDimensions(pageNum: number): { width: number; height: number } | null {
      return PdfPreviewBridge.getPageDimensions(pageNum);
    },

    cleanup(): void {
      PdfPreviewBridge.cleanup();
      pageCount = 0;
      log.debug("Cleaned up PDF resources");
    },

    isDocumentLoaded(): boolean {
      return PdfPreviewBridge.currentDoc !== null;
    },
  };
}

// Create singleton instance
export const docSignPdfBridge = createDocSignPdfBridge();

// Expose on window.DocSign namespace
declare global {
  interface Window {
    DocSign?: {
      // PDF bridge functions
      loadPdf: DocSignPdfBridge["loadPdf"];
      renderAllPages: DocSignPdfBridge["renderAllPages"];
      renderPage: DocSignPdfBridge["renderPage"];
      getPageCount: DocSignPdfBridge["getPageCount"];
      getPageDimensions: DocSignPdfBridge["getPageDimensions"];
      cleanup: DocSignPdfBridge["cleanup"];
      isDocumentLoaded: DocSignPdfBridge["isDocumentLoaded"];
      // Error message functions
      getUserFriendlyError: typeof getUserFriendlyError;
      categorizeError: typeof categorizeError;
      createUserError: typeof createUserError;
      getOfflineError: typeof getOfflineError;
      getFileTooLargeError: typeof getFileTooLargeError;
      getUnsupportedFileError: typeof getUnsupportedFileError;
      // Error UI functions
      showErrorModal: typeof showErrorModal;
      hideErrorModal: typeof hideErrorModal;
      showErrorToast: typeof showErrorToast;
      hideErrorToast: typeof hideErrorToast;
      showConfirmDialog: typeof showConfirmDialog;
      // Sync Manager
      SyncManager: typeof SyncManager;
      getSyncManager: typeof getSyncManager;
      initSyncManager: typeof initSyncManager;
      SYNC_EVENTS: typeof SYNC_EVENTS;
      onSyncStarted: typeof onSyncStarted;
      onSyncCompleted: typeof onSyncCompleted;
      onSyncFailed: typeof onSyncFailed;
      onSyncProgress: typeof onSyncProgress;
      onOnlineStatusChanged: typeof onOnlineStatusChanged;
    };
  }
}

/**
 * Initialize the DocSign namespace on window
 */
export function initDocSignNamespace(): void {
  window.DocSign = {
    // PDF bridge functions
    loadPdf: docSignPdfBridge.loadPdf.bind(docSignPdfBridge),
    renderAllPages: docSignPdfBridge.renderAllPages.bind(docSignPdfBridge),
    renderPage: docSignPdfBridge.renderPage.bind(docSignPdfBridge),
    getPageCount: docSignPdfBridge.getPageCount.bind(docSignPdfBridge),
    getPageDimensions: docSignPdfBridge.getPageDimensions.bind(docSignPdfBridge),
    cleanup: docSignPdfBridge.cleanup.bind(docSignPdfBridge),
    isDocumentLoaded: docSignPdfBridge.isDocumentLoaded.bind(docSignPdfBridge),
    // Error message functions
    getUserFriendlyError,
    categorizeError,
    createUserError,
    getOfflineError,
    getFileTooLargeError,
    getUnsupportedFileError,
    // Error UI functions
    showErrorModal,
    hideErrorModal,
    showErrorToast,
    hideErrorToast,
    showConfirmDialog,
    // Sync Manager
    SyncManager,
    getSyncManager,
    initSyncManager,
    SYNC_EVENTS,
    onSyncStarted,
    onSyncCompleted,
    onSyncFailed,
    onSyncProgress,
    onOnlineStatusChanged,
  };
  log.info("PDF bridge, error handling, and sync manager initialized on window.DocSign");
}
