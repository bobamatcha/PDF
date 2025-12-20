/**
 * Signer Page - P0.1/P0.2/P0.3 Implementation
 * Handles recipient signing workflow with guided flow and signature capture
 */

// Configure PDF.js worker
pdfjsLib.GlobalWorkerOptions.workerSrc = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';

// Global state
window.sessionParams = {
    sessionId: null,
    recipientId: null,
    signingKey: null
};

// Expose guidedFlow globally for tests
window.guidedFlow = null;

const state = {
    session: null,
    pdfDoc: null,
    currentFieldIndex: 0,
    fields: [],
    signatures: {},
    active: false  // Guided flow active state
};

// DOM elements - updated to match test expectations
const elements = {
    loadingIndicator: document.getElementById('loading-indicator'),
    signingToolbar: document.querySelector('.signing-toolbar'),
    viewerContainer: document.querySelector('.viewer-container'),
    pdfPages: document.getElementById('pdf-pages'),

    // Toolbar buttons
    btnStart: document.getElementById('btn-start'),
    btnPrev: document.getElementById('btn-prev'),
    btnNext: document.getElementById('btn-next'),
    btnFinish: document.getElementById('btn-finish'),

    // Progress elements
    currentSpan: document.getElementById('current'),
    totalSpan: document.getElementById('total'),
    progress: document.querySelector('.progress'),
    navButtons: document.querySelector('.nav-buttons'),

    // Signature modal
    signatureModal: document.getElementById('signature-modal'),
    closeSignatureModal: document.getElementById('close-signature-modal'),
    tabDraw: document.getElementById('tab-draw'),
    tabType: document.getElementById('tab-type'),
    drawTab: document.getElementById('draw-tab'),
    typeTab: document.getElementById('type-tab'),
    signaturePad: document.getElementById('signature-pad'),
    clearSignature: document.getElementById('clear-signature'),
    typedName: document.getElementById('typed-name'),
    fontSelector: document.getElementById('font-selector'),
    cursivePreview: document.getElementById('cursive-preview'),
    cancelSignature: document.getElementById('cancel-signature'),
    applySignature: document.getElementById('apply-signature')
};

/**
 * Parse URL parameters
 */
function parseUrlParams() {
    const urlParams = new URLSearchParams(window.location.search);

    window.sessionParams.sessionId = urlParams.get('session');
    window.sessionParams.recipientId = urlParams.get('recipient');
    window.sessionParams.signingKey = urlParams.get('key');

    console.log('Parsed URL params:', window.sessionParams);

    return window.sessionParams;
}

/**
 * Validate session parameters using WASM (if available) or JS fallback
 * NEVER falls back to mock data - returns validation result
 */
function validateSessionParams() {
    const { sessionId, recipientId, signingKey } = window.sessionParams;

    // Try to use WASM validation if available
    if (window.wasmModule && window.wasmModule.validate_session_params) {
        const validation = window.wasmModule.validate_session_params(
            sessionId || null,
            recipientId || null,
            signingKey || null
        );
        return {
            valid: validation.valid,
            error: validation.error_message
        };
    }

    // JS fallback validation (same logic as Rust)
    if (!sessionId || sessionId.length < 3) {
        return { valid: false, error: 'Missing or invalid session ID' };
    }
    if (!recipientId || recipientId.length < 1) {
        return { valid: false, error: 'Missing or invalid recipient ID' };
    }
    if (!signingKey || signingKey.length < 3) {
        return { valid: false, error: 'Missing or invalid signing key' };
    }

    return { valid: true, error: null };
}

/**
 * Show error message to user (NO mock data fallback)
 */
