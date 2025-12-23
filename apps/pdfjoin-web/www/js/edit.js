// Edit PDF functionality
// Uses lazy-loaded PDF.js and EditSession from WASM

import { ensurePdfJsLoaded, PdfBridge } from './pdf-bridge.js';

const { EditSession, format_bytes } = window.wasmBindings;

let editSession = null;
let currentTool = 'select';
let currentPage = 1;
let operationHistory = [];  // For undo (stores operation IDs)
let textItems = new Map();  // pageNum -> array of text items with positions
let activeEditItem = null;  // Currently editing text item

export function setupEditView() {
    const dropZone = document.getElementById('edit-drop-zone');
    const fileInput = document.getElementById('edit-file-input');
    const browseBtn = document.getElementById('edit-browse-btn');
    const removeBtn = document.getElementById('edit-remove-btn');
    const downloadBtn = document.getElementById('edit-download-btn');
    const goBackBtn = document.getElementById('edit-go-back-btn');
    const useSplitBtn = document.getElementById('edit-use-split-btn');
    const undoBtn = document.getElementById('edit-undo-btn');

    // File input
    browseBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        fileInput.click();
    });
    dropZone.addEventListener('click', () => fileInput.click());

    // Drag and drop
    dropZone.addEventListener('dragover', (e) => {
        e.preventDefault();
        dropZone.classList.add('drag-over');
    });
    dropZone.addEventListener('dragleave', () => dropZone.classList.remove('drag-over'));
    dropZone.addEventListener('drop', (e) => {
        e.preventDefault();
        dropZone.classList.remove('drag-over');
        if (e.dataTransfer.files.length > 0) {
            handleEditFile(e.dataTransfer.files[0]);
        }
    });

    fileInput.addEventListener('change', (e) => {
        if (e.target.files.length > 0) {
            handleEditFile(e.target.files[0]);
        }
    });

    // Actions
    removeBtn.addEventListener('click', resetEditView);
    downloadBtn.addEventListener('click', downloadEditedPdf);
    undoBtn.addEventListener('click', undoLastOperation);

    // Signed PDF warning actions
    goBackBtn.addEventListener('click', resetEditView);
    useSplitBtn.addEventListener('click', () => {
        resetEditView();
        document.querySelector('[data-tab="split"]').click();
    });

    // Tool buttons
    document.querySelectorAll('.tool-btn[id^="tool-"]').forEach(btn => {
        btn.addEventListener('click', () => {
            currentTool = btn.id.replace('tool-', '');
            document.querySelectorAll('.tool-btn[id^="tool-"]').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            updateCursor();
        });
    });

    // Page navigation
    document.getElementById('edit-prev-page').addEventListener('click', () => navigatePage(-1));
    document.getElementById('edit-next-page').addEventListener('click', () => navigatePage(1));

    // Error dismiss
    document.querySelector('#edit-error .dismiss').addEventListener('click', () => {
        document.getElementById('edit-error').classList.add('hidden');
    });
}

async function handleEditFile(file) {
    if (file.type !== 'application/pdf') {
        showError('edit-error', 'Please select a PDF file');
        return;
    }

    try {
        const bytes = new Uint8Array(await file.arrayBuffer());
        editSession = new EditSession(file.name, bytes);

        // Check if signed
        if (editSession.isSigned) {
            document.getElementById('edit-drop-zone').classList.add('hidden');
            document.getElementById('edit-signed-warning').classList.remove('hidden');
            return;
        }

        // Show editor
        document.getElementById('edit-drop-zone').classList.add('hidden');
        document.getElementById('edit-editor').classList.remove('hidden');

        // Update file info
        document.getElementById('edit-file-name').textContent = file.name;
        document.getElementById('edit-file-details').textContent =
            `${editSession.pageCount} pages - ${format_bytes(bytes.length)}`;

        // Lazy load PDF.js and render
        await ensurePdfJsLoaded();
        await PdfBridge.loadDocument(editSession.getDocumentBytes());
        await renderAllPages();

        updatePageNavigation();
        updateButtons();

    } catch (e) {
        showError('edit-error', 'Failed to load PDF: ' + e);
        console.error(e);
    }
}

