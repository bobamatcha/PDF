// PDF.js Lazy Loader
// Loads PDF.js only when needed to avoid bloating the main bundle
// PDF.js is ~1.3MB total (313KB main + 1MB worker)

let pdfJsLoaded = false;
let pdfJsLoadPromise = null;

/**
 * Ensures PDF.js is loaded before use.
 * Call this before any pdfjsLib usage.
 * Safe to call multiple times - will only load once.
 *
 * @returns {Promise<void>} Resolves when PDF.js is ready
 * @throws {Error} If PDF.js fails to load
 */
export async function ensurePdfJsLoaded() {
    if (pdfJsLoaded) {
        return;
    }

    if (pdfJsLoadPromise) {
        return pdfJsLoadPromise;
    }

    pdfJsLoadPromise = new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = './js/vendor/pdf.min.js';

        script.onload = () => {
            // Configure worker path - worker runs in separate thread for non-blocking rendering
            window.pdfjsLib.GlobalWorkerOptions.workerSrc = './js/vendor/pdf.worker.min.js';
            pdfJsLoaded = true;
            console.log('PDF.js loaded successfully (lazy)');
            resolve();
        };

        script.onerror = (e) => {
            pdfJsLoadPromise = null; // Allow retry
            reject(new Error('Failed to load PDF.js: ' + e.message));
        };

        document.head.appendChild(script);
    });

    return pdfJsLoadPromise;
}

/**
 * Check if PDF.js is currently loaded
 * @returns {boolean}
 */
export function isPdfJsLoaded() {
    return pdfJsLoaded;
}
