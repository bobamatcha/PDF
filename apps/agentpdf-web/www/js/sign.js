/**
 * agentPDF Signing Page - JavaScript Controller
 * Handles recipient signing workflow with guided flow and signature capture
 *
 * Magic link parameters:
 * - session: Session ID for the signing request
 * - recipient: Recipient ID for the signer
 * - key: Signing key for authentication
 */

// Configure PDF.js worker
if (typeof pdfjsLib !== 'undefined' && pdfjsLib.GlobalWorkerOptions) {
    pdfjsLib.GlobalWorkerOptions.workerSrc = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';
}

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
    allFields: [],
    signatures: {},
    active: false  // Guided flow active state
};

// DOM elements
const elements = {
    loadingIndicator: document.getElementById('loading-indicator'),
    signingToolbar: document.querySelector('.signing-toolbar'),
    viewerContainer: document.querySelector('.viewer-container'),
    pdfPages: document.getElementById('pdf-pages'),

    // Consent landing page
    consentLanding: document.getElementById('consent-landing'),
    senderName: document.getElementById('sender-name'),
    senderEmail: document.getElementById('sender-email'),
    documentName: document.getElementById('document-name'),
    dateSent: document.getElementById('date-sent'),
    btnReviewDocument: document.getElementById('btn-review-document'),
    linkDecline: document.getElementById('link-decline'),

    // Toolbar buttons
    btnStart: document.getElementById('btn-start'),
    btnPrev: document.getElementById('btn-prev'),
    btnNext: document.getElementById('btn-next'),
    btnFinish: document.getElementById('btn-finish'),
    btnDecline: document.getElementById('btn-decline'),

    // Progress elements
    currentSpan: document.getElementById('current'),
    totalSpan: document.getElementById('total'),
    progress: document.querySelector('.progress'),
    navButtons: document.querySelector('.nav-buttons'),
    signingProgressContainer: document.getElementById('signing-progress-container'),
    signingProgressFill: document.getElementById('signing-progress-fill'),

    // Signature modal
    signatureModal: document.getElementById('signature-modal'),
    closeSignatureModal: document.getElementById('close-signature-modal'),
    tabDraw: document.getElementById('tab-draw'),
    tabType: document.getElementById('tab-type'),
    drawTab: document.getElementById('draw-tab'),
    typeTab: document.getElementById('type-tab'),
    signaturePad: document.getElementById('signature-pad'),
    undoSignature: document.getElementById('undo-signature'),
    clearSignature: document.getElementById('clear-signature'),
    typedName: document.getElementById('typed-name'),
    fontSelector: document.getElementById('font-selector'),
    cursivePreview: document.getElementById('cursive-preview'),
    cancelSignature: document.getElementById('cancel-signature'),
    applySignature: document.getElementById('apply-signature'),

    // Decline modal
    declineModal: document.getElementById('decline-modal'),
    closeDeclineModal: document.getElementById('close-decline-modal'),
    declineReason: document.getElementById('decline-reason'),
    cancelDecline: document.getElementById('cancel-decline'),
    confirmDecline: document.getElementById('confirm-decline'),

    // Expiry page elements
    expiryPage: document.getElementById('expiry-page'),
    expiredDocumentName: document.getElementById('expired-document-name'),
    expiredSenderName: document.getElementById('expired-sender-name'),
    expiredSenderEmail: document.getElementById('expired-sender-email'),
    expiredSenderEmailLink: document.getElementById('expired-sender-email-link'),
    btnRequestNewLink: document.getElementById('btn-request-new-link'),
    requestSentSuccess: document.getElementById('request-sent-success')
};

/**
 * Parse URL parameters for magic link
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
 * Validate session parameters
 */
