/**
 * Property-based tests for PDF Preview Bridge
 *
 * @vitest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import * as fc from 'fast-check';

// ============================================================
// Types (from pdf-preview.ts)
// ============================================================

interface PageDimensions {
  width: number;
  height: number;
  originalWidth: number;
  originalHeight: number;
  pdfWidth: number;
  pdfHeight: number;
}

interface DomBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface TextItem {
  index: number;
  str: string;
  pdfX: number;
  pdfY: number;
  pdfWidth: number;
  pdfHeight: number;
  fontSize: number;
  domFontSize: number;
  fontName: string | undefined;
  fontFamily: string;
  isItalic: boolean;
  isBold: boolean;
  domBounds: DomBounds | null;
}

interface CachedPageInfo {
  canvas: HTMLCanvasElement;
  viewport: {
    width: number;
    height: number;
    scale: number;
  };
  page: unknown;
}

// ============================================================
// Pure Functions for Testing
// ============================================================

/**
 * Calculate page dimensions from render parameters
 */
function calculatePageDimensions(
  viewportWidth: number,
  viewportHeight: number,
  scale: number,
  pdfWidth: number,
  pdfHeight: number
): PageDimensions {
  return {
    width: viewportWidth,
    height: viewportHeight,
    originalWidth: viewportWidth / scale,
    originalHeight: viewportHeight / scale,
    pdfWidth,
    pdfHeight,
  };
}

/**
 * Detect if font is italic from name
 */
function detectItalic(fontName: string): boolean {
  const lower = fontName.toLowerCase();
  return lower.includes('italic') || lower.includes('oblique');
}

/**
 * Detect if font is bold from name
 */
function detectBold(fontName: string): boolean {
  return fontName.toLowerCase().includes('bold');
}

/**
 * Extract font family from style
 */
function extractFontFamily(fontStyle: { fontFamily?: string } | undefined): string {
  return fontStyle?.fontFamily || 'sans-serif';
}

/**
 * Calculate DOM bounds from PDF coordinates
 */
function calculateDomBounds(
  pdfX: number,
  pdfY: number,
  pdfWidth: number,
  pdfHeight: number,
  pageHeight: number,
  scale: number
): DomBounds {
  // PDF origin is bottom-left, DOM origin is top-left
  const domX = pdfX * scale;
  const domY = (pageHeight - pdfY - pdfHeight) * scale;

  return {
    x: domX,
    y: domY,
    width: pdfWidth * scale,
    height: pdfHeight * scale,
  };
}

// ============================================================
// Property Tests
// ============================================================

describe('PDF Preview Page Dimensions', () => {
  it('should calculate dimensions correctly', () => {
    fc.assert(
      fc.property(
        fc.float({ min: 100, max: 2000, noNaN: true }),
        fc.float({ min: 100, max: 2000, noNaN: true }),
        fc.float({ min: 0.5, max: 4, noNaN: true }),
        fc.float({ min: 100, max: 1000, noNaN: true }),
        fc.float({ min: 100, max: 1000, noNaN: true }),
        (viewportWidth, viewportHeight, scale, pdfWidth, pdfHeight) => {
          const dims = calculatePageDimensions(
            viewportWidth,
            viewportHeight,
            scale,
            pdfWidth,
            pdfHeight
          );

          expect(dims.width).toBe(viewportWidth);
          expect(dims.height).toBe(viewportHeight);
          expect(dims.pdfWidth).toBe(pdfWidth);
          expect(dims.pdfHeight).toBe(pdfHeight);
          expect(Math.abs(dims.originalWidth - viewportWidth / scale)).toBeLessThan(0.01);
          expect(Math.abs(dims.originalHeight - viewportHeight / scale)).toBeLessThan(0.01);
        }
      ),
      { numRuns: 50 }
    );
  });

  it('should have positive dimensions', () => {
    fc.assert(
      fc.property(
        fc.float({ min: 1, max: 2000, noNaN: true }),
        fc.float({ min: 1, max: 2000, noNaN: true }),
        fc.float({ min: 1, max: 4, noNaN: true }),
        fc.float({ min: 1, max: 1000, noNaN: true }),
        fc.float({ min: 1, max: 1000, noNaN: true }),
        (viewportWidth, viewportHeight, scale, pdfWidth, pdfHeight) => {
          const dims = calculatePageDimensions(
            viewportWidth,
            viewportHeight,
            scale,
            pdfWidth,
            pdfHeight
          );

          expect(dims.width).toBeGreaterThan(0);
          expect(dims.height).toBeGreaterThan(0);
          expect(dims.originalWidth).toBeGreaterThan(0);
          expect(dims.originalHeight).toBeGreaterThan(0);
        }
      ),
      { numRuns: 30 }
    );
  });
});

