/**
 * Web Worker for PDF signing operations (ES Module)
 * Moves heavy WASM operations off the main thread
 */

import init, { DocSign } from './pkg/docsign_wasm.js';

let wasmInitialized = false;
let docSign = null;

// Initialize WASM
async function initWasm() {
    try {
        await init();
        wasmInitialized = true;
        self.postMessage({ type: 'init', success: true });
    } catch (err) {
        self.postMessage({ type: 'init', success: false, error: err.message });
    }
}

// Load PDF into worker
function loadPdf(pdfBytes) {
    try {
        docSign = new DocSign();
        docSign.load_pdf(new Uint8Array(pdfBytes));
        self.postMessage({ type: 'loaded', success: true });
    } catch (err) {
        self.postMessage({ type: 'loaded', success: false, error: err.message });
    }
}

// Add a text field (date, text, initials)
function addTextField(page, x, y, width, height, value, fieldType) {
    try {
        docSign.add_text_field(page, x, y, width, height, value, fieldType);
        return { success: true };
    } catch (err) {
        return { success: false, error: err.message };
    }
}

// Add a checkbox
function addCheckbox(page, x, y, size, checked) {
    try {
        docSign.add_checkbox_field(page, x, y, size, checked);
        return { success: true };
    } catch (err) {
        return { success: false, error: err.message };
    }
}

// Sign a single field - this is the heavy operation
function signField(page, x, y, width, height, reason, signerEmail) {
    try {
        docSign.set_signer_email(signerEmail);

        // Use progress callback version for real-time updates
        const progressCallback = (stage, percent) => {
            self.postMessage({ type: 'progress', stage, percent });
        };

        docSign.sign_document_with_progress(page, x, y, width, height, reason, progressCallback);

        return { success: true };
    } catch (err) {
        // Fallback to non-progress version if callback not supported
        try {
            self.postMessage({ type: 'progress', stage: 'signing', percent: 30 });
            docSign.sign_document(page, x, y, width, height, reason);
            self.postMessage({ type: 'progress', stage: 'complete', percent: 100 });
            return { success: true };
        } catch (fallbackErr) {
            return { success: false, error: fallbackErr.message };
        }
    }
}

// Get the signed PDF bytes
function getSignedPdf() {
    try {
        const pdfBytes = docSign.get_document_bytes();
        return { success: true, data: pdfBytes };
    } catch (err) {
        return { success: false, error: err.message };
    }
}

// Handle messages from main thread
self.onmessage = async function(e) {
    const { id, action, data } = e.data;

    try {
        switch (action) {
            case 'init':
                await initWasm();
                break;

            case 'load':
                loadPdf(data.pdfBytes);
                break;

            case 'addTextField':
                const textResult = addTextField(
                    data.page, data.x, data.y,
                    data.width, data.height,
                    data.value, data.fieldType
                );
                self.postMessage({ id, type: 'result', ...textResult });
                break;

            case 'addCheckbox':
                const cbResult = addCheckbox(
                    data.page, data.x, data.y,
                    data.size, data.checked
                );
                self.postMessage({ id, type: 'result', ...cbResult });
                break;

            case 'sign':
                const signResult = signField(
                    data.page, data.x, data.y,
                    data.width, data.height,
                    data.reason, data.signerEmail
                );
                self.postMessage({ id, type: 'result', ...signResult });
                break;

            case 'getPdf':
                const pdfResult = getSignedPdf();
                if (pdfResult.success) {
                    self.postMessage(
                        { id, type: 'pdf', data: pdfResult.data.buffer },
                        [pdfResult.data.buffer]
                    );
                } else {
                    self.postMessage({ id, type: 'error', error: pdfResult.error });
                }
                break;

            default:
                self.postMessage({ id, type: 'error', error: `Unknown action: ${action}` });
        }
    } catch (err) {
        self.postMessage({ id, type: 'error', error: err.message });
    }
};
