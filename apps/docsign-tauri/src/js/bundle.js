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

// src/ts/pdf-preview.ts
var PdfPreviewBridge = {
  currentDoc: null,
  pageCanvases: /* @__PURE__ */ new Map(),
  /**
   * Load a PDF document from bytes
   * @param data PDF file as Uint8Array or ArrayBuffer
   * @returns Number of pages in the document
   */
  async loadDocument(data) {
    await ensurePdfJsLoaded();
    const typedArray = data instanceof Uint8Array ? data : new Uint8Array(data);
    if (!window.pdfjsLib) {
      throw new Error("PDF.js not loaded");
    }
    this.currentDoc = await window.pdfjsLib.getDocument(typedArray).promise;
    return this.currentDoc.numPages;
  },
  /**
   * Render a page to a canvas element
   * @param pageNum 1-indexed page number
   * @param canvas Canvas element to render to
   * @param scale Rendering scale (default 1.5 for good quality)
   * @returns Page dimensions in various coordinate systems
   */
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
  /**
   * Get cached page dimensions
   * @param pageNum 1-indexed page number
   * @returns Page dimensions or null if not rendered
   */
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
  /**
   * Get cached page info (canvas, viewport, page)
   * @param pageNum 1-indexed page number
   * @returns Cached page info or undefined
   */
  getPageInfo(pageNum) {
    return this.pageCanvases.get(pageNum);
  },
  /**
   * Extract text with position information for each text item
   * Useful for signature field detection and text overlay
   * @param pageNum 1-indexed page number
   * @returns Array of text items with position and style info
   */
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
  /**
   * Cleanup resources - call when done with the document
   */
  cleanup() {
    if (this.currentDoc) {
      this.currentDoc.destroy();
      this.currentDoc = null;
    }
    this.pageCanvases.clear();
  }
};
var previewBridge = PdfPreviewBridge;
window.PdfPreviewBridge = PdfPreviewBridge;

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

// src/ts/error-messages.ts
var ERROR_PATTERNS = [
  {
    category: "network",
    patterns: [
      /network/i,
      /fetch/i,
      /offline/i,
      /connection/i,
      /internet/i,
      /failed to load/i,
      /timeout/i,
      /ECONNREFUSED/i,
      /ENOTFOUND/i,
      /ERR_NETWORK/i,
      /net::/i
    ]
  },
  {
    category: "password-protected",
    patterns: [
      /password/i,
      /encrypted/i,
      /protected/i,
      /decrypt/i,
      /locked/i,
      /access denied.*pdf/i
    ]
  },
  {
    category: "signature-invalid",
    patterns: [
      /signature.*invalid/i,
      /invalid.*signature/i,
      /signature.*failed/i,
      /failed.*signature/i,
      /signature.*error/i,
      /sign.*failed/i,
      /could not sign/i,
      /signing.*error/i
    ]
  },
  {
    category: "session-expired",
    patterns: [
      /session.*expired/i,
      /expired.*session/i,
      /link.*expired/i,
      /expired.*link/i,
      /session.*not found/i,
      /invalid.*session/i,
      /401/i,
      /403/i,
      /unauthorized/i
    ]
  },
  {
    category: "file-corrupt",
    patterns: [
      /corrupt/i,
      /invalid.*pdf/i,
      /pdf.*invalid/i,
      /malformed/i,
      /cannot.*read/i,
      /parse.*error/i,
      /invalid.*document/i,
      /damaged/i
    ]
  },
  {
    category: "authentication",
    patterns: [
      /authentication/i,
      /login/i,
      /credentials/i,
      /identity/i,
      /verification.*failed/i
    ]
  }
];
var FRIENDLY_ERRORS = {
  network: {
    title: "Connection Problem",
    message: "We could not connect to the internet right now. Your document is completely safe and has not been lost. Please check your internet connection and try again when you are back online.",
    action: "Try Again",
    icon: "wifi-off"
  },
  "password-protected": {
    title: "This PDF is Password-Protected",
    message: "This document has a password that prevents us from opening it. Please contact the person who sent you this document and ask them for the password, or request an unprotected version.",
    action: "Enter Password",
    icon: "lock"
  },
  "signature-invalid": {
    title: "Signature Problem",
    message: "We had trouble adding your signature to the document. This sometimes happens if the signature was drawn too quickly. Please try drawing your signature again, taking your time with each stroke.",
    action: "Try Again",
    icon: "signature"
  },
  "session-expired": {
    title: "Signing Link Has Expired",
    message: "The link you used to sign this document is no longer active. This can happen if some time has passed since you received the email. Please contact the sender to request a new signing link.",
    action: "Request New Link",
    icon: "clock"
  },
  "file-corrupt": {
    title: "Document Problem",
    message: "We could not open this document because it may be damaged or in an unsupported format. Please contact the sender and ask them to send the document again.",
    action: "Contact Sender",
    icon: "file"
  },
  authentication: {
    title: "Identity Verification Problem",
    message: "We could not verify your identity to access this document. Please make sure you are using the correct signing link from your email.",
    action: "Check Link",
    icon: "user"
  },
  generic: {
    title: "Something Went Wrong",
    message: "We ran into an unexpected problem, but your document is safe. If this keeps happening, please contact the person who sent you this document for help.",
    action: "Go Back",
    icon: "alert"
  }
};
function categorizeError(error) {
  const message = typeof error === "string" ? error : error.message;
  for (const { patterns, category } of ERROR_PATTERNS) {
    for (const pattern of patterns) {
      if (pattern.test(message)) {
        return category;
      }
    }
  }
  return "generic";
}
function getUserFriendlyError(error) {
  const category = categorizeError(error);
  return { ...FRIENDLY_ERRORS[category] };
}
function createUserError(title, message, action, icon = "alert") {
  return { title, message, action, icon };
}
function getOfflineError() {
  return {
    title: "You Are Offline",
    message: "Your device is not connected to the internet. Your work is saved locally and will be sent automatically when you reconnect. You can continue working offline.",
    action: "Continue",
    icon: "wifi-off"
  };
}
function getFileTooLargeError(maxSizeMb = 25) {
  return {
    title: "File Is Too Large",
    message: `This file is larger than ${maxSizeMb} MB, which is the maximum size we can handle. Please contact the sender and ask them to send a smaller version of the document.`,
    action: "Go Back",
    icon: "file"
  };
}
function getUnsupportedFileError() {
  return {
    title: "Unsupported File Type",
    message: "We can only work with PDF documents. If you received a different type of file, please ask the sender to convert it to PDF format.",
    action: "Go Back",
    icon: "file"
  };
}

// src/ts/error-ui.ts
var ICON_SVGS = {
  "wifi-off": `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <line x1="1" y1="1" x2="23" y2="23"></line>
    <path d="M16.72 11.06A10.94 10.94 0 0 1 19 12.55"></path>
    <path d="M5 12.55a10.94 10.94 0 0 1 5.17-2.39"></path>
    <path d="M10.71 5.05A16 16 0 0 1 22.58 9"></path>
    <path d="M1.42 9a15.91 15.91 0 0 1 4.7-2.88"></path>
    <path d="M8.53 16.11a6 6 0 0 1 6.95 0"></path>
    <line x1="12" y1="20" x2="12.01" y2="20"></line>
  </svg>`,
  lock: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
    <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
  </svg>`,
  signature: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M20 17.5a2.5 2.5 0 0 1-2.5 2.5H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h5.5L14 6.5H20a2 2 0 0 1 2 2v9"></path>
    <path d="M18 17l-3 3 1-6 5-5-3-3-5 5-6 1 3 3z"></path>
  </svg>`,
  clock: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <polyline points="12 6 12 12 16 14"></polyline>
  </svg>`,
  file: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path>
    <polyline points="13 2 13 9 20 9"></polyline>
  </svg>`,
  alert: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <line x1="12" y1="8" x2="12" y2="12"></line>
    <line x1="12" y1="16" x2="12.01" y2="16"></line>
  </svg>`,
  user: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
    <circle cx="12" cy="7" r="4"></circle>
  </svg>`
};
var TOAST_ICONS = {
  error: `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <line x1="15" y1="9" x2="9" y2="15"></line>
    <line x1="9" y1="9" x2="15" y2="15"></line>
  </svg>`,
  warning: `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
    <line x1="12" y1="9" x2="12" y2="13"></line>
    <line x1="12" y1="17" x2="12.01" y2="17"></line>
  </svg>`,
  success: `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
    <polyline points="22 4 12 14.01 9 11.01"></polyline>
  </svg>`
};
var currentModal = null;
var currentToast = null;
var toastTimeout = null;
function createModalOverlay() {
  const overlay = document.createElement("div");
  overlay.className = "modal-overlay error-modal-overlay";
  overlay.setAttribute("role", "dialog");
  overlay.setAttribute("aria-modal", "true");
  return overlay;
}
function createModalContent(error) {
  const content = document.createElement("div");
  content.className = "modal-content error-modal-content confirm-dialog";
  content.setAttribute("role", "alertdialog");
  content.setAttribute("aria-labelledby", "error-modal-title");
  content.setAttribute("aria-describedby", "error-modal-message");
  content.innerHTML = `
    <div class="confirm-icon error-icon" aria-hidden="true">
      ${ICON_SVGS[error.icon]}
    </div>
    <h2 id="error-modal-title" class="confirm-title error-title">${escapeHtml(error.title)}</h2>
    <p id="error-modal-message" class="confirm-message error-modal-message">${escapeHtml(error.message)}</p>
    <div class="confirm-actions error-modal-actions">
      <button type="button" class="btn-primary btn-large error-modal-action" data-action="primary">
        ${escapeHtml(error.action)}
      </button>
      <button type="button" class="btn-secondary error-modal-dismiss" data-action="dismiss">
        Close
      </button>
    </div>
  `;
  return content;
}
function showErrorModal(error, onAction, onDismiss) {
  hideErrorModal();
  const overlay = createModalOverlay();
  const content = createModalContent(error);
  overlay.appendChild(content);
  currentModal = overlay;
  document.body.appendChild(overlay);
  const primaryButton = content.querySelector(
    '[data-action="primary"]'
  );
  const dismissButton = content.querySelector(
    '[data-action="dismiss"]'
  );
  const focusableElements = content.querySelectorAll("button");
  const firstFocusable = focusableElements[0];
  const lastFocusable = focusableElements[focusableElements.length - 1];
  setTimeout(() => {
    primaryButton?.focus();
  }, 100);
  const handlePrimaryClick = () => {
    hideErrorModal();
    onAction?.();
  };
  const handleDismissClick = () => {
    hideErrorModal();
    onDismiss?.();
  };
  const handleOverlayClick = (e) => {
    if (e.target === overlay) {
      hideErrorModal();
      onDismiss?.();
    }
  };
  const handleKeydown = (e) => {
    if (e.key === "Escape") {
      hideErrorModal();
      onDismiss?.();
    }
    if (e.key === "Tab") {
      if (e.shiftKey && document.activeElement === firstFocusable) {
        e.preventDefault();
        lastFocusable?.focus();
      } else if (!e.shiftKey && document.activeElement === lastFocusable) {
        e.preventDefault();
        firstFocusable?.focus();
      }
    }
  };
  primaryButton?.addEventListener("click", handlePrimaryClick);
  dismissButton?.addEventListener("click", handleDismissClick);
  overlay.addEventListener("click", handleOverlayClick);
  document.addEventListener("keydown", handleKeydown);
  overlay._cleanup = () => {
    primaryButton?.removeEventListener("click", handlePrimaryClick);
    dismissButton?.removeEventListener("click", handleDismissClick);
    overlay.removeEventListener("click", handleOverlayClick);
    document.removeEventListener("keydown", handleKeydown);
  };
}
function hideErrorModal() {
  if (currentModal) {
    const cleanup = currentModal._cleanup;
    cleanup?.();
    currentModal.remove();
    currentModal = null;
  }
}
function showErrorToast(message, type = "error", duration = 5e3) {
  hideErrorToast();
  const toast = document.createElement("div");
  toast.className = `error-toast error-toast-${type}`;
  toast.setAttribute("role", "alert");
  toast.setAttribute("aria-live", type === "error" ? "assertive" : "polite");
  const alertClass = type === "error" ? "alert-error" : type === "warning" ? "alert-warning" : "alert-success";
  toast.innerHTML = `
    <div class="error-toast-content ${alertClass}">
      <span class="error-toast-icon" aria-hidden="true">
        ${TOAST_ICONS[type]}
      </span>
      <span class="error-toast-message">${escapeHtml(message)}</span>
      <button type="button" class="error-toast-close" aria-label="Close notification">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    </div>
  `;
  toast.style.cssText = `
    position: fixed;
    bottom: var(--spacing-lg, 32px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 2000;
    max-width: calc(100% - var(--spacing-lg, 32px) * 2);
    width: 100%;
    max-width: 600px;
    animation: error-toast-slide-up 0.3s ease-out;
  `;
  const content = toast.querySelector(".error-toast-content");
  if (content) {
    content.style.cssText = `
      display: flex;
      align-items: center;
      gap: var(--spacing-sm, 16px);
      padding: var(--spacing-md, 24px);
      border-radius: var(--border-radius-lg, 12px);
      font-size: var(--font-size-lg, 22px);
      font-weight: 500;
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    `;
  }
  const closeButton = toast.querySelector(".error-toast-close");
  if (closeButton) {
    closeButton.style.cssText = `
      background: none;
      border: none;
      cursor: pointer;
      padding: 8px;
      margin-left: auto;
      opacity: 0.7;
      transition: opacity 0.2s;
      color: inherit;
      min-width: 44px;
      min-height: 44px;
      display: flex;
      align-items: center;
      justify-content: center;
    `;
    closeButton.addEventListener("click", hideErrorToast);
    closeButton.addEventListener("mouseenter", () => {
      closeButton.style.opacity = "1";
    });
    closeButton.addEventListener("mouseleave", () => {
      closeButton.style.opacity = "0.7";
    });
  }
  if (!document.getElementById("error-toast-styles")) {
    const style = document.createElement("style");
    style.id = "error-toast-styles";
    style.textContent = `
      @keyframes error-toast-slide-up {
        from {
          transform: translateX(-50%) translateY(100%);
          opacity: 0;
        }
        to {
          transform: translateX(-50%) translateY(0);
          opacity: 1;
        }
      }
      @keyframes error-toast-slide-down {
        from {
          transform: translateX(-50%) translateY(0);
          opacity: 1;
        }
        to {
          transform: translateX(-50%) translateY(100%);
          opacity: 0;
        }
      }
    `;
    document.head.appendChild(style);
  }
  document.body.appendChild(toast);
  currentToast = toast;
  toastTimeout = setTimeout(() => {
    hideErrorToast();
  }, duration);
}
function hideErrorToast() {
  if (toastTimeout) {
    clearTimeout(toastTimeout);
    toastTimeout = null;
  }
  if (currentToast) {
    currentToast.style.animation = "error-toast-slide-down 0.3s ease-in forwards";
    const toastToRemove = currentToast;
    currentToast = null;
    setTimeout(() => {
      toastToRemove.remove();
    }, 300);
  }
}
function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}
function showConfirmDialog(title, message, confirmText = "Yes", cancelText = "No") {
  return new Promise((resolve) => {
    const userError = {
      title,
      message,
      action: confirmText,
      icon: "alert"
    };
    hideErrorModal();
    const overlay = createModalOverlay();
    const content = document.createElement("div");
    content.className = "modal-content error-modal-content confirm-dialog";
    content.setAttribute("role", "alertdialog");
    content.setAttribute("aria-labelledby", "confirm-modal-title");
    content.setAttribute("aria-describedby", "confirm-modal-message");
    content.innerHTML = `
      <div class="confirm-icon" aria-hidden="true">
        ${ICON_SVGS.alert}
      </div>
      <h2 id="confirm-modal-title" class="confirm-title">${escapeHtml(title)}</h2>
      <p id="confirm-modal-message" class="confirm-message">${escapeHtml(message)}</p>
      <div class="confirm-actions">
        <button type="button" class="btn-primary btn-large" data-action="confirm">
          ${escapeHtml(confirmText)}
        </button>
        <button type="button" class="btn-secondary" data-action="cancel">
          ${escapeHtml(cancelText)}
        </button>
      </div>
    `;
    overlay.appendChild(content);
    currentModal = overlay;
    document.body.appendChild(overlay);
    const confirmButton = content.querySelector(
      '[data-action="confirm"]'
    );
    const cancelButton = content.querySelector(
      '[data-action="cancel"]'
    );
    setTimeout(() => {
      confirmButton?.focus();
    }, 100);
    const cleanup = () => {
      hideErrorModal();
    };
    confirmButton?.addEventListener("click", () => {
      cleanup();
      resolve(true);
    });
    cancelButton?.addEventListener("click", () => {
      cleanup();
      resolve(false);
    });
    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) {
        cleanup();
        resolve(false);
      }
    });
    const handleKeydown = (e) => {
      if (e.key === "Escape") {
        document.removeEventListener("keydown", handleKeydown);
        cleanup();
        resolve(false);
      }
    };
    document.addEventListener("keydown", handleKeydown);
  });
}