function validateSessionParams() {
    const { sessionId, recipientId, signingKey } = window.sessionParams;

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
 * Show error message to user
 */
function showSessionError(message) {
    const container = elements.viewerContainer || document.body;

    if (elements.loadingIndicator) {
        elements.loadingIndicator.classList.add('hidden');
    }

    const errorDiv = document.createElement('div');
    errorDiv.className = 'session-error';
    errorDiv.innerHTML = `
        <div style="text-align: center; padding: 3rem; color: var(--text-secondary, #666);">
            <div style="font-size: 3rem; margin-bottom: 1rem;">!</div>
            <h2 style="color: #ef4444; margin-bottom: 1rem;">Invalid Signing Link</h2>
            <p style="margin-bottom: 1rem;">${message}</p>
            <p style="font-size: 18px; color: #888;">
                Please check that you're using the correct link from your email invitation.
            </p>
        </div>
    `;
    container.innerHTML = '';
    container.appendChild(errorDiv);
    container.style.display = 'flex';
    container.style.alignItems = 'center';
    container.style.justifyContent = 'center';

    if (elements.btnStart) elements.btnStart.style.display = 'none';
    if (elements.btnFinish) elements.btnFinish.style.display = 'none';
}

/**
 * Show the session expiry page
 */
function showExpiryPage(sessionData) {
    console.log('[EXPIRY] Showing expiry page for session:', sessionData);

    if (elements.loadingIndicator) {
        elements.loadingIndicator.classList.add('hidden');
    }

    if (elements.consentLanding) {
        elements.consentLanding.classList.add('hidden');
    }
    if (elements.signingToolbar) {
        elements.signingToolbar.style.display = 'none';
    }
    if (elements.viewerContainer) {
        elements.viewerContainer.style.display = 'none';
    }

    const metadata = sessionData.metadata || {};
    const documentName = metadata.filename || sessionData.document_name || 'Document';
    const senderName = metadata.created_by || sessionData.sender_name || 'Unknown Sender';
    const senderEmail = metadata.sender_email || sessionData.sender_email || '';

    if (elements.expiredDocumentName) {
        elements.expiredDocumentName.textContent = documentName;
    }
    if (elements.expiredSenderName) {
        elements.expiredSenderName.textContent = senderName;
    }
    if (elements.expiredSenderEmail) {
        elements.expiredSenderEmail.textContent = senderEmail || '-';
    }
    if (elements.expiredSenderEmailLink) {
        if (senderEmail) {
            elements.expiredSenderEmailLink.href = `mailto:${senderEmail}`;
            elements.expiredSenderEmailLink.textContent = senderEmail;
            elements.expiredSenderEmailLink.parentElement.style.display = 'block';
        } else {
            elements.expiredSenderEmailLink.parentElement.style.display = 'none';
        }
    }

    if (elements.expiryPage) {
        elements.expiryPage.classList.remove('hidden');
    }

    if (elements.requestSentSuccess) {
        elements.requestSentSuccess.classList.add('hidden');
    }
    if (elements.btnRequestNewLink) {
        elements.btnRequestNewLink.disabled = false;
        elements.btnRequestNewLink.textContent = 'Request New Link';
    }
}

/**
 * Handle request for new signing link
 */
async function handleRequestNewLink() {
    const { sessionId, recipientId } = window.sessionParams;

    if (!sessionId) {
        console.error('[EXPIRY] No session ID available');
        return;
    }

    if (elements.btnRequestNewLink) {
        elements.btnRequestNewLink.disabled = true;
        elements.btnRequestNewLink.textContent = 'Sending...';
    }

    const WORKER_API = window.API_BASE || 'https://api.agentpdf.org';

    try {
        const response = await fetch(`${WORKER_API}/session/${sessionId}/request-link`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                recipient_id: recipientId
            })
        });

        if (response.ok) {
            console.log('[EXPIRY] Request new link sent successfully');

            if (elements.requestSentSuccess) {
                elements.requestSentSuccess.classList.remove('hidden');
            }
            if (elements.btnRequestNewLink) {
                elements.btnRequestNewLink.textContent = 'Request Sent';
            }
        } else {
            const errorData = await response.json().catch(() => ({}));
            console.error('[EXPIRY] Failed to request new link:', errorData);

            if (elements.btnRequestNewLink) {
                elements.btnRequestNewLink.disabled = false;
                elements.btnRequestNewLink.textContent = 'Request New Link';
            }

            alert(errorData.message || 'Failed to send request. Please try again or contact the sender directly.');
        }
    } catch (err) {
        console.error('[EXPIRY] Error requesting new link:', err);

        if (elements.btnRequestNewLink) {
            elements.btnRequestNewLink.disabled = false;
            elements.btnRequestNewLink.textContent = 'Request New Link';
        }

        alert('Failed to send request. Please try again or contact the sender directly.');
    }
}

