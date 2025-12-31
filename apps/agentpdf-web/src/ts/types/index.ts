// Re-export all types
export * from './pdf-types';

// Global window type augmentation
import type { PDFJSLib, IPdfBridge } from './pdf-types';

declare global {
  interface Window {
    wasmBindings?: {
      // agentpdf WASM bindings (from agentpdf-wasm)
      render_template?: (templateName: string, inputs: string) => Promise<Uint8Array>;
      list_templates?: () => string[];
      // pdfjoin WASM bindings (for split/merge)
      PdfJoinSession?: new (mode: number) => PdfJoinSessionInstance;
    };
    pdfjsLib?: PDFJSLib;
    ensurePdfJsLoaded?: () => Promise<void>;
    PdfBridge?: IPdfBridge;
  }
}

/**
 * PdfJoinSession instance for split/merge operations
 */
export interface PdfJoinSessionInstance {
  addDocument(name: string, data: Uint8Array): void;
  setPageSelection(ranges: string): void;
  reorderDocuments(order: number[]): void;
  execute(): Uint8Array;
  free(): void;
}

/**
 * Session modes for pdfjoin operations
 */
export enum SessionMode {
  Split = 0,
  Merge = 1,
}
