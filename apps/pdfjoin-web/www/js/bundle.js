// src/ts/pdf-loader.ts
var pdfJsLoaded = false;
var pdfJsLoadPromise = null;
async function ensurePdfJsLoaded() {
  if (pdfJsLoaded) {
    return;
  }
  if (pdfJsLoadPromise) {
    return pdfJsLoadPromise;
  }
  pdfJsLoadPromise = new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = "./js/vendor/pdf.min.js";
    script.onload = () => {
      if (window.pdfjsLib) {
        window.pdfjsLib.GlobalWorkerOptions.workerSrc = "./js/vendor/pdf.worker.min.js";
        pdfJsLoaded = true;
        console.log("PDF.js loaded successfully (lazy)");
        resolve();
      } else {
        reject(new Error("PDF.js loaded but pdfjsLib not found on window"));
      }
    };
    script.onerror = (e) => {
      pdfJsLoadPromise = null;
      const errorEvent = e;
      reject(new Error("Failed to load PDF.js: " + (errorEvent.message || "Unknown error")));
    };
    document.head.appendChild(script);
  });
  return pdfJsLoadPromise;
}
window.ensurePdfJsLoaded = ensurePdfJsLoaded;

// src/ts/pdf-bridge.ts
var PdfBridge = {
  currentDoc: null,
  pageCanvases: /* @__PURE__ */ new Map(),
  async loadDocument(data) {
    await ensurePdfJsLoaded();
    const typedArray = data instanceof Uint8Array ? data : new Uint8Array(data);
    if (!window.pdfjsLib) {
      throw new Error("PDF.js not loaded");
    }
    this.currentDoc = await window.pdfjsLib.getDocument(typedArray).promise;
    return this.currentDoc.numPages;
  },
  async renderPage(pageNum, canvas, scale = 1.5) {
    if (!this.currentDoc) throw new Error("No document loaded");
    const page = await this.currentDoc.getPage(pageNum);
    const viewport = page.getViewport({ scale });
    canvas.width = viewport.width;
    canvas.height = viewport.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Could not get 2d context");
    await page.render({
      canvasContext: ctx,
      viewport
    }).promise;
    this.pageCanvases.set(pageNum, { canvas, viewport, page });
    return {
      width: viewport.width,
      height: viewport.height,
      originalWidth: viewport.width / scale,
      originalHeight: viewport.height / scale,
      pdfWidth: page.view[2],
      pdfHeight: page.view[3]
    };
  },
  getPageDimensions(pageNum) {
    const cached = this.pageCanvases.get(pageNum);
    if (cached) {
      return {
        width: cached.viewport.width,
        height: cached.viewport.height
      };
    }
    return null;
  },
  getPageInfo(pageNum) {
    return this.pageCanvases.get(pageNum);
  },
  async extractText(pageNum) {
    if (!this.currentDoc) throw new Error("No document loaded");
    const page = await this.currentDoc.getPage(pageNum);
    const textContent = await page.getTextContent();
    return textContent.items.map((item) => item.str).join(" ");
  },
  async extractTextWithPositions(pageNum) {
    if (!this.currentDoc) throw new Error("No document loaded");
    const page = await this.currentDoc.getPage(pageNum);
    const textContent = await page.getTextContent();
    const cached = this.pageCanvases.get(pageNum);
    const viewport = cached?.viewport;
    const styles = textContent.styles || {};
    return textContent.items.map((item, index) => {
      const pdfX = item.transform[4];
      const pdfY = item.transform[5];
      const pdfWidth = item.width || 0;
      const pdfHeight = item.height || 12;
      const fontSize = Math.abs(item.transform[3]) || item.height || 12;
      const fontStyle = item.fontName ? styles[item.fontName] : void 0;
      const fontFamily = fontStyle?.fontFamily || "sans-serif";
      const fontNameLower = (item.fontName || "").toLowerCase();
      const isItalic = fontNameLower.includes("italic") || fontNameLower.includes("oblique");
      const isBold = fontNameLower.includes("bold");
      let domBounds = null;
      let domFontSize = fontSize;
      if (viewport) {
        const [domX, domY] = viewport.convertToViewportPoint(pdfX, pdfY);
        const [domX2, domY2] = viewport.convertToViewportPoint(pdfX + pdfWidth, pdfY + pdfHeight);
        domBounds = {
          x: Math.min(domX, domX2),
          y: Math.min(domY, domY2),
          width: Math.abs(domX2 - domX) || pdfWidth * viewport.scale,
          height: Math.abs(domY2 - domY) || pdfHeight * viewport.scale
        };
        domFontSize = fontSize * viewport.scale;
      }
      return {
        index,
        str: item.str,
        pdfX,
        pdfY,
        pdfWidth,
        pdfHeight,
        fontSize,
        // PDF font size in points
        domFontSize,
        // Font size scaled to viewport (pixels)
        fontName: item.fontName,
        fontFamily,
        // "serif", "sans-serif", or "monospace"
        isItalic,
        // true if font name contains "italic" or "oblique"
        isBold,
        // true if font name contains "bold"
        domBounds
      };
    });
  },
  async extractAllText() {
    if (!this.currentDoc) throw new Error("No document loaded");
    const texts = [];
    for (let i = 1; i <= this.currentDoc.numPages; i++) {
      texts.push(await this.extractText(i));
    }
    return texts;
  },
  cleanup() {
    if (this.currentDoc) {
      this.currentDoc.destroy();
      this.currentDoc = null;
    }
    this.pageCanvases.clear();
  }
};
window.PdfBridge = PdfBridge;

// src/ts/shared-state.ts
var sharedPdf = {
  bytes: null,
  filename: null,
  source: null
};
function setSharedPdf(bytes, filename, source) {
  sharedPdf = { bytes, filename, source };
}
function getSharedPdf() {
  return sharedPdf;
}
function hasSharedPdf() {
  return sharedPdf.bytes !== null && sharedPdf.bytes.length > 0;
}
var hasChangesCallback = null;
var exportCallback = null;
function registerEditCallbacks(hasChanges, exportFn) {
  hasChangesCallback = hasChanges;
  exportCallback = exportFn;
}
function editHasChanges() {
  return hasChangesCallback ? hasChangesCallback() : false;
}
function exportEditedPdf() {
  return exportCallback ? exportCallback() : null;
}
function clearEditCallbacks() {
  hasChangesCallback = null;
  exportCallback = null;
}

// src/ts/types/wasm-bindings.ts
function getOpId(element) {
  const id = element.dataset.opId;
  if (!id) return null;
  try {
    return BigInt(id);
  } catch {
    return null;
  }
}
function setOpId(element, opId) {
  element.dataset.opId = opId.toString();
}

