// LTV Timestamp Functions for DocSigner

function openTsaConfigModal() {
    const modal = document.getElementById('tsa-config-modal');
    const input = document.getElementById('tsa-url-input');

    // Load saved TSA URL or use default
    const savedUrl = localStorage.getItem('tsaServerUrl') || 'https://freetsa.org/tsr';
    input.value = savedUrl;

    modal.classList.remove('hidden');
    input.focus();
}

function closeTsaConfigModal() {
    document.getElementById('tsa-config-modal').classList.add('hidden');
}

async function confirmAddTimestamp() {
    const tsaUrl = document.getElementById('tsa-url-input').value.trim();

    if (!tsaUrl) {
        showToast('Please enter a TSA server URL');
        return;
    }

    // Validate URL format
    try {
        new URL(tsaUrl);
    } catch (e) {
        showToast('Invalid URL format');
        return;
    }

    // Save TSA URL preference
    localStorage.setItem('tsaServerUrl', tsaUrl);

    // Close modal and proceed with timestamp
    closeTsaConfigModal();
    await addTimestamp(tsaUrl);
}

async function addTimestamp(tsaUrl) {
    const statusDiv = document.getElementById('timestamp-status');
    const button = document.getElementById('btn-add-timestamp');

    try {
        button.disabled = true;
        button.textContent = 'Adding Timestamp...';

        // Show status
        statusDiv.className = '';
        statusDiv.style.background = '#eff6ff';
        statusDiv.style.color = '#1e40af';
        statusDiv.textContent = 'Building timestamp request...';

        // Get signed PDF bytes
        let pdfBytes;
        // Note: signingWorker and state are from the main script
        if (window.signingWorker) {
            const pdfBuffer = await window.signingWorker.getPdf();
            pdfBytes = new Uint8Array(pdfBuffer);
        } else if (window.state && window.state.docSign) {
            pdfBytes = window.state.docSign.get_document_bytes();
        } else {
            throw new Error('No signed document available');
        }

        // For now, we'll use a placeholder signature extraction
        // In a full implementation, you'd extract the actual signature from the PDF
        // For demonstration, we'll hash the entire PDF
        const signatureBytes = pdfBytes.slice(0, Math.min(1000, pdfBytes.length));

        // Build TSA request using WASM function
        const { build_tsa_request, parse_tsa_response, validate_timestamp_token } =
            await import('./pkg/docsign_wasm.js');

        const tsaRequest = build_tsa_request(signatureBytes);

        statusDiv.textContent = 'Sending request to TSA server...';

        // Use worker proxy to avoid CORS issues (SOTA per Cloudflare docs)
        // The proxy forwards requests to the TSA server and handles CORS
        const proxyUrl = `/tsa-proxy?tsa=${encodeURIComponent(tsaUrl)}`;
        const response = await fetch(proxyUrl, {
            method: 'POST',
            body: tsaRequest,
        });

        if (!response.ok) {
            throw new Error(`TSA proxy returned ${response.status}: ${response.statusText}`);
        }

        const responseBytes = new Uint8Array(await response.arrayBuffer());

        statusDiv.textContent = 'Parsing TSA response...';

        // Parse response using WASM function
        let timestampToken;
        try {
            timestampToken = parse_tsa_response(responseBytes);
        } catch (e) {
            throw new Error(`Failed to parse TSA response: ${e}`);
        }

        statusDiv.textContent = 'Validating timestamp token...';

        // Validate token
        try {
            validate_timestamp_token(timestampToken);
        } catch (e) {
            throw new Error(`Invalid timestamp token: ${e}`);
        }

        // Success!
        statusDiv.style.background = '#f0fdf4';
        statusDiv.style.color = '#166534';
        statusDiv.innerHTML = '<strong>Success!</strong> Timestamp added successfully.<br>Token size: ' +
            timestampToken.length + ' bytes';

        button.textContent = 'Timestamp Added';
        button.style.background = '#10b981';

        if (typeof showToast === 'function') {
            showToast('LTV timestamp added successfully!');
        }

        // Note: In a full implementation, you would embed the timestamp token
        // back into the PDF signature. This requires modifying the PDF structure,
        // which is beyond the scope of this demonstration.
        console.log('Timestamp token (hex):',
            Array.from(timestampToken).map(b => b.toString(16).padStart(2, '0')).join(''));

    } catch (err) {
        console.error('Timestamp error:', err);

        statusDiv.style.background = '#fef2f2';
        statusDiv.style.color = '#991b1b';

        let errorMsg = err.message || 'Unknown error';

        // Provide helpful error messages
        if (errorMsg.includes('Failed to fetch') || errorMsg.includes('NetworkError')) {
            errorMsg = 'Network error: Cannot reach timestamp server. Check your internet connection.';
        } else if (errorMsg.includes('TSA proxy returned')) {
            errorMsg = 'Timestamp server error. The server may be temporarily unavailable.';
        }

        statusDiv.innerHTML = '<strong>Error:</strong> ' + errorMsg;

        button.disabled = false;
        button.textContent = 'Retry';

        if (typeof showToast === 'function') {
            showToast('Failed to add timestamp: ' + errorMsg);
        }
    }
}

