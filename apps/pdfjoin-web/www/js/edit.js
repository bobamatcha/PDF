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
let activeTextInput = null; // Currently focused text input (for B/I buttons)

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
            // Toggle whiteout-tool-active class for border visibility
            const viewer = document.getElementById('edit-viewer');
            if (viewer) {
                if (currentTool === 'whiteout') {
                    viewer.classList.add('whiteout-tool-active');
                } else {
                    viewer.classList.remove('whiteout-tool-active');
                }
            }
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

    // Bold/Italic style buttons
    const boldBtn = document.getElementById('style-bold');
    const italicBtn = document.getElementById('style-italic');

    boldBtn.addEventListener('click', () => toggleBold());
    italicBtn.addEventListener('click', () => toggleItalic());

    // Global keyboard shortcuts for Cmd+B and Cmd+I
    document.addEventListener('keydown', (e) => {
        if ((e.metaKey || e.ctrlKey) && activeTextInput) {
            if (e.key === 'b' || e.key === 'B') {
                e.preventDefault();
                toggleBold();
            } else if (e.key === 'i' || e.key === 'I') {
                e.preventDefault();
                toggleItalic();
            }
        }
    });

    // Font size controls
    document.getElementById('font-size-decrease').addEventListener('click', () => decreaseFontSize());
    document.getElementById('font-size-increase').addEventListener('click', () => increaseFontSize());
    document.getElementById('font-size-value').addEventListener('change', (e) => setFontSize(e.target.value));

    // Font family control
    document.getElementById('style-font-family').addEventListener('change', (e) => setFontFamily(e.target.value));
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

    // Check if clicking on an existing text overlay - if so, edit it
    const textOverlay = elementAtClick?.closest('.edit-text-overlay') || e.target.closest('.edit-text-overlay');
    if (textOverlay && currentTool === 'text') {
        // Edit the existing text overlay instead of creating new
        editExistingTextOverlay(textOverlay, pageNum);
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
    // Create auto-expanding contentEditable span
    const input = document.createElement('span');
    input.contentEditable = true;
    input.className = 'edit-text-input';
    input.style.position = 'absolute';
    input.style.left = domX + 'px';
    input.style.top = domY + 'px';
    input.style.minWidth = '20px';
    input.style.minHeight = '1em';
    input.style.fontSize = '12px';
    input.style.fontFamily = 'sans-serif';
    input.style.padding = '2px 4px';
    input.style.border = '1px solid #007bff';
    input.style.borderRadius = '2px';
    input.style.outline = 'none';
    input.style.zIndex = '100';
    input.style.display = 'inline-block';
    input.style.whiteSpace = 'pre-wrap';
    input.style.wordBreak = 'break-word';
    input.style.background = 'white';

    // Initialize text styling state
    input.dataset.isBold = 'false';
    input.dataset.isItalic = 'false';
    input.dataset.fontSize = '12';
    input.dataset.fontFamily = 'sans-serif';

    overlay.appendChild(input);
    input.focus();
    setActiveTextInput(input);

    function saveText() {
        const text = input.textContent.trim();
        const isBold = input.dataset.isBold === 'true';
        const isItalic = input.dataset.isItalic === 'true';
        const fontSize = parseInt(input.dataset.fontSize) || 12;
        const fontFamily = input.dataset.fontFamily || 'sans-serif';
        input.remove();
        setActiveTextInput(null);

        if (!text) return;

        // Get actual dimensions of the text for PDF operation
        const textWidth = Math.max(input.offsetWidth, 50);
        const textHeight = Math.max(input.offsetHeight, 20);

        // Add to session (PDF coordinates, height adjusted)
        const opId = editSession.addText(pageNum, pdfX, pdfY - 20, textWidth, textHeight, text, fontSize, '#000000', fontFamily, isItalic, isBold);
        operationHistory.push(opId);

        // Add visual overlay
        const textEl = document.createElement('div');
        textEl.className = 'edit-text-overlay';
        textEl.textContent = text;
        textEl.style.left = domX + 'px';
        textEl.style.top = domY + 'px';
        textEl.style.fontSize = fontSize + 'px';
        textEl.style.fontFamily = fontFamily;
        if (isBold) textEl.style.fontWeight = 'bold';
        if (isItalic) textEl.style.fontStyle = 'italic';
        textEl.dataset.opId = opId;
        textEl.dataset.fontSize = fontSize;
        textEl.dataset.fontFamily = fontFamily;
        textEl.dataset.isBold = isBold ? 'true' : 'false';
        textEl.dataset.isItalic = isItalic ? 'true' : 'false';

        overlay.appendChild(textEl);

        // Make text overlay draggable with Select tool
        makeTextOverlayDraggable(textEl, pageNum);

        updateButtons();
    }

    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            saveText();
        } else if (e.key === 'Escape') {
            e.preventDefault();
            input.remove();
            setActiveTextInput(null);
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

function editExistingTextOverlay(textOverlay, pageNum) {
    // Get existing text and style
    const existingText = textOverlay.textContent;
    const existingOpId = textOverlay.dataset.opId;
    const isBold = textOverlay.style.fontWeight === 'bold' || textOverlay.style.fontWeight === '700';
    const isItalic = textOverlay.style.fontStyle === 'italic';
    const fontSize = parseInt(textOverlay.dataset.fontSize) || 12;
    const fontFamily = textOverlay.dataset.fontFamily || 'sans-serif';

    // Get position
    const domX = parseFloat(textOverlay.style.left);
    const domY = parseFloat(textOverlay.style.top);

    // Get the overlay container
    const overlay = textOverlay.parentElement;

    // Remove the old operation from session
    if (existingOpId && editSession) {
        editSession.removeOperation(BigInt(existingOpId));
        // Remove from history
        const historyIndex = operationHistory.indexOf(BigInt(existingOpId));
        if (historyIndex > -1) {
            operationHistory.splice(historyIndex, 1);
        }
    }

    // Hide the text overlay while editing (don't remove yet in case of cancel)
    textOverlay.style.display = 'none';

    // Create auto-expanding contentEditable span at the same position
    const input = document.createElement('span');
    input.contentEditable = true;
    input.className = 'edit-text-input';
    input.style.position = 'absolute';
    input.style.left = domX + 'px';
    input.style.top = domY + 'px';
    input.style.minWidth = '20px';
    input.style.minHeight = '1em';
    input.style.fontSize = fontSize + 'px';
    input.style.fontFamily = fontFamily;
    input.style.padding = '2px 4px';
    input.style.border = '1px solid #007bff';
    input.style.borderRadius = '2px';
    input.style.outline = 'none';
    input.style.zIndex = '100';
    input.style.display = 'inline-block';
    input.style.whiteSpace = 'pre-wrap';
    input.style.wordBreak = 'break-word';
    input.style.background = 'white';
    input.textContent = existingText;

    // Initialize text styling state from existing overlay
    input.dataset.isBold = isBold ? 'true' : 'false';
    input.dataset.isItalic = isItalic ? 'true' : 'false';
    input.dataset.fontSize = fontSize;
    input.dataset.fontFamily = fontFamily;
    if (isBold) input.style.fontWeight = 'bold';
    if (isItalic) input.style.fontStyle = 'italic';

    overlay.appendChild(input);
    input.focus();
    // Select all text for easy replacement
    const range = document.createRange();
    range.selectNodeContents(input);
    const selection = window.getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    setActiveTextInput(input);

    // Get page info for coordinate conversion
    const pageInfo = PdfBridge.getPageInfo(pageNum);
    const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
    const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
    const pdfX = domX * scaleX;
    const pdfY = pageInfo.page.view[3] - (domY * scaleY);

    function saveEditedText() {
        const text = input.textContent.trim();
        const newIsBold = input.dataset.isBold === 'true';
        const newIsItalic = input.dataset.isItalic === 'true';
        const newFontSize = parseInt(input.dataset.fontSize) || 12;
        const newFontFamily = input.dataset.fontFamily || 'sans-serif';

        // Get actual dimensions before removing
        const textWidth = Math.max(input.offsetWidth, 50);
        const textHeight = Math.max(input.offsetHeight, 20);

        input.remove();
        setActiveTextInput(null);

        if (!text) {
            // User cleared the text - remove the overlay
            textOverlay.remove();
            updateButtons();
            return;
        }

        // Add new operation with updated text and dimensions
        const opId = editSession.addText(pageNum, pdfX, pdfY - 20, textWidth, textHeight, text, newFontSize, '#000000', newFontFamily, newIsItalic, newIsBold);
        operationHistory.push(opId);

        // Update existing overlay IN PLACE (don't remove and recreate - that changes DOM order)
        textOverlay.textContent = text;
        textOverlay.style.display = '';  // Make visible again
        textOverlay.style.fontSize = newFontSize + 'px';
        textOverlay.style.fontFamily = newFontFamily;
        textOverlay.style.fontWeight = newIsBold ? 'bold' : 'normal';
        textOverlay.style.fontStyle = newIsItalic ? 'italic' : 'normal';
        textOverlay.dataset.opId = opId;
        textOverlay.dataset.fontSize = newFontSize;
        textOverlay.dataset.fontFamily = newFontFamily;
        textOverlay.dataset.isBold = newIsBold ? 'true' : 'false';
        textOverlay.dataset.isItalic = newIsItalic ? 'true' : 'false';

        updateButtons();
    }

    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            saveEditedText();
        } else if (e.key === 'Escape') {
            e.preventDefault();
            input.remove();
            setActiveTextInput(null);
            // Restore the old text overlay since user cancelled
            textOverlay.style.display = '';
            // Re-add the operation that was removed
            if (existingText) {
                const opId = editSession.addText(pageNum, pdfX, pdfY - 20, 200, 20, existingText, fontSize, '#000000', fontFamily, isItalic, isBold);
                operationHistory.push(opId);
                textOverlay.dataset.opId = opId;
            }
        }
    });

    input.addEventListener('blur', () => {
        setTimeout(() => {
            if (input.parentElement) {
                saveEditedText();
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

    // Only Select or Whiteout tools can drag whiteout overlays
    if (currentTool !== 'select' && currentTool !== 'whiteout') return;

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

// ============ Text Overlay Dragging ============

let draggingTextOverlay = null;
let textDragStartX = 0;
let textDragStartY = 0;
let textDragStartLeft = 0;
let textDragStartTop = 0;

function makeTextOverlayDraggable(textEl, pageNum) {
    textEl.style.cursor = 'move';

    // Click handler for editing with Text tool
    textEl.addEventListener('click', (e) => {
        if (currentTool === 'text') {
            e.preventDefault();
            e.stopPropagation();
            editExistingTextOverlay(textEl, pageNum);
        }
    });

    textEl.addEventListener('mousedown', (e) => {
        // Only Select tool can drag text overlays
        if (currentTool !== 'select') return;

        e.preventDefault();
        e.stopPropagation();

        draggingTextOverlay = textEl;
        textDragStartX = e.clientX;
        textDragStartY = e.clientY;
        textDragStartLeft = parseFloat(textEl.style.left);
        textDragStartTop = parseFloat(textEl.style.top);

        document.addEventListener('mousemove', handleTextDrag);
        document.addEventListener('mouseup', endTextDrag);
    });
}

// Make replacement overlays (from editing existing PDF text) re-editable
// Uses "undo and re-edit" approach: removes the old replacement and triggers fresh edit on original text
function makeReplaceOverlayEditable(replaceEl, pageNum) {
    replaceEl.style.cursor = 'pointer';

    // Click handler for re-editing via undo-and-reedit
    replaceEl.addEventListener('click', (e) => {
        e.preventDefault();
        e.stopPropagation();

        // 1. Get the original textItem data stored in the overlay
        const originalTextItemJson = replaceEl.dataset.originalTextItem;
        const textItemIndex = replaceEl.dataset.textItemIndex;
        const opId = replaceEl.dataset.opId;

        if (!originalTextItemJson) {
            console.error('Cannot re-edit: no original text item data stored');
            return;
        }

        // Capture the user's intermediate text (what they last saved) BEFORE removing the overlay
        const intermediateText = replaceEl.textContent;

        const textItem = JSON.parse(originalTextItemJson);
        // Override the original text with the user's intermediate text for the editor
        // This way the editor shows what the user last typed, not the original PDF text
        textItem.str = intermediateText;

        // 2. Remove the replacement operation from the edit session
        if (opId && editSession) {
            editSession.removeOperation(BigInt(opId));
            // Remove from history
            const historyIndex = operationHistory.indexOf(BigInt(opId));
            if (historyIndex > -1) {
                operationHistory.splice(historyIndex, 1);
            }
        }

        // 3. Keep the replacement overlay visible to cover the canvas during editing
        // Mark it for removal when save happens (applyTextReplacement will clean it up)
        replaceEl.dataset.pendingRemoval = 'true';

        // 4. Don't unhide the original text item - keep it hidden so canvas text doesn't show
        const originalSpan = document.querySelector(
            `.text-item[data-page="${pageNum}"][data-index="${textItemIndex}"]`
        );

        // 5. Trigger fresh edit on the original text item
        if (originalSpan) {
            startTextEdit(pageNum, parseInt(textItemIndex), textItem, originalSpan);
        } else {
            console.error('Cannot find original text item span to re-edit');
        }
    });

    // Hover highlight for visual feedback
    replaceEl.addEventListener('mouseenter', () => {
        if (currentTool === 'select' || currentTool === 'text' || currentTool === 'edit-text') {
            replaceEl.style.outline = '2px solid #007bff';
        }
    });
    replaceEl.addEventListener('mouseleave', () => {
        replaceEl.style.outline = '';
    });
}

function handleTextDrag(e) {
    if (!draggingTextOverlay) return;

    const dx = e.clientX - textDragStartX;
    const dy = e.clientY - textDragStartY;

    draggingTextOverlay.style.left = (textDragStartLeft + dx) + 'px';
    draggingTextOverlay.style.top = (textDragStartTop + dy) + 'px';
}

function endTextDrag(e) {
    if (!draggingTextOverlay) return;

    // Remove event listeners first
    document.removeEventListener('mousemove', handleTextDrag);
    document.removeEventListener('mouseup', endTextDrag);

    const textEl = draggingTextOverlay;
    draggingTextOverlay = null;

    // Only update if position actually changed
    const newLeft = parseFloat(textEl.style.left);
    const newTop = parseFloat(textEl.style.top);
    if (newLeft === textDragStartLeft && newTop === textDragStartTop) return;

    // Get operation data
    const opId = textEl.dataset.opId;
    const pageNum = parseInt(textEl.closest('.edit-page').dataset.page);
    const text = textEl.textContent;
    const fontSize = parseInt(textEl.dataset.fontSize) || 12;
    const fontFamily = textEl.dataset.fontFamily || 'sans-serif';
    const isBold = textEl.dataset.isBold === 'true';
    const isItalic = textEl.dataset.isItalic === 'true';

    // Remove old operation
    if (opId && editSession) {
        try {
            editSession.removeOperation(BigInt(opId));
            const historyIndex = operationHistory.indexOf(BigInt(opId));
            if (historyIndex > -1) {
                operationHistory.splice(historyIndex, 1);
            }
        } catch (err) {
            console.error('Error removing text operation:', err);
        }
    }

    // Convert new position to PDF coordinates
    const pageInfo = PdfBridge.getPageInfo(pageNum);
    if (pageInfo) {
        const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
        const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;

        const pdfX = newLeft * scaleX;
        const pdfY = pageInfo.page.view[3] - (newTop * scaleY);

        // Add new text operation at new position
        const newOpId = editSession.addText(
            pageNum, pdfX, pdfY - 20, 200, 20, text,
            fontSize, '#000000', fontFamily, isItalic, isBold
        );
        operationHistory.push(newOpId);
        textEl.dataset.opId = newOpId;
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

    // Store original dimensions for potential restoration
    const originalWidth = domWidth;
    const originalHeight = domHeight;

    // Detect covered text style
    const coveredStyle = await detectCoveredTextStyle(pageNum, domX, domY, domWidth, domHeight);

    // Create auto-expanding contentEditable span INSIDE the whiteout
    const input = document.createElement('span');
    input.contentEditable = true;
    input.className = 'whiteout-text-input';
    input.style.display = 'block';
    input.style.minWidth = '100%';
    input.style.minHeight = '100%';
    input.style.border = 'none';
    input.style.outline = 'none';
    input.style.background = 'transparent';
    input.style.padding = '2px 4px';
    input.style.boxSizing = 'border-box';
    input.style.textAlign = 'center';
    input.style.whiteSpace = 'pre-wrap';
    input.style.wordBreak = 'break-word';
    input.style.overflow = 'visible';

    // Apply covered text style (including bold/italic)
    input.style.fontSize = coveredStyle.fontSize + 'px';
    input.style.fontFamily = coveredStyle.fontFamily;
    input.style.color = '#000000';
    if (coveredStyle.isBold) input.style.fontWeight = 'bold';
    if (coveredStyle.isItalic) input.style.fontStyle = 'italic';

    // Store style info for saving
    input.dataset.fontSize = coveredStyle.fontSize;
    input.dataset.fontFamily = coveredStyle.fontFamily;
    input.dataset.isBold = coveredStyle.isBold ? 'true' : 'false';
    input.dataset.isItalic = coveredStyle.isItalic ? 'true' : 'false';

    whiteRect.appendChild(input);
    whiteRect.classList.add('editing');
    // Allow whiteout to expand with content
    whiteRect.style.overflow = 'visible';
    input.focus();
    setActiveTextInput(input);

    // Auto-expand whiteout as user types
    function expandWhiteoutForText() {
        const textWidth = input.scrollWidth + 16; // padding
        const textHeight = input.scrollHeight + 8;
        const currentWidth = parseFloat(whiteRect.style.width);
        const currentHeight = parseFloat(whiteRect.style.height);

        if (textWidth > currentWidth) {
            whiteRect.style.width = textWidth + 'px';
        }
        if (textHeight > currentHeight) {
            whiteRect.style.height = textHeight + 'px';
        }
    }

    input.addEventListener('input', expandWhiteoutForText);

    // Handle Enter to save
    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            whiteRect.classList.remove('editing');
            saveWhiteoutText(whiteRect, pageNum, input, originalWidth, originalHeight);
        } else if (e.key === 'Escape') {
            e.preventDefault();
            whiteRect.classList.remove('editing');
            // Restore original dimensions
            whiteRect.style.width = originalWidth + 'px';
            whiteRect.style.height = originalHeight + 'px';
            whiteRect.style.overflow = 'hidden';
            input.remove();
            setActiveTextInput(null);
        }
    });

    // Handle blur to save
    input.addEventListener('blur', () => {
        // Small delay to allow click events on style buttons to process
        setTimeout(() => {
            // Don't close if still focused (style button click refocuses)
            if (input.matches(':focus')) return;

            whiteRect.classList.remove('editing');
            if (input.parentElement && input.textContent.trim()) {
                saveWhiteoutText(whiteRect, pageNum, input, originalWidth, originalHeight);
            } else if (input.parentElement) {
                // Restore original dimensions
                whiteRect.style.width = originalWidth + 'px';
                whiteRect.style.height = originalHeight + 'px';
                whiteRect.style.overflow = 'hidden';
                input.remove();
                setActiveTextInput(null);
            }
        }, 200);
    });
}

async function detectCoveredTextStyle(pageNum, domX, domY, domWidth, domHeight) {
    // Default style
    const defaultStyle = {
        fontSize: 12,
        fontFamily: 'Helvetica, Arial, sans-serif',
        isBold: false,
        isItalic: false
    };

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
            fontFamily: item.fontFamily || defaultStyle.fontFamily,
            isBold: item.isBold || false,
            isItalic: item.isItalic || false
        };
    } catch (err) {
        console.error('Error detecting covered text style:', err);
        return defaultStyle;
    }
}

function saveWhiteoutText(whiteRect, pageNum, input, originalWidth, originalHeight) {
    const text = input.textContent.trim();
    if (!text) {
        // Restore original dimensions if no text
        if (originalWidth) whiteRect.style.width = originalWidth + 'px';
        if (originalHeight) whiteRect.style.height = originalHeight + 'px';
        whiteRect.style.overflow = 'hidden';
        input.remove();
        setActiveTextInput(null);
        return;
    }

    // Get position and style info (including potentially expanded dimensions)
    const domX = parseFloat(whiteRect.style.left);
    const domY = parseFloat(whiteRect.style.top);
    const domWidth = parseFloat(whiteRect.style.width);
    const domHeight = parseFloat(whiteRect.style.height);
    const fontSize = parseFloat(input.dataset.fontSize) || 12;
    const fontFamily = input.dataset.fontFamily || null;
    const isBold = input.dataset.isBold === 'true';
    const isItalic = input.dataset.isItalic === 'true';

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

    // If whiteout was resized, update the whiteout operation
    if (originalWidth && originalHeight && (domWidth !== originalWidth || domHeight !== originalHeight)) {
        const existingOpId = whiteRect.dataset.opId;
        if (existingOpId && editSession) {
            editSession.removeOperation(BigInt(existingOpId));
            // Remove from history
            const historyIndex = operationHistory.indexOf(BigInt(existingOpId));
            if (historyIndex > -1) {
                operationHistory.splice(historyIndex, 1);
            }
            // Add new whiteout with updated dimensions
            const newWhiteOpId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight);
            operationHistory.push(newWhiteOpId);
            whiteRect.dataset.opId = newWhiteOpId;
        }
    }

    // Add text annotation at the whiteout position (with font styling)
    const opId = editSession.addText(
        pageNum,
        pdfX,
        pdfY,
        pdfWidth,
        pdfHeight,
        text,
        fontSize,
        '#000000',
        fontFamily,  // font name
        isItalic,    // is_italic
        isBold       // is_bold
    );
    operationHistory.push(opId);

    // Replace input with text span INSIDE the whiteout (auto-sizing)
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
    if (isBold) textSpan.style.fontWeight = 'bold';
    if (isItalic) textSpan.style.fontStyle = 'italic';
    textSpan.style.whiteSpace = 'pre-wrap';
    textSpan.style.wordBreak = 'break-word';
    textSpan.dataset.opId = opId;
    textSpan.dataset.fontSize = fontSize;
    textSpan.dataset.fontFamily = fontFamily || 'sans-serif';
    textSpan.dataset.isBold = isBold ? 'true' : 'false';
    textSpan.dataset.isItalic = isItalic ? 'true' : 'false';

    // Remove input and add text span
    input.remove();
    setActiveTextInput(null);
    whiteRect.style.overflow = 'hidden';
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
    // Store font size and family for style controls
    input.dataset.fontSize = Math.round(fontSize);
    input.dataset.fontFamily = textItem.fontFamily || 'sans-serif';
    // Initialize bold/italic state from detected text item
    input.dataset.isBold = textItem.isBold ? 'true' : 'false';
    input.dataset.isItalic = textItem.isItalic ? 'true' : 'false';
    if (textItem.isItalic) input.style.fontStyle = 'italic';
    if (textItem.isBold) input.style.fontWeight = 'bold';

    // Position near the text item
    const bounds = textItem.domBounds;
    editor.style.left = bounds.x + 'px';
    editor.style.top = (bounds.y + bounds.height + 5) + 'px';

    const pageDiv = document.querySelector(`.edit-page[data-page="${pageNum}"]`);
    pageDiv.appendChild(editor);

    // Focus input and register with style buttons
    input.focus();
    input.select();
    setActiveTextInput(input);

    // Event handlers
    editor.querySelector('.text-editor-save').addEventListener('click', () => {
        const newText = input.value;
        const isBold = input.dataset.isBold === 'true';
        const isItalic = input.dataset.isItalic === 'true';
        const customFontSize = parseFloat(input.dataset.fontSize) || null;
        const customFontFamily = input.dataset.fontFamily || null;
        if (newText !== textItem.str || isBold !== textItem.isBold || isItalic !== textItem.isItalic ||
            customFontSize !== Math.round((textItem.pdfHeight || 12) * 1.5) || customFontFamily !== textItem.fontFamily) {
            applyTextReplacement(pageNum, textItem, newText, isBold, isItalic, customFontSize, customFontFamily);
        }
        closeTextEditor();
    });

    editor.querySelector('.text-editor-cancel').addEventListener('click', closeTextEditor);

    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            const newText = input.value;
            const isBold = input.dataset.isBold === 'true';
            const isItalic = input.dataset.isItalic === 'true';
            const customFontSize = parseFloat(input.dataset.fontSize) || null;
            const customFontFamily = input.dataset.fontFamily || null;
            if (newText !== textItem.str || isBold !== textItem.isBold || isItalic !== textItem.isItalic ||
                customFontSize !== Math.round((textItem.pdfHeight || 12) * 1.5) || customFontFamily !== textItem.fontFamily) {
                applyTextReplacement(pageNum, textItem, newText, isBold, isItalic, customFontSize, customFontFamily);
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

    setActiveTextInput(null);
}

function applyTextReplacement(pageNum, textItem, newText, isBold = null, isItalic = null, customFontSize = null, customFontFamily = null) {
    // Get page info for coordinate conversion
    const pageInfo = PdfBridge.getPageInfo(pageNum);
    if (!pageInfo) return;

    // Use explicit bold/italic if provided, otherwise fall back to detected values
    const useBold = isBold !== null ? isBold : (textItem.isBold || false);
    const useItalic = isItalic !== null ? isItalic : (textItem.isItalic || false);

    // Use custom font size if provided (from toolbar), otherwise estimate from text height
    // customFontSize comes in as DOM pixels, need to convert to PDF points
    const renderScale = 1.5;
    const fontSize = customFontSize !== null ? (customFontSize / renderScale) : (textItem.pdfHeight || 12.0);

    // Use custom font family if provided (from toolbar)
    const useFontFamily = customFontFamily || textItem.fontFamily || null;

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
        // Font family from toolbar or PDF.js styles
        useFontFamily,
        // Font style flags
        useItalic,
        useBold
    );

    operationHistory.push(opId);

    // Calculate DOM font size (use custom or scale from PDF)
    const domFontSize = customFontSize !== null ? customFontSize : ((textItem.pdfHeight || 12) * renderScale);
    const fontFamily = mapFontFamilyForPreview(useFontFamily);

    // Add visual indicator (replacement overlay) with matching font
    const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);

    // Clean up any old overlay marked for removal (from re-editing)
    const oldOverlay = overlay.querySelector('.edit-replace-overlay[data-pending-removal="true"]');
    if (oldOverlay) {
        oldOverlay.remove();
    }

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
    if (useItalic) replaceEl.style.fontStyle = 'italic';
    if (useBold) replaceEl.style.fontWeight = 'bold';

    replaceEl.dataset.opId = opId;
    // Store original textItem for undo-and-reedit approach
    replaceEl.dataset.textItemIndex = textItem.index;
    replaceEl.dataset.originalTextItem = JSON.stringify({
        index: textItem.index,
        str: textItem.str,
        pdfX: textItem.pdfX,
        pdfY: textItem.pdfY,
        pdfWidth: textItem.pdfWidth,
        pdfHeight: textItem.pdfHeight,
        fontFamily: textItem.fontFamily,
        isBold: textItem.isBold,
        isItalic: textItem.isItalic,
        domBounds: textItem.domBounds
    });
    overlay.appendChild(replaceEl);

    // Make replacement overlay re-editable (click to undo and re-edit)
    makeReplaceOverlayEditable(replaceEl, pageNum);

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

