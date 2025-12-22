// PDF Tools - Single Page App
// Uses window.wasmBindings from Trunk-injected WASM loader

const { PdfJoinSession, SessionMode, format_bytes } = window.wasmBindings;

let splitSession = null;
let mergeSession = null;

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

    console.log('PDF Tools initialized (WASM-first architecture)');
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
        const bytes = new Uint8Array(await file.arrayBuffer());
        const info = splitSession.addDocument(file.name, bytes);

        // Update UI
        document.getElementById('split-drop-zone').classList.add('hidden');
        document.getElementById('split-editor').classList.remove('hidden');

        document.getElementById('split-file-name').textContent = file.name;
        document.getElementById('split-file-details').textContent =
            `${info.page_count} pages - ${format_bytes(info.size_bytes)}`;

        // Set default range
        document.getElementById('page-range').value = `1-${info.page_count}`;
        document.getElementById('split-btn').disabled = !splitSession.canExecute();
    } catch (e) {
        alert('Error: ' + e);
    }
}

function resetSplitView() {
    splitSession.removeDocument(0);

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

    splitBtn.disabled = true;
    progress.classList.remove('hidden');

    try {
        const result = splitSession.execute();
        downloadBlob(result, 'split.pdf');
    } catch (e) {
        alert('Split failed: ' + e);
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

// ============ Merge View ============

function setupMergeView() {
    const dropZone = document.getElementById('merge-drop-zone');
    const fileInput = document.getElementById('merge-file-input');
    const browseBtn = document.getElementById('merge-browse-btn');
    const addBtn = document.getElementById('merge-add-btn');
    const mergeBtn = document.getElementById('merge-btn');

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

        try {
            const bytes = new Uint8Array(await file.arrayBuffer());
            mergeSession.addDocument(file.name, bytes);
        } catch (e) {
            alert(`Error loading ${file.name}: ${e}`);
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

    // Update count
    document.getElementById('merge-count').textContent = `(${count})`;

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
        downloadBlob(result, 'merged.pdf');
    } catch (e) {
        alert('Merge failed: ' + e);
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