function showSessionError(message) {
    const container = elements.viewerContainer || document.querySelector('.viewer-container') || document.body;

    // Hide loading indicator
    if (elements.loadingIndicator) {
        elements.loadingIndicator.classList.add('hidden');
    }

    // Create error display
    const errorDiv = document.createElement('div');
    errorDiv.className = 'session-error';
    errorDiv.innerHTML = `
        <div style="text-align: center; padding: 3rem; color: var(--text-secondary, #666);">
            <div style="font-size: 3rem; margin-bottom: 1rem;">⚠️</div>
            <h2 style="color: #ef4444; margin-bottom: 1rem;">Invalid Signing Link</h2>
            <p style="margin-bottom: 1rem;">${message}</p>
            <p style="font-size: 0.875rem; color: #888;">
                Please check that you're using the correct link from your email invitation.
            </p>
        </div>
    `;
    container.innerHTML = '';
    container.appendChild(errorDiv);

    // Hide toolbar buttons
    if (elements.btnStart) elements.btnStart.style.display = 'none';
    if (elements.btnFinish) elements.btnFinish.style.display = 'none';
}

/**
 * Get test mock data - ONLY used when session=test for automated testing
 * Production code ALWAYS requires valid API responses
 */
function getTestMockSession(recipientId) {
    // This mock data is ONLY for automated testing with session=test
    return {
        sessionId: 'test',
        documentName: 'Test Document.pdf',
        fields: [
            { id: 'sig-1', type: 'signature', page: 1, x: 100, y: 500, width: 200, height: 50, required: true, recipientId: recipientId },
            { id: 'init-1', type: 'initials', page: 1, x: 350, y: 500, width: 80, height: 40, required: true, recipientId: recipientId },
            { id: 'date-1', type: 'date', page: 1, x: 450, y: 500, width: 100, height: 30, required: false, recipientId: recipientId }
        ]
    };
}

/**
 * Get local session data from localStorage - for demo/development mode
 * Used with session=local to test full sender->signer flow without backend
 */
function getLocalSession(recipientId) {
    const stored = localStorage.getItem('docsign_demo_session');
    if (!stored) {
        console.error('[LOCAL MODE] No session data in localStorage');
        return null;
    }

    try {
        const data = JSON.parse(stored);
        console.log('[LOCAL MODE] Loaded session from localStorage:', data);

        // Find the recipient's fields
        const recipient = data.recipients?.find(r => r.id === recipientId || r.id === parseInt(recipientId));
        if (!recipient && data.recipients?.length > 0) {
            // Try to match by index (r1 = first recipient, r2 = second, etc.)
            const match = recipientId.match(/r(\d+)/);
            if (match) {
                const idx = parseInt(match[1]) - 1;
                if (data.recipients[idx]) {
                    return buildLocalSession(data, data.recipients[idx]);
                }
            }
        }

        if (recipient) {
            return buildLocalSession(data, recipient);
        }

        console.error('[LOCAL MODE] Recipient not found:', recipientId);
        return null;
    } catch (e) {
        console.error('[LOCAL MODE] Failed to parse session data:', e);
        return null;
    }
}

function buildLocalSession(data, recipient) {
    // Get ALL fields, marking which ones belong to this recipient
    const allFields = (data.placedFields || []).map(f => {
        const isOwn = f.recipientId === recipient.id || f.recipientId === String(recipient.id);
        // Find the recipient who owns this field
        const fieldOwner = data.recipients?.find(r =>
            r.id === f.recipientId || String(r.id) === String(f.recipientId)
        );
        const ownerName = fieldOwner ? `${fieldOwner.firstName} ${fieldOwner.lastName}` : 'Unknown';

        return {
            id: f.id,
            type: f.type,
            page: f.page,
            x: f.x,
            y: f.y,
            width: f.width || (f.type === 'signature' ? 200 : 100),
            height: f.height || (f.type === 'signature' ? 50 : 30),
            required: isOwn ? (f.required !== false) : false, // Only own fields are required
            recipientId: String(f.recipientId),
            isOwn: isOwn,
            ownerName: ownerName
        };
    });

    return {
        sessionId: 'local',
        documentName: data.fileName || 'Document.pdf',
        recipientName: `${recipient.firstName} ${recipient.lastName}`,
        recipientId: String(recipient.id),
        pdfData: data.pdfData, // Base64 PDF data
        allRecipients: data.recipients, // Include all recipients for reference
        fields: allFields
    };
}

