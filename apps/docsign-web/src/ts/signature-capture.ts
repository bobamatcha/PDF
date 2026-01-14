/**
 * Signature Capture Component
 *
 * DOCSIGN_PLAN Phase 3: Signing UX Polish - Signature Capture Improvements
 *
 * Geriatric-friendly signature capture with:
 * - Large canvas (minimum 200px height)
 * - Thick stroke width (3-4px) for visibility
 * - Navy blue ink for classic signature look
 * - Undo/redo functionality (stroke-level)
 * - Clear "Start Over" and "Undo" buttons (60px height)
 * - Touch and mouse support with pressure detection
 * - Baseline guide for writing alignment
 * - Full accessibility (ARIA labels, keyboard accessible)
 */

/**
 * Point in a stroke with optional pressure
 */
export interface StrokePoint {
  x: number;
  y: number;
  pressure?: number;
}

/**
 * A complete stroke (pen down to pen up)
 */
export interface Stroke {
  points: StrokePoint[];
  color: string;
  width: number;
}

/**
 * Options for SignatureCapture
 */
export interface SignatureCaptureOptions {
  /** Container element to render into */
  container: HTMLElement;
  /** Canvas width in pixels (default: 100% of container) */
  width?: number;
  /** Canvas height in pixels (default: 200px for geriatric-friendly) */
  height?: number;
  /** Stroke color (default: '#000080' navy blue) */
  strokeColor?: string;
  /** Stroke width in pixels (default: 3 for desktop, 4 for touch) */
  strokeWidth?: number;
  /** Background color (default: '#ffffff') */
  backgroundColor?: string;
  /** Show baseline guide for writing alignment (default: true) */
  showGuides?: boolean;
  /** Aria label for the canvas (default: 'Signature drawing area') */
  ariaLabel?: string;
}

/**
 * SignatureCapture class - Geriatric-friendly signature drawing component
 *
 * @example
 * ```typescript
 * const capture = new SignatureCapture({
 *   container: document.getElementById('sig-container'),
 *   height: 250,
 *   showGuides: true
 * });
 *
 * capture.onchange = (isEmpty) => {
 *   submitButton.disabled = isEmpty;
 * };
 *
 * // Export signature
 * const dataUrl = capture.toDataURL('png');
 * ```
 */
export class SignatureCapture {
  // Configuration
  private container: HTMLElement;
  private width: number;
  private height: number;
  private strokeColor: string;
  private strokeWidth: number;
  private backgroundColor: string;
  private showGuides: boolean;

  // DOM elements
  private wrapper: HTMLDivElement;
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private guideCanvas: HTMLCanvasElement | null = null;
  private guideCtx: CanvasRenderingContext2D | null = null;

  // Drawing state
  private isDrawing: boolean = false;
  private currentStroke: StrokePoint[] = [];
  private strokes: Stroke[] = [];
  private redoStack: Stroke[] = [];

  // Touch detection
  private isTouchDevice: boolean = false;

  // Event handler
  public onchange: ((isEmpty: boolean) => void) | null = null;

  // Bound event handlers for cleanup
  private boundHandlers: {
    mouseDown: (e: MouseEvent) => void;
    mouseMove: (e: MouseEvent) => void;
    mouseUp: () => void;
    mouseLeave: () => void;
    touchStart: (e: TouchEvent) => void;
    touchMove: (e: TouchEvent) => void;
    touchEnd: () => void;
    resize: () => void;
  };

  constructor(options: SignatureCaptureOptions) {
    this.container = options.container;
    this.height = options.height ?? 200;
    this.strokeColor = options.strokeColor ?? "#000080"; // Navy blue
    this.backgroundColor = options.backgroundColor ?? "#ffffff";
    this.showGuides = options.showGuides ?? true;

    // Detect touch device for stroke width
    this.isTouchDevice =
      "ontouchstart" in window || navigator.maxTouchPoints > 0;
    this.strokeWidth =
      options.strokeWidth ?? (this.isTouchDevice ? 4 : 3);

    // Calculate width based on container
    const containerWidth = this.container.clientWidth || 400;
    this.width = options.width ?? containerWidth;

    // Create wrapper element
    this.wrapper = this.createWrapper();

    // Create guide canvas if needed
    if (this.showGuides) {
      this.guideCanvas = this.createGuideCanvas();
      this.guideCtx = this.guideCanvas.getContext("2d");
    }

    // Create main canvas
    this.canvas = this.createCanvas();
    const ctx = this.canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Failed to get 2D context from canvas");
    }
    this.ctx = ctx;

