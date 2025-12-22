// PDFJoin - Single Page App
// Uses window.wasmBindings from Trunk-injected WASM loader

const { PdfJoinSession, SessionMode, format_bytes } = window.wasmBindings;

// Size thresholds
const LARGE_FILE_WARNING_BYTES = 50 * 1024 * 1024; // 50 MB
const VERY_LARGE_FILE_WARNING_BYTES = 100 * 1024 * 1024; // 100 MB

let splitSession = null;
let mergeSession = null;
let splitOriginalFilename = null; // Track original filename for smart naming

export function init() {
    // Initialize WASM sessions
    splitSession = new PdfJoinSession(SessionMode.Split);
    mergeSession = new PdfJoinSession(SessionMode.Merge);

    // Set up progress callbacks
    splitSession.setProgressCallback(onSplitProgress);
    mergeSession.setProgressCallback(onMergeProgress);

    // Set up UI
    setupTabs();
    setupSplitView();
    setupMergeView();

    console.log('PDFJoin initialized (WASM-first architecture)');
}

// ============ Tab Navigation ============

function setupTabs() {
    const tabs = document.querySelectorAll('.tab');
    tabs.forEach(tab => {
        tab.addEventListener('click', () => {
            // Update active tab
            tabs.forEach(t => t.classList.remove('active'));
            tab.classList.add('active');

            // Show corresponding view
            const tabName = tab.dataset.tab;
            document.querySelectorAll('.view').forEach(v => v.classList.add('hidden'));
            document.getElementById(`${tabName}-view`).classList.remove('hidden');
        });
    });
}

// ============ Split View ============

function setupSplitView() {
    const dropZone = document.getElementById('split-drop-zone');
    const fileInput = document.getElementById('split-file-input');
    const browseBtn = document.getElementById('split-browse-btn');
    const removeBtn = document.getElementById('split-remove-btn');
    const splitBtn = document.getElementById('split-btn');
    const rangeInput = document.getElementById('page-range');

    browseBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        fileInput.click();
    });

    dropZone.addEventListener('click', () => fileInput.click());
    dropZone.addEventListener('dragover', (e) => {
        e.preventDefault();
        dropZone.classList.add('drag-over');
    });
    dropZone.addEventListener('dragleave', () => dropZone.classList.remove('drag-over'));
    dropZone.addEventListener('drop', (e) => {
        e.preventDefault();
        dropZone.classList.remove('drag-over');
        const files = e.dataTransfer.files;
        if (files.length > 0 && files[0].type === 'application/pdf') {
            handleSplitFile(files[0]);
        }
    });

    fileInput.addEventListener('change', (e) => {
        if (e.target.files.length > 0) handleSplitFile(e.target.files[0]);
    });

    removeBtn.addEventListener('click', resetSplitView);
    splitBtn.addEventListener('click', executeSplit);
    rangeInput.addEventListener('input', validateRange);
}

async function handleSplitFile(file) {
    try {
        // Check file size and warn if large
        if (file.size > VERY_LARGE_FILE_WARNING_BYTES) {
            if (!confirm(`This file is ${format_bytes(file.size)} which is very large. Processing may be slow or fail on some devices. Continue?`)) {
                return;
            }
        } else if (file.size > LARGE_FILE_WARNING_BYTES) {
            console.warn(`Large file: ${format_bytes(file.size)} - processing may take longer`);
        }

        const bytes = new Uint8Array(await file.arrayBuffer());
        const info = splitSession.addDocument(file.name, bytes);

        // Store original filename for smart output naming
        splitOriginalFilename = file.name.replace(/\.pdf$/i, '');

        // Update UI
        document.getElementById('split-drop-zone').classList.add('hidden');
        document.getElementById('split-editor').classList.remove('hidden');

        document.getElementById('split-file-name').textContent = file.name;
        document.getElementById('split-file-details').textContent =
            `${info.page_count} pages - ${format_bytes(info.size_bytes)}`;

        // Update example chips with page count
        updateExampleChips(info.page_count);

        // Don't auto-fill range - let placeholder show syntax examples
        document.getElementById('page-range').value = '';
        document.getElementById('split-btn').disabled = true;
    } catch (e) {
        showError('split-error', e.toString());
    }
}

function resetSplitView() {
    splitSession.removeDocument(0);
    splitOriginalFilename = null;

    document.getElementById('split-drop-zone').classList.remove('hidden');
    document.getElementById('split-editor').classList.add('hidden');
    document.getElementById('split-file-input').value = '';
    document.getElementById('page-range').value = '';
    document.getElementById('split-btn').disabled = true;
}

function validateRange() {
    const rangeInput = document.getElementById('page-range');
    const splitBtn = document.getElementById('split-btn');

    try {
        splitSession.setPageSelection(rangeInput.value);
        rangeInput.classList.remove('invalid');
        splitBtn.disabled = !splitSession.canExecute();
    } catch (e) {
        rangeInput.classList.add('invalid');
        splitBtn.disabled = true;
    }
}

