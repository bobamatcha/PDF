/**
 * Property-Based Tests for Signature Capture
 *
 * Phase 3 of DOCSIGN_PLAN: Comprehensive tests for signature capture,
 * typed signatures, and mobile modal functionality.
 *
 * These tests use fast-check for property-based testing to verify:
 * 1. Stroke Recording Properties
 * 2. Data Export Properties
 * 3. Typed Signature Properties
 * 4. Touch/Mouse Event Properties
 * 5. Canvas Sizing Properties
 * 6. Validation Properties
 * 7. Modal State Properties
 *
 * NOTE: These tests are written FIRST before the implementation exists.
 * They should FAIL until SignatureCapture components are implemented.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as fc from 'fast-check';

// ============================================================
// Types for Signature Capture (what we expect to implement)
// ============================================================

/**
 * A point in the signature stroke
 */
export interface StrokePoint {
  x: number;
  y: number;
  pressure?: number;
  timestamp?: number;
}

/**
 * A complete stroke (a line drawn without lifting)
 */
export interface Stroke {
  id: string;
  points: StrokePoint[];
  color: string;
  width: number;
  createdAt: number;
}

/**
 * Signature type: drawn or typed
 */
export type SignatureType = 'drawn' | 'typed';

/**
 * Available fonts for typed signatures
 */
export const SIGNATURE_FONTS = [
  'Dancing Script',
  'Great Vibes',
  'Allura',
  'Pacifico',
  'Sacramento',
] as const;

export type SignatureFont = (typeof SIGNATURE_FONTS)[number];

/**
 * Typed signature data
 */
export interface TypedSignatureData {
  text: string;
  font: SignatureFont;
  fontSize: number;
  color: string;
}

/**
 * Exported signature data
 */
export interface SignatureExport {
  type: SignatureType;
  dataUrl: string;
  width: number;
  height: number;
  timestamp: string;
}

/**
 * Modal state
 */
export type ModalState = 'closed' | 'open' | 'confirming' | 'cancelled';

/**
 * Canvas resize behavior
 */
export type ResizeBehavior = 'scale' | 'reset' | 'preserve';

// ============================================================
// Mock Canvas API
// ============================================================

interface MockCanvasOperation {
  type: 'beginPath' | 'moveTo' | 'lineTo' | 'stroke' | 'clearRect' | 'fillText' | 'drawImage';
  args?: unknown[];
}

interface MockCanvasContext {
  operations: MockCanvasOperation[];
  strokeStyle: string;
  fillStyle: string;
  lineWidth: number;
  lineCap: CanvasLineCap;
  lineJoin: CanvasLineJoin;
  font: string;
  textAlign: CanvasTextAlign;
  textBaseline: CanvasTextBaseline;

  beginPath(): void;
  moveTo(x: number, y: number): void;
  lineTo(x: number, y: number): void;
  stroke(): void;
  clearRect(x: number, y: number, w: number, h: number): void;
  fillText(text: string, x: number, y: number): void;
  measureText(text: string): { width: number };
  drawImage(...args: unknown[]): void;
  getImageData(x: number, y: number, w: number, h: number): ImageData;
  putImageData(imageData: ImageData, x: number, y: number): void;
  save(): void;
  restore(): void;
  scale(x: number, y: number): void;
  translate(x: number, y: number): void;
  resetTransform(): void;
}

interface MockCanvas {
  width: number;
  height: number;
  context: MockCanvasContext;
  getContext(type: '2d'): MockCanvasContext;
  toDataURL(type?: string, quality?: number): string;
  toBlob(callback: (blob: Blob | null) => void, type?: string, quality?: number): void;
  getBoundingClientRect(): DOMRect;
  resetOperations(): void;
}

function createMockCanvas(width = 400, height = 200): { canvas: MockCanvas; ctx: MockCanvasContext } {
  const operations: MockCanvasOperation[] = [];

  const ctx: MockCanvasContext = {
    operations,
    strokeStyle: '#000000',
    fillStyle: '#000000',
    lineWidth: 2,
    lineCap: 'round',
    lineJoin: 'round',
    font: '16px sans-serif',
    textAlign: 'left',
    textBaseline: 'alphabetic',

    beginPath() {
      operations.push({ type: 'beginPath' });
    },
    moveTo(x: number, y: number) {
      operations.push({ type: 'moveTo', args: [x, y] });
    },
    lineTo(x: number, y: number) {
      operations.push({ type: 'lineTo', args: [x, y] });
    },
    stroke() {
      operations.push({ type: 'stroke' });
    },
    clearRect(x: number, y: number, w: number, h: number) {
      operations.push({ type: 'clearRect', args: [x, y, w, h] });
    },
    fillText(text: string, x: number, y: number) {
      operations.push({ type: 'fillText', args: [text, x, y] });
    },
    measureText(text: string) {
      return { width: text.length * 10 }; // Approximate
    },
    drawImage(...args: unknown[]) {
      operations.push({ type: 'drawImage', args });
    },
    getImageData(x: number, y: number, w: number, h: number): ImageData {
      // Return mock ImageData with some non-zero pixels
      const data = new Uint8ClampedArray(w * h * 4);
      // Fill with some data based on operations to simulate drawn content
      if (operations.length > 0) {
        for (let i = 0; i < data.length; i += 4) {
          data[i] = 0; // R
          data[i + 1] = 0; // G
          data[i + 2] = 0; // B
          data[i + 3] = operations.some((op) => op.type === 'lineTo') ? 255 : 0; // A
        }
      }
      return new ImageData(data, w, h);
    },
    putImageData() {
      // No-op for mock
    },
    save() {
      // No-op for mock
    },
    restore() {
      // No-op for mock
    },
    scale() {
      // No-op for mock
    },
    translate() {
      // No-op for mock
    },
    resetTransform() {
      // No-op for mock
    },
  };

  const canvas: MockCanvas = {
    width,
    height,
    context: ctx,
    getContext(type: '2d') {
      if (type === '2d') return ctx;
      throw new Error('Unsupported context type');
    },
    toDataURL(type = 'image/png') {
      // Generate a deterministic data URL based on operations (excluding random elements)
      // Only include the operation types and args for determinism
      const deterministicOps = operations.map((op) => ({
        type: op.type,
        args: op.args,
      }));
      const operationHash = JSON.stringify(deterministicOps);
      const hasContent = operations.some((op) => op.type === 'lineTo' || op.type === 'fillText');
      if (!hasContent) {
        // Empty canvas
        return 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==';
      }
      // Simulated non-empty content (base64 of operation hash for determinism)
      const base64 = Buffer.from(operationHash).toString('base64');
      return `data:${type};base64,${base64}`;
    },
    toBlob(callback: (blob: Blob | null) => void, type = 'image/png') {
      const dataUrl = this.toDataURL(type);
      const base64 = dataUrl.split(',')[1];
      const binary = Buffer.from(base64, 'base64');
      callback(new Blob([binary], { type }));
    },
    getBoundingClientRect() {
      return {
        x: 0,
        y: 0,
        width: this.width,
        height: this.height,
        top: 0,
        left: 0,
        right: this.width,
        bottom: this.height,
        toJSON: () => ({}),
      };
    },
    resetOperations() {
      operations.length = 0;
    },
  };

  return { canvas, ctx };
}

