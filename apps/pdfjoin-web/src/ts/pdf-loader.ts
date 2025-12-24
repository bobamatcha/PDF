// PDF.js Lazy Loader
// Loads PDF.js only when needed to avoid bloating the main bundle
// PDF.js is ~1.3MB total (313KB main + 1MB worker)

import type { PDFJSLib } from './types';

let pdfJsLoaded = false;
let pdfJsLoadPromise: Promise<void> | null = null;

/**
 * Ensures PDF.js is loaded before use.
 * Call this before any pdfjsLib usage.
 * Safe to call multiple times - will only load once.
 *
 * @returns Resolves when PDF.js is ready
 * @throws If PDF.js fails to load
 */
export async function ensurePdfJsLoaded(): Promise<void> {
  if (pdfJsLoaded) {
    return;
  }

  if (pdfJsLoadPromise) {
    return pdfJsLoadPromise;
  }

  pdfJsLoadPromise = new Promise<void>((resolve, reject) => {
    const script = document.createElement('script');
    script.src = './js/vendor/pdf.min.js';

    script.onload = (): void => {
      // Configure worker path - worker runs in separate thread for non-blocking rendering
      if (window.pdfjsLib) {
        window.pdfjsLib.GlobalWorkerOptions.workerSrc = './js/vendor/pdf.worker.min.js';
        pdfJsLoaded = true;
        console.log('PDF.js loaded successfully (lazy)');
        resolve();
      } else {
        reject(new Error('PDF.js loaded but pdfjsLib not found on window'));
      }
    };

    script.onerror = (e): void => {
      pdfJsLoadPromise = null; // Allow retry
      const errorEvent = e as ErrorEvent;
      reject(new Error('Failed to load PDF.js: ' + (errorEvent.message || 'Unknown error')));
    };

    document.head.appendChild(script);
  });

  return pdfJsLoadPromise;
}

/**
 * Check if PDF.js is currently loaded
 */
export function isPdfJsLoaded(): boolean {
  return pdfJsLoaded;
}

// Expose on window for backwards compatibility
window.ensurePdfJsLoaded = ensurePdfJsLoaded;