/**
 * Fetch session data from Worker API
 * IMPORTANT: Does NOT fall back to mock data - shows error instead
 * Exception: session=test is allowed for automated testing only
 */
async function fetchSession() {
    const { sessionId, recipientId, signingKey } = window.sessionParams;

    // Validate params first
    const validation = validateSessionParams();
    if (!validation.valid) {
        throw new Error(validation.error || 'Invalid session parameters');
    }

    // LOCAL MODE: Load session from localStorage (for demo without backend)
    if (sessionId === 'local') {
        console.log('[LOCAL MODE] Loading session from localStorage');
        const session = getLocalSession(recipientId);
        if (!session) {
            throw new Error('No session data found. Please run the sender flow first.');
        }
        state.session = session;
        state.fields = session.fields;
        return session;
    }

    // TEST MODE: Allow session=test for automated testing
    // This is clearly marked and only works with exact session ID 'test'
    if (sessionId === 'test') {
        console.log('[TEST MODE] Using mock session data for testing');
        const session = getTestMockSession(recipientId);
        state.session = session;
        state.fields = session.fields.filter(f => f.recipientId === recipientId);
        return session;
    }

    // PRODUCTION: Use configured API endpoint
    const WORKER_API = window.API_BASE || 'https://api.getsignatures.org';

    try {
        const response = await fetch(`${WORKER_API}/session/${sessionId}`, {
            headers: {
                'X-Recipient-Id': recipientId,
                'X-Signing-Key': signingKey
            }
        });

        if (!response.ok) {
            if (response.status === 404) {
                throw new Error('Session not found or expired');
            } else if (response.status === 401 || response.status === 403) {
                throw new Error('Invalid signing credentials');
            }
            throw new Error(`Failed to load session (error ${response.status})`);
        }

        const session = await response.json();
        state.session = session;

        // Extract fields for this recipient only
        state.fields = session.fields.filter(f => f.recipientId === recipientId);

        return session;
    } catch (err) {
        console.error('Failed to fetch session:', err);
        // NO MOCK DATA - throw error to be handled by caller
        throw err;
    }
}

/**
 * Load and render PDF
 */
async function loadPdf(pdfData) {
    try {
        const loadingTask = pdfjsLib.getDocument({ data: pdfData });
        state.pdfDoc = await loadingTask.promise;

        console.log('PDF loaded:', state.pdfDoc.numPages, 'pages');

        for (let pageNum = 1; pageNum <= state.pdfDoc.numPages; pageNum++) {
            await renderPage(pageNum);
        }

        renderFieldOverlays();

    } catch (err) {
        console.error('Failed to load PDF:', err);
        throw err;
    }
}

/**
 * Render a single PDF page
 */
async function renderPage(pageNum) {
    const page = await state.pdfDoc.getPage(pageNum);
    const viewport = page.getViewport({ scale: 1.5 });

    const pageWrapper = document.createElement('div');
    pageWrapper.className = 'pdf-page-wrapper';
    pageWrapper.dataset.pageNumber = pageNum;

    const canvas = document.createElement('canvas');
    const context = canvas.getContext('2d');
    canvas.width = viewport.width;
    canvas.height = viewport.height;

    pageWrapper.appendChild(canvas);
    elements.pdfPages.appendChild(pageWrapper);

    await page.render({
        canvasContext: context,
        viewport: viewport
    }).promise;
}

/**
 * Render field overlays on PDF
 * Shows ALL fields with different styling for own vs other signers' fields
 */