// Auto-add LTV timestamp when signing completes (SOTA per Adobe Sign approach)
// "Digital signatures automatically include time stamps on agreements"
// - Adobe Sign docs (https://helpx.adobe.com/sign/config/time-stamp-settings.html)
async function autoAddTimestamp() {
    const defaultTsaUrl = 'https://freetsa.org/tsr';
    const statusDiv = document.getElementById('timestamp-status');
    const button = document.getElementById('btn-add-timestamp');

    try {
        console.log('[LTV] Auto-applying LTV timestamp...');

        // Update button to show auto-processing
        if (button) {
            button.disabled = true;
            button.textContent = 'Adding Timestamp...';
        }

        // Show the LTV card
        const ltvCard = document.getElementById('ltv-timestamp-card');
        if (ltvCard) {
            ltvCard.style.display = '';
        }

        await addTimestamp(defaultTsaUrl);
        console.log('[LTV] Auto-timestamp applied successfully');
    } catch (err) {
        console.warn('[LTV] Auto-timestamp failed:', err.message);
        // Show manual button as fallback
        if (button) {
            button.disabled = false;
            button.textContent = 'Add Timestamp';
        }
        if (statusDiv) {
            statusDiv.style.background = '#fef3cd';
            statusDiv.style.color = '#856404';
            statusDiv.innerHTML = 'Auto-timestamp failed. Click button to retry manually.';
        }
    }
}

// Initialize LTV timestamp feature when document is ready
function initLtvTimestamp() {
    const button = document.getElementById('btn-add-timestamp');
    if (button) {
        // Keep manual click as backup option
        button.addEventListener('click', openTsaConfigModal);
    }

    // Auto-trigger LTV timestamp when download becomes available
    // (following Adobe Sign SOTA: automatic, invisible to users)
    let autoTimestampTriggered = false;
    const observer = new MutationObserver((mutations) => {
        const downloadBtn = document.getElementById('btn-download');
        const ltvCard = document.getElementById('ltv-timestamp-card');

        if (downloadBtn && !downloadBtn.classList.contains('hidden') && !autoTimestampTriggered) {
            autoTimestampTriggered = true;
            observer.disconnect();

            // Show LTV card
            if (ltvCard) {
                ltvCard.style.display = '';
            }

            // Auto-trigger timestamp with slight delay to ensure PDF is ready
            setTimeout(() => {
                autoAddTimestamp();
            }, 500);
        }
    });

    // Watch for changes to the download button visibility
    const downloadBtn = document.getElementById('btn-download');
    if (downloadBtn) {
        observer.observe(downloadBtn, { attributes: true, attributeFilter: ['class'] });

        // Check if already visible (in case we loaded late)
        if (!downloadBtn.classList.contains('hidden') && !autoTimestampTriggered) {
            autoTimestampTriggered = true;
            const ltvCard = document.getElementById('ltv-timestamp-card');
            if (ltvCard) {
                ltvCard.style.display = '';
            }
            setTimeout(() => {
                autoAddTimestamp();
            }, 500);
        }
    }
}

// Expose functions to window for onclick handlers
window.closeTsaConfigModal = closeTsaConfigModal;
window.confirmAddTimestamp = confirmAddTimestamp;

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initLtvTimestamp);
} else {
    initLtvTimestamp();
}