// src/ts/session.ts
var OFFLINE_QUEUE_KEY = "docsign_offline_queue";
function deserializeQueuedSubmission(json) {
  const parsed = JSON.parse(json);
  if (typeof parsed.sessionId !== "string") {
    throw new Error("Invalid sessionId in queued submission");
  }
  if (typeof parsed.recipientId !== "string") {
    throw new Error("Invalid recipientId in queued submission");
  }
  if (typeof parsed.signingKey !== "string") {
    throw new Error("Invalid signingKey in queued submission");
  }
  if (typeof parsed.signatures !== "object" || parsed.signatures === null) {
    throw new Error("Invalid signatures in queued submission");
  }
  if (typeof parsed.completedAt !== "string") {
    throw new Error("Invalid completedAt in queued submission");
  }
  if (typeof parsed.timestamp !== "number") {
    throw new Error("Invalid timestamp in queued submission");
  }
  return parsed;
}
function getOfflineQueue() {
  if (typeof localStorage === "undefined") {
    return [];
  }
  const json = localStorage.getItem(OFFLINE_QUEUE_KEY);
  if (!json) {
    return [];
  }
  try {
    const parsed = JSON.parse(json);
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed.map(deserializeQueuedSubmission);
  } catch {
    return [];
  }
}
function removeFromOfflineQueue(sessionId, recipientId) {
  if (typeof localStorage === "undefined") {
    return;
  }
  const queue = getOfflineQueue();
  const filtered = queue.filter((s) => !(s.sessionId === sessionId && s.recipientId === recipientId));
  localStorage.setItem(OFFLINE_QUEUE_KEY, JSON.stringify(filtered));
}

// src/ts/sync-events.ts
var SYNC_EVENTS = {
  STARTED: "docsign:sync-started",
  COMPLETED: "docsign:sync-completed",
  FAILED: "docsign:sync-failed",
  PROGRESS: "docsign:sync-progress",
  ONLINE_STATUS_CHANGED: "docsign:online-status-changed"
};
function dispatchSyncStarted(detail) {
  const event = new CustomEvent(SYNC_EVENTS.STARTED, {
    detail,
    bubbles: true
  });
  window.dispatchEvent(event);
}
function dispatchSyncCompleted(detail) {
  const event = new CustomEvent(SYNC_EVENTS.COMPLETED, {
    detail,
    bubbles: true
  });
  window.dispatchEvent(event);
}
function dispatchSyncFailed(detail) {
  const event = new CustomEvent(SYNC_EVENTS.FAILED, {
    detail,
    bubbles: true
  });
  window.dispatchEvent(event);
}
function dispatchSyncProgress(detail) {
  const event = new CustomEvent(SYNC_EVENTS.PROGRESS, {
    detail,
    bubbles: true
  });
  window.dispatchEvent(event);
}
function dispatchOnlineStatusChanged(detail) {
  const event = new CustomEvent(SYNC_EVENTS.ONLINE_STATUS_CHANGED, {
    detail,
    bubbles: true
  });
  window.dispatchEvent(event);
}
function onSyncStarted(callback) {
  const handler = (e) => {
    callback(e.detail);
  };
  window.addEventListener(SYNC_EVENTS.STARTED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.STARTED, handler);
}
function onSyncCompleted(callback) {
  const handler = (e) => {
    callback(e.detail);
  };
  window.addEventListener(SYNC_EVENTS.COMPLETED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.COMPLETED, handler);
}
function onSyncFailed(callback) {
  const handler = (e) => {
    callback(e.detail);
  };
  window.addEventListener(SYNC_EVENTS.FAILED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.FAILED, handler);
}
function onSyncProgress(callback) {
  const handler = (e) => {
    callback(e.detail);
  };
  window.addEventListener(SYNC_EVENTS.PROGRESS, handler);
  return () => window.removeEventListener(SYNC_EVENTS.PROGRESS, handler);
}
function onOnlineStatusChanged(callback) {
  const handler = (e) => {
    callback(e.detail);
  };
  window.addEventListener(SYNC_EVENTS.ONLINE_STATUS_CHANGED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.ONLINE_STATUS_CHANGED, handler);
}

// src/ts/sync-manager.ts
var SYNC_STATE_KEY = "docsign_sync_state";
var SYNC_ERRORS_KEY = "docsign_sync_errors";
var OFFLINE_MODE_KEY = "docsign_offline_mode";
function loadPersistedState() {
  if (typeof localStorage === "undefined") {
    return { lastSyncAttempt: null, lastSuccessfulSync: null };
  }
  try {
    const json = localStorage.getItem(SYNC_STATE_KEY);
    if (json) {
      return JSON.parse(json);
    }
  } catch {
  }
  return { lastSyncAttempt: null, lastSuccessfulSync: null };
}
function savePersistedState(state) {
  if (typeof localStorage === "undefined") {
    return;
  }
  localStorage.setItem(SYNC_STATE_KEY, JSON.stringify(state));
}
function loadSyncErrors() {
  if (typeof localStorage === "undefined") {
    return [];
  }
  try {
    const json = localStorage.getItem(SYNC_ERRORS_KEY);
    if (json) {
      return JSON.parse(json);
    }
  } catch {
  }
  return [];
}
function saveSyncErrors(errors) {
  if (typeof localStorage === "undefined") {
    return;
  }
  localStorage.setItem(SYNC_ERRORS_KEY, JSON.stringify(errors));
}
function isExplicitOfflineMode() {
  if (typeof localStorage === "undefined") {
    return false;
  }
  return localStorage.getItem(OFFLINE_MODE_KEY) === "true";
}
var SyncManager = class {
  constructor(config) {
    this.isSyncing = false;
    this.isStarted = false;
    this.retryTimeoutId = null;
    this.periodicRetryId = null;
    // ============================================================
    // Private Methods
    // ============================================================
    this.handleOnline = () => {
      console.log("[SyncManager] Device came online");
      dispatchOnlineStatusChanged({
        online: true,
        timestamp: (/* @__PURE__ */ new Date()).toISOString()
      });
      if (!isExplicitOfflineMode()) {
        setTimeout(() => this.syncNow(), 1e3);
      }
    };
    this.handleOffline = () => {
      console.log("[SyncManager] Device went offline");
      dispatchOnlineStatusChanged({
        online: false,
        timestamp: (/* @__PURE__ */ new Date()).toISOString()
      });
    };
    this.config = {
      syncEndpoint: config.syncEndpoint,
      minBackoffMs: config.minBackoffMs ?? 1e3,
      maxBackoffMs: config.maxBackoffMs ?? 3e4,
      retryIntervalMs: config.retryIntervalMs ?? 3e4,
      maxRetries: config.maxRetries ?? 10
    };
    this.persistedState = loadPersistedState();
    this.errors = loadSyncErrors();
  }
  /**
   * Begin monitoring online status and syncing
   */
  start() {
    if (this.isStarted) {
      console.log("[SyncManager] Already started");
      return;
    }
    this.isStarted = true;
    console.log("[SyncManager] Starting sync manager");
    window.addEventListener("online", this.handleOnline);
    window.addEventListener("offline", this.handleOffline);
    if (navigator.onLine && !isExplicitOfflineMode()) {
      this.syncNow();
    }
    this.startPeriodicRetry();
  }
  /**
   * Stop monitoring and syncing
   */
  stop() {
    if (!this.isStarted) {
      return;
    }
    this.isStarted = false;
    console.log("[SyncManager] Stopping sync manager");
    window.removeEventListener("online", this.handleOnline);
    window.removeEventListener("offline", this.handleOffline);
    if (this.retryTimeoutId) {
      clearTimeout(this.retryTimeoutId);
      this.retryTimeoutId = null;
    }
    if (this.periodicRetryId) {
      clearInterval(this.periodicRetryId);
      this.periodicRetryId = null;
    }
  }
  /**
   * Force an immediate sync attempt
   */
  async syncNow() {
    if (isExplicitOfflineMode()) {
      console.log("[SyncManager] Skipping sync - explicit offline mode");
      return;
    }
    if (!navigator.onLine) {
      console.log("[SyncManager] Skipping sync - offline");
      return;
    }
    if (this.isSyncing) {
      console.log("[SyncManager] Skipping sync - already in progress");
      return;
    }
    const queue = getOfflineQueue();
    if (queue.length === 0) {
      console.log("[SyncManager] Nothing to sync");
      return;
    }
    this.isSyncing = true;
    const startTime = Date.now();
    const timestamp = (/* @__PURE__ */ new Date()).toISOString();
    this.persistedState.lastSyncAttempt = timestamp;
    savePersistedState(this.persistedState);
    dispatchSyncStarted({
      pendingCount: queue.length,
      timestamp
    });
    console.log(`[SyncManager] Starting sync of ${queue.length} items`);
    let syncedCount = 0;
    for (let i = 0; i < queue.length; i++) {
      const item = queue[i];
      dispatchSyncProgress({
        current: i + 1,
        total: queue.length,
        sessionId: item.sessionId,
        percentage: Math.round((i + 1) / queue.length * 100)
      });
      const success = await this.syncItem(item);
      if (success) {
        syncedCount++;
      }
    }
    this.isSyncing = false;
    const completedTimestamp = (/* @__PURE__ */ new Date()).toISOString();
    const durationMs = Date.now() - startTime;
    if (syncedCount === queue.length) {
      this.persistedState.lastSuccessfulSync = completedTimestamp;
      savePersistedState(this.persistedState);
    }
    dispatchSyncCompleted({
      syncedCount,
      timestamp: completedTimestamp,
      durationMs
    });
    console.log(`[SyncManager] Sync completed: ${syncedCount}/${queue.length} items`);
  }
  /**
   * Get current sync status
   */
  getStatus() {
    const queue = getOfflineQueue();
    return {
      pendingCount: queue.length,
      lastSyncAttempt: this.persistedState.lastSyncAttempt,
      lastSuccessfulSync: this.persistedState.lastSuccessfulSync,
      isSyncing: this.isSyncing,
      isOnline: navigator.onLine,
      errors: [...this.errors]
    };
  }
  /**
   * Clear error history
   */
  clearErrors() {
    this.errors = [];
    saveSyncErrors(this.errors);
    console.log("[SyncManager] Errors cleared");
  }
  /**
   * Set explicit offline mode
   * When enabled, sync will not happen automatically
   */
  setOfflineMode(enabled) {
    if (typeof localStorage === "undefined") {
      return;
    }
    if (enabled) {
      localStorage.setItem(OFFLINE_MODE_KEY, "true");
      console.log("[SyncManager] Offline mode enabled");
    } else {
      localStorage.removeItem(OFFLINE_MODE_KEY);
      console.log("[SyncManager] Offline mode disabled");
      if (navigator.onLine) {
        this.syncNow();
      }
    }
  }
  /**
   * Check if explicit offline mode is enabled
   */
  isOfflineModeEnabled() {
    return isExplicitOfflineMode();
  }
  /**
   * Notify that a new signature was saved locally
   * This triggers a sync attempt if conditions are met
   */
  notifyNewSignature() {
    console.log("[SyncManager] New signature saved, checking for sync");
    if (navigator.onLine && !isExplicitOfflineMode() && !this.isSyncing) {
      setTimeout(() => this.syncNow(), 500);
    }
  }
  startPeriodicRetry() {
    if (this.periodicRetryId) {
      return;
    }
    this.periodicRetryId = setInterval(() => {
      if (navigator.onLine && !isExplicitOfflineMode() && !this.isSyncing) {
        const queue = getOfflineQueue();
        if (queue.length > 0) {
          console.log("[SyncManager] Periodic retry triggered");
          this.syncNow();
        }
      }
    }, this.config.retryIntervalMs);
  }
  async syncItem(item) {
    const errorKey = `${item.sessionId}:${item.recipientId}`;
    const existingError = this.errors.find(
      (e) => e.sessionId === item.sessionId && e.recipientId === item.recipientId
    );
    const attemptCount = (existingError?.attemptCount ?? 0) + 1;
    if (attemptCount > this.config.maxRetries) {
      console.log(`[SyncManager] Max retries exceeded for ${errorKey}, skipping`);
      return false;
    }
    try {
      const response = await this.postSignature(item);
      if (response.ok) {
        removeFromOfflineQueue(item.sessionId, item.recipientId);
        this.removeError(item.sessionId, item.recipientId);
        console.log(`[SyncManager] Successfully synced ${errorKey}`);
        return true;
      }
      if (response.status === 409) {
        const serverData = await response.json();
        await this.handleConflict(item, serverData);
        return true;
      }
      const errorText = await response.text();
      this.recordError(item, `Server error ${response.status}: ${errorText}`, attemptCount);
      this.scheduleRetry(item, attemptCount);
      return false;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      console.error(`[SyncManager] Failed to sync ${errorKey}:`, errorMessage);
      this.recordError(item, errorMessage, attemptCount);
      this.scheduleRetry(item, attemptCount);
      return false;
    }
  }
  async postSignature(item) {
    return fetch(this.config.syncEndpoint, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        sessionId: item.sessionId,
        recipientId: item.recipientId,
        signingKey: item.signingKey,
        signatures: item.signatures,
        completedAt: item.completedAt,
        clientTimestamp: item.timestamp
      })
    });
  }
  async handleConflict(item, serverData) {
    console.log(`[SyncManager] Handling conflict for ${item.sessionId}`);
    if (serverData.serverTimestamp && serverData.serverTimestamp > item.timestamp) {
      console.log("[SyncManager] Server has newer data, preferring server");
      removeFromOfflineQueue(item.sessionId, item.recipientId);
      return;
    }
    if (serverData.signatures && item.signatures) {
      console.log("[SyncManager] Merging local signatures with server");
      const mergedSignatures = {
        ...serverData.signatures,
        ...item.signatures
        // Local takes precedence for newer
      };
      const mergedItem = {
        ...item,
        signatures: mergedSignatures
      };
      const retryResponse = await this.postSignature(mergedItem);
      if (retryResponse.ok) {
        removeFromOfflineQueue(item.sessionId, item.recipientId);
        console.log("[SyncManager] Conflict resolved with merge");
      }
    } else {
      removeFromOfflineQueue(item.sessionId, item.recipientId);
    }
    this.removeError(item.sessionId, item.recipientId);
  }
  recordError(item, error, attemptCount) {
    const timestamp = (/* @__PURE__ */ new Date()).toISOString();
    this.removeError(item.sessionId, item.recipientId);
    const syncError = {
      sessionId: item.sessionId,
      recipientId: item.recipientId,
      error,
      attemptCount,
      lastAttempt: timestamp
    };
    this.errors.push(syncError);
    saveSyncErrors(this.errors);
    dispatchSyncFailed({
      sessionId: item.sessionId,
      error,
      attemptCount,
      timestamp,
      willRetry: attemptCount < this.config.maxRetries
    });
  }
  removeError(sessionId, recipientId) {
    const index = this.errors.findIndex(
      (e) => e.sessionId === sessionId && e.recipientId === recipientId
    );
    if (index !== -1) {
      this.errors.splice(index, 1);
      saveSyncErrors(this.errors);
    }
  }
  scheduleRetry(item, attemptCount) {
    if (attemptCount >= this.config.maxRetries) {
      return;
    }
    const delay = Math.min(
      this.config.minBackoffMs * Math.pow(2, attemptCount - 1),
      this.config.maxBackoffMs
    );
    console.log(
      `[SyncManager] Scheduling retry for ${item.sessionId} in ${delay}ms (attempt ${attemptCount})`
    );
  }
};
var syncManagerInstance = null;
function getSyncManager(config) {
  if (!syncManagerInstance) {
    if (!config) {
      throw new Error("SyncManager not initialized. Call getSyncManager with config first.");
    }
    syncManagerInstance = new SyncManager(config);
  }
  return syncManagerInstance;
}
function initSyncManager(config) {
  const manager = getSyncManager(config);
  manager.start();
  return manager;
}