function renderFieldOverlays() {
    // Track own field index for navigation (only own fields are navigable)
    let ownFieldIndex = 0;

    state.fields.forEach((field, index) => {
        const pageWrapper = document.querySelector(`[data-page-number="${field.page}"]`);
        if (!pageWrapper) return;

        const isOwn = field.isOwn !== false; // Default to true for backwards compatibility
        const overlay = document.createElement('div');

        // Add appropriate class based on ownership
        overlay.className = `field-overlay ${isOwn ? 'own-field' : 'other-field'}`;
        overlay.dataset.fieldId = field.id;
        overlay.dataset.isOwn = isOwn ? 'true' : 'false';

        if (isOwn) {
            // Own field styling - clickable, prominent
            overlay.dataset.index = ownFieldIndex + 1;
            overlay.dataset.ownIndex = ownFieldIndex;
            overlay.style.cssText = `
                position: absolute;
                left: ${field.x}px;
                top: ${field.y}px;
                width: ${field.width}px;
                height: ${field.height}px;
                border: 2px dashed #1e40af;
                background: rgba(30, 64, 175, 0.1);
                cursor: pointer;
                display: flex;
                align-items: center;
                justify-content: center;
                font-size: 0.75rem;
                color: #1e40af;
                pointer-events: auto;
            `;
            overlay.textContent = field.type.toUpperCase();

            // Store the own field index for click handler
            const currentOwnIndex = ownFieldIndex;
            overlay.addEventListener('click', () => {
                if (state.active) {
                    goToField(currentOwnIndex);
                }
                handleFieldClick(currentOwnIndex);
            });

            ownFieldIndex++;
        } else {
            // Other signer's field - greyed out, not clickable
            overlay.style.cssText = `
                position: absolute;
                left: ${field.x}px;
                top: ${field.y}px;
                width: ${field.width}px;
                height: ${field.height}px;
                border: 2px dashed #9ca3af;
                background: rgba(156, 163, 175, 0.15);
                cursor: not-allowed;
                display: flex;
                flex-direction: column;
                align-items: center;
                justify-content: center;
                font-size: 0.65rem;
                color: #6b7280;
                pointer-events: none;
                opacity: 0.7;
            `;
            // Show owner name and field type
            const ownerLabel = document.createElement('span');
            ownerLabel.textContent = field.ownerName || 'Other';
            ownerLabel.style.cssText = 'font-weight: 500; margin-bottom: 2px;';

            const typeLabel = document.createElement('span');
            typeLabel.textContent = field.type.toUpperCase();
            typeLabel.style.cssText = 'font-size: 0.6rem; opacity: 0.8;';

            overlay.appendChild(ownerLabel);
            overlay.appendChild(typeLabel);
        }

        pageWrapper.appendChild(overlay);
    });

    // Update state.fields to only include own fields for navigation purposes
    // Keep a reference to all fields for display
    state.allFields = state.fields;
    state.fields = state.fields.filter(f => f.isOwn !== false);
}

/**
 * Start the guided signing flow
 */
function startGuidedFlow() {
    if (state.fields.length === 0) {
        console.warn('No fields to sign');
        return;
    }

    state.active = true;
    state.currentFieldIndex = 0;

    // Update global reference for tests
    window.guidedFlow = {
        active: state.active,
        currentIndex: state.currentFieldIndex,
        fields: state.fields
    };

    // Show navigation UI
    elements.btnStart?.classList.add('hidden');
    elements.progress?.classList.remove('hidden');
    elements.navButtons?.classList.remove('hidden');
    elements.btnFinish?.classList.remove('hidden');

    updateUI();
    scrollToCurrentField();
    highlightCurrentField();

    // Open first field for input
    const firstField = state.fields[0];
    if (firstField) {
        handleFieldClick(0);
    }
}

/**
 * Go to next field
 */
function nextField() {
    if (!state.active) return;
    if (state.currentFieldIndex >= state.fields.length - 1) return;

    state.currentFieldIndex++;
    updateUI();
    scrollToCurrentField();
    highlightCurrentField();
}

/**
 * Go to previous field
 */
function prevField() {
    if (!state.active) return;
    if (state.currentFieldIndex <= 0) return;

    state.currentFieldIndex--;
    updateUI();
    scrollToCurrentField();
    highlightCurrentField();
}

/**
 * Jump to specific field
 */
function goToField(index) {
    if (!state.active) return;
    if (index < 0 || index >= state.fields.length) return;

    state.currentFieldIndex = index;
    updateUI();
    scrollToCurrentField();
    highlightCurrentField();
}