async function executeSplit() {
    const splitBtn = document.getElementById('split-btn');
    const progress = document.getElementById('split-progress');
    const rangeInput = document.getElementById('page-range');
    const multiFileCheckbox = document.getElementById('split-multiple-files');

    splitBtn.disabled = true;
    progress.classList.remove('hidden');

    try {
        const isMultiFile = multiFileCheckbox?.checked;
        const fullRange = rangeInput.value;

        if (isMultiFile && fullRange.includes(',')) {
            // Multi-file mode: split each comma-separated range into its own file
            const ranges = fullRange.split(',').map(r => r.trim()).filter(r => r);

            for (let i = 0; i < ranges.length; i++) {
                const range = ranges[i];
                // Update progress
                const progressText = document.querySelector('#split-progress .progress-text');
                if (progressText) {
                    progressText.textContent = `Processing range ${i + 1} of ${ranges.length}...`;
                }

                // Set selection to just this range and execute
                splitSession.setPageSelection(range);
                const result = splitSession.execute();

                // Download with range-specific filename
                const rangeLabel = range.replace(/\s+/g, '');
                const filename = `${splitOriginalFilename || 'split'}-pages-${rangeLabel}.pdf`;
                downloadBlob(result, filename);

                // Small delay between downloads to avoid browser issues
                if (i < ranges.length - 1) {
                    await new Promise(r => setTimeout(r, 100));
                }
            }

            // Restore original selection
            splitSession.setPageSelection(fullRange);
        } else {
            // Single file mode (original behavior)
            const result = splitSession.execute();
            const range = fullRange.replace(/\s+/g, '').replace(/,/g, '_');
            const filename = `${splitOriginalFilename || 'split'}-pages-${range}.pdf`;
            downloadBlob(result, filename);
        }
    } catch (e) {
        showError('split-error', 'Split failed: ' + e);
    } finally {
        splitBtn.disabled = false;
        setTimeout(() => progress.classList.add('hidden'), 500);
    }
}

function onSplitProgress(current, total, message) {
    const progressFill = document.querySelector('#split-progress .progress-fill');
    const progressText = document.querySelector('#split-progress .progress-text');
    if (progressFill) progressFill.style.width = `${(current / total) * 100}%`;
    if (progressText) progressText.textContent = message;
}

function updateExampleChips(pageCount) {
    const container = document.getElementById('range-chips');
    if (!container) return;

    container.innerHTML = '';

    // Generate dynamic chips based on page count
    const chips = [];

    if (pageCount >= 1) {
        chips.push({ label: 'First page', range: '1' });
    }
    if (pageCount >= 5) {
        chips.push({ label: 'First 5', range: '1-5' });
    }
    if (pageCount >= 3) {
        const last3Start = pageCount - 2;
        chips.push({ label: 'Last 3', range: `${last3Start}-${pageCount}` });
    }
    if (pageCount >= 1) {
        chips.push({ label: 'All pages', range: `1-${pageCount}` });
    }

    chips.forEach(({ label, range }) => {
        const chip = document.createElement('button');
        chip.className = 'chip';
        chip.type = 'button';
        chip.textContent = label;
        chip.dataset.range = range;
        chip.addEventListener('click', () => {
            const rangeInput = document.getElementById('page-range');
            rangeInput.value = range;
            validateRange();
        });
        container.appendChild(chip);
    });
}

// ============ Merge View ============

function setupMergeView() {
    const dropZone = document.getElementById('merge-drop-zone');
    const fileInput = document.getElementById('merge-file-input');
    const browseBtn = document.getElementById('merge-browse-btn');
    const addBtn = document.getElementById('merge-add-btn');
    const mergeBtn = document.getElementById('merge-btn');
    const fileList = document.getElementById('merge-file-list');

    browseBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        fileInput.click();
    });

    dropZone.addEventListener('click', () => fileInput.click());
    dropZone.addEventListener('dragover', (e) => {
        e.preventDefault();
        dropZone.classList.add('drag-over');
    });
    dropZone.addEventListener('dragleave', () => dropZone.classList.remove('drag-over'));
    dropZone.addEventListener('drop', (e) => {
        e.preventDefault();
        dropZone.classList.remove('drag-over');
        handleMergeFiles(e.dataTransfer.files);
    });

    // Also allow drag-and-drop on the file list for adding more files
    fileList.addEventListener('dragover', (e) => {
        e.preventDefault();
        fileList.classList.add('drag-over');
    });
    fileList.addEventListener('dragleave', () => fileList.classList.remove('drag-over'));
    fileList.addEventListener('drop', (e) => {
        e.preventDefault();
        fileList.classList.remove('drag-over');
        handleMergeFiles(e.dataTransfer.files);
    });

    fileInput.addEventListener('change', (e) => {
        handleMergeFiles(e.target.files);
        e.target.value = ''; // Allow re-selecting same files
    });

    addBtn.addEventListener('click', () => fileInput.click());
    mergeBtn.addEventListener('click', executeMerge);
}

