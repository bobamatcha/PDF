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

// src/ts/app.ts
console.log("agentPDF Template Completion Engine loaded");
export {
  FieldType,
  PageOperations,
  PdfBridge,
  TemplateEditor,
  appendPdf,
  clearAllFields,
  clientRectToCanvasRelative,
  deleteField,
  domPointToPdf,
  domRectToPdf,
  ensurePdfJsLoaded,
  exportFieldsAsJson,
  getAllFields,
  getCurrentTool,
  getPageRenderInfo,
  isPdfJsLoaded,
  loadPdf,
  mergePdfs,
  parsePageRanges,
  pdfPointToDom,
  pdfRectToDom,
  placeField,
  renderAllPages,
  renderPage,
  selectField,
  setCurrentStyle,
  setTool,
  splitPdf,
  startDrag,
  updateFieldStyle
};
//# sourceMappingURL=bundle.js.map