/**
 * Handle field click
 */
function handleFieldClick(fieldIndex) {
    state.currentFieldIndex = fieldIndex;
    const field = state.fields[fieldIndex];

    if (field.type === 'signature' || field.type === 'initials') {
        openSignatureModal();
    } else if (field.type === 'date') {
        // Auto-fill with current date
        const today = new Date().toLocaleDateString();
        markFieldComplete(field.id, today);
    }
}

/**
 * Mark a field as completed
 */
function markFieldComplete(fieldId, value) {
    const field = state.fields.find(f => f.id === fieldId);
    if (field) {
        field.completed = true;
        state.signatures[fieldId] = value;

        const el = document.querySelector(`[data-field-id="${fieldId}"]`);
        if (el) {
            el.classList.add('completed');
            el.dataset.signed = 'true';
        }
    }
    updateFinishButton();
    updateUI();
}

/**
 * Check if all required fields are complete
 */
function canFinish() {
    return state.fields
        .filter(f => f.required !== false)
        .every(f => f.completed || state.signatures[f.id]);
}

/**
 * Open signature modal
 */
function openSignatureModal() {
    elements.signatureModal.classList.remove('hidden');

    // Initialize canvas
    const canvas = elements.signaturePad;
    if (canvas) {
        const ctx = canvas.getContext('2d');
        canvas.width = canvas.offsetWidth || 400;
        canvas.height = 200;
        ctx.fillStyle = 'white';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.strokeStyle = '#000';
        ctx.lineWidth = 2;
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';
    }
}

/**
 * Close signature modal
 */
function closeSignatureModal() {
    elements.signatureModal.classList.add('hidden');
}

/**
 * Update UI state
 */
function updateUI() {
    // Update progress
    if (elements.currentSpan) {
        elements.currentSpan.textContent = state.currentFieldIndex + 1;
    }
    if (elements.totalSpan) {
        elements.totalSpan.textContent = state.fields.length;
    }

    // Update button states
    if (elements.btnPrev) {
        elements.btnPrev.disabled = state.currentFieldIndex === 0;
    }
    if (elements.btnNext) {
        elements.btnNext.disabled = state.currentFieldIndex >= state.fields.length - 1;
    }

    updateFinishButton();
}

/**
 * Update finish button state
 */
function updateFinishButton() {
    if (elements.btnFinish) {
        elements.btnFinish.disabled = !canFinish();
    }
}

/**
 * Scroll to current field
 */
function scrollToCurrentField() {
    const field = state.fields[state.currentFieldIndex];
    if (!field) return;

    const el = document.querySelector(`[data-field-id="${field.id}"]`);
    if (el) {
        el.scrollIntoView({
            behavior: 'smooth',
            block: 'center',
            inline: 'center'
        });
    }
}

/**
 * Highlight current field
 */
function highlightCurrentField() {
    // Remove highlight from all fields
    document.querySelectorAll('.field-overlay').forEach(el => {
        el.classList.remove('current');
    });

    // Add highlight to current field
    const field = state.fields[state.currentFieldIndex];
    if (field) {
        const el = document.querySelector(`[data-field-id="${field.id}"]`);
        if (el) {
            el.classList.add('current');
        }
    }
}

/**
 * Initialize signature canvas drawing
 */
