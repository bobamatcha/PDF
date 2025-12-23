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

// Whiteout drawing state
let isDrawing = false;
let drawStartX = 0;
let drawStartY = 0;
let drawOverlay = null;
let drawPageNum = null;
let drawPreviewEl = null;

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
            // Deselect whiteout when changing tools
            deselectWhiteout();
        });
    });

    // Click on viewer to deselect whiteout
    document.getElementById('edit-viewer')?.addEventListener('click', (e) => {
        // Only deselect if not clicking on a whiteout or its handles
        if (!e.target.closest('.edit-whiteout-overlay')) {
            deselectWhiteout();
        }
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

        // Set up mouse handlers for whiteout drawing on the PAGE div (not overlay)
        // This ensures events are captured even when text layer is on top
        pageDiv.addEventListener('mousedown', (e) => handleWhiteoutStart(e, i, overlay, pageDiv));
        pageDiv.addEventListener('mousemove', (e) => handleWhiteoutMove(e));
        pageDiv.addEventListener('mouseup', (e) => handleWhiteoutEnd(e, i));
        pageDiv.addEventListener('mouseleave', (e) => {
            if (isDrawing) handleWhiteoutCancel();
        });
    }
}

function handleOverlayClick(e, pageNum) {
    if (currentTool === 'select') return;

    // Check if clicking on or inside a whiteout - if so, open its editor
    // Use elementFromPoint for more accurate detection (handles synthetic events)
    const elementAtClick = document.elementFromPoint(e.clientX, e.clientY);
    const whiteout = elementAtClick?.closest('.edit-whiteout-overlay') || e.target.closest('.edit-whiteout-overlay');
    if (whiteout) {
        // Open the whiteout's text editor instead of creating new annotation
        openWhiteoutTextEditor(whiteout, pageNum);
        return;
    }

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
    // Create inline text input instead of using prompt()
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'edit-text-input';
    input.style.position = 'absolute';
    input.style.left = domX + 'px';
    input.style.top = domY + 'px';
    input.style.minWidth = '150px';
    input.style.fontSize = '12px';
    input.style.padding = '2px 4px';
    input.style.border = '1px solid #007bff';
    input.style.borderRadius = '2px';
    input.style.outline = 'none';
    input.style.zIndex = '100';
    input.placeholder = 'Type text...';

    overlay.appendChild(input);
    input.focus();

    function saveText() {
        const text = input.value.trim();
        input.remove();

        if (!text) return;

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

    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            saveText();
        } else if (e.key === 'Escape') {
            e.preventDefault();
            input.remove();
        }
    });

    input.addEventListener('blur', () => {
        // Small delay to allow click events to process
        setTimeout(() => {
            if (input.parentElement) {
                saveText();
            }
        }, 100);
    });
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

// ============ Whiteout Drawing Functions ============

let drawPageDiv = null;

function handleWhiteoutStart(e, pageNum, overlay, pageDiv) {
    if (currentTool !== 'whiteout') return;

    // Prevent text selection while drawing
    e.preventDefault();
    e.stopPropagation();

    isDrawing = true;
    drawOverlay = overlay;
    drawPageNum = pageNum;
    drawPageDiv = pageDiv;

    // Get position relative to the page div
    const rect = pageDiv.getBoundingClientRect();
    drawStartX = e.clientX - rect.left;
    drawStartY = e.clientY - rect.top;

    // Create preview rectangle
    drawPreviewEl = document.createElement('div');
    drawPreviewEl.className = 'whiteout-preview';
    drawPreviewEl.style.left = drawStartX + 'px';
    drawPreviewEl.style.top = drawStartY + 'px';
    drawPreviewEl.style.width = '0px';
    drawPreviewEl.style.height = '0px';
    pageDiv.appendChild(drawPreviewEl);
}