    // Assemble DOM
    if (this.guideCanvas) {
      this.wrapper.appendChild(this.guideCanvas);
    }
    this.wrapper.appendChild(this.canvas);
    this.container.appendChild(this.wrapper);

    // Initialize canvases
    this.initializeCanvas();
    if (this.showGuides) {
      this.drawGuides();
    }

    // Bind event handlers
    this.boundHandlers = {
      mouseDown: this.handleMouseDown.bind(this),
      mouseMove: this.handleMouseMove.bind(this),
      mouseUp: this.handleMouseUp.bind(this),
      mouseLeave: this.handleMouseLeave.bind(this),
      touchStart: this.handleTouchStart.bind(this),
      touchMove: this.handleTouchMove.bind(this),
      touchEnd: this.handleTouchEnd.bind(this),
      resize: this.handleResize.bind(this),
    };

    // Attach event listeners
    this.attachEventListeners();
  }

  /**
   * Create the wrapper element
   */
  private createWrapper(): HTMLDivElement {
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
  private createGuideCanvas(): HTMLCanvasElement {
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
  private createCanvas(): HTMLCanvasElement {
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

    // Accessibility attributes
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
  private initializeCanvas(): void {
    // Clear with background color
    this.ctx.fillStyle = this.backgroundColor;
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    // Set stroke properties
    this.ctx.strokeStyle = this.strokeColor;
    this.ctx.lineWidth = this.strokeWidth;
    this.ctx.lineCap = "round";
    this.ctx.lineJoin = "round";
  }

  /**
   * Draw baseline guides
   */
  private drawGuides(): void {
    if (!this.guideCtx || !this.guideCanvas) return;

    const ctx = this.guideCtx;
    const width = this.guideCanvas.width;
    const height = this.guideCanvas.height;

    // Clear
    ctx.clearRect(0, 0, width, height);

    // Draw baseline at 70% from top (typical signature baseline position)
    const baselineY = height * 0.7;
    ctx.strokeStyle = "#e0e0e0";
    ctx.lineWidth = 2;
    ctx.setLineDash([10, 5]);

    ctx.beginPath();
    ctx.moveTo(20, baselineY);
    ctx.lineTo(width - 20, baselineY);
    ctx.stroke();

    // Reset line dash
    ctx.setLineDash([]);

    // Draw "Sign here" hint text (subtle)
    ctx.fillStyle = "#cccccc";
    ctx.font = "italic 16px var(--font-family-body, sans-serif)";
    ctx.textAlign = "center";
    ctx.fillText("Sign on the line above", width / 2, height - 15);
  }

  /**
   * Attach event listeners
   */
  private attachEventListeners(): void {
    // Mouse events
    this.canvas.addEventListener("mousedown", this.boundHandlers.mouseDown);
    this.canvas.addEventListener("mousemove", this.boundHandlers.mouseMove);
    this.canvas.addEventListener("mouseup", this.boundHandlers.mouseUp);
    this.canvas.addEventListener("mouseleave", this.boundHandlers.mouseLeave);

    // Touch events
    this.canvas.addEventListener("touchstart", this.boundHandlers.touchStart, {
      passive: false,
    });
    this.canvas.addEventListener("touchmove", this.boundHandlers.touchMove, {
      passive: false,
    });
    this.canvas.addEventListener("touchend", this.boundHandlers.touchEnd);

    // Resize handler
    window.addEventListener("resize", this.boundHandlers.resize);
  }

  /**
   * Handle window resize
   */
  private handleResize(): void {
    // Redraw on resize if needed
    const containerWidth = this.container.clientWidth;
    if (containerWidth > 0 && containerWidth !== this.width) {
      // Store current strokes
      const savedStrokes = [...this.strokes];

      // Update dimensions
      this.width = containerWidth;
      this.canvas.width = this.width;
      if (this.guideCanvas) {
        this.guideCanvas.width = this.width;
      }

      // Reinitialize
      this.initializeCanvas();
      if (this.showGuides) {
        this.drawGuides();
      }

      // Restore strokes
      this.strokes = savedStrokes;
      this.redrawAllStrokes();
    }
  }

  /**
   * Get point from mouse event
   */
  private getMousePoint(e: MouseEvent): StrokePoint {
    const rect = this.canvas.getBoundingClientRect();
    const scaleX = this.canvas.width / rect.width;
    const scaleY = this.canvas.height / rect.height;

    return {
      x: (e.clientX - rect.left) * scaleX,
      y: (e.clientY - rect.top) * scaleY,
      pressure: 0.5, // Default pressure for mouse
    };
  }

  /**
   * Get point from touch event
   */
  private getTouchPoint(e: TouchEvent): StrokePoint {
    const rect = this.canvas.getBoundingClientRect();
    const scaleX = this.canvas.width / rect.width;
    const scaleY = this.canvas.height / rect.height;
    const touch = e.touches[0];

    // Try to get pressure from touch if available
    let pressure = 0.5;
    if ("force" in touch && typeof touch.force === "number") {
      pressure = touch.force;
    }

    return {
      x: (touch.clientX - rect.left) * scaleX,
      y: (touch.clientY - rect.top) * scaleY,
      pressure,
    };
  }

  /**
   * Start drawing
   */
  private startDrawing(point: StrokePoint): void {
    this.isDrawing = true;
    this.currentStroke = [point];

    // Clear redo stack when new stroke starts
    this.redoStack = [];

    // Begin path
    this.ctx.beginPath();
    this.ctx.moveTo(point.x, point.y);
  }

  /**
   * Continue drawing
   */
  private continueDrawing(point: StrokePoint): void {
    if (!this.isDrawing) return;

    this.currentStroke.push(point);

    // Draw line to new point
    // Adjust line width based on pressure if available
    const pressure = point.pressure ?? 0.5;
    const dynamicWidth = this.strokeWidth * (0.5 + pressure);
    this.ctx.lineWidth = dynamicWidth;

    this.ctx.lineTo(point.x, point.y);
    this.ctx.stroke();

    // Continue path from current point
    this.ctx.beginPath();
    this.ctx.moveTo(point.x, point.y);
  }

  /**
   * End drawing
   */
  private endDrawing(): void {
    if (!this.isDrawing) return;

    this.isDrawing = false;

    // Save the stroke if it has points
    if (this.currentStroke.length > 0) {
      this.strokes.push({
        points: this.currentStroke,
        color: this.strokeColor,
        width: this.strokeWidth,
      });
      this.currentStroke = [];

      // Notify change
      this.notifyChange();
    }
  }

  /**
   * Mouse event handlers
   */
  private handleMouseDown(e: MouseEvent): void {
    e.preventDefault();
    const point = this.getMousePoint(e);
    this.startDrawing(point);
  }

  private handleMouseMove(e: MouseEvent): void {
    e.preventDefault();
    const point = this.getMousePoint(e);
    this.continueDrawing(point);
  }

  private handleMouseUp(): void {
    this.endDrawing();
  }

  private handleMouseLeave(): void {
    this.endDrawing();
  }

  /**
   * Touch event handlers
   */
  private handleTouchStart(e: TouchEvent): void {
    e.preventDefault(); // Prevent scrolling
    if (e.touches.length === 1) {
      const point = this.getTouchPoint(e);
      this.startDrawing(point);
    }
  }

  private handleTouchMove(e: TouchEvent): void {
    e.preventDefault(); // Prevent scrolling
    if (e.touches.length === 1) {
      const point = this.getTouchPoint(e);
      this.continueDrawing(point);
    }
  }

  private handleTouchEnd(): void {
    this.endDrawing();
  }

  /**
   * Redraw all strokes (used after undo/clear/resize)
   */
  private redrawAllStrokes(): void {
    // Clear canvas
    this.ctx.fillStyle = this.backgroundColor;
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    // Redraw each stroke
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

    // Reset to current stroke settings
    this.ctx.strokeStyle = this.strokeColor;
    this.ctx.lineWidth = this.strokeWidth;
  }

  /**
   * Notify change listeners
   */
  private notifyChange(): void {
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
  public clear(): void {
    // Move all strokes to redo stack for potential redo
    this.redoStack = [...this.strokes];
    this.strokes = [];
    this.currentStroke = [];

    // Redraw (effectively clears)
    this.redrawAllStrokes();

    // Notify change
    this.notifyChange();

    // Announce to screen readers
    this.announceToScreenReader("Signature cleared");
  }

  /**
   * Undo the last stroke
   */
  public undo(): void {
    if (this.strokes.length === 0) return;

    // Move last stroke to redo stack
    const lastStroke = this.strokes.pop();
    if (lastStroke) {
      this.redoStack.push(lastStroke);
    }

    // Redraw
    this.redrawAllStrokes();

    // Notify change
    this.notifyChange();

    // Announce to screen readers
    this.announceToScreenReader("Stroke undone");
  }

  /**
   * Redo the last undone stroke
   */
  public redo(): void {
    if (this.redoStack.length === 0) return;

    // Move last undone stroke back to strokes
    const stroke = this.redoStack.pop();
    if (stroke) {
      this.strokes.push(stroke);
    }

    // Redraw
    this.redrawAllStrokes();

    // Notify change
    this.notifyChange();

    // Announce to screen readers
    this.announceToScreenReader("Stroke restored");
  }

  /**
   * Check if the signature pad is empty
   */
  public isEmpty(): boolean {
    return this.strokes.length === 0;
  }

  /**
   * Check if undo is available
   */
  public canUndo(): boolean {
    return this.strokes.length > 0;
  }

  /**
   * Check if redo is available
   */
  public canRedo(): boolean {
    return this.redoStack.length > 0;
  }

  /**
   * Get stroke count
   */
  public getStrokeCount(): number {
    return this.strokes.length;
  }

  /**
   * Export signature as data URL
   * @param format - 'png' or 'svg'
   */
  public toDataURL(format: "png" | "svg" = "png"): string {
    if (format === "svg") {
      return this.toSVG();
    }
    return this.canvas.toDataURL("image/png");
  }

  /**
   * Export signature as SVG string
   */
  private toSVG(): string {
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
  public toBlob(): Promise<Blob> {
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
        1.0
      );
    });
  }

  /**
   * Get the raw strokes data
   */
  public getStrokes(): Stroke[] {
    return [...this.strokes];
  }

  /**
   * Load strokes from data
   */
  public loadStrokes(strokes: Stroke[]): void {
    this.strokes = strokes.map((s) => ({
      points: [...s.points],
      color: s.color,
      width: s.width,
    }));
    this.redoStack = [];
    this.redrawAllStrokes();
    this.notifyChange();
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

    // Remove after announcement
    setTimeout(() => {
      announcement.remove();
    }, 1000);
  }

  /**
   * Destroy the component and clean up
   */
  public destroy(): void {
    // Remove event listeners
    this.canvas.removeEventListener("mousedown", this.boundHandlers.mouseDown);
    this.canvas.removeEventListener("mousemove", this.boundHandlers.mouseMove);
    this.canvas.removeEventListener("mouseup", this.boundHandlers.mouseUp);
    this.canvas.removeEventListener("mouseleave", this.boundHandlers.mouseLeave);
    this.canvas.removeEventListener("touchstart", this.boundHandlers.touchStart);
    this.canvas.removeEventListener("touchmove", this.boundHandlers.touchMove);
    this.canvas.removeEventListener("touchend", this.boundHandlers.touchEnd);
    window.removeEventListener("resize", this.boundHandlers.resize);

    // Remove from DOM
    this.wrapper.remove();

    // Clear references
    this.strokes = [];
    this.redoStack = [];
    this.onchange = null;
  }

  /**
   * Get the canvas element (for advanced use cases)
   */
  public getCanvas(): HTMLCanvasElement {
    return this.canvas;
  }

  /**
   * Get the wrapper element
   */
  public getWrapper(): HTMLDivElement {
    return this.wrapper;
  }
}

// Default export
export default SignatureCapture;