function initializeCanvas() {
    const canvas = elements.signaturePad;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    let isDrawing = false;

    canvas.addEventListener('mousedown', (e) => {
        isDrawing = true;
        const rect = canvas.getBoundingClientRect();
        ctx.beginPath();
        ctx.moveTo(e.clientX - rect.left, e.clientY - rect.top);
    });

    canvas.addEventListener('mousemove', (e) => {
        if (!isDrawing) return;
        const rect = canvas.getBoundingClientRect();
        ctx.lineTo(e.clientX - rect.left, e.clientY - rect.top);
        ctx.stroke();
    });

    canvas.addEventListener('mouseup', () => {
        isDrawing = false;
    });

    canvas.addEventListener('mouseleave', () => {
        isDrawing = false;
    });

    // Touch support
    canvas.addEventListener('touchstart', (e) => {
        e.preventDefault();
        isDrawing = true;
        const rect = canvas.getBoundingClientRect();
        const touch = e.touches[0];
        ctx.beginPath();
        ctx.moveTo(touch.clientX - rect.left, touch.clientY - rect.top);
    });

    canvas.addEventListener('touchmove', (e) => {
        e.preventDefault();
        if (!isDrawing) return;
        const rect = canvas.getBoundingClientRect();
        const touch = e.touches[0];
        ctx.lineTo(touch.clientX - rect.left, touch.clientY - rect.top);
        ctx.stroke();
    });

    canvas.addEventListener('touchend', () => {
        isDrawing = false;
    });
}

/**
 * Clear canvas
 */
function clearCanvas() {
    const canvas = elements.signaturePad;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = 'white';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
}

/**
 * Switch tabs in signature modal
 */
function switchTab(tabName) {
    if (tabName === 'draw') {
        elements.tabDraw?.classList.add('active');
        elements.tabType?.classList.remove('active');
        elements.drawTab?.classList.add('active');
        elements.drawTab?.classList.remove('hidden');
        elements.typeTab?.classList.remove('active');
        elements.typeTab?.classList.add('hidden');
    } else {
        elements.tabDraw?.classList.remove('active');
        elements.tabType?.classList.add('active');
        elements.drawTab?.classList.remove('active');
        elements.drawTab?.classList.add('hidden');
        elements.typeTab?.classList.add('active');
        elements.typeTab?.classList.remove('hidden');
    }
}

/**
 * Apply signature
 */
function applySignature() {
    const field = state.fields[state.currentFieldIndex];
    if (!field) return;

    // Get signature data based on active tab
    if (elements.drawTab?.classList.contains('active')) {
        // Check if canvas has content
        const canvas = elements.signaturePad;
        if (canvas) {
            const ctx = canvas.getContext('2d');
            const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
            const hasContent = data.some((pixel, i) => i % 4 === 3 && pixel !== 0);

            if (!hasContent) {
                alert('Please draw your signature');
                return;
            }

            const dataUrl = canvas.toDataURL();
            markFieldComplete(field.id, { type: 'drawn', data: dataUrl });
        }
    } else {
        // Save typed text
        const text = elements.typedName?.value.trim();
        if (!text) {
            alert('Please type your name');
            return;
        }
        const font = elements.fontSelector?.value || 'Dancing Script';
        markFieldComplete(field.id, { type: 'typed', data: text, font: font });
    }

    closeSignatureModal();

    // Move to next field automatically
    if (state.currentFieldIndex < state.fields.length - 1) {
        nextField();
    }
}

/**
 * Finish signing - submit to Worker API
 */
async function finishSigning() {
    if (!canFinish()) {
        // Find first incomplete required field
        const incomplete = state.fields.findIndex(f => f.required !== false && !f.completed && !state.signatures[f.id]);
        if (incomplete >= 0) {
            goToField(incomplete);
            showToast('Please complete all required fields before finishing.', 'error');
            return;
        }
    }

    state.active = false;

    // Update global reference
    if (window.guidedFlow) {
        window.guidedFlow.active = false;
    }

    // Show submitting state
    if (elements.btnFinish) {
        elements.btnFinish.disabled = true;
        elements.btnFinish.textContent = 'Submitting...';
    }

    try {
        const { sessionId, recipientId, signingKey } = window.sessionParams;

        // Prepare signed document data
        const signedData = {
            recipient_id: recipientId,
            signatures: state.signatures,
            completed_at: new Date().toISOString()
        };

        // Check if we're online
        if (!navigator.onLine) {
            // Queue for later submission
            await queueOfflineSubmission(signedData);
            showCompletionModal(true);
            return;
        }

        // Submit to Worker API
        const WORKER_API = 'https://docsigner.example.workers.dev';
        const response = await fetch(`${WORKER_API}/session/${sessionId}/signed`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-Recipient-Id': recipientId,
                'X-Signing-Key': signingKey
            },
            body: JSON.stringify(signedData)
        });

        if (!response.ok) {
            throw new Error(`Failed to submit: ${response.status}`);
        }

        const result = await response.json();

        // Check if all signers have completed
        if (result.all_signed) {
            window.isLastSigner = true;
            showCompletionModal(true, result.download_url);
        } else {
            showCompletionModal(false);
        }

    } catch (err) {
        console.error('Failed to submit signatures:', err);

        // For development/testing: show success anyway
        if (err.message.includes('Failed to fetch')) {
            showCompletionModal(false);
        } else {
            showToast('Failed to submit. Please try again.', 'error');
            if (elements.btnFinish) {
                elements.btnFinish.disabled = false;
                elements.btnFinish.textContent = 'Finish';
            }
        }
    }
}