function handleWhiteoutMove(e) {
    if (!isDrawing || !drawPreviewEl || !drawPageDiv) return;

    const rect = drawPageDiv.getBoundingClientRect();
    const currentX = e.clientX - rect.left;
    const currentY = e.clientY - rect.top;

    // Calculate rectangle dimensions (handle negative widths/dragging in any direction)
    const x = Math.min(drawStartX, currentX);
    const y = Math.min(drawStartY, currentY);
    const width = Math.abs(currentX - drawStartX);
    const height = Math.abs(currentY - drawStartY);

    drawPreviewEl.style.left = x + 'px';
    drawPreviewEl.style.top = y + 'px';
    drawPreviewEl.style.width = width + 'px';
    drawPreviewEl.style.height = height + 'px';
}

function handleWhiteoutEnd(e, pageNum) {
    if (!isDrawing || !drawPreviewEl || !drawPageDiv) return;

    const rect = drawPageDiv.getBoundingClientRect();
    const endX = e.clientX - rect.left;
    const endY = e.clientY - rect.top;

    // Calculate rectangle dimensions in DOM coords
    const domX = Math.min(drawStartX, endX);
    const domY = Math.min(drawStartY, endY);
    const domWidth = Math.abs(endX - drawStartX);
    const domHeight = Math.abs(endY - drawStartY);

    // Remove preview
    if (drawPreviewEl) {
        drawPreviewEl.remove();
        drawPreviewEl = null;
    }

    // Only add if rectangle is big enough (at least 5x5 pixels)
    if (domWidth >= 5 && domHeight >= 5) {
        addWhiteoutAtPosition(pageNum, domX, domY, domWidth, domHeight);
    }

    isDrawing = false;
    drawOverlay = null;
    drawPageDiv = null;
    drawPageNum = null;
}

function handleWhiteoutCancel() {
    if (drawPreviewEl) {
        drawPreviewEl.remove();
        drawPreviewEl = null;
    }
    isDrawing = false;
    drawOverlay = null;
    drawPageDiv = null;
    drawPageNum = null;
}

let selectedWhiteout = null;

function addWhiteoutAtPosition(pageNum, domX, domY, domWidth, domHeight) {
    // Get page info for coordinate conversion
    const pageInfo = PdfBridge.getPageInfo(pageNum);
    if (!pageInfo) return;

    // Convert DOM coordinates to PDF coordinates
    const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
    const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;

    const pdfX = domX * scaleX;
    const pdfWidth = domWidth * scaleX;
    const pdfHeight = domHeight * scaleY;
    // PDF Y is from bottom, DOM Y is from top
    const pdfY = pageInfo.page.view[3] - ((domY + domHeight) * scaleY);

    // Add to session
    const opId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight);
    operationHistory.push(opId);

    // Add visual overlay
    const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);
    const whiteRect = document.createElement('div');
    whiteRect.className = 'edit-whiteout-overlay';
    whiteRect.style.left = domX + 'px';
    whiteRect.style.top = domY + 'px';
    whiteRect.style.width = domWidth + 'px';
    whiteRect.style.height = domHeight + 'px';
    whiteRect.dataset.opId = opId;
    whiteRect.dataset.page = pageNum;

    // Mousedown to select and start move
    whiteRect.addEventListener('mousedown', (e) => {
        // Don't interfere with resize handles
        if (e.target.classList.contains('resize-handle')) return;

        e.stopPropagation();
        e.preventDefault();
        selectWhiteout(whiteRect);
        startMove(e, whiteRect);
    });

    // Double-click to add text inside whiteout
    whiteRect.addEventListener('dblclick', (e) => {
        e.stopPropagation();
        openWhiteoutTextEditor(whiteRect, pageNum);
    });

    overlay.appendChild(whiteRect);

    // Auto-select the newly created whiteout
    selectWhiteout(whiteRect);

    updateButtons();
}

function selectWhiteout(whiteRect) {
    // Deselect previous
    if (selectedWhiteout) {
        selectedWhiteout.classList.remove('selected');
        // Remove resize handles
        selectedWhiteout.querySelectorAll('.resize-handle').forEach(h => h.remove());
    }

    selectedWhiteout = whiteRect;
    whiteRect.classList.add('selected');

    // Add resize handles
    const handles = ['nw', 'n', 'ne', 'w', 'e', 'sw', 's', 'se'];
    handles.forEach(pos => {
        const handle = document.createElement('div');
        handle.className = `resize-handle ${pos}`;
        handle.dataset.handle = pos;
        handle.addEventListener('mousedown', (e) => startResize(e, whiteRect, pos));
        whiteRect.appendChild(handle);
    });
}