async function renderAllPages() {
    const container = document.getElementById('edit-pages');
    container.innerHTML = '';
    textItems.clear();

    for (let i = 1; i <= editSession.pageCount; i++) {
        const pageDiv = document.createElement('div');
        pageDiv.className = 'edit-page';
        pageDiv.dataset.page = i;

        const canvas = document.createElement('canvas');
        pageDiv.appendChild(canvas);

        // Overlay container for annotations
        const overlay = document.createElement('div');
        overlay.className = 'overlay-container';
        overlay.dataset.page = i;
        pageDiv.appendChild(overlay);

        // Text layer for hover/click on existing text
        const textLayer = document.createElement('div');
        textLayer.className = 'text-layer';
        textLayer.dataset.page = i;
        pageDiv.appendChild(textLayer);

        container.appendChild(pageDiv);

        // Render page
        await PdfBridge.renderPage(i, canvas, 1.5);

        // Extract text and render text layer for editing
        const items = await PdfBridge.extractTextWithPositions(i);
        textItems.set(i, items);
        renderTextLayer(textLayer, items, i);

        // Set up click handler for adding annotations
        overlay.addEventListener('click', (e) => handleOverlayClick(e, i));
    }
}

function handleOverlayClick(e, pageNum) {
    if (currentTool === 'select') return;

    const overlay = e.currentTarget;
    const rect = overlay.getBoundingClientRect();
    const domX = e.clientX - rect.left;
    const domY = e.clientY - rect.top;

    // Get page info for coordinate conversion
    const pageInfo = PdfBridge.getPageInfo(pageNum);
    if (!pageInfo) return;

    // Convert to PDF coordinates (origin at bottom-left)
    const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
    const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
    const pdfX = domX * scaleX;
    const pdfY = pageInfo.page.view[3] - (domY * scaleY);  // Flip Y

    switch (currentTool) {
        case 'text':
            addTextAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY);
            break;
        case 'checkbox':
            addCheckboxAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY);
            break;
        case 'highlight':
            addHighlightAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY);
            break;
    }
}

function addTextAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY) {
    const text = prompt('Enter text:');
    if (!text || text.trim() === '') return;

    // Add to session (PDF coordinates, height adjusted)
    const opId = editSession.addText(pageNum, pdfX, pdfY - 20, 200, 20, text, 12, '#000000');
    operationHistory.push(opId);

    // Add visual overlay
    const textEl = document.createElement('div');
    textEl.className = 'edit-text-overlay';
    textEl.textContent = text;
    textEl.style.left = domX + 'px';
    textEl.style.top = domY + 'px';
    textEl.dataset.opId = opId;

    overlay.appendChild(textEl);
    updateButtons();
}

function addCheckboxAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY) {
    const opId = editSession.addCheckbox(pageNum, pdfX - 10, pdfY - 10, 20, 20, true);
    operationHistory.push(opId);

    const checkbox = document.createElement('div');
    checkbox.className = 'edit-checkbox-overlay checked';
    checkbox.textContent = '\u2713';  // Checkmark
    checkbox.style.left = (domX - 10) + 'px';
    checkbox.style.top = (domY - 10) + 'px';
    checkbox.dataset.opId = opId;

    // Toggle on click
    checkbox.addEventListener('click', (e) => {
        e.stopPropagation();
        checkbox.classList.toggle('checked');
        checkbox.textContent = checkbox.classList.contains('checked') ? '\u2713' : '';
    });

    overlay.appendChild(checkbox);
    updateButtons();
}

function addHighlightAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY) {
    // For simplicity, create a fixed-size highlight
    const width = 150;
    const height = 20;

    const opId = editSession.addHighlight(pageNum, pdfX, pdfY - height, width, height, '#FFFF00', 0.3);
    operationHistory.push(opId);

    const highlight = document.createElement('div');
    highlight.className = 'edit-highlight-overlay';
    highlight.style.left = domX + 'px';
    highlight.style.top = domY + 'px';
    highlight.style.width = '150px';
    highlight.style.height = '20px';
    highlight.dataset.opId = opId;

    overlay.appendChild(highlight);
    updateButtons();
}

// ============ Text Editing Functions ============