// src/ts/sign-pdf-bridge.ts
function base64ToUint8Array(base64) {
  const cleanBase64 = base64.replace(/^data:[^;]+;base64,/, "");
  const binaryString = atob(cleanBase64);
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes;
}
function createDocSignPdfBridge() {
  let pageCount = 0;
  return {
    async loadPdf(data) {
      try {
        PdfPreviewBridge.cleanup();
        let bytes;
        if (typeof data === "string") {
          bytes = base64ToUint8Array(data);
        } else {
          bytes = data;
        }
        pageCount = await PdfPreviewBridge.loadDocument(bytes);
        console.log("[DocSign] PDF loaded:", pageCount, "pages");
        return {
          numPages: pageCount,
          success: true
        };
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        console.error("[DocSign] Failed to load PDF:", errorMessage);
        pageCount = 0;
        return {
          numPages: 0,
          success: false,
          error: errorMessage
        };
      }
    },
    async renderAllPages(config) {
      const { container, scale = 1.5, pageWrapperClass = "pdf-page-wrapper" } = config;
      const results = [];
      if (!PdfPreviewBridge.currentDoc) {
        console.error("[DocSign] No document loaded");
        return results;
      }
      container.innerHTML = "";
      for (let pageNum = 1; pageNum <= pageCount; pageNum++) {
        const pageWrapper = document.createElement("div");
        pageWrapper.className = pageWrapperClass;
        pageWrapper.dataset.pageNumber = String(pageNum);
        const canvas = document.createElement("canvas");
        pageWrapper.appendChild(canvas);
        container.appendChild(pageWrapper);
        const result = await this.renderPage(pageNum, canvas, scale);
        results.push(result);
      }
      console.log("[DocSign] Rendered", results.length, "pages");
      return results;
    },
    async renderPage(pageNum, canvas, scale = 1.5) {
      try {
        const dimensions = await PdfPreviewBridge.renderPage(pageNum, canvas, scale);
        return {
          pageNum,
          dimensions,
          canvas,
          success: true
        };
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        console.error(`[DocSign] Failed to render page ${pageNum}:`, errorMessage);
        return {
          pageNum,
          dimensions: {
            width: 0,
            height: 0,
            originalWidth: 0,
            originalHeight: 0,
            pdfWidth: 0,
            pdfHeight: 0
          },
          canvas,
          success: false,
          error: errorMessage
        };
      }
    },
    getPageCount() {
      return pageCount;
    },
    getPageDimensions(pageNum) {
      return PdfPreviewBridge.getPageDimensions(pageNum);
    },
    cleanup() {
      PdfPreviewBridge.cleanup();
      pageCount = 0;
      console.log("[DocSign] Cleaned up PDF resources");
    },
    isDocumentLoaded() {
      return PdfPreviewBridge.currentDoc !== null;
    }
  };
}
var docSignPdfBridge = createDocSignPdfBridge();
function initDocSignNamespace() {
  window.DocSign = {
    // PDF bridge functions
    loadPdf: docSignPdfBridge.loadPdf.bind(docSignPdfBridge),
    renderAllPages: docSignPdfBridge.renderAllPages.bind(docSignPdfBridge),
    renderPage: docSignPdfBridge.renderPage.bind(docSignPdfBridge),
    getPageCount: docSignPdfBridge.getPageCount.bind(docSignPdfBridge),
    getPageDimensions: docSignPdfBridge.getPageDimensions.bind(docSignPdfBridge),
    cleanup: docSignPdfBridge.cleanup.bind(docSignPdfBridge),
    isDocumentLoaded: docSignPdfBridge.isDocumentLoaded.bind(docSignPdfBridge),
    // Error message functions
    getUserFriendlyError,
    categorizeError,
    createUserError,
    getOfflineError,
    getFileTooLargeError,
    getUnsupportedFileError,
    // Error UI functions
    showErrorModal,
    hideErrorModal,
    showErrorToast,
    hideErrorToast,
    showConfirmDialog,
    // Sync Manager
    SyncManager,
    getSyncManager,
    initSyncManager,
    SYNC_EVENTS,
    onSyncStarted,
    onSyncCompleted,
    onSyncFailed,
    onSyncProgress,
    onOnlineStatusChanged
  };
  console.log("[DocSign] PDF bridge, error handling, and sync manager initialized on window.DocSign");
}

// src/ts/local-session-manager.ts
var DB_NAME = "docsign_local";
var DB_VERSION = 1;
var STORES = {
  SESSIONS: "sessions",
  PDF_CACHE: "pdf_cache",
  SIGNATURE_QUEUE: "signature_queue"
};
function openDatabase() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onerror = () => {
      console.error("[LocalSessionManager] Failed to open database:", request.error);
      reject(request.error);
    };
    request.onsuccess = () => {
      resolve(request.result);
    };
    request.onupgradeneeded = (event) => {
      const db = event.target.result;
      if (!db.objectStoreNames.contains(STORES.SESSIONS)) {
        const sessionsStore = db.createObjectStore(STORES.SESSIONS, { keyPath: "sessionId" });
        sessionsStore.createIndex("recipientId", "recipientId", { unique: false });
        sessionsStore.createIndex("status", "status", { unique: false });
        sessionsStore.createIndex("createdAt", "createdAt", { unique: false });
      }
      if (!db.objectStoreNames.contains(STORES.PDF_CACHE)) {
        db.createObjectStore(STORES.PDF_CACHE, { keyPath: "sessionId" });
      }
      if (!db.objectStoreNames.contains(STORES.SIGNATURE_QUEUE)) {
        const queueStore = db.createObjectStore(STORES.SIGNATURE_QUEUE, { keyPath: ["sessionId", "recipientId"] });
        queueStore.createIndex("timestamp", "timestamp", { unique: false });
      }
      console.log("[LocalSessionManager] Database schema created/upgraded");
    };
  });
}
async function getFromStore(storeName, key) {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(storeName, "readonly");
    const store = transaction.objectStore(storeName);
    const request = store.get(key);
    request.onsuccess = () => {
      db.close();
      resolve(request.result);
    };
    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}
async function putToStore(storeName, value) {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(storeName, "readwrite");
    const store = transaction.objectStore(storeName);
    const request = store.put(value);
    request.onsuccess = () => {
      db.close();
      resolve();
    };
    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}
async function deleteFromStore(storeName, key) {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(storeName, "readwrite");
    const store = transaction.objectStore(storeName);
    const request = store.delete(key);
    request.onsuccess = () => {
      db.close();
      resolve();
    };
    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}
async function getAllFromStore(storeName) {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(storeName, "readonly");
    const store = transaction.objectStore(storeName);
    const request = store.getAll();
    request.onsuccess = () => {
      db.close();
      resolve(request.result);
    };
    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}
var LocalSessionManager = class {
  /**
   * Get a session from local storage
   * @param sessionId Session ID to retrieve
   * @param _recipientId Optional recipient ID to filter fields (reserved for future use)
   * @returns The session if found, undefined otherwise
   */
  static async getSession(sessionId, _recipientId) {
    try {
      const session = await getFromStore(STORES.SESSIONS, sessionId);
      if (!session) {
        console.log("[LocalSessionManager] Session not found locally:", sessionId);
        return void 0;
      }
      if (session.expiresAt) {
        const expiresAt = new Date(session.expiresAt).getTime();
        if (Date.now() > expiresAt) {
          console.log("[LocalSessionManager] Session expired:", sessionId);
          session.status = "expired";
          return session;
        }
      }
      console.log("[LocalSessionManager] Session found locally:", sessionId);
      return session;
    } catch (err) {
      console.error("[LocalSessionManager] Error getting session:", err);
      return void 0;
    }
  }
  /**
   * Save a session to local storage
   * @param session The session to save
   */
  static async saveSession(session) {
    try {
      await putToStore(STORES.SESSIONS, session);
      console.log("[LocalSessionManager] Session saved:", session.sessionId);
    } catch (err) {
      console.error("[LocalSessionManager] Error saving session:", err);
      throw err;
    }
  }
  /**
   * Cache a server response for offline use
   * @param serverResponse The server response to cache
   */
  static async cacheSession(serverResponse) {
    try {
      const session = {
        sessionId: String(serverResponse.sessionId || serverResponse.session_id || ""),
        recipientId: String(serverResponse.recipientId || serverResponse.recipient_id || ""),
        documentName: String(serverResponse.documentName || serverResponse.document_name || "Document"),
        metadata: serverResponse.metadata,
        fields: serverResponse.fields || [],
        recipients: serverResponse.recipients,
        status: serverResponse.status || "pending",
        createdAt: String(serverResponse.createdAt || serverResponse.created_at || (/* @__PURE__ */ new Date()).toISOString()),
        expiresAt: serverResponse.expiresAt || serverResponse.expires_at,
        isServerCached: true,
        lastSyncedAt: (/* @__PURE__ */ new Date()).toISOString()
      };
      await this.saveSession(session);
      console.log("[LocalSessionManager] Server session cached:", session.sessionId);
    } catch (err) {
      console.error("[LocalSessionManager] Error caching server session:", err);
    }
  }
  /**
   * Cache PDF data for a session
   * @param sessionId Session ID
   * @param pdfData PDF data as base64 string or Uint8Array
   */
  static async cachePdfData(sessionId, pdfData) {
    try {
      let base64Data;
      if (pdfData instanceof Uint8Array) {
        const binary = String.fromCharCode(...pdfData);
        base64Data = btoa(binary);
      } else {
        base64Data = pdfData;
      }
      await putToStore(STORES.PDF_CACHE, {
        sessionId,
        pdfData: base64Data,
        cachedAt: (/* @__PURE__ */ new Date()).toISOString()
      });
      console.log("[LocalSessionManager] PDF data cached for session:", sessionId);
    } catch (err) {
      console.error("[LocalSessionManager] Error caching PDF data:", err);
    }
  }
  /**
   * Get cached PDF data for a session
   * @param sessionId Session ID
   * @returns PDF data as base64 string, or undefined if not cached
   */
  static async getCachedPdfData(sessionId) {
    try {
      const cached = await getFromStore(
        STORES.PDF_CACHE,
        sessionId
      );
      return cached?.pdfData;
    } catch (err) {
      console.error("[LocalSessionManager] Error getting cached PDF:", err);
      return void 0;
    }
  }
  /**
   * Save signatures to the session
   * @param sessionId Session ID
   * @param signatures Signatures record
   */
  static async saveSignatures(sessionId, signatures) {
    try {
      const session = await this.getSession(sessionId);
      if (session) {
        session.signatures = { ...session.signatures, ...signatures };
        session.status = "in_progress";
        await this.saveSession(session);
        console.log("[LocalSessionManager] Signatures saved for session:", sessionId);
      } else {
        console.warn("[LocalSessionManager] Cannot save signatures - session not found:", sessionId);
      }
    } catch (err) {
      console.error("[LocalSessionManager] Error saving signatures:", err);
      throw err;
    }
  }
  /**
   * Mark session as completed
   * @param sessionId Session ID
   */
  static async completeSession(sessionId) {
    try {
      const session = await this.getSession(sessionId);
      if (session) {
        session.status = "completed";
        await this.saveSession(session);
        console.log("[LocalSessionManager] Session completed:", sessionId);
      }
    } catch (err) {
      console.error("[LocalSessionManager] Error completing session:", err);
      throw err;
    }
  }
  /**
   * Queue a signature submission for later sync
   * @param submission The queued signature submission
   */
  static async queueForSync(submission) {
    try {
      await putToStore(STORES.SIGNATURE_QUEUE, submission);
      console.log("[LocalSessionManager] Submission queued for sync:", submission.sessionId);
    } catch (err) {
      console.error("[LocalSessionManager] Error queueing submission:", err);
      throw err;
    }
  }
  /**
   * Get all queued submissions
   * @returns Array of queued submissions
   */
  static async getQueuedSubmissions() {
    try {
      return await getAllFromStore(STORES.SIGNATURE_QUEUE);
    } catch (err) {
      console.error("[LocalSessionManager] Error getting queued submissions:", err);
      return [];
    }
  }
  /**
   * Remove a submission from the queue
   * @param sessionId Session ID
   * @param recipientId Recipient ID
   */
  static async removeFromQueue(sessionId, recipientId) {
    try {
      await deleteFromStore(STORES.SIGNATURE_QUEUE, [sessionId, recipientId]);
      console.log("[LocalSessionManager] Removed from queue:", sessionId, recipientId);
    } catch (err) {
      console.error("[LocalSessionManager] Error removing from queue:", err);
    }
  }
  /**
   * Delete a session from local storage
   * @param sessionId Session ID to delete
   */
  static async deleteSession(sessionId) {
    try {
      await deleteFromStore(STORES.SESSIONS, sessionId);
      await deleteFromStore(STORES.PDF_CACHE, sessionId);
      console.log("[LocalSessionManager] Session deleted:", sessionId);
    } catch (err) {
      console.error("[LocalSessionManager] Error deleting session:", err);
    }
  }
  /**
   * Get all sessions for a recipient
   * @param recipientId Recipient ID
   * @returns Array of sessions
   */
  static async getSessionsForRecipient(recipientId) {
    try {
      const allSessions = await getAllFromStore(STORES.SESSIONS);
      return allSessions.filter((s) => s.recipientId === recipientId);
    } catch (err) {
      console.error("[LocalSessionManager] Error getting sessions for recipient:", err);
      return [];
    }
  }
  /**
   * Clear all local data (for testing/debugging)
   */
  static async clearAll() {
    try {
      const db = await openDatabase();
      await new Promise((resolve, reject) => {
        const transaction = db.transaction(
          [STORES.SESSIONS, STORES.PDF_CACHE, STORES.SIGNATURE_QUEUE],
          "readwrite"
        );
        transaction.objectStore(STORES.SESSIONS).clear();
        transaction.objectStore(STORES.PDF_CACHE).clear();
        transaction.objectStore(STORES.SIGNATURE_QUEUE).clear();
        transaction.oncomplete = () => {
          db.close();
          resolve();
        };
        transaction.onerror = () => {
          db.close();
          reject(transaction.error);
        };
      });
      console.log("[LocalSessionManager] All local data cleared");
    } catch (err) {
      console.error("[LocalSessionManager] Error clearing all data:", err);
    }
  }
  /**
   * Check if IndexedDB is available
   * @returns true if IndexedDB is supported
   */
  static isAvailable() {
    return typeof indexedDB !== "undefined";
  }
};
var LocalSessionManagerInstance = class {
  async createSession(document2, recipients, fields = []) {
    const sessionId = crypto.randomUUID();
    const now = (/* @__PURE__ */ new Date()).toISOString();
    const session = {
      sessionId,
      recipientId: recipients[0]?.id?.toString() || "",
      documentName: "Untitled Document",
      fields,
      recipients,
      status: "pending",
      createdAt: now,
      expiresAt: null
    };
    if (document2.length > 0) {
      await LocalSessionManager.cachePdfData(sessionId, document2);
    }
    await LocalSessionManager.saveSession(session);
    return {
      id: sessionId,
      recipients,
      fields,
      status: "pending",
      createdAt: now,
      expiresAt: null
    };
  }
  async getSession(sessionId) {
    const localSession = await LocalSessionManager.getSession(sessionId);
    if (!localSession) return null;
    return {
      id: localSession.sessionId,
      recipients: localSession.recipients || [],
      fields: localSession.fields,
      status: localSession.status,
      createdAt: localSession.createdAt,
      expiresAt: localSession.expiresAt || null
    };
  }
  async updateSessionStatus(sessionId, status) {
    const session = await LocalSessionManager.getSession(sessionId);
    if (session) {
      session.status = status;
      await LocalSessionManager.saveSession(session);
    }
  }
  async recordSignature(sessionId, fieldId, signatureData, type = "draw", recipientId = "") {
    const session = await LocalSessionManager.getSession(sessionId);
    if (session) {
      const signatures = session.signatures || {};
      signatures[fieldId] = {
        fieldId,
        type,
        data: signatureData,
        timestamp: (/* @__PURE__ */ new Date()).toISOString(),
        recipientId
      };
      await LocalSessionManager.saveSignatures(sessionId, signatures);
    }
  }
  async getSignedDocument(sessionId) {
    const pdfData = await LocalSessionManager.getCachedPdfData(sessionId);
    if (!pdfData) return null;
    const binary = atob(pdfData);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes;
  }
  async deleteSession(sessionId) {
    await LocalSessionManager.deleteSession(sessionId);
  }
  async listSessions() {
    const sessions = await getAllFromStore(STORES.SESSIONS);
    return sessions.map((s) => ({
      id: s.sessionId,
      recipientCount: s.recipients?.length || 0,
      fieldCount: s.fields.length,
      completedFieldCount: Object.keys(s.signatures || {}).length,
      status: s.status,
      createdAt: s.createdAt,
      expiresAt: s.expiresAt || null
    }));
  }
};
var localSessionManager = new LocalSessionManagerInstance();
function initLocalSessionNamespace() {
  if (!window.DocSign) {
    window.DocSign = {};
  }
  const docSign = window.DocSign;
  docSign.LocalSessionManager = LocalSessionManager;
  docSign.localSessionManager = localSessionManager;
  docSign.createSession = localSessionManager.createSession.bind(localSessionManager);
  docSign.getSession = localSessionManager.getSession.bind(localSessionManager);
  docSign.updateSessionStatus = localSessionManager.updateSessionStatus.bind(localSessionManager);
  docSign.recordSignature = localSessionManager.recordSignature.bind(localSessionManager);
  docSign.getSignedDocument = localSessionManager.getSignedDocument.bind(localSessionManager);
  docSign.deleteSession = localSessionManager.deleteSession.bind(localSessionManager);
  docSign.listSessions = localSessionManager.listSessions.bind(localSessionManager);
  console.log("[LocalSessionManager] Session management initialized on window.DocSign");
}