function deselectWhiteout() {
    if (selectedWhiteout) {
        selectedWhiteout.classList.remove('selected');
        selectedWhiteout.querySelectorAll('.resize-handle').forEach(h => h.remove());
        selectedWhiteout = null;
    }
}

// Resize state
let resizing = false;
let resizeTarget = null;
let resizeHandle = '';
let resizeStartX = 0;
let resizeStartY = 0;
let resizeStartRect = null;

function startResize(e, whiteRect, handle) {
    e.preventDefault();
    e.stopPropagation();

    resizing = true;
    resizeTarget = whiteRect;
    resizeHandle = handle;
    resizeStartX = e.clientX;
    resizeStartY = e.clientY;
    resizeStartRect = {
        left: parseFloat(whiteRect.style.left),
        top: parseFloat(whiteRect.style.top),
        width: parseFloat(whiteRect.style.width),
        height: parseFloat(whiteRect.style.height)
    };

    document.addEventListener('mousemove', handleResize);
    document.addEventListener('mouseup', endResize);
}

function handleResize(e) {
    if (!resizing || !resizeTarget) return;

    const dx = e.clientX - resizeStartX;
    const dy = e.clientY - resizeStartY;

    let newLeft = resizeStartRect.left;
    let newTop = resizeStartRect.top;
    let newWidth = resizeStartRect.width;
    let newHeight = resizeStartRect.height;

    // Adjust based on which handle is being dragged
    if (resizeHandle.includes('w')) {
        newLeft = resizeStartRect.left + dx;
        newWidth = resizeStartRect.width - dx;
    }
    if (resizeHandle.includes('e')) {
        newWidth = resizeStartRect.width + dx;
    }
    if (resizeHandle.includes('n')) {
        newTop = resizeStartRect.top + dy;
        newHeight = resizeStartRect.height - dy;
    }
    if (resizeHandle.includes('s')) {
        newHeight = resizeStartRect.height + dy;
    }

    // Ensure minimum size
    if (newWidth < 10) {
        if (resizeHandle.includes('w')) {
            newLeft = resizeStartRect.left + resizeStartRect.width - 10;
        }
        newWidth = 10;
    }
    if (newHeight < 10) {
        if (resizeHandle.includes('n')) {
            newTop = resizeStartRect.top + resizeStartRect.height - 10;
        }
        newHeight = 10;
    }

    resizeTarget.style.left = newLeft + 'px';
    resizeTarget.style.top = newTop + 'px';
    resizeTarget.style.width = newWidth + 'px';
    resizeTarget.style.height = newHeight + 'px';
}

function endResize(e) {
    if (!resizing || !resizeTarget) return;

    // IMPORTANT: Remove event listeners FIRST to prevent stuck state
    document.removeEventListener('mousemove', handleResize);
    document.removeEventListener('mouseup', endResize);

    // Store reference before clearing state
    const target = resizeTarget;
    const pageNum = parseInt(target.dataset.page);
    const opId = parseInt(target.dataset.opId);

    // Clear state immediately
    resizing = false;
    resizeTarget = null;
    resizeHandle = '';

    // Now update the PDF operation (errors here won't leave listeners stuck)
    try {
        editSession.removeOperation(opId);

        const pageInfo = PdfBridge.getPageInfo(pageNum);
        if (pageInfo) {
            const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
            const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;

            const domX = parseFloat(target.style.left);
            const domY = parseFloat(target.style.top);
            const domWidth = parseFloat(target.style.width);
            const domHeight = parseFloat(target.style.height);

            const pdfX = domX * scaleX;
            const pdfWidth = domWidth * scaleX;
            const pdfHeight = domHeight * scaleY;
            const pdfY = pageInfo.page.view[3] - ((domY + domHeight) * scaleY);

            const newOpId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight);
            target.dataset.opId = newOpId;

            // Update operation history
            const idx = operationHistory.indexOf(opId);
            if (idx !== -1) {
                operationHistory[idx] = newOpId;
            }
        }
    } catch (err) {
        console.error('Error updating resize operation:', err);
    }
}

