// src/ts/pdf-loader.ts
var pdfJsLoaded = false;
var pdfJsLoadPromise = null;
async function ensurePdfJsLoaded() {
  if (pdfJsLoaded) {
    return;
  }
  if (pdfJsLoadPromise) {
    return pdfJsLoadPromise;
  }
  pdfJsLoadPromise = new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = "./js/vendor/pdf.min.js";
    script.onload = () => {
      if (window.pdfjsLib) {
        window.pdfjsLib.GlobalWorkerOptions.workerSrc = "./js/vendor/pdf.worker.min.js";
        pdfJsLoaded = true;
        console.log("PDF.js loaded successfully (lazy)");
        resolve();
      } else {
        reject(new Error("PDF.js loaded but pdfjsLib not found on window"));
      }
    };
    script.onerror = (e) => {
      pdfJsLoadPromise = null;
      const errorEvent = e;
      reject(new Error("Failed to load PDF.js: " + (errorEvent.message || "Unknown error")));
    };
    document.head.appendChild(script);
  });
  return pdfJsLoadPromise;
}
function isPdfJsLoaded() {
  return pdfJsLoaded;
}
window.ensurePdfJsLoaded = ensurePdfJsLoaded;

// src/ts/pdf-preview.ts
var PdfPreviewBridge = {
  currentDoc: null,
  pageCanvases: /* @__PURE__ */ new Map(),
  /**
   * Load a PDF document from bytes
   * @param data PDF file as Uint8Array or ArrayBuffer
   * @returns Number of pages in the document
   */
  async loadDocument(data) {
    await ensurePdfJsLoaded();
    const typedArray = data instanceof Uint8Array ? data : new Uint8Array(data);
    if (!window.pdfjsLib) {
      throw new Error("PDF.js not loaded");
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
  async renderPage(pageNum, canvas, scale = 1.5) {
    if (!this.currentDoc) throw new Error("No document loaded");
    const page = await this.currentDoc.getPage(pageNum);
    const viewport = page.getViewport({ scale });
    canvas.width = viewport.width;
    canvas.height = viewport.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Could not get 2d context");
    await page.render({
      canvasContext: ctx,
      viewport
    }).promise;
    this.pageCanvases.set(pageNum, { canvas, viewport, page });
    return {
      width: viewport.width,
      height: viewport.height,
      originalWidth: viewport.width / scale,
      originalHeight: viewport.height / scale,
      pdfWidth: page.view[2],
      pdfHeight: page.view[3]
    };
  },
  /**
   * Get cached page dimensions
   * @param pageNum 1-indexed page number
   * @returns Page dimensions or null if not rendered
   */
  getPageDimensions(pageNum) {
    const cached = this.pageCanvases.get(pageNum);
    if (cached) {
      return {
        width: cached.viewport.width,
        height: cached.viewport.height
      };
    }
    return null;
  },
  /**
   * Get cached page info (canvas, viewport, page)
   * @param pageNum 1-indexed page number
   * @returns Cached page info or undefined
   */
  getPageInfo(pageNum) {
    return this.pageCanvases.get(pageNum);
  },
  /**
   * Extract text with position information for each text item
   * Useful for signature field detection and text overlay
   * @param pageNum 1-indexed page number
   * @returns Array of text items with position and style info
   */
  async extractTextWithPositions(pageNum) {
    if (!this.currentDoc) throw new Error("No document loaded");
    const page = await this.currentDoc.getPage(pageNum);
    const textContent = await page.getTextContent();
    const cached = this.pageCanvases.get(pageNum);
    const viewport = cached?.viewport;
    const styles = textContent.styles || {};
    return textContent.items.map((item, index) => {
      const pdfX = item.transform[4];
      const pdfY = item.transform[5];
      const pdfWidth = item.width || 0;
      const pdfHeight = item.height || 12;
      const fontSize = Math.abs(item.transform[3]) || item.height || 12;
      const fontStyle = item.fontName ? styles[item.fontName] : void 0;
      const fontFamily = fontStyle?.fontFamily || "sans-serif";
      const fontNameLower = (item.fontName || "").toLowerCase();
      const isItalic = fontNameLower.includes("italic") || fontNameLower.includes("oblique");
      const isBold = fontNameLower.includes("bold");
      let domBounds = null;
      let domFontSize = fontSize;
      if (viewport) {
        const [domX, domY] = viewport.convertToViewportPoint(pdfX, pdfY);
        const [domX2, domY2] = viewport.convertToViewportPoint(pdfX + pdfWidth, pdfY + pdfHeight);
        domBounds = {
          x: Math.min(domX, domX2),
          y: Math.min(domY, domY2),
          width: Math.abs(domX2 - domX) || pdfWidth * viewport.scale,
          height: Math.abs(domY2 - domY) || pdfHeight * viewport.scale
        };
        domFontSize = fontSize * viewport.scale;
      }
      return {
        index,
        str: item.str,
        pdfX,
        pdfY,
        pdfWidth,
        pdfHeight,
        fontSize,
        // PDF font size in points
        domFontSize,
        // Font size scaled to viewport (pixels)
        fontName: item.fontName,
        fontFamily,
        // "serif", "sans-serif", or "monospace"
        isItalic,
        // true if font name contains "italic" or "oblique"
        isBold,
        // true if font name contains "bold"
        domBounds
      };
    });
  },
  /**
   * Cleanup resources - call when done with the document
   */
  cleanup() {
    if (this.currentDoc) {
      this.currentDoc.destroy();
      this.currentDoc = null;
    }
    this.pageCanvases.clear();
  }
};
var previewBridge = PdfPreviewBridge;
window.PdfPreviewBridge = PdfPreviewBridge;

// src/ts/coord-utils.ts
function domRectToPdf(viewport, domX, domY, domWidth, domHeight) {
  const [pdfX1, pdfY1] = viewport.convertToPdfPoint(domX, domY);
  const [pdfX2, pdfY2] = viewport.convertToPdfPoint(domX + domWidth, domY + domHeight);
  return {
    x: Math.min(pdfX1, pdfX2),
    y: Math.min(pdfY1, pdfY2),
    width: Math.abs(pdfX2 - pdfX1),
    height: Math.abs(pdfY2 - pdfY1)
  };
}
function domPointToPdf(viewport, domX, domY) {
  return viewport.convertToPdfPoint(domX, domY);
}
function pdfRectToDom(viewport, pdfX, pdfY, pdfWidth, pdfHeight) {
  const pdfRect = [
    pdfX,
    pdfY,
    pdfX + pdfWidth,
    pdfY + pdfHeight
  ];
  const [domX1, domY1, domX2, domY2] = viewport.convertToViewportRectangle(pdfRect);
  return {
    x: Math.min(domX1, domX2),
    y: Math.min(domY1, domY2),
    width: Math.abs(domX2 - domX1),
    height: Math.abs(domY2 - domY1)
  };
}
function pdfPointToDom(viewport, pdfX, pdfY) {
  return viewport.convertToViewportPoint(pdfX, pdfY);
}
function getPageRenderInfo(pageInfo, pageDiv) {
  if (!pageInfo) return null;
  const canvas = pageDiv?.querySelector("canvas");
  if (!canvas) return null;
  return {
    canvas,
    canvasRect: canvas.getBoundingClientRect(),
    viewport: pageInfo.viewport
  };
}

// src/ts/main.ts
function init() {
  console.log("DocSign TypeScript initialized");
  console.log("PDF Preview Bridge available:", typeof PdfPreviewBridge !== "undefined");
}
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
export {
  PdfPreviewBridge,
  domPointToPdf,
  domRectToPdf,
  ensurePdfJsLoaded,
  getPageRenderInfo,
  isPdfJsLoaded,
  pdfPointToDom,
  pdfRectToDom,
  previewBridge
};
//# sourceMappingURL=bundle.js.map