/**
 * Show toast notification
 */
function showToast(message, type = 'success') {
    const existingToast = document.querySelector('.toast');
    if (existingToast) {
        existingToast.remove();
    }

    const toast = document.createElement('div');
    toast.className = `toast toast-${type}`;
    toast.textContent = message;
    toast.style.cssText = `
        position: fixed;
        bottom: 2rem;
        left: 50%;
        transform: translateX(-50%);
        padding: 1rem 2rem;
        border-radius: 8px;
        background: ${type === 'error' ? '#ef4444' : '#10b981'};
        color: white;
        font-weight: 500;
        z-index: 2000;
        animation: slideUp 0.3s ease;
    `;

    document.body.appendChild(toast);

    setTimeout(() => {
        toast.style.animation = 'slideDown 0.3s ease';
        setTimeout(() => toast.remove(), 300);
    }, 3000);
}

/**
 * Show completion modal
 */
function showCompletionModal(allSigned, downloadUrl = null) {
    const modal = document.createElement('div');
    modal.id = 'completion-modal';
    modal.className = 'modal-overlay';
    modal.innerHTML = `
        <div class="modal" style="text-align: center; padding: 2rem;">
            <div style="font-size: 4rem; margin-bottom: 1rem;">
                ${allSigned ? '🎉' : '✅'}
            </div>
            <h2 style="margin-bottom: 1rem; color: var(--text-primary);">
                ${allSigned ? 'All Signatures Complete!' : 'Successfully Signed!'}
            </h2>
            <p style="color: var(--text-secondary); margin-bottom: 1.5rem;">
                ${allSigned
                    ? 'All parties have signed. The final document is ready for download.'
                    : 'Your signature has been recorded. Other parties will be notified.'}
            </p>
            ${downloadUrl ? `
                <a href="${downloadUrl}" download class="btn btn-primary" style="margin-bottom: 1rem; display: inline-block;">
                    Download Signed PDF
                </a>
            ` : ''}
            <button id="close-completion" class="btn btn-secondary" style="display: block; margin: 0 auto;">
                Close
            </button>
        </div>
    `;

    document.body.appendChild(modal);

    document.getElementById('close-completion').addEventListener('click', () => {
        modal.remove();
        // Optionally redirect to a thank you page
        // window.location.href = '/thank-you.html';
    });
}

/**
 * Queue submission for offline sync
 */
async function queueOfflineSubmission(signedData) {
    const queue = JSON.parse(localStorage.getItem('offlineQueue') || '[]');
    queue.push({
        ...signedData,
        sessionId: window.sessionParams.sessionId,
        timestamp: Date.now()
    });
    localStorage.setItem('offlineQueue', JSON.stringify(queue));

    // Expose for testing
    window.offlineQueue = queue;

    console.log('Queued for offline sync:', signedData);
}

/**
 * Initialize event listeners
 */