// Map detected font family to dropdown option value
function mapFontFamilyToDropdown(fontFamily) {
    if (!fontFamily) return 'sans-serif';

    const lower = fontFamily.toLowerCase();

    // Exact matches for dropdown values
    if (lower === 'sans-serif') return 'sans-serif';
    if (lower === 'serif') return 'serif';
    if (lower === 'monospace') return 'monospace';
    if (lower === 'arial') return 'Arial';
    if (lower === 'times new roman') return 'Times New Roman';
    if (lower === 'georgia') return 'Georgia';
    if (lower === 'courier new') return 'Courier New';
    if (lower === 'verdana') return 'Verdana';
    if (lower === 'trebuchet ms') return 'Trebuchet MS';

    // Partial matches for detected fonts
    if (lower.includes('times')) return 'Times New Roman';
    if (lower.includes('arial')) return 'Arial';
    if (lower.includes('helvetica')) return 'sans-serif';
    if (lower.includes('courier') || lower.includes('mono')) return 'Courier New';
    if (lower.includes('georgia')) return 'Georgia';
    if (lower.includes('verdana')) return 'Verdana';
    if (lower.includes('trebuchet')) return 'Trebuchet MS';

    // Default to sans-serif
    return 'sans-serif';
}

// ============ Bold/Italic Style Functions ============