// ============ Move Whiteout Functions ============

let moving = false;
let moveTarget = null;
let moveStartX = 0;
let moveStartY = 0;
let moveStartLeft = 0;
let moveStartTop = 0;

function startMove(e, whiteRect) {
    // Don't start move if we're resizing
    if (resizing) return;

    e.preventDefault();
    e.stopPropagation();

    moving = true;
    moveTarget = whiteRect;
    moveStartX = e.clientX;
    moveStartY = e.clientY;
    moveStartLeft = parseFloat(whiteRect.style.left);
    moveStartTop = parseFloat(whiteRect.style.top);

    document.addEventListener('mousemove', handleMove);
    document.addEventListener('mouseup', endMove);
}

function handleMove(e) {
    if (!moving || !moveTarget) return;

    const dx = e.clientX - moveStartX;
    const dy = e.clientY - moveStartY;

    moveTarget.style.left = (moveStartLeft + dx) + 'px';
    moveTarget.style.top = (moveStartTop + dy) + 'px';
}

function endMove(e) {
    if (!moving || !moveTarget) return;

    // IMPORTANT: Remove event listeners FIRST to prevent stuck state
    document.removeEventListener('mousemove', handleMove);
    document.removeEventListener('mouseup', endMove);

    // Store reference before clearing state
    const target = moveTarget;
    const pageNum = parseInt(target.dataset.page);
    const opId = parseInt(target.dataset.opId);

    // Clear state immediately
    moving = false;
    moveTarget = null;

    // Now update the PDF operation (errors here won't leave listeners stuck)
    try {
        editSession.removeOperation(opId);

        const pageInfo = PdfBridge.getPageInfo(pageNum);
        if (pageInfo) {
            const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
            const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;

            const domX = parseFloat(target.style.left);
            const domY = parseFloat(target.style.top);
            const domWidth = parseFloat(target.style.width);
            const domHeight = parseFloat(target.style.height);

            const pdfX = domX * scaleX;
            const pdfWidth = domWidth * scaleX;
            const pdfHeight = domHeight * scaleY;
            const pdfY = pageInfo.page.view[3] - ((domY + domHeight) * scaleY);

            const newOpId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight);
            target.dataset.opId = newOpId;

            // Update operation history
            const idx = operationHistory.indexOf(opId);
            if (idx !== -1) {
                operationHistory[idx] = newOpId;
            }
        }
    } catch (err) {
        console.error('Error updating move operation:', err);
    }
}

// ============ Whiteout Text Editor ============

async function openWhiteoutTextEditor(whiteRect, pageNum) {
    // Check if already editing
    if (whiteRect.querySelector('.whiteout-text-input')) {
        return;
    }

    // Get whiteout dimensions
    const domX = parseFloat(whiteRect.style.left);
    const domY = parseFloat(whiteRect.style.top);
    const domWidth = parseFloat(whiteRect.style.width);
    const domHeight = parseFloat(whiteRect.style.height);

    // Detect covered text style
    const coveredStyle = await detectCoveredTextStyle(pageNum, domX, domY, domWidth, domHeight);

    // Create input INSIDE the whiteout
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'whiteout-text-input';
    input.placeholder = '';
    input.style.width = '100%';
    input.style.height = '100%';
    input.style.border = 'none';
    input.style.outline = 'none';
    input.style.background = 'transparent';
    input.style.padding = '2px 4px';
    input.style.boxSizing = 'border-box';
    input.style.textAlign = 'center';

    // Apply covered text style
    input.style.fontSize = coveredStyle.fontSize + 'px';
    input.style.fontFamily = coveredStyle.fontFamily;
    input.style.color = '#000000';

    // Store style info for saving
    input.dataset.fontSize = coveredStyle.fontSize;
    input.dataset.fontFamily = coveredStyle.fontFamily;

    whiteRect.appendChild(input);
    input.focus();

    // Handle Enter to save
    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            saveWhiteoutText(whiteRect, pageNum, input);
        } else if (e.key === 'Escape') {
            e.preventDefault();
            input.remove();
        }
    });

    // Handle blur to save
    input.addEventListener('blur', () => {
        // Small delay to allow click events to process
        setTimeout(() => {
            if (input.parentElement && input.value.trim()) {
                saveWhiteoutText(whiteRect, pageNum, input);
            } else if (input.parentElement) {
                input.remove();
            }
        }, 100);
    });
}