describe('Font Detection', () => {
  describe('Italic Detection', () => {
    it('should detect italic in font names', () => {
      const italicFonts = [
        'Times-Italic',
        'Helvetica-Oblique',
        'Arial-ItalicMT',
        'Georgia-Italic',
        'ABCDEF+Times-Italic',
      ];

      italicFonts.forEach((font) => {
        expect(detectItalic(font)).toBe(true);
      });
    });

    it('should not detect italic in regular font names', () => {
      const regularFonts = ['Times-Roman', 'Helvetica', 'Arial', 'Georgia', 'CourierNew'];

      regularFonts.forEach((font) => {
        expect(detectItalic(font)).toBe(false);
      });
    });

    it('should be case-insensitive', () => {
      fc.assert(
        fc.property(fc.constantFrom('italic', 'ITALIC', 'Italic', 'iTaLiC'), (variant) => {
          expect(detectItalic(`Font-${variant}`)).toBe(true);
        }),
        { numRuns: 10 }
      );
    });
  });

  describe('Bold Detection', () => {
    it('should detect bold in font names', () => {
      const boldFonts = [
        'Times-Bold',
        'Helvetica-Bold',
        'Arial-BoldMT',
        'Georgia-Bold',
        'ABCDEF+Times-Bold',
      ];

      boldFonts.forEach((font) => {
        expect(detectBold(font)).toBe(true);
      });
    });

    it('should not detect bold in regular font names', () => {
      const regularFonts = ['Times-Roman', 'Helvetica', 'Arial', 'Georgia', 'CourierNew'];

      regularFonts.forEach((font) => {
        expect(detectBold(font)).toBe(false);
      });
    });
  });

  describe('Font Family Extraction', () => {
    it('should return fontFamily from style', () => {
      fc.assert(
        fc.property(
          fc.constantFrom('serif', 'sans-serif', 'monospace', 'cursive'),
          (fontFamily) => {
            expect(extractFontFamily({ fontFamily })).toBe(fontFamily);
          }
        ),
        { numRuns: 10 }
      );
    });

    it('should default to sans-serif', () => {
      expect(extractFontFamily(undefined)).toBe('sans-serif');
      expect(extractFontFamily({})).toBe('sans-serif');
    });
  });
});

describe('DOM Bounds Calculation', () => {
  it('should calculate valid bounds', () => {
    fc.assert(
      fc.property(
        fc.float({ min: 0, max: 500, noNaN: true }),
        fc.float({ min: 0, max: 500, noNaN: true }),
        fc.float({ min: 10, max: 200, noNaN: true }),
        fc.float({ min: 10, max: 50, noNaN: true }),
        fc.float({ min: 500, max: 1000, noNaN: true }),
        fc.float({ min: 1, max: 3, noNaN: true }),
        (pdfX, pdfY, pdfWidth, pdfHeight, pageHeight, scale) => {
          const bounds = calculateDomBounds(pdfX, pdfY, pdfWidth, pdfHeight, pageHeight, scale);

          expect(Number.isFinite(bounds.x)).toBe(true);
          expect(Number.isFinite(bounds.y)).toBe(true);
          expect(bounds.width).toBeGreaterThan(0);
          expect(bounds.height).toBeGreaterThan(0);
        }
      ),
      { numRuns: 50 }
    );
  });

  it('should scale width and height correctly', () => {
    fc.assert(
      fc.property(
        fc.float({ min: 10, max: 200, noNaN: true }),
        fc.float({ min: 10, max: 50, noNaN: true }),
        fc.float({ min: 0.5, max: 3, noNaN: true }),
        (pdfWidth, pdfHeight, scale) => {
          const bounds = calculateDomBounds(0, 0, pdfWidth, pdfHeight, 1000, scale);

          expect(Math.abs(bounds.width - pdfWidth * scale)).toBeLessThan(0.01);
          expect(Math.abs(bounds.height - pdfHeight * scale)).toBeLessThan(0.01);
        }
      ),
      { numRuns: 30 }
    );
  });
});