function setActiveTextInput(input) {
    activeTextInput = input;
    updateStyleButtons();

    if (input) {
        // Track focus/blur to update active state
        input.addEventListener('blur', handleTextInputBlur);
    }
}

function handleTextInputBlur() {
    // Small delay to allow click events on style buttons to process
    setTimeout(() => {
        if (activeTextInput && !activeTextInput.matches(':focus')) {
            activeTextInput.removeEventListener('blur', handleTextInputBlur);
            activeTextInput = null;
            updateStyleButtons();
        }
    }, 150);
}

function updateStyleButtons() {
    const boldBtn = document.getElementById('style-bold');
    const italicBtn = document.getElementById('style-italic');
    const fontSizeDecrease = document.getElementById('font-size-decrease');
    const fontSizeIncrease = document.getElementById('font-size-increase');
    const fontSizeValue = document.getElementById('font-size-value');
    const fontFamilySelect = document.getElementById('style-font-family');

    if (!activeTextInput) {
        boldBtn.disabled = true;
        italicBtn.disabled = true;
        fontSizeDecrease.disabled = true;
        fontSizeIncrease.disabled = true;
        fontSizeValue.disabled = true;
        fontFamilySelect.disabled = true;
        boldBtn.classList.remove('active');
        italicBtn.classList.remove('active');
        return;
    }

    boldBtn.disabled = false;
    italicBtn.disabled = false;
    fontSizeDecrease.disabled = false;
    fontSizeIncrease.disabled = false;
    fontSizeValue.disabled = false;
    fontFamilySelect.disabled = false;

    // Check current state from input's dataset or computed style
    const isBold = activeTextInput.dataset.isBold === 'true' ||
                   activeTextInput.style.fontWeight === 'bold' ||
                   activeTextInput.style.fontWeight === '700';
    const isItalic = activeTextInput.dataset.isItalic === 'true' ||
                     activeTextInput.style.fontStyle === 'italic';

    boldBtn.classList.toggle('active', isBold);
    italicBtn.classList.toggle('active', isItalic);

    // Sync font size value
    const fontSize = activeTextInput.dataset.fontSize || '12';
    fontSizeValue.value = fontSize;

    // Sync font family dropdown (map detected font to dropdown option)
    const fontFamily = activeTextInput.dataset.fontFamily || 'sans-serif';
    fontFamilySelect.value = mapFontFamilyToDropdown(fontFamily);
}