// src/ts/typed-signature.ts
var SIGNATURE_FONTS = [
  { name: "Dancing Script", label: "Classic Cursive", style: "flowing" },
  { name: "Great Vibes", label: "Elegant Script", style: "formal" },
  { name: "Pacifico", label: "Casual Handwriting", style: "casual" },
  { name: "Sacramento", label: "Flowing Script", style: "flowing" },
  { name: "Allura", label: "Formal Calligraphy", style: "calligraphy" }
];
var TypedSignature = class {
  constructor(options) {
    // DOM elements
    this.inputElement = null;
    this.fontSelectorContainer = null;
    this.previewCanvas = null;
    this.previewContainer = null;
    // State
    this.text = "";
    this.destroyed = false;
    this.container = options.container;
    this.fonts = options.fonts || SIGNATURE_FONTS.map((f) => f.name);
    this.currentFont = options.defaultFont || "Dancing Script";
    this.fontSize = options.fontSize || 48;
    this.textColor = options.textColor || "#000080";
    this.backgroundColor = options.backgroundColor || "#ffffff";
    this.placeholder = options.placeholder || "Type your name";
    this.onChange = options.onChange;
    this.render();
  }
  /**
   * Set the signature text
   */
  setText(text) {
    this.text = text;
    if (this.inputElement && this.inputElement.value !== text) {
      this.inputElement.value = text;
    }
    this.updatePreview();
    this.onChange?.(this.text, this.currentFont);
  }
  /**
   * Get the current signature text
   */
  getText() {
    return this.text;
  }
  /**
   * Set the current font
   */
  setFont(fontFamily) {
    if (this.fonts.includes(fontFamily)) {
      this.currentFont = fontFamily;
      this.updateFontSelection();
      this.updatePreview();
      this.onChange?.(this.text, this.currentFont);
    }
  }
  /**
   * Get the current font
   */
  getFont() {
    return this.currentFont;
  }
  /**
   * Check if signature is empty
   */
  isEmpty() {
    return !this.text || this.text.trim() === "";
  }
  /**
   * Export signature as data URL
   */
  toDataURL() {
    const canvas = this.toCanvas();
    return canvas.toDataURL("image/png");
  }
  /**
   * Export signature as canvas
   */
  toCanvas() {
    const dpr = window.devicePixelRatio || 1;
    const width = 400;
    const height = 100;
    const canvas = document.createElement("canvas");
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Could not get canvas 2D context");
    }
    ctx.scale(dpr, dpr);
    ctx.fillStyle = this.backgroundColor;
    ctx.fillRect(0, 0, width, height);
    if (this.text.trim()) {
      ctx.font = `${this.fontSize}px '${this.currentFont}', cursive`;
      ctx.fillStyle = this.textColor;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      const textMetrics = ctx.measureText(this.text);
      const actualHeight = textMetrics.actualBoundingBoxAscent + textMetrics.actualBoundingBoxDescent;
      const yOffset = (height - actualHeight) / 2 + textMetrics.actualBoundingBoxAscent;
      ctx.fillText(this.text, width / 2, yOffset || height / 2, width - 20);
    }
    return canvas;
  }
  /**
   * Clean up and remove the component
   */
  destroy() {
    this.destroyed = true;
    this.container.innerHTML = "";
    this.inputElement = null;
    this.fontSelectorContainer = null;
    this.previewCanvas = null;
    this.previewContainer = null;
  }
  /**
   * Render the component into the container
   */
  render() {
    this.container.innerHTML = "";
    const wrapper = document.createElement("div");
    wrapper.className = "typed-signature-wrapper";
    wrapper.style.cssText = `
      display: flex;
      flex-direction: column;
      gap: 1.5rem;
    `;
    this.inputElement = this.createInput();
    wrapper.appendChild(this.inputElement);
    const fontSection = document.createElement("div");
    fontSection.className = "typed-signature-font-section";
    const fontLabel = document.createElement("label");
    fontLabel.textContent = "Choose a style for your signature";
    fontLabel.style.cssText = `
      display: block;
      font-size: 18px;
      font-weight: 600;
      color: #111827;
      margin-bottom: 1rem;
    `;
    fontSection.appendChild(fontLabel);
    this.fontSelectorContainer = this.createFontSelector();
    fontSection.appendChild(this.fontSelectorContainer);
    wrapper.appendChild(fontSection);
    this.previewContainer = this.createPreviewSection();
    wrapper.appendChild(this.previewContainer);
    this.container.appendChild(wrapper);
    this.updatePreview();
  }
  /**
   * Create the text input element
   */
  createInput() {
    const input = document.createElement("input");
    input.type = "text";
    input.placeholder = this.placeholder;
    input.className = "typed-signature-input";
    input.autocomplete = "name";
    input.autocapitalize = "words";
    input.style.cssText = `
      width: 100%;
      min-height: 60px;
      padding: 16px 20px;
      border: 2px solid #d1d5db;
      border-radius: 8px;
      font-size: 24px;
      font-family: inherit;
      text-align: center;
      color: #111827;
      background: #ffffff;
      transition: border-color 0.2s, box-shadow 0.2s;
      outline: none;
    `;
    input.addEventListener("focus", () => {
      input.style.borderColor = "#1e40af";
      input.style.boxShadow = "0 0 0 3px rgba(30, 64, 175, 0.1)";
    });
    input.addEventListener("blur", () => {
      input.style.borderColor = "#d1d5db";
      input.style.boxShadow = "none";
    });
    input.addEventListener("input", () => {
      this.text = input.value;
      this.updatePreview();
      this.updateFontPreviews();
      this.onChange?.(this.text, this.currentFont);
    });
    return input;
  }
  /**
   * Create the font selector with visual previews
   */
  createFontSelector() {
    const container = document.createElement("div");
    container.className = "typed-signature-fonts";
    container.setAttribute("role", "radiogroup");
    container.setAttribute("aria-label", "Signature style");
    container.style.cssText = `
      display: flex;
      flex-direction: column;
      gap: 12px;
    `;
    this.fonts.forEach((fontName, index) => {
      const fontOption = this.createFontOption(fontName, index);
      container.appendChild(fontOption);
    });
    return container;
  }
  /**
   * Create a single font option with radio button and preview
   */
  createFontOption(fontName, index) {
    const fontInfo = SIGNATURE_FONTS.find((f) => f.name === fontName);
    const label = fontInfo?.label || fontName;
    const optionContainer = document.createElement("label");
    optionContainer.className = "typed-signature-font-option";
    optionContainer.style.cssText = `
      display: flex;
      align-items: center;
      gap: 16px;
      padding: 16px;
      border: 2px solid ${this.currentFont === fontName ? "#1e40af" : "#e5e7eb"};
      border-radius: 12px;
      cursor: pointer;
      background: ${this.currentFont === fontName ? "rgba(30, 64, 175, 0.05)" : "#ffffff"};
      transition: all 0.2s;
    `;
    optionContainer.addEventListener("mouseenter", () => {
      if (this.currentFont !== fontName) {
        optionContainer.style.borderColor = "#9ca3af";
        optionContainer.style.background = "#f9fafb";
      }
    });
    optionContainer.addEventListener("mouseleave", () => {
      if (this.currentFont !== fontName) {
        optionContainer.style.borderColor = "#e5e7eb";
        optionContainer.style.background = "#ffffff";
      }
    });
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = "signature-font";
    radio.value = fontName;
    radio.checked = this.currentFont === fontName;
    radio.id = `font-${index}`;
    radio.style.cssText = `
      width: 32px;
      height: 32px;
      margin: 0;
      cursor: pointer;
      accent-color: #1e40af;
      flex-shrink: 0;
    `;
    radio.addEventListener("change", () => {
      if (radio.checked) {
        this.setFont(fontName);
      }
    });
    const previewArea = document.createElement("div");
    previewArea.className = "font-preview-area";
    previewArea.style.cssText = `
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 4px;
    `;
    const fontLabel = document.createElement("span");
    fontLabel.textContent = label;
    fontLabel.style.cssText = `
      font-size: 14px;
      color: #6b7280;
      font-weight: 500;
    `;
    const preview = document.createElement("span");
    preview.className = "font-preview-text";
    preview.dataset.font = fontName;
    preview.textContent = this.text || "Your Name";
    preview.style.cssText = `
      font-family: '${fontName}', cursive;
      font-size: 32px;
      color: ${this.textColor};
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    `;
    previewArea.appendChild(fontLabel);
    previewArea.appendChild(preview);
    optionContainer.appendChild(radio);
    optionContainer.appendChild(previewArea);
    return optionContainer;
  }
  /**
   * Create the preview section showing the final signature look
   */
  createPreviewSection() {
    const container = document.createElement("div");
    container.className = "typed-signature-preview-section";
    container.style.cssText = `
      margin-top: 0.5rem;
    `;
    const label = document.createElement("label");
    label.textContent = "Your signature will look like:";
    label.style.cssText = `
      display: block;
      font-size: 16px;
      color: #6b7280;
      margin-bottom: 0.75rem;
    `;
    container.appendChild(label);
    const canvasContainer = document.createElement("div");
    canvasContainer.style.cssText = `
      background: #ffffff;
      border: 2px solid #e5e7eb;
      border-radius: 12px;
      padding: 1.5rem;
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100px;
    `;
    this.previewCanvas = document.createElement("canvas");
    this.previewCanvas.width = 400;
    this.previewCanvas.height = 100;
    this.previewCanvas.style.cssText = `
      max-width: 100%;
      height: auto;
      display: block;
    `;
    canvasContainer.appendChild(this.previewCanvas);
    container.appendChild(canvasContainer);
    return container;
  }
  /**
   * Update the preview canvas with current text and font
   */
  updatePreview() {
    if (this.destroyed || !this.previewCanvas) return;
    const ctx = this.previewCanvas.getContext("2d");
    if (!ctx) return;
    const width = this.previewCanvas.width;
    const height = this.previewCanvas.height;
    ctx.fillStyle = this.backgroundColor;
    ctx.fillRect(0, 0, width, height);
    if (this.text.trim()) {
      ctx.font = `${this.fontSize}px '${this.currentFont}', cursive`;
      ctx.fillStyle = this.textColor;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(this.text, width / 2, height / 2, width - 20);
    } else {
      ctx.font = "16px system-ui, sans-serif";
      ctx.fillStyle = "#9ca3af";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText("Type your name above to see preview", width / 2, height / 2);
    }
  }
  /**
   * Update font previews to show user's typed name
   */
  updateFontPreviews() {
    if (this.destroyed || !this.fontSelectorContainer) return;
    const previews = this.fontSelectorContainer.querySelectorAll(".font-preview-text");
    previews.forEach((preview) => {
      preview.textContent = this.text || "Your Name";
    });
  }
  /**
   * Update font selection styling
   */
  updateFontSelection() {
    if (this.destroyed || !this.fontSelectorContainer) return;
    const options = this.fontSelectorContainer.querySelectorAll(".typed-signature-font-option");
    options.forEach((option) => {
      const radio = option.querySelector("input[type='radio']");
      const isSelected = radio?.value === this.currentFont;
      if (radio) {
        radio.checked = isSelected;
      }
      option.style.borderColor = isSelected ? "#1e40af" : "#e5e7eb";
      option.style.background = isSelected ? "rgba(30, 64, 175, 0.05)" : "#ffffff";
    });
  }
};
function createTypedSignature(options) {
  return new TypedSignature(options);
}
if (typeof window !== "undefined") {
  window.TypedSignature = TypedSignature;
  window.createTypedSignature = createTypedSignature;
  window.SIGNATURE_FONTS = SIGNATURE_FONTS;
}