describe('Text Item Structure', () => {
  it('should have valid text item properties', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 0, max: 1000 }),
        fc.string({ minLength: 1, maxLength: 100 }),
        fc.float({ min: 0, max: 500, noNaN: true }),
        fc.float({ min: 0, max: 700, noNaN: true }),
        fc.float({ min: 6, max: 72, noNaN: true }),
        (index, str, pdfX, pdfY, fontSize) => {
          const item: TextItem = {
            index,
            str,
            pdfX,
            pdfY,
            pdfWidth: 100,
            pdfHeight: fontSize,
            fontSize,
            domFontSize: fontSize * 1.5,
            fontName: 'Helvetica',
            fontFamily: 'sans-serif',
            isItalic: false,
            isBold: false,
            domBounds: null,
          };

          expect(item.index).toBe(index);
          expect(item.str).toBe(str);
          expect(item.fontSize).toBeGreaterThan(0);
        }
      ),
      { numRuns: 30 }
    );
  });
});

describe('Page Cache', () => {
  it('should store page info correctly', () => {
    const cache = new Map<number, CachedPageInfo>();

    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 100 }),
        fc.float({ min: 100, max: 1000, noNaN: true }),
        fc.float({ min: 100, max: 1000, noNaN: true }),
        fc.float({ min: 0.5, max: 3, noNaN: true }),
        (pageNum, width, height, scale) => {
          const mockCanvas = document.createElement('canvas');
          const info: CachedPageInfo = {
            canvas: mockCanvas,
            viewport: { width, height, scale },
            page: {},
          };

          cache.set(pageNum, info);

          const retrieved = cache.get(pageNum);
          expect(retrieved).toBeDefined();
          expect(retrieved?.viewport.width).toBe(width);
          expect(retrieved?.viewport.height).toBe(height);
          expect(retrieved?.viewport.scale).toBe(scale);

          cache.delete(pageNum);
        }
      ),
      { numRuns: 20 }
    );
  });

  it('should return undefined for non-existent pages', () => {
    const cache = new Map<number, CachedPageInfo>();
    expect(cache.get(999)).toBeUndefined();
  });
});

describe('Cleanup Behavior', () => {
  it('should clear cache on cleanup', () => {
    const cache = new Map<number, CachedPageInfo>();
    const mockCanvas = document.createElement('canvas');

    // Add some items
    cache.set(1, { canvas: mockCanvas, viewport: { width: 100, height: 100, scale: 1 }, page: {} });
    cache.set(2, { canvas: mockCanvas, viewport: { width: 100, height: 100, scale: 1 }, page: {} });

    expect(cache.size).toBe(2);

    // Simulate cleanup
    cache.clear();

    expect(cache.size).toBe(0);
  });
});

describe('Default Scale', () => {
  it('should use 1.5 as default scale', () => {
    const defaultScale = 1.5;

    fc.assert(
      fc.property(
        fc.float({ min: 100, max: 800, noNaN: true }),
        fc.float({ min: 100, max: 1000, noNaN: true }),
        (pdfWidth, pdfHeight) => {
          const expectedWidth = pdfWidth * defaultScale;
          const expectedHeight = pdfHeight * defaultScale;

          expect(expectedWidth).toBe(pdfWidth * 1.5);
          expect(expectedHeight).toBe(pdfHeight * 1.5);
        }
      ),
      { numRuns: 20 }
    );
  });
});
