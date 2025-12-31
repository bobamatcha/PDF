/**
 * Mobile Signature Modal
 *
 * A mobile-optimized signature modal that provides the best possible
 * signing experience on phones and tablets. Designed for elderly users
 * with focus on large touch targets, clear feedback, and orientation handling.
 *
 * Features:
 * - Full-screen modal on phones (100vh, 100vw)
 * - Large 60px touch targets for all interactive elements
 * - Orientation detection with landscape encouragement
 * - Palm rejection and multi-touch handling
 * - Smooth stroke rendering with requestAnimationFrame
 * - Focus trap and keyboard accessibility
 * - High contrast mode support
 */

/**
 * Options for configuring the mobile signature modal
 */
export interface MobileModalOptions {
  /** Title shown in the modal header */
  title?: string;
  /** Instructions displayed above the signature area */
  instructions?: string;
  /** Callback when signature is completed */
  onComplete?: (signature: string) => void;
  /** Callback when modal is cancelled */
  onCancel?: () => void;
}

/**
 * Result returned when a signature is captured
 */
export interface SignatureResult {
  /** Base64 data URL of the signature image */
  dataUrl: string;
  /** Type of signature: drawn or typed */
  type: "drawn" | "typed";
  /** ISO timestamp when signature was captured */
  timestamp: string;
}

/**
 * Point coordinates for drawing
 */
interface Point {
  x: number;
  y: number;
  pressure?: number;
}

/**
 * Stroke data for undo functionality
 */
interface Stroke {
  points: Point[];
  color: string;
  width: number;
}

/**
 * MobileSignatureModal Class
 *
 * Creates a full-screen, mobile-optimized signature capture modal.
 * Handles touch events, orientation changes, and provides a smooth
 * signing experience for elderly users.
 */
export class MobileSignatureModal {
  private options: Required<MobileModalOptions>;
  private modalElement: HTMLDivElement | null = null;
  private canvasElement: HTMLCanvasElement | null = null;
  private ctx: CanvasRenderingContext2D | null = null;
  private isModalOpen = false;

  // Drawing state
  private isDrawing = false;
  private points: Point[] = [];
  private strokes: Stroke[] = [];
  private currentStroke: Stroke | null = null;
  private animationFrameId: number | null = null;

  // Touch state
  private activeTouchId: number | null = null;

  // Pen settings
  private penColor = "#000000";
  private penWidth = 3;

  // Resize debounce
  private resizeTimeout: number | null = null;

  // Focus trap elements
  private focusableElements: HTMLElement[] = [];
  private firstFocusable: HTMLElement | null = null;
  private lastFocusable: HTMLElement | null = null;
  private previousActiveElement: Element | null = null;

  // Promise resolution for open() method
  private resolvePromise: ((result: SignatureResult | null) => void) | null = null;

