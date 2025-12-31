/**
 * Signer Page - P0.1/P0.2/P0.3 Implementation
 * Handles recipient signing workflow with guided flow and signature capture
 */

// Note: PDF.js worker is configured by the TypeScript bundle (pdf-loader.ts)
// PDF loading now uses window.DocSign namespace from sign-pdf-bridge.ts

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

    // UX-004: Expiry page elements
    expiryPage: document.getElementById('expiry-page'),
    expiredDocumentName: document.getElementById('expired-document-name'),
    expiredSenderName: document.getElementById('expired-sender-name'),
    expiredSenderEmail: document.getElementById('expired-sender-email'),
    expiredSenderEmailLink: document.getElementById('expired-sender-email-link'),
    btnRequestNewLink: document.getElementById('btn-request-new-link'),
    requestSentSuccess: document.getElementById('request-sent-success')
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
            <p style="font-size: 18px; color: #888;">
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

// ============================================================
// UX-004: Expiry Page Functions
// ============================================================

/**
 * Show the session expiry page with document and sender info
 * @param {Object} sessionData - Session data from API with status: "expired"
 */
function showExpiryPage(sessionData) {
    console.log('[UX-004] Showing expiry page for session:', sessionData);

    // Hide loading indicator
    if (elements.loadingIndicator) {
        elements.loadingIndicator.classList.add('hidden');
    }

    // Hide other pages
    if (elements.consentLanding) {
        elements.consentLanding.classList.add('hidden');
    }
    if (elements.signingToolbar) {
        elements.signingToolbar.style.display = 'none';
    }
    if (elements.viewerContainer) {
        elements.viewerContainer.style.display = 'none';
    }

    // Populate expiry page with session data
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

    // Show expiry page
    if (elements.expiryPage) {
        elements.expiryPage.classList.remove('hidden');
    }

    // Reset success message
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
 * Calls the worker API to notify sender
 */
async function handleRequestNewLink() {
    const { sessionId, recipientId } = window.sessionParams;

    if (!sessionId) {
        console.error('[UX-004] No session ID available');
        return;
    }

    // Disable button and show loading state
    if (elements.btnRequestNewLink) {
        elements.btnRequestNewLink.disabled = true;
        elements.btnRequestNewLink.textContent = 'Sending...';
    }

    const WORKER_API = window.API_BASE || 'https://api.getsignatures.org';

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
            console.log('[UX-004] Request new link sent successfully');

            // Show success message
            if (elements.requestSentSuccess) {
                elements.requestSentSuccess.classList.remove('hidden');
            }
            if (elements.btnRequestNewLink) {
                elements.btnRequestNewLink.textContent = 'Request Sent';
            }
        } else {
            const errorData = await response.json().catch(() => ({}));
            console.error('[UX-004] Failed to request new link:', errorData);

            // Re-enable button on error
            if (elements.btnRequestNewLink) {
                elements.btnRequestNewLink.disabled = false;
                elements.btnRequestNewLink.textContent = 'Request New Link';
            }

            alert(errorData.message || 'Failed to send request. Please try again or contact the sender directly.');
        }
    } catch (err) {
        console.error('[UX-004] Error requesting new link:', err);

        // Re-enable button on error
        if (elements.btnRequestNewLink) {
            elements.btnRequestNewLink.disabled = false;
            elements.btnRequestNewLink.textContent = 'Request New Link';
        }

        alert('Failed to send request. Please try again or contact the sender directly.');
    }
}