function toggleBold() {
    if (!activeTextInput) return;

    const currentBold = activeTextInput.dataset.isBold === 'true';
    const newBold = !currentBold;

    activeTextInput.dataset.isBold = newBold;
    activeTextInput.style.fontWeight = newBold ? 'bold' : 'normal';

    updateStyleButtons();
    activeTextInput.focus();
}

function toggleItalic() {
    if (!activeTextInput) return;

    const currentItalic = activeTextInput.dataset.isItalic === 'true';
    const newItalic = !currentItalic;

    activeTextInput.dataset.isItalic = newItalic;
    activeTextInput.style.fontStyle = newItalic ? 'italic' : 'normal';

    updateStyleButtons();
    activeTextInput.focus();
}

function increaseFontSize() {
    if (!activeTextInput) return;
    const current = parseInt(activeTextInput.dataset.fontSize) || 12;
    setFontSize(Math.min(current + 2, 72));
}

function decreaseFontSize() {
    if (!activeTextInput) return;
    const current = parseInt(activeTextInput.dataset.fontSize) || 12;
    setFontSize(Math.max(current - 2, 6));
}

function setFontSize(size) {
    if (!activeTextInput) return;
    size = Math.max(6, Math.min(72, parseInt(size) || 12));
    activeTextInput.dataset.fontSize = size;
    activeTextInput.style.fontSize = size + 'px';
    document.getElementById('font-size-value').value = size;
    updateStyleButtons();
    activeTextInput.focus();
}

function setFontFamily(family) {
    if (!activeTextInput) return;
    activeTextInput.dataset.fontFamily = family;
    activeTextInput.style.fontFamily = family;
    updateStyleButtons();
    activeTextInput.focus();
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