async function handleMergeFiles(files) {
    for (const file of files) {
        if (file.type !== 'application/pdf') continue;

        // Check file size and warn if large
        if (file.size > VERY_LARGE_FILE_WARNING_BYTES) {
            if (!confirm(`"${file.name}" is ${format_bytes(file.size)} which is very large. Processing may be slow. Continue?`)) {
                continue;
            }
        }

        try {
            const bytes = new Uint8Array(await file.arrayBuffer());
            mergeSession.addDocument(file.name, bytes);
        } catch (e) {
            showError('merge-error', `${file.name}: ${e}`);
        }
    }

    updateMergeFileList();
}

function updateMergeFileList() {
    const infos = mergeSession.getDocumentInfos();
    const count = mergeSession.getDocumentCount();

    // Show/hide appropriate sections
    const hasFiles = count > 0;
    document.getElementById('merge-drop-zone').classList.toggle('hidden', hasFiles);
    document.getElementById('merge-file-list').classList.toggle('hidden', !hasFiles);

    // Update count and total size
    const totalSize = infos.reduce((sum, info) => sum + info.size_bytes, 0);
    const totalPages = infos.reduce((sum, info) => sum + info.page_count, 0);
    document.getElementById('merge-count').textContent = `(${count} files, ${totalPages} pages, ${format_bytes(totalSize)})`;

    // Update file list
    const ul = document.getElementById('merge-files');
    ul.innerHTML = '';

    infos.forEach((info, idx) => {
        const li = document.createElement('li');
        li.draggable = true;
        li.dataset.index = idx;
        li.innerHTML = `
            <span class="drag-handle">☰</span>
            <span class="file-name">${info.name}</span>
            <span class="file-size">${info.page_count} pages - ${format_bytes(info.size_bytes)}</span>
            <button class="remove-btn" data-index="${idx}">×</button>
        `;

        // Remove button
        li.querySelector('.remove-btn').addEventListener('click', () => {
            mergeSession.removeDocument(idx);
            updateMergeFileList();
        });

        // Drag events for reordering
        li.addEventListener('dragstart', onDragStart);
        li.addEventListener('dragover', onDragOver);
        li.addEventListener('drop', onDrop);
        li.addEventListener('dragend', onDragEnd);

        ul.appendChild(li);
    });

    // Update merge button state
    document.getElementById('merge-btn').disabled = !mergeSession.canExecute();
}

// Drag and drop reordering
let draggedIndex = null;

function onDragStart(e) {
    draggedIndex = parseInt(e.target.dataset.index);
    e.target.classList.add('dragging');
}

function onDragOver(e) {
    e.preventDefault();
    const li = e.target.closest('li');
    if (li) li.classList.add('drag-over');
}

function onDrop(e) {
    e.preventDefault();
    const li = e.target.closest('li');
    if (!li) return;

    const dropIndex = parseInt(li.dataset.index);
    if (draggedIndex !== null && draggedIndex !== dropIndex) {
        // Build new order
        const count = mergeSession.getDocumentCount();
        const order = [...Array(count).keys()];
        order.splice(draggedIndex, 1);
        order.splice(dropIndex, 0, draggedIndex);

        try {
            mergeSession.reorderDocuments(new Uint32Array(order));
            updateMergeFileList();
        } catch (e) {
            console.error('Reorder failed:', e);
        }
    }
}

function onDragEnd(e) {
    draggedIndex = null;
    document.querySelectorAll('.dragging, .drag-over').forEach(el => {
        el.classList.remove('dragging', 'drag-over');
    });
}

async function executeMerge() {
    const mergeBtn = document.getElementById('merge-btn');
    const progress = document.getElementById('merge-progress');

    mergeBtn.disabled = true;
    progress.classList.remove('hidden');

    try {
        const result = mergeSession.execute();
        // Smart filename: merged-3-files.pdf
        const count = mergeSession.getDocumentCount();
        const filename = `merged-${count}-files.pdf`;
        downloadBlob(result, filename);
    } catch (e) {
        showError('merge-error', 'Merge failed: ' + e);
    } finally {
        mergeBtn.disabled = false;
        setTimeout(() => progress.classList.add('hidden'), 500);
    }
}

function onMergeProgress(current, total, message) {
    const progressFill = document.querySelector('#merge-progress .progress-fill');
    const progressText = document.querySelector('#merge-progress .progress-text');
    if (progressFill) progressFill.style.width = `${(current / total) * 100}%`;
    if (progressText) progressText.textContent = message;
}

// ============ Utilities ============

function showError(containerId, message) {
    const container = document.getElementById(containerId);
    const textEl = container.querySelector('.error-text');
    const dismissBtn = container.querySelector('.dismiss');

    textEl.textContent = message;
    container.classList.remove('hidden');

    // Auto-dismiss after 8 seconds
    const timer = setTimeout(() => container.classList.add('hidden'), 8000);

    // Manual dismiss
    dismissBtn.onclick = () => {
        clearTimeout(timer);
        container.classList.add('hidden');
    };
}

function downloadBlob(data, filename) {
    const blob = new Blob([data], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
}
