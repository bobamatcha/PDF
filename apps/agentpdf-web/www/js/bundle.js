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
function isPdfJsLoaded() {
  return pdfJsLoaded;
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
        domFontSize,
        fontName: item.fontName,
        fontFamily,
        isItalic,
        isBold,
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

// src/ts/coord-utils.ts
function domRectToPdf(viewport, domX, domY, domWidth, domHeight) {
  const [pdfX1, pdfY1] = viewport.convertToPdfPoint(domX, domY);
  const [pdfX2, pdfY2] = viewport.convertToPdfPoint(domX + domWidth, domY + domHeight);
  return {
    x: Math.min(pdfX1, pdfX2),
    y: Math.min(pdfY1, pdfY2),
    width: Math.abs(pdfX2 - pdfX1),
    height: Math.abs(pdfY2 - pdfY1)
  };
}
function domPointToPdf(viewport, domX, domY) {
  return viewport.convertToPdfPoint(domX, domY);
}
function pdfRectToDom(viewport, pdfX, pdfY, pdfWidth, pdfHeight) {
  const pdfRect = [
    pdfX,
    pdfY,
    pdfX + pdfWidth,
    pdfY + pdfHeight
  ];
  const [domX1, domY1, domX2, domY2] = viewport.convertToViewportRectangle(pdfRect);
  return {
    x: Math.min(domX1, domX2),
    y: Math.min(domY1, domY2),
    width: Math.abs(domX2 - domX1),
    height: Math.abs(domY2 - domY1)
  };
}
function pdfPointToDom(viewport, pdfX, pdfY) {
  return viewport.convertToViewportPoint(pdfX, pdfY);
}
function clientRectToCanvasRelative(clientRect, canvasRect) {
  return {
    x: clientRect.left - canvasRect.left,
    y: clientRect.top - canvasRect.top,
    width: clientRect.width,
    height: clientRect.height
  };
}
function getPageRenderInfo(pageInfo, pageDiv) {
  if (!pageInfo) return null;
  const canvas = pageDiv?.querySelector("canvas");
  if (!canvas) return null;
  return {
    canvas,
    canvasRect: canvas.getBoundingClientRect(),
    viewport: pageInfo.viewport
  };
}

// src/ts/template-editor.ts
var FieldType = /* @__PURE__ */ ((FieldType2) => {
  FieldType2["Text"] = "text";
  FieldType2["Signature"] = "signature";
  FieldType2["Initials"] = "initials";
  FieldType2["Checkbox"] = "checkbox";
  FieldType2["Date"] = "date";
  return FieldType2;
})(FieldType || {});
var MIN_FIELD_WIDTH = 100;
var MIN_FIELD_HEIGHT = 44;
var DEFAULT_TEXT_WIDTH = 200;
var DEFAULT_TEXT_HEIGHT = 48;
var DEFAULT_SIGNATURE_WIDTH = 200;
var DEFAULT_SIGNATURE_HEIGHT = 60;
var DEFAULT_INITIALS_WIDTH = 80;
var DEFAULT_INITIALS_HEIGHT = 44;
var DEFAULT_CHECKBOX_SIZE = 24;
var DEFAULT_DATE_WIDTH = 150;
var DEFAULT_DATE_HEIGHT = 44;
var DEFAULT_STYLE = {
  fontSize: 14,
  fontFamily: "sans-serif",
  isBold: false,
  isItalic: false,
  color: "#000000"
};
var state = {
  currentTool: null,
  fields: /* @__PURE__ */ new Map(),
  selectedFieldId: null,
  currentStyle: { ...DEFAULT_STYLE },
  pdfBytes: null,
  pageCount: 0
};
function generateFieldId() {
  return `field-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}
function getDefaultDimensions(type) {
  switch (type) {
    case "text" /* Text */:
      return { width: DEFAULT_TEXT_WIDTH, height: DEFAULT_TEXT_HEIGHT };
    case "signature" /* Signature */:
      return { width: DEFAULT_SIGNATURE_WIDTH, height: DEFAULT_SIGNATURE_HEIGHT };
    case "initials" /* Initials */:
      return { width: DEFAULT_INITIALS_WIDTH, height: DEFAULT_INITIALS_HEIGHT };
    case "checkbox" /* Checkbox */:
      return { width: DEFAULT_CHECKBOX_SIZE, height: DEFAULT_CHECKBOX_SIZE };
    case "date" /* Date */:
      return { width: DEFAULT_DATE_WIDTH, height: DEFAULT_DATE_HEIGHT };
  }
}
function createFieldElement(field, pageContainer) {
  const el = document.createElement("div");
  el.id = field.id;
  el.className = `template-field template-field-${field.type}`;
  el.dataset.fieldId = field.id;
  el.dataset.fieldType = field.type;
  el.dataset.pageNum = field.pageNum.toString();
  el.style.position = "absolute";
  el.style.left = `${field.domX}px`;
  el.style.top = `${field.domY}px`;
  el.style.width = `${field.domWidth}px`;
  el.style.height = `${field.domHeight}px`;
  el.style.border = "2px solid #0066cc";
  el.style.borderRadius = "4px";
  el.style.backgroundColor = "rgba(255, 255, 255, 0.9)";
  el.style.cursor = "move";
  el.style.boxSizing = "border-box";
  switch (field.type) {
    case "text" /* Text */:
    case "date" /* Date */:
      createTextFieldContent(el, field);
      break;
    case "signature" /* Signature */:
      createSignatureFieldContent(el, field);
      break;
    case "initials" /* Initials */:
      createInitialsFieldContent(el, field);
      break;
    case "checkbox" /* Checkbox */:
      createCheckboxFieldContent(el, field);
      break;
  }
  addResizeHandles(el, field);
  addDeleteButton(el, field);
  const overlay = pageContainer.querySelector(".field-overlay");
  if (overlay) {
    overlay.appendChild(el);
  }
  return el;
}
function createTextFieldContent(el, field) {
  const input = document.createElement("input");
  input.type = field.type === "date" /* Date */ ? "date" : "text";
  input.className = "field-input";
  input.value = field.value;
  input.placeholder = field.type === "date" /* Date */ ? "Select date" : "Enter text...";
  input.style.fontSize = `${field.style.fontSize}px`;
  input.style.fontFamily = field.style.fontFamily;
  input.style.fontWeight = field.style.isBold ? "bold" : "normal";
  input.style.fontStyle = field.style.isItalic ? "italic" : "normal";
  input.style.color = field.style.color;
  input.style.border = "none";
  input.style.outline = "none";
  input.style.width = "100%";
  input.style.height = "100%";
  input.style.padding = "4px 8px";
  input.style.boxSizing = "border-box";
  input.style.backgroundColor = "transparent";
  input.addEventListener("input", () => {
    field.value = input.value;
  });
  input.addEventListener("focus", () => {
    selectField(field.id);
  });
  el.appendChild(input);
}
function createSignatureFieldContent(el, field) {
  el.style.backgroundColor = "rgba(255, 245, 230, 0.9)";
  el.style.borderColor = "#cc6600";
  const label = document.createElement("div");
  label.className = "field-label";
  label.textContent = "Signature";
  label.style.textAlign = "center";
  label.style.color = "#cc6600";
  label.style.fontSize = "12px";
  label.style.fontStyle = "italic";
  label.style.lineHeight = `${field.domHeight}px`;
  el.appendChild(label);
}
function createInitialsFieldContent(el, field) {
  el.style.backgroundColor = "rgba(230, 245, 255, 0.9)";
  el.style.borderColor = "#0099cc";
  const input = document.createElement("input");
  input.type = "text";
  input.className = "field-input";
  input.value = field.value;
  input.placeholder = "AB";
  input.maxLength = 4;
  input.style.fontSize = `${field.style.fontSize}px`;
  input.style.fontFamily = field.style.fontFamily;
  input.style.fontWeight = "bold";
  input.style.textAlign = "center";
  input.style.border = "none";
  input.style.outline = "none";
  input.style.width = "100%";
  input.style.height = "100%";
  input.style.padding = "4px";
  input.style.boxSizing = "border-box";
  input.style.backgroundColor = "transparent";
  input.addEventListener("input", () => {
    field.value = input.value.toUpperCase();
    input.value = field.value;
  });
  input.addEventListener("focus", () => {
    selectField(field.id);
  });
  el.appendChild(input);
}
function createCheckboxFieldContent(el, field) {
  el.style.backgroundColor = "rgba(240, 255, 240, 0.9)";
  el.style.borderColor = "#00cc66";
  el.style.display = "flex";
  el.style.alignItems = "center";
  el.style.justifyContent = "center";
  const checkbox = document.createElement("input");
  checkbox.type = "checkbox";
  checkbox.className = "field-checkbox";
  checkbox.checked = field.checked || false;
  checkbox.style.width = "18px";
  checkbox.style.height = "18px";
  checkbox.style.cursor = "pointer";
  checkbox.addEventListener("change", () => {
    field.checked = checkbox.checked;
    field.value = checkbox.checked ? "Yes" : "No";
  });
  checkbox.addEventListener("focus", () => {
    selectField(field.id);
  });
  el.appendChild(checkbox);
}
function addResizeHandles(el, field) {
  const handles = ["nw", "n", "ne", "w", "e", "sw", "s", "se"];
  handles.forEach((position) => {
    const handle = document.createElement("div");
    handle.className = `resize-handle resize-${position}`;
    handle.style.position = "absolute";
    handle.style.width = "8px";
    handle.style.height = "8px";
    handle.style.backgroundColor = "#0066cc";
    handle.style.borderRadius = "50%";
    handle.style.display = "none";
    switch (position) {
      case "nw":
        handle.style.top = "-4px";
        handle.style.left = "-4px";
        handle.style.cursor = "nw-resize";
        break;
      case "n":
        handle.style.top = "-4px";
        handle.style.left = "50%";
        handle.style.transform = "translateX(-50%)";
        handle.style.cursor = "n-resize";
        break;
      case "ne":
        handle.style.top = "-4px";
        handle.style.right = "-4px";
        handle.style.cursor = "ne-resize";
        break;
      case "w":
        handle.style.top = "50%";
        handle.style.left = "-4px";
        handle.style.transform = "translateY(-50%)";
        handle.style.cursor = "w-resize";
        break;
      case "e":
        handle.style.top = "50%";
        handle.style.right = "-4px";
        handle.style.transform = "translateY(-50%)";
        handle.style.cursor = "e-resize";
        break;
      case "sw":
        handle.style.bottom = "-4px";
        handle.style.left = "-4px";
        handle.style.cursor = "sw-resize";
        break;
      case "s":
        handle.style.bottom = "-4px";
        handle.style.left = "50%";
        handle.style.transform = "translateX(-50%)";
        handle.style.cursor = "s-resize";
        break;
      case "se":
        handle.style.bottom = "-4px";
        handle.style.right = "-4px";
        handle.style.cursor = "se-resize";
        break;
    }
    handle.addEventListener("mousedown", (e) => {
      e.stopPropagation();
      startResize(field.id, position, e);
    });
    el.appendChild(handle);
  });
}
function addDeleteButton(el, field) {
  const btn = document.createElement("button");
  btn.className = "field-delete-btn";
  btn.textContent = "\xD7";
  btn.style.position = "absolute";
  btn.style.top = "-12px";
  btn.style.right = "-12px";
  btn.style.width = "24px";
  btn.style.height = "24px";
  btn.style.borderRadius = "50%";
  btn.style.border = "none";
  btn.style.backgroundColor = "#cc0000";
  btn.style.color = "white";
  btn.style.cursor = "pointer";
  btn.style.fontSize = "16px";
  btn.style.lineHeight = "1";
  btn.style.display = "none";
  btn.addEventListener("click", (e) => {
    e.stopPropagation();
    deleteField(field.id);
  });
  el.appendChild(btn);
}
function placeField(type, pageNum, domX, domY, viewport) {
  const dimensions = getDefaultDimensions(type);
  const id = generateFieldId();
  const pdfRect = domRectToPdf(viewport, domX, domY, dimensions.width, dimensions.height);
  const field = {
    id,
    type,
    pageNum,
    domX,
    domY,
    domWidth: dimensions.width,
    domHeight: dimensions.height,
    pdfX: pdfRect.x,
    pdfY: pdfRect.y,
    pdfWidth: pdfRect.width,
    pdfHeight: pdfRect.height,
    value: "",
    style: { ...state.currentStyle },
    checked: type === "checkbox" /* Checkbox */ ? false : void 0
  };
  state.fields.set(id, field);
  const pageContainer = document.querySelector(`[data-page="${pageNum}"]`);
  if (pageContainer) {
    createFieldElement(field, pageContainer);
  }
  selectField(id);
  return field;
}
function selectField(id) {
  if (state.selectedFieldId) {
    const prevEl = document.getElementById(state.selectedFieldId);
    if (prevEl) {
      prevEl.classList.remove("selected");
      prevEl.querySelectorAll(".resize-handle, .field-delete-btn").forEach((h) => {
        h.style.display = "none";
      });
    }
  }
  state.selectedFieldId = id;
  const el = document.getElementById(id);
  if (el) {
    el.classList.add("selected");
    el.querySelectorAll(".resize-handle, .field-delete-btn").forEach((h) => {
      h.style.display = "block";
    });
  }
}
function deleteField(id) {
  const el = document.getElementById(id);
  if (el) {
    el.remove();
  }
  state.fields.delete(id);
  if (state.selectedFieldId === id) {
    state.selectedFieldId = null;
  }
}
function updateFieldStyle(id, style) {
  const field = state.fields.get(id);
  if (!field) return;
  Object.assign(field.style, style);
  const el = document.getElementById(id);
  if (!el) return;
  const input = el.querySelector("input");
  if (input) {
    if (style.fontSize !== void 0) input.style.fontSize = `${style.fontSize}px`;
    if (style.fontFamily !== void 0) input.style.fontFamily = style.fontFamily;
    if (style.isBold !== void 0) input.style.fontWeight = style.isBold ? "bold" : "normal";
    if (style.isItalic !== void 0) input.style.fontStyle = style.isItalic ? "italic" : "normal";
    if (style.color !== void 0) input.style.color = style.color;
  }
}
var dragState = null;
var resizeState = null;
function startDrag(id, e) {
  const field = state.fields.get(id);
  if (!field) return;
  dragState = {
    fieldId: id,
    startX: e.clientX,
    startY: e.clientY,
    startFieldX: field.domX,
    startFieldY: field.domY
  };
  selectField(id);
  document.addEventListener("mousemove", onDragMove);
  document.addEventListener("mouseup", onDragEnd);
}
function onDragMove(e) {
  if (!dragState) return;
  const field = state.fields.get(dragState.fieldId);
  if (!field) return;
  const dx = e.clientX - dragState.startX;
  const dy = e.clientY - dragState.startY;
  field.domX = dragState.startFieldX + dx;
  field.domY = dragState.startFieldY + dy;
  const el = document.getElementById(dragState.fieldId);
  if (el) {
    el.style.left = `${field.domX}px`;
    el.style.top = `${field.domY}px`;
  }
  const pageInfo = PdfBridge.getPageInfo(field.pageNum);
  if (pageInfo) {
    const pdfRect = domRectToPdf(pageInfo.viewport, field.domX, field.domY, field.domWidth, field.domHeight);
    field.pdfX = pdfRect.x;
    field.pdfY = pdfRect.y;
  }
}
function onDragEnd() {
  dragState = null;
  document.removeEventListener("mousemove", onDragMove);
  document.removeEventListener("mouseup", onDragEnd);
}
function startResize(id, handle, e) {
  const field = state.fields.get(id);
  if (!field) return;
  resizeState = {
    fieldId: id,
    handle,
    startX: e.clientX,
    startY: e.clientY,
    startRect: {
      x: field.domX,
      y: field.domY,
      width: field.domWidth,
      height: field.domHeight
    }
  };
  document.addEventListener("mousemove", onResizeMove);
  document.addEventListener("mouseup", onResizeEnd);
}
function onResizeMove(e) {
  if (!resizeState) return;
  const field = state.fields.get(resizeState.fieldId);
  if (!field) return;
  const dx = e.clientX - resizeState.startX;
  const dy = e.clientY - resizeState.startY;
  const { handle, startRect } = resizeState;
  let newX = startRect.x;
  let newY = startRect.y;
  let newWidth = startRect.width;
  let newHeight = startRect.height;
  if (handle.includes("w")) {
    newX = startRect.x + dx;
    newWidth = startRect.width - dx;
  }
  if (handle.includes("e")) {
    newWidth = startRect.width + dx;
  }
  if (handle.includes("n")) {
    newY = startRect.y + dy;
    newHeight = startRect.height - dy;
  }
  if (handle.includes("s")) {
    newHeight = startRect.height + dy;
  }
  if (newWidth < MIN_FIELD_WIDTH) {
    if (handle.includes("w")) {
      newX = startRect.x + startRect.width - MIN_FIELD_WIDTH;
    }
    newWidth = MIN_FIELD_WIDTH;
  }
  if (newHeight < MIN_FIELD_HEIGHT) {
    if (handle.includes("n")) {
      newY = startRect.y + startRect.height - MIN_FIELD_HEIGHT;
    }
    newHeight = MIN_FIELD_HEIGHT;
  }
  field.domX = newX;
  field.domY = newY;
  field.domWidth = newWidth;
  field.domHeight = newHeight;
  const el = document.getElementById(resizeState.fieldId);
  if (el) {
    el.style.left = `${newX}px`;
    el.style.top = `${newY}px`;
    el.style.width = `${newWidth}px`;
    el.style.height = `${newHeight}px`;
  }
  const pageInfo = PdfBridge.getPageInfo(field.pageNum);
  if (pageInfo) {
    const pdfRect = domRectToPdf(pageInfo.viewport, field.domX, field.domY, field.domWidth, field.domHeight);
    field.pdfX = pdfRect.x;
    field.pdfY = pdfRect.y;
    field.pdfWidth = pdfRect.width;
    field.pdfHeight = pdfRect.height;
  }
}
function onResizeEnd() {
  resizeState = null;
  document.removeEventListener("mousemove", onResizeMove);
  document.removeEventListener("mouseup", onResizeEnd);
}
function setTool(tool) {
  state.currentTool = tool;
  const container = document.querySelector(".template-editor-container");
  if (container) {
    if (tool === "select" || tool === null) {
      container.style.cursor = "default";
    } else {
      container.style.cursor = "crosshair";
    }
  }
}
function getCurrentTool() {
  return state.currentTool;
}
function setCurrentStyle(style) {
  Object.assign(state.currentStyle, style);
  if (state.selectedFieldId) {
    updateFieldStyle(state.selectedFieldId, style);
  }
}
async function loadPdf(bytes) {
  state.pdfBytes = bytes;
  state.pageCount = await PdfBridge.loadDocument(bytes);
  return state.pageCount;
}
async function renderPage(pageNum, container) {
  container.innerHTML = "";
  container.className = "template-editor-page";
  container.dataset.page = pageNum.toString();
  const canvas = document.createElement("canvas");
  canvas.className = "page-canvas";
  container.appendChild(canvas);
  const overlay = document.createElement("div");
  overlay.className = "field-overlay";
  overlay.style.position = "absolute";
  overlay.style.top = "0";
  overlay.style.left = "0";
  overlay.style.width = "100%";
  overlay.style.height = "100%";
  overlay.style.pointerEvents = "none";
  container.appendChild(overlay);
  const dims = await PdfBridge.renderPage(pageNum, canvas);
  container.style.width = `${dims.width}px`;
  container.style.height = `${dims.height}px`;
  container.style.position = "relative";
  canvas.addEventListener("click", (e) => {
    if (state.currentTool && state.currentTool !== "select") {
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      const pageInfo = PdfBridge.getPageInfo(pageNum);
      if (pageInfo) {
        placeField(state.currentTool, pageNum, x, y, pageInfo.viewport);
        setTool("select");
      }
    }
  });
}
async function renderAllPages(container) {
  container.innerHTML = "";
  for (let i = 1; i <= state.pageCount; i++) {
    const pageDiv = document.createElement("div");
    pageDiv.className = "page-wrapper";
    pageDiv.style.marginBottom = "20px";
    container.appendChild(pageDiv);
    await renderPage(i, pageDiv);
  }
}
function getAllFields() {
  return Array.from(state.fields.values());
}
function exportFieldsAsJson() {
  return JSON.stringify(getAllFields(), null, 2);
}
function clearAllFields() {
  state.fields.forEach((_, id) => {
    const el = document.getElementById(id);
    if (el) el.remove();
  });
  state.fields.clear();
  state.selectedFieldId = null;
}
var TemplateEditor = {
  // Field types
  FieldType,
  // Field operations
  placeField,
  selectField,
  deleteField,
  updateFieldStyle,
  getAllFields,
  exportFieldsAsJson,
  clearAllFields,
  // Tool management
  setTool,
  getCurrentTool,
  setCurrentStyle,
  // PDF operations
  loadPdf,
  renderPage,
  renderAllPages,
  // Drag operations
  startDrag
};
window.TemplateEditor = TemplateEditor;

// src/ts/page-operations.ts
async function splitPdf(pdfBytes, pageRanges) {
  try {
    const PdfJoinSession = window.wasmBindings?.PdfJoinSession;
    if (!PdfJoinSession) {
      return { success: false, error: "pdfjoin-wasm not loaded" };
    }
    const session = new PdfJoinSession(0 /* Split */);
    try {
      session.addDocument("source.pdf", pdfBytes);
      session.setPageSelection(pageRanges);
      const result = session.execute();
      return { success: true, data: result };
    } finally {
      session.free();
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { success: false, error: message };
  }
}
function parsePageRanges(rangeStr, totalPages) {
  const pages = /* @__PURE__ */ new Set();
  const parts = rangeStr.split(",").map((p) => p.trim());
  for (const part of parts) {
    if (part.includes("-")) {
      const [start, end] = part.split("-").map((n) => parseInt(n.trim(), 10));
      if (!isNaN(start) && !isNaN(end)) {
        for (let i = Math.max(1, start); i <= Math.min(totalPages, end); i++) {
          pages.add(i);
        }
      }
    } else {
      const num = parseInt(part, 10);
      if (!isNaN(num) && num >= 1 && num <= totalPages) {
        pages.add(num);
      }
    }
  }
  return Array.from(pages).sort((a, b) => a - b);
}
async function mergePdfs(documents) {
  if (documents.length === 0) {
    return { success: false, error: "No documents to merge" };
  }
  if (documents.length === 1) {
    return { success: true, data: documents[0].bytes };
  }
  try {
    const PdfJoinSession = window.wasmBindings?.PdfJoinSession;
    if (!PdfJoinSession) {
      return { success: false, error: "pdfjoin-wasm not loaded" };
    }
    const session = new PdfJoinSession(1 /* Merge */);
    try {
      for (const doc of documents) {
        session.addDocument(doc.name, doc.bytes);
      }
      const result = session.execute();
      return { success: true, data: result };
    } finally {
      session.free();
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { success: false, error: message };
  }
}
async function appendPdf(existingPdf, existingName, newPdf, newName, prepend = false) {
  const documents = prepend ? [
    { name: newName, bytes: newPdf },
    { name: existingName, bytes: existingPdf }
  ] : [
    { name: existingName, bytes: existingPdf },
    { name: newName, bytes: newPdf }
  ];
  return mergePdfs(documents);
}
var PageOperations = {
  // Split
  splitPdf,
  parsePageRanges,
  // Merge
  mergePdfs,
  appendPdf
};
window.PageOperations = PageOperations;

// src/ts/recipient-manager.ts
var SigningMode = /* @__PURE__ */ ((SigningMode2) => {
  SigningMode2["Parallel"] = "parallel";
  SigningMode2["Sequential"] = "sequential";
  return SigningMode2;
})(SigningMode || {});
var RECIPIENT_COLORS = [
  "#0066CC",
  // Blue
  "#CC6600",
  // Orange
  "#00CC66",
  // Green
  "#CC0066",
  // Magenta
  "#6600CC",
  // Purple
  "#00CCCC"
  // Teal
];
var state2 = {
  recipients: /* @__PURE__ */ new Map(),
  assignments: /* @__PURE__ */ new Map(),
  signingMode: "parallel" /* Parallel */,
  nextOrder: 1
};
function generateRecipientId() {
  return `recipient-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}
function getNextColor() {
  const usedColors = Array.from(state2.recipients.values()).map((r) => r.color);
  const availableColor = RECIPIENT_COLORS.find((c) => !usedColors.includes(c));
  return availableColor || RECIPIENT_COLORS[state2.recipients.size % RECIPIENT_COLORS.length];
}
function addRecipient(name, email, role = "signer") {
  const id = generateRecipientId();
  const recipient = {
    id,
    name: name.trim(),
    email: email.trim().toLowerCase(),
    role,
    order: state2.nextOrder++,
    color: getNextColor()
  };
  state2.recipients.set(id, recipient);
  renderRecipientList();
  return recipient;
}
function removeRecipient(id) {
  state2.recipients.delete(id);
  for (const [fieldId, recipientId] of state2.assignments.entries()) {
    if (recipientId === id) {
      state2.assignments.delete(fieldId);
      updateFieldVisual(fieldId, null);
    }
  }
  reorderRecipients();
  renderRecipientList();
}
function updateRecipient(id, updates) {
  const recipient = state2.recipients.get(id);
  if (!recipient) return;
  if (updates.name !== void 0) recipient.name = updates.name.trim();
  if (updates.email !== void 0) recipient.email = updates.email.trim().toLowerCase();
  if (updates.role !== void 0) recipient.role = updates.role;
  if (updates.order !== void 0) recipient.order = updates.order;
  renderRecipientList();
}
function reorderRecipients() {
  const sorted = Array.from(state2.recipients.values()).sort((a, b) => a.order - b.order);
  sorted.forEach((r, i) => {
    r.order = i + 1;
  });
  state2.nextOrder = sorted.length + 1;
}
function moveRecipientUp(id) {
  const recipient = state2.recipients.get(id);
  if (!recipient || recipient.order <= 1) return;
  for (const r of state2.recipients.values()) {
    if (r.order === recipient.order - 1) {
      r.order = recipient.order;
      recipient.order = recipient.order - 1;
      break;
    }
  }
  renderRecipientList();
}
function moveRecipientDown(id) {
  const recipient = state2.recipients.get(id);
  if (!recipient || recipient.order >= state2.recipients.size) return;
  for (const r of state2.recipients.values()) {
    if (r.order === recipient.order + 1) {
      r.order = recipient.order;
      recipient.order = recipient.order + 1;
      break;
    }
  }
  renderRecipientList();
}
function getAllRecipients() {
  return Array.from(state2.recipients.values()).sort((a, b) => a.order - b.order);
}
function getRecipient(id) {
  return state2.recipients.get(id);
}
function assignFieldToRecipient(fieldId, recipientId) {
  const recipient = state2.recipients.get(recipientId);
  if (!recipient) return;
  state2.assignments.set(fieldId, recipientId);
  updateFieldVisual(fieldId, recipient);
}
function unassignField(fieldId) {
  state2.assignments.delete(fieldId);
  updateFieldVisual(fieldId, null);
}
function getFieldRecipient(fieldId) {
  const recipientId = state2.assignments.get(fieldId);
  if (!recipientId) return void 0;
  return state2.recipients.get(recipientId);
}
function getRecipientFields(recipientId) {
  const fields = [];
  for (const [fieldId, rId] of state2.assignments.entries()) {
    if (rId === recipientId) {
      fields.push(fieldId);
    }
  }
  return fields;
}
function getAllAssignments() {
  return Array.from(state2.assignments.entries()).map(([fieldId, recipientId]) => ({
    fieldId,
    recipientId
  }));
}
function updateFieldVisual(fieldId, recipient) {
  const el = document.getElementById(fieldId);
  if (!el) return;
  if (recipient) {
    el.style.borderColor = recipient.color;
    el.style.boxShadow = `0 0 0 3px ${recipient.color}33`;
    let badge = el.querySelector(".recipient-badge");
    if (!badge) {
      badge = document.createElement("div");
      badge.className = "recipient-badge";
      badge.style.position = "absolute";
      badge.style.top = "-24px";
      badge.style.left = "0";
      badge.style.fontSize = "12px";
      badge.style.fontWeight = "bold";
      badge.style.padding = "2px 6px";
      badge.style.borderRadius = "4px";
      badge.style.whiteSpace = "nowrap";
      el.appendChild(badge);
    }
    badge.textContent = recipient.name;
    badge.style.backgroundColor = recipient.color;
    badge.style.color = "#FFFFFF";
  } else {
    el.style.borderColor = "#0066cc";
    el.style.boxShadow = "none";
    const badge = el.querySelector(".recipient-badge");
    if (badge) badge.remove();
  }
}
function setSigningMode(mode) {
  state2.signingMode = mode;
  renderRecipientList();
}
function getSigningMode() {
  return state2.signingMode;
}
function renderRecipientList() {
  const container = document.getElementById("recipient-list");
  if (!container) return;
  container.innerHTML = "";
  const header = document.createElement("div");
  header.className = "recipient-header";
  header.style.marginBottom = "20px";
  header.innerHTML = `
    <h3 style="font-size: 20px; margin: 0 0 12px 0; color: #333;">Recipients</h3>
    <div style="display: flex; gap: 12px; margin-bottom: 16px;">
      <button id="mode-parallel" class="mode-btn ${state2.signingMode === "parallel" /* Parallel */ ? "active" : ""}"
              style="flex: 1; height: 48px; font-size: 16px; border-radius: 8px; border: 2px solid #0066CC; cursor: pointer;
                     background: ${state2.signingMode === "parallel" /* Parallel */ ? "#0066CC" : "#fff"};
                     color: ${state2.signingMode === "parallel" /* Parallel */ ? "#fff" : "#0066CC"};">
        All at Once
      </button>
      <button id="mode-sequential" class="mode-btn ${state2.signingMode === "sequential" /* Sequential */ ? "active" : ""}"
              style="flex: 1; height: 48px; font-size: 16px; border-radius: 8px; border: 2px solid #0066CC; cursor: pointer;
                     background: ${state2.signingMode === "sequential" /* Sequential */ ? "#0066CC" : "#fff"};
                     color: ${state2.signingMode === "sequential" /* Sequential */ ? "#fff" : "#0066CC"};">
        In Order
      </button>
    </div>
  `;
  container.appendChild(header);
  header.querySelector("#mode-parallel")?.addEventListener("click", () => setSigningMode("parallel" /* Parallel */));
  header.querySelector("#mode-sequential")?.addEventListener("click", () => setSigningMode("sequential" /* Sequential */));
  const recipients = getAllRecipients();
  if (recipients.length === 0) {
    const emptyMsg = document.createElement("p");
    emptyMsg.style.cssText = "font-size: 16px; color: #666; text-align: center; padding: 20px;";
    emptyMsg.textContent = "No recipients added yet";
    container.appendChild(emptyMsg);
  } else {
    recipients.forEach((recipient) => {
      const row = createRecipientRow(recipient);
      container.appendChild(row);
    });
  }
  const addBtn = document.createElement("button");
  addBtn.id = "add-recipient-btn";
  addBtn.className = "add-recipient-btn";
  addBtn.style.cssText = `
    width: 100%;
    height: 60px;
    font-size: 18px;
    font-weight: bold;
    border: 2px dashed #0066CC;
    border-radius: 12px;
    background: #f0f8ff;
    color: #0066CC;
    cursor: pointer;
    margin-top: 16px;
  `;
  addBtn.textContent = "+ Add Recipient";
  addBtn.addEventListener("click", showAddRecipientForm);
  container.appendChild(addBtn);
}
function createRecipientRow(recipient) {
  const row = document.createElement("div");
  row.className = "recipient-row";
  row.dataset.recipientId = recipient.id;
  row.style.cssText = `
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    margin-bottom: 12px;
    background: #fff;
    border: 2px solid ${recipient.color};
    border-radius: 12px;
    border-left: 8px solid ${recipient.color};
  `;
  if (state2.signingMode === "sequential" /* Sequential */) {
    const orderNum = document.createElement("div");
    orderNum.className = "recipient-order";
    orderNum.style.cssText = `
      width: 36px;
      height: 36px;
      border-radius: 50%;
      background: ${recipient.color};
      color: #fff;
      font-size: 18px;
      font-weight: bold;
      display: flex;
      align-items: center;
      justify-content: center;
    `;
    orderNum.textContent = recipient.order.toString();
    row.appendChild(orderNum);
  }
  const info = document.createElement("div");
  info.className = "recipient-info";
  info.style.cssText = "flex: 1; min-width: 0;";
  info.innerHTML = `
    <div style="font-size: 18px; font-weight: bold; color: #333; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
      ${escapeHtml(recipient.name)}
    </div>
    <div style="font-size: 14px; color: #666; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
      ${escapeHtml(recipient.email)}
    </div>
    <div style="font-size: 12px; color: #999; margin-top: 4px;">
      ${getRecipientFields(recipient.id).length} fields assigned
    </div>
  `;
  row.appendChild(info);
  const actions = document.createElement("div");
  actions.className = "recipient-actions";
  actions.style.cssText = "display: flex; gap: 8px; flex-shrink: 0;";
  if (state2.signingMode === "sequential" /* Sequential */) {
    const upBtn = createActionButton("\u25B2", "Move Up", () => moveRecipientUp(recipient.id));
    upBtn.disabled = recipient.order <= 1;
    actions.appendChild(upBtn);
    const downBtn = createActionButton("\u25BC", "Move Down", () => moveRecipientDown(recipient.id));
    downBtn.disabled = recipient.order >= state2.recipients.size;
    actions.appendChild(downBtn);
  }
  const deleteBtn = createActionButton("\xD7", "Remove", () => {
    if (confirm(`Remove ${recipient.name} from recipients?`)) {
      removeRecipient(recipient.id);
    }
  });
  deleteBtn.style.backgroundColor = "#ffebeb";
  deleteBtn.style.color = "#cc0000";
  deleteBtn.style.borderColor = "#cc0000";
  actions.appendChild(deleteBtn);
  row.appendChild(actions);
  return row;
}
function createActionButton(text, title, onClick) {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.title = title;
  btn.textContent = text;
  btn.style.cssText = `
    width: 44px;
    height: 44px;
    font-size: 18px;
    border: 2px solid #ccc;
    border-radius: 8px;
    background: #fff;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  `;
  btn.addEventListener("click", onClick);
  return btn;
}
function showAddRecipientForm() {
  const container = document.getElementById("recipient-list");
  if (!container) return;
  const addBtn = container.querySelector("#add-recipient-btn");
  if (addBtn) addBtn.style.display = "none";
  const form = document.createElement("div");
  form.id = "add-recipient-form";
  form.className = "add-recipient-form";
  form.style.cssText = `
    padding: 20px;
    background: #f0f8ff;
    border: 2px solid #0066CC;
    border-radius: 12px;
    margin-top: 16px;
  `;
  form.innerHTML = `
    <h4 style="font-size: 18px; margin: 0 0 16px 0; color: #333;">Add New Recipient</h4>
    <div style="margin-bottom: 16px;">
      <label style="font-size: 16px; display: block; margin-bottom: 8px; color: #333;">Full Name</label>
      <input type="text" id="new-recipient-name" placeholder="John Smith"
             style="width: 100%; height: 48px; font-size: 18px; padding: 8px 12px; border: 2px solid #ccc; border-radius: 8px; box-sizing: border-box;">
    </div>
    <div style="margin-bottom: 16px;">
      <label style="font-size: 16px; display: block; margin-bottom: 8px; color: #333;">Email Address</label>
      <input type="email" id="new-recipient-email" placeholder="john@example.com"
             style="width: 100%; height: 48px; font-size: 18px; padding: 8px 12px; border: 2px solid #ccc; border-radius: 8px; box-sizing: border-box;">
    </div>
    <div style="margin-bottom: 20px;">
      <label style="font-size: 16px; display: block; margin-bottom: 8px; color: #333;">Role</label>
      <select id="new-recipient-role"
              style="width: 100%; height: 48px; font-size: 18px; padding: 8px 12px; border: 2px solid #ccc; border-radius: 8px; box-sizing: border-box;">
        <option value="signer">Signer - Must sign the document</option>
        <option value="reviewer">Reviewer - Views and approves</option>
        <option value="cc">CC - Receives copy only</option>
      </select>
    </div>
    <div style="display: flex; gap: 12px;">
      <button id="save-recipient-btn" type="button"
              style="flex: 1; height: 60px; font-size: 18px; font-weight: bold; background: #0066CC; color: #fff; border: none; border-radius: 12px; cursor: pointer;">
        Add Recipient
      </button>
      <button id="cancel-recipient-btn" type="button"
              style="width: 100px; height: 60px; font-size: 16px; background: #fff; color: #666; border: 2px solid #ccc; border-radius: 12px; cursor: pointer;">
        Cancel
      </button>
    </div>
  `;
  container.appendChild(form);
  const nameInput = document.getElementById("new-recipient-name");
  nameInput?.focus();
  document.getElementById("save-recipient-btn")?.addEventListener("click", () => {
    const name = document.getElementById("new-recipient-name").value;
    const email = document.getElementById("new-recipient-email").value;
    const role = document.getElementById("new-recipient-role").value;
    if (!name.trim()) {
      alert("Please enter a name");
      return;
    }
    if (!email.trim() || !isValidEmail(email)) {
      alert("Please enter a valid email address");
      return;
    }
    addRecipient(name, email, role);
    form.remove();
  });
  document.getElementById("cancel-recipient-btn")?.addEventListener("click", () => {
    form.remove();
    if (addBtn) addBtn.style.display = "block";
  });
}
function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}
function isValidEmail(email) {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}
function exportRecipientData() {
  return {
    recipients: getAllRecipients(),
    assignments: getAllAssignments(),
    signingMode: state2.signingMode
  };
}
function importRecipientData(data) {
  state2.recipients.clear();
  state2.assignments.clear();
  data.recipients.forEach((r) => {
    state2.recipients.set(r.id, r);
  });
  state2.nextOrder = data.recipients.length + 1;
  data.assignments.forEach((a) => {
    state2.assignments.set(a.fieldId, a.recipientId);
    const recipient = state2.recipients.get(a.recipientId);
    if (recipient) {
      updateFieldVisual(a.fieldId, recipient);
    }
  });
  state2.signingMode = data.signingMode;
  renderRecipientList();
}
function clearAllRecipients() {
  for (const fieldId of state2.assignments.keys()) {
    updateFieldVisual(fieldId, null);
  }
  state2.recipients.clear();
  state2.assignments.clear();
  state2.nextOrder = 1;
  renderRecipientList();
}
var RecipientManager = {
  // Recipient operations
  addRecipient,
  removeRecipient,
  updateRecipient,
  getRecipient,
  getAllRecipients,
  moveRecipientUp,
  moveRecipientDown,
  // Field assignments
  assignFieldToRecipient,
  unassignField,
  getFieldRecipient,
  getRecipientFields,
  getAllAssignments,
  // Signing mode
  setSigningMode,
  getSigningMode,
  SigningMode,
  // UI
  renderRecipientList,
  // Import/Export
  exportRecipientData,
  importRecipientData,
  clearAllRecipients
};
window.RecipientManager = RecipientManager;

