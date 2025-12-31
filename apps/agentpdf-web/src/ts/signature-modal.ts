/**
 * Signature Modal Controller
 *
 * Manages the signature capture modal with both draw and type options.
 * Integrates with the TypedSignature component for enhanced typed signatures.
 *
 * Geriatric UX principles applied:
 * - Large 60px touch targets
 * - Clear tab labels
 * - High contrast colors
 * - Real-time feedback
 */

import { TypedSignature, SIGNATURE_FONTS } from "./typed-signature";
import { SignatureCapture } from "./signature-capture";
import type { Stroke } from "./signature-capture";

/**
 * Signature result returned when user applies their signature
 */
export interface SignatureResult {
  /** Field ID that was signed */
  fieldId: string | null;
  /** Signature as PNG data URL */
  signatureData: string;
  /** How the signature was created */
  mode: "draw" | "type";
  /** For typed signatures: the text */
  text?: string;
  /** For typed signatures: the font used */
  font?: string;
}

/**
 * Options for SignatureModal
 */
export interface SignatureModalOptions {
  /** Callback when signature is applied */
  onApply?: (result: SignatureResult) => void;
  /** Callback when modal is cancelled */
  onCancel?: () => void;
  /** Pen color for drawing (default: #000000) */
  penColor?: string;
  /** Pen width for drawing (default: 2) */
  penWidth?: number;
}

/**
 * SignatureModal class
 *
 * Controls the signature capture modal, supporting both drawn and typed signatures.
 */
export class SignatureModal {
  private modal: HTMLElement;
  private onApply: (result: SignatureResult) => void;
  private onCancel: () => void;
  private penColor: string;
  private penWidth: number;

  // Mode state
  private mode: "draw" | "type" = "draw";
  private currentFieldId: string | null = null;

  // DOM elements
  private drawTab: HTMLElement | null = null;
  private typeTab: HTMLElement | null = null;
  private drawPanel: HTMLElement | null = null;
  private typePanel: HTMLElement | null = null;
  private drawTabBtn: HTMLElement | null = null;
  private typeTabBtn: HTMLElement | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private ctx: CanvasRenderingContext2D | null = null;
  private clearBtn: HTMLElement | null = null;
  private applyBtn: HTMLElement | null = null;
  private cancelBtn: HTMLElement | null = null;
  private closeBtn: HTMLElement | null = null;

  // Typed signature component
  private typedSignature: TypedSignature | null = null;
  private typedSignatureContainer: HTMLElement | null = null;

  // Drawing state
  private isDrawing: boolean = false;
  private lastX: number = 0;
  private lastY: number = 0;
  private hasDrawn: boolean = false;

  constructor(modalElement: HTMLElement, options: SignatureModalOptions = {}) {
    this.modal = modalElement;
    this.onApply = options.onApply || (() => {});
    this.onCancel = options.onCancel || (() => {});
    this.penColor = options.penColor || "#000000";
    this.penWidth = options.penWidth || 2;

    this.initializeElements();
    this.bindEvents();
    this.enhanceTypeTab();
  }

