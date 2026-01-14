// PDF Preview Bridge for DocSign - Preview Only (No Editing)
// Simplified version of pdfjoin-web's pdf-bridge.ts
// Uses lazy loading to avoid bloating the main bundle

import { ensurePdfJsLoaded } from './pdf-loader';
import type {
  PDFJSDocument,
  PDFJSPage,
  PDFJSViewport,
  CachedPageInfo,
  PageDimensions,
  TextItem,
  IPdfPreviewBridge,
  PdfJsTextContent,
  DomBounds,
} from './types/pdf-types';

/**
 * PdfPreviewBridge - Preview-only interface between TypeScript and PDF.js
 *
 * Capabilities:
 * - loadDocument: Load a PDF from bytes
 * - renderPage: Render a page to a canvas
 * - getPageDimensions: Get cached page dimensions
 * - extractTextWithPositions: Extract text with position info
 * - cleanup: Release resources
 *
 * NOT included (editing is handled separately in docsign):
 * - addText, addHighlight, addCheckbox, etc.
 * - Any modification operations
 */
export const PdfPreviewBridge: IPdfPreviewBridge = {
  currentDoc: null,
  pageCanvases: new Map<number, CachedPageInfo>(),

  /**
   * Load a PDF document from bytes
   * @param data PDF file as Uint8Array or ArrayBuffer
   * @returns Number of pages in the document
   */
  async loadDocument(data: Uint8Array | ArrayBuffer): Promise<number> {
    // Lazy load PDF.js on first use
    await ensurePdfJsLoaded();

    const typedArray = data instanceof Uint8Array ? data : new Uint8Array(data);

    if (!window.pdfjsLib) {
      throw new Error('PDF.js not loaded');
    }

    this.currentDoc = await window.pdfjsLib.getDocument(typedArray).promise;
    return this.currentDoc.numPages;
  },

  /**
   * Render a page to a canvas element
   * @param pageNum 1-indexed page number
   * @param canvas Canvas element to render to
   * @param scale Rendering scale (default 1.5 for good quality)
   * @returns Page dimensions in various coordinate systems
   */
  async renderPage(pageNum: number, canvas: HTMLCanvasElement, scale = 1.5): Promise<PageDimensions> {
    if (!this.currentDoc) throw new Error('No document loaded');

    const page: PDFJSPage = await this.currentDoc.getPage(pageNum);
    const viewport: PDFJSViewport = page.getViewport({ scale });

    canvas.width = viewport.width;
    canvas.height = viewport.height;

    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('Could not get 2d context');

    await page.render({
      canvasContext: ctx,
      viewport: viewport,
    }).promise;

    this.pageCanvases.set(pageNum, { canvas, viewport, page });

    return {
      width: viewport.width,
      height: viewport.height,
      originalWidth: viewport.width / scale,
      originalHeight: viewport.height / scale,
      pdfWidth: page.view[2],
      pdfHeight: page.view[3],
    };
  },

  /**
   * Get cached page dimensions
   * @param pageNum 1-indexed page number
   * @returns Page dimensions or null if not rendered
   */
  getPageDimensions(pageNum: number): { width: number; height: number } | null {
    const cached = this.pageCanvases.get(pageNum);
    if (cached) {
      return {
        width: cached.viewport.width,
        height: cached.viewport.height,
      };
    }
    return null;
  },

  /**
   * Get cached page info (canvas, viewport, page)
   * @param pageNum 1-indexed page number
   * @returns Cached page info or undefined
   */
  getPageInfo(pageNum: number): CachedPageInfo | undefined {
    return this.pageCanvases.get(pageNum);
  },

  /**
   * Extract text with position information for each text item
   * Useful for signature field detection and text overlay
   * @param pageNum 1-indexed page number
   * @returns Array of text items with position and style info
   */
  async extractTextWithPositions(pageNum: number): Promise<TextItem[]> {
    if (!this.currentDoc) throw new Error('No document loaded');

    const page = await this.currentDoc.getPage(pageNum);
    const textContent: PdfJsTextContent = await page.getTextContent();
    const cached = this.pageCanvases.get(pageNum);
    const viewport = cached?.viewport;

    // Get font styles map (fontName -> { fontFamily, ascent, descent, vertical })
    const styles = textContent.styles || {};

    return textContent.items.map((item, index): TextItem => {
      // PDF.js transform: [scaleX, skewX, skewY, scaleY, x, y]
      const pdfX = item.transform[4];
      const pdfY = item.transform[5];
      const pdfWidth = item.width || 0;
      const pdfHeight = item.height || 12; // Default font height

      // Font size is the absolute value of the scaleY component (transform[3])
      // This represents how tall the font renders in PDF points
      const fontSize = Math.abs(item.transform[3]) || item.height || 12;

      // Get font family from styles (e.g., "serif", "sans-serif", "monospace")
      const fontStyle = item.fontName ? styles[item.fontName] : undefined;
      const fontFamily = fontStyle?.fontFamily || 'sans-serif';

      // Detect italic/bold from font name
      // Font names often contain style info: "Times-Italic", "Helvetica-Bold", etc.
      const fontNameLower = (item.fontName || '').toLowerCase();
      const isItalic = fontNameLower.includes('italic') || fontNameLower.includes('oblique');
      const isBold = fontNameLower.includes('bold');

      // Convert PDF coords to DOM coords if viewport available
      let domBounds: DomBounds | null = null;
      let domFontSize = fontSize; // DOM-scaled font size
      if (viewport) {
        // PDF origin is bottom-left, viewport is top-left
        const [domX, domY] = viewport.convertToViewportPoint(pdfX, pdfY);
        const [domX2, domY2] = viewport.convertToViewportPoint(pdfX + pdfWidth, pdfY + pdfHeight);
        domBounds = {
          x: Math.min(domX, domX2),
          y: Math.min(domY, domY2),
          width: Math.abs(domX2 - domX) || pdfWidth * viewport.scale,
          height: Math.abs(domY2 - domY) || pdfHeight * viewport.scale,
        };
        // Scale font size to match viewport
        domFontSize = fontSize * viewport.scale;
      }

      return {
        index,
        str: item.str,
        pdfX,
        pdfY,
        pdfWidth,
        pdfHeight,
        fontSize, // PDF font size in points
        domFontSize, // Font size scaled to viewport (pixels)
        fontName: item.fontName,
        fontFamily, // "serif", "sans-serif", or "monospace"
        isItalic, // true if font name contains "italic" or "oblique"
        isBold, // true if font name contains "bold"
        domBounds,
      };
    });
  },

  /**
   * Cleanup resources - call when done with the document
   */
  cleanup(): void {
    if (this.currentDoc) {
      this.currentDoc.destroy();
      this.currentDoc = null;
    }
    this.pageCanvases.clear();
  },
};

// Export singleton instance
export const previewBridge = PdfPreviewBridge;

// Expose on window for debugging and backwards compatibility
declare global {
  interface Window {
    PdfPreviewBridge?: IPdfPreviewBridge;
  }
}

window.PdfPreviewBridge = PdfPreviewBridge;