// src/ts/dispatch-modal.ts
var BUTTON_HEIGHT = 60;
var BUTTON_FONT_SIZE = 18;
var LABEL_FONT_SIZE = 18;
var BODY_FONT_SIZE = 16;
var BORDER_RADIUS = 12;
var state3 = {
  isOpen: false,
  modalElement: null,
  dispatchStatus: /* @__PURE__ */ new Map(),
  documentName: "Document",
  pdfBytes: null,
  onSendCallback: null
};
function openDispatchModal(documentName, pdfBytes) {
  if (state3.isOpen) return;
  if (documentName) state3.documentName = documentName;
  if (pdfBytes) state3.pdfBytes = pdfBytes;
  state3.dispatchStatus.clear();
  createModalElement();
  state3.isOpen = true;
  document.body.style.overflow = "hidden";
  state3.modalElement?.focus();
}
function closeDispatchModal() {
  if (!state3.isOpen || !state3.modalElement) return;
  state3.modalElement.remove();
  state3.modalElement = null;
  state3.isOpen = false;
  document.body.style.overflow = "";
}
function isModalOpen() {
  return state3.isOpen;
}
function setEmailCallback(callback) {
  state3.onSendCallback = callback;
}
function createModalElement() {
  const overlay = document.createElement("div");
  overlay.id = "dispatch-modal-overlay";
  overlay.style.cssText = `
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
    padding: 20px;
  `;
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) {
      closeDispatchModal();
    }
  });
  const modal = document.createElement("div");
  modal.id = "dispatch-modal";
  modal.setAttribute("role", "dialog");
  modal.setAttribute("aria-labelledby", "dispatch-modal-title");
  modal.setAttribute("tabindex", "-1");
  modal.style.cssText = `
    background: #fff;
    border-radius: ${BORDER_RADIUS}px;
    max-width: 600px;
    width: 100%;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  `;
  modal.appendChild(createModalHeader());
  modal.appendChild(createModalBody());
  modal.appendChild(createModalFooter());
  overlay.appendChild(modal);
  document.body.appendChild(overlay);
  state3.modalElement = overlay;
  overlay.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      closeDispatchModal();
    }
  });
}
function createModalHeader() {
  const header = document.createElement("div");
  header.className = "dispatch-modal-header";
  header.style.cssText = `
    padding: 24px 24px 16px 24px;
    border-bottom: 2px solid #eee;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  `;
  const titleGroup = document.createElement("div");
  titleGroup.innerHTML = `
    <h2 id="dispatch-modal-title" style="margin: 0 0 8px 0; font-size: 24px; color: #333;">
      Send for Signatures
    </h2>
    <p style="margin: 0; font-size: ${BODY_FONT_SIZE}px; color: #666;">
      ${escapeHtml2(state3.documentName)}
    </p>
  `;
  header.appendChild(titleGroup);
  const closeBtn = document.createElement("button");
  closeBtn.type = "button";
  closeBtn.setAttribute("aria-label", "Close modal");
  closeBtn.style.cssText = `
    width: 48px;
    height: 48px;
    font-size: 28px;
    border: none;
    background: #f5f5f5;
    border-radius: 50%;
    cursor: pointer;
    color: #666;
    display: flex;
    align-items: center;
    justify-content: center;
  `;
  closeBtn.textContent = "\xD7";
  closeBtn.addEventListener("click", closeDispatchModal);
  header.appendChild(closeBtn);
  return header;
}
function createModalBody() {
  const body = document.createElement("div");
  body.className = "dispatch-modal-body";
  body.style.cssText = "padding: 24px;";
  const recipients = RecipientManager.getAllRecipients();
  const signingMode = RecipientManager.getSigningMode();
  const fields = getAllFields();
  const modeIndicator = document.createElement("div");
  modeIndicator.style.cssText = `
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: ${signingMode === "sequential" /* Sequential */ ? "#fff8e6" : "#e6f4ff"};
    border-radius: ${BORDER_RADIUS}px;
    margin-bottom: 20px;
  `;
  modeIndicator.innerHTML = `
    <span style="font-size: 28px;">${signingMode === "sequential" /* Sequential */ ? "\u23F3" : "\u2B1C"}</span>
    <div>
      <div style="font-size: ${LABEL_FONT_SIZE}px; font-weight: bold; color: #333;">
        ${signingMode === "sequential" /* Sequential */ ? "Sequential Signing" : "Parallel Signing"}
      </div>
      <div style="font-size: 14px; color: #666;">
        ${signingMode === "sequential" /* Sequential */ ? "Recipients will sign one after another in order" : "All recipients will receive the document at the same time"}
      </div>
    </div>
  `;
  body.appendChild(modeIndicator);
  const warnings = validateDispatch(recipients, fields);
  if (warnings.length > 0) {
    const warningBox = document.createElement("div");
    warningBox.style.cssText = `
      padding: 16px;
      background: #fff3e6;
      border: 2px solid #ff9800;
      border-radius: ${BORDER_RADIUS}px;
      margin-bottom: 20px;
    `;
    warningBox.innerHTML = `
      <div style="font-size: ${LABEL_FONT_SIZE}px; font-weight: bold; color: #e65100; margin-bottom: 8px;">
        Please Review
      </div>
      <ul style="margin: 0; padding-left: 20px; font-size: ${BODY_FONT_SIZE}px; color: #333;">
        ${warnings.map((w) => `<li>${escapeHtml2(w)}</li>`).join("")}
      </ul>
    `;
    body.appendChild(warningBox);
  }
  const heading = document.createElement("h3");
  heading.style.cssText = `font-size: ${LABEL_FONT_SIZE}px; margin: 0 0 16px 0; color: #333;`;
  heading.textContent = `Recipients (${recipients.length})`;
  body.appendChild(heading);
  if (recipients.length === 0) {
    const emptyState = document.createElement("div");
    emptyState.style.cssText = `
      text-align: center;
      padding: 40px 20px;
      background: #f9f9f9;
      border-radius: ${BORDER_RADIUS}px;
    `;
    emptyState.innerHTML = `
      <div style="font-size: 48px; margin-bottom: 16px;">
        <span role="img" aria-label="No recipients">&#128100;</span>
      </div>
      <div style="font-size: ${LABEL_FONT_SIZE}px; color: #666;">
        No recipients added yet
      </div>
      <div style="font-size: 14px; color: #999; margin-top: 8px;">
        Add recipients before sending the document
      </div>
    `;
    body.appendChild(emptyState);
    return body;
  }
  const recipientList = document.createElement("div");
  recipientList.className = "dispatch-recipient-list";
  recipientList.style.cssText = "display: flex; flex-direction: column; gap: 12px;";
  recipients.forEach((recipient) => {
    const card = createRecipientCard(recipient, fields);
    recipientList.appendChild(card);
  });
  body.appendChild(recipientList);
  return body;
}
function createRecipientCard(recipient, allFields) {
  const assignedFieldIds = RecipientManager.getRecipientFields(recipient.id);
  const assignedFields = allFields.filter((f) => assignedFieldIds.includes(f.id));
  const signingMode = RecipientManager.getSigningMode();
  const status = state3.dispatchStatus.get(recipient.id) || "pending";
  const card = document.createElement("div");
  card.className = "dispatch-recipient-card";
  card.dataset.recipientId = recipient.id;
  card.style.cssText = `
    padding: 20px;
    background: #fff;
    border: 2px solid ${recipient.color};
    border-left: 8px solid ${recipient.color};
    border-radius: ${BORDER_RADIUS}px;
    position: relative;
  `;
  const statusBadge = document.createElement("div");
  statusBadge.className = "dispatch-status-badge";
  statusBadge.style.cssText = `
    position: absolute;
    top: 12px;
    right: 12px;
    padding: 6px 12px;
    border-radius: 20px;
    font-size: 14px;
    font-weight: bold;
    ${getStatusStyle(status)}
  `;
  statusBadge.textContent = getStatusLabel(status);
  card.appendChild(statusBadge);
  const infoSection = document.createElement("div");
  infoSection.style.cssText = "padding-right: 100px;";
  let orderHtml = "";
  if (signingMode === "sequential" /* Sequential */) {
    orderHtml = `
      <span style="
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 32px;
        height: 32px;
        background: ${recipient.color};
        color: #fff;
        border-radius: 50%;
        font-weight: bold;
        font-size: 16px;
        margin-right: 12px;
      ">${recipient.order}</span>
    `;
  }
  infoSection.innerHTML = `
    <div style="display: flex; align-items: center; margin-bottom: 8px;">
      ${orderHtml}
      <span style="font-size: ${LABEL_FONT_SIZE}px; font-weight: bold; color: #333;">
        ${escapeHtml2(recipient.name)}
      </span>
      <span style="margin-left: 12px; font-size: 14px; color: #666; background: #f0f0f0; padding: 2px 8px; border-radius: 4px;">
        ${getRoleLabel(recipient.role)}
      </span>
    </div>
    <div style="font-size: ${BODY_FONT_SIZE}px; color: #666; margin-bottom: 12px;">
      ${escapeHtml2(recipient.email)}
    </div>
  `;
  card.appendChild(infoSection);
  const fieldsSection = document.createElement("div");
  fieldsSection.style.cssText = `
    padding-top: 12px;
    border-top: 1px solid #eee;
  `;
  if (assignedFields.length === 0) {
    fieldsSection.innerHTML = `
      <div style="font-size: 14px; color: #999; font-style: italic;">
        No fields assigned to this recipient
      </div>
    `;
  } else {
    const fieldsSummary = summarizeFields(assignedFields);
    fieldsSection.innerHTML = `
      <div style="font-size: 14px; color: #333; font-weight: bold; margin-bottom: 8px;">
        ${assignedFields.length} field${assignedFields.length !== 1 ? "s" : ""} to complete:
      </div>
      <div style="display: flex; flex-wrap: wrap; gap: 8px;">
        ${fieldsSummary.map((fs) => `
          <span style="
            padding: 4px 10px;
            background: ${recipient.color}20;
            color: ${recipient.color};
            border-radius: 4px;
            font-size: 13px;
            font-weight: 500;
          ">${fs.count} ${fs.label}</span>
        `).join("")}
      </div>
    `;
  }
  card.appendChild(fieldsSection);
  return card;
}
function createModalFooter() {
  const footer = document.createElement("div");
  footer.className = "dispatch-modal-footer";
  footer.style.cssText = `
    padding: 20px 24px 24px 24px;
    border-top: 2px solid #eee;
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  `;
  const recipients = RecipientManager.getAllRecipients();
  const fields = getAllFields();
  const canSend = recipients.length > 0 && validateDispatch(recipients, fields).length === 0;
  const cancelBtn = document.createElement("button");
  cancelBtn.type = "button";
  cancelBtn.style.cssText = `
    flex: 1;
    min-width: 140px;
    height: ${BUTTON_HEIGHT}px;
    font-size: ${BUTTON_FONT_SIZE}px;
    font-weight: bold;
    background: #fff;
    color: #666;
    border: 2px solid #ccc;
    border-radius: ${BORDER_RADIUS}px;
    cursor: pointer;
  `;
  cancelBtn.textContent = "Cancel";
  cancelBtn.addEventListener("click", closeDispatchModal);
  footer.appendChild(cancelBtn);
  const sendBtn = document.createElement("button");
  sendBtn.type = "button";
  sendBtn.id = "dispatch-send-btn";
  sendBtn.disabled = !canSend;
  sendBtn.style.cssText = `
    flex: 2;
    min-width: 200px;
    height: ${BUTTON_HEIGHT}px;
    font-size: ${BUTTON_FONT_SIZE}px;
    font-weight: bold;
    background: ${canSend ? "#0066CC" : "#ccc"};
    color: #fff;
    border: none;
    border-radius: ${BORDER_RADIUS}px;
    cursor: ${canSend ? "pointer" : "not-allowed"};
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
  `;
  sendBtn.innerHTML = `
    <span style="font-size: 24px;">&#9993;</span>
    Send to ${recipients.length} Recipient${recipients.length !== 1 ? "s" : ""}
  `;
  if (canSend) {
    sendBtn.addEventListener("click", handleSend);
  }
  footer.appendChild(sendBtn);
  return footer;
}
async function handleSend() {
  const sendBtn = document.getElementById("dispatch-send-btn");
  if (!sendBtn) return;
  const recipients = RecipientManager.getAllRecipients();
  const signingMode = RecipientManager.getSigningMode();
  sendBtn.disabled = true;
  sendBtn.innerHTML = `
    <span class="spinner" style="
      width: 24px;
      height: 24px;
      border: 3px solid #ffffff40;
      border-top-color: #fff;
      border-radius: 50%;
      animation: spin 1s linear infinite;
    "></span>
    Sending...
  `;
  if (!document.getElementById("dispatch-spinner-style")) {
    const style = document.createElement("style");
    style.id = "dispatch-spinner-style";
    style.textContent = "@keyframes spin { to { transform: rotate(360deg); } }";
    document.head.appendChild(style);
  }
  const results = [];
  for (const recipient of recipients) {
    state3.dispatchStatus.set(recipient.id, "sending");
    updateRecipientCardStatus(recipient.id, "sending");
    try {
      const payload = {
        to: recipient.email,
        recipientName: recipient.name,
        documentName: state3.documentName,
        signingUrl: generateSigningUrl(recipient.id),
        order: recipient.order,
        isSequential: signingMode === "sequential" /* Sequential */,
        previousSigners: signingMode === "sequential" /* Sequential */ ? recipients.filter((r) => r.order < recipient.order).map((r) => r.name) : void 0
      };
      let success = false;
      if (state3.onSendCallback) {
        success = await state3.onSendCallback(payload);
      } else {
        await simulateSend();
        success = true;
      }
      state3.dispatchStatus.set(recipient.id, success ? "sent" : "error");
      updateRecipientCardStatus(recipient.id, success ? "sent" : "error");
      results.push({
        success,
        recipientId: recipient.id,
        email: recipient.email,
        timestamp: /* @__PURE__ */ new Date()
      });
    } catch (err) {
      state3.dispatchStatus.set(recipient.id, "error");
      updateRecipientCardStatus(recipient.id, "error");
      results.push({
        success: false,
        recipientId: recipient.id,
        email: recipient.email,
        error: err instanceof Error ? err.message : String(err)
      });
    }
  }
  const allSuccess = results.every((r) => r.success);
  const someSuccess = results.some((r) => r.success);
  sendBtn.style.background = allSuccess ? "#00aa00" : someSuccess ? "#ff9800" : "#cc0000";
  sendBtn.innerHTML = allSuccess ? '<span style="font-size: 24px;">&#10004;</span> Sent Successfully!' : `<span style="font-size: 24px;">&#9888;</span> ${results.filter((r) => r.success).length} of ${results.length} Sent`;
  sendBtn.disabled = false;
  sendBtn.addEventListener("click", closeDispatchModal);
}
function updateRecipientCardStatus(recipientId, status) {
  const card = document.querySelector(`[data-recipient-id="${recipientId}"]`);
  if (!card) return;
  const badge = card.querySelector(".dispatch-status-badge");
  if (badge) {
    badge.style.cssText = `
      position: absolute;
      top: 12px;
      right: 12px;
      padding: 6px 12px;
      border-radius: 20px;
      font-size: 14px;
      font-weight: bold;
      ${getStatusStyle(status)}
    `;
    badge.textContent = getStatusLabel(status);
  }
}
function simulateSend() {
  return new Promise((resolve) => setTimeout(resolve, 500 + Math.random() * 1e3));
}
function generateSigningUrl(recipientId) {
  const baseUrl = window.location.origin;
  const documentId = crypto.randomUUID ? crypto.randomUUID() : Date.now().toString(36);
  return `${baseUrl}/sign/${documentId}/${recipientId}`;
}
function validateDispatch(recipients, fields) {
  const warnings = [];
  if (recipients.length === 0) {
    warnings.push("Add at least one recipient");
    return warnings;
  }
  const signers = recipients.filter((r) => r.role === "signer");
  if (signers.length === 0) {
    warnings.push("Add at least one signer (not just reviewers or CC)");
  }
  const signatureFields = fields.filter((f) => f.type === "signature");
  if (signatureFields.length === 0) {
    warnings.push("Add at least one signature field to the document");
  }
  for (const signer of signers) {
    const signerFields = RecipientManager.getRecipientFields(signer.id);
    const signerSignatures = fields.filter(
      (f) => signerFields.includes(f.id) && f.type === "signature"
    );
    if (signerSignatures.length === 0) {
      warnings.push(`${signer.name} has no signature fields assigned`);
    }
  }
  const assignments = RecipientManager.getAllAssignments();
  const assignedFieldIds = new Set(assignments.map((a) => a.fieldId));
  const unassignedSignatures = signatureFields.filter((f) => !assignedFieldIds.has(f.id));
  if (unassignedSignatures.length > 0) {
    warnings.push(`${unassignedSignatures.length} signature field(s) not assigned to any recipient`);
  }
  return warnings;
}
function escapeHtml2(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}
function getStatusLabel(status) {
  switch (status) {
    case "pending":
      return "Pending";
    case "sending":
      return "Sending...";
    case "sent":
      return "Sent";
    case "error":
      return "Failed";
  }
}
function getStatusStyle(status) {
  switch (status) {
    case "pending":
      return "background: #f0f0f0; color: #666;";
    case "sending":
      return "background: #e6f4ff; color: #0066CC;";
    case "sent":
      return "background: #e6ffe6; color: #00aa00;";
    case "error":
      return "background: #ffe6e6; color: #cc0000;";
  }
}
function getRoleLabel(role) {
  switch (role) {
    case "signer":
      return "Signer";
    case "reviewer":
      return "Reviewer";
    case "cc":
      return "CC";
    default:
      return role;
  }
}
function summarizeFields(fields) {
  const counts = {};
  for (const field of fields) {
    const label = getFieldTypeLabel(field.type);
    counts[label] = (counts[label] || 0) + 1;
  }
  return Object.entries(counts).map(([label, count]) => ({ label, count }));
}
function getFieldTypeLabel(type) {
  switch (type) {
    case "signature":
      return "Signature";
    case "initials":
      return "Initials";
    case "text":
      return "Text";
    case "date":
      return "Date";
    case "checkbox":
      return "Checkbox";
    default:
      return type;
  }
}
var DispatchModal = {
  // Modal lifecycle
  open: openDispatchModal,
  close: closeDispatchModal,
  isOpen: isModalOpen,
  // Configuration
  setEmailCallback,
  // Types
  DispatchStatus: null
  // Type export
};
window.DispatchModal = DispatchModal;