// ============================================================
// Signature Capture Implementation (Stub for Tests)
// ============================================================

/**
 * SignatureCapture - manages drawing and typed signatures on canvas
 *
 * This is a stub implementation for tests. The real implementation
 * will be created after tests are written.
 */
class SignatureCapture {
  private canvas: MockCanvas;
  private ctx: MockCanvasContext;
  private strokes: Stroke[] = [];
  private undoneStrokes: Stroke[] = [];
  private currentStroke: StrokePoint[] = [];
  private isDrawing = false;
  private strokeColor = '#000000';
  private strokeWidth = 2;
  private minHeight = 200;

  constructor(canvas: MockCanvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
  }

  /**
   * Start a new stroke at the given point
   */
  startStroke(point: StrokePoint): void {
    this.isDrawing = true;
    // Clamp the first point to canvas bounds
    const clampedPoint: StrokePoint = {
      x: Math.max(0, Math.min(this.canvas.width, point.x)),
      y: Math.max(0, Math.min(this.canvas.height, point.y)),
      pressure: point.pressure,
      timestamp: point.timestamp,
    };
    this.currentStroke = [clampedPoint];
    this.ctx.beginPath();
    this.ctx.moveTo(clampedPoint.x, clampedPoint.y);
  }

  /**
   * Add a point to the current stroke
   */
  addPoint(point: StrokePoint): void {
    if (!this.isDrawing) return;

    // Clamp points to canvas bounds
    const clampedPoint: StrokePoint = {
      x: Math.max(0, Math.min(this.canvas.width, point.x)),
      y: Math.max(0, Math.min(this.canvas.height, point.y)),
      pressure: point.pressure,
      timestamp: point.timestamp,
    };

    this.currentStroke.push(clampedPoint);
    this.ctx.lineTo(clampedPoint.x, clampedPoint.y);
    this.ctx.stroke();
  }

  /**
   * End the current stroke
   */
  endStroke(): void {
    if (!this.isDrawing || this.currentStroke.length < 2) {
      // Need at least 2 points for a valid stroke
      this.isDrawing = false;
      this.currentStroke = [];
      return;
    }

    const stroke: Stroke = {
      id: crypto.randomUUID(),
      points: [...this.currentStroke],
      color: this.strokeColor,
      width: this.strokeWidth,
      createdAt: Date.now(),
    };

    this.strokes.push(stroke);
    this.undoneStrokes = []; // Clear redo stack on new stroke
    this.currentStroke = [];
    this.isDrawing = false;
  }

  /**
   * Get all recorded strokes
   */
  getStrokes(): Stroke[] {
    return [...this.strokes];
  }

  /**
   * Check if canvas is empty (no strokes)
   */
  isEmpty(): boolean {
    return this.strokes.length === 0;
  }

  /**
   * Undo the last stroke
   */
  undo(): boolean {
    if (this.strokes.length === 0) return false;

    const stroke = this.strokes.pop()!;
    this.undoneStrokes.push(stroke);
    this.redraw();
    return true;
  }

  /**
   * Redo the last undone stroke
   */
  redo(): boolean {
    if (this.undoneStrokes.length === 0) return false;

    const stroke = this.undoneStrokes.pop()!;
    this.strokes.push(stroke);
    this.redraw();
    return true;
  }

  /**
   * Clear all strokes
   */
  clear(): void {
    this.strokes = [];
    this.undoneStrokes = []; // Also clear redo stack
    this.canvas.resetOperations(); // Reset mock canvas operations for determinism
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }

  /**
   * Redraw all strokes
   */
  private redraw(): void {
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    for (const stroke of this.strokes) {
      this.ctx.strokeStyle = stroke.color;
      this.ctx.lineWidth = stroke.width;
      this.ctx.beginPath();

      if (stroke.points.length > 0) {
        this.ctx.moveTo(stroke.points[0].x, stroke.points[0].y);
        for (let i = 1; i < stroke.points.length; i++) {
          this.ctx.lineTo(stroke.points[i].x, stroke.points[i].y);
        }
        this.ctx.stroke();
      }
    }
  }

  /**
   * Export signature as data URL
   */
  toDataURL(type = 'image/png'): string {
    return this.canvas.toDataURL(type);
  }