  constructor(options: MobileModalOptions = {}) {
    this.options = {
      title: options.title ?? "Sign Here",
      instructions: options.instructions ?? "Draw your signature with your finger",
      onComplete: options.onComplete ?? (() => {}),
      onCancel: options.onCancel ?? (() => {}),
    };

    // Bind event handlers
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
  public open(): Promise<SignatureResult | null> {
    return new Promise((resolve) => {
      this.resolvePromise = resolve;
      this.showModal();
    });
  }

  /**
   * Closes the modal without saving
   */
  public close(): void {
    this.hideModal(null);
  }

  /**
   * Returns whether the modal is currently open
   */
  public isOpen(): boolean {
    return this.isModalOpen;
  }

  /**
   * Creates and shows the modal
   */
  private showModal(): void {
    if (this.isModalOpen) return;

    // Store previous active element for focus restoration
    this.previousActiveElement = document.activeElement;

    // Create modal structure
    this.createModalDOM();

    // Add to document
    document.body.appendChild(this.modalElement!);

    // Lock body scroll
    document.body.style.overflow = "hidden";
    document.body.style.position = "fixed";
    document.body.style.width = "100%";
    document.body.style.height = "100%";

    // Add event listeners
    this.addEventListeners();

    // Initialize canvas after DOM is ready
    requestAnimationFrame(() => {
      this.initializeCanvas();
      this.setupFocusTrap();

      // Force layout recalc for animations
      this.modalElement!.offsetHeight;

      // Animate in
      this.modalElement!.classList.add("mobile-signature-modal--visible");
    });

    this.isModalOpen = true;
  }

  /**
   * Hides and destroys the modal
   */
  private hideModal(result: SignatureResult | null): void {
    if (!this.isModalOpen) return;

    // Remove event listeners
    this.removeEventListeners();

    // Animate out
    this.modalElement?.classList.remove("mobile-signature-modal--visible");

    // Wait for animation before cleanup
    setTimeout(() => {
      // Restore body scroll
      document.body.style.overflow = "";
      document.body.style.position = "";
      document.body.style.width = "";
      document.body.style.height = "";

      // Remove from DOM
      this.modalElement?.remove();
      this.modalElement = null;
      this.canvasElement = null;
      this.ctx = null;

      // Restore focus
      if (this.previousActiveElement && "focus" in this.previousActiveElement) {
        (this.previousActiveElement as HTMLElement).focus();
      }

      this.isModalOpen = false;

      // Resolve promise
      if (this.resolvePromise) {
        this.resolvePromise(result);
        this.resolvePromise = null;
      }

      // Call callbacks
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
  private createModalDOM(): void {
    // Inject CSS if not already present
    this.injectStyles();

    // Create modal container
    const modal = document.createElement("div");
    modal.className = "mobile-signature-modal";
    modal.setAttribute("role", "dialog");
    modal.setAttribute("aria-modal", "true");
    modal.setAttribute("aria-label", this.options.title);

    // Detect orientation
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

    // Bind button actions
    modal.querySelectorAll("[data-action]").forEach((btn) => {
      const action = btn.getAttribute("data-action");
      btn.addEventListener("click", () => this.handleAction(action!));
    });
  }

  /**
   * Creates the rotate hint HTML
   */
  private createRotateHint(): string {
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
  private injectStyles(): void {
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
  private initializeCanvas(): void {
    if (!this.canvasElement) return;

    const wrapper = this.canvasElement.parentElement;
    if (!wrapper) return;

    // Get the wrapper dimensions
    const rect = wrapper.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;

    // Set canvas size
    this.canvasElement.width = rect.width * dpr;
    this.canvasElement.height = rect.height * dpr;

    // Get context and scale
    this.ctx = this.canvasElement.getContext("2d");
    if (!this.ctx) return;

    this.ctx.scale(dpr, dpr);

    // Configure stroke style
    this.ctx.strokeStyle = this.penColor;
    this.ctx.lineWidth = this.penWidth;
    this.ctx.lineCap = "round";
    this.ctx.lineJoin = "round";
    this.ctx.fillStyle = this.penColor;

    // Clear canvas
    this.clearCanvas();

    // Redraw existing strokes
    this.redrawStrokes();
  }

  /**
   * Adds event listeners
   */
  private addEventListeners(): void {
    // Keyboard
    document.addEventListener("keydown", this.handleKeyDown);

    // Resize and orientation
    window.addEventListener("resize", this.handleResize);
    window.addEventListener("orientationchange", this.handleOrientationChange);

    // Canvas touch events (passive: false to prevent scroll)
    if (this.canvasElement) {
      this.canvasElement.addEventListener("touchstart", this.handleTouchStart, {
        passive: false,
      });
      this.canvasElement.addEventListener("touchmove", this.handleTouchMove, {
        passive: false,
      });
      this.canvasElement.addEventListener("touchend", this.handleTouchEnd, {
        passive: false,
      });
      this.canvasElement.addEventListener("touchcancel", this.handleTouchEnd, {
        passive: false,
      });

      // Mouse events for testing on desktop
      this.canvasElement.addEventListener("mousedown", this.handleMouseDown);
      this.canvasElement.addEventListener("mousemove", this.handleMouseMove);
      this.canvasElement.addEventListener("mouseup", this.handleMouseUp);
      this.canvasElement.addEventListener("mouseleave", this.handleMouseUp);
    }
  }

  /**
   * Removes event listeners
   */
  private removeEventListeners(): void {
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

    // Cancel any pending animation frame
    if (this.animationFrameId !== null) {
      cancelAnimationFrame(this.animationFrameId);
      this.animationFrameId = null;
    }
  }

  /**
   * Sets up the focus trap
   */
  private setupFocusTrap(): void {
    if (!this.modalElement) return;

    // Find all focusable elements
    this.focusableElements = Array.from(
      this.modalElement.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      )
    ).filter((el) => !el.hasAttribute("disabled") && el.offsetParent !== null);

    if (this.focusableElements.length > 0) {
      this.firstFocusable = this.focusableElements[0];
      this.lastFocusable = this.focusableElements[this.focusableElements.length - 1];

      // Focus the canvas initially for immediate signing
      const canvas = this.modalElement.querySelector<HTMLElement>(".mobile-signature-canvas");
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
  private handleKeyDown(e: KeyboardEvent): void {
    // Escape to close
    if (e.key === "Escape") {
      e.preventDefault();
      this.close();
      return;
    }

    // Tab for focus trap
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
  private handleResize(): void {
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
  private handleOrientationChange(): void {
    // Wait for orientation change to complete
    setTimeout(() => {
      this.initializeCanvas();
      this.updateRotateHint();
    }, 100);
  }

  /**
   * Updates the rotate hint visibility
   */
  private updateRotateHint(): void {
    if (!this.modalElement) return;

    const hint = this.modalElement.querySelector(".rotate-hint");
    const existingHint = hint !== null;
    const shouldShow = this.isPortrait();

    if (shouldShow && !existingHint) {
      // Add hint
      const canvasArea = this.modalElement.querySelector(".mobile-signature-canvas-area");
      const wrapper = this.modalElement.querySelector(".mobile-signature-canvas-wrapper");
      if (canvasArea && wrapper) {
        const hintEl = document.createElement("div");
        hintEl.innerHTML = this.createRotateHint();
        canvasArea.insertBefore(hintEl.firstElementChild!, wrapper);
      }
    } else if (!shouldShow && existingHint) {
      // Remove hint
      hint?.remove();
    }
  }

  /**
   * Checks if device is in portrait orientation
   */
  private isPortrait(): boolean {
    return window.innerHeight > window.innerWidth;
  }

  /**
   * Handles touch start
   */
  private handleTouchStart(e: TouchEvent): void {
    e.preventDefault();

    // Ignore if already drawing (multi-touch rejection)
    if (this.activeTouchId !== null) return;

    const touch = e.touches[0];

    // Palm rejection: ignore if touch radius is too large (> 30px suggests palm)
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
  private handleTouchMove(e: TouchEvent): void {
    e.preventDefault();

    if (this.activeTouchId === null) return;

    // Find our tracked touch
    let touch: Touch | null = null;
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
  private handleTouchEnd(e: TouchEvent): void {
    e.preventDefault();

    // Check if our tracked touch ended
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
  private handleMouseDown(e: MouseEvent): void {
    const point = this.getMousePos(e);
    this.startStroke(point);
  }

  /**
   * Handles mouse move (for desktop testing)
   */
  private handleMouseMove(e: MouseEvent): void {
    if (!this.isDrawing) return;
    const point = this.getMousePos(e);
    this.continueStroke(point);
  }

  /**
   * Handles mouse up (for desktop testing)
   */
  private handleMouseUp(): void {
    this.endStroke();
  }

  /**
   * Gets point from touch event
   */
  private getTouchPos(touch: Touch): Point {
    if (!this.canvasElement) return { x: 0, y: 0 };

    const rect = this.canvasElement.getBoundingClientRect();
    return {
      x: touch.clientX - rect.left,
      y: touch.clientY - rect.top,
      pressure: touch.force || 0.5,
    };
  }

  /**
   * Gets point from mouse event
   */
  private getMousePos(e: MouseEvent): Point {
    if (!this.canvasElement) return { x: 0, y: 0 };

    const rect = this.canvasElement.getBoundingClientRect();
    return {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
      pressure: 0.5,
    };
  }

  /**
   * Starts a new stroke
   */
  private startStroke(point: Point): void {
    this.isDrawing = true;
    this.points = [point];

    this.currentStroke = {
      points: [point],
      color: this.penColor,
      width: this.penWidth,
    };

    // Draw a dot at the start point
    if (this.ctx) {
      this.ctx.beginPath();
      this.ctx.arc(point.x, point.y, this.penWidth / 2, 0, Math.PI * 2);
      this.ctx.fill();
    }
  }

  /**
   * Continues the current stroke
   */
  private continueStroke(point: Point): void {
    if (!this.isDrawing || !this.ctx || !this.currentStroke) return;

    this.points.push(point);
    this.currentStroke.points.push(point);

    // Use requestAnimationFrame for smooth rendering
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
  private renderPendingPoints(): void {
    if (!this.ctx || this.points.length < 2) return;

    // Use quadratic bezier for smooth curves
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
      // Simple line for first few points
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
  private endStroke(): void {
    if (!this.isDrawing) return;

    this.isDrawing = false;

    // Save stroke for undo
    if (this.currentStroke && this.currentStroke.points.length > 0) {
      this.strokes.push(this.currentStroke);
    }

    this.currentStroke = null;
    this.points = [];
  }

  /**
   * Clears the canvas
   */
  private clearCanvas(): void {
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
  private redrawStrokes(): void {
    if (!this.ctx) return;

    for (const stroke of this.strokes) {
      this.ctx.strokeStyle = stroke.color;
      this.ctx.lineWidth = stroke.width;
      this.ctx.fillStyle = stroke.color;

      if (stroke.points.length === 0) continue;

      // Draw first point as dot
      const firstPoint = stroke.points[0];
      this.ctx.beginPath();
      this.ctx.arc(firstPoint.x, firstPoint.y, stroke.width / 2, 0, Math.PI * 2);
      this.ctx.fill();

      // Draw curves
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

        // Draw remaining segments as lines
        if (stroke.points.length === 2) {
          this.ctx.beginPath();
          this.ctx.moveTo(stroke.points[0].x, stroke.points[0].y);
          this.ctx.lineTo(stroke.points[1].x, stroke.points[1].y);
          this.ctx.stroke();
        }
      }
    }

    // Restore original pen settings
    this.ctx.strokeStyle = this.penColor;
    this.ctx.lineWidth = this.penWidth;
    this.ctx.fillStyle = this.penColor;
  }

  /**
   * Handles button actions
   */
  private handleAction(action: string): void {
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
  private saveAndClose(): void {
    if (!this.canvasElement) {
      this.close();
      return;
    }

    // Check if signature is empty
    if (this.strokes.length === 0) {
      // Show a brief error message
      this.showValidationError("Please draw your signature first");
      return;
    }

    // Get the signature as data URL
    const dataUrl = this.canvasElement.toDataURL("image/png");

    const result: SignatureResult = {
      dataUrl,
      type: "drawn",
      timestamp: new Date().toISOString(),
    };

    this.hideModal(result);
  }

  /**
   * Shows a validation error message
   */
  private showValidationError(message: string): void {
    // Use native alert for simplicity and accessibility
    // In production, this could be a styled toast notification
    alert(message);
  }

  /**
   * Escapes HTML to prevent XSS
   */
  private escapeHtml(text: string): string {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }
}

/**
 * Checks if the current device is mobile
 * Uses matchMedia for reliable detection
 */
export function isMobileDevice(): boolean {
  return window.matchMedia("(max-width: 768px)").matches;
}

/**
 * Factory function to create the appropriate signature modal
 * based on device type
 */
export function createSignatureModal(
  options: MobileModalOptions
): MobileSignatureModal {
  return new MobileSignatureModal(options);
}

// Export for window access in non-module contexts
if (typeof window !== "undefined") {
  (window as unknown as Record<string, unknown>).MobileSignatureModal = MobileSignatureModal;
  (window as unknown as Record<string, unknown>).isMobileDevice = isMobileDevice;
  (window as unknown as Record<string, unknown>).createSignatureModal = createSignatureModal;
}