// src/ts/mobile-signature-modal.ts
var MobileSignatureModal = class {
  constructor(options = {}) {
    this.modalElement = null;
    this.canvasElement = null;
    this.ctx = null;
    this.isModalOpen = false;
    // Drawing state
    this.isDrawing = false;
    this.points = [];
    this.strokes = [];
    this.currentStroke = null;
    this.animationFrameId = null;
    // Touch state
    this.activeTouchId = null;
    // Pen settings
    this.penColor = "#000000";
    this.penWidth = 3;
    // Resize debounce
    this.resizeTimeout = null;
    // Focus trap elements
    this.focusableElements = [];
    this.firstFocusable = null;
    this.lastFocusable = null;
    this.previousActiveElement = null;
    // Promise resolution for open() method
    this.resolvePromise = null;
    this.options = {
      title: options.title ?? "Sign Here",
      instructions: options.instructions ?? "Draw your signature with your finger",
      onComplete: options.onComplete ?? (() => {
      }),
      onCancel: options.onCancel ?? (() => {
      })
    };
    this.handleKeyDown = this.handleKeyDown.bind(this);
    this.handleResize = this.handleResize.bind(this);
    this.handleOrientationChange = this.handleOrientationChange.bind(this);
    this.handleTouchStart = this.handleTouchStart.bind(this);
    this.handleTouchMove = this.handleTouchMove.bind(this);
    this.handleTouchEnd = this.handleTouchEnd.bind(this);
    this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseUp = this.handleMouseUp.bind(this);
  }
  /**
   * Opens the signature modal and returns a Promise that resolves
   * with the signature result or null if cancelled.
   */
  open() {
    return new Promise((resolve) => {
      this.resolvePromise = resolve;
      this.showModal();
    });
  }
  /**
   * Closes the modal without saving
   */
  close() {
    this.hideModal(null);
  }
  /**
   * Returns whether the modal is currently open
   */
  isOpen() {
    return this.isModalOpen;
  }
  /**
   * Creates and shows the modal
   */
  showModal() {
    if (this.isModalOpen) return;
    this.previousActiveElement = document.activeElement;
    this.createModalDOM();
    document.body.appendChild(this.modalElement);
    document.body.style.overflow = "hidden";
    document.body.style.position = "fixed";
    document.body.style.width = "100%";
    document.body.style.height = "100%";
    this.addEventListeners();
    requestAnimationFrame(() => {
      this.initializeCanvas();
      this.setupFocusTrap();
      this.modalElement.offsetHeight;
      this.modalElement.classList.add("mobile-signature-modal--visible");
    });
    this.isModalOpen = true;
  }
  /**
   * Hides and destroys the modal
   */
  hideModal(result) {
    if (!this.isModalOpen) return;
    this.removeEventListeners();
    this.modalElement?.classList.remove("mobile-signature-modal--visible");
    setTimeout(() => {
      document.body.style.overflow = "";
      document.body.style.position = "";
      document.body.style.width = "";
      document.body.style.height = "";
      this.modalElement?.remove();
      this.modalElement = null;
      this.canvasElement = null;
      this.ctx = null;
      if (this.previousActiveElement && "focus" in this.previousActiveElement) {
        this.previousActiveElement.focus();
      }
      this.isModalOpen = false;
      if (this.resolvePromise) {
        this.resolvePromise(result);
        this.resolvePromise = null;
      }
      if (result) {
        this.options.onComplete(result.dataUrl);
      } else {
        this.options.onCancel();
      }
    }, 200);
  }
  /**
   * Creates the modal DOM structure
   */
  createModalDOM() {
    this.injectStyles();
    const modal = document.createElement("div");
    modal.className = "mobile-signature-modal";
    modal.setAttribute("role", "dialog");
    modal.setAttribute("aria-modal", "true");
    modal.setAttribute("aria-label", this.options.title);
    const isPortrait = this.isPortrait();
    modal.innerHTML = `
      <div class="mobile-signature-header">
        <h2 class="mobile-signature-title">${this.escapeHtml(this.options.title)}</h2>
        <button
          type="button"
          class="mobile-signature-close"
          aria-label="Close"
          data-action="close"
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      <div class="mobile-signature-canvas-area">
        <p class="mobile-signature-instructions">${this.escapeHtml(this.options.instructions)}</p>
        ${isPortrait ? this.createRotateHint() : ""}
        <div class="mobile-signature-canvas-wrapper">
          <canvas
            class="mobile-signature-canvas"
            aria-label="Signature drawing area"
            tabindex="0"
          ></canvas>
          <div class="mobile-signature-baseline">Sign above this line</div>
        </div>
      </div>

      <div class="mobile-signature-footer">
        <button
          type="button"
          class="mobile-signature-btn mobile-signature-btn--secondary"
          data-action="start-over"
          aria-label="Start over - clear signature"
        >
          Start Over
        </button>
        <button
          type="button"
          class="mobile-signature-btn mobile-signature-btn--secondary"
          data-action="undo"
          aria-label="Undo last stroke"
        >
          Undo
        </button>
        <button
          type="button"
          class="mobile-signature-btn mobile-signature-btn--primary"
          data-action="done"
          aria-label="Done - save signature"
        >
          Done
        </button>
      </div>
    `;
    this.modalElement = modal;
    this.canvasElement = modal.querySelector(".mobile-signature-canvas");
    modal.querySelectorAll("[data-action]").forEach((btn) => {
      const action = btn.getAttribute("data-action");
      btn.addEventListener("click", () => this.handleAction(action));
    });
  }
  /**
   * Creates the rotate hint HTML
   */
  createRotateHint() {
    return `
      <div class="rotate-hint" role="status" aria-live="polite">
        <svg class="rotate-hint-icon" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <rect x="4" y="2" width="16" height="20" rx="2" ry="2"></rect>
          <line x1="12" y1="18" x2="12.01" y2="18"></line>
        </svg>
        <span>Rotate your device for a better signing experience</span>
      </div>
    `;
  }
  /**
   * Injects the required CSS styles
   */
  injectStyles() {
    if (document.getElementById("mobile-signature-modal-styles")) return;
    const style = document.createElement("style");
    style.id = "mobile-signature-modal-styles";
    style.textContent = `
      /* Mobile Signature Modal - Full Screen Overlay */
      .mobile-signature-modal {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        width: 100vw;
        height: 100vh;
        height: 100dvh; /* Dynamic viewport height for mobile browsers */
        background-color: var(--color-bg-primary, #ffffff);
        display: flex;
        flex-direction: column;
        z-index: 10000;
        opacity: 0;
        transform: translateY(100%);
        transition: opacity 0.2s ease, transform 0.2s ease;
        overflow: hidden;
        /* Safe area insets for notched devices */
        padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left);
      }

      .mobile-signature-modal--visible {
        opacity: 1;
        transform: translateY(0);
      }

      /* Header - Fixed at top */
      .mobile-signature-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 16px 20px;
        background-color: var(--color-bg-primary, #ffffff);
        border-bottom: 2px solid var(--color-text-secondary, #4a4a4a);
        flex-shrink: 0;
        min-height: 60px;
      }

      .mobile-signature-title {
        font-size: var(--font-size-xl, 28px);
        font-weight: 600;
        margin: 0;
        color: var(--color-text-primary, #1a1a1a);
      }

      .mobile-signature-close {
        width: 60px;
        height: 60px;
        min-width: 60px;
        min-height: 60px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: transparent;
        border: 2px solid var(--color-text-secondary, #4a4a4a);
        border-radius: var(--border-radius, 8px);
        cursor: pointer;
        color: var(--color-text-primary, #1a1a1a);
        padding: 0;
        transition: background-color 0.2s ease, border-color 0.2s ease;
      }

      .mobile-signature-close:hover,
      .mobile-signature-close:focus {
        background-color: var(--color-bg-secondary, #f8f8f8);
        border-color: var(--color-action-bg, #0056b3);
      }

      .mobile-signature-close:focus {
        outline: var(--focus-ring-width, 4px) solid var(--focus-ring-color, #0066cc);
        outline-offset: var(--focus-ring-offset, 2px);
      }

      .mobile-signature-close svg {
        width: 28px;
        height: 28px;
      }

      /* Canvas Area - Fills available space */
      .mobile-signature-canvas-area {
        flex: 1;
        display: flex;
        flex-direction: column;
        padding: 16px;
        min-height: 0;
        overflow: hidden;
      }

      .mobile-signature-instructions {
        text-align: center;
        font-size: var(--font-size-lg, 22px);
        color: var(--color-text-secondary, #4a4a4a);
        margin: 0 0 12px 0;
        flex-shrink: 0;
      }

      .mobile-signature-canvas-wrapper {
        flex: 1;
        position: relative;
        border: 3px solid var(--color-text-secondary, #4a4a4a);
        border-radius: var(--border-radius-lg, 12px);
        background-color: #ffffff;
        overflow: hidden;
        min-height: 150px;
      }

      .mobile-signature-canvas {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        cursor: crosshair;
        touch-action: none;
        background-color: transparent;
      }

      .mobile-signature-canvas:focus {
        outline: var(--focus-ring-width, 4px) solid var(--focus-ring-color, #0066cc);
        outline-offset: -4px;
      }

      .mobile-signature-baseline {
        position: absolute;
        bottom: 30%;
        left: 16px;
        right: 16px;
        border-top: 2px dashed var(--color-text-secondary, #4a4a4a);
        padding-top: 4px;
        text-align: center;
        font-size: var(--font-size-sm, 16px);
        color: var(--color-text-secondary, #4a4a4a);
        pointer-events: none;
        opacity: 0.6;
      }

      /* Rotate Hint */
      .rotate-hint {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 12px;
        padding: 12px 16px;
        margin-bottom: 12px;
        background-color: var(--color-warning-bg, #fef3cd);
        border: 2px solid var(--color-warning-border, #d4a200);
        border-radius: var(--border-radius, 8px);
        font-size: var(--font-size-base, 18px);
        color: var(--color-warning, #8a5700);
        text-align: center;
        flex-shrink: 0;
      }

      .rotate-hint-icon {
        flex-shrink: 0;
        animation: rotate-hint-wobble 1.5s ease-in-out infinite;
      }

      @keyframes rotate-hint-wobble {
        0%, 100% { transform: rotate(-10deg); }
        50% { transform: rotate(10deg); }
      }

      /* Hide rotate hint in landscape */
      @media (orientation: landscape) {
        .rotate-hint {
          display: none;
        }
      }

      /* Footer - Fixed at bottom */
      .mobile-signature-footer {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: var(--button-gap, 24px);
        padding: 16px 20px;
        background-color: var(--color-bg-primary, #ffffff);
        border-top: 2px solid var(--color-text-secondary, #4a4a4a);
        flex-shrink: 0;
        flex-wrap: wrap;
      }

      .mobile-signature-btn {
        min-width: 100px;
        min-height: 60px;
        height: 60px;
        padding: 12px 24px;
        font-size: var(--font-size-action, 24px);
        font-weight: 600;
        border-radius: var(--border-radius, 8px);
        cursor: pointer;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        transition: background-color 0.2s ease, transform 0.1s ease;
        white-space: nowrap;
      }

      .mobile-signature-btn:active {
        transform: scale(0.98);
      }

      .mobile-signature-btn:focus {
        outline: var(--focus-ring-width, 4px) solid var(--focus-ring-color, #0066cc);
        outline-offset: var(--focus-ring-offset, 2px);
      }

      .mobile-signature-btn--secondary {
        background-color: var(--color-bg-primary, #ffffff);
        color: var(--color-action-bg, #0056b3);
        border: 2px solid var(--color-action-bg, #0056b3);
      }

      .mobile-signature-btn--secondary:hover {
        background-color: var(--color-bg-secondary, #f8f8f8);
      }

      .mobile-signature-btn--primary {
        background-color: var(--color-action-bg, #0056b3);
        color: var(--color-action-text, #ffffff);
        border: 2px solid var(--color-action-border, #003d82);
      }

      .mobile-signature-btn--primary:hover {
        background-color: var(--color-action-bg-hover, #003d82);
      }

      /* High Contrast Mode Support */
      @media (prefers-contrast: high) {
        .mobile-signature-modal {
          background-color: #ffffff;
        }

        .mobile-signature-header,
        .mobile-signature-footer {
          border-color: #000000;
        }

        .mobile-signature-canvas-wrapper {
          border-color: #000000;
          border-width: 4px;
        }

        .mobile-signature-btn {
          border-width: 3px;
        }

        .mobile-signature-close {
          border-width: 3px;
        }
      }

      /* Dark Mode Support */
      @media (prefers-color-scheme: dark) {
        .mobile-signature-modal {
          background-color: var(--color-bg-primary, #1a1a1a);
        }

        .mobile-signature-canvas-wrapper {
          background-color: #ffffff;
        }
      }

      /* Responsive adjustments for landscape on mobile */
      @media (orientation: landscape) and (max-height: 500px) {
        .mobile-signature-header {
          padding: 8px 16px;
          min-height: 50px;
        }

        .mobile-signature-title {
          font-size: 20px;
        }

        .mobile-signature-close {
          width: 50px;
          height: 50px;
          min-width: 50px;
          min-height: 50px;
        }

        .mobile-signature-canvas-area {
          padding: 8px 16px;
        }

        .mobile-signature-instructions {
          font-size: 16px;
          margin-bottom: 8px;
        }

        .mobile-signature-footer {
          padding: 8px 16px;
          gap: 16px;
        }

        .mobile-signature-btn {
          min-height: 50px;
          height: 50px;
          padding: 8px 20px;
          font-size: 18px;
        }
      }

      /* Very small screens */
      @media (max-width: 360px) {
        .mobile-signature-footer {
          gap: 12px;
        }

        .mobile-signature-btn {
          padding: 8px 16px;
          min-width: 80px;
        }
      }
    `;
    document.head.appendChild(style);
  }
  /**
   * Initializes the canvas for drawing
   */
  initializeCanvas() {
    if (!this.canvasElement) return;
    const wrapper = this.canvasElement.parentElement;
    if (!wrapper) return;
    const rect = wrapper.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    this.canvasElement.width = rect.width * dpr;
    this.canvasElement.height = rect.height * dpr;
    this.ctx = this.canvasElement.getContext("2d");
    if (!this.ctx) return;
    this.ctx.scale(dpr, dpr);
    this.ctx.strokeStyle = this.penColor;
    this.ctx.lineWidth = this.penWidth;
    this.ctx.lineCap = "round";
    this.ctx.lineJoin = "round";
    this.ctx.fillStyle = this.penColor;
    this.clearCanvas();
    this.redrawStrokes();
  }
  /**
   * Adds event listeners
   */
  addEventListeners() {
    document.addEventListener("keydown", this.handleKeyDown);
    window.addEventListener("resize", this.handleResize);
    window.addEventListener("orientationchange", this.handleOrientationChange);
    if (this.canvasElement) {
      this.canvasElement.addEventListener("touchstart", this.handleTouchStart, {
        passive: false
      });
      this.canvasElement.addEventListener("touchmove", this.handleTouchMove, {
        passive: false
      });
      this.canvasElement.addEventListener("touchend", this.handleTouchEnd, {
        passive: false
      });
      this.canvasElement.addEventListener("touchcancel", this.handleTouchEnd, {
        passive: false
      });
      this.canvasElement.addEventListener("mousedown", this.handleMouseDown);
      this.canvasElement.addEventListener("mousemove", this.handleMouseMove);
      this.canvasElement.addEventListener("mouseup", this.handleMouseUp);
      this.canvasElement.addEventListener("mouseleave", this.handleMouseUp);
    }
  }
  /**
   * Removes event listeners
   */
  removeEventListeners() {
    document.removeEventListener("keydown", this.handleKeyDown);
    window.removeEventListener("resize", this.handleResize);
    window.removeEventListener("orientationchange", this.handleOrientationChange);
    if (this.canvasElement) {
      this.canvasElement.removeEventListener("touchstart", this.handleTouchStart);
      this.canvasElement.removeEventListener("touchmove", this.handleTouchMove);
      this.canvasElement.removeEventListener("touchend", this.handleTouchEnd);
      this.canvasElement.removeEventListener("touchcancel", this.handleTouchEnd);
      this.canvasElement.removeEventListener("mousedown", this.handleMouseDown);
      this.canvasElement.removeEventListener("mousemove", this.handleMouseMove);
      this.canvasElement.removeEventListener("mouseup", this.handleMouseUp);
      this.canvasElement.removeEventListener("mouseleave", this.handleMouseUp);
    }
    if (this.animationFrameId !== null) {
      cancelAnimationFrame(this.animationFrameId);
      this.animationFrameId = null;
    }
  }
  /**
   * Sets up the focus trap
   */
  setupFocusTrap() {
    if (!this.modalElement) return;
    this.focusableElements = Array.from(
      this.modalElement.querySelectorAll(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      )
    ).filter((el) => !el.hasAttribute("disabled") && el.offsetParent !== null);
    if (this.focusableElements.length > 0) {
      this.firstFocusable = this.focusableElements[0];
      this.lastFocusable = this.focusableElements[this.focusableElements.length - 1];
      const canvas = this.modalElement.querySelector(".mobile-signature-canvas");
      if (canvas) {
        canvas.focus();
      } else {
        this.firstFocusable.focus();
      }
    }
  }
  /**
   * Handles keyboard events
   */
  handleKeyDown(e) {
    if (e.key === "Escape") {
      e.preventDefault();
      this.close();
      return;
    }
    if (e.key === "Tab" && this.firstFocusable && this.lastFocusable) {
      if (e.shiftKey) {
        if (document.activeElement === this.firstFocusable) {
          e.preventDefault();
          this.lastFocusable.focus();
        }
      } else {
        if (document.activeElement === this.lastFocusable) {
          e.preventDefault();
          this.firstFocusable.focus();
        }
      }
    }
  }
  /**
   * Handles resize events with debounce
   */
  handleResize() {
    if (this.resizeTimeout !== null) {
      clearTimeout(this.resizeTimeout);
    }
    this.resizeTimeout = window.setTimeout(() => {
      this.initializeCanvas();
      this.updateRotateHint();
      this.resizeTimeout = null;
    }, 150);
  }
  /**
   * Handles orientation change
   */
  handleOrientationChange() {
    setTimeout(() => {
      this.initializeCanvas();
      this.updateRotateHint();
    }, 100);
  }
  /**
   * Updates the rotate hint visibility
   */
  updateRotateHint() {
    if (!this.modalElement) return;
    const hint = this.modalElement.querySelector(".rotate-hint");
    const existingHint = hint !== null;
    const shouldShow = this.isPortrait();
    if (shouldShow && !existingHint) {
      const canvasArea = this.modalElement.querySelector(".mobile-signature-canvas-area");
      const wrapper = this.modalElement.querySelector(".mobile-signature-canvas-wrapper");
      if (canvasArea && wrapper) {
        const hintEl = document.createElement("div");
        hintEl.innerHTML = this.createRotateHint();
        canvasArea.insertBefore(hintEl.firstElementChild, wrapper);
      }
    } else if (!shouldShow && existingHint) {
      hint?.remove();
    }
  }
  /**
   * Checks if device is in portrait orientation
   */
  isPortrait() {
    return window.innerHeight > window.innerWidth;
  }
  /**
   * Handles touch start
   */
  handleTouchStart(e) {
    e.preventDefault();
    if (this.activeTouchId !== null) return;
    const touch = e.touches[0];
    if (touch.radiusX && touch.radiusY) {
      const avgRadius = (touch.radiusX + touch.radiusY) / 2;
      if (avgRadius > 30) {
        return;
      }
    }
    this.activeTouchId = touch.identifier;
    const point = this.getTouchPos(touch);
    this.startStroke(point);
  }
  /**
   * Handles touch move
   */
  handleTouchMove(e) {
    e.preventDefault();
    if (this.activeTouchId === null) return;
    let touch = null;
    for (let i = 0; i < e.touches.length; i++) {
      if (e.touches[i].identifier === this.activeTouchId) {
        touch = e.touches[i];
        break;
      }
    }
    if (!touch) return;
    const point = this.getTouchPos(touch);
    this.continueStroke(point);
  }
  /**
   * Handles touch end
   */
  handleTouchEnd(e) {
    e.preventDefault();
    let touchEnded = true;
    for (let i = 0; i < e.touches.length; i++) {
      if (e.touches[i].identifier === this.activeTouchId) {
        touchEnded = false;
        break;
      }
    }
    if (touchEnded) {
      this.endStroke();
      this.activeTouchId = null;
    }
  }
  /**
   * Handles mouse down (for desktop testing)
   */
  handleMouseDown(e) {
    const point = this.getMousePos(e);
    this.startStroke(point);
  }
  /**
   * Handles mouse move (for desktop testing)
   */
  handleMouseMove(e) {
    if (!this.isDrawing) return;
    const point = this.getMousePos(e);
    this.continueStroke(point);
  }
  /**
   * Handles mouse up (for desktop testing)
   */
  handleMouseUp() {
    this.endStroke();
  }
  /**
   * Gets point from touch event
   */
  getTouchPos(touch) {
    if (!this.canvasElement) return { x: 0, y: 0 };
    const rect = this.canvasElement.getBoundingClientRect();
    return {
      x: touch.clientX - rect.left,
      y: touch.clientY - rect.top,
      pressure: touch.force || 0.5
    };
  }
  /**
   * Gets point from mouse event
   */
  getMousePos(e) {
    if (!this.canvasElement) return { x: 0, y: 0 };
    const rect = this.canvasElement.getBoundingClientRect();
    return {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
      pressure: 0.5
    };
  }
  /**
   * Starts a new stroke
   */
  startStroke(point) {
    this.isDrawing = true;
    this.points = [point];
    this.currentStroke = {
      points: [point],
      color: this.penColor,
      width: this.penWidth
    };
    if (this.ctx) {
      this.ctx.beginPath();
      this.ctx.arc(point.x, point.y, this.penWidth / 2, 0, Math.PI * 2);
      this.ctx.fill();
    }
  }
  /**
   * Continues the current stroke
   */
  continueStroke(point) {
    if (!this.isDrawing || !this.ctx || !this.currentStroke) return;
    this.points.push(point);
    this.currentStroke.points.push(point);
    if (this.animationFrameId === null) {
      this.animationFrameId = requestAnimationFrame(() => {
        this.renderPendingPoints();
        this.animationFrameId = null;
      });
    }
  }
  /**
   * Renders pending points with smooth bezier curves
   */
  renderPendingPoints() {
    if (!this.ctx || this.points.length < 2) return;
    if (this.points.length >= 3) {
      const p1 = this.points[this.points.length - 3];
      const p2 = this.points[this.points.length - 2];
      const p3 = this.points[this.points.length - 1];
      const midX = (p2.x + p3.x) / 2;
      const midY = (p2.y + p3.y) / 2;
      this.ctx.beginPath();
      this.ctx.moveTo(p1.x, p1.y);
      this.ctx.quadraticCurveTo(p2.x, p2.y, midX, midY);
      this.ctx.stroke();
    } else {
      const lastPoint = this.points[this.points.length - 2];
      const currentPoint = this.points[this.points.length - 1];
      this.ctx.beginPath();
      this.ctx.moveTo(lastPoint.x, lastPoint.y);
      this.ctx.lineTo(currentPoint.x, currentPoint.y);
      this.ctx.stroke();
    }
  }
  /**
   * Ends the current stroke
   */
  endStroke() {
    if (!this.isDrawing) return;
    this.isDrawing = false;
    if (this.currentStroke && this.currentStroke.points.length > 0) {
      this.strokes.push(this.currentStroke);
    }
    this.currentStroke = null;
    this.points = [];
  }
  /**
   * Clears the canvas
   */
  clearCanvas() {
    if (!this.ctx || !this.canvasElement) return;
    const dpr = window.devicePixelRatio || 1;
    this.ctx.clearRect(
      0,
      0,
      this.canvasElement.width / dpr,
      this.canvasElement.height / dpr
    );
  }
  /**
   * Redraws all saved strokes
   */
  redrawStrokes() {
    if (!this.ctx) return;
    for (const stroke of this.strokes) {
      this.ctx.strokeStyle = stroke.color;
      this.ctx.lineWidth = stroke.width;
      this.ctx.fillStyle = stroke.color;
      if (stroke.points.length === 0) continue;
      const firstPoint = stroke.points[0];
      this.ctx.beginPath();
      this.ctx.arc(firstPoint.x, firstPoint.y, stroke.width / 2, 0, Math.PI * 2);
      this.ctx.fill();
      if (stroke.points.length >= 2) {
        for (let i = 2; i < stroke.points.length; i++) {
          const p1 = stroke.points[i - 2];
          const p2 = stroke.points[i - 1];
          const p3 = stroke.points[i];
          const midX = (p2.x + p3.x) / 2;
          const midY = (p2.y + p3.y) / 2;
          this.ctx.beginPath();
          this.ctx.moveTo(p1.x, p1.y);
          this.ctx.quadraticCurveTo(p2.x, p2.y, midX, midY);
          this.ctx.stroke();
        }
        if (stroke.points.length === 2) {
          this.ctx.beginPath();
          this.ctx.moveTo(stroke.points[0].x, stroke.points[0].y);
          this.ctx.lineTo(stroke.points[1].x, stroke.points[1].y);
          this.ctx.stroke();
        }
      }
    }
    this.ctx.strokeStyle = this.penColor;
    this.ctx.lineWidth = this.penWidth;
    this.ctx.fillStyle = this.penColor;
  }
  /**
   * Handles button actions
   */
  handleAction(action) {
    switch (action) {
      case "close":
        this.close();
        break;
      case "start-over":
        this.strokes = [];
        this.clearCanvas();
        break;
      case "undo":
        this.strokes.pop();
        this.clearCanvas();
        this.redrawStrokes();
        break;
      case "done":
        this.saveAndClose();
        break;
    }
  }
  /**
   * Saves the signature and closes the modal
   */
  saveAndClose() {
    if (!this.canvasElement) {
      this.close();
      return;
    }
    if (this.strokes.length === 0) {
      this.showValidationError("Please draw your signature first");
      return;
    }
    const dataUrl = this.canvasElement.toDataURL("image/png");
    const result = {
      dataUrl,
      type: "drawn",
      timestamp: (/* @__PURE__ */ new Date()).toISOString()
    };
    this.hideModal(result);
  }
  /**
   * Shows a validation error message
   */
  showValidationError(message) {
    alert(message);
  }
  /**
   * Escapes HTML to prevent XSS
   */
  escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }
};
function isMobileDevice() {
  return window.matchMedia("(max-width: 768px)").matches;
}
function createSignatureModal(options) {
  return new MobileSignatureModal(options);
}
if (typeof window !== "undefined") {
  window.MobileSignatureModal = MobileSignatureModal;
  window.isMobileDevice = isMobileDevice;
  window.createSignatureModal = createSignatureModal;
}