function renderTextLayer(textLayer, items, pageNum) {
    textLayer.innerHTML = '';

    items.forEach((item, index) => {
        if (!item.str.trim()) return;  // Skip whitespace-only items
        if (!item.domBounds) return;   // Skip items without position

        const span = document.createElement('span');
        span.className = 'text-item';
        span.dataset.page = pageNum;
        span.dataset.index = index;
        span.textContent = item.str;

        // Position using DOM bounds
        span.style.left = item.domBounds.x + 'px';
        span.style.top = item.domBounds.y + 'px';
        span.style.width = Math.max(item.domBounds.width, 10) + 'px';
        span.style.height = Math.max(item.domBounds.height, 12) + 'px';

        // Hover highlight
        span.addEventListener('mouseenter', () => {
            if (currentTool === 'select' || currentTool === 'edit-text') {
                span.classList.add('hover');
            }
        });
        span.addEventListener('mouseleave', () => {
            span.classList.remove('hover');
        });

        // Click to edit
        span.addEventListener('click', (e) => {
            e.stopPropagation();
            if (currentTool === 'select' || currentTool === 'edit-text') {
                startTextEdit(pageNum, index, item, span);
            }
        });

        textLayer.appendChild(span);
    });
}

function startTextEdit(pageNum, index, textItem, spanElement) {
    // Close any existing editor
    closeTextEditor();

    activeEditItem = { pageNum, index, textItem, spanElement };

    // Create inline editor
    const editor = document.createElement('div');
    editor.className = 'text-editor-popup';
    editor.innerHTML = `
        <input type="text" class="text-editor-input" value="${escapeHtml(textItem.str)}" />
        <div class="text-editor-actions">
            <button class="text-editor-save">Save</button>
            <button class="text-editor-cancel">Cancel</button>
        </div>
    `;

    // Position near the text item
    const bounds = textItem.domBounds;
    editor.style.left = bounds.x + 'px';
    editor.style.top = (bounds.y + bounds.height + 5) + 'px';

    const pageDiv = document.querySelector(`.edit-page[data-page="${pageNum}"]`);
    pageDiv.appendChild(editor);

    // Focus input
    const input = editor.querySelector('.text-editor-input');
    input.focus();
    input.select();

    // Event handlers
    editor.querySelector('.text-editor-save').addEventListener('click', () => {
        const newText = input.value;
        if (newText !== textItem.str) {
            applyTextReplacement(pageNum, textItem, newText);
        }
        closeTextEditor();
    });

    editor.querySelector('.text-editor-cancel').addEventListener('click', closeTextEditor);

    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            const newText = input.value;
            if (newText !== textItem.str) {
                applyTextReplacement(pageNum, textItem, newText);
            }
            closeTextEditor();
        } else if (e.key === 'Escape') {
            closeTextEditor();
        }
    });

    // Mark span as editing
    spanElement.classList.add('editing');
}

function closeTextEditor() {
    const editor = document.querySelector('.text-editor-popup');
    if (editor) editor.remove();

    if (activeEditItem) {
        activeEditItem.spanElement.classList.remove('editing');
        activeEditItem = null;
    }
}

function applyTextReplacement(pageNum, textItem, newText) {
    // Get page info for coordinate conversion
    const pageInfo = PdfBridge.getPageInfo(pageNum);
    if (!pageInfo) return;

    // Estimate font size from text height (PDF points)
    // The pdfHeight from PDF.js is typically close to the font size
    const fontSize = textItem.pdfHeight || 12.0;

    // Use PDF coordinates from text item
    // Note: We add a small padding to the height for the white cover
    const opId = editSession.replaceText(
        pageNum,
        // Original rect (to cover)
        textItem.pdfX,
        textItem.pdfY,
        textItem.pdfWidth || 100,
        textItem.pdfHeight || 14,
        // New rect (same position)
        textItem.pdfX,
        textItem.pdfY,
        textItem.pdfWidth || 100,
        textItem.pdfHeight || 14,
        // Text
        textItem.str,
        newText,
        fontSize,
        '#000000',
        // Font family from PDF.js styles (e.g., "serif", "sans-serif", "monospace")
        textItem.fontFamily || null
    );

    operationHistory.push(opId);

    // Add visual indicator (replacement overlay)
    const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);
    const replaceEl = document.createElement('div');
    replaceEl.className = 'edit-replace-overlay';
    replaceEl.textContent = newText;
    replaceEl.style.left = textItem.domBounds.x + 'px';
    replaceEl.style.top = textItem.domBounds.y + 'px';
    replaceEl.style.minWidth = textItem.domBounds.width + 'px';
    replaceEl.style.height = textItem.domBounds.height + 'px';
    replaceEl.dataset.opId = opId;
    overlay.appendChild(replaceEl);

    // Hide original text item visually
    const span = document.querySelector(
        `.text-item[data-page="${pageNum}"][data-index="${textItem.index}"]`
    );
    if (span) span.classList.add('replaced');

    updateButtons();
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