async function detectCoveredTextStyle(pageNum, domX, domY, domWidth, domHeight) {
    // Default style
    const defaultStyle = { fontSize: 12, fontFamily: 'Helvetica, Arial, sans-serif' };

    try {
        // Get text items from this page
        const textItems = await PdfBridge.extractTextWithPositions(pageNum);
        if (!textItems || textItems.length === 0) {
            return defaultStyle;
        }

        // Find text items that overlap with the whiteout area
        const overlapping = textItems.filter(item => {
            if (!item.domBounds) return false;
            const b = item.domBounds;
            // Check if text item intersects with whiteout
            return !(b.x + b.width < domX || b.x > domX + domWidth ||
                     b.y + b.height < domY || b.y > domY + domHeight);
        });

        if (overlapping.length === 0) {
            return defaultStyle;
        }

        // Use the first overlapping item's style
        const item = overlapping[0];
        // Use domFontSize (viewport-scaled) for display in the DOM
        return {
            fontSize: item.domFontSize || item.fontSize || 12,
            fontFamily: item.fontFamily || defaultStyle.fontFamily
        };
    } catch (err) {
        console.error('Error detecting covered text style:', err);
        return defaultStyle;
    }
}

function saveWhiteoutText(whiteRect, pageNum, input) {
    const text = input.value.trim();
    if (!text) {
        input.remove();
        return;
    }

    // Get position and style info
    const domX = parseFloat(whiteRect.style.left);
    const domY = parseFloat(whiteRect.style.top);
    const domWidth = parseFloat(whiteRect.style.width);
    const domHeight = parseFloat(whiteRect.style.height);
    const fontSize = parseFloat(input.dataset.fontSize) || 12;

    // Convert to PDF coordinates
    const pageInfo = PdfBridge.getPageInfo(pageNum);
    if (!pageInfo) {
        input.remove();
        return;
    }

    const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
    const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;

    const pdfX = domX * scaleX;
    const pdfWidth = domWidth * scaleX;
    const pdfHeight = domHeight * scaleY;
    const pdfY = pageInfo.page.view[3] - ((domY + domHeight) * scaleY);

    // Add text annotation at the whiteout position
    const opId = editSession.addText(
        pageNum,
        pdfX,
        pdfY,
        pdfWidth,
        pdfHeight,
        text,
        fontSize,
        '#000000'
    );
    operationHistory.push(opId);

    // Replace input with text span INSIDE the whiteout
    const textSpan = document.createElement('span');
    textSpan.className = 'whiteout-text-content';
    textSpan.textContent = text;
    textSpan.style.display = 'flex';
    textSpan.style.alignItems = 'center';
    textSpan.style.justifyContent = 'center';
    textSpan.style.width = '100%';
    textSpan.style.height = '100%';
    textSpan.style.fontSize = fontSize + 'px';
    textSpan.style.fontFamily = input.dataset.fontFamily || 'Helvetica, Arial, sans-serif';
    textSpan.style.color = '#000000';
    textSpan.style.overflow = 'hidden';
    textSpan.style.textOverflow = 'ellipsis';
    textSpan.style.whiteSpace = 'nowrap';
    textSpan.dataset.opId = opId;

    // Remove input and add text span
    input.remove();
    whiteRect.appendChild(textSpan);

    // Store text op ID on whiteRect for reference
    whiteRect.dataset.textOpId = opId;

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

    // Map CSS generic font family to web-safe fonts for preview
    const fontFamily = mapFontFamilyForPreview(textItem.fontFamily);
    // Use pdfHeight scaled by 1.5 (our render scale) for preview size
    const fontSize = (textItem.pdfHeight || 12) * 1.5;

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

    // Apply font styling to input for accurate preview
    const input = editor.querySelector('.text-editor-input');
    input.style.fontFamily = fontFamily;
    input.style.fontSize = fontSize + 'px';
    // Preserve italic/bold styles
    if (textItem.isItalic) input.style.fontStyle = 'italic';
    if (textItem.isBold) input.style.fontWeight = 'bold';

    // Position near the text item
    const bounds = textItem.domBounds;
    editor.style.left = bounds.x + 'px';
    editor.style.top = (bounds.y + bounds.height + 5) + 'px';

    const pageDiv = document.querySelector(`.edit-page[data-page="${pageNum}"]`);
    pageDiv.appendChild(editor);

    // Focus input (already queried above for styling)
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
    // Note: The Rust code adds padding to the white cover rectangle
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
        textItem.fontFamily || null,
        // Font style flags (detected from PDF.js font name)
        textItem.isItalic || false,
        textItem.isBold || false
    );

    operationHistory.push(opId);

    // Calculate DOM font size (scaled by 1.5 render scale)
    const domFontSize = (textItem.pdfHeight || 12) * 1.5;
    const fontFamily = mapFontFamilyForPreview(textItem.fontFamily);

    // Add visual indicator (replacement overlay) with matching font
    const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);
    const replaceEl = document.createElement('div');
    replaceEl.className = 'edit-replace-overlay';
    replaceEl.textContent = newText;

    // Position with generous padding to ensure full coverage of original canvas text
    const padding = 15; // Liberal padding to cover descenders, ascenders, and rendering artifacts
    replaceEl.style.left = (textItem.domBounds.x - padding) + 'px';
    replaceEl.style.top = (textItem.domBounds.y - padding) + 'px';
    replaceEl.style.minWidth = (textItem.domBounds.width + padding * 2) + 'px';
    replaceEl.style.minHeight = (textItem.domBounds.height + padding * 2) + 'px';
    replaceEl.style.padding = padding + 'px';
    replaceEl.style.boxSizing = 'border-box';

    // Apply matching font styling (family, size, italic, bold)
    replaceEl.style.fontFamily = fontFamily;
    replaceEl.style.fontSize = domFontSize + 'px';
    replaceEl.style.lineHeight = '1';
    if (textItem.isItalic) replaceEl.style.fontStyle = 'italic';
    if (textItem.isBold) replaceEl.style.fontWeight = 'bold';

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