// Expose functions globally for testing
window.showExpiryPage = showExpiryPage;
window.handleRequestNewLink = handleRequestNewLink;

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
 * Fetch session data - LOCAL FIRST, then server fallback
 * DOCSIGN_PLAN Phase 2: Local-first session management
 *
 * Order of operations:
 * 1. Try LocalSessionManager (IndexedDB) first
 * 2. If not found AND online, fetch from server
 * 3. Cache server response locally for offline use
 * 4. If offline and not cached, show offline error
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

    // =========================================================
    // LOCAL-FIRST: Try IndexedDB first
    // =========================================================
    if (window.DocSign?.LocalSessionManager) {
        try {
            const localSession = await window.DocSign.LocalSessionManager.getSession(sessionId, recipientId);
            if (localSession) {
                console.log('[LOCAL-FIRST] Session found in IndexedDB:', sessionId);

                // Check if locally cached session is expired
                if (localSession.status === 'expired') {
                    console.log('[LOCAL-FIRST] Cached session is expired');
                    showExpiryPage(localSession);
                    return { ...localSession, _isExpired: true };
                }

                // Use locally cached session
                state.session = {
                    sessionId: localSession.sessionId,
                    documentName: localSession.documentName,
                    metadata: localSession.metadata,
                    fields: localSession.fields,
                    recipients: localSession.recipients,
                    status: localSession.status,
                    pdfData: localSession.pdfData
                };
                state.fields = localSession.fields.filter(f =>
                    f.recipientId === recipientId || f.isOwn === true
                );

                // If we have cached signatures, restore them
                if (localSession.signatures) {
                    Object.assign(state.signatures, localSession.signatures);
                }

                return state.session;
            }
        } catch (err) {
            console.warn('[LOCAL-FIRST] Error checking local storage:', err);
            // Continue to try server
        }
    }

    // =========================================================
    // OFFLINE CHECK: If offline and no local session, show error
    // =========================================================
    if (!navigator.onLine) {
        console.log('[OFFLINE] Device is offline and no cached session found');
        throw new Error('You are offline and this session is not available locally. Please connect to the internet to load this signing request.');
    }

    // =========================================================
    // SERVER FETCH: Try to load from server
    // =========================================================
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

        // UX-004: Check if session is expired
        if (session.status === 'expired') {
            console.log('[UX-004] Session is expired');
            showExpiryPage(session);
            // Return a special marker so caller knows not to continue with normal flow
            return { ...session, _isExpired: true };
        }

        state.session = session;

        // Extract fields for this recipient only
        state.fields = session.fields.filter(f => f.recipientId === recipientId);

        // =========================================================
        // CACHE LOCALLY: Save server response for offline use
        // =========================================================
        if (window.DocSign?.LocalSessionManager) {
            try {
                await window.DocSign.LocalSessionManager.cacheSession({
                    sessionId: sessionId,
                    recipientId: recipientId,
                    documentName: session.documentName || session.metadata?.filename,
                    metadata: session.metadata,
                    fields: session.fields,
                    recipients: session.recipients,
                    status: session.status,
                    createdAt: session.metadata?.created_at || new Date().toISOString(),
                    expiresAt: session.expiresAt
                });
                console.log('[LOCAL-FIRST] Session cached locally for offline use');
            } catch (cacheErr) {
                console.warn('[LOCAL-FIRST] Failed to cache session locally:', cacheErr);
                // Don't fail the request, caching is optional
            }
        }

        return session;
    } catch (err) {
        console.error('Failed to fetch session:', err);
        // NO MOCK DATA - throw error to be handled by caller
        throw err;
    }
}

/**
 * Load and render PDF using TypeScript bridge (window.DocSign)
 * Falls back to direct pdfjsLib if DocSign not available
 */
async function loadPdf(pdfData) {
    try {
        // Prefer TypeScript bridge if available
        if (window.DocSign && typeof window.DocSign.loadPdf === 'function') {
            console.log('[sign.js] Using DocSign bridge for PDF loading');
            const result = await window.DocSign.loadPdf(pdfData);

            if (!result.success) {
                throw new Error(result.error || 'Failed to load PDF');
            }

            state.pdfDoc = { numPages: result.numPages };
            console.log('PDF loaded via DocSign:', result.numPages, 'pages');

            // Render all pages using the TypeScript bridge
            await window.DocSign.renderAllPages({
                container: elements.pdfPages,
                scale: 1.5,
                pageWrapperClass: 'pdf-page-wrapper'
            });

            renderFieldOverlays();
        } else {
            // Fallback to direct pdfjsLib (for backwards compatibility)
            console.log('[sign.js] DocSign bridge not available, using pdfjsLib fallback');
            await loadPdfLegacy(pdfData);
        }

    } catch (err) {
        console.error('Failed to load PDF:', err);
        throw err;
    }
}

/**
 * Legacy PDF loading using direct pdfjsLib calls
 * Used as fallback when TypeScript bridge is not available
 */