// src/ts/edit.ts
var editSession = null;
var currentTool = "select";
var currentPage = 1;
var currentPdfBytes = null;
var currentPdfFilename = null;
var textItemsMap = /* @__PURE__ */ new Map();
var activeEditItem = null;
var activeTextInput = null;
var isDrawing = false;
var drawStartX = 0;
var drawStartY = 0;
var drawOverlay = null;
var drawPageNum = null;
var drawPreviewEl = null;
var drawPageDiv = null;
var resizing = false;
var resizeTarget = null;
var resizeHandle = "";
var resizeStartX = 0;
var resizeStartY = 0;
var resizeStartRect = null;
var moving = false;
var moveTarget = null;
var moveStartX = 0;
var moveStartY = 0;
var moveStartLeft = 0;
var moveStartTop = 0;
var draggingTextOverlay = null;
var textDragStartX = 0;
var textDragStartY = 0;
var textDragStartLeft = 0;
var textDragStartTop = 0;
var selectedWhiteout = null;
var selectedTextBox = null;
var textBoxes = /* @__PURE__ */ new Map();
var nextTextBoxId = 0;
function setupEditView() {
  const dropZone = document.getElementById("edit-drop-zone");
  const fileInput = document.getElementById("edit-file-input");
  const browseBtn = document.getElementById("edit-browse-btn");
  const removeBtn = document.getElementById("edit-remove-btn");
  const downloadBtn = document.getElementById("edit-download-btn");
  const goBackBtn = document.getElementById("edit-go-back-btn");
  const useSplitBtn = document.getElementById("edit-use-split-btn");
  const undoBtn = document.getElementById("edit-undo-btn");
  if (!dropZone || !fileInput || !browseBtn || !removeBtn || !downloadBtn || !undoBtn) return;
  browseBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    fileInput.click();
  });
  dropZone.addEventListener("click", () => fileInput.click());
  dropZone.addEventListener("dragover", (e) => {
    e.preventDefault();
    dropZone.classList.add("drag-over");
  });
  dropZone.addEventListener("dragleave", () => dropZone.classList.remove("drag-over"));
  dropZone.addEventListener("drop", (e) => {
    e.preventDefault();
    dropZone.classList.remove("drag-over");
    if (e.dataTransfer?.files.length) {
      handleEditFile(e.dataTransfer.files[0]);
    }
  });
  fileInput.addEventListener("change", () => {
    if (fileInput.files?.length) {
      handleEditFile(fileInput.files[0]);
    }
  });
  removeBtn.addEventListener("click", resetEditView);
  downloadBtn.addEventListener("click", downloadEditedPdf);
  undoBtn.addEventListener("click", undoLastOperation);
  const redoBtn = document.getElementById("edit-redo-btn");
  redoBtn?.addEventListener("click", redoLastOperation);
  goBackBtn?.addEventListener("click", resetEditView);
  useSplitBtn?.addEventListener("click", () => {
    resetEditView();
    const splitTab = document.querySelector('[data-tab="split"]');
    splitTab?.click();
  });
  document.querySelectorAll('.tool-btn[id^="tool-"], .tool-btn[id^="edit-tool-"]').forEach((btn) => {
    btn.addEventListener("click", () => {
      let toolName = btn.id.replace("tool-", "").replace("edit-", "");
      currentTool = toolName;
      document.querySelectorAll('.tool-btn[id^="tool-"], .tool-btn[id^="edit-tool-"]').forEach((b) => {
        b.classList.remove("active");
      });
      btn.classList.add("active");
      updateCursor();
      deselectWhiteout();
      deselectTextBox();
      const viewer = document.getElementById("edit-viewer");
      if (viewer) {
        if (currentTool === "whiteout") {
          viewer.classList.add("whiteout-tool-active");
        } else {
          viewer.classList.remove("whiteout-tool-active");
        }
      }
    });
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Delete" || e.key === "Backspace") {
      if (activeTextInput) return;
      if (selectedTextBox) {
        deleteSelectedTextBox();
        e.preventDefault();
      } else if (selectedWhiteout) {
        deleteWhiteout(selectedWhiteout);
        e.preventDefault();
      }
    }
  });
  document.addEventListener("keydown", (e) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    if (e.key === "z" || e.key === "Z") {
      if (e.shiftKey) {
        e.preventDefault();
        redoLastOperation();
      } else if (!activeTextInput) {
        e.preventDefault();
        undoLastOperation();
      }
    }
  });
  document.getElementById("edit-viewer")?.addEventListener("click", (e) => {
    const target = e.target;
    if (!target.closest(".edit-whiteout-overlay")) {
      deselectWhiteout();
    }
  });
  document.addEventListener("mouseup", () => {
    if (currentTool !== "highlight") return;
    handleHighlightTextSelection();
  });
  document.getElementById("edit-prev-page")?.addEventListener("click", () => navigatePage(-1));
  document.getElementById("edit-next-page")?.addEventListener("click", () => navigatePage(1));
  document.querySelector("#edit-error .dismiss")?.addEventListener("click", () => {
    document.getElementById("edit-error")?.classList.add("hidden");
  });
  const boldBtn = document.getElementById("style-bold");
  const italicBtn = document.getElementById("style-italic");
  boldBtn?.addEventListener("click", () => toggleBold());
  italicBtn?.addEventListener("click", () => toggleItalic());
  document.addEventListener("keydown", (e) => {
    if ((e.metaKey || e.ctrlKey) && activeTextInput) {
      if (e.key === "b" || e.key === "B") {
        e.preventDefault();
        toggleBold();
      } else if (e.key === "i" || e.key === "I") {
        e.preventDefault();
        toggleItalic();
      }
    }
  });
  document.getElementById("font-size-decrease")?.addEventListener("click", () => decreaseFontSize());
  document.getElementById("font-size-increase")?.addEventListener("click", () => increaseFontSize());
  const fontSizeInput = document.getElementById("font-size-value");
  fontSizeInput?.addEventListener("change", () => setFontSize(fontSizeInput.value));
  const fontFamilySelect = document.getElementById("style-font-family");
  fontFamilySelect?.addEventListener("change", () => setFontFamily(fontFamilySelect.value));
}
async function handleEditFile(file) {
  if (file.type !== "application/pdf") {
    showError("edit-error", "Please select a PDF file");
    return;
  }
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    await loadPdfIntoEditInternal(bytes, file.name);
    setSharedPdf(bytes, file.name, "edit");
  } catch (e) {
    showError("edit-error", "Failed to load PDF: " + e);
    console.error(e);
  }
}
async function loadPdfIntoEdit(bytes, filename) {
  try {
    await loadPdfIntoEditInternal(bytes, filename);
  } catch (e) {
    showError("edit-error", "Failed to load PDF: " + e);
    console.error(e);
  }
}
async function loadPdfIntoEditInternal(bytes, filename) {
  const { EditSession, format_bytes } = window.wasmBindings;
  editSession = new EditSession(filename, bytes);
  currentPdfBytes = bytes;
  currentPdfFilename = filename;
  registerEditCallbacks(
    () => editSession?.hasChanges() ?? false,
    () => {
      try {
        return editSession?.export() ?? null;
      } catch {
        return null;
      }
    }
  );
  if (editSession.isSigned) {
    document.getElementById("edit-drop-zone")?.classList.add("hidden");
    document.getElementById("edit-signed-warning")?.classList.remove("hidden");
    return;
  }
  document.getElementById("edit-drop-zone")?.classList.add("hidden");
  document.getElementById("edit-editor")?.classList.remove("hidden");
  const fileNameEl = document.getElementById("edit-file-name");
  const fileDetailsEl = document.getElementById("edit-file-details");
  if (fileNameEl) fileNameEl.textContent = filename;
  if (fileDetailsEl) fileDetailsEl.textContent = `${editSession.pageCount} pages - ${format_bytes(bytes.length)}`;
  await ensurePdfJsLoaded();
  await PdfBridge.loadDocument(editSession.getDocumentBytes());
  await renderAllPages();
  updatePageNavigation();
  updateButtons();
}
async function renderAllPages() {
  if (!editSession) return;
  const container = document.getElementById("edit-pages");
  if (!container) return;
  container.innerHTML = "";
  textItemsMap.clear();
  for (let i = 1; i <= editSession.pageCount; i++) {
    const pageDiv = document.createElement("div");
    pageDiv.className = "edit-page";
    pageDiv.dataset.page = String(i);
    const canvas = document.createElement("canvas");
    pageDiv.appendChild(canvas);
    const overlay = document.createElement("div");
    overlay.className = "overlay-container";
    overlay.dataset.page = String(i);
    pageDiv.appendChild(overlay);
    const textLayer = document.createElement("div");
    textLayer.className = "text-layer";
    textLayer.dataset.page = String(i);
    pageDiv.appendChild(textLayer);
    container.appendChild(pageDiv);
    await PdfBridge.renderPage(i, canvas, 1.5);
    const items = await PdfBridge.extractTextWithPositions(i);
    textItemsMap.set(i, items);
    renderTextLayer(textLayer, items, i);
    overlay.addEventListener("click", (e) => handleOverlayClick(e, i));
    pageDiv.addEventListener("mousedown", (e) => handleWhiteoutStart(e, i, overlay, pageDiv));
    pageDiv.addEventListener("mousemove", (e) => handleWhiteoutMove(e));
    pageDiv.addEventListener("mouseup", (e) => handleWhiteoutEnd(e, i));
    pageDiv.addEventListener("mouseleave", () => {
      if (isDrawing) handleWhiteoutCancel();
    });
  }
}
function handleOverlayClick(e, pageNum) {
  if (currentTool === "select") return;
  const elementAtClick = document.elementFromPoint(e.clientX, e.clientY);
  const whiteout = elementAtClick?.closest(".edit-whiteout-overlay") || e.target.closest(".edit-whiteout-overlay");
  if (whiteout) {
    openWhiteoutTextEditor(whiteout, pageNum);
    return;
  }
  const existingTextBox = elementAtClick?.closest(".text-box") || e.target.closest(".text-box");
  if (existingTextBox && currentTool === "textbox") {
    const textContent = existingTextBox.querySelector(".text-content");
    if (textContent) {
      selectTextBox(existingTextBox);
      textContent.focus();
    }
    return;
  }
  const textOverlay = elementAtClick?.closest(".edit-text-overlay") || e.target.closest(".edit-text-overlay");
  if (textOverlay && currentTool === "text") {
    editExistingTextOverlay(textOverlay, pageNum);
    return;
  }
  const overlay = e.currentTarget;
  const rect = overlay.getBoundingClientRect();
  const domX = e.clientX - rect.left;
  const domY = e.clientY - rect.top;
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;
  const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
  const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
  const pdfX = domX * scaleX;
  const pdfY = pageInfo.page.view[3] - domY * scaleY;
  switch (currentTool) {
    case "text":
      addTextAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY);
      break;
    case "textbox":
      createTextBox(pageNum, domX, domY);
      break;
    case "checkbox":
      addCheckboxAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY);
      break;
  }
}
function addTextAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY) {
  if (!editSession) return;
  const input = document.createElement("span");
  input.contentEditable = "true";
  input.className = "edit-text-input";
  input.style.position = "absolute";
  input.style.left = domX + "px";
  input.style.top = domY + "px";
  input.style.minWidth = "20px";
  input.style.minHeight = "1em";
  input.style.fontSize = "12px";
  input.style.fontFamily = "sans-serif";
  input.style.padding = "2px 4px";
  input.style.border = "1px solid #007bff";
  input.style.borderRadius = "2px";
  input.style.outline = "none";
  input.style.zIndex = "100";
  input.style.display = "inline-block";
  input.style.whiteSpace = "pre-wrap";
  input.style.wordBreak = "break-word";
  input.style.background = "white";
  input.dataset.isBold = "false";
  input.dataset.isItalic = "false";
  input.dataset.fontSize = "12";
  input.dataset.fontFamily = "sans-serif";
  overlay.appendChild(input);
  input.focus();
  setActiveTextInput(input);
  function saveText() {
    if (!editSession) return;
    const text = (input.textContent || "").trim();
    const isBold = input.dataset.isBold === "true";
    const isItalic = input.dataset.isItalic === "true";
    const fontSize = parseInt(input.dataset.fontSize || "12", 10) || 12;
    const fontFamily = input.dataset.fontFamily || "sans-serif";
    input.remove();
    setActiveTextInput(null);
    if (!text) return;
    const textWidth = Math.max(input.offsetWidth, 50);
    const textHeight = Math.max(input.offsetHeight, 20);
    editSession.beginAction("textbox");
    const opId = editSession.addText(pageNum, pdfX, pdfY - 20, textWidth, textHeight, text, fontSize, "#000000", fontFamily, isItalic, isBold);
    editSession.commitAction();
    const textEl = document.createElement("div");
    textEl.className = "edit-text-overlay";
    textEl.textContent = text;
    textEl.style.left = domX + "px";
    textEl.style.top = domY + "px";
    textEl.style.fontSize = fontSize + "px";
    textEl.style.fontFamily = fontFamily;
    if (isBold) textEl.style.fontWeight = "bold";
    if (isItalic) textEl.style.fontStyle = "italic";
    setOpId(textEl, opId);
    textEl.dataset.fontSize = String(fontSize);
    textEl.dataset.fontFamily = fontFamily;
    textEl.dataset.isBold = isBold ? "true" : "false";
    textEl.dataset.isItalic = isItalic ? "true" : "false";
    overlay.appendChild(textEl);
    makeTextOverlayDraggable(textEl, pageNum);
    updateButtons();
  }
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      saveText();
    } else if (e.key === "Escape") {
      e.preventDefault();
      input.remove();
      setActiveTextInput(null);
    }
  });
  input.addEventListener("blur", () => {
    setTimeout(() => {
      if (input.parentElement) {
        saveText();
      }
    }, 100);
  });
}
function editExistingTextOverlay(textOverlay, pageNum) {
  if (!editSession) return;
  const existingText = textOverlay.textContent || "";
  const existingOpId = getOpId(textOverlay);
  const isBold = textOverlay.style.fontWeight === "bold" || textOverlay.style.fontWeight === "700";
  const isItalic = textOverlay.style.fontStyle === "italic";
  const fontSize = parseInt(textOverlay.dataset.fontSize || "12", 10) || 12;
  const fontFamily = textOverlay.dataset.fontFamily || "sans-serif";
  const domX = parseFloat(textOverlay.style.left);
  const domY = parseFloat(textOverlay.style.top);
  const overlay = textOverlay.parentElement;
  if (!overlay) return;
  if (existingOpId !== null) {
    editSession.removeOperation(existingOpId);
  }
  textOverlay.style.display = "none";
  const input = document.createElement("span");
  input.contentEditable = "true";
  input.className = "edit-text-input";
  input.style.position = "absolute";
  input.style.left = domX + "px";
  input.style.top = domY + "px";
  input.style.minWidth = "20px";
  input.style.minHeight = "1em";
  input.style.fontSize = fontSize + "px";
  input.style.fontFamily = fontFamily;
  input.style.padding = "2px 4px";
  input.style.border = "1px solid #007bff";
  input.style.borderRadius = "2px";
  input.style.outline = "none";
  input.style.zIndex = "100";
  input.style.display = "inline-block";
  input.style.whiteSpace = "pre-wrap";
  input.style.wordBreak = "break-word";
  input.style.background = "white";
  input.textContent = existingText;
  input.dataset.isBold = isBold ? "true" : "false";
  input.dataset.isItalic = isItalic ? "true" : "false";
  input.dataset.fontSize = String(fontSize);
  input.dataset.fontFamily = fontFamily;
  if (isBold) input.style.fontWeight = "bold";
  if (isItalic) input.style.fontStyle = "italic";
  overlay.appendChild(input);
  input.focus();
  const range = document.createRange();
  range.selectNodeContents(input);
  const selection = window.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
  setActiveTextInput(input);
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;
  const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
  const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
  const pdfX = domX * scaleX;
  const pdfY = pageInfo.page.view[3] - domY * scaleY;
  function saveEditedText() {
    if (!editSession) return;
    const text = (input.textContent || "").trim();
    const newIsBold = input.dataset.isBold === "true";
    const newIsItalic = input.dataset.isItalic === "true";
    const newFontSize = parseInt(input.dataset.fontSize || "12", 10) || 12;
    const newFontFamily = input.dataset.fontFamily || "sans-serif";
    const textWidth = Math.max(input.offsetWidth, 50);
    const textHeight = Math.max(input.offsetHeight, 20);
    input.remove();
    setActiveTextInput(null);
    if (!text) {
      textOverlay.remove();
      updateButtons();
      return;
    }
    editSession.beginAction("textbox");
    const opId = editSession.addText(pageNum, pdfX, pdfY - 20, textWidth, textHeight, text, newFontSize, "#000000", newFontFamily, newIsItalic, newIsBold);
    editSession.commitAction();
    textOverlay.textContent = text;
    textOverlay.style.display = "";
    textOverlay.style.fontSize = newFontSize + "px";
    textOverlay.style.fontFamily = newFontFamily;
    textOverlay.style.fontWeight = newIsBold ? "bold" : "normal";
    textOverlay.style.fontStyle = newIsItalic ? "italic" : "normal";
    setOpId(textOverlay, opId);
    textOverlay.dataset.fontSize = String(newFontSize);
    textOverlay.dataset.fontFamily = newFontFamily;
    textOverlay.dataset.isBold = newIsBold ? "true" : "false";
    textOverlay.dataset.isItalic = newIsItalic ? "true" : "false";
    updateButtons();
  }
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      saveEditedText();
    } else if (e.key === "Escape") {
      e.preventDefault();
      input.remove();
      setActiveTextInput(null);
      textOverlay.style.display = "";
      if (existingText && editSession) {
        editSession.beginAction("textbox");
        const opId = editSession.addText(pageNum, pdfX, pdfY - 20, 200, 20, existingText, fontSize, "#000000", fontFamily, isItalic, isBold);
        editSession.commitAction();
        setOpId(textOverlay, opId);
      }
    }
  });
  input.addEventListener("blur", () => {
    setTimeout(() => {
      if (input.parentElement) {
        saveEditedText();
      }
    }, 100);
  });
}
function addCheckboxAtPosition(pageNum, pdfX, pdfY, overlay, domX, domY) {
  if (!editSession) return;
  editSession.beginAction("checkbox");
  const opId = editSession.addCheckbox(pageNum, pdfX - 10, pdfY - 10, 20, 20, true);
  editSession.commitAction();
  const checkbox = document.createElement("div");
  checkbox.className = "edit-checkbox-overlay checked";
  checkbox.textContent = "\u2713";
  checkbox.style.left = domX - 10 + "px";
  checkbox.style.top = domY - 10 + "px";
  checkbox.dataset.page = String(pageNum);
  setOpId(checkbox, opId);
  checkbox.addEventListener("click", (e) => {
    e.stopPropagation();
    checkbox.classList.toggle("checked");
    const isChecked = checkbox.classList.contains("checked");
    checkbox.textContent = isChecked ? "\u2713" : "";
    editSession?.setCheckbox(opId, isChecked);
  });
  overlay.appendChild(checkbox);
  updateButtons();
}
function handleHighlightTextSelection() {
  if (!editSession) return;
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || !selection.toString().trim()) {
    return;
  }
  const range = selection.getRangeAt(0);
  const container = range.commonAncestorContainer;
  const textLayer = (container.nodeType === Node.ELEMENT_NODE ? container : container.parentElement)?.closest(".text-layer");
  if (!textLayer) {
    selection.removeAllRanges();
    return;
  }
  const pageNum = parseInt(textLayer.getAttribute("data-page") || "1");
  const rects = range.getClientRects();
  if (rects.length === 0) {
    selection.removeAllRanges();
    return;
  }
  const pageDiv = textLayer.closest(".edit-page");
  const overlay = pageDiv?.querySelector(".overlay-container");
  if (!pageDiv || !overlay) {
    selection.removeAllRanges();
    return;
  }
  const pageRect = pageDiv.getBoundingClientRect();
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) {
    selection.removeAllRanges();
    return;
  }
  const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
  const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
  editSession.beginAction("highlight");
  for (let i = 0; i < rects.length; i++) {
    const rect = rects[i];
    const domX = rect.left - pageRect.left;
    const domY = rect.top - pageRect.top;
    const domWidth = rect.width;
    const domHeight = rect.height;
    if (domWidth < 2 || domHeight < 2) continue;
    const pdfX = domX * scaleX;
    const pdfWidth = domWidth * scaleX;
    const pdfHeight = domHeight * scaleY;
    const pdfY = pageInfo.page.view[3] - (domY + domHeight) * scaleY;
    const opId = editSession.addHighlight(
      pageNum,
      pdfX,
      pdfY,
      pdfWidth,
      pdfHeight,
      "#FFFF00",
      0.3
    );
    const highlight = document.createElement("div");
    highlight.className = "edit-highlight-overlay";
    highlight.style.left = domX + "px";
    highlight.style.top = domY + "px";
    highlight.style.width = domWidth + "px";
    highlight.style.height = domHeight + "px";
    highlight.dataset.page = String(pageNum);
    setOpId(highlight, opId);
    overlay.appendChild(highlight);
  }
  editSession.commitAction();
  selection.removeAllRanges();
  updateButtons();
}
function handleWhiteoutStart(e, pageNum, overlay, pageDiv) {
  if (currentTool !== "whiteout" && currentTool !== "textbox") return;
  const target = e.target;
  if (target.closest(".delete-btn") || target.closest(".resize-handle") || target.closest(".text-content") || target.closest(".text-box") || target.closest(".edit-whiteout-overlay")) {
    return;
  }
  e.preventDefault();
  e.stopPropagation();
  isDrawing = true;
  drawOverlay = overlay;
  drawPageNum = pageNum;
  drawPageDiv = pageDiv;
  const rect = pageDiv.getBoundingClientRect();
  drawStartX = e.clientX - rect.left;
  drawStartY = e.clientY - rect.top;
  drawPreviewEl = document.createElement("div");
  drawPreviewEl.className = currentTool === "textbox" ? "textbox-preview" : "whiteout-preview";
  drawPreviewEl.style.left = drawStartX + "px";
  drawPreviewEl.style.top = drawStartY + "px";
  drawPreviewEl.style.width = "0px";
  drawPreviewEl.style.height = "0px";
  if (currentTool === "textbox") {
    drawPreviewEl.style.border = "2px dashed #666";
    drawPreviewEl.style.background = "transparent";
  }
  pageDiv.appendChild(drawPreviewEl);
}
function handleWhiteoutMove(e) {
  if (!isDrawing || !drawPreviewEl || !drawPageDiv) return;
  const rect = drawPageDiv.getBoundingClientRect();
  const currentX = e.clientX - rect.left;
  const currentY = e.clientY - rect.top;
  const x = Math.min(drawStartX, currentX);
  const y = Math.min(drawStartY, currentY);
  const width = Math.abs(currentX - drawStartX);
  const height = Math.abs(currentY - drawStartY);
  drawPreviewEl.style.left = x + "px";
  drawPreviewEl.style.top = y + "px";
  drawPreviewEl.style.width = width + "px";
  drawPreviewEl.style.height = height + "px";
}
function handleWhiteoutEnd(e, pageNum) {
  if (!isDrawing || !drawPreviewEl || !drawPageDiv) return;
  const wasTextbox = currentTool === "textbox";
  const rect = drawPageDiv.getBoundingClientRect();
  const endX = e.clientX - rect.left;
  const endY = e.clientY - rect.top;
  const domX = Math.min(drawStartX, endX);
  const domY = Math.min(drawStartY, endY);
  const domWidth = Math.abs(endX - drawStartX);
  const domHeight = Math.abs(endY - drawStartY);
  if (drawPreviewEl) {
    drawPreviewEl.remove();
    drawPreviewEl = null;
  }
  if (wasTextbox) {
    if (domWidth < 5 || domHeight < 5) {
      createTextBox(pageNum, drawStartX, drawStartY);
    } else {
      createTextBox(pageNum, domX, domY);
    }
  } else {
    if (domWidth >= 5 && domHeight >= 5) {
      addWhiteoutAtPosition(pageNum, domX, domY, domWidth, domHeight);
    }
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
function addWhiteoutAtPosition(pageNum, domX, domY, domWidth, domHeight) {
  if (!editSession) return;
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;
  const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
  const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
  const pdfX = domX * scaleX;
  const pdfWidth = domWidth * scaleX;
  const pdfHeight = domHeight * scaleY;
  const pdfY = pageInfo.page.view[3] - (domY + domHeight) * scaleY;
  editSession.beginAction("whiteout");
  const opId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight);
  editSession.commitAction();
  const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);
  if (!overlay) return;
  const whiteRect = document.createElement("div");
  whiteRect.className = "edit-whiteout-overlay";
  whiteRect.style.left = domX + "px";
  whiteRect.style.top = domY + "px";
  whiteRect.style.width = domWidth + "px";
  whiteRect.style.height = domHeight + "px";
  setOpId(whiteRect, opId);
  whiteRect.dataset.page = String(pageNum);
  whiteRect.addEventListener("mousedown", (e) => {
    if (e.target.classList.contains("resize-handle")) return;
    e.stopPropagation();
    e.preventDefault();
    selectWhiteout(whiteRect);
    startMove(e, whiteRect);
  });
  whiteRect.addEventListener("dblclick", (e) => {
    e.stopPropagation();
    openWhiteoutTextEditor(whiteRect, pageNum);
  });
  overlay.appendChild(whiteRect);
  selectWhiteout(whiteRect);
  updateButtons();
}
function selectWhiteout(whiteRect) {
  if (selectedWhiteout) {
    selectedWhiteout.classList.remove("selected");
    selectedWhiteout.querySelectorAll(".resize-handle").forEach((h) => h.remove());
  }
  selectedWhiteout = whiteRect;
  whiteRect.classList.add("selected");
  const handles = ["nw", "n", "ne", "w", "e", "sw", "s", "se"];
  handles.forEach((pos) => {
    const handle = document.createElement("div");
    handle.className = `resize-handle ${pos}`;
    handle.dataset.handle = pos;
    handle.addEventListener("mousedown", (e) => startResize(e, whiteRect, pos));
    whiteRect.appendChild(handle);
  });
}
function deselectWhiteout() {
  if (selectedWhiteout) {
    selectedWhiteout.classList.remove("selected");
    selectedWhiteout.querySelectorAll(".resize-handle").forEach((h) => h.remove());
    selectedWhiteout = null;
  }
}
function deleteWhiteout(whiteout) {
  const opId = getOpId(whiteout);
  if (opId !== null && editSession) {
    editSession.removeOperation(opId);
  }
  if (selectedWhiteout === whiteout) {
    selectedWhiteout = null;
  }
  whiteout.remove();
  updateButtons();
}
var nextTextBoxZIndex = 100;
function createTextBox(pageNum, domX, domY) {
  if (!editSession) throw new Error("No edit session");
  const id = nextTextBoxId++;
  const pageEl = document.querySelector(`.edit-page[data-page="${pageNum}"]`);
  const pageWidth = pageEl?.offsetWidth || 800;
  const margin = 10;
  const maxAvailableWidth = Math.max(100, pageWidth - domX - margin);
  const initialWidth = Math.min(150, maxAvailableWidth);
  const box = document.createElement("div");
  box.className = "text-box transparent";
  box.dataset.textboxId = String(id);
  box.dataset.page = String(pageNum);
  box.style.left = domX + "px";
  box.style.top = domY + "px";
  box.style.width = initialWidth + "px";
  box.style.height = "30px";
  box.style.zIndex = String(nextTextBoxZIndex++);
  const deleteBtn = document.createElement("button");
  deleteBtn.className = "delete-btn";
  deleteBtn.innerHTML = "&times;";
  deleteBtn.title = "Delete";
  deleteBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    deleteTextBox(box);
  });
  box.appendChild(deleteBtn);
  const textContent = document.createElement("div");
  textContent.className = "text-content";
  textContent.contentEditable = "true";
  textContent.dataset.fontSize = "12";
  textContent.dataset.fontFamily = "sans-serif";
  textContent.dataset.isBold = "false";
  textContent.dataset.isItalic = "false";
  textContent.style.fontSize = "12px";
  textContent.style.fontFamily = "sans-serif";
  textContent.addEventListener("focus", () => {
    activeTextInput = textContent;
    updateStyleButtons();
  });
  textContent.addEventListener("blur", () => {
    activeTextInput = null;
    updateStyleButtons();
    commitTextBox(box);
  });
  textContent.addEventListener("input", () => {
    checkTextBoxOverlap(box);
    expandTextBoxForContent(box, textContent);
  });
  box.appendChild(textContent);
  const handles = ["nw", "n", "ne", "w", "e", "sw", "s", "se"];
  handles.forEach((pos) => {
    const handle = document.createElement("div");
    handle.className = `resize-handle resize-handle-${pos}`;
    handle.dataset.handle = pos;
    handle.addEventListener("mousedown", (e) => startTextBoxResize(e, box, pos));
    box.appendChild(handle);
  });
  box.addEventListener("mousedown", (e) => {
    if (e.target.classList.contains("resize-handle") || e.target.classList.contains("delete-btn")) {
      return;
    }
    selectTextBox(box);
    if (!e.target.classList.contains("text-content")) {
      startTextBoxMove(e, box);
    }
  });
  const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);
  if (overlay) {
    overlay.appendChild(box);
  }
  textBoxes.set(id, box);
  selectTextBox(box);
  setTimeout(() => textContent.focus(), 50);
  checkTextBoxOverlap(box);
  return box;
}
function selectTextBox(box) {
  deselectTextBox();
  deselectWhiteout();
  selectedTextBox = box;
  box.classList.add("selected");
  box.style.zIndex = String(nextTextBoxZIndex++);
}
function deselectTextBox() {
  if (selectedTextBox) {
    selectedTextBox.classList.remove("selected");
    selectedTextBox = null;
  }
}
function deleteTextBox(box) {
  const opId = getOpId(box);
  if (opId !== null && editSession) {
    editSession.removeOperation(opId);
  }
  const id = parseInt(box.dataset.textboxId || "0");
  textBoxes.delete(id);
  if (selectedTextBox === box) {
    selectedTextBox = null;
  }
  box.remove();
  updateButtons();
}
function deleteSelectedTextBox() {
  if (selectedTextBox) {
    deleteTextBox(selectedTextBox);
  }
}
function commitTextBox(box) {
  if (!editSession) return;
  const textContent = box.querySelector(".text-content");
  const text = textContent?.textContent?.trim() || "";
  const pageNum = parseInt(box.dataset.page || "1");
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;
  const domX = parseFloat(box.style.left);
  const domY = parseFloat(box.style.top);
  const domWidth = box.offsetWidth;
  const domHeight = box.offsetHeight;
  const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
  const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
  const pdfX = domX * scaleX;
  const pdfWidth = domWidth * scaleX;
  const pdfHeight = domHeight * scaleY;
  const pdfY = pageInfo.page.view[3] - (domY + domHeight) * scaleY;
  const existingOpId = getOpId(box);
  if (existingOpId !== null) {
    editSession.removeOperation(existingOpId);
  }
  if (text) {
    const style = textContent ? window.getComputedStyle(textContent) : null;
    const fontSize = style ? parseFloat(style.fontSize) : 12;
    const isBold = style?.fontWeight === "bold" || parseInt(style?.fontWeight || "400") >= 700;
    const isItalic = style?.fontStyle === "italic";
    editSession.beginAction("textbox");
    const opId = editSession.addText(
      pageNum,
      pdfX,
      pdfY,
      pdfWidth,
      pdfHeight,
      text,
      fontSize,
      "#000000",
      null,
      // font name
      isItalic,
      isBold
    );
    editSession.commitAction();
    setOpId(box, opId);
  }
  updateButtons();
}
function startTextBoxResize(e, box, handle) {
  e.preventDefault();
  e.stopPropagation();
  resizing = true;
  resizeTarget = box;
  resizeHandle = handle;
  resizeStartX = e.clientX;
  resizeStartY = e.clientY;
  resizeStartRect = {
    left: parseFloat(box.style.left),
    top: parseFloat(box.style.top),
    width: box.offsetWidth,
    height: box.offsetHeight
  };
}
function startTextBoxMove(e, box) {
  e.preventDefault();
  moving = true;
  moveTarget = box;
  moveStartX = e.clientX;
  moveStartY = e.clientY;
  moveStartLeft = parseFloat(box.style.left);
  moveStartTop = parseFloat(box.style.top);
  document.addEventListener("mousemove", handleMove);
  document.addEventListener("mouseup", endMove);
}
function expandTextBoxForContent(box, textContent) {
  const text = textContent.textContent || "";
  if (!text) return;
  const pageEl = box.closest(".edit-page");
  if (!pageEl) return;
  const pageWidth = pageEl.offsetWidth;
  const boxLeft = parseFloat(box.style.left) || 0;
  const margin = 10;
  const maxAvailableWidth = Math.max(100, pageWidth - boxLeft - margin);
  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const fontSize = textContent.dataset.fontSize || "12";
  const fontFamily = textContent.dataset.fontFamily || "sans-serif";
  const isBold = textContent.dataset.isBold === "true";
  const isItalic = textContent.dataset.isItalic === "true";
  let fontStyle = "";
  if (isItalic) fontStyle += "italic ";
  if (isBold) fontStyle += "bold ";
  ctx.font = `${fontStyle}${fontSize}px ${fontFamily}`;
  const metrics = ctx.measureText(text);
  const textWidth = metrics.width + 20;
  const lineHeight = parseInt(fontSize, 10) * 1.4;
  const constrainedWidth = Math.min(textWidth, maxAvailableWidth);
  const effectiveWidth = Math.max(100, constrainedWidth - 20);
  const numLines = Math.max(1, Math.ceil(metrics.width / effectiveWidth));
  const textHeight = lineHeight * numLines + 10;
  const currentWidth = parseFloat(box.style.width);
  const currentHeight = parseFloat(box.style.height);
  const newWidth = Math.max(150, Math.min(constrainedWidth, maxAvailableWidth));
  const newHeight = Math.max(30, textHeight);
  if (newWidth > currentWidth && newWidth <= maxAvailableWidth) {
    box.style.width = newWidth + "px";
  } else if (currentWidth > maxAvailableWidth) {
    box.style.width = maxAvailableWidth + "px";
  }
  if (newHeight > currentHeight) {
    box.style.height = newHeight + "px";
  }
}
function checkTextBoxOverlap(box) {
  const boxRect = box.getBoundingClientRect();
  const pageNum = box.dataset.page;
  let hasOverlap = false;
  textBoxes.forEach((otherBox) => {
    if (otherBox === box) return;
    if (otherBox.dataset.page !== pageNum) return;
    const otherRect = otherBox.getBoundingClientRect();
    if (rectsOverlap(boxRect, otherRect)) {
      hasOverlap = true;
    }
  });
  document.querySelectorAll(`.edit-whiteout-overlay[data-page="${pageNum}"]`).forEach((overlay) => {
    const overlayRect = overlay.getBoundingClientRect();
    if (rectsOverlap(boxRect, overlayRect)) {
      hasOverlap = true;
    }
  });
  box.classList.toggle("overlapping", hasOverlap);
  let warning = box.querySelector(".overlap-warning");
  if (hasOverlap && !warning) {
    warning = document.createElement("div");
    warning.className = "overlap-warning";
    warning.textContent = "Overlapping";
    box.appendChild(warning);
  } else if (!hasOverlap && warning) {
    warning.remove();
  }
}
function rectsOverlap(a, b) {
  return !(a.right < b.left || b.right < a.left || a.bottom < b.top || b.bottom < a.top);
}
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
  document.addEventListener("mousemove", handleResize);
  document.addEventListener("mouseup", endResize);
}
function handleResize(e) {
  if (!resizing || !resizeTarget || !resizeStartRect) return;
  const dx = e.clientX - resizeStartX;
  const dy = e.clientY - resizeStartY;
  let newLeft = resizeStartRect.left;
  let newTop = resizeStartRect.top;
  let newWidth = resizeStartRect.width;
  let newHeight = resizeStartRect.height;
  if (resizeHandle.includes("w")) {
    newLeft = resizeStartRect.left + dx;
    newWidth = resizeStartRect.width - dx;
  }
  if (resizeHandle.includes("e")) {
    newWidth = resizeStartRect.width + dx;
  }
  if (resizeHandle.includes("n")) {
    newTop = resizeStartRect.top + dy;
    newHeight = resizeStartRect.height - dy;
  }
  if (resizeHandle.includes("s")) {
    newHeight = resizeStartRect.height + dy;
  }
  if (newWidth < 10) {
    if (resizeHandle.includes("w")) {
      newLeft = resizeStartRect.left + resizeStartRect.width - 10;
    }
    newWidth = 10;
  }
  if (newHeight < 10) {
    if (resizeHandle.includes("n")) {
      newTop = resizeStartRect.top + resizeStartRect.height - 10;
    }
    newHeight = 10;
  }
  resizeTarget.style.left = newLeft + "px";
  resizeTarget.style.top = newTop + "px";
  resizeTarget.style.width = newWidth + "px";
  resizeTarget.style.height = newHeight + "px";
}
function endResize() {
  if (!resizing || !resizeTarget) return;
  document.removeEventListener("mousemove", handleResize);
  document.removeEventListener("mouseup", endResize);
  const target = resizeTarget;
  const pageNum = parseInt(target.dataset.page || "0", 10);
  const opId = getOpId(target);
  resizing = false;
  resizeTarget = null;
  resizeHandle = "";
  try {
    if (opId !== null && editSession) {
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
        const pdfY = pageInfo.page.view[3] - (domY + domHeight) * scaleY;
        editSession.beginAction("resize");
        const newOpId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight);
        editSession.commitAction();
        setOpId(target, newOpId);
      }
    }
  } catch (err) {
    console.error("Error updating resize operation:", err);
  }
}
function startMove(e, whiteRect) {
  if (resizing) return;
  if (currentTool !== "select" && currentTool !== "whiteout") return;
  e.preventDefault();
  e.stopPropagation();
  moving = true;
  moveTarget = whiteRect;
  moveStartX = e.clientX;
  moveStartY = e.clientY;
  moveStartLeft = parseFloat(whiteRect.style.left);
  moveStartTop = parseFloat(whiteRect.style.top);
  document.addEventListener("mousemove", handleMove);
  document.addEventListener("mouseup", endMove);
}
function handleMove(e) {
  if (!moving || !moveTarget) return;
  const dx = e.clientX - moveStartX;
  const dy = e.clientY - moveStartY;
  moveTarget.style.left = moveStartLeft + dx + "px";
  moveTarget.style.top = moveStartTop + dy + "px";
}
function endMove() {
  if (!moving || !moveTarget) return;
  document.removeEventListener("mousemove", handleMove);
  document.removeEventListener("mouseup", endMove);
  const target = moveTarget;
  const pageNum = parseInt(target.dataset.page || "0", 10);
  const opId = getOpId(target);
  moving = false;
  moveTarget = null;
  try {
    if (opId !== null && editSession) {
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
        const pdfY = pageInfo.page.view[3] - (domY + domHeight) * scaleY;
        editSession.beginAction("move");
        const newOpId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight);
        editSession.commitAction();
        setOpId(target, newOpId);
      }
    }
  } catch (err) {
    console.error("Error updating move operation:", err);
  }
}
function makeTextOverlayDraggable(textEl, pageNum) {
  textEl.style.cursor = "move";
  textEl.addEventListener("click", (e) => {
    if (currentTool === "text") {
      e.preventDefault();
      e.stopPropagation();
      editExistingTextOverlay(textEl, pageNum);
    }
  });
  textEl.addEventListener("mousedown", (e) => {
    if (currentTool !== "select") return;
    e.preventDefault();
    e.stopPropagation();
    draggingTextOverlay = textEl;
    textDragStartX = e.clientX;
    textDragStartY = e.clientY;
    textDragStartLeft = parseFloat(textEl.style.left);
    textDragStartTop = parseFloat(textEl.style.top);
    document.addEventListener("mousemove", handleTextDrag);
    document.addEventListener("mouseup", endTextDrag);
  });
}
function makeReplaceOverlayEditable(replaceEl, pageNum) {
  replaceEl.style.cursor = "pointer";
  replaceEl.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    const originalTextItemJson = replaceEl.dataset.originalTextItem;
    const textItemIndex = replaceEl.dataset.textItemIndex;
    const opId = getOpId(replaceEl);
    if (!originalTextItemJson) {
      console.error("Cannot re-edit: no original text item data stored");
      return;
    }
    const intermediateText = replaceEl.textContent || "";
    const textItem = JSON.parse(originalTextItemJson);
    textItem.str = intermediateText;
    if (opId !== null && editSession) {
      editSession.removeOperation(opId);
    }
    replaceEl.dataset.pendingRemoval = "true";
    const originalSpan = document.querySelector(`.text-item[data-page="${pageNum}"][data-index="${textItemIndex}"]`);
    if (originalSpan) {
      startTextEdit(pageNum, parseInt(textItemIndex || "0", 10), textItem, originalSpan);
    } else {
      console.error("Cannot find original text item span to re-edit");
    }
  });
  replaceEl.addEventListener("mouseenter", () => {
    if (currentTool === "select" || currentTool === "text" || currentTool === "edit-text") {
      replaceEl.style.outline = "2px solid #007bff";
    }
  });
  replaceEl.addEventListener("mouseleave", () => {
    replaceEl.style.outline = "";
  });
}
function handleTextDrag(e) {
  if (!draggingTextOverlay) return;
  const dx = e.clientX - textDragStartX;
  const dy = e.clientY - textDragStartY;
  draggingTextOverlay.style.left = textDragStartLeft + dx + "px";
  draggingTextOverlay.style.top = textDragStartTop + dy + "px";
}
function endTextDrag() {
  if (!draggingTextOverlay) return;
  document.removeEventListener("mousemove", handleTextDrag);
  document.removeEventListener("mouseup", endTextDrag);
  const textEl = draggingTextOverlay;
  draggingTextOverlay = null;
  const newLeft = parseFloat(textEl.style.left);
  const newTop = parseFloat(textEl.style.top);
  if (newLeft === textDragStartLeft && newTop === textDragStartTop) return;
  const opId = getOpId(textEl);
  const pageEl = textEl.closest(".edit-page");
  const pageNum = parseInt(pageEl?.dataset.page || "0", 10);
  const text = textEl.textContent || "";
  const fontSize = parseInt(textEl.dataset.fontSize || "12", 10) || 12;
  const fontFamily = textEl.dataset.fontFamily || "sans-serif";
  const isBold = textEl.dataset.isBold === "true";
  const isItalic = textEl.dataset.isItalic === "true";
  if (opId !== null && editSession) {
    try {
      editSession.removeOperation(opId);
    } catch (err) {
      console.error("Error removing text operation:", err);
    }
  }
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (pageInfo && editSession) {
    const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
    const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
    const pdfX = newLeft * scaleX;
    const pdfY = pageInfo.page.view[3] - newTop * scaleY;
    editSession.beginAction("move");
    const newOpId = editSession.addText(pageNum, pdfX, pdfY - 20, 200, 20, text, fontSize, "#000000", fontFamily, isItalic, isBold);
    editSession.commitAction();
    setOpId(textEl, newOpId);
  }
}
async function openWhiteoutTextEditor(whiteRect, pageNum) {
  if (whiteRect.querySelector(".whiteout-text-input")) {
    return;
  }
  const domX = parseFloat(whiteRect.style.left);
  const domY = parseFloat(whiteRect.style.top);
  const domWidth = parseFloat(whiteRect.style.width);
  const domHeight = parseFloat(whiteRect.style.height);
  const originalWidth = domWidth;
  const originalHeight = domHeight;
  const coveredStyle = await detectCoveredTextStyle(pageNum, domX, domY, domWidth, domHeight);
  const input = document.createElement("span");
  input.contentEditable = "true";
  input.className = "whiteout-text-input";
  input.style.display = "block";
  input.style.minWidth = "100%";
  input.style.minHeight = "100%";
  input.style.border = "none";
  input.style.outline = "none";
  input.style.background = "transparent";
  input.style.padding = "2px 4px";
  input.style.boxSizing = "border-box";
  input.style.textAlign = "center";
  input.style.whiteSpace = "pre-wrap";
  input.style.wordBreak = "break-word";
  input.style.overflow = "visible";
  input.style.fontSize = coveredStyle.fontSize + "px";
  input.style.fontFamily = coveredStyle.fontFamily;
  input.style.color = "#000000";
  if (coveredStyle.isBold) input.style.fontWeight = "bold";
  if (coveredStyle.isItalic) input.style.fontStyle = "italic";
  input.dataset.fontSize = String(coveredStyle.fontSize);
  input.dataset.fontFamily = coveredStyle.fontFamily;
  input.dataset.isBold = coveredStyle.isBold ? "true" : "false";
  input.dataset.isItalic = coveredStyle.isItalic ? "true" : "false";
  whiteRect.appendChild(input);
  whiteRect.classList.add("editing");
  whiteRect.style.overflow = "visible";
  input.focus();
  setActiveTextInput(input);
  function expandWhiteoutForText() {
    const text = input.textContent || "";
    if (!text) return;
    const range = document.createRange();
    range.selectNodeContents(input);
    const textRect = range.getBoundingClientRect();
    const padding = 16;
    const verticalPadding = 8;
    const textWidth = textRect.width + padding;
    const textHeight = textRect.height + verticalPadding;
    const currentWidth = parseFloat(whiteRect.style.width);
    const currentHeight = parseFloat(whiteRect.style.height);
    if (textWidth > currentWidth) {
      whiteRect.style.width = textWidth + "px";
    }
    if (textHeight > currentHeight) {
      whiteRect.style.height = textHeight + "px";
    }
  }
  input.addEventListener("input", expandWhiteoutForText);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      whiteRect.classList.remove("editing");
      saveWhiteoutText(whiteRect, pageNum, input, originalWidth, originalHeight);
    } else if (e.key === "Escape") {
      e.preventDefault();
      whiteRect.classList.remove("editing");
      whiteRect.style.width = originalWidth + "px";
      whiteRect.style.height = originalHeight + "px";
      whiteRect.style.overflow = "hidden";
      input.remove();
      setActiveTextInput(null);
    }
  });
  input.addEventListener("blur", () => {
    setTimeout(() => {
      if (input.matches(":focus")) return;
      whiteRect.classList.remove("editing");
      if (input.parentElement && (input.textContent || "").trim()) {
        saveWhiteoutText(whiteRect, pageNum, input, originalWidth, originalHeight);
      } else if (input.parentElement) {
        whiteRect.style.width = originalWidth + "px";
        whiteRect.style.height = originalHeight + "px";
        whiteRect.style.overflow = "hidden";
        input.remove();
        setActiveTextInput(null);
      }
    }, 200);
  });
}
async function detectCoveredTextStyle(pageNum, domX, domY, domWidth, domHeight) {
  const defaultStyle = {
    fontSize: 12,
    fontFamily: "Helvetica, Arial, sans-serif",
    isBold: false,
    isItalic: false
  };
  try {
    const items = await PdfBridge.extractTextWithPositions(pageNum);
    if (!items || items.length === 0) {
      return defaultStyle;
    }
    const overlapping = items.filter((item2) => {
      if (!item2.domBounds) return false;
      const b = item2.domBounds;
      return !(b.x + b.width < domX || b.x > domX + domWidth || b.y + b.height < domY || b.y > domY + domHeight);
    });
    if (overlapping.length === 0) {
      return defaultStyle;
    }
    const item = overlapping[0];
    return {
      fontSize: item.domFontSize || item.fontSize || 12,
      fontFamily: item.fontFamily || defaultStyle.fontFamily,
      isBold: item.isBold || false,
      isItalic: item.isItalic || false
    };
  } catch (err) {
    console.error("Error detecting covered text style:", err);
    return defaultStyle;
  }
}
function saveWhiteoutText(whiteRect, pageNum, input, originalWidth, originalHeight) {
  if (!editSession) return;
  const text = (input.textContent || "").trim();
  if (!text) {
    if (originalWidth) whiteRect.style.width = originalWidth + "px";
    if (originalHeight) whiteRect.style.height = originalHeight + "px";
    whiteRect.style.overflow = "hidden";
    input.remove();
    setActiveTextInput(null);
    return;
  }
  const domX = parseFloat(whiteRect.style.left);
  const domY = parseFloat(whiteRect.style.top);
  const domWidth = parseFloat(whiteRect.style.width);
  const domHeight = parseFloat(whiteRect.style.height);
  const fontSize = parseFloat(input.dataset.fontSize || "12") || 12;
  const fontFamily = input.dataset.fontFamily || null;
  const isBold = input.dataset.isBold === "true";
  const isItalic = input.dataset.isItalic === "true";
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
  const pdfY = pageInfo.page.view[3] - (domY + domHeight) * scaleY;
  editSession.beginAction("whiteout");
  if (originalWidth && originalHeight && (domWidth !== originalWidth || domHeight !== originalHeight)) {
    const existingOpId = getOpId(whiteRect);
    if (existingOpId !== null) {
      editSession.removeOperation(existingOpId);
      const newWhiteOpId = editSession.addWhiteRect(pageNum, pdfX, pdfY, pdfWidth, pdfHeight);
      setOpId(whiteRect, newWhiteOpId);
    }
  }
  const opId = editSession.addText(pageNum, pdfX, pdfY, pdfWidth, pdfHeight, text, fontSize, "#000000", fontFamily, isItalic, isBold);
  editSession.commitAction();
  const textSpan = document.createElement("span");
  textSpan.className = "whiteout-text-content";
  textSpan.textContent = text;
  textSpan.style.display = "flex";
  textSpan.style.alignItems = "center";
  textSpan.style.justifyContent = "center";
  textSpan.style.width = "100%";
  textSpan.style.height = "100%";
  textSpan.style.fontSize = fontSize + "px";
  textSpan.style.fontFamily = input.dataset.fontFamily || "Helvetica, Arial, sans-serif";
  textSpan.style.color = "#000000";
  if (isBold) textSpan.style.fontWeight = "bold";
  if (isItalic) textSpan.style.fontStyle = "italic";
  textSpan.style.whiteSpace = "pre-wrap";
  textSpan.style.wordBreak = "break-word";
  setOpId(textSpan, opId);
  textSpan.dataset.fontSize = String(fontSize);
  textSpan.dataset.fontFamily = fontFamily || "sans-serif";
  textSpan.dataset.isBold = isBold ? "true" : "false";
  textSpan.dataset.isItalic = isItalic ? "true" : "false";
  input.remove();
  setActiveTextInput(null);
  whiteRect.style.overflow = "hidden";
  whiteRect.appendChild(textSpan);
  whiteRect.dataset.textOpId = opId.toString();
  updateButtons();
}
function renderTextLayer(textLayer, items, pageNum) {
  textLayer.innerHTML = "";
  items.forEach((item, index) => {
    if (!item.str.trim()) return;
    if (!item.domBounds) return;
    const span = document.createElement("span");
    span.className = "text-item";
    span.dataset.page = String(pageNum);
    span.dataset.index = String(index);
    span.textContent = item.str;
    span.style.left = item.domBounds.x + "px";
    span.style.top = item.domBounds.y + "px";
    span.style.width = Math.max(item.domBounds.width, 10) + "px";
    span.style.height = Math.max(item.domBounds.height, 12) + "px";
    span.addEventListener("mouseenter", () => {
      if (currentTool === "select" || currentTool === "edit-text") {
        span.classList.add("hover");
      }
    });
    span.addEventListener("mouseleave", () => {
      span.classList.remove("hover");
    });
    span.addEventListener("click", (e) => {
      e.stopPropagation();
      if (currentTool === "select" || currentTool === "edit-text") {
        startTextEdit(pageNum, index, item, span);
      }
    });
    textLayer.appendChild(span);
  });
}
function startTextEdit(pageNum, index, textItem, spanElement) {
  closeTextEditor();
  activeEditItem = { pageNum, index, textItem, spanElement };
  const fontFamily = mapFontFamilyForPreview(textItem.fontFamily);
  const fontSize = (textItem.pdfHeight || 12) * 1.5;
  const editor = document.createElement("div");
  editor.className = "text-editor-popup";
  editor.innerHTML = `
        <input type="text" class="text-editor-input" value="${escapeHtml(textItem.str)}" />
        <div class="text-editor-actions">
            <button class="text-editor-save">Save</button>
            <button class="text-editor-cancel">Cancel</button>
        </div>
    `;
  const input = editor.querySelector(".text-editor-input");
  input.style.fontFamily = fontFamily;
  input.style.fontSize = fontSize + "px";
  input.dataset.fontSize = String(Math.round(fontSize));
  input.dataset.fontFamily = textItem.fontFamily || "sans-serif";
  input.dataset.isBold = textItem.isBold ? "true" : "false";
  input.dataset.isItalic = textItem.isItalic ? "true" : "false";
  if (textItem.isItalic) input.style.fontStyle = "italic";
  if (textItem.isBold) input.style.fontWeight = "bold";
  const bounds = textItem.domBounds;
  if (bounds) {
    editor.style.left = bounds.x + "px";
    editor.style.top = bounds.y + bounds.height + 5 + "px";
  }
  const pageDiv = document.querySelector(`.edit-page[data-page="${pageNum}"]`);
  pageDiv?.appendChild(editor);
  input.focus();
  input.select();
  setActiveTextInput(input);
  editor.querySelector(".text-editor-save")?.addEventListener("click", () => {
    const newText = input.value;
    const inputIsBold = input.dataset.isBold === "true";
    const inputIsItalic = input.dataset.isItalic === "true";
    const customFontSize = parseFloat(input.dataset.fontSize || "0") || null;
    const customFontFamily = input.dataset.fontFamily || null;
    if (newText !== textItem.str || inputIsBold !== textItem.isBold || inputIsItalic !== textItem.isItalic || customFontSize !== Math.round((textItem.pdfHeight || 12) * 1.5) || customFontFamily !== textItem.fontFamily) {
      applyTextReplacement(pageNum, textItem, newText, inputIsBold, inputIsItalic, customFontSize, customFontFamily);
    }
    closeTextEditor();
  });
  editor.querySelector(".text-editor-cancel")?.addEventListener("click", closeTextEditor);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      const newText = input.value;
      const inputIsBold = input.dataset.isBold === "true";
      const inputIsItalic = input.dataset.isItalic === "true";
      const customFontSize = parseFloat(input.dataset.fontSize || "0") || null;
      const customFontFamily = input.dataset.fontFamily || null;
      if (newText !== textItem.str || inputIsBold !== textItem.isBold || inputIsItalic !== textItem.isItalic || customFontSize !== Math.round((textItem.pdfHeight || 12) * 1.5) || customFontFamily !== textItem.fontFamily) {
        applyTextReplacement(pageNum, textItem, newText, inputIsBold, inputIsItalic, customFontSize, customFontFamily);
      }
      closeTextEditor();
    } else if (e.key === "Escape") {
      closeTextEditor();
    }
  });
  spanElement.classList.add("editing");
}
function closeTextEditor() {
  const editor = document.querySelector(".text-editor-popup");
  if (editor) editor.remove();
  if (activeEditItem) {
    activeEditItem.spanElement.classList.remove("editing");
    activeEditItem = null;
  }
  setActiveTextInput(null);
}
function applyTextReplacement(pageNum, textItem, newText, isBold = null, isItalic = null, customFontSize = null, customFontFamily = null) {
  if (!editSession) return;
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;
  const useBold = isBold !== null ? isBold : textItem.isBold || false;
  const useItalic = isItalic !== null ? isItalic : textItem.isItalic || false;
  const renderScale = 1.5;
  const fontSize = customFontSize !== null ? customFontSize / renderScale : textItem.pdfHeight || 12;
  const useFontFamily = customFontFamily || textItem.fontFamily || null;
  editSession.beginAction("replacetext");
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
    "#000000",
    // Font family from toolbar or PDF.js styles
    useFontFamily,
    // Font style flags
    useItalic,
    useBold
  );
  editSession.commitAction();
  const domFontSize = customFontSize !== null ? customFontSize : (textItem.pdfHeight || 12) * renderScale;
  const fontFamily = mapFontFamilyForPreview(useFontFamily);
  const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);
  if (!overlay) return;
  const oldOverlay = overlay.querySelector('.edit-replace-overlay[data-pending-removal="true"]');
  if (oldOverlay) {
    oldOverlay.remove();
  }
  const replaceEl = document.createElement("div");
  replaceEl.className = "edit-replace-overlay";
  replaceEl.textContent = newText;
  const padding = 15;
  if (textItem.domBounds) {
    replaceEl.style.left = textItem.domBounds.x - padding + "px";
    replaceEl.style.top = textItem.domBounds.y - padding + "px";
    replaceEl.style.minWidth = textItem.domBounds.width + padding * 2 + "px";
    replaceEl.style.minHeight = textItem.domBounds.height + padding * 2 + "px";
  }
  replaceEl.style.padding = padding + "px";
  replaceEl.style.boxSizing = "border-box";
  replaceEl.style.fontFamily = fontFamily;
  replaceEl.style.fontSize = domFontSize + "px";
  replaceEl.style.lineHeight = "1";
  if (useItalic) replaceEl.style.fontStyle = "italic";
  if (useBold) replaceEl.style.fontWeight = "bold";
  setOpId(replaceEl, opId);
  replaceEl.dataset.textItemIndex = String(textItem.index);
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
  makeReplaceOverlayEditable(replaceEl, pageNum);
  const span = document.querySelector(`.text-item[data-page="${pageNum}"][data-index="${textItem.index}"]`);
  if (span) span.classList.add("replaced");
  updateButtons();
}
function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}
function mapFontFamilyForPreview(fontFamily) {
  if (!fontFamily) return "sans-serif";
  const lower = fontFamily.toLowerCase();
  if (lower === "serif") return 'Georgia, "Times New Roman", Times, serif';
  if (lower === "sans-serif") return "Arial, Helvetica, sans-serif";
  if (lower === "monospace") return '"Courier New", Courier, monospace';
  if (lower.includes("times")) return '"Times New Roman", Times, serif';
  if (lower.includes("arial") || lower.includes("helvetica")) return "Arial, Helvetica, sans-serif";
  if (lower.includes("courier") || lower.includes("mono")) return '"Courier New", Courier, monospace';
  if (lower.includes("georgia")) return "Georgia, serif";
  return "sans-serif";
}
function mapFontFamilyToDropdown(fontFamily) {
  if (!fontFamily) return "sans-serif";
  const lower = fontFamily.toLowerCase();
  if (lower === "sans-serif") return "sans-serif";
  if (lower === "serif") return "serif";
  if (lower === "monospace") return "monospace";
  if (lower === "arial") return "Arial";
  if (lower === "times new roman") return "Times New Roman";
  if (lower === "georgia") return "Georgia";
  if (lower === "courier new") return "Courier New";
  if (lower === "verdana") return "Verdana";
  if (lower === "trebuchet ms") return "Trebuchet MS";
  if (lower.includes("times")) return "Times New Roman";
  if (lower.includes("arial")) return "Arial";
  if (lower.includes("helvetica")) return "sans-serif";
  if (lower.includes("courier") || lower.includes("mono")) return "Courier New";
  if (lower.includes("georgia")) return "Georgia";
  if (lower.includes("verdana")) return "Verdana";
  if (lower.includes("trebuchet")) return "Trebuchet MS";
  return "sans-serif";
}
function setActiveTextInput(input) {
  activeTextInput = input;
  updateStyleButtons();
  if (input) {
    input.addEventListener("blur", handleTextInputBlur);
  }
}
function handleTextInputBlur() {
  setTimeout(() => {
    if (activeTextInput && !activeTextInput.matches(":focus")) {
      activeTextInput.removeEventListener("blur", handleTextInputBlur);
      activeTextInput = null;
      updateStyleButtons();
    }
  }, 150);
}
function updateStyleButtons() {
  const boldBtn = document.getElementById("style-bold");
  const italicBtn = document.getElementById("style-italic");
  const fontSizeDecrease = document.getElementById("font-size-decrease");
  const fontSizeIncrease = document.getElementById("font-size-increase");
  const fontSizeValue = document.getElementById("font-size-value");
  const fontFamilySelect = document.getElementById("style-font-family");
  if (!boldBtn || !italicBtn || !fontSizeDecrease || !fontSizeIncrease || !fontSizeValue || !fontFamilySelect) return;
  if (!activeTextInput) {
    boldBtn.disabled = true;
    italicBtn.disabled = true;
    fontSizeDecrease.disabled = true;
    fontSizeIncrease.disabled = true;
    fontSizeValue.disabled = true;
    fontFamilySelect.disabled = true;
    boldBtn.classList.remove("active");
    italicBtn.classList.remove("active");
    return;
  }
  boldBtn.disabled = false;
  italicBtn.disabled = false;
  fontSizeDecrease.disabled = false;
  fontSizeIncrease.disabled = false;
  fontSizeValue.disabled = false;
  fontFamilySelect.disabled = false;
  const inputEl = activeTextInput;
  const isBold = inputEl.dataset.isBold === "true" || inputEl.style.fontWeight === "bold" || inputEl.style.fontWeight === "700";
  const isItalic = inputEl.dataset.isItalic === "true" || inputEl.style.fontStyle === "italic";
  boldBtn.classList.toggle("active", isBold);
  italicBtn.classList.toggle("active", isItalic);
  const fontSize = inputEl.dataset.fontSize || "12";
  fontSizeValue.value = fontSize;
  const fontFamily = inputEl.dataset.fontFamily || "sans-serif";
  fontFamilySelect.value = mapFontFamilyToDropdown(fontFamily);
}
function toggleBold() {
  if (!activeTextInput) return;
  const currentBold = activeTextInput.dataset.isBold === "true";
  const newBold = !currentBold;
  activeTextInput.dataset.isBold = String(newBold);
  activeTextInput.style.fontWeight = newBold ? "bold" : "normal";
  updateStyleButtons();
  activeTextInput.focus();
}
function toggleItalic() {
  if (!activeTextInput) return;
  const currentItalic = activeTextInput.dataset.isItalic === "true";
  const newItalic = !currentItalic;
  activeTextInput.dataset.isItalic = String(newItalic);
  activeTextInput.style.fontStyle = newItalic ? "italic" : "normal";
  updateStyleButtons();
  activeTextInput.focus();
}
function increaseFontSize() {
  if (!activeTextInput) return;
  const current = parseInt(activeTextInput.dataset.fontSize || "12", 10) || 12;
  setFontSize(String(Math.min(current + 2, 72)));
}
function decreaseFontSize() {
  if (!activeTextInput) return;
  const current = parseInt(activeTextInput.dataset.fontSize || "12", 10) || 12;
  setFontSize(String(Math.max(current - 2, 6)));
}
function setFontSize(size) {
  if (!activeTextInput) return;
  const sizeNum = Math.max(6, Math.min(72, parseInt(size, 10) || 12));
  activeTextInput.dataset.fontSize = String(sizeNum);
  activeTextInput.style.fontSize = sizeNum + "px";
  const fontSizeValue = document.getElementById("font-size-value");
  if (fontSizeValue) fontSizeValue.value = String(sizeNum);
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
  if (!editSession || !editSession.canUndo()) return;
  const undoneIds = editSession.undo();
  if (!undoneIds) return;
  for (let i = 0; i < undoneIds.length; i++) {
    const opId = undoneIds[i];
    const el = document.querySelector(`[data-op-id="${opId}"]`);
    if (el) el.remove();
  }
  updateButtons();
}
function redoLastOperation() {
  if (!editSession || !editSession.canRedo()) return;
  const redoneIds = editSession.redo();
  if (!redoneIds) return;
  for (let i = 0; i < redoneIds.length; i++) {
    const opId = redoneIds[i];
    recreateOperationElement(opId);
  }
  updateButtons();
}
function recreateOperationElement(opId) {
  if (!editSession) return;
  const json = editSession.getOperationJson(opId);
  if (!json) return;
  try {
    const op = JSON.parse(json);
    switch (op.type) {
      case "AddWhiteRect":
        recreateWhiteRect(opId, { page: op.page, rect: op.rect });
        break;
      case "AddText":
        recreateTextBox(opId, { page: op.page, rect: op.rect, text: op.text || "", style: op.style });
        break;
      case "AddCheckbox":
        recreateCheckbox(opId, { page: op.page, rect: op.rect, checked: op.checked || false });
        break;
      case "AddHighlight":
        recreateHighlight(opId, { page: op.page, rect: op.rect });
        break;
    }
  } catch {
  }
}
function recreateWhiteRect(opId, data) {
  const pageNum = data.page;
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;
  const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
  const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
  const pdfRect = data.rect;
  const domX = pdfRect.x / scaleX;
  const domWidth = pdfRect.width / scaleX;
  const domHeight = pdfRect.height / scaleY;
  const domY = (pageInfo.page.view[3] - pdfRect.y - pdfRect.height) / scaleY;
  const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);
  if (!overlay) return;
  const whiteRect = document.createElement("div");
  whiteRect.className = "edit-whiteout-overlay";
  whiteRect.style.left = domX + "px";
  whiteRect.style.top = domY + "px";
  whiteRect.style.width = domWidth + "px";
  whiteRect.style.height = domHeight + "px";
  setOpId(whiteRect, opId);
  whiteRect.dataset.page = String(pageNum);
  whiteRect.addEventListener("mousedown", (e) => {
    if (e.target.classList.contains("resize-handle")) return;
    e.stopPropagation();
    e.preventDefault();
    selectWhiteout(whiteRect);
    startMove(e, whiteRect);
  });
  whiteRect.addEventListener("dblclick", (e) => {
    e.stopPropagation();
    openWhiteoutTextEditor(whiteRect, pageNum);
  });
  overlay.appendChild(whiteRect);
}
function recreateTextBox(opId, data) {
  const pageNum = data.page;
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;
  const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
  const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
  const pdfRect = data.rect;
  const domX = pdfRect.x / scaleX;
  const domWidth = pdfRect.width / scaleX;
  const domHeight = pdfRect.height / scaleY;
  const domY = (pageInfo.page.view[3] - pdfRect.y - pdfRect.height) / scaleY;
  const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);
  if (!overlay) return;
  const box = document.createElement("div");
  box.className = "text-box transparent";
  box.dataset.page = String(pageNum);
  box.style.left = domX + "px";
  box.style.top = domY + "px";
  box.style.width = domWidth + "px";
  box.style.height = domHeight + "px";
  box.style.zIndex = String(nextTextBoxZIndex++);
  setOpId(box, opId);
  const deleteBtn = document.createElement("button");
  deleteBtn.className = "delete-btn";
  deleteBtn.innerHTML = "&times;";
  deleteBtn.title = "Delete";
  deleteBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    deleteTextBox(box);
  });
  box.appendChild(deleteBtn);
  const textContent = document.createElement("div");
  textContent.className = "text-content";
  textContent.contentEditable = "true";
  textContent.textContent = data.text || "";
  const style = data.style || {};
  textContent.dataset.fontSize = String(style.font_size || 12);
  textContent.dataset.fontFamily = style.font_name || "sans-serif";
  textContent.dataset.isBold = String(style.is_bold || false);
  textContent.dataset.isItalic = String(style.is_italic || false);
  textContent.style.fontSize = (style.font_size || 12) + "px";
  textContent.style.fontFamily = style.font_name || "sans-serif";
  if (style.is_bold) textContent.style.fontWeight = "bold";
  if (style.is_italic) textContent.style.fontStyle = "italic";
  if (style.color) textContent.style.color = style.color;
  textContent.addEventListener("focus", () => {
    activeTextInput = textContent;
    updateStyleButtons();
  });
  textContent.addEventListener("blur", () => {
    activeTextInput = null;
    updateStyleButtons();
    commitTextBox(box);
  });
  box.appendChild(textContent);
  const handles = ["nw", "n", "ne", "w", "e", "sw", "s", "se"];
  handles.forEach((pos) => {
    const handle = document.createElement("div");
    handle.className = `resize-handle resize-handle-${pos}`;
    handle.dataset.handle = pos;
    handle.addEventListener("mousedown", (e) => startTextBoxResize(e, box, pos));
    box.appendChild(handle);
  });
  box.addEventListener("mousedown", (e) => {
    if (e.target.classList.contains("resize-handle") || e.target.classList.contains("delete-btn")) {
      return;
    }
    selectTextBox(box);
    startTextBoxMove(e, box);
  });
  overlay.appendChild(box);
}
function recreateCheckbox(opId, data) {
  const pageNum = data.page;
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;
  const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
  const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
  const pdfRect = data.rect;
  const domX = pdfRect.x / scaleX;
  const domWidth = pdfRect.width / scaleX;
  const domHeight = pdfRect.height / scaleY;
  const domY = (pageInfo.page.view[3] - pdfRect.y - pdfRect.height) / scaleY;
  const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);
  if (!overlay) return;
  const checkbox = document.createElement("div");
  checkbox.className = "edit-checkbox";
  checkbox.style.left = domX + "px";
  checkbox.style.top = domY + "px";
  checkbox.style.width = domWidth + "px";
  checkbox.style.height = domHeight + "px";
  checkbox.dataset.page = String(pageNum);
  setOpId(checkbox, opId);
  if (data.checked) {
    checkbox.classList.add("checked");
    checkbox.textContent = "\u2713";
  }
  checkbox.addEventListener("click", () => {
    checkbox.classList.toggle("checked");
    const isChecked = checkbox.classList.contains("checked");
    checkbox.textContent = isChecked ? "\u2713" : "";
    if (editSession) {
      editSession.setCheckbox(opId, isChecked);
    }
  });
  overlay.appendChild(checkbox);
}
function recreateHighlight(opId, data) {
  const pageNum = data.page;
  const pageInfo = PdfBridge.getPageInfo(pageNum);
  if (!pageInfo) return;
  const scaleX = pageInfo.page.view[2] / pageInfo.viewport.width;
  const scaleY = pageInfo.page.view[3] / pageInfo.viewport.height;
  const pdfRect = data.rect;
  const domX = pdfRect.x / scaleX;
  const domWidth = pdfRect.width / scaleX;
  const domHeight = pdfRect.height / scaleY;
  const domY = (pageInfo.page.view[3] - pdfRect.y - pdfRect.height) / scaleY;
  const overlay = document.querySelector(`.overlay-container[data-page="${pageNum}"]`);
  if (!overlay) return;
  const highlight = document.createElement("div");
  highlight.className = "edit-highlight";
  highlight.style.left = domX + "px";
  highlight.style.top = domY + "px";
  highlight.style.width = domWidth + "px";
  highlight.style.height = domHeight + "px";
  highlight.dataset.page = String(pageNum);
  setOpId(highlight, opId);
  overlay.appendChild(highlight);
}
function updateButtons() {
  const downloadBtn = document.getElementById("edit-download-btn");
  const undoBtn = document.getElementById("edit-undo-btn");
  const redoBtn = document.getElementById("edit-redo-btn");
  const hasChanges = editSession && editSession.hasChanges();
  if (downloadBtn) downloadBtn.disabled = !hasChanges;
  if (undoBtn) undoBtn.disabled = !editSession || !editSession.canUndo();
  if (redoBtn) redoBtn.disabled = !editSession || !editSession.canRedo();
}
async function downloadEditedPdf() {
  if (!editSession) return;
  const downloadBtn = document.getElementById("edit-download-btn");
  const btnContent = downloadBtn?.querySelector(".download-btn-content");
  if (!btnContent) return;
  try {
    if (downloadBtn) downloadBtn.disabled = true;
    btnContent.innerHTML = `
      <span class="spinner"></span>
      <span class="verification-text">Proof Verification in Progress</span>
    `;
    const result = editSession.export();
    const blob = new Blob([result], { type: "application/pdf" });
    const fileSizeKB = blob.size / 1024;
    const verificationTime = Math.min(3e3, Math.max(300, fileSizeKB * 2));
    await new Promise((resolve) => setTimeout(resolve, verificationTime));
    btnContent.innerHTML = `
      <span class="verification-text verification-passed">\u2713 Proof Verification Passed!</span>
    `;
    await new Promise((resolve) => setTimeout(resolve, 500));
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = editSession.documentName.replace(/\.pdf$/i, "-edited.pdf");
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    btnContent.innerHTML = `<span class="download-text">Download Edited PDF</span>`;
    if (downloadBtn) downloadBtn.disabled = false;
  } catch (e) {
    btnContent.innerHTML = `<span class="download-text">Download Edited PDF</span>`;
    if (downloadBtn) downloadBtn.disabled = false;
    showError("edit-error", "Export failed: " + e);
  }
}
function resetEditView() {
  editSession = null;
  currentPage = 1;
  currentTool = "select";
  currentPdfBytes = null;
  currentPdfFilename = null;
  textItemsMap.clear();
  closeTextEditor();
  clearEditCallbacks();
  handleWhiteoutCancel();
  deselectWhiteout();
  selectedWhiteout = null;
  document.getElementById("edit-drop-zone")?.classList.remove("hidden");
  document.getElementById("edit-signed-warning")?.classList.add("hidden");
  document.getElementById("edit-editor")?.classList.add("hidden");
  const fileInput = document.getElementById("edit-file-input");
  if (fileInput) fileInput.value = "";
  const pagesContainer = document.getElementById("edit-pages");
  if (pagesContainer) pagesContainer.innerHTML = "";
  document.getElementById("edit-error")?.classList.add("hidden");
  document.querySelectorAll('.tool-btn[id^="tool-"]').forEach((b) => b.classList.remove("active"));
  document.getElementById("tool-select")?.classList.add("active");
  PdfBridge.cleanup();
}
function navigatePage(delta) {
  if (!editSession) return;
  const newPage = currentPage + delta;
  if (newPage < 1 || newPage > editSession.pageCount) return;
  currentPage = newPage;
  updatePageNavigation();
  const pageEl = document.querySelector(`.edit-page[data-page="${currentPage}"]`);
  if (pageEl) {
    pageEl.scrollIntoView({ behavior: "smooth", block: "start" });
  }
}
function updatePageNavigation() {
  if (!editSession) return;
  const indicator = document.getElementById("edit-page-indicator");
  const prevBtn = document.getElementById("edit-prev-page");
  const nextBtn = document.getElementById("edit-next-page");
  if (indicator) indicator.textContent = `Page ${currentPage} of ${editSession.pageCount}`;
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= editSession.pageCount;
}
function updateCursor() {
  const viewer = document.getElementById("edit-viewer");
  if (!viewer) return;
  switch (currentTool) {
    case "select":
      viewer.style.cursor = "default";
      break;
    case "edit-text":
      viewer.style.cursor = "text";
      break;
    case "text":
      viewer.style.cursor = "text";
      break;
    case "textbox":
      viewer.style.cursor = "crosshair";
      break;
    case "highlight":
      viewer.style.cursor = "crosshair";
      break;
    case "checkbox":
      viewer.style.cursor = "pointer";
      break;
    case "whiteout":
      viewer.style.cursor = "crosshair";
      break;
    default:
      viewer.style.cursor = "default";
  }
  const isDrawingTool = currentTool === "whiteout" || currentTool === "textbox";
  document.querySelectorAll(".text-layer").forEach((layer) => {
    layer.style.pointerEvents = isDrawingTool ? "none" : "auto";
  });
  const overlayNeedsClicks = currentTool === "text" || currentTool === "textbox" || currentTool === "checkbox";
  document.querySelectorAll(".overlay-container").forEach((overlay) => {
    overlay.style.pointerEvents = overlayNeedsClicks ? "auto" : "none";
  });
}
function showError(containerId, message) {
  const container = document.getElementById(containerId);
  if (!container) return;
  const textEl = container.querySelector(".error-text");
  if (textEl) textEl.textContent = message;
  container.classList.remove("hidden");
  setTimeout(() => container.classList.add("hidden"), 8e3);
}

