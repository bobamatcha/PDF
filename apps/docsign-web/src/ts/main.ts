/**
 * DocSign TypeScript Entry Point
 *
 * This is the main entry point for the TypeScript codebase.
 * It will eventually replace the inlined JavaScript in sign.html.
 *
 * Architecture: Preview-only PDF rendering (no editing)
 * - PDF.js renders pages to canvas (read-only)
 * - Signature fields overlay on top (not embedded until signing)
 * - All signing happens locally in WASM
 */

// Import core modules
import { ensurePdfJsLoaded, isPdfJsLoaded } from "./pdf-loader";
import { PdfPreviewBridge, previewBridge } from "./pdf-preview";
import {
  domRectToPdf,
  pdfRectToDom,
  domPointToPdf,
  pdfPointToDom,
  getPageRenderInfo,
} from "./coord-utils";
import type { IPdfPreviewBridge, PageDimensions, TextItem } from "./types/pdf-types";

// Re-export for backwards compatibility and external access
export {
  ensurePdfJsLoaded,
  isPdfJsLoaded,
  PdfPreviewBridge,
  previewBridge,
  domRectToPdf,
  pdfRectToDom,
  domPointToPdf,
  pdfPointToDom,
  getPageRenderInfo,
};

export type { IPdfPreviewBridge, PageDimensions, TextItem };

/**
 * Initialize DocSign application
 */
function init(): void {
  console.log("DocSign TypeScript initialized");
  console.log("PDF Preview Bridge available:", typeof PdfPreviewBridge !== "undefined");
}

// Initialize when DOM is ready
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