// src/ts/app.ts
console.log("agentPDF Template Completion Engine loaded");
export {
  DispatchModal,
  FieldType,
  PageOperations,
  PdfBridge,
  RecipientManager,
  SigningMode,
  TemplateEditor,
  addRecipient,
  appendPdf,
  assignFieldToRecipient,
  clearAllFields,
  clearAllRecipients,
  clientRectToCanvasRelative,
  closeDispatchModal,
  deleteField,
  domPointToPdf,
  domRectToPdf,
  ensurePdfJsLoaded,
  exportFieldsAsJson,
  exportRecipientData,
  getAllAssignments,
  getAllFields,
  getAllRecipients,
  getCurrentTool,
  getFieldRecipient,
  getPageRenderInfo,
  getRecipient,
  getRecipientFields,
  getSigningMode,
  importRecipientData,
  isModalOpen,
  isPdfJsLoaded,
  loadPdf,
  mergePdfs,
  moveRecipientDown,
  moveRecipientUp,
  openDispatchModal,
  parsePageRanges,
  pdfPointToDom,
  pdfRectToDom,
  placeField,
  removeRecipient,
  renderAllPages,
  renderPage,
  renderRecipientList,
  selectField,
  setCurrentStyle,
  setEmailCallback,
  setSigningMode,
  setTool,
  splitPdf,
  startDrag,
  unassignField,
  updateFieldStyle,
  updateRecipient
};
//# sourceMappingURL=bundle.js.map