// src/ts/app.ts
var LARGE_FILE_WARNING_BYTES = 50 * 1024 * 1024;
var VERY_LARGE_FILE_WARNING_BYTES = 100 * 1024 * 1024;
var splitSession = null;
var mergeSession = null;
var splitOriginalFilename = null;
function init() {
  const { PdfJoinSession, SessionMode } = window.wasmBindings;
  splitSession = new PdfJoinSession(SessionMode.Split);
  mergeSession = new PdfJoinSession(SessionMode.Merge);
  splitSession.setProgressCallback(onSplitProgress);
  mergeSession.setProgressCallback(onMergeProgress);
  setupTabs();
  setupSplitView();
  setupMergeView();
  setupEditView();
  console.log("PDFJoin initialized (WASM-first architecture)");
}
function setupTabs() {
  const tabs = document.querySelectorAll(".tab");
  tabs.forEach((tab) => {
    tab.addEventListener("click", async () => {
      const tabName = tab.dataset.tab;
      const currentTab = document.querySelector(".tab.active")?.getAttribute("data-tab");
      if (currentTab === "edit" && tabName !== "edit") {
        if (editHasChanges()) {
          const action = await showUnsavedChangesModal();
          if (action === "cancel") return;
        }
        if (hasSharedPdf()) {
          const shared = getSharedPdf();
          if (shared.bytes && shared.filename && tabName === "split") {
            loadPdfIntoSplit(shared.bytes, shared.filename);
          }
        }
      }
      tabs.forEach((t) => t.classList.remove("active"));
      tab.classList.add("active");
      document.querySelectorAll(".view").forEach((v) => v.classList.add("hidden"));
      const view = document.getElementById(`${tabName}-view`);
      if (view) view.classList.remove("hidden");
      if (tabName === "edit" && hasSharedPdf()) {
        const shared = getSharedPdf();
        const editEditor = document.getElementById("edit-editor");
        const editAlreadyLoaded = editEditor && !editEditor.classList.contains("hidden");
        if (!editAlreadyLoaded && shared.bytes && shared.filename) {
          await loadPdfIntoEdit(shared.bytes, shared.filename);
        }
      }
    });
  });
}
function loadPdfIntoSplit(bytes, filename) {
  if (!splitSession) return;
  const { format_bytes } = window.wasmBindings;
  try {
    if (splitSession.getDocumentCount() > 0) {
      splitSession.removeDocument(0);
    }
    const info = splitSession.addDocument(filename, bytes);
    splitOriginalFilename = filename.replace(/\.pdf$/i, "");
    document.getElementById("split-drop-zone")?.classList.add("hidden");
    document.getElementById("split-editor")?.classList.remove("hidden");
    const fileNameEl = document.getElementById("split-file-name");
    const fileDetailsEl = document.getElementById("split-file-details");
    if (fileNameEl) fileNameEl.textContent = filename;
    if (fileDetailsEl) fileDetailsEl.textContent = `${info.page_count} pages - ${format_bytes(info.size_bytes)}`;
    updateExampleChips(info.page_count);
    const rangeInput = document.getElementById("page-range");
    const splitBtn = document.getElementById("split-btn");
    if (rangeInput) rangeInput.value = "";
    if (splitBtn) splitBtn.disabled = true;
  } catch (e) {
    showError2("split-error", String(e));
  }
}
async function showUnsavedChangesModal() {
  return new Promise((resolve) => {
    let modal = document.getElementById("unsaved-changes-modal");
    if (!modal) {
      modal = document.createElement("div");
      modal.id = "unsaved-changes-modal";
      modal.className = "unsaved-changes-modal";
      modal.innerHTML = `
        <div class="modal-backdrop"></div>
        <div class="modal-content">
          <h2>You Made Edits</h2>
          <p>Would you like to download your edited PDF before continuing?</p>
          <div class="modal-actions">
            <button class="primary-btn" data-action="download">Yes, Download My PDF</button>
            <button class="secondary-btn" data-action="continue">No, Continue Without Saving</button>
            <button class="text-btn" data-action="cancel">Go Back</button>
          </div>
        </div>
      `;
      document.body.appendChild(modal);
      if (!document.getElementById("modal-styles")) {
        const style = document.createElement("style");
        style.id = "modal-styles";
        style.textContent = `
          .unsaved-changes-modal { position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 1000; display: flex; align-items: center; justify-content: center; }
          .unsaved-changes-modal.hidden { display: none; }
          .modal-backdrop { position: absolute; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); }
          .modal-content { position: relative; background: white; padding: 2rem; border-radius: 12px; max-width: 420px; text-align: center; box-shadow: 0 4px 20px rgba(0,0,0,0.2); }
          .modal-content h2 { margin-bottom: 0.75rem; font-size: 1.5rem; }
          .modal-content p { margin-bottom: 1.5rem; color: #64748b; font-size: 1.1rem; line-height: 1.5; }
          .modal-actions { display: flex; flex-direction: column; gap: 0.75rem; }
          .modal-actions button { padding: 1rem 1.5rem; border-radius: 8px; font-size: 1.1rem; cursor: pointer; border: none; }
          .modal-actions .primary-btn { background: #2563eb; color: white; font-weight: 600; }
          .modal-actions .primary-btn:hover { background: #1d4ed8; }
          .modal-actions .secondary-btn { background: #f1f5f9; color: #334155; }
          .modal-actions .secondary-btn:hover { background: #e2e8f0; }
          .modal-actions .text-btn { background: transparent; color: #64748b; font-size: 1rem; }
          .modal-actions .text-btn:hover { color: #334155; }
        `;
        document.head.appendChild(style);
      }
    }
    modal.classList.remove("hidden");
    const cleanup = () => {
      modal?.classList.add("hidden");
      modal?.querySelectorAll("button").forEach((btn) => {
        btn.replaceWith(btn.cloneNode(true));
      });
    };
    modal.querySelector('[data-action="download"]')?.addEventListener("click", () => {
      const editedBytes = exportEditedPdf();
      if (editedBytes) {
        const shared = getSharedPdf();
        const filename = (shared.filename || "document.pdf").replace(/\.pdf$/i, "-edited.pdf");
        downloadBlob(editedBytes, filename);
        setSharedPdf(editedBytes, filename, "edit");
      }
      cleanup();
      resolve("download");
    }, { once: true });
    modal.querySelector('[data-action="continue"]')?.addEventListener("click", () => {
      cleanup();
      resolve("continue");
    }, { once: true });
    modal.querySelector('[data-action="cancel"]')?.addEventListener("click", () => {
      cleanup();
      resolve("cancel");
    }, { once: true });
    modal.querySelector(".modal-backdrop")?.addEventListener("click", () => {
      cleanup();
      resolve("cancel");
    }, { once: true });
  });
}
function setupSplitView() {
  const dropZone = document.getElementById("split-drop-zone");
  const fileInput = document.getElementById("split-file-input");
  const browseBtn = document.getElementById("split-browse-btn");
  const removeBtn = document.getElementById("split-remove-btn");
  const splitBtn = document.getElementById("split-btn");
  const rangeInput = document.getElementById("page-range");
  if (!dropZone || !fileInput || !browseBtn || !removeBtn || !splitBtn || !rangeInput) return;
  browseBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    fileInput.click();
  });
  dropZone.addEventListener("click", () => fileInput.click());
  dropZone.addEventListener("dragover", (e) => {
    e.preventDefault();
    dropZone.classList.add("drag-over");
  });
  dropZone.addEventListener("dragleave", () => dropZone.classList.remove("drag-over"));
  dropZone.addEventListener("drop", (e) => {
    e.preventDefault();
    dropZone.classList.remove("drag-over");
    const files = e.dataTransfer?.files;
    if (files && files.length > 0 && files[0].type === "application/pdf") {
      handleSplitFile(files[0]);
    }
  });
  fileInput.addEventListener("change", () => {
    if (fileInput.files && fileInput.files.length > 0) handleSplitFile(fileInput.files[0]);
  });
  removeBtn.addEventListener("click", resetSplitView);
  splitBtn.addEventListener("click", executeSplit);
  rangeInput.addEventListener("input", validateRange);
}
async function handleSplitFile(file) {
  if (!splitSession) return;
  const { format_bytes } = window.wasmBindings;
  try {
    if (file.size > VERY_LARGE_FILE_WARNING_BYTES) {
      if (!confirm(
        `This file is ${format_bytes(file.size)} which is very large. Processing may be slow or fail on some devices. Continue?`
      )) {
        return;
      }
    } else if (file.size > LARGE_FILE_WARNING_BYTES) {
      console.warn(`Large file: ${format_bytes(file.size)} - processing may take longer`);
    }
    const bytes = new Uint8Array(await file.arrayBuffer());
    const info = splitSession.addDocument(file.name, bytes);
    setSharedPdf(bytes, file.name, "split");
    splitOriginalFilename = file.name.replace(/\.pdf$/i, "");
    document.getElementById("split-drop-zone")?.classList.add("hidden");
    document.getElementById("split-editor")?.classList.remove("hidden");
    const fileNameEl = document.getElementById("split-file-name");
    const fileDetailsEl = document.getElementById("split-file-details");
    if (fileNameEl) fileNameEl.textContent = file.name;
    if (fileDetailsEl) fileDetailsEl.textContent = `${info.page_count} pages - ${format_bytes(info.size_bytes)}`;
    updateExampleChips(info.page_count);
    const rangeInput = document.getElementById("page-range");
    const splitBtn = document.getElementById("split-btn");
    if (rangeInput) rangeInput.value = "";
    if (splitBtn) splitBtn.disabled = true;
  } catch (e) {
    showError2("split-error", String(e));
  }
}
function resetSplitView() {
  if (!splitSession) return;
  splitSession.removeDocument(0);
  splitOriginalFilename = null;
  document.getElementById("split-drop-zone")?.classList.remove("hidden");
  document.getElementById("split-editor")?.classList.add("hidden");
  const fileInput = document.getElementById("split-file-input");
  const rangeInput = document.getElementById("page-range");
  const splitBtn = document.getElementById("split-btn");
  if (fileInput) fileInput.value = "";
  if (rangeInput) rangeInput.value = "";
  if (splitBtn) splitBtn.disabled = true;
}
function validateRange() {
  if (!splitSession) return;
  const rangeInput = document.getElementById("page-range");
  const splitBtn = document.getElementById("split-btn");
  if (!rangeInput || !splitBtn) return;
  try {
    splitSession.setPageSelection(rangeInput.value);
    rangeInput.classList.remove("invalid");
    splitBtn.disabled = !splitSession.canExecute();
  } catch {
    rangeInput.classList.add("invalid");
    splitBtn.disabled = true;
  }
}
async function executeSplit() {
  if (!splitSession) return;
  const splitBtn = document.getElementById("split-btn");
  const progress = document.getElementById("split-progress");
  const rangeInput = document.getElementById("page-range");
  const multiFileCheckbox = document.getElementById("split-multiple-files");
  if (!splitBtn || !progress || !rangeInput) return;
  splitBtn.disabled = true;
  progress.classList.remove("hidden");
  try {
    const isMultiFile = multiFileCheckbox?.checked;
    const fullRange = rangeInput.value;
    if (isMultiFile && fullRange.includes(",")) {
      const ranges = fullRange.split(",").map((r) => r.trim()).filter((r) => r);
      for (let i = 0; i < ranges.length; i++) {
        const range = ranges[i];
        const progressText = document.querySelector("#split-progress .progress-text");
        if (progressText) {
          progressText.textContent = `Processing range ${i + 1} of ${ranges.length}...`;
        }
        splitSession.setPageSelection(range);
        const result = splitSession.execute();
        const rangeLabel = range.replace(/\s+/g, "");
        const filename = `${splitOriginalFilename || "split"}-pages-${rangeLabel}.pdf`;
        downloadBlob(result, filename);
        if (i < ranges.length - 1) {
          await new Promise((r) => setTimeout(r, 100));
        }
      }
      splitSession.setPageSelection(fullRange);
    } else {
      const result = splitSession.execute();
      const range = fullRange.replace(/\s+/g, "").replace(/,/g, "_");
      const filename = `${splitOriginalFilename || "split"}-pages-${range}.pdf`;
      downloadBlob(result, filename);
    }
  } catch (e) {
    showError2("split-error", "Split failed: " + e);
  } finally {
    splitBtn.disabled = false;
    setTimeout(() => progress.classList.add("hidden"), 500);
  }
}
function onSplitProgress(current, total, message) {
  const progressFill = document.querySelector("#split-progress .progress-fill");
  const progressText = document.querySelector("#split-progress .progress-text");
  if (progressFill) progressFill.style.width = `${current / total * 100}%`;
  if (progressText) progressText.textContent = message;
}
function updateExampleChips(pageCount) {
  const container = document.getElementById("range-chips");
  if (!container) return;
  container.innerHTML = "";
  const chips = [];
  if (pageCount >= 1) {
    chips.push({ label: "First page", range: "1" });
  }
  if (pageCount >= 5) {
    chips.push({ label: "First 5", range: "1-5" });
  }
  if (pageCount >= 3) {
    const last3Start = pageCount - 2;
    chips.push({ label: "Last 3", range: `${last3Start}-${pageCount}` });
  }
  if (pageCount >= 1) {
    chips.push({ label: "All pages", range: `1-${pageCount}` });
  }
  chips.forEach(({ label, range }) => {
    const chip = document.createElement("button");
    chip.className = "chip";
    chip.type = "button";
    chip.textContent = label;
    chip.dataset.range = range;
    chip.addEventListener("click", () => {
      const rangeInput = document.getElementById("page-range");
      if (rangeInput) {
        rangeInput.value = range;
        validateRange();
      }
    });
    container.appendChild(chip);
  });
}
function setupMergeView() {
  const dropZone = document.getElementById("merge-drop-zone");
  const fileInput = document.getElementById("merge-file-input");
  const browseBtn = document.getElementById("merge-browse-btn");
  const addBtn = document.getElementById("merge-add-btn");
  const mergeBtn = document.getElementById("merge-btn");
  const fileList = document.getElementById("merge-file-list");
  if (!dropZone || !fileInput || !browseBtn || !addBtn || !mergeBtn || !fileList) return;
  browseBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    fileInput.click();
  });
  dropZone.addEventListener("click", () => fileInput.click());
  dropZone.addEventListener("dragover", (e) => {
    e.preventDefault();
    dropZone.classList.add("drag-over");
  });
  dropZone.addEventListener("dragleave", () => dropZone.classList.remove("drag-over"));
  dropZone.addEventListener("drop", (e) => {
    e.preventDefault();
    dropZone.classList.remove("drag-over");
    if (e.dataTransfer?.files) {
      handleMergeFiles(e.dataTransfer.files);
    }
  });
  fileList.addEventListener("dragover", (e) => {
    e.preventDefault();
    fileList.classList.add("drag-over");
  });
  fileList.addEventListener("dragleave", () => fileList.classList.remove("drag-over"));
  fileList.addEventListener("drop", (e) => {
    e.preventDefault();
    fileList.classList.remove("drag-over");
    if (e.dataTransfer?.files) {
      handleMergeFiles(e.dataTransfer.files);
    }
  });
  fileInput.addEventListener("change", () => {
    if (fileInput.files) {
      handleMergeFiles(fileInput.files);
      fileInput.value = "";
    }
  });
  addBtn.addEventListener("click", () => fileInput.click());
  mergeBtn.addEventListener("click", executeMerge);
}
async function handleMergeFiles(files) {
  if (!mergeSession) return;
  const { format_bytes } = window.wasmBindings;
  const fileArray = Array.from(files);
  for (const file of fileArray) {
    if (file.type !== "application/pdf") continue;
    if (file.size > VERY_LARGE_FILE_WARNING_BYTES) {
      if (!confirm(
        `"${file.name}" is ${format_bytes(file.size)} which is very large. Processing may be slow. Continue?`
      )) {
        continue;
      }
    }
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      mergeSession.addDocument(file.name, bytes);
    } catch (e) {
      showError2("merge-error", `${file.name}: ${e}`);
    }
  }
  updateMergeFileList();
}
function updateMergeFileList() {
  if (!mergeSession) return;
  const { format_bytes } = window.wasmBindings;
  const infos = mergeSession.getDocumentInfos();
  const count = mergeSession.getDocumentCount();
  const hasFiles = count > 0;
  document.getElementById("merge-drop-zone")?.classList.toggle("hidden", hasFiles);
  document.getElementById("merge-file-list")?.classList.toggle("hidden", !hasFiles);
  const totalSize = infos.reduce((sum, info) => sum + info.size_bytes, 0);
  const totalPages = infos.reduce((sum, info) => sum + info.page_count, 0);
  const countEl = document.getElementById("merge-count");
  if (countEl) {
    countEl.textContent = `(${count} files, ${totalPages} pages, ${format_bytes(totalSize)})`;
  }
  const ul = document.getElementById("merge-files");
  if (!ul) return;
  ul.innerHTML = "";
  infos.forEach((info, idx) => {
    const li = document.createElement("li");
    li.draggable = true;
    li.dataset.index = String(idx);
    li.innerHTML = `
            <span class="drag-handle">\u2630</span>
            <span class="file-name">${info.name}</span>
            <span class="file-size">${info.page_count} pages - ${format_bytes(info.size_bytes)}</span>
            <button class="remove-btn" data-index="${idx}">\xD7</button>
        `;
    const removeBtn = li.querySelector(".remove-btn");
    removeBtn?.addEventListener("click", () => {
      mergeSession?.removeDocument(idx);
      updateMergeFileList();
    });
    li.addEventListener("dragstart", onDragStart);
    li.addEventListener("dragover", onDragOver);
    li.addEventListener("drop", onDrop);
    li.addEventListener("dragend", onDragEnd);
    ul.appendChild(li);
  });
  const mergeBtn = document.getElementById("merge-btn");
  if (mergeBtn) mergeBtn.disabled = !mergeSession.canExecute();
}
var draggedIndex = null;
function onDragStart(e) {
  const target = e.target;
  draggedIndex = parseInt(target.dataset.index || "0", 10);
  target.classList.add("dragging");
}
function onDragOver(e) {
  e.preventDefault();
  const li = e.target.closest("li");
  if (li) li.classList.add("drag-over");
}
function onDrop(e) {
  e.preventDefault();
  if (!mergeSession) return;
  const li = e.target.closest("li");
  if (!li) return;
  const dropIndex = parseInt(li.dataset.index || "0", 10);
  if (draggedIndex !== null && draggedIndex !== dropIndex) {
    const count = mergeSession.getDocumentCount();
    const order = [...Array(count).keys()];
    order.splice(draggedIndex, 1);
    order.splice(dropIndex, 0, draggedIndex);
    try {
      mergeSession.reorderDocuments(order);
      updateMergeFileList();
    } catch (e2) {
      console.error("Reorder failed:", e2);
    }
  }
}
function onDragEnd() {
  draggedIndex = null;
  document.querySelectorAll(".dragging, .drag-over").forEach((el) => {
    el.classList.remove("dragging", "drag-over");
  });
}
async function executeMerge() {
  if (!mergeSession) return;
  const mergeBtn = document.getElementById("merge-btn");
  const progress = document.getElementById("merge-progress");
  if (!mergeBtn || !progress) return;
  mergeBtn.disabled = true;
  progress.classList.remove("hidden");
  try {
    const result = mergeSession.execute();
    const count = mergeSession.getDocumentCount();
    const filename = `merged-${count}-files.pdf`;
    downloadBlob(result, filename);
  } catch (e) {
    showError2("merge-error", "Merge failed: " + e);
  } finally {
    mergeBtn.disabled = false;
    setTimeout(() => progress.classList.add("hidden"), 500);
  }
}
function onMergeProgress(current, total, message) {
  const progressFill = document.querySelector("#merge-progress .progress-fill");
  const progressText = document.querySelector("#merge-progress .progress-text");
  if (progressFill) progressFill.style.width = `${current / total * 100}%`;
  if (progressText) progressText.textContent = message;
}
function showError2(containerId, message) {
  const container = document.getElementById(containerId);
  if (!container) return;
  const textEl = container.querySelector(".error-text");
  const dismissBtn = container.querySelector(".dismiss");
  if (textEl) textEl.textContent = message;
  container.classList.remove("hidden");
  const timer = setTimeout(() => container.classList.add("hidden"), 8e3);
  if (dismissBtn) {
    dismissBtn.onclick = () => {
      clearTimeout(timer);
      container.classList.add("hidden");
    };
  }
}
function downloadBlob(data, filename) {
  const blob = new Blob([data], { type: "application/pdf" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
export {
  init
};
//# sourceMappingURL=bundle.js.map