// src/ts/signature-capture.ts
var SignatureCapture = class {
  constructor(options) {
    this.guideCanvas = null;
    this.guideCtx = null;
    // Drawing state
    this.isDrawing = false;
    this.currentStroke = [];
    this.strokes = [];
    this.redoStack = [];
    // Touch detection
    this.isTouchDevice = false;
    // Event handler
    this.onchange = null;
    this.container = options.container;
    this.height = options.height ?? 200;
    this.strokeColor = options.strokeColor ?? "#000080";
    this.backgroundColor = options.backgroundColor ?? "#ffffff";
    this.showGuides = options.showGuides ?? true;
    this.isTouchDevice = "ontouchstart" in window || navigator.maxTouchPoints > 0;
    this.strokeWidth = options.strokeWidth ?? (this.isTouchDevice ? 4 : 3);
    const containerWidth = this.container.clientWidth || 400;
    this.width = options.width ?? containerWidth;
    this.wrapper = this.createWrapper();
    if (this.showGuides) {
      this.guideCanvas = this.createGuideCanvas();
      this.guideCtx = this.guideCanvas.getContext("2d");
    }
    this.canvas = this.createCanvas();
    const ctx = this.canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Failed to get 2D context from canvas");
    }
    this.ctx = ctx;
    if (this.guideCanvas) {
      this.wrapper.appendChild(this.guideCanvas);
    }
    this.wrapper.appendChild(this.canvas);
    this.container.appendChild(this.wrapper);
    this.initializeCanvas();
    if (this.showGuides) {
      this.drawGuides();
    }
    this.boundHandlers = {
      mouseDown: this.handleMouseDown.bind(this),
      mouseMove: this.handleMouseMove.bind(this),
      mouseUp: this.handleMouseUp.bind(this),
      mouseLeave: this.handleMouseLeave.bind(this),
      touchStart: this.handleTouchStart.bind(this),
      touchMove: this.handleTouchMove.bind(this),
      touchEnd: this.handleTouchEnd.bind(this),
      resize: this.handleResize.bind(this)
    };
    this.attachEventListeners();
  }
  /**
   * Create the wrapper element
   */
  createWrapper() {
    const wrapper = document.createElement("div");
    wrapper.className = "signature-capture-wrapper";
    wrapper.style.cssText = `
      position: relative;
      width: 100%;
      max-width: ${this.width}px;
      height: ${this.height}px;
      border: 3px solid var(--color-text-secondary, #4a4a4a);
      border-radius: var(--border-radius-lg, 12px);
      background-color: ${this.backgroundColor};
      overflow: hidden;
      touch-action: none;
      user-select: none;
      -webkit-user-select: none;
    `;
    return wrapper;
  }
  /**
   * Create the guide canvas (for baseline)
   */
  createGuideCanvas() {
    const canvas = document.createElement("canvas");
    canvas.className = "signature-capture-guides";
    canvas.width = this.width;
    canvas.height = this.height;
    canvas.style.cssText = `
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      pointer-events: none;
    `;
    return canvas;
  }
  /**
   * Create the main drawing canvas
   */
  createCanvas() {
    const canvas = document.createElement("canvas");
    canvas.className = "signature-capture-canvas";
    canvas.width = this.width;
    canvas.height = this.height;
    canvas.style.cssText = `
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      cursor: crosshair;
      touch-action: none;
    `;
    canvas.setAttribute("role", "img");
    canvas.setAttribute(
      "aria-label",
      "Signature drawing area. Use mouse or touch to draw your signature."
    );
    canvas.tabIndex = 0;
    return canvas;
  }
  /**
   * Initialize canvas with background and stroke settings
   */
  initializeCanvas() {
    this.ctx.fillStyle = this.backgroundColor;
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    this.ctx.strokeStyle = this.strokeColor;
    this.ctx.lineWidth = this.strokeWidth;
    this.ctx.lineCap = "round";
    this.ctx.lineJoin = "round";
  }
  /**
   * Draw baseline guides
   */
  drawGuides() {
    if (!this.guideCtx || !this.guideCanvas) return;
    const ctx = this.guideCtx;
    const width = this.guideCanvas.width;
    const height = this.guideCanvas.height;
    ctx.clearRect(0, 0, width, height);
    const baselineY = height * 0.7;
    ctx.strokeStyle = "#e0e0e0";
    ctx.lineWidth = 2;
    ctx.setLineDash([10, 5]);
    ctx.beginPath();
    ctx.moveTo(20, baselineY);
    ctx.lineTo(width - 20, baselineY);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.fillStyle = "#cccccc";
    ctx.font = "italic 16px var(--font-family-body, sans-serif)";
    ctx.textAlign = "center";
    ctx.fillText("Sign on the line above", width / 2, height - 15);
  }
  /**
   * Attach event listeners
   */
  attachEventListeners() {
    this.canvas.addEventListener("mousedown", this.boundHandlers.mouseDown);
    this.canvas.addEventListener("mousemove", this.boundHandlers.mouseMove);
    this.canvas.addEventListener("mouseup", this.boundHandlers.mouseUp);
    this.canvas.addEventListener("mouseleave", this.boundHandlers.mouseLeave);
    this.canvas.addEventListener("touchstart", this.boundHandlers.touchStart, {
      passive: false
    });
    this.canvas.addEventListener("touchmove", this.boundHandlers.touchMove, {
      passive: false
    });
    this.canvas.addEventListener("touchend", this.boundHandlers.touchEnd);
    window.addEventListener("resize", this.boundHandlers.resize);
  }
  /**
   * Handle window resize
   */
  handleResize() {
    const containerWidth = this.container.clientWidth;
    if (containerWidth > 0 && containerWidth !== this.width) {
      const savedStrokes = [...this.strokes];
      this.width = containerWidth;
      this.canvas.width = this.width;
      if (this.guideCanvas) {
        this.guideCanvas.width = this.width;
      }
      this.initializeCanvas();
      if (this.showGuides) {
        this.drawGuides();
      }
      this.strokes = savedStrokes;
      this.redrawAllStrokes();
    }
  }
  /**
   * Get point from mouse event
   */
  getMousePoint(e) {
    const rect = this.canvas.getBoundingClientRect();
    const scaleX = this.canvas.width / rect.width;
    const scaleY = this.canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: (e.clientY - rect.top) * scaleY,
      pressure: 0.5
      // Default pressure for mouse
    };
  }
  /**
   * Get point from touch event
   */
  getTouchPoint(e) {
    const rect = this.canvas.getBoundingClientRect();
    const scaleX = this.canvas.width / rect.width;
    const scaleY = this.canvas.height / rect.height;
    const touch = e.touches[0];
    let pressure = 0.5;
    if ("force" in touch && typeof touch.force === "number") {
      pressure = touch.force;
    }
    return {
      x: (touch.clientX - rect.left) * scaleX,
      y: (touch.clientY - rect.top) * scaleY,
      pressure
    };
  }
  /**
   * Start drawing
   */
  startDrawing(point) {
    this.isDrawing = true;
    this.currentStroke = [point];
    this.redoStack = [];
    this.ctx.beginPath();
    this.ctx.moveTo(point.x, point.y);
  }
  /**
   * Continue drawing
   */
  continueDrawing(point) {
    if (!this.isDrawing) return;
    this.currentStroke.push(point);
    const pressure = point.pressure ?? 0.5;
    const dynamicWidth = this.strokeWidth * (0.5 + pressure);
    this.ctx.lineWidth = dynamicWidth;
    this.ctx.lineTo(point.x, point.y);
    this.ctx.stroke();
    this.ctx.beginPath();
    this.ctx.moveTo(point.x, point.y);
  }
  /**
   * End drawing
   */
  endDrawing() {
    if (!this.isDrawing) return;
    this.isDrawing = false;
    if (this.currentStroke.length > 0) {
      this.strokes.push({
        points: this.currentStroke,
        color: this.strokeColor,
        width: this.strokeWidth
      });
      this.currentStroke = [];
      this.notifyChange();
    }
  }
  /**
   * Mouse event handlers
   */
  handleMouseDown(e) {
    e.preventDefault();
    const point = this.getMousePoint(e);
    this.startDrawing(point);
  }
  handleMouseMove(e) {
    e.preventDefault();
    const point = this.getMousePoint(e);
    this.continueDrawing(point);
  }
  handleMouseUp() {
    this.endDrawing();
  }
  handleMouseLeave() {
    this.endDrawing();
  }
  /**
   * Touch event handlers
   */
  handleTouchStart(e) {
    e.preventDefault();
    if (e.touches.length === 1) {
      const point = this.getTouchPoint(e);
      this.startDrawing(point);
    }
  }
  handleTouchMove(e) {
    e.preventDefault();
    if (e.touches.length === 1) {
      const point = this.getTouchPoint(e);
      this.continueDrawing(point);
    }
  }
  handleTouchEnd() {
    this.endDrawing();
  }
  /**
   * Redraw all strokes (used after undo/clear/resize)
   */
  redrawAllStrokes() {
    this.ctx.fillStyle = this.backgroundColor;
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    for (const stroke of this.strokes) {
      if (stroke.points.length === 0) continue;
      this.ctx.strokeStyle = stroke.color;
      this.ctx.lineWidth = stroke.width;
      this.ctx.beginPath();
      const firstPoint = stroke.points[0];
      this.ctx.moveTo(firstPoint.x, firstPoint.y);
      for (let i = 1; i < stroke.points.length; i++) {
        const point = stroke.points[i];
        const pressure = point.pressure ?? 0.5;
        this.ctx.lineWidth = stroke.width * (0.5 + pressure);
        this.ctx.lineTo(point.x, point.y);
        this.ctx.stroke();
        this.ctx.beginPath();
        this.ctx.moveTo(point.x, point.y);
      }
    }
    this.ctx.strokeStyle = this.strokeColor;
    this.ctx.lineWidth = this.strokeWidth;
  }
  /**
   * Notify change listeners
   */
  notifyChange() {
    if (this.onchange) {
      this.onchange(this.isEmpty());
    }
  }
  // ========================================
  // Public API
  // ========================================
  /**
   * Clear the canvas and reset all strokes
   */
  clear() {
    this.redoStack = [...this.strokes];
    this.strokes = [];
    this.currentStroke = [];
    this.redrawAllStrokes();
    this.notifyChange();
    this.announceToScreenReader("Signature cleared");
  }
  /**
   * Undo the last stroke
   */
  undo() {
    if (this.strokes.length === 0) return;
    const lastStroke = this.strokes.pop();
    if (lastStroke) {
      this.redoStack.push(lastStroke);
    }
    this.redrawAllStrokes();
    this.notifyChange();
    this.announceToScreenReader("Stroke undone");
  }
  /**
   * Redo the last undone stroke
   */
  redo() {
    if (this.redoStack.length === 0) return;
    const stroke = this.redoStack.pop();
    if (stroke) {
      this.strokes.push(stroke);
    }
    this.redrawAllStrokes();
    this.notifyChange();
    this.announceToScreenReader("Stroke restored");
  }
  /**
   * Check if the signature pad is empty
   */
  isEmpty() {
    return this.strokes.length === 0;
  }
  /**
   * Check if undo is available
   */
  canUndo() {
    return this.strokes.length > 0;
  }
  /**
   * Check if redo is available
   */
  canRedo() {
    return this.redoStack.length > 0;
  }
  /**
   * Get stroke count
   */
  getStrokeCount() {
    return this.strokes.length;
  }
  /**
   * Export signature as data URL
   * @param format - 'png' or 'svg'
   */
  toDataURL(format = "png") {
    if (format === "svg") {
      return this.toSVG();
    }
    return this.canvas.toDataURL("image/png");
  }
  /**
   * Export signature as SVG string
   */
  toSVG() {
    const width = this.canvas.width;
    const height = this.canvas.height;
    let pathData = "";
    for (const stroke of this.strokes) {
      if (stroke.points.length === 0) continue;
      const firstPoint = stroke.points[0];
      pathData += `M ${firstPoint.x} ${firstPoint.y} `;
      for (let i = 1; i < stroke.points.length; i++) {
        const point = stroke.points[i];
        pathData += `L ${point.x} ${point.y} `;
      }
    }
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
      <rect width="${width}" height="${height}" fill="${this.backgroundColor}"/>
      <path d="${pathData}" stroke="${this.strokeColor}" stroke-width="${this.strokeWidth}" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`;
    return `data:image/svg+xml;base64,${btoa(svg)}`;
  }
  /**
   * Export signature as Blob
   */
  toBlob() {
    return new Promise((resolve, reject) => {
      this.canvas.toBlob(
        (blob) => {
          if (blob) {
            resolve(blob);
          } else {
            reject(new Error("Failed to create blob from canvas"));
          }
        },
        "image/png",
        1
      );
    });
  }
  /**
   * Get the raw strokes data
   */
  getStrokes() {
    return [...this.strokes];
  }
  /**
   * Load strokes from data
   */
  loadStrokes(strokes) {
    this.strokes = strokes.map((s) => ({
      points: [...s.points],
      color: s.color,
      width: s.width
    }));
    this.redoStack = [];
    this.redrawAllStrokes();
    this.notifyChange();
  }
  /**
   * Announce to screen readers
   */
  announceToScreenReader(message) {
    const announcement = document.createElement("div");
    announcement.setAttribute("role", "status");
    announcement.setAttribute("aria-live", "polite");
    announcement.className = "visually-hidden";
    announcement.textContent = message;
    document.body.appendChild(announcement);
    setTimeout(() => {
      announcement.remove();
    }, 1e3);
  }
  /**
   * Destroy the component and clean up
   */
  destroy() {
    this.canvas.removeEventListener("mousedown", this.boundHandlers.mouseDown);
    this.canvas.removeEventListener("mousemove", this.boundHandlers.mouseMove);
    this.canvas.removeEventListener("mouseup", this.boundHandlers.mouseUp);
    this.canvas.removeEventListener("mouseleave", this.boundHandlers.mouseLeave);
    this.canvas.removeEventListener("touchstart", this.boundHandlers.touchStart);
    this.canvas.removeEventListener("touchmove", this.boundHandlers.touchMove);
    this.canvas.removeEventListener("touchend", this.boundHandlers.touchEnd);
    window.removeEventListener("resize", this.boundHandlers.resize);
    this.wrapper.remove();
    this.strokes = [];
    this.redoStack = [];
    this.onchange = null;
  }
  /**
   * Get the canvas element (for advanced use cases)
   */
  getCanvas() {
    return this.canvas;
  }
  /**
   * Get the wrapper element
   */
  getWrapper() {
    return this.wrapper;
  }
};

// src/ts/signature-modal.ts
var SignatureModal = class {
  constructor(modalElement, options = {}) {
    // Mode state
    this.mode = "draw";
    this.currentFieldId = null;
    // DOM elements
    this.drawTab = null;
    this.typeTab = null;
    this.drawPanel = null;
    this.typePanel = null;
    this.drawTabBtn = null;
    this.typeTabBtn = null;
    this.canvas = null;
    this.ctx = null;
    this.clearBtn = null;
    this.applyBtn = null;
    this.cancelBtn = null;
    this.closeBtn = null;
    // Typed signature component
    this.typedSignature = null;
    this.typedSignatureContainer = null;
    // Drawing state
    this.isDrawing = false;
    this.lastX = 0;
    this.lastY = 0;
    this.hasDrawn = false;
    this.modal = modalElement;
    this.onApply = options.onApply || (() => {
    });
    this.onCancel = options.onCancel || (() => {
    });
    this.penColor = options.penColor || "#000000";
    this.penWidth = options.penWidth || 2;
    this.initializeElements();
    this.bindEvents();
    this.enhanceTypeTab();
  }
  /**
   * Initialize DOM element references
   */
  initializeElements() {
    this.drawTabBtn = this.modal.querySelector('[data-tab="draw"]');
    this.typeTabBtn = this.modal.querySelector('[data-tab="type"]');
    this.drawPanel = this.modal.querySelector("#draw-tab");
    this.typePanel = this.modal.querySelector("#type-tab");
    this.canvas = this.modal.querySelector("#signature-pad");
    this.clearBtn = this.modal.querySelector("#clear-signature");
    this.applyBtn = this.modal.querySelector("#apply-signature");
    this.cancelBtn = this.modal.querySelector("#cancel-signature");
    this.closeBtn = this.modal.querySelector("#close-signature-modal");
    if (this.canvas) {
      this.ctx = this.canvas.getContext("2d");
    }
  }
  /**
   * Bind event listeners
   */
  bindEvents() {
    this.drawTabBtn?.addEventListener("click", () => this.switchTab("draw"));
    this.typeTabBtn?.addEventListener("click", () => this.switchTab("type"));
    this.clearBtn?.addEventListener("click", () => this.clearCanvas());
    this.applyBtn?.addEventListener("click", () => this.apply());
    this.cancelBtn?.addEventListener("click", () => this.hide());
    this.closeBtn?.addEventListener("click", () => this.hide());
    this.modal.addEventListener("click", (e) => {
      if (e.target === this.modal) {
        this.hide();
      }
    });
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && !this.modal.classList.contains("hidden")) {
        this.hide();
      }
    });
    if (this.canvas) {
      this.bindCanvasEvents();
    }
  }
  /**
   * Bind canvas drawing events
   */
  bindCanvasEvents() {
    if (!this.canvas) return;
    this.canvas.addEventListener("mousedown", this.handlePointerDown.bind(this));
    this.canvas.addEventListener("mousemove", this.handlePointerMove.bind(this));
    this.canvas.addEventListener("mouseup", this.handlePointerUp.bind(this));
    this.canvas.addEventListener("mouseleave", this.handlePointerUp.bind(this));
    this.canvas.addEventListener("touchstart", this.handleTouchStart.bind(this));
    this.canvas.addEventListener("touchmove", this.handleTouchMove.bind(this));
    this.canvas.addEventListener("touchend", this.handlePointerUp.bind(this));
    this.canvas.addEventListener("touchstart", (e) => e.preventDefault());
    this.canvas.addEventListener("touchmove", (e) => e.preventDefault());
  }
  /**
   * Enhance the type tab with the new TypedSignature component
   */
  enhanceTypeTab() {
    if (!this.typePanel) return;
    this.typePanel.innerHTML = "";
    this.typedSignatureContainer = document.createElement("div");
    this.typedSignatureContainer.id = "typed-signature-container";
    this.typePanel.appendChild(this.typedSignatureContainer);
    this.typedSignature = new TypedSignature({
      container: this.typedSignatureContainer,
      fonts: SIGNATURE_FONTS.map((f) => f.name),
      defaultFont: "Dancing Script",
      fontSize: 48,
      textColor: "#000080",
      backgroundColor: "#ffffff",
      placeholder: "Type your full name"
    });
  }
  /**
   * Switch between draw and type tabs
   */
  switchTab(tab) {
    this.mode = tab;
    if (tab === "draw") {
      this.drawTabBtn?.classList.add("active");
      this.typeTabBtn?.classList.remove("active");
      this.drawPanel?.classList.add("active");
      this.drawPanel?.classList.remove("hidden");
      this.typePanel?.classList.remove("active");
      this.typePanel?.classList.add("hidden");
      this.initCanvas();
    } else {
      this.typeTabBtn?.classList.add("active");
      this.drawTabBtn?.classList.remove("active");
      this.typePanel?.classList.add("active");
      this.typePanel?.classList.remove("hidden");
      this.drawPanel?.classList.remove("active");
      this.drawPanel?.classList.add("hidden");
    }
  }
  /**
   * Initialize canvas for drawing
   */
  initCanvas() {
    if (!this.canvas || !this.ctx) return;
    const rect = this.canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    this.canvas.width = rect.width * dpr;
    this.canvas.height = rect.height * dpr;
    this.ctx.scale(dpr, dpr);
    const isMobile = window.innerWidth < 768;
    this.ctx.strokeStyle = this.penColor;
    this.ctx.lineWidth = isMobile ? Math.max(this.penWidth, 3) : this.penWidth;
    this.ctx.lineCap = "round";
    this.ctx.lineJoin = "round";
    this.clearCanvas();
  }
  /**
   * Clear the drawing canvas
   */
  clearCanvas() {
    if (!this.canvas || !this.ctx) return;
    this.ctx.fillStyle = "#ffffff";
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    this.ctx.fillStyle = this.penColor;
    this.hasDrawn = false;
  }
  /**
   * Get pointer position from mouse event
   */
  getPointerPos(e) {
    if (!this.canvas) return { x: 0, y: 0 };
    const rect = this.canvas.getBoundingClientRect();
    return {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top
    };
  }
  /**
   * Get pointer position from touch event
   */
  getTouchPos(e) {
    if (!this.canvas) return { x: 0, y: 0 };
    const rect = this.canvas.getBoundingClientRect();
    const touch = e.touches[0];
    return {
      x: touch.clientX - rect.left,
      y: touch.clientY - rect.top
    };
  }
  /**
   * Handle pointer down (start drawing)
   */
  handlePointerDown(e) {
    this.isDrawing = true;
    const pos = this.getPointerPos(e);
    this.lastX = pos.x;
    this.lastY = pos.y;
    if (this.ctx) {
      this.ctx.beginPath();
      this.ctx.arc(pos.x, pos.y, this.penWidth / 2, 0, Math.PI * 2);
      this.ctx.fill();
    }
    this.hasDrawn = true;
  }
  /**
   * Handle pointer move (draw)
   */
  handlePointerMove(e) {
    if (!this.isDrawing || !this.ctx) return;
    const pos = this.getPointerPos(e);
    this.ctx.beginPath();
    this.ctx.moveTo(this.lastX, this.lastY);
    this.ctx.lineTo(pos.x, pos.y);
    this.ctx.stroke();
    this.lastX = pos.x;
    this.lastY = pos.y;
    this.hasDrawn = true;
  }
  /**
   * Handle pointer up (stop drawing)
   */
  handlePointerUp() {
    this.isDrawing = false;
  }
  /**
   * Handle touch start
   */
  handleTouchStart(e) {
    this.isDrawing = true;
    const pos = this.getTouchPos(e);
    this.lastX = pos.x;
    this.lastY = pos.y;
    if (this.ctx) {
      this.ctx.beginPath();
      this.ctx.arc(pos.x, pos.y, this.penWidth / 2, 0, Math.PI * 2);
      this.ctx.fill();
    }
    this.hasDrawn = true;
  }
  /**
   * Handle touch move
   */
  handleTouchMove(e) {
    if (!this.isDrawing || !this.ctx) return;
    const pos = this.getTouchPos(e);
    this.ctx.beginPath();
    this.ctx.moveTo(this.lastX, this.lastY);
    this.ctx.lineTo(pos.x, pos.y);
    this.ctx.stroke();
    this.lastX = pos.x;
    this.lastY = pos.y;
    this.hasDrawn = true;
  }
  /**
   * Check if drawn signature is empty
   */
  isCanvasEmpty() {
    return !this.hasDrawn;
  }
  /**
   * Apply the signature
   */
  apply() {
    let signatureData;
    let text;
    let font;
    if (this.mode === "draw") {
      if (this.isCanvasEmpty()) {
        alert("Please draw your signature");
        return;
      }
      if (this.canvas) {
        signatureData = this.canvas.toDataURL("image/png");
      } else {
        return;
      }
    } else {
      if (!this.typedSignature || this.typedSignature.isEmpty()) {
        alert("Please type your name");
        return;
      }
      signatureData = this.typedSignature.toDataURL();
      text = this.typedSignature.getText();
      font = this.typedSignature.getFont();
    }
    const result = {
      fieldId: this.currentFieldId,
      signatureData,
      mode: this.mode,
      text,
      font
    };
    this.onApply(result);
    this.hide();
  }
  /**
   * Show the modal for a specific field
   */
  show(fieldId) {
    this.currentFieldId = fieldId || null;
    this.modal.classList.remove("hidden");
    const modalContent = this.modal.querySelector(".modal");
    if (modalContent && window.innerWidth < 768) {
      modalContent.classList.add("bottom-sheet-mobile");
    }
    this.switchTab("draw");
    this.clearCanvas();
    if (this.typedSignature) {
      this.typedSignature.setText("");
    }
  }
  /**
   * Hide the modal
   */
  hide() {
    this.modal.classList.add("hidden");
    this.onCancel();
  }
  /**
   * Destroy the modal controller
   */
  destroy() {
    if (this.typedSignature) {
      this.typedSignature.destroy();
      this.typedSignature = null;
    }
  }
};
function initSignatureModal(modalElement, options = {}) {
  return new SignatureModal(modalElement, options);
}
var SignatureCaptureModal = class {
  constructor(options = {}) {
    // DOM elements
    this.overlay = null;
    this.modalEl = null;
    this.capture = null;
    // Buttons
    this.btnStartOver = null;
    this.btnUndo = null;
    this.btnRedo = null;
    this.btnUseSignature = null;
    this.btnCancel = null;
    // State
    this.isOpenState = false;
    this.previousActiveElement = null;
    // Bound handlers for cleanup
    this.boundKeydownHandler = null;
    this.title = options.title ?? "Draw Your Signature";
    this.instructions = options.instructions ?? "Use your finger or mouse to sign below. Take your time.";
    this.labels = {
      startOver: options.labels?.startOver ?? "Start Over",
      undoStroke: options.labels?.undoStroke ?? "Undo Last Stroke",
      redoStroke: options.labels?.redoStroke ?? "Redo",
      useSignature: options.labels?.useSignature ?? "Use This Signature",
      cancel: options.labels?.cancel ?? "Cancel"
    };
    this.canvasHeight = options.canvasHeight ?? 220;
    this.strokeColor = options.strokeColor ?? "#000080";
    this.showGuides = options.showGuides ?? true;
    this.closeOnBackdrop = options.closeOnBackdrop ?? true;
    this.closeOnEscape = options.closeOnEscape ?? true;
    this.onAcceptCallback = options.onAccept ?? null;
    this.onCancelCallback = options.onCancel ?? null;
  }
  /**
   * Create the modal DOM structure
   */
  createModalDOM() {
    this.overlay = document.createElement("div");
    this.overlay.className = "signature-capture-modal-overlay modal-overlay";
    this.overlay.setAttribute("role", "dialog");
    this.overlay.setAttribute("aria-modal", "true");
    this.overlay.setAttribute("aria-labelledby", "sig-capture-modal-title");
    this.overlay.setAttribute(
      "aria-describedby",
      "sig-capture-modal-instructions"
    );
    this.modalEl = document.createElement("div");
    this.modalEl.className = "signature-capture-modal modal-content";
    this.modalEl.style.cssText = `
      max-width: 600px;
      width: 90%;
      max-height: 90vh;
      overflow-y: auto;
      padding: var(--spacing-lg, 32px);
      display: flex;
      flex-direction: column;
      gap: var(--spacing-md, 24px);
      background-color: var(--color-bg-primary, #ffffff);
      border-radius: var(--border-radius-lg, 12px);
    `;
    const header = this.createHeader();
    const instructionsEl = document.createElement("p");
    instructionsEl.id = "sig-capture-modal-instructions";
    instructionsEl.className = "signature-capture-modal-instructions";
    instructionsEl.textContent = this.instructions;
    instructionsEl.style.cssText = `
      font-size: var(--font-size-lg, 22px);
      color: var(--color-text-secondary, #4a4a4a);
      margin: 0;
      text-align: center;
      line-height: 1.5;
    `;
    const captureContainer = document.createElement("div");
    captureContainer.className = "signature-capture-modal-pad";
    captureContainer.style.cssText = `
      width: 100%;
      min-height: ${this.canvasHeight}px;
    `;
    const actionRow = this.createActionRow();
    const bottomRow = this.createBottomRow();
    this.modalEl.appendChild(header);
    this.modalEl.appendChild(instructionsEl);
    this.modalEl.appendChild(captureContainer);
    this.modalEl.appendChild(actionRow);
    this.modalEl.appendChild(bottomRow);
    this.overlay.appendChild(this.modalEl);
    this.btnUndo?.addEventListener("click", () => this.handleUndo());
    this.btnRedo?.addEventListener("click", () => this.handleRedo());
    this.btnStartOver?.addEventListener("click", () => this.handleStartOver());
    this.btnCancel?.addEventListener("click", () => this.handleCancel());
    this.btnUseSignature?.addEventListener("click", () => this.handleAccept());
    if (this.closeOnBackdrop) {
      this.overlay.addEventListener("click", (e) => {
        if (e.target === this.overlay) {
          this.handleCancel();
        }
      });
    }
    this.capture = new SignatureCapture({
      container: captureContainer,
      height: this.canvasHeight,
      strokeColor: this.strokeColor,
      showGuides: this.showGuides
    });
    this.capture.onchange = () => {
      this.updateButtonStates();
    };
  }
  /**
   * Create header with title and close button
   */
  createHeader() {
    const header = document.createElement("div");
    header.className = "signature-capture-modal-header";
    header.style.cssText = `
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
    `;
    const titleEl = document.createElement("h2");
    titleEl.id = "sig-capture-modal-title";
    titleEl.className = "signature-capture-modal-title";
    titleEl.textContent = this.title;
    titleEl.style.cssText = `
      font-size: var(--font-size-xl, 28px);
      font-weight: 700;
      margin: 0;
      color: var(--color-text-primary, #1a1a1a);
    `;
    const closeBtn = document.createElement("button");
    closeBtn.className = "signature-capture-modal-close";
    closeBtn.setAttribute("aria-label", "Close signature dialog");
    closeBtn.innerHTML = `
      <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    `;
    closeBtn.style.cssText = `
      background: none;
      border: none;
      cursor: pointer;
      padding: 8px;
      color: var(--color-text-secondary, #4a4a4a);
      min-width: 44px;
      min-height: 44px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: var(--border-radius, 8px);
      transition: background-color 0.2s;
    `;
    closeBtn.addEventListener("click", () => this.handleCancel());
    header.appendChild(titleEl);
    header.appendChild(closeBtn);
    return header;
  }
  /**
   * Create action row with undo/redo/clear buttons
   */
  createActionRow() {
    const actionRow = document.createElement("div");
    actionRow.className = "signature-capture-modal-actions";
    actionRow.style.cssText = `
      display: flex;
      justify-content: center;
      gap: var(--spacing-sm, 16px);
      flex-wrap: wrap;
    `;
    this.btnUndo = this.createButton(
      this.labels.undoStroke,
      "secondary",
      "undo-stroke",
      `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 7v6h6"></path>
        <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"></path>
      </svg>`
    );
    this.btnUndo.disabled = true;
    this.btnRedo = this.createButton(
      this.labels.redoStroke,
      "secondary",
      "redo-stroke",
      `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21 7v6h-6"></path>
        <path d="M3 17a9 9 0 0 1 9-9 9 9 0 0 1 6 2.3L21 13"></path>
      </svg>`
    );
    this.btnRedo.disabled = true;
    this.btnStartOver = this.createButton(
      this.labels.startOver,
      "secondary",
      "start-over",
      `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M2.5 2v6h6M2.66 15.57a10 10 0 1 0 .57-8.38"></path>
      </svg>`
    );
    this.btnStartOver.disabled = true;
    actionRow.appendChild(this.btnUndo);
    actionRow.appendChild(this.btnRedo);
    actionRow.appendChild(this.btnStartOver);
    return actionRow;
  }
  /**
   * Create bottom row with cancel and accept buttons
   */
  createBottomRow() {
    const bottomRow = document.createElement("div");
    bottomRow.className = "signature-capture-modal-bottom";
    bottomRow.style.cssText = `
      display: flex;
      justify-content: center;
      gap: var(--button-gap, 24px);
      flex-wrap: wrap;
      margin-top: var(--spacing-sm, 16px);
    `;
    this.btnCancel = this.createButton(this.labels.cancel, "secondary", "cancel");
    this.btnCancel.style.minWidth = "140px";
    this.btnUseSignature = this.createButton(
      this.labels.useSignature,
      "primary",
      "use-signature",
      `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="20 6 9 17 4 12"></polyline>
      </svg>`
    );
    this.btnUseSignature.classList.add("btn-large");
    this.btnUseSignature.style.cssText += `
      min-width: 220px;
      font-size: var(--font-size-xl, 28px);
    `;
    this.btnUseSignature.disabled = true;
    bottomRow.appendChild(this.btnCancel);
    bottomRow.appendChild(this.btnUseSignature);
    return bottomRow;
  }
  /**
   * Create a styled button
   */
  createButton(text, variant, action, icon) {
    const btn = document.createElement("button");
    btn.className = `btn-${variant} signature-capture-modal-btn`;
    btn.setAttribute("data-action", action);
    btn.style.cssText = `
      min-height: 60px;
      padding: var(--spacing-sm, 16px) var(--spacing-md, 24px);
      font-size: var(--font-size-action, 24px);
      font-weight: 600;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      border-radius: var(--border-radius, 8px);
      cursor: pointer;
      transition: all 0.2s;
    `;
    if (variant === "primary") {
      btn.style.cssText += `
        background-color: var(--color-action-bg, #0056b3);
        color: var(--color-action-text, #ffffff);
        border: 2px solid var(--color-action-border, #003d82);
      `;
    } else {
      btn.style.cssText += `
        background-color: var(--color-bg-primary, #ffffff);
        color: var(--color-action-bg, #0056b3);
        border: 2px solid var(--color-action-bg, #0056b3);
      `;
    }
    if (icon) {
      btn.innerHTML = `<span class="btn-icon" aria-hidden="true">${icon}</span><span>${text}</span>`;
    } else {
      btn.textContent = text;
    }
    return btn;
  }
  /**
   * Update button states based on signature state
   */
  updateButtonStates() {
    if (!this.capture) return;
    const isEmpty = this.capture.isEmpty();
    const canUndo = this.capture.canUndo();
    const canRedo = this.capture.canRedo();
    if (this.btnUndo) {
      this.btnUndo.disabled = !canUndo;
    }
    if (this.btnRedo) {
      this.btnRedo.disabled = !canRedo;
    }
    if (this.btnStartOver) {
      this.btnStartOver.disabled = isEmpty;
    }
    if (this.btnUseSignature) {
      this.btnUseSignature.disabled = isEmpty;
    }
  }
  /**
   * Handle keyboard events
   */
  handleKeydown(e) {
    if (e.key === "Escape" && this.closeOnEscape) {
      e.preventDefault();
      this.handleCancel();
    }
    if (e.key === "Tab" && this.modalEl) {
      const focusableElements = this.modalEl.querySelectorAll(
        'button:not([disabled]), [tabindex]:not([tabindex="-1"])'
      );
      const firstFocusable = focusableElements[0];
      const lastFocusable = focusableElements[focusableElements.length - 1];
      if (e.shiftKey && document.activeElement === firstFocusable) {
        e.preventDefault();
        lastFocusable?.focus();
      } else if (!e.shiftKey && document.activeElement === lastFocusable) {
        e.preventDefault();
        firstFocusable?.focus();
      }
    }
    if ((e.ctrlKey || e.metaKey) && e.key === "z") {
      e.preventDefault();
      if (e.shiftKey) {
        this.handleRedo();
      } else {
        this.handleUndo();
      }
    }
  }
  /**
   * Handle undo action
   */
  handleUndo() {
    this.capture?.undo();
    this.updateButtonStates();
  }
  /**
   * Handle redo action
   */
  handleRedo() {
    this.capture?.redo();
    this.updateButtonStates();
  }
  /**
   * Handle start over action
   */
  handleStartOver() {
    this.capture?.clear();
    this.updateButtonStates();
  }
  /**
   * Handle accept action
   */
  handleAccept() {
    if (!this.capture || this.capture.isEmpty()) return;
    const dataUrl = this.capture.toDataURL("png");
    const strokes = this.capture.getStrokes();
    this.close();
    if (this.onAcceptCallback) {
      this.onAcceptCallback(dataUrl, strokes);
    }
  }
  /**
   * Handle cancel action
   */
  handleCancel() {
    this.close();
    if (this.onCancelCallback) {
      this.onCancelCallback();
    }
  }
  /**
   * Announce to screen readers
   */
  announceToScreenReader(message) {
    const announcement = document.createElement("div");
    announcement.setAttribute("role", "status");
    announcement.setAttribute("aria-live", "polite");
    announcement.className = "visually-hidden";
    announcement.textContent = message;
    document.body.appendChild(announcement);
    setTimeout(() => {
      announcement.remove();
    }, 1e3);
  }
  // ========================================
  // Public API
  // ========================================
  /**
   * Open the signature modal
   */
  open() {
    if (this.isOpenState) return;
    this.previousActiveElement = document.activeElement;
    if (!this.overlay) {
      this.createModalDOM();
    }
    if (this.overlay) {
      document.body.appendChild(this.overlay);
    }
    document.body.style.overflow = "hidden";
    this.boundKeydownHandler = this.handleKeydown.bind(this);
    document.addEventListener("keydown", this.boundKeydownHandler);
    setTimeout(() => {
      if (this.capture) {
        this.capture.getCanvas().focus();
      }
    }, 100);
    this.isOpenState = true;
    this.announceToScreenReader("Signature dialog opened. Draw your signature.");
  }
  /**
   * Close the signature modal
   */
  close() {
    if (!this.isOpenState) return;
    if (this.boundKeydownHandler) {
      document.removeEventListener("keydown", this.boundKeydownHandler);
      this.boundKeydownHandler = null;
    }
    document.body.style.overflow = "";
    if (this.overlay) {
      this.overlay.remove();
    }
    if (this.capture) {
      this.capture.destroy();
      this.capture = null;
    }
    this.overlay = null;
    this.modalEl = null;
    this.btnUndo = null;
    this.btnRedo = null;
    this.btnStartOver = null;
    this.btnCancel = null;
    this.btnUseSignature = null;
    if (this.previousActiveElement instanceof HTMLElement) {
      this.previousActiveElement.focus();
    }
    this.isOpenState = false;
    this.announceToScreenReader("Signature dialog closed");
  }
  /**
   * Check if modal is currently open
   */
  isOpen() {
    return this.isOpenState;
  }
  /**
   * Get the SignatureCapture instance (if modal is open)
   */
  getCapture() {
    return this.capture;
  }
  /**
   * Destroy the modal completely
   */
  destroy() {
    this.close();
    this.onAcceptCallback = null;
    this.onCancelCallback = null;
  }
};
function createSignatureCaptureModal(options) {
  return new SignatureCaptureModal(options);
}
if (typeof window !== "undefined") {
  window.SignatureModal = SignatureModal;
  window.initSignatureModal = initSignatureModal;
  window.SignatureCaptureModal = SignatureCaptureModal;
  window.createSignatureCaptureModal = createSignatureCaptureModal;
}

// src/ts/main.ts
var DEFAULT_SYNC_ENDPOINT = "/api/signatures/sync";
function init() {
  initDocSignNamespace();
  initLocalSessionNamespace();
  const syncEndpoint = window.DOCSIGN_SYNC_ENDPOINT || DEFAULT_SYNC_ENDPOINT;
  initSyncManager({
    syncEndpoint,
    minBackoffMs: 1e3,
    maxBackoffMs: 3e4,
    retryIntervalMs: 3e4,
    maxRetries: 10
  });
  if (typeof window !== "undefined" && window.DocSign) {
    const docSign = window.DocSign;
    docSign.TypedSignature = TypedSignature;
    docSign.createTypedSignature = createTypedSignature;
    docSign.SIGNATURE_FONTS = SIGNATURE_FONTS;
    docSign.MobileSignatureModal = MobileSignatureModal;
    docSign.isMobileDevice = isMobileDevice;
    docSign.createSignatureModal = createSignatureModal;
    docSign.SignatureModal = SignatureModal;
    docSign.initSignatureModal = initSignatureModal;
    docSign.SignatureCapture = SignatureCapture;
    docSign.SignatureCaptureModal = SignatureCaptureModal;
    docSign.createSignatureCaptureModal = createSignatureCaptureModal;
  }
  console.log("DocSign TypeScript initialized");
  console.log("PDF Preview Bridge available:", typeof PdfPreviewBridge !== "undefined");
  console.log("DocSign namespace available:", typeof window.DocSign !== "undefined");
  console.log("LocalSessionManager available:", typeof LocalSessionManager !== "undefined");
  console.log("SyncManager available:", typeof SyncManager !== "undefined");
  console.log("TypedSignature available:", typeof TypedSignature !== "undefined");
  console.log("MobileSignatureModal available:", typeof MobileSignatureModal !== "undefined");
  console.log("SignatureCapture available:", typeof SignatureCapture !== "undefined");
  console.log("SignatureCaptureModal available:", typeof SignatureCaptureModal !== "undefined");
}
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
export {
  LocalSessionManager,
  MobileSignatureModal,
  PdfPreviewBridge,
  SIGNATURE_FONTS,
  SYNC_EVENTS,
  SignatureCapture,
  SignatureCaptureModal,
  SignatureModal,
  SyncManager,
  TypedSignature,
  categorizeError,
  createSignatureCaptureModal,
  createSignatureModal,
  createTypedSignature,
  createUserError,
  docSignPdfBridge,
  domPointToPdf,
  domRectToPdf,
  ensurePdfJsLoaded,
  getFileTooLargeError,
  getOfflineError,
  getPageRenderInfo,
  getSyncManager,
  getUnsupportedFileError,
  getUserFriendlyError,
  hideErrorModal,
  hideErrorToast,
  initDocSignNamespace,
  initLocalSessionNamespace,
  initSignatureModal,
  initSyncManager,
  isMobileDevice,
  isPdfJsLoaded,
  localSessionManager,
  onOnlineStatusChanged,
  onSyncCompleted,
  onSyncFailed,
  onSyncProgress,
  onSyncStarted,
  pdfPointToDom,
  pdfRectToDom,
  previewBridge,
  showConfirmDialog,
  showErrorModal,
  showErrorToast
};
//# sourceMappingURL=bundle.js.map