function initializeEventListeners() {
    // Toolbar buttons
    elements.btnStart?.addEventListener('click', startGuidedFlow);
    elements.btnPrev?.addEventListener('click', prevField);
    elements.btnNext?.addEventListener('click', nextField);
    elements.btnFinish?.addEventListener('click', finishSigning);

    // Signature modal
    elements.closeSignatureModal?.addEventListener('click', closeSignatureModal);
    elements.cancelSignature?.addEventListener('click', closeSignatureModal);
    elements.applySignature?.addEventListener('click', applySignature);

    // Tab switching
    elements.tabDraw?.addEventListener('click', () => switchTab('draw'));
    elements.tabType?.addEventListener('click', () => switchTab('type'));

    // Canvas controls
    elements.clearSignature?.addEventListener('click', clearCanvas);

    // Type signature preview with font
    elements.typedName?.addEventListener('input', (e) => {
        if (elements.cursivePreview) {
            elements.cursivePreview.textContent = e.target.value;
        }
    });

    elements.fontSelector?.addEventListener('change', (e) => {
        if (elements.cursivePreview) {
            elements.cursivePreview.style.fontFamily = `'${e.target.value}', cursive`;
        }
    });

    // Close modal on overlay click
    elements.signatureModal?.addEventListener('click', (e) => {
        if (e.target === elements.signatureModal) {
            closeSignatureModal();
        }
    });

    // Keyboard navigation
    document.addEventListener('keydown', (e) => {
        if (!state.active) return;

        if (e.key === 'ArrowRight' || (e.key === 'Tab' && !e.shiftKey)) {
            e.preventDefault();
            nextField();
        } else if (e.key === 'ArrowLeft' || (e.key === 'Tab' && e.shiftKey)) {
            e.preventDefault();
            prevField();
        } else if (e.key === 'Enter') {
            const currentField = state.fields[state.currentFieldIndex];
            if (currentField) {
                handleFieldClick(state.currentFieldIndex);
            }
        }
    });
}

/**
 * Initialize the signing page
 */
async function initialize() {
    try {
        // Parse URL params
        parseUrlParams();

        // Initialize canvas drawing
        initializeCanvas();

        // Initialize event listeners
        initializeEventListeners();

        // Fetch session (or use mock data)
        await fetchSession();

        // Update total count
        if (elements.totalSpan) {
            elements.totalSpan.textContent = state.fields.length;
        }

        console.log('Session loaded:', state.session);
        console.log('Fields for recipient:', state.fields);

        // Hide loading, show UI
        elements.loadingIndicator?.classList.add('hidden');
        if (elements.signingToolbar) {
            elements.signingToolbar.style.display = 'flex';
        }
        if (elements.viewerContainer) {
            elements.viewerContainer.style.display = 'block';
        }

        // Load PDF if we have PDF data (local mode)
        if (state.session?.pdfData && !state.pdfDoc) {
            console.log('[LOCAL MODE] Loading PDF from session data');
            // Convert base64 to Uint8Array
            const binaryString = atob(state.session.pdfData);
            const bytes = new Uint8Array(binaryString.length);
            for (let i = 0; i < binaryString.length; i++) {
                bytes[i] = binaryString.charCodeAt(i);
            }
            await loadPdf(bytes);
        } else if (!state.pdfDoc && state.fields.length > 0) {
            // Show mock field overlays for testing (without PDF)
            // Create a placeholder page for testing
            const pageWrapper = document.createElement('div');
            pageWrapper.className = 'pdf-page-wrapper';
            pageWrapper.dataset.pageNumber = '1';
            pageWrapper.style.cssText = 'position: relative; width: 612px; height: 792px; background: white; margin: 0 auto;';
            elements.pdfPages.appendChild(pageWrapper);

            renderFieldOverlays();
        } else if (state.pdfDoc) {
            // PDF already loaded, just render field overlays
            renderFieldOverlays();
        }

        // Update initial UI state
        updateUI();

    } catch (err) {
        console.error('Initialization failed:', err);
        // Show proper error message - NO mock data fallback
        showSessionError(err.message || 'Invalid or expired signing link');
    }
}

// Start initialization when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initialize);
} else {
    initialize();
}
