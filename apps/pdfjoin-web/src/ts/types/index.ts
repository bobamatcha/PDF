// Re-export all types
export * from './wasm-bindings';
export * from './pdf-types';

// Global window type augmentation
import type { WasmBindings } from './wasm-bindings';
import type { PDFJSLib, IPdfBridge } from './pdf-types';

declare global {
  interface Window {
    wasmBindings: WasmBindings;
    pdfjsLib?: PDFJSLib;
    ensurePdfJsLoaded?: () => Promise<void>;
    PdfBridge?: IPdfBridge;
  }
}