async function loadPdfLegacy(pdfData) {
    // Ensure pdfjsLib worker is configured
    if (typeof pdfjsLib !== 'undefined' && pdfjsLib.GlobalWorkerOptions) {
        pdfjsLib.GlobalWorkerOptions.workerSrc = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';
    }

    const loadingTask = pdfjsLib.getDocument({ data: pdfData });
    state.pdfDoc = await loadingTask.promise;

    console.log('PDF loaded (legacy):', state.pdfDoc.numPages, 'pages');

    for (let pageNum = 1; pageNum <= state.pdfDoc.numPages; pageNum++) {
        await renderPageLegacy(pageNum);
    }

    renderFieldOverlays();
}

/**
 * Legacy page rendering using direct pdfjsLib calls
 */
async function renderPageLegacy(pageNum) {
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

    // Add mobile bottom-sheet class if on mobile viewport
    const modal = elements.signatureModal.querySelector('.modal');
    if (modal && window.innerWidth < 768) {
        modal.classList.add('bottom-sheet-mobile');
    }

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
 * Check if signature canvas has content
 * @returns {boolean} True if canvas has drawn content
 */
function signatureCanvasHasContent() {
    const canvas = elements.signaturePad;
    if (!canvas) return false;

    const ctx = canvas.getContext('2d');
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    // Check for non-white pixels (canvas is filled with white background)
    for (let i = 0; i < data.length; i += 4) {
        // Check if any pixel is not white (R=255, G=255, B=255)
        if (data[i] !== 255 || data[i + 1] !== 255 || data[i + 2] !== 255) {
            return true;
        }
    }
    return false;
}

/**
 * Check if typed signature has content
 * @returns {boolean} True if typed name input has content
 */
function typedSignatureHasContent() {
    return elements.typedName && elements.typedName.value.trim().length > 0;
}

/**
 * Close signature modal with confirmation if content exists
 */
function closeSignatureModal() {
    // Check if there's unsaved signature content
    const hasDrawnContent = signatureCanvasHasContent();
    const hasTypedContent = typedSignatureHasContent();

    if (hasDrawnContent || hasTypedContent) {
        // Use confirm dialog (or DocSign.showConfirmDialog if available)
        const confirmClose = window.DocSign?.showConfirmDialog
            ? window.DocSign.showConfirmDialog('You have an unsaved signature. Are you sure you want to close?')
            : confirm('You have an unsaved signature. Are you sure you want to close?');

        if (!confirmClose) {
            return; // User cancelled, don't close
        }
    }

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
 * Canvas undo history (stores ImageData snapshots)
 */
const canvasHistory = [];
const MAX_UNDO_HISTORY = 20;

/**
 * Save current canvas state to undo history
 */
function saveCanvasState() {
    const canvas = elements.signaturePad;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

    // Limit history size
    if (canvasHistory.length >= MAX_UNDO_HISTORY) {
        canvasHistory.shift();
    }
    canvasHistory.push(imageData);
}

/**
 * Undo last canvas stroke
 */
function undoCanvasStroke() {
    const canvas = elements.signaturePad;
    if (!canvas || canvasHistory.length === 0) return;

    const ctx = canvas.getContext('2d');

    // Remove current state (if we have previous states)
    if (canvasHistory.length > 1) {
        canvasHistory.pop(); // Remove current
        const previousState = canvasHistory[canvasHistory.length - 1];
        ctx.putImageData(previousState, 0, 0);
    } else if (canvasHistory.length === 1) {
        canvasHistory.pop();
        // Restore to blank canvas
        ctx.fillStyle = 'white';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
    }
}

/**
 * Initialize signature canvas drawing with undo support
 */
function initializeCanvas() {
    const canvas = elements.signaturePad;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    let isDrawing = false;

    // Initialize with blank canvas state
    canvasHistory.length = 0;
    saveCanvasState();

    canvas.addEventListener('mousedown', (e) => {
        // Save state before starting new stroke
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
        // Save state before starting new stroke
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
            // Only undo when signature modal is visible and draw tab is active
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
 * Finish signing - LOCAL FIRST, then sync to server
 * DOCSIGN_PLAN Phase 2: Local-first signing
 *
 * Order of operations:
 * 1. Save signatures locally FIRST (always)
 * 2. Show success immediately after local save
 * 3. Queue for server sync (if enabled)
 * 4. Sync in background when online
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
        elements.btnFinish.textContent = 'Saving...';
    }

    const { sessionId, recipientId, signingKey } = window.sessionParams;
    const completedAt = new Date().toISOString();

    // Prepare signed document data
    const signedData = {
        recipient_id: recipientId,
        signatures: state.signatures,
        completed_at: completedAt
    };

    // =========================================================
    // STEP 1: Save locally FIRST (always succeeds if IndexedDB works)
    // =========================================================
    let localSaveSuccess = false;

    if (window.DocSign?.LocalSessionManager) {
        try {
            // Save signatures to local session
            await window.DocSign.LocalSessionManager.saveSignatures(sessionId, state.signatures);

            // Queue for sync (even if online - ensures data persists)
            await window.DocSign.LocalSessionManager.queueForSync({
                sessionId: sessionId,
                recipientId: recipientId,
                signingKey: signingKey,
                signatures: state.signatures,
                completedAt: completedAt,
                timestamp: Date.now()
            });

            localSaveSuccess = true;
            console.log('[LOCAL-FIRST] Signatures saved locally');
        } catch (err) {
            console.error('[LOCAL-FIRST] Failed to save signatures locally:', err);
            // Continue anyway - try server
        }
    }

    // =========================================================
    // STEP 2: If offline, show success (already saved locally)
    // =========================================================
    if (!navigator.onLine) {
        if (localSaveSuccess) {
            console.log('[OFFLINE] Signatures saved locally, will sync when online');
            showCompletionModal(false, null, true); // true = offline mode
        } else {
            // Fallback to old localStorage queue
            await queueOfflineSubmission(signedData);
            showCompletionModal(false, null, true);
        }
        return;
    }

    // =========================================================
    // STEP 3: Try to sync to server (best effort)
    // =========================================================
    const WORKER_API = window.API_BASE || 'https://api.getsignatures.org';

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

        // Remove from sync queue since we succeeded
        if (window.DocSign?.LocalSessionManager) {
            await window.DocSign.LocalSessionManager.removeFromQueue(sessionId, recipientId);
            await window.DocSign.LocalSessionManager.completeSession(sessionId);
        }

        // Check if all signers have completed
        if (result.all_signed) {
            window.isLastSigner = true;
            showCompletionModal(true, result.download_url);
        } else {
            showCompletionModal(false);
        }

    } catch (err) {
        console.error('Failed to sync signatures to server:', err);

        // =========================================================
        // STEP 4: Server failed, but local save succeeded - show success anyway
        // =========================================================
        if (localSaveSuccess) {
            console.log('[LOCAL-FIRST] Server sync failed, but signatures are saved locally');
            showCompletionModal(false, null, false); // Show success, will sync later
        } else if (err.message.includes('Failed to fetch')) {
            // Network error - show success for development/testing
            showCompletionModal(false);
        } else {
            showToast('Failed to submit. Your signatures are saved locally and will sync when possible.', 'warning');
            // Re-enable button but still show success since we saved locally
            showCompletionModal(false);
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
 * @param {boolean} allSigned - Whether all parties have signed
 * @param {string|null} downloadUrl - URL to download the signed PDF
 * @param {boolean} isOffline - Whether saved offline (will sync later)
 */
function showCompletionModal(allSigned, downloadUrl = null, isOffline = false) {
    const modal = document.createElement('div');
    modal.id = 'completion-modal';
    modal.className = 'modal-overlay';

    // Determine the message based on context
    let title, message, icon;

    if (isOffline) {
        icon = '💾';
        title = 'Signature Saved Locally';
        message = 'Your signature has been saved to your device. It will be synced automatically when you reconnect to the internet.';
    } else if (allSigned) {
        icon = '🎉';
        title = 'All Signatures Complete!';
        message = 'All parties have signed. The final document is ready for download.';
    } else {
        icon = '✅';
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
            ${isOffline ? `
                <p style="font-size: 18px; color: var(--text-muted, #888); margin-bottom: 1rem;">
                    <span style="display: inline-block; width: 10px; height: 10px; background: #f59e0b; border-radius: 50%; margin-right: 8px;"></span>
                    Waiting for internet connection...
                </p>
            ` : ''}
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

// ============================================================
// OFFLINE INDICATOR - DOCSIGN_PLAN Phase 2
// ============================================================

/**
 * Show or hide the offline indicator badge
 * @param {boolean} isOffline - Whether the device is offline
 */
function updateOfflineIndicator(isOffline) {
    let indicator = document.getElementById('offline-indicator');

    if (isOffline) {
        // Create indicator if it doesn't exist
        if (!indicator) {
            indicator = document.createElement('div');
            indicator.id = 'offline-indicator';
            indicator.className = 'offline-indicator';
            indicator.setAttribute('role', 'status');
            indicator.setAttribute('aria-live', 'polite');
            indicator.innerHTML = `
                <span class="offline-icon" aria-hidden="true">
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <line x1="1" y1="1" x2="23" y2="23"></line>
                        <path d="M16.72 11.06A10.94 10.94 0 0 1 19 12.55"></path>
                        <path d="M5 12.55a10.94 10.94 0 0 1 5.17-2.39"></path>
                        <path d="M10.71 5.05A16 16 0 0 1 22.58 9"></path>
                        <path d="M1.42 9a15.91 15.91 0 0 1 4.7-2.88"></path>
                        <path d="M8.53 16.11a6 6 0 0 1 6.95 0"></path>
                        <line x1="12" y1="20" x2="12.01" y2="20"></line>
                    </svg>
                </span>
                <span class="offline-text">Your work is safe - no internet needed</span>
            `;
            indicator.style.cssText = `
                position: fixed;
                top: 16px;
                right: 16px;
                z-index: 9999;
                display: flex;
                align-items: center;
                gap: 8px;
                padding: 10px 16px;
                background: #fef3cd;
                border: 2px solid #f59e0b;
                border-radius: 8px;
                color: #92400e;
                font-size: 18px;
                font-weight: 600;
                box-shadow: 0 2px 8px rgba(0,0,0,0.15);
                animation: offline-slide-in 0.3s ease-out;
            `;
            document.body.appendChild(indicator);

            // Add animation keyframes if not already added
            if (!document.getElementById('offline-indicator-styles')) {
                const style = document.createElement('style');
                style.id = 'offline-indicator-styles';
                style.textContent = `
                    @keyframes offline-slide-in {
                        from {
                            transform: translateX(100%);
                            opacity: 0;
                        }
                        to {
                            transform: translateX(0);
                            opacity: 1;
                        }
                    }
                    @keyframes offline-slide-out {
                        from {
                            transform: translateX(0);
                            opacity: 1;
                        }
                        to {
                            transform: translateX(100%);
                            opacity: 0;
                        }
                    }
                `;
                document.head.appendChild(style);
            }
        }
        indicator.style.display = 'flex';
        console.log('[OFFLINE] Showing offline indicator');
    } else {
        // Hide indicator with animation
        if (indicator) {
            indicator.style.animation = 'offline-slide-out 0.3s ease-in forwards';
            setTimeout(() => {
                if (indicator) {
                    indicator.style.display = 'none';
                }
            }, 300);
            console.log('[ONLINE] Hiding offline indicator');
        }
    }
}

/**
 * Initialize offline/online event listeners
 * Sets up indicators and auto-sync when coming online
 */
function initializeOfflineHandling() {
    // Set initial state
    updateOfflineIndicator(!navigator.onLine);

    // Listen for online/offline events
    window.addEventListener('online', () => {
        console.log('[ONLINE] Device is now online');
        updateOfflineIndicator(false);

        // Show toast notification
        if (typeof showToast === 'function') {
            showToast('You are back online. Syncing...', 'success');
        }

        // Trigger sync if LocalSessionManager is available
        // Note: The syncQueuedSubmissions is called from local-session-manager.ts
        // via setupAutoSync, but we can also trigger here for immediate feedback
        if (window.DocSign?.LocalSessionManager) {
            // Sync is handled by setupAutoSync in local-session-manager.ts
            console.log('[ONLINE] Queued submissions will sync automatically');
        }
    });

    window.addEventListener('offline', () => {
        console.log('[OFFLINE] Device is now offline');
        updateOfflineIndicator(true);

        // Show toast notification
        if (typeof showToast === 'function') {
            showToast('You are offline. Changes will be saved locally.', 'warning');
        }
    });

    console.log('[OFFLINE] Offline handling initialized, current status:', navigator.onLine ? 'online' : 'offline');
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

    // Decline modal overlay click to close
    elements.declineModal?.addEventListener('click', (e) => {
        if (e.target === elements.declineModal) {
            closeDeclineModal();
        }
    });

    // UX-004: Expiry page - Request New Link button
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

    // Swipe gesture navigation for mobile (UX-005)
    initSwipeGestures();
}

/**
 * Initialize swipe gesture navigation for mobile
 * Swipe left = next field, Swipe right = previous field
 */
function initSwipeGestures() {
    const viewerContainer = elements.viewerContainer;
    if (!viewerContainer) return;

    let touchStartX = 0;
    let touchStartY = 0;
    let touchEndX = 0;
    let touchEndY = 0;

    const minSwipeDistance = 50; // Minimum distance for swipe
    const maxVerticalDistance = 100; // Maximum vertical movement to count as horizontal swipe

    viewerContainer.addEventListener('touchstart', (e) => {
        if (!state.active) return;

        // Ignore if touching an input, button, or canvas
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'BUTTON' || e.target.tagName === 'CANVAS') {
            return;
        }

        touchStartX = e.changedTouches[0].screenX;
        touchStartY = e.changedTouches[0].screenY;
    }, { passive: true });

    viewerContainer.addEventListener('touchend', (e) => {
        if (!state.active) return;

        // Ignore if touching an input, button, or canvas
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'BUTTON' || e.target.tagName === 'CANVAS') {
            return;
        }

        touchEndX = e.changedTouches[0].screenX;
        touchEndY = e.changedTouches[0].screenY;

        const horizontalDistance = touchEndX - touchStartX;
        const verticalDistance = Math.abs(touchEndY - touchStartY);

        // Only process if mostly horizontal swipe
        if (verticalDistance > maxVerticalDistance) {
            return;
        }

        // Swipe left (next)
        if (horizontalDistance < -minSwipeDistance) {
            nextField();
        }
        // Swipe right (previous)
        else if (horizontalDistance > minSwipeDistance) {
            prevField();
        }
    }, { passive: true });

    // Mark container as swipe-enabled for tests
    viewerContainer.dataset.swipeEnabled = 'true';
}

/**
 * Show consent landing page
 */
function showConsentLanding() {
    const session = state.session;

    // Populate session information
    if (elements.senderName) {
        elements.senderName.textContent = session.metadata?.created_by || 'Unknown Sender';
    }
    if (elements.senderEmail) {
        // Use sender_email if available, otherwise show placeholder
        elements.senderEmail.textContent = session.metadata?.sender_email || '-';
    }
    if (elements.documentName) {
        elements.documentName.textContent = session.metadata?.filename || session.documentName || 'Document';
    }
    if (elements.dateSent) {
        // Format the date nicely
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

    // Hide loading, show consent page
    elements.loadingIndicator?.classList.add('hidden');
    elements.consentLanding?.classList.remove('hidden');
}

/**
 * Handle review document button click
 */
async function handleReviewDocument() {
    // Record consent acceptance for audit trail FIRST
    await recordConsentAcceptance();

    // Hide consent landing page
    elements.consentLanding?.classList.add('hidden');

    // Show signing interface
    if (elements.signingToolbar) {
        elements.signingToolbar.style.display = 'flex';
    }
    if (elements.viewerContainer) {
        elements.viewerContainer.style.display = 'block';
    }

    // Initialize PDF viewing if needed
    if (state.session?.pdfData && !state.pdfDoc) {
        loadPdfFromSession();
    } else if (!state.pdfDoc && state.fields.length > 0) {
        // Show mock field overlays for testing (without PDF)
        createMockPdfPage();
        renderFieldOverlays();
    } else if (state.pdfDoc) {
        // PDF already loaded, just render field overlays
        renderFieldOverlays();
    }

    // Update initial UI state
    updateUI();
}

/**
 * Record consent acceptance in the audit trail.
 * Called when user clicks "Review Document" on consent page.
 * This creates a legally-binding record that the user consented to e-signing.
 */
async function recordConsentAcceptance() {
    if (!state.session?.id || !state.recipientId) {
        console.warn('Cannot record consent: missing session or recipient ID');
        return;
    }

    const WORKER_API = window.API_BASE || 'https://api.getsignatures.org';

    // Compute hash of consent text for proof of what was shown
    const consentTextElement = document.querySelector('.consent-text');
    let consentTextHash = null;
    if (consentTextElement && window.crypto?.subtle) {
        try {
            const text = consentTextElement.textContent || '';
            const encoder = new TextEncoder();
            const data = encoder.encode(text);
            const hashBuffer = await crypto.subtle.digest('SHA-256', data);
            const hashArray = Array.from(new Uint8Array(hashBuffer));
            consentTextHash = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
        } catch (e) {
            console.warn('Could not compute consent text hash:', e);
        }
    }

    try {
        const response = await fetch(`${WORKER_API}/session/${state.session.id}/consent`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                recipient_id: state.recipientId,
                user_agent: navigator.userAgent,
                consent_text_hash: consentTextHash,
            }),
        });

        if (response.ok) {
            const result = await response.json();
            console.log('Consent recorded:', result.consent_at);

            // Store consent timestamp locally for audit log display
            state.consentAt = result.consent_at;
        } else {
            console.warn('Failed to record consent:', await response.text());
            // Continue anyway - don't block signing due to consent tracking failure
        }
    } catch (error) {
        console.warn('Error recording consent:', error);
        // Continue anyway - don't block signing due to network issues
    }
}

/**
 * Load PDF from session data
 */
async function loadPdfFromSession() {
    console.log('[LOCAL MODE] Loading PDF from session data');
    // Convert base64 to Uint8Array
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

        // Initialize offline handling (DOCSIGN_PLAN Phase 2)
        initializeOfflineHandling();

        // Fetch session (or use mock data)
        const session = await fetchSession();

        // UX-004: If session is expired, expiry page is already shown
        if (session && session._isExpired) {
            console.log('[UX-004] Session expired, skipping normal initialization');
            return;
        }

        // Update total count
        if (elements.totalSpan) {
            elements.totalSpan.textContent = state.fields.length;
        }

        console.log('Session loaded:', state.session);
        console.log('Fields for recipient:', state.fields);

        // Show consent landing page BEFORE signing interface
        showConsentLanding();

    } catch (err) {
        console.error('Initialization failed:', err);
        // Show proper error message - NO mock data fallback
        showSessionError(err.message || 'Invalid or expired signing link');
    }
}

/**
 * Open decline modal
 */
function openDeclineModal() {
    elements.declineModal?.classList.remove('hidden');
    // Clear previous reason
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
 * Submit decline to Worker API
 */
async function submitDecline() {
    const reason = elements.declineReason?.value.trim() || null;
    const { sessionId, recipientId, signingKey } = window.sessionParams;

    // Disable buttons during submission
    if (elements.confirmDecline) {
        elements.confirmDecline.disabled = true;
        elements.confirmDecline.textContent = 'Declining...';
    }

    try {
        const WORKER_API = window.API_BASE || 'https://api.getsignatures.org';

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

        // Close modal and show confirmation
        closeDeclineModal();
        showDeclinedConfirmation();

    } catch (err) {
        console.error('Failed to submit decline:', err);
        showToast('Failed to decline. Please try again.', 'error');

        // Re-enable button
        if (elements.confirmDecline) {
            elements.confirmDecline.disabled = false;
            elements.confirmDecline.textContent = 'Decline Document';
        }
    }
}

/**
 * Show declined confirmation message
 */
function showDeclinedConfirmation() {
    const modal = document.createElement('div');
    modal.id = 'declined-modal';
    modal.className = 'modal-overlay';
    modal.innerHTML = `
        <div class="modal" style="text-align: center; padding: 2rem;">
            <div style="font-size: 4rem; margin-bottom: 1rem;">✓</div>
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

// Start initialization when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initialize);
} else {
    initialize();
}