function undoLastOperation() {
    if (operationHistory.length === 0) return;

    const opId = operationHistory.pop();
    editSession.removeOperation(opId);

    // Remove from DOM
    const el = document.querySelector(`[data-op-id="${opId}"]`);
    if (el) el.remove();

    updateButtons();
}

function updateButtons() {
    const downloadBtn = document.getElementById('edit-download-btn');
    const undoBtn = document.getElementById('edit-undo-btn');

    const hasChanges = editSession && editSession.hasChanges();
    downloadBtn.disabled = !hasChanges;
    undoBtn.disabled = operationHistory.length === 0;
}

async function downloadEditedPdf() {
    try {
        const result = editSession.export();
        const blob = new Blob([result], { type: 'application/pdf' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = editSession.documentName.replace(/\.pdf$/i, '-edited.pdf');
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

    } catch (e) {
        showError('edit-error', 'Export failed: ' + e);
    }
}

function resetEditView() {
    editSession = null;
    currentPage = 1;
    operationHistory = [];
    currentTool = 'select';
    textItems.clear();
    closeTextEditor();

    // Reset UI
    document.getElementById('edit-drop-zone').classList.remove('hidden');
    document.getElementById('edit-signed-warning').classList.add('hidden');
    document.getElementById('edit-editor').classList.add('hidden');
    document.getElementById('edit-file-input').value = '';
    document.getElementById('edit-pages').innerHTML = '';
    document.getElementById('edit-error').classList.add('hidden');

    // Reset tool buttons
    document.querySelectorAll('.tool-btn[id^="tool-"]').forEach(b => b.classList.remove('active'));
    document.getElementById('tool-select').classList.add('active');

    // Cleanup PDF.js
    PdfBridge.cleanup();
}

function navigatePage(delta) {
    if (!editSession) return;

    const newPage = currentPage + delta;
    if (newPage < 1 || newPage > editSession.pageCount) return;

    currentPage = newPage;
    updatePageNavigation();

    // Scroll to page
    const pageEl = document.querySelector(`.edit-page[data-page="${currentPage}"]`);
    if (pageEl) {
        pageEl.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
}

function updatePageNavigation() {
    if (!editSession) return;

    const indicator = document.getElementById('edit-page-indicator');
    const prevBtn = document.getElementById('edit-prev-page');
    const nextBtn = document.getElementById('edit-next-page');

    indicator.textContent = `Page ${currentPage} of ${editSession.pageCount}`;
    prevBtn.disabled = currentPage <= 1;
    nextBtn.disabled = currentPage >= editSession.pageCount;
}

function updateCursor() {
    const viewer = document.getElementById('edit-viewer');
    if (!viewer) return;

    switch (currentTool) {
        case 'select': viewer.style.cursor = 'default'; break;
        case 'edit-text': viewer.style.cursor = 'text'; break;
        case 'text': viewer.style.cursor = 'text'; break;
        case 'highlight': viewer.style.cursor = 'crosshair'; break;
        case 'checkbox': viewer.style.cursor = 'pointer'; break;
        default: viewer.style.cursor = 'default';
    }
}

function showError(containerId, message) {
    const container = document.getElementById(containerId);
    if (!container) return;

    const textEl = container.querySelector('.error-text');
    if (textEl) textEl.textContent = message;

    container.classList.remove('hidden');

    // Auto-dismiss after 8 seconds
    setTimeout(() => container.classList.add('hidden'), 8000);
}