// Expose functions globally
window.showExpiryPage = showExpiryPage;
window.handleRequestNewLink = handleRequestNewLink;

/**
 * Get test mock data - ONLY for session=test
 */
function getTestMockSession(recipientId) {
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
 * Get local session data from localStorage
 */
function getLocalSession(recipientId) {
    const stored = localStorage.getItem('agentpdf_demo_session');
    if (!stored) {
        console.error('[LOCAL MODE] No session data in localStorage');
        return null;
    }

    try {
        const data = JSON.parse(stored);
        console.log('[LOCAL MODE] Loaded session from localStorage:', data);

        const recipient = data.recipients?.find(r => r.id === recipientId || r.id === parseInt(recipientId));
        if (!recipient && data.recipients?.length > 0) {
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
    const allFields = (data.placedFields || []).map(f => {
        const isOwn = f.recipientId === recipient.id || f.recipientId === String(recipient.id);
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
            required: isOwn ? (f.required !== false) : false,
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
        pdfData: data.pdfData,
        allRecipients: data.recipients,
        fields: allFields
    };
}

/**
 * Fetch session data from API or localStorage
 */
async function fetchSession() {
    const { sessionId, recipientId, signingKey } = window.sessionParams;

    const validation = validateSessionParams();
    if (!validation.valid) {
        throw new Error(validation.error || 'Invalid session parameters');
    }

    // LOCAL MODE: Load session from localStorage
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
    if (sessionId === 'test') {
        console.log('[TEST MODE] Using mock session data for testing');
        const session = getTestMockSession(recipientId);
        state.session = session;
        state.fields = session.fields.filter(f => f.recipientId === recipientId);
        return session;
    }

    // Check if offline
    if (!navigator.onLine) {
        console.log('[OFFLINE] Device is offline');
        throw new Error('You are offline. Please connect to the internet to load this signing request.');
    }

    // Fetch from server
    const WORKER_API = window.API_BASE || 'https://api.agentpdf.org';

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

        // Check if session is expired
        if (session.status === 'expired') {
            console.log('[EXPIRY] Session is expired');
            showExpiryPage(session);
            return { ...session, _isExpired: true };
        }

        state.session = session;
        state.fields = session.fields.filter(f => f.recipientId === recipientId);

        return session;
    } catch (err) {
        console.error('Failed to fetch session:', err);
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
 */
function renderFieldOverlays() {
    let ownFieldIndex = 0;

    state.fields.forEach((field, index) => {
        const pageWrapper = document.querySelector(`[data-page-number="${field.page}"]`);
        if (!pageWrapper) return;

        const isOwn = field.isOwn !== false;
        const overlay = document.createElement('div');

        overlay.className = `field-overlay ${isOwn ? 'own-field' : 'other-field'}`;
        overlay.dataset.fieldId = field.id;
        overlay.dataset.isOwn = isOwn ? 'true' : 'false';

        if (isOwn) {
            overlay.dataset.index = ownFieldIndex + 1;
            overlay.dataset.ownIndex = ownFieldIndex;
            overlay.style.cssText = `
                position: absolute;
                left: ${field.x}px;
                top: ${field.y}px;
                width: ${field.width}px;
                height: ${field.height}px;
                border: 2px dashed #005163;
                background: rgba(0, 81, 99, 0.1);
                cursor: pointer;
                display: flex;
                align-items: center;
                justify-content: center;
                font-size: 0.75rem;
                color: #005163;
                pointer-events: auto;
            `;
            overlay.textContent = field.type.toUpperCase();

            const currentOwnIndex = ownFieldIndex;
            overlay.addEventListener('click', () => {
                if (state.active) {
                    goToField(currentOwnIndex);
                }
                handleFieldClick(currentOwnIndex);
            });

            ownFieldIndex++;
        } else {
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

    window.guidedFlow = {
        active: state.active,
        currentIndex: state.currentFieldIndex,
        fields: state.fields
    };

    elements.btnStart?.classList.add('hidden');
    elements.progress?.classList.remove('hidden');
    elements.navButtons?.classList.remove('hidden');
    elements.btnFinish?.classList.remove('hidden');
    elements.signingProgressContainer?.classList.remove('hidden');

    updateUI();
    scrollToCurrentField();
    highlightCurrentField();

    const firstField = state.fields[0];
    if (firstField) {
        handleFieldClick(0);
    }
}

/**
 * Navigate to next field
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
 * Navigate to previous field
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

    const modal = elements.signatureModal.querySelector('.modal');
    if (modal && window.innerWidth < 768) {
        modal.classList.add('bottom-sheet-mobile');
    }

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
    const hasDrawnContent = signatureCanvasHasContent();
    const hasTypedContent = typedSignatureHasContent();

    if (hasDrawnContent || hasTypedContent) {
        const confirmClose = confirm('You have an unsaved signature. Are you sure you want to close?');
        if (!confirmClose) {
            return;
        }
    }

    elements.signatureModal.classList.add('hidden');
}

/**
 * Check if signature canvas has content
 */
function signatureCanvasHasContent() {
    const canvas = elements.signaturePad;
    if (!canvas) return false;

    const ctx = canvas.getContext('2d');
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    for (let i = 0; i < data.length; i += 4) {
        if (data[i] !== 255 || data[i + 1] !== 255 || data[i + 2] !== 255) {
            return true;
        }
    }
    return false;
}

/**
 * Check if typed signature has content
 */
function typedSignatureHasContent() {
    return elements.typedName && elements.typedName.value.trim().length > 0;
}

/**
 * Update UI state
 */
function updateUI() {
    if (elements.currentSpan) {
        elements.currentSpan.textContent = state.currentFieldIndex + 1;
    }
    if (elements.totalSpan) {
        elements.totalSpan.textContent = state.fields.length;
    }

    // Update progress bar
    if (elements.signingProgressFill && state.fields.length > 0) {
        const completedCount = state.fields.filter(f => f.completed || state.signatures[f.id]).length;
        const progress = (completedCount / state.fields.length) * 100;
        elements.signingProgressFill.style.width = `${progress}%`;
    }

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
    document.querySelectorAll('.field-overlay').forEach(el => {
        el.classList.remove('current');
    });

    const field = state.fields[state.currentFieldIndex];
    if (field) {
        const el = document.querySelector(`[data-field-id="${field.id}"]`);
        if (el) {
            el.classList.add('current');
        }
    }
}

// Canvas undo history
const canvasHistory = [];
const MAX_UNDO_HISTORY = 20;

function saveCanvasState() {
    const canvas = elements.signaturePad;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

    if (canvasHistory.length >= MAX_UNDO_HISTORY) {
        canvasHistory.shift();
    }
    canvasHistory.push(imageData);
}

function undoCanvasStroke() {
    const canvas = elements.signaturePad;
    if (!canvas || canvasHistory.length === 0) return;

    const ctx = canvas.getContext('2d');

    if (canvasHistory.length > 1) {
        canvasHistory.pop();
        const previousState = canvasHistory[canvasHistory.length - 1];
        ctx.putImageData(previousState, 0, 0);
    } else if (canvasHistory.length === 1) {
        canvasHistory.pop();
        ctx.fillStyle = 'white';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
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

    canvasHistory.length = 0;
    saveCanvasState();

    canvas.addEventListener('mousedown', (e) => {
        saveCanvasState();
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
        saveCanvasState();
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

    // Keyboard shortcut: Ctrl+Z or Cmd+Z to undo
    document.addEventListener('keydown', (e) => {
        if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
            if (!elements.signatureModal?.classList.contains('hidden') &&
                elements.drawTab?.classList.contains('active')) {
                e.preventDefault();
                undoCanvasStroke();
            }
        }
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
        elements.drawTab?.removeAttribute('hidden');
        elements.typeTab?.classList.remove('active');
        elements.typeTab?.classList.add('hidden');
        elements.typeTab?.setAttribute('hidden', '');
    } else {
        elements.tabDraw?.classList.remove('active');
        elements.tabType?.classList.add('active');
        elements.drawTab?.classList.remove('active');
        elements.drawTab?.classList.add('hidden');
        elements.drawTab?.setAttribute('hidden', '');
        elements.typeTab?.classList.add('active');
        elements.typeTab?.classList.remove('hidden');
        elements.typeTab?.removeAttribute('hidden');
    }
}

/**
 * Apply signature
 */
function applySignature() {
    const field = state.fields[state.currentFieldIndex];
    if (!field) return;

    if (elements.drawTab?.classList.contains('active') && !elements.drawTab?.hasAttribute('hidden')) {
        const canvas = elements.signaturePad;
        if (canvas) {
            if (!signatureCanvasHasContent()) {
                alert('Please draw your signature');
                return;
            }

            const dataUrl = canvas.toDataURL();
            markFieldComplete(field.id, { type: 'drawn', data: dataUrl });
        }
    } else {
        const text = elements.typedName?.value.trim();
        if (!text) {
            alert('Please type your name');
            return;
        }
        const font = elements.fontSelector?.value || 'Dancing Script';
        markFieldComplete(field.id, { type: 'typed', data: text, font: font });
    }

    elements.signatureModal.classList.add('hidden');

    if (state.currentFieldIndex < state.fields.length - 1) {
        nextField();
    }
}

/**
 * Finish signing
 */
async function finishSigning() {
    if (!canFinish()) {
        const incomplete = state.fields.findIndex(f => f.required !== false && !f.completed && !state.signatures[f.id]);
        if (incomplete >= 0) {
            goToField(incomplete);
            showToast('Please complete all required fields before finishing.', 'error');
            return;
        }
    }

    state.active = false;

    if (window.guidedFlow) {
        window.guidedFlow.active = false;
    }

    if (elements.btnFinish) {
        elements.btnFinish.disabled = true;
        elements.btnFinish.textContent = 'Saving...';
    }

    const { sessionId, recipientId, signingKey } = window.sessionParams;
    const completedAt = new Date().toISOString();

    const signedData = {
        recipient_id: recipientId,
        signatures: state.signatures,
        completed_at: completedAt
    };

    // For local mode, just show success
    if (sessionId === 'local' || sessionId === 'test') {
        showCompletionModal(false);
        return;
    }

    // Try to submit to server
    const WORKER_API = window.API_BASE || 'https://api.agentpdf.org';

    try {
        if (elements.btnFinish) {
            elements.btnFinish.textContent = 'Syncing...';
        }

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

        if (result.all_signed) {
            window.isLastSigner = true;
            showCompletionModal(true, result.download_url);
        } else {
            showCompletionModal(false);
        }

    } catch (err) {
        console.error('Failed to sync signatures to server:', err);
        // Show success anyway - offline-first approach
        showCompletionModal(false);
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
    `;

    document.body.appendChild(toast);

    setTimeout(() => {
        toast.remove();
    }, 3000);
}

/**
 * Show completion modal
 */
function showCompletionModal(allSigned, downloadUrl = null) {
    const modal = document.createElement('div');
    modal.id = 'completion-modal';
    modal.className = 'modal-overlay';

    let title, message, icon;

    if (allSigned) {
        icon = '';
        title = 'All Signatures Complete!';
        message = 'All parties have signed. The final document is ready for download.';
    } else {
        icon = '';
        title = 'Successfully Signed!';
        message = 'Your signature has been recorded. Other parties will be notified.';
    }

    modal.innerHTML = `
        <div class="modal" style="text-align: center; padding: 2rem;">
            <div style="font-size: 4rem; margin-bottom: 1rem;">
                ${icon}
            </div>
            <h2 style="margin-bottom: 1rem; color: var(--text-primary);">
                ${title}
            </h2>
            <p style="color: var(--text-secondary); margin-bottom: 1.5rem;">
                ${message}
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
    });
}

/**
 * Show consent landing page
 */
function showConsentLanding() {
    const session = state.session;

    if (elements.senderName) {
        elements.senderName.textContent = session.metadata?.created_by || 'Unknown Sender';
    }
    if (elements.senderEmail) {
        elements.senderEmail.textContent = session.metadata?.sender_email || '-';
    }
    if (elements.documentName) {
        elements.documentName.textContent = session.metadata?.filename || session.documentName || 'Document';
    }
    if (elements.dateSent) {
        const dateStr = session.metadata?.created_at;
        if (dateStr) {
            try {
                const date = new Date(dateStr);
                elements.dateSent.textContent = date.toLocaleDateString('en-US', {
                    year: 'numeric',
                    month: 'long',
                    day: 'numeric',
                    hour: '2-digit',
                    minute: '2-digit'
                });
            } catch {
                elements.dateSent.textContent = dateStr;
            }
        } else {
            elements.dateSent.textContent = '-';
        }
    }

    elements.loadingIndicator?.classList.add('hidden');
    elements.consentLanding?.classList.remove('hidden');
}

/**
 * Handle review document button click
 */
async function handleReviewDocument() {
    elements.consentLanding?.classList.add('hidden');

    if (elements.signingToolbar) {
        elements.signingToolbar.style.display = 'flex';
    }
    if (elements.viewerContainer) {
        elements.viewerContainer.style.display = 'block';
    }

    if (state.session?.pdfData && !state.pdfDoc) {
        await loadPdfFromSession();
    } else if (!state.pdfDoc && state.fields.length > 0) {
        createMockPdfPage();
        renderFieldOverlays();
    } else if (state.pdfDoc) {
        renderFieldOverlays();
    }

    updateUI();
}

/**
 * Load PDF from session data
 */
async function loadPdfFromSession() {
    console.log('[LOCAL MODE] Loading PDF from session data');
    const binaryString = atob(state.session.pdfData);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
    }
    await loadPdf(bytes);
}

/**
 * Create mock PDF page for testing
 */
function createMockPdfPage() {
    const pageWrapper = document.createElement('div');
    pageWrapper.className = 'pdf-page-wrapper';
    pageWrapper.dataset.pageNumber = '1';
    pageWrapper.style.cssText = 'position: relative; width: 612px; height: 792px; background: white; margin: 0 auto;';
    elements.pdfPages.appendChild(pageWrapper);
}

/**
 * Open decline modal
 */
function openDeclineModal() {
    elements.declineModal?.classList.remove('hidden');
    if (elements.declineReason) {
        elements.declineReason.value = '';
    }
}

/**
 * Close decline modal
 */
function closeDeclineModal() {
    elements.declineModal?.classList.add('hidden');
}

/**
 * Submit decline
 */
async function submitDecline() {
    const reason = elements.declineReason?.value.trim() || null;
    const { sessionId, recipientId, signingKey } = window.sessionParams;

    if (elements.confirmDecline) {
        elements.confirmDecline.disabled = true;
        elements.confirmDecline.textContent = 'Declining...';
    }

    try {
        const WORKER_API = window.API_BASE || 'https://api.agentpdf.org';

        const response = await fetch(`${WORKER_API}/session/${sessionId}/decline`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
                'X-Recipient-Id': recipientId,
                'X-Signing-Key': signingKey
            },
            body: JSON.stringify({
                recipient_id: recipientId,
                reason: reason
            })
        });

        if (!response.ok) {
            throw new Error(`Failed to decline: ${response.status}`);
        }

        closeDeclineModal();
        showDeclinedConfirmation();

    } catch (err) {
        console.error('Failed to submit decline:', err);
        showToast('Failed to decline. Please try again.', 'error');

        if (elements.confirmDecline) {
            elements.confirmDecline.disabled = false;
            elements.confirmDecline.textContent = 'Decline Document';
        }
    }
}

/**
 * Show declined confirmation
 */
function showDeclinedConfirmation() {
    const modal = document.createElement('div');
    modal.id = 'declined-modal';
    modal.className = 'modal-overlay';
    modal.innerHTML = `
        <div class="modal" style="text-align: center; padding: 2rem;">
            <div style="font-size: 4rem; margin-bottom: 1rem;"></div>
            <h2 style="margin-bottom: 1rem; color: var(--text-primary);">Document Declined</h2>
            <p style="color: var(--text-secondary); margin-bottom: 1.5rem;">
                The sender has been notified of your decision.
            </p>
            <button id="close-declined" class="btn btn-primary">
                Close
            </button>
        </div>
    `;

    document.body.appendChild(modal);

    document.getElementById('close-declined').addEventListener('click', () => {
        modal.remove();
    });
}

/**
 * Update offline indicator
 */
function updateOfflineIndicator(isOffline) {
    const indicator = document.getElementById('offline-indicator');
    if (indicator) {
        if (isOffline) {
            indicator.classList.remove('hidden');
        } else {
            indicator.classList.add('hidden');
        }
    }
}

/**
 * Initialize event listeners
 */
function initializeEventListeners() {
    // Consent landing page buttons
    elements.btnReviewDocument?.addEventListener('click', handleReviewDocument);
    elements.linkDecline?.addEventListener('click', (e) => {
        e.preventDefault();
        openDeclineModal();
    });

    // Toolbar buttons
    elements.btnStart?.addEventListener('click', startGuidedFlow);
    elements.btnPrev?.addEventListener('click', prevField);
    elements.btnNext?.addEventListener('click', nextField);
    elements.btnFinish?.addEventListener('click', finishSigning);
    elements.btnDecline?.addEventListener('click', openDeclineModal);

    // Decline modal
    elements.closeDeclineModal?.addEventListener('click', closeDeclineModal);
    elements.cancelDecline?.addEventListener('click', closeDeclineModal);
    elements.confirmDecline?.addEventListener('click', submitDecline);

    elements.declineModal?.addEventListener('click', (e) => {
        if (e.target === elements.declineModal) {
            closeDeclineModal();
        }
    });

    // Expiry page - Request New Link button
    elements.btnRequestNewLink?.addEventListener('click', handleRequestNewLink);

    // Signature modal
    elements.closeSignatureModal?.addEventListener('click', closeSignatureModal);
    elements.cancelSignature?.addEventListener('click', closeSignatureModal);
    elements.applySignature?.addEventListener('click', applySignature);

    // Tab switching
    elements.tabDraw?.addEventListener('click', () => switchTab('draw'));
    elements.tabType?.addEventListener('click', () => switchTab('type'));

    // Canvas controls
    elements.undoSignature?.addEventListener('click', undoCanvasStroke);
    elements.clearSignature?.addEventListener('click', () => {
        canvasHistory.length = 0;
        clearCanvas();
        saveCanvasState();
    });

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

    // Offline/online events
    window.addEventListener('online', () => {
        updateOfflineIndicator(false);
        showToast('You are back online.', 'success');
    });

    window.addEventListener('offline', () => {
        updateOfflineIndicator(true);
        showToast('You are offline. Changes will be saved locally.', 'warning');
    });
}

/**
 * Initialize the signing page
 */
async function initialize() {
    try {
        parseUrlParams();
        initializeCanvas();
        initializeEventListeners();

        // Set initial offline state
        updateOfflineIndicator(!navigator.onLine);

        const session = await fetchSession();

        // If session is expired, expiry page is already shown
        if (session && session._isExpired) {
            console.log('[EXPIRY] Session expired, skipping normal initialization');
            return;
        }

        if (elements.totalSpan) {
            elements.totalSpan.textContent = state.fields.length;
        }

        console.log('Session loaded:', state.session);
        console.log('Fields for recipient:', state.fields);

        showConsentLanding();

    } catch (err) {
        console.error('Initialization failed:', err);
        showSessionError(err.message || 'Invalid or expired signing link');
    }
}

// Start initialization when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initialize);
} else {
    initialize();
}
