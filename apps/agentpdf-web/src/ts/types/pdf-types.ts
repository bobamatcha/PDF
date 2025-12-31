// PDF.js and PdfBridge type definitions
// Copied from pdfjoin-web for Template Completion Engine

/**
 * PDF.js text item from getTextContent()
 */
export interface PdfJsTextItem {
  str: string;
  transform: [number, number, number, number, number, number]; // [scaleX, skewX, skewY, scaleY, x, y]
  width?: number;
  height?: number;
  fontName?: string;
}

/**
 * PDF.js text content styles
 */
export interface PdfJsTextStyles {
  [fontName: string]: {
    fontFamily?: string;
    ascent?: number;
    descent?: number;
    vertical?: boolean;
  };
}

/**
 * PDF.js text content result
 */
export interface PdfJsTextContent {
  items: PdfJsTextItem[];
  styles?: PdfJsTextStyles;
}

/**
 * DOM bounds for text positioning
 */
export interface DomBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Enhanced text item with position and style info
 */
export interface TextItem {
  index: number;
  str: string;
  pdfX: number;
  pdfY: number;
  pdfWidth: number;
  pdfHeight: number;
  fontSize: number;
  domFontSize: number;
  fontName?: string;
  fontFamily: string;
  isItalic: boolean;
  isBold: boolean;
  domBounds: DomBounds | null;
}

/**
 * PDF.js viewport (minimal types)
 */
export interface PDFJSViewport {
  width: number;
  height: number;
  scale: number;
  /** Convert PDF coordinates to viewport (DOM) coordinates */
  convertToViewportPoint(x: number, y: number): [number, number];
  /** Convert viewport (DOM) coordinates to PDF coordinates */
  convertToPdfPoint(x: number, y: number): [number, number];
  /** Convert PDF rectangle [x1, y1, x2, y2] to viewport rectangle */
  convertToViewportRectangle(rect: [number, number, number, number]): [number, number, number, number];
}

/**
 * PDF.js render task
 */
export interface PDFJSRenderTask {
  promise: Promise<void>;
}

/**
 * PDF.js page (minimal types)
 */
export interface PDFJSPage {
  view: [number, number, number, number]; // [x1, y1, x2, y2]
  getViewport(options: { scale: number }): PDFJSViewport;
  render(options: { canvasContext: CanvasRenderingContext2D; viewport: PDFJSViewport }): PDFJSRenderTask;
  getTextContent(): Promise<PdfJsTextContent>;
}

/**
 * PDF.js document loading task
 */
export interface PDFJSDocumentLoadingTask {
  promise: Promise<PDFJSDocument>;
}

/**
 * PDF.js document (minimal types)
 */
export interface PDFJSDocument {
  numPages: number;
  getPage(pageNum: number): Promise<PDFJSPage>;
  destroy(): void;
}

/**
 * PDF.js library (minimal types)
 */
export interface PDFJSLib {
  GlobalWorkerOptions: { workerSrc: string };
  getDocument(data: Uint8Array): PDFJSDocumentLoadingTask;
}

/**
 * Page render info cached by PdfBridge
 */
export interface CachedPageInfo {
  canvas: HTMLCanvasElement;
  viewport: PDFJSViewport;
  page: PDFJSPage;
}

/**
 * Page dimensions returned from renderPage
 */
export interface PageDimensions {
  width: number;
  height: number;
  originalWidth: number;
  originalHeight: number;
  pdfWidth: number;
  pdfHeight: number;
}

/**
 * PdfBridge interface
 */
export interface IPdfBridge {
  currentDoc: PDFJSDocument | null;
  pageCanvases: Map<number, CachedPageInfo>;

  loadDocument(data: Uint8Array | ArrayBuffer): Promise<number>;
  renderPage(pageNum: number, canvas: HTMLCanvasElement, scale?: number): Promise<PageDimensions>;
  getPageDimensions(pageNum: number): { width: number; height: number } | null;
  getPageInfo(pageNum: number): CachedPageInfo | undefined;
  extractText(pageNum: number): Promise<string>;
  extractTextWithPositions(pageNum: number): Promise<TextItem[]>;
  extractAllText(): Promise<string[]>;
  cleanup(): void;
}