// Map PDF.js font family to web-safe CSS font for preview
function mapFontFamilyForPreview(fontFamily) {
    if (!fontFamily) return 'sans-serif';

    const lower = fontFamily.toLowerCase();

    // CSS generic families
    if (lower === 'serif') return 'Georgia, "Times New Roman", Times, serif';
    if (lower === 'sans-serif') return 'Arial, Helvetica, sans-serif';
    if (lower === 'monospace') return '"Courier New", Courier, monospace';

    // Specific font names
    if (lower.includes('times')) return '"Times New Roman", Times, serif';
    if (lower.includes('arial') || lower.includes('helvetica')) return 'Arial, Helvetica, sans-serif';
    if (lower.includes('courier') || lower.includes('mono')) return '"Courier New", Courier, monospace';
    if (lower.includes('georgia')) return 'Georgia, serif';

    // Default to sans-serif
    return 'sans-serif';
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

    // Reset whiteout drawing and selection state
    handleWhiteoutCancel();
    deselectWhiteout();
    selectedWhiteout = null;

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
        case 'whiteout': viewer.style.cursor = 'crosshair'; break;
        default: viewer.style.cursor = 'default';
    }

    // Disable text layer pointer events when whiteout tool is active
    // This allows mouse events to reach the page div for drawing
    document.querySelectorAll('.text-layer').forEach(layer => {
        layer.style.pointerEvents = (currentTool === 'whiteout') ? 'none' : 'auto';
    });
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