  /**
   * Initialize DOM element references
   */
  private initializeElements(): void {
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
  private bindEvents(): void {
    // Tab switching
    this.drawTabBtn?.addEventListener("click", () => this.switchTab("draw"));
    this.typeTabBtn?.addEventListener("click", () => this.switchTab("type"));

    // Clear button
    this.clearBtn?.addEventListener("click", () => this.clearCanvas());

    // Apply button
    this.applyBtn?.addEventListener("click", () => this.apply());

    // Cancel and close buttons
    this.cancelBtn?.addEventListener("click", () => this.hide());
    this.closeBtn?.addEventListener("click", () => this.hide());

    // Close on backdrop click
    this.modal.addEventListener("click", (e) => {
      if (e.target === this.modal) {
        this.hide();
      }
    });

    // Close on Escape key
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && !this.modal.classList.contains("hidden")) {
        this.hide();
      }
    });

    // Canvas drawing events
    if (this.canvas) {
      this.bindCanvasEvents();
    }
  }

  /**
   * Bind canvas drawing events
   */
  private bindCanvasEvents(): void {
    if (!this.canvas) return;

    // Mouse events
    this.canvas.addEventListener("mousedown", this.handlePointerDown.bind(this));
    this.canvas.addEventListener("mousemove", this.handlePointerMove.bind(this));
    this.canvas.addEventListener("mouseup", this.handlePointerUp.bind(this));
    this.canvas.addEventListener("mouseleave", this.handlePointerUp.bind(this));

    // Touch events
    this.canvas.addEventListener("touchstart", this.handleTouchStart.bind(this));
    this.canvas.addEventListener("touchmove", this.handleTouchMove.bind(this));
    this.canvas.addEventListener("touchend", this.handlePointerUp.bind(this));

    // Prevent scrolling on touch
    this.canvas.addEventListener("touchstart", (e) => e.preventDefault());
    this.canvas.addEventListener("touchmove", (e) => e.preventDefault());
  }

  /**
   * Enhance the type tab with the new TypedSignature component
   */
  private enhanceTypeTab(): void {
    if (!this.typePanel) return;

    // Clear existing content
    this.typePanel.innerHTML = "";

    // Create container for TypedSignature
    this.typedSignatureContainer = document.createElement("div");
    this.typedSignatureContainer.id = "typed-signature-container";
    this.typePanel.appendChild(this.typedSignatureContainer);

    // Initialize TypedSignature component
    this.typedSignature = new TypedSignature({
      container: this.typedSignatureContainer,
      fonts: SIGNATURE_FONTS.map((f) => f.name),
      defaultFont: "Dancing Script",
      fontSize: 48,
      textColor: "#000080",
      backgroundColor: "#ffffff",
      placeholder: "Type your full name",
    });
  }

  /**
   * Switch between draw and type tabs
   */
  private switchTab(tab: "draw" | "type"): void {
    this.mode = tab;

    if (tab === "draw") {
      this.drawTabBtn?.classList.add("active");
      this.typeTabBtn?.classList.remove("active");
      this.drawPanel?.classList.add("active");
      this.drawPanel?.classList.remove("hidden");
      this.typePanel?.classList.remove("active");
      this.typePanel?.classList.add("hidden");

      // Re-initialize canvas when switching to draw
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
  private initCanvas(): void {
    if (!this.canvas || !this.ctx) return;

    const rect = this.canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;

    // Set canvas size accounting for DPR
    this.canvas.width = rect.width * dpr;
    this.canvas.height = rect.height * dpr;
    this.ctx.scale(dpr, dpr);

    // Set drawing styles - thicker on mobile
    const isMobile = window.innerWidth < 768;
    this.ctx.strokeStyle = this.penColor;
    this.ctx.lineWidth = isMobile ? Math.max(this.penWidth, 3) : this.penWidth;
    this.ctx.lineCap = "round";
    this.ctx.lineJoin = "round";

    // Clear canvas
    this.clearCanvas();
  }

  /**
   * Clear the drawing canvas
   */
  private clearCanvas(): void {
    if (!this.canvas || !this.ctx) return;

    this.ctx.fillStyle = "#ffffff";
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    this.ctx.fillStyle = this.penColor;
    this.hasDrawn = false;
  }

  /**
   * Get pointer position from mouse event
   */
  private getPointerPos(e: MouseEvent): { x: number; y: number } {
    if (!this.canvas) return { x: 0, y: 0 };
    const rect = this.canvas.getBoundingClientRect();
    return {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
    };
  }

  /**
   * Get pointer position from touch event
   */
  private getTouchPos(e: TouchEvent): { x: number; y: number } {
    if (!this.canvas) return { x: 0, y: 0 };
    const rect = this.canvas.getBoundingClientRect();
    const touch = e.touches[0];
    return {
      x: touch.clientX - rect.left,
      y: touch.clientY - rect.top,
    };
  }

  /**
   * Handle pointer down (start drawing)
   */
  private handlePointerDown(e: MouseEvent): void {
    this.isDrawing = true;
    const pos = this.getPointerPos(e);
    this.lastX = pos.x;
    this.lastY = pos.y;

    // Draw a dot for single clicks
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
  private handlePointerMove(e: MouseEvent): void {
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
  private handlePointerUp(): void {
    this.isDrawing = false;
  }

  /**
   * Handle touch start
   */
  private handleTouchStart(e: TouchEvent): void {
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
  private handleTouchMove(e: TouchEvent): void {
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
  private isCanvasEmpty(): boolean {
    return !this.hasDrawn;
  }

  /**
   * Apply the signature
   */
  private apply(): void {
    let signatureData: string;
    let text: string | undefined;
    let font: string | undefined;

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

    const result: SignatureResult = {
      fieldId: this.currentFieldId,
      signatureData,
      mode: this.mode,
      text,
      font,
    };

    this.onApply(result);
    this.hide();
  }

  /**
   * Show the modal for a specific field
   */
  show(fieldId?: string): void {
    this.currentFieldId = fieldId || null;
    this.modal.classList.remove("hidden");

    // Add mobile bottom-sheet class if on mobile
    const modalContent = this.modal.querySelector(".modal") as HTMLElement;
    if (modalContent && window.innerWidth < 768) {
      modalContent.classList.add("bottom-sheet-mobile");
    }

    // Reset to draw tab
    this.switchTab("draw");

    // Clear previous signature
    this.clearCanvas();
    if (this.typedSignature) {
      this.typedSignature.setText("");
    }
  }

  /**
   * Hide the modal
   */
  hide(): void {
    this.modal.classList.add("hidden");
    this.onCancel();
  }

  /**
   * Destroy the modal controller
   */
  destroy(): void {
    if (this.typedSignature) {
      this.typedSignature.destroy();
      this.typedSignature = null;
    }
  }
}

/**
 * Initialize signature modal from existing HTML element
 */
export function initSignatureModal(
  modalElement: HTMLElement,
  options: SignatureModalOptions = {}
): SignatureModal {
  return new SignatureModal(modalElement, options);
}

// ============================================================================
// SignatureCaptureModal - Phase 3: Self-contained modal with improved capture
// ============================================================================

/**
 * Options for SignatureCaptureModal (Phase 3)
 */
export interface SignatureCaptureModalOptions {
  /** Modal title (default: "Draw Your Signature") */
  title?: string;
  /** Instructions text (default: "Use your finger or mouse...") */
  instructions?: string;
  /** Button labels */
  labels?: {
    startOver?: string;
    undoStroke?: string;
    redoStroke?: string;
    useSignature?: string;
    cancel?: string;
  };
  /** Canvas height in pixels (default: 220) */
  canvasHeight?: number;
  /** Stroke color (default: '#000080' navy blue) */
  strokeColor?: string;
  /** Show baseline guide (default: true) */
  showGuides?: boolean;
  /** Called when signature is accepted */
  onAccept?: (dataUrl: string, strokes: Stroke[]) => void;
  /** Called when modal is cancelled/closed */
  onCancel?: () => void;
  /** Allow close by clicking backdrop (default: true) */
  closeOnBackdrop?: boolean;
  /** Allow close by pressing Escape (default: true) */
  closeOnEscape?: boolean;
}

/**
 * SignatureCaptureModal - Self-contained modal with improved SignatureCapture
 *
 * AGENTPDF_PLAN Phase 3: Creates its own DOM structure with geriatric-friendly UX
 *
 * Features:
 * - Large modal (90% width on mobile, 600px max on desktop)
 * - Clear title and instructions
 * - "Start Over", "Undo Last Stroke", "Redo" buttons
 * - "Use This Signature" button (60px height)
 * - Full accessibility support
 *
 * @example
 * ```typescript
 * const modal = new SignatureCaptureModal({
 *   onAccept: (dataUrl, strokes) => {
 *     console.log('Signature accepted:', dataUrl);
 *   }
 * });
 *
 * modal.open();
 * ```
 */
export class SignatureCaptureModal {
  // Configuration
  private title: string;
  private instructions: string;
  private labels: Required<NonNullable<SignatureCaptureModalOptions["labels"]>>;
  private canvasHeight: number;
  private strokeColor: string;
  private showGuides: boolean;
  private closeOnBackdrop: boolean;
  private closeOnEscape: boolean;

  // Callbacks
  private onAcceptCallback: ((dataUrl: string, strokes: Stroke[]) => void) | null;
  private onCancelCallback: (() => void) | null;

  // DOM elements
  private overlay: HTMLDivElement | null = null;
  private modalEl: HTMLDivElement | null = null;
  private capture: SignatureCapture | null = null;

  // Buttons
  private btnStartOver: HTMLButtonElement | null = null;
  private btnUndo: HTMLButtonElement | null = null;
  private btnRedo: HTMLButtonElement | null = null;
  private btnUseSignature: HTMLButtonElement | null = null;
  private btnCancel: HTMLButtonElement | null = null;

  // State
  private isOpenState: boolean = false;
  private previousActiveElement: Element | null = null;

  // Bound handlers for cleanup
  private boundKeydownHandler: ((e: KeyboardEvent) => void) | null = null;

  constructor(options: SignatureCaptureModalOptions = {}) {
    this.title = options.title ?? "Draw Your Signature";
    this.instructions =
      options.instructions ??
      "Use your finger or mouse to sign below. Take your time.";
    this.labels = {
      startOver: options.labels?.startOver ?? "Start Over",
      undoStroke: options.labels?.undoStroke ?? "Undo Last Stroke",
      redoStroke: options.labels?.redoStroke ?? "Redo",
      useSignature: options.labels?.useSignature ?? "Use This Signature",
      cancel: options.labels?.cancel ?? "Cancel",
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
  private createModalDOM(): void {
    // Create overlay
    this.overlay = document.createElement("div");
    this.overlay.className = "signature-capture-modal-overlay modal-overlay";
    this.overlay.setAttribute("role", "dialog");
    this.overlay.setAttribute("aria-modal", "true");
    this.overlay.setAttribute("aria-labelledby", "sig-capture-modal-title");
    this.overlay.setAttribute(
      "aria-describedby",
      "sig-capture-modal-instructions"
    );

    // Create modal container
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

    // Create header with title and close button
    const header = this.createHeader();

    // Instructions
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

    // Capture container
    const captureContainer = document.createElement("div");
    captureContainer.className = "signature-capture-modal-pad";
    captureContainer.style.cssText = `
      width: 100%;
      min-height: ${this.canvasHeight}px;
    `;

    // Action buttons row (undo/redo/clear)
    const actionRow = this.createActionRow();

    // Bottom buttons row (Cancel / Use Signature)
    const bottomRow = this.createBottomRow();

    // Assemble modal
    this.modalEl.appendChild(header);
    this.modalEl.appendChild(instructionsEl);
    this.modalEl.appendChild(captureContainer);
    this.modalEl.appendChild(actionRow);
    this.modalEl.appendChild(bottomRow);
    this.overlay.appendChild(this.modalEl);

    // Event listeners for buttons
    this.btnUndo?.addEventListener("click", () => this.handleUndo());
    this.btnRedo?.addEventListener("click", () => this.handleRedo());
    this.btnStartOver?.addEventListener("click", () => this.handleStartOver());
    this.btnCancel?.addEventListener("click", () => this.handleCancel());
    this.btnUseSignature?.addEventListener("click", () => this.handleAccept());

    // Backdrop click
    if (this.closeOnBackdrop) {
      this.overlay.addEventListener("click", (e) => {
        if (e.target === this.overlay) {
          this.handleCancel();
        }
      });
    }

    // Create SignatureCapture component
    this.capture = new SignatureCapture({
      container: captureContainer,
      height: this.canvasHeight,
      strokeColor: this.strokeColor,
      showGuides: this.showGuides,
    });

    // Listen for signature changes
    this.capture.onchange = () => {
      this.updateButtonStates();
    };
  }

  /**
   * Create header with title and close button
   */
  private createHeader(): HTMLDivElement {
    const header = document.createElement("div");
    header.className = "signature-capture-modal-header";
    header.style.cssText = `
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
    `;

    // Title
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

    // Close button (X)
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
  private createActionRow(): HTMLDivElement {
    const actionRow = document.createElement("div");
    actionRow.className = "signature-capture-modal-actions";
    actionRow.style.cssText = `
      display: flex;
      justify-content: center;
      gap: var(--spacing-sm, 16px);
      flex-wrap: wrap;
    `;

    // Undo button with icon
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

    // Redo button with icon
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

    // Start Over button with icon
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
  private createBottomRow(): HTMLDivElement {
    const bottomRow = document.createElement("div");
    bottomRow.className = "signature-capture-modal-bottom";
    bottomRow.style.cssText = `
      display: flex;
      justify-content: center;
      gap: var(--button-gap, 24px);
      flex-wrap: wrap;
      margin-top: var(--spacing-sm, 16px);
    `;

    // Cancel button
    this.btnCancel = this.createButton(this.labels.cancel, "secondary", "cancel");
    this.btnCancel.style.minWidth = "140px";

    // Use Signature button (primary, large)
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
  private createButton(
    text: string,
    variant: "primary" | "secondary",
    action: string,
    icon?: string
  ): HTMLButtonElement {
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
  private updateButtonStates(): void {
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
  private handleKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape" && this.closeOnEscape) {
      e.preventDefault();
      this.handleCancel();
    }

    // Tab trap
    if (e.key === "Tab" && this.modalEl) {
      const focusableElements = this.modalEl.querySelectorAll<HTMLElement>(
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

    // Undo/Redo keyboard shortcuts
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
  private handleUndo(): void {
    this.capture?.undo();
    this.updateButtonStates();
  }

  /**
   * Handle redo action
   */
  private handleRedo(): void {
    this.capture?.redo();
    this.updateButtonStates();
  }

  /**
   * Handle start over action
   */
  private handleStartOver(): void {
    this.capture?.clear();
    this.updateButtonStates();
  }

  /**
   * Handle accept action
   */
  private handleAccept(): void {
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
  private handleCancel(): void {
    this.close();

    if (this.onCancelCallback) {
      this.onCancelCallback();
    }
  }

  /**
   * Announce to screen readers
   */
  private announceToScreenReader(message: string): void {
    const announcement = document.createElement("div");
    announcement.setAttribute("role", "status");
    announcement.setAttribute("aria-live", "polite");
    announcement.className = "visually-hidden";
    announcement.textContent = message;
    document.body.appendChild(announcement);

    setTimeout(() => {
      announcement.remove();
    }, 1000);
  }

  // ========================================
  // Public API
  // ========================================

  /**
   * Open the signature modal
   */
  public open(): void {
    if (this.isOpenState) return;

    // Store current focus
    this.previousActiveElement = document.activeElement;

    // Create DOM if not exists
    if (!this.overlay) {
      this.createModalDOM();
    }

    // Add to document
    if (this.overlay) {
      document.body.appendChild(this.overlay);
    }

    // Prevent body scroll
    document.body.style.overflow = "hidden";

    // Set up keyboard handler
    this.boundKeydownHandler = this.handleKeydown.bind(this);
    document.addEventListener("keydown", this.boundKeydownHandler);

    // Focus the canvas
    setTimeout(() => {
      if (this.capture) {
        this.capture.getCanvas().focus();
      }
    }, 100);

    this.isOpenState = true;

    // Announce to screen readers
    this.announceToScreenReader("Signature dialog opened. Draw your signature.");
  }

  /**
   * Close the signature modal
   */
  public close(): void {
    if (!this.isOpenState) return;

    // Remove keyboard handler
    if (this.boundKeydownHandler) {
      document.removeEventListener("keydown", this.boundKeydownHandler);
      this.boundKeydownHandler = null;
    }

    // Restore body scroll
    document.body.style.overflow = "";

    // Remove from DOM
    if (this.overlay) {
      this.overlay.remove();
    }

    // Destroy capture component
    if (this.capture) {
      this.capture.destroy();
      this.capture = null;
    }

    // Reset DOM references
    this.overlay = null;
    this.modalEl = null;
    this.btnUndo = null;
    this.btnRedo = null;
    this.btnStartOver = null;
    this.btnCancel = null;
    this.btnUseSignature = null;

    // Restore focus
    if (this.previousActiveElement instanceof HTMLElement) {
      this.previousActiveElement.focus();
    }

    this.isOpenState = false;

    // Announce to screen readers
    this.announceToScreenReader("Signature dialog closed");
  }

  /**
   * Check if modal is currently open
   */
  public isOpen(): boolean {
    return this.isOpenState;
  }

  /**
   * Get the SignatureCapture instance (if modal is open)
   */
  public getCapture(): SignatureCapture | null {
    return this.capture;
  }

  /**
   * Destroy the modal completely
   */
  public destroy(): void {
    this.close();
    this.onAcceptCallback = null;
    this.onCancelCallback = null;
  }
}

/**
 * Factory function to create SignatureCaptureModal
 */
export function createSignatureCaptureModal(
  options?: SignatureCaptureModalOptions
): SignatureCaptureModal {
  return new SignatureCaptureModal(options);
}

// Export for window access
if (typeof window !== "undefined") {
  (window as unknown as { SignatureModal: typeof SignatureModal }).SignatureModal =
    SignatureModal;
  (
    window as unknown as { initSignatureModal: typeof initSignatureModal }
  ).initSignatureModal = initSignatureModal;
  (
    window as unknown as { SignatureCaptureModal: typeof SignatureCaptureModal }
  ).SignatureCaptureModal = SignatureCaptureModal;
  (
    window as unknown as {
      createSignatureCaptureModal: typeof createSignatureCaptureModal;
    }
  ).createSignatureCaptureModal = createSignatureCaptureModal;
}