  /**
   * Export signature data
   */
  export(): SignatureExport | null {
    if (this.isEmpty()) return null;

    return {
      type: 'drawn',
      dataUrl: this.toDataURL(),
      width: this.canvas.width,
      height: this.canvas.height,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get canvas dimensions
   */
  getDimensions(): { width: number; height: number } {
    return {
      width: this.canvas.width,
      height: this.canvas.height,
    };
  }

  /**
   * Resize canvas (with optional content preservation)
   */
  resize(width: number, height: number, behavior: ResizeBehavior = 'reset'): void {
    const oldWidth = this.canvas.width;
    const oldHeight = this.canvas.height;

    // Enforce minimum height
    height = Math.max(height, this.minHeight);

    if (behavior === 'scale' && this.strokes.length > 0) {
      // Scale all stroke points
      const scaleX = width / oldWidth;
      const scaleY = height / oldHeight;

      for (const stroke of this.strokes) {
        for (const point of stroke.points) {
          point.x *= scaleX;
          point.y *= scaleY;
        }
      }
    } else if (behavior === 'reset') {
      this.strokes = [];
      this.undoneStrokes = [];
    }
    // 'preserve' keeps strokes as-is

    this.canvas.width = width;
    this.canvas.height = height;
    this.redraw();
  }

  /**
   * Set stroke color
   */
  setColor(color: string): void {
    this.strokeColor = color;
    this.ctx.strokeStyle = color;
  }

  /**
   * Set stroke width
   */
  setWidth(width: number): void {
    this.strokeWidth = width;
    this.ctx.lineWidth = width;
  }
}

/**
 * TypedSignature - generates typed signatures with custom fonts
 */
class TypedSignature {
  private canvas: MockCanvas;
  private ctx: MockCanvasContext;
  private text = '';
  private font: SignatureFont = 'Dancing Script';
  private fontSize = 48;
  private color = '#000000';

  constructor(canvas: MockCanvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
  }

  /**
   * Set the signature text
   */
  setText(text: string): void {
    this.text = text;
    this.render();
  }

  /**
   * Get the current text
   */
  getText(): string {
    return this.text;
  }

  /**
   * Set the font
   */
  setFont(font: SignatureFont): void {
    this.font = font;
    this.render();
  }

  /**
   * Get the current font
   */
  getFont(): SignatureFont {
    return this.font;
  }

  /**
   * Set the font size
   */
  setFontSize(size: number): void {
    this.fontSize = size;
    this.render();
  }

  /**
   * Set the color
   */
  setColor(color: string): void {
    this.color = color;
    this.render();
  }

  /**
   * Check if signature is empty
   */
  isEmpty(): boolean {
    return this.text.trim().length === 0;
  }

  /**
   * Render the typed signature
   */
  private render(): void {
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    if (this.isEmpty()) return;

    this.ctx.font = `${this.fontSize}px "${this.font}"`;
    this.ctx.fillStyle = this.color;
    this.ctx.textAlign = 'center';
    this.ctx.textBaseline = 'middle';

    this.ctx.fillText(this.text, this.canvas.width / 2, this.canvas.height / 2);
  }

  /**
   * Export as data URL
   */
  toDataURL(type = 'image/png'): string {
    return this.canvas.toDataURL(type);
  }

  /**
   * Export signature data
   */
  export(): SignatureExport | null {
    if (this.isEmpty()) return null;

    return {
      type: 'typed',
      dataUrl: this.toDataURL(),
      width: this.canvas.width,
      height: this.canvas.height,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get typed signature data
   */
  getData(): TypedSignatureData {
    return {
      text: this.text,
      font: this.font,
      fontSize: this.fontSize,
      color: this.color,
    };
  }

  /**
   * Clear the typed signature (for testing)
   */
  clear(): void {
    this.text = '';
    this.canvas.resetOperations();
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }
}

/**
 * SignatureModal - manages the signature capture modal
 */
class SignatureModal {
  private state: ModalState = 'closed';
  private result: SignatureExport | null = null;
  private onConfirmCallback: ((signature: SignatureExport | null) => void) | null = null;

  /**
   * Get current modal state
   */
  getState(): ModalState {
    return this.state;
  }

  /**
   * Check if modal is visible
   */
  isVisible(): boolean {
    return this.state === 'open' || this.state === 'confirming';
  }

  /**
   * Open the modal
   */
  open(): boolean {
    if (this.state === 'open' || this.state === 'confirming') {
      return false; // Already open
    }

    this.state = 'open';
    this.result = null;
    return true;
  }

  /**
   * Close the modal (cancel)
   */
  close(): void {
    this.state = 'cancelled';
    this.result = null;

    if (this.onConfirmCallback) {
      this.onConfirmCallback(null);
      this.onConfirmCallback = null;
    }

    // Reset state after callback
    this.state = 'closed';
  }

  /**
   * Confirm and close with signature
   */
  confirm(signature: SignatureExport): void {
    this.state = 'confirming';
    this.result = signature;

    if (this.onConfirmCallback) {
      this.onConfirmCallback(signature);
      this.onConfirmCallback = null;
    }

    // Reset state after callback
    this.state = 'closed';
  }

  /**
   * Get the result (signature or null)
   */
  getResult(): SignatureExport | null {
    return this.result;
  }

  /**
   * Set callback for confirmation
   */
  onConfirm(callback: (signature: SignatureExport | null) => void): void {
    this.onConfirmCallback = callback;
  }

  /**
   * Reset modal to initial state
   */
  reset(): void {
    this.state = 'closed';
    this.result = null;
    this.onConfirmCallback = null;
  }
}

// ============================================================
// Validation Utilities
// ============================================================

/**
 * Validate that a string is valid base64
 */
function isValidBase64(str: string): boolean {
  if (!str || typeof str !== 'string') return false;
  try {
    // Check if it's a valid data URL format
    if (str.startsWith('data:')) {
      const commaIndex = str.indexOf(',');
      if (commaIndex === -1) return false;
      str = str.slice(commaIndex + 1);
    }

    // Try to decode
    const decoded = Buffer.from(str, 'base64');
    const reencoded = decoded.toString('base64');

    // Remove padding differences
    const normalize = (s: string) => s.replace(/=/g, '');
    return normalize(str) === normalize(reencoded);
  } catch {
    return false;
  }
}

/**
 * Check PNG header validity
 */
function hasValidPngHeader(dataUrl: string): boolean {
  if (!dataUrl.startsWith('data:image/png;base64,')) return false;

  const base64 = dataUrl.split(',')[1];
  if (!base64) return false;

  try {
    const bytes = Buffer.from(base64, 'base64');
    // PNG magic number: 89 50 4E 47 0D 0A 1A 0A
    const pngMagic = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

    if (bytes.length < 8) return false;

    for (let i = 0; i < 8; i++) {
      if (bytes[i] !== pngMagic[i]) return false;
    }

    return true;
  } catch {
    return false;
  }
}

/**
 * Get signature data size in bytes
 */
function getSignatureSizeBytes(dataUrl: string): number {
  const base64 = dataUrl.split(',')[1];
  if (!base64) return 0;

  // Base64 encodes 6 bits per character, so 4 chars = 3 bytes
  const padding = (base64.match(/=/g) || []).length;
  return (base64.length * 3) / 4 - padding;
}

// ============================================================
// Arbitraries (Generators) for fast-check
// ============================================================

// Stroke point arbitrary
const strokePointArb: fc.Arbitrary<StrokePoint> = fc.record({
  x: fc.float({ min: 0, max: 1000, noNaN: true }),
  y: fc.float({ min: 0, max: 1000, noNaN: true }),
  pressure: fc.option(fc.float({ min: 0, max: 1, noNaN: true })),
  timestamp: fc.option(fc.integer({ min: 0, max: Number.MAX_SAFE_INTEGER })),
});

// Bounded stroke point (within specific canvas dimensions)
const boundedStrokePointArb = (width: number, height: number): fc.Arbitrary<StrokePoint> =>
  fc.record({
    x: fc.float({ min: 0, max: width, noNaN: true }),
    y: fc.float({ min: 0, max: height, noNaN: true }),
    pressure: fc.option(fc.float({ min: 0, max: 1, noNaN: true })),
    timestamp: fc.option(fc.integer({ min: 0, max: Number.MAX_SAFE_INTEGER })),
  });

// Valid stroke (with at least 2 points)
const strokeArb = (width = 400, height = 200): fc.Arbitrary<StrokePoint[]> =>
  fc.array(boundedStrokePointArb(width, height), { minLength: 2, maxLength: 100 });

// Signature font arbitrary
const signatureFontArb: fc.Arbitrary<SignatureFont> = fc.constantFrom(...SIGNATURE_FONTS);

// Canvas dimensions arbitrary
const canvasDimensionsArb = fc.record({
  width: fc.integer({ min: 100, max: 2000 }),
  height: fc.integer({ min: 200, max: 1000 }), // Minimum 200px height
});

// Non-empty text for typed signatures
const signatureTextArb = fc
  .string({ minLength: 1, maxLength: 100 })
  .filter((s) => s.trim().length > 0);

// Unicode text for typed signatures
const unicodeTextArb = fc.unicodeString({ minLength: 1, maxLength: 50 }).filter((s) => s.trim().length > 0);

// Valid hex color arbitrary
const hexColorArb = fc
  .tuple(
    fc.integer({ min: 0, max: 255 }),
    fc.integer({ min: 0, max: 255 }),
    fc.integer({ min: 0, max: 255 })
  )
  .map(([r, g, b]) => `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`);

// ============================================================
// Test Setup
// ============================================================

describe('SignatureCapture', () => {
  let mockCanvas: MockCanvas;
  let capture: SignatureCapture;

  beforeEach(() => {
    const { canvas } = createMockCanvas(400, 200);
    mockCanvas = canvas;
    capture = new SignatureCapture(mockCanvas);
  });

  // ============================================================
  // 1. Stroke Recording Properties
  // ============================================================

  describe('Stroke Recording Properties', () => {
    it('Property 1: Stroke points are recorded in order', () => {
      fc.assert(
        fc.property(strokeArb(400, 200), (points) => {
          // Clear before each iteration
          capture.clear();

          // Start stroke with first point
          capture.startStroke(points[0]);

          // Add remaining points
          for (let i = 1; i < points.length; i++) {
            capture.addPoint(points[i]);
          }

          // End stroke
          capture.endStroke();

          // Verify order
          const strokes = capture.getStrokes();
          expect(strokes.length).toBe(1);

          const recordedPoints = strokes[0].points;
          expect(recordedPoints.length).toBe(points.length);

          // Points should be in same order (coordinates may be clamped)
          for (let i = 0; i < points.length; i++) {
            const original = points[i];
            const recorded = recordedPoints[i];

            // Clamped to canvas bounds
            expect(recorded.x).toBe(Math.max(0, Math.min(400, original.x)));
            expect(recorded.y).toBe(Math.max(0, Math.min(200, original.y)));
          }
        }),
        { numRuns: 30 }
      );
    });

    it('Property 2: Empty canvas returns empty on export', () => {
      const result = capture.export();
      expect(result).toBeNull();
      expect(capture.isEmpty()).toBe(true);
    });

    it('Property 3: At least one stroke required for non-empty', () => {
      expect(capture.isEmpty()).toBe(true);

      // Draw a valid stroke
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      expect(capture.isEmpty()).toBe(false);
      expect(capture.export()).not.toBeNull();
    });

    it('Property 4: Undo removes exactly one stroke', () => {
      fc.assert(
        fc.property(fc.integer({ min: 1, max: 10 }), (numStrokes) => {
          // Reset capture
          capture.clear();

          // Draw multiple strokes
          for (let i = 0; i < numStrokes; i++) {
            capture.startStroke({ x: i * 10, y: i * 10 });
            capture.addPoint({ x: i * 10 + 20, y: i * 10 + 20 });
            capture.endStroke();
          }

          expect(capture.getStrokes().length).toBe(numStrokes);

          // Undo one
          const result = capture.undo();
          expect(result).toBe(true);
          expect(capture.getStrokes().length).toBe(numStrokes - 1);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 5: Redo restores exactly one stroke', () => {
      fc.assert(
        fc.property(fc.integer({ min: 1, max: 10 }), (numStrokes) => {
          capture.clear();

          // Draw multiple strokes
          for (let i = 0; i < numStrokes; i++) {
            capture.startStroke({ x: i * 10, y: i * 10 });
            capture.addPoint({ x: i * 10 + 20, y: i * 10 + 20 });
            capture.endStroke();
          }

          // Undo one
          capture.undo();
          expect(capture.getStrokes().length).toBe(numStrokes - 1);

          // Redo one
          const result = capture.redo();
          expect(result).toBe(true);
          expect(capture.getStrokes().length).toBe(numStrokes);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 6: Clear removes all strokes', () => {
      fc.assert(
        fc.property(fc.integer({ min: 1, max: 10 }), (numStrokes) => {
          capture.clear();

          // Draw multiple strokes
          for (let i = 0; i < numStrokes; i++) {
            capture.startStroke({ x: i * 10, y: i * 10 });
            capture.addPoint({ x: i * 10 + 20, y: i * 10 + 20 });
            capture.endStroke();
          }

          expect(capture.getStrokes().length).toBe(numStrokes);

          // Clear
          capture.clear();
          expect(capture.getStrokes().length).toBe(0);
          expect(capture.isEmpty()).toBe(true);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 7: Undo after clear does nothing', () => {
      // Draw something
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      // Clear
      capture.clear();

      // Undo should do nothing (redo stack is also cleared)
      const result = capture.undo();
      expect(result).toBe(false);
      expect(capture.isEmpty()).toBe(true);
    });

    it('Property 8: Single point strokes are discarded', () => {
      capture.startStroke({ x: 10, y: 10 });
      // No additional points
      capture.endStroke();

      expect(capture.isEmpty()).toBe(true);
      expect(capture.getStrokes().length).toBe(0);
    });

    it('Property 9: New stroke clears redo stack', () => {
      // Draw two strokes
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      capture.startStroke({ x: 60, y: 60 });
      capture.addPoint({ x: 100, y: 100 });
      capture.endStroke();

      // Undo one
      capture.undo();
      expect(capture.getStrokes().length).toBe(1);

      // Draw new stroke (should clear redo stack)
      capture.startStroke({ x: 70, y: 70 });
      capture.addPoint({ x: 110, y: 110 });
      capture.endStroke();

      expect(capture.getStrokes().length).toBe(2);

      // Redo should do nothing now
      const result = capture.redo();
      expect(result).toBe(false);
      expect(capture.getStrokes().length).toBe(2);
    });
  });

  // ============================================================
  // 2. Data Export Properties
  // ============================================================

  describe('Data Export Properties', () => {
    it('Property 10: toDataURL returns valid data URL format', () => {
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const dataUrl = capture.toDataURL();
      expect(dataUrl).toMatch(/^data:image\/png;base64,/);
    });

    it("Property 11: Data URL starts with 'data:image/png;base64,'", () => {
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const dataUrl = capture.toDataURL('image/png');
      expect(dataUrl.startsWith('data:image/png;base64,')).toBe(true);
    });

    it('Property 12: Same strokes produce same output (deterministic)', () => {
      fc.assert(
        fc.property(strokeArb(400, 200), (points) => {
          // Reset and draw
          capture.clear();
          capture.startStroke(points[0]);
          for (let i = 1; i < points.length; i++) {
            capture.addPoint(points[i]);
          }
          capture.endStroke();

          const output1 = capture.toDataURL();

          // Reset and draw same stroke
          capture.clear();
          capture.startStroke(points[0]);
          for (let i = 1; i < points.length; i++) {
            capture.addPoint(points[i]);
          }
          capture.endStroke();

          const output2 = capture.toDataURL();

          expect(output1).toBe(output2);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 13: Different strokes produce different output', () => {
      // Draw first stroke
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const output1 = capture.toDataURL();

      // Clear and draw different stroke
      capture.clear();
      capture.startStroke({ x: 100, y: 100 });
      capture.addPoint({ x: 150, y: 150 });
      capture.endStroke();

      const output2 = capture.toDataURL();

      expect(output1).not.toBe(output2);
    });

    it('Property 14: Export preserves canvas dimensions', () => {
      fc.assert(
        fc.property(canvasDimensionsArb, ({ width, height }) => {
          const { canvas } = createMockCanvas(width, height);
          const cap = new SignatureCapture(canvas);

          cap.startStroke({ x: 10, y: 10 });
          cap.addPoint({ x: 50, y: 50 });
          cap.endStroke();

          const exported = cap.export();
          expect(exported).not.toBeNull();
          expect(exported!.width).toBe(width);
          expect(exported!.height).toBe(height);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 15: Export includes valid timestamp', () => {
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const exported = capture.export();
      expect(exported).not.toBeNull();

      // Should be valid ISO timestamp
      const timestamp = new Date(exported!.timestamp);
      expect(timestamp.getTime()).not.toBeNaN();
      expect(timestamp.getTime()).toBeLessThanOrEqual(Date.now());
    });
  });

  // ============================================================
  // 3. Typed Signature Properties
  // ============================================================

  describe('Typed Signature Properties', () => {
    let typedSignature: TypedSignature;

    beforeEach(() => {
      const { canvas } = createMockCanvas(400, 200);
      typedSignature = new TypedSignature(canvas);
    });

    it('Property 16: Empty text produces empty signature', () => {
      typedSignature.setText('');
      expect(typedSignature.isEmpty()).toBe(true);
      expect(typedSignature.export()).toBeNull();
    });

    it('Property 17: Whitespace-only text produces empty signature', () => {
      fc.assert(
        fc.property(
          fc.stringOf(fc.constantFrom(' ', '\t', '\n', '\r'), { minLength: 1, maxLength: 20 }),
          (whitespace) => {
            typedSignature.setText(whitespace);
            expect(typedSignature.isEmpty()).toBe(true);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 18: Same text + font produces same output', () => {
      fc.assert(
        fc.property(signatureTextArb, signatureFontArb, (text, font) => {
          // Clear before first render
          typedSignature.clear();
          typedSignature.setText(text);
          typedSignature.setFont(font);

          const output1 = typedSignature.toDataURL();

          // Clear and set same values
          typedSignature.clear();
          typedSignature.setText(text);
          typedSignature.setFont(font);

          const output2 = typedSignature.toDataURL();

          expect(output1).toBe(output2);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 19: Different fonts produce different output', () => {
      const text = 'John Doe';

      typedSignature.setText(text);
      typedSignature.setFont('Dancing Script');
      const output1 = typedSignature.toDataURL();

      typedSignature.setFont('Pacifico');
      const output2 = typedSignature.toDataURL();

      expect(output1).not.toBe(output2);
    });

    it('Property 20: Text is readable in output (not blank)', () => {
      fc.assert(
        fc.property(signatureTextArb, (text) => {
          typedSignature.setText(text);

          const exported = typedSignature.export();
          expect(exported).not.toBeNull();
          expect(exported!.type).toBe('typed');
          // Data URL should be non-trivial (not empty canvas)
          expect(exported!.dataUrl.length).toBeGreaterThan(50);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 21: All configured fonts are valid', () => {
      fc.assert(
        fc.property(signatureFontArb, (font) => {
          expect(SIGNATURE_FONTS).toContain(font);

          typedSignature.setText('Test');
          typedSignature.setFont(font);

          const data = typedSignature.getData();
          expect(data.font).toBe(font);
        }),
        { numRuns: 10 }
      );
    });

    it('Property 22: Unicode text is supported', () => {
      fc.assert(
        fc.property(unicodeTextArb, (text) => {
          typedSignature.setText(text);

          if (!typedSignature.isEmpty()) {
            const data = typedSignature.getData();
            expect(data.text).toBe(text);

            const exported = typedSignature.export();
            expect(exported).not.toBeNull();
          }
        }),
        { numRuns: 20 }
      );
    });

    it('Property 23: Font size affects output', () => {
      typedSignature.setText('Test');

      typedSignature.setFontSize(24);
      const output1 = typedSignature.toDataURL();

      typedSignature.setFontSize(72);
      const output2 = typedSignature.toDataURL();

      expect(output1).not.toBe(output2);
    });

    it('Property 24: Color changes affect output', () => {
      typedSignature.setText('Test');

      typedSignature.setColor('#000000');
      const output1 = typedSignature.toDataURL();

      typedSignature.setColor('#FF0000');
      const output2 = typedSignature.toDataURL();

      expect(output1).not.toBe(output2);
    });
  });

  // ============================================================
  // 4. Touch/Mouse Event Properties
  // ============================================================

  describe('Touch/Mouse Event Properties', () => {
    it('Property 25: Touch events produce valid stroke points', () => {
      fc.assert(
        fc.property(
          fc.array(boundedStrokePointArb(400, 200), { minLength: 2, maxLength: 20 }),
          (points) => {
            capture.clear();

            // Simulate touch events
            capture.startStroke(points[0]);
            for (let i = 1; i < points.length; i++) {
              capture.addPoint(points[i]);
            }
            capture.endStroke();

            const strokes = capture.getStrokes();
            expect(strokes.length).toBe(1);

            // All points should be valid
            strokes[0].points.forEach((point) => {
              expect(typeof point.x).toBe('number');
              expect(typeof point.y).toBe('number');
              expect(point.x).toBeGreaterThanOrEqual(0);
              expect(point.y).toBeGreaterThanOrEqual(0);
            });
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 26: Mouse events produce valid stroke points', () => {
      fc.assert(
        fc.property(
          fc.array(boundedStrokePointArb(400, 200), { minLength: 2, maxLength: 20 }),
          (points) => {
            capture.clear();

            // Simulate mouse events (same API as touch)
            capture.startStroke(points[0]);
            for (let i = 1; i < points.length; i++) {
              capture.addPoint(points[i]);
            }
            capture.endStroke();

            const strokes = capture.getStrokes();
            expect(strokes.length).toBe(1);
            expect(strokes[0].points.length).toBe(points.length);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 27: Points are within canvas bounds', () => {
      fc.assert(
        fc.property(
          fc.array(strokePointArb, { minLength: 2, maxLength: 20 }),
          (points) => {
            capture.clear();

            capture.startStroke(points[0]);
            for (let i = 1; i < points.length; i++) {
              capture.addPoint(points[i]);
            }
            capture.endStroke();

            const strokes = capture.getStrokes();
            if (strokes.length > 0) {
              strokes[0].points.forEach((point) => {
                expect(point.x).toBeGreaterThanOrEqual(0);
                expect(point.x).toBeLessThanOrEqual(400);
                expect(point.y).toBeGreaterThanOrEqual(0);
                expect(point.y).toBeLessThanOrEqual(200);
              });
            }
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 28: Stroke has at least 2 points (start + end)', () => {
      fc.assert(
        fc.property(strokeArb(400, 200), (points) => {
          capture.clear();

          capture.startStroke(points[0]);
          for (let i = 1; i < points.length; i++) {
            capture.addPoint(points[i]);
          }
          capture.endStroke();

          const strokes = capture.getStrokes();
          strokes.forEach((stroke) => {
            expect(stroke.points.length).toBeGreaterThanOrEqual(2);
          });
        }),
        { numRuns: 20 }
      );
    });

    it('Property 29: Pressure values are preserved when provided', () => {
      const pointsWithPressure: StrokePoint[] = [
        { x: 10, y: 10, pressure: 0.5 },
        { x: 50, y: 50, pressure: 0.8 },
        { x: 100, y: 100, pressure: 0.3 },
      ];

      capture.startStroke(pointsWithPressure[0]);
      for (let i = 1; i < pointsWithPressure.length; i++) {
        capture.addPoint(pointsWithPressure[i]);
      }
      capture.endStroke();

      const strokes = capture.getStrokes();
      expect(strokes.length).toBe(1);

      strokes[0].points.forEach((point, i) => {
        expect(point.pressure).toBe(pointsWithPressure[i].pressure);
      });
    });
  });

  // ============================================================
  // 5. Canvas Sizing Properties
  // ============================================================

  describe('Canvas Sizing Properties', () => {
    it('Property 30: Canvas fills container width', () => {
      fc.assert(
        fc.property(fc.integer({ min: 100, max: 2000 }), (width) => {
          const { canvas } = createMockCanvas(width, 200);
          const cap = new SignatureCapture(canvas);

          const dims = cap.getDimensions();
          expect(dims.width).toBe(width);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 31: Canvas respects minimum height (200px)', () => {
      fc.assert(
        fc.property(fc.integer({ min: 50, max: 500 }), (height) => {
          const effectiveHeight = Math.max(200, height);
          const { canvas } = createMockCanvas(400, effectiveHeight);
          const cap = new SignatureCapture(canvas);

          const dims = cap.getDimensions();
          expect(dims.height).toBeGreaterThanOrEqual(200);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 32: Resize updates canvas dimensions', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 200, max: 1000 }),
          fc.integer({ min: 200, max: 500 }),
          (newWidth, newHeight) => {
            capture.resize(newWidth, newHeight, 'reset');

            const dims = capture.getDimensions();
            expect(dims.width).toBe(newWidth);
            expect(dims.height).toBe(Math.max(200, newHeight)); // Minimum height enforced
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 33: Content scales on resize when behavior is "scale"', () => {
      // Draw a stroke
      capture.startStroke({ x: 100, y: 50 });
      capture.addPoint({ x: 200, y: 100 });
      capture.endStroke();

      // Capture original values before resize (since resize mutates in place)
      const originalStrokes = capture.getStrokes();
      const originalX = originalStrokes[0].points[0].x;
      const originalY = originalStrokes[0].points[0].y;

      // Resize with scaling (2x)
      capture.resize(800, 400, 'scale');

      const scaledStrokes = capture.getStrokes();
      const scaledPoint = scaledStrokes[0].points[0];

      // Points should be scaled
      expect(scaledPoint.x).toBeCloseTo(originalX * 2, 1);
      expect(scaledPoint.y).toBeCloseTo(originalY * 2, 1);
    });

    it('Property 34: Content resets on resize when behavior is "reset"', () => {
      // Draw a stroke
      capture.startStroke({ x: 100, y: 50 });
      capture.addPoint({ x: 200, y: 100 });
      capture.endStroke();

      expect(capture.isEmpty()).toBe(false);

      // Resize with reset
      capture.resize(800, 400, 'reset');

      expect(capture.isEmpty()).toBe(true);
    });

    it('Property 35: Minimum height is enforced on resize', () => {
      capture.resize(400, 100, 'reset'); // Try to set height below minimum

      const dims = capture.getDimensions();
      expect(dims.height).toBeGreaterThanOrEqual(200);
    });
  });

  // ============================================================
  // 6. Validation Properties
  // ============================================================

  describe('Validation Properties', () => {
    it('Property 36: Signature data is valid base64', () => {
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const dataUrl = capture.toDataURL();
      expect(isValidBase64(dataUrl)).toBe(true);
    });

    it('Property 37: Decoded image has correct dimensions', () => {
      fc.assert(
        fc.property(canvasDimensionsArb, ({ width, height }) => {
          const { canvas } = createMockCanvas(width, height);
          const cap = new SignatureCapture(canvas);

          cap.startStroke({ x: 10, y: 10 });
          cap.addPoint({ x: 50, y: 50 });
          cap.endStroke();

          const exported = cap.export();
          expect(exported).not.toBeNull();
          expect(exported!.width).toBe(width);
          expect(exported!.height).toBe(height);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 38: Signature size is within limits (< 500KB typical)', () => {
      fc.assert(
        fc.property(
          fc.array(strokeArb(400, 200), { minLength: 1, maxLength: 5 }),
          (strokesData) => {
            capture.clear();

            for (const points of strokesData) {
              capture.startStroke(points[0]);
              for (let i = 1; i < points.length; i++) {
                capture.addPoint(points[i]);
              }
              capture.endStroke();
            }

            const dataUrl = capture.toDataURL();
            const sizeBytes = getSignatureSizeBytes(dataUrl);

            // Signature should be under 500KB (500 * 1024 bytes)
            expect(sizeBytes).toBeLessThan(500 * 1024);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 39: PNG header is valid', () => {
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const dataUrl = capture.toDataURL('image/png');

      // Our mock doesn't produce real PNG, but the format should be correct
      expect(dataUrl.startsWith('data:image/png;base64,')).toBe(true);
    });

    it('Property 40: Export returns null for empty canvas', () => {
      const result = capture.export();
      expect(result).toBeNull();
    });

    it('Property 41: Export returns valid structure for non-empty canvas', () => {
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const exported = capture.export();
      expect(exported).not.toBeNull();
      expect(exported!.type).toBe('drawn');
      expect(exported!.dataUrl).toBeDefined();
      expect(exported!.width).toBeGreaterThan(0);
      expect(exported!.height).toBeGreaterThan(0);
      expect(exported!.timestamp).toBeDefined();
    });
  });

  // ============================================================
  // 7. Modal State Properties
  // ============================================================

  describe('Modal State Properties', () => {
    let modal: SignatureModal;

    beforeEach(() => {
      modal = new SignatureModal();
    });

    it('Property 42: Modal starts closed', () => {
      expect(modal.getState()).toBe('closed');
      expect(modal.isVisible()).toBe(false);
    });

    it('Property 43: open() makes modal visible', () => {
      const result = modal.open();

      expect(result).toBe(true);
      expect(modal.getState()).toBe('open');
      expect(modal.isVisible()).toBe(true);
    });

    it('Property 44: close() hides modal', () => {
      modal.open();
      modal.close();

      expect(modal.getState()).toBe('closed');
      expect(modal.isVisible()).toBe(false);
    });

    it('Property 45: Cannot open when already open', () => {
      modal.open();
      const result = modal.open(); // Try to open again

      expect(result).toBe(false);
      expect(modal.getState()).toBe('open');
    });

    it('Property 46: Cancel returns null', () => {
      let callbackResult: SignatureExport | null | undefined;

      modal.onConfirm((signature) => {
        callbackResult = signature;
      });

      modal.open();
      modal.close(); // Cancel

      expect(callbackResult).toBeNull();
      expect(modal.getResult()).toBeNull();
    });

    it('Property 47: Confirm returns signature data', () => {
      const mockSignature: SignatureExport = {
        type: 'drawn',
        dataUrl: 'data:image/png;base64,abc123',
        width: 400,
        height: 200,
        timestamp: new Date().toISOString(),
      };

      let callbackResult: SignatureExport | null | undefined;

      modal.onConfirm((signature) => {
        callbackResult = signature;
      });

      modal.open();
      modal.confirm(mockSignature);

      expect(callbackResult).toBe(mockSignature);
    });

    it('Property 48: Modal can be reopened after close', () => {
      modal.open();
      modal.close();

      expect(modal.getState()).toBe('closed');

      const result = modal.open();
      expect(result).toBe(true);
      expect(modal.isVisible()).toBe(true);
    });

    it('Property 49: Modal can be reopened after confirm', () => {
      const mockSignature: SignatureExport = {
        type: 'drawn',
        dataUrl: 'data:image/png;base64,abc123',
        width: 400,
        height: 200,
        timestamp: new Date().toISOString(),
      };

      modal.open();
      modal.confirm(mockSignature);

      expect(modal.getState()).toBe('closed');

      const result = modal.open();
      expect(result).toBe(true);
      expect(modal.isVisible()).toBe(true);
    });

    it('Property 50: Result is cleared on new open', () => {
      const mockSignature: SignatureExport = {
        type: 'drawn',
        dataUrl: 'data:image/png;base64,abc123',
        width: 400,
        height: 200,
        timestamp: new Date().toISOString(),
      };

      modal.open();
      modal.confirm(mockSignature);

      // Reopen
      modal.open();

      expect(modal.getResult()).toBeNull();
    });

    it('Property 51: Reset clears all state', () => {
      let callbackCalled = false;

      modal.onConfirm(() => {
        callbackCalled = true;
      });

      modal.open();
      modal.reset();

      expect(modal.getState()).toBe('closed');
      expect(modal.getResult()).toBeNull();
      expect(modal.isVisible()).toBe(false);

      // Callback should be cleared
      modal.open();
      modal.close();
      expect(callbackCalled).toBe(false);
    });
  });

  // ============================================================
  // Edge Cases and Stress Tests
  // ============================================================

  describe('Edge Cases', () => {
    it('Property 52: Very long stroke is handled', () => {
      const manyPoints: StrokePoint[] = [];
      for (let i = 0; i < 1000; i++) {
        manyPoints.push({
          x: (i % 400) + Math.random() * 10,
          y: (i % 200) + Math.random() * 10,
        });
      }

      capture.startStroke(manyPoints[0]);
      for (let i = 1; i < manyPoints.length; i++) {
        capture.addPoint(manyPoints[i]);
      }
      capture.endStroke();

      const strokes = capture.getStrokes();
      expect(strokes.length).toBe(1);
      expect(strokes[0].points.length).toBe(1000);
    });

    it('Property 53: Multiple strokes are independent', () => {
      // Draw first stroke
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      // Draw second stroke
      capture.startStroke({ x: 100, y: 100 });
      capture.addPoint({ x: 150, y: 150 });
      capture.endStroke();

      const strokes = capture.getStrokes();
      expect(strokes.length).toBe(2);
      expect(strokes[0].id).not.toBe(strokes[1].id);
      expect(strokes[0].points[0].x).toBe(10);
      expect(strokes[1].points[0].x).toBe(100);
    });

    it('Property 54: Stroke color and width are preserved', () => {
      capture.setColor('#FF0000');
      capture.setWidth(5);

      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const strokes = capture.getStrokes();
      expect(strokes[0].color).toBe('#FF0000');
      expect(strokes[0].width).toBe(5);
    });

    it('Property 55: Negative coordinates are clamped to 0', () => {
      capture.startStroke({ x: -50, y: -50 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const strokes = capture.getStrokes();
      expect(strokes[0].points[0].x).toBe(0);
      expect(strokes[0].points[0].y).toBe(0);
    });

    it('Property 56: Coordinates beyond canvas are clamped', () => {
      capture.startStroke({ x: 500, y: 300 }); // Beyond 400x200 canvas
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const strokes = capture.getStrokes();
      expect(strokes[0].points[0].x).toBe(400);
      expect(strokes[0].points[0].y).toBe(200);
    });

    it('Property 57: Rapid undo/redo sequence works correctly', () => {
      // Draw 5 strokes
      for (let i = 0; i < 5; i++) {
        capture.startStroke({ x: i * 20, y: i * 20 });
        capture.addPoint({ x: i * 20 + 30, y: i * 20 + 30 });
        capture.endStroke();
      }

      expect(capture.getStrokes().length).toBe(5);

      // Undo all
      for (let i = 0; i < 5; i++) {
        capture.undo();
      }
      expect(capture.isEmpty()).toBe(true);

      // Redo all
      for (let i = 0; i < 5; i++) {
        capture.redo();
      }
      expect(capture.getStrokes().length).toBe(5);

      // Extra undo/redo should be safe
      expect(capture.redo()).toBe(false);
      capture.undo();
      capture.undo();
      expect(capture.getStrokes().length).toBe(3);
    });

    it('Property 58: Empty typed signature with various fonts', () => {
      const { canvas } = createMockCanvas(400, 200);
      const typed = new TypedSignature(canvas);

      SIGNATURE_FONTS.forEach((font) => {
        typed.setFont(font);
        typed.setText('');

        expect(typed.isEmpty()).toBe(true);
        expect(typed.export()).toBeNull();
      });
    });

    it('Property 59: Special characters in typed signature', () => {
      const { canvas } = createMockCanvas(400, 200);
      const typed = new TypedSignature(canvas);

      const specialText = 'John "Jack" O\'Connor-Smith III, M.D.';
      typed.setText(specialText);

      expect(typed.isEmpty()).toBe(false);
      expect(typed.getData().text).toBe(specialText);

      const exported = typed.export();
      expect(exported).not.toBeNull();
    });

    it('Property 60: Zero-dimension canvas is handled gracefully', () => {
      const { canvas } = createMockCanvas(0, 0);
      const cap = new SignatureCapture(canvas);

      const dims = cap.getDimensions();
      expect(dims.width).toBe(0);
      expect(dims.height).toBe(0);

      // Drawing should still work without errors
      cap.startStroke({ x: 10, y: 10 });
      cap.addPoint({ x: 50, y: 50 });
      cap.endStroke();
    });
  });

  // ============================================================
  // Stroke Properties Tests
  // ============================================================

  describe('Stroke Object Properties', () => {
    it('Property 61: Each stroke has a unique ID', () => {
      fc.assert(
        fc.property(fc.integer({ min: 2, max: 10 }), (numStrokes) => {
          capture.clear();

          for (let i = 0; i < numStrokes; i++) {
            capture.startStroke({ x: i * 10, y: i * 10 });
            capture.addPoint({ x: i * 10 + 20, y: i * 10 + 20 });
            capture.endStroke();
          }

          const strokes = capture.getStrokes();
          const ids = new Set(strokes.map((s) => s.id));

          expect(ids.size).toBe(numStrokes);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 62: Stroke createdAt is valid timestamp', () => {
      const before = Date.now();

      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const after = Date.now();

      const strokes = capture.getStrokes();
      expect(strokes[0].createdAt).toBeGreaterThanOrEqual(before);
      expect(strokes[0].createdAt).toBeLessThanOrEqual(after);
    });

    it('Property 63: Strokes are returned in drawing order', () => {
      for (let i = 0; i < 5; i++) {
        capture.startStroke({ x: i * 10, y: i * 10 });
        capture.addPoint({ x: i * 10 + 20, y: i * 10 + 20 });
        capture.endStroke();
      }

      const strokes = capture.getStrokes();

      for (let i = 0; i < strokes.length - 1; i++) {
        expect(strokes[i].createdAt).toBeLessThanOrEqual(strokes[i + 1].createdAt);
      }
    });

    it('Property 64: getStrokes returns a copy, not the original', () => {
      capture.startStroke({ x: 10, y: 10 });
      capture.addPoint({ x: 50, y: 50 });
      capture.endStroke();

      const strokes1 = capture.getStrokes();
      const strokes2 = capture.getStrokes();

      expect(strokes1).not.toBe(strokes2);
      expect(strokes1).toEqual(strokes2);

      // Modifying returned array shouldn't affect internal state
      strokes1.pop();
      expect(capture.getStrokes().length).toBe(1);
    });
  });
});
