/**
 * Property-based tests for coordinate conversion utilities
 */

import { describe, it, expect, vi } from 'vitest';
import * as fc from 'fast-check';
import type { PDFJSViewport } from '../types/pdf-types';

// ============================================================
// Mock Viewport for Testing
// ============================================================

/**
 * Create a mock PDF.js viewport for testing coordinate conversions
 */
function createMockViewport(
  width: number,
  height: number,
  scale: number = 1.0,
  rotation: number = 0
): PDFJSViewport {
  return {
    width: width * scale,
    height: height * scale,
    scale,
    rotation,
    viewBox: [0, 0, width, height],
    transform: [scale, 0, 0, -scale, 0, height * scale],
    offsetX: 0,
    offsetY: 0,
    convertToPdfPoint: (domX: number, domY: number): [number, number] => {
      // DOM to PDF: flip Y axis, apply inverse scale
      const pdfX = domX / scale;
      const pdfY = height - domY / scale;
      return [pdfX, pdfY];
    },
    convertToViewportPoint: (pdfX: number, pdfY: number): [number, number] => {
      // PDF to DOM: flip Y axis, apply scale
      const domX = pdfX * scale;
      const domY = (height - pdfY) * scale;
      return [domX, domY];
    },
    convertToViewportRectangle: (
      rect: [number, number, number, number]
    ): [number, number, number, number] => {
      const [x1, y1, x2, y2] = rect;
      const domX1 = x1 * scale;
      const domY1 = (height - y2) * scale;
      const domX2 = x2 * scale;
      const domY2 = (height - y1) * scale;
      return [domX1, domY1, domX2, domY2];
    },
    clone: vi.fn(),
  };
}

// ============================================================
// Pure Coordinate Functions for Testing
// ============================================================

interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

function domRectToPdf(
  viewport: PDFJSViewport,
  domX: number,
  domY: number,
  domWidth: number,
  domHeight: number
): Rect {
  const [pdfX1, pdfY1] = viewport.convertToPdfPoint(domX, domY);
  const [pdfX2, pdfY2] = viewport.convertToPdfPoint(domX + domWidth, domY + domHeight);

  return {
    x: Math.min(pdfX1, pdfX2),
    y: Math.min(pdfY1, pdfY2),
    width: Math.abs(pdfX2 - pdfX1),
    height: Math.abs(pdfY2 - pdfY1),
  };
}

function pdfRectToDom(
  viewport: PDFJSViewport,
  pdfX: number,
  pdfY: number,
  pdfWidth: number,
  pdfHeight: number
): Rect {
  const pdfRect: [number, number, number, number] = [
    pdfX,
    pdfY,
    pdfX + pdfWidth,
    pdfY + pdfHeight,
  ];

  const [domX1, domY1, domX2, domY2] = viewport.convertToViewportRectangle(pdfRect);

  return {
    x: Math.min(domX1, domX2),
    y: Math.min(domY1, domY2),
    width: Math.abs(domX2 - domX1),
    height: Math.abs(domY2 - domY1),
  };
}

// ============================================================
// Property Tests
// ============================================================

describe('Coordinate Conversion Property Tests', () => {
  describe('DOM to PDF Conversion', () => {
    it('should preserve rectangle area under conversion', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 100, max: 1000, noNaN: true }),
          fc.float({ min: 100, max: 1000, noNaN: true }),
          fc.float({ min: 0.5, max: 3.0, noNaN: true }),
          fc.float({ min: 0, max: 500, noNaN: true }),
          fc.float({ min: 0, max: 500, noNaN: true }),
          fc.float({ min: 10, max: 200, noNaN: true }),
          fc.float({ min: 10, max: 200, noNaN: true }),
          (pageWidth, pageHeight, scale, domX, domY, domWidth, domHeight) => {
            const viewport = createMockViewport(pageWidth, pageHeight, scale);
            const pdfRect = domRectToPdf(viewport, domX, domY, domWidth, domHeight);

            // Area should be preserved (scaled appropriately)
            const domArea = domWidth * domHeight;
            const pdfArea = pdfRect.width * pdfRect.height;
            const expectedPdfArea = domArea / (scale * scale);

            expect(Math.abs(pdfArea - expectedPdfArea)).toBeLessThan(0.01);
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should produce positive dimensions', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 100, max: 1000, noNaN: true }),
          fc.float({ min: 100, max: 1000, noNaN: true }),
          fc.float({ min: 0, max: 500, noNaN: true }),
          fc.float({ min: 0, max: 500, noNaN: true }),
          fc.float({ min: 1, max: 200, noNaN: true }),
          fc.float({ min: 1, max: 200, noNaN: true }),
          (pageWidth, pageHeight, domX, domY, domWidth, domHeight) => {
            const viewport = createMockViewport(pageWidth, pageHeight);
            const pdfRect = domRectToPdf(viewport, domX, domY, domWidth, domHeight);

            expect(pdfRect.width).toBeGreaterThan(0);
            expect(pdfRect.height).toBeGreaterThan(0);
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should handle zero origin correctly', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 100, max: 1000, noNaN: true }),
          fc.float({ min: 100, max: 1000, noNaN: true }),
          fc.float({ min: 10, max: 200, noNaN: true }),
          fc.float({ min: 10, max: 200, noNaN: true }),
          (pageWidth, pageHeight, domWidth, domHeight) => {
            const viewport = createMockViewport(pageWidth, pageHeight);
            const pdfRect = domRectToPdf(viewport, 0, 0, domWidth, domHeight);

            // DOM origin (0,0) should produce valid PDF coordinates
            expect(pdfRect.x).toBe(0);
            // Y coordinate is valid (could be anywhere depending on page height)
            expect(Number.isFinite(pdfRect.y)).toBe(true);
          }
        ),
        { numRuns: 30 }
      );
    });
  });

  describe('PDF to DOM Conversion', () => {
    it('should produce positive dimensions', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 100, max: 1000, noNaN: true }),
          fc.float({ min: 100, max: 1000, noNaN: true }),
          fc.float({ min: 0, max: 400, noNaN: true }),
          fc.float({ min: 0, max: 400, noNaN: true }),
          fc.float({ min: 10, max: 200, noNaN: true }),
          fc.float({ min: 10, max: 200, noNaN: true }),
          (pageWidth, pageHeight, pdfX, pdfY, pdfWidth, pdfHeight) => {
            const viewport = createMockViewport(pageWidth, pageHeight);
            const domRect = pdfRectToDom(viewport, pdfX, pdfY, pdfWidth, pdfHeight);

            expect(domRect.width).toBeGreaterThan(0);
            expect(domRect.height).toBeGreaterThan(0);
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should scale dimensions by viewport scale', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 100, max: 1000, noNaN: true }),
          fc.float({ min: 100, max: 1000, noNaN: true }),
          fc.float({ min: 0.5, max: 3.0, noNaN: true }),
          fc.float({ min: 10, max: 200, noNaN: true }),
          fc.float({ min: 10, max: 200, noNaN: true }),
          (pageWidth, pageHeight, scale, pdfWidth, pdfHeight) => {
            const viewport = createMockViewport(pageWidth, pageHeight, scale);
            const domRect = pdfRectToDom(viewport, 0, 0, pdfWidth, pdfHeight);

            expect(Math.abs(domRect.width - pdfWidth * scale)).toBeLessThan(0.01);
            expect(Math.abs(domRect.height - pdfHeight * scale)).toBeLessThan(0.01);
          }
        ),
        { numRuns: 50 }
      );
    });
  });

  describe('Roundtrip Conversion', () => {
    it('should preserve rectangle after DOM -> PDF -> DOM conversion', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 200, max: 800, noNaN: true }),
          fc.float({ min: 200, max: 800, noNaN: true }),
          fc.float({ min: 10, max: 100, noNaN: true }),
          fc.float({ min: 10, max: 100, noNaN: true }),
          fc.float({ min: 20, max: 100, noNaN: true }),
          fc.float({ min: 20, max: 100, noNaN: true }),
          (pageWidth, pageHeight, domX, domY, domWidth, domHeight) => {
            const viewport = createMockViewport(pageWidth, pageHeight);

            // DOM -> PDF
            const pdfRect = domRectToPdf(viewport, domX, domY, domWidth, domHeight);

            // PDF -> DOM
            const roundtripRect = pdfRectToDom(
              viewport,
              pdfRect.x,
              pdfRect.y,
              pdfRect.width,
              pdfRect.height
            );

            // Should approximately match original
            expect(Math.abs(roundtripRect.width - domWidth)).toBeLessThan(1);
            expect(Math.abs(roundtripRect.height - domHeight)).toBeLessThan(1);
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should preserve rectangle after PDF -> DOM -> PDF conversion', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 200, max: 800, noNaN: true }),
          fc.float({ min: 200, max: 800, noNaN: true }),
          fc.float({ min: 10, max: 300, noNaN: true }),
          fc.float({ min: 10, max: 300, noNaN: true }),
          fc.float({ min: 20, max: 100, noNaN: true }),
          fc.float({ min: 20, max: 100, noNaN: true }),
          (pageWidth, pageHeight, pdfX, pdfY, pdfWidth, pdfHeight) => {
            const viewport = createMockViewport(pageWidth, pageHeight);

            // PDF -> DOM
            const domRect = pdfRectToDom(viewport, pdfX, pdfY, pdfWidth, pdfHeight);

            // DOM -> PDF
            const roundtripRect = domRectToPdf(
              viewport,
              domRect.x,
              domRect.y,
              domRect.width,
              domRect.height
            );

            // Should approximately match original
            expect(Math.abs(roundtripRect.width - pdfWidth)).toBeLessThan(1);
            expect(Math.abs(roundtripRect.height - pdfHeight)).toBeLessThan(1);
          }
        ),
        { numRuns: 50 }
      );
    });
  });

  describe('Scale Invariants', () => {
    it('should maintain proportions across different scales', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 100, max: 800, noNaN: true }),
          fc.float({ min: 100, max: 800, noNaN: true }),
          fc.float({ min: 0.5, max: 2.0, noNaN: true }),
          fc.float({ min: 1.5, max: 3.0, noNaN: true }),
          fc.float({ min: 50, max: 200, noNaN: true }),
          fc.float({ min: 25, max: 100, noNaN: true }),
          (pageWidth, pageHeight, scale1, scale2, pdfWidth, pdfHeight) => {
            const viewport1 = createMockViewport(pageWidth, pageHeight, scale1);
            const viewport2 = createMockViewport(pageWidth, pageHeight, scale2);

            const domRect1 = pdfRectToDom(viewport1, 0, 0, pdfWidth, pdfHeight);
            const domRect2 = pdfRectToDom(viewport2, 0, 0, pdfWidth, pdfHeight);

            // Aspect ratio should be preserved
            const ratio1 = domRect1.width / domRect1.height;
            const ratio2 = domRect2.width / domRect2.height;

            expect(Math.abs(ratio1 - ratio2)).toBeLessThan(0.01);
          }
        ),
        { numRuns: 30 }
      );
    });
  });

  describe('Edge Cases', () => {
    it('should handle small rectangles', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 100, max: 800, noNaN: true }),
          fc.float({ min: 100, max: 800, noNaN: true }),
          fc.float({ min: 0, max: 100, noNaN: true }),
          fc.float({ min: 0, max: 100, noNaN: true }),
          fc.float({ min: 1, max: 5, noNaN: true }),
          fc.float({ min: 1, max: 5, noNaN: true }),
          (pageWidth, pageHeight, x, y, width, height) => {
            const viewport = createMockViewport(pageWidth, pageHeight);
            const pdfRect = domRectToPdf(viewport, x, y, width, height);

            // Should still produce valid output
            expect(Number.isFinite(pdfRect.x)).toBe(true);
            expect(Number.isFinite(pdfRect.y)).toBe(true);
            expect(pdfRect.width).toBeGreaterThan(0);
            expect(pdfRect.height).toBeGreaterThan(0);
          }
        ),
        { numRuns: 30 }
      );
    });

    it('should handle large page dimensions', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 1000, max: 5000, noNaN: true }),
          fc.float({ min: 1000, max: 5000, noNaN: true }),
          fc.float({ min: 0, max: 500, noNaN: true }),
          fc.float({ min: 0, max: 500, noNaN: true }),
          fc.float({ min: 50, max: 200, noNaN: true }),
          fc.float({ min: 50, max: 200, noNaN: true }),
          (pageWidth, pageHeight, x, y, width, height) => {
            const viewport = createMockViewport(pageWidth, pageHeight);
            const pdfRect = domRectToPdf(viewport, x, y, width, height);

            expect(Number.isFinite(pdfRect.x)).toBe(true);
            expect(Number.isFinite(pdfRect.y)).toBe(true);
            expect(Number.isFinite(pdfRect.width)).toBe(true);
            expect(Number.isFinite(pdfRect.height)).toBe(true);
          }
        ),
        { numRuns: 20 }
      );
    });
  });
});

describe('Point Conversion Tests', () => {
  it('should flip Y axis correctly', () => {
    fc.assert(
      fc.property(
        fc.float({ min: 100, max: 800, noNaN: true }),
        fc.float({ min: 100, max: 800, noNaN: true }),
        fc.float({ min: 0, max: 400, noNaN: true }),
        fc.float({ min: 0, max: 400, noNaN: true }),
        (pageWidth, pageHeight, domX, domY) => {
          const viewport = createMockViewport(pageWidth, pageHeight);
          const [pdfX, pdfY] = viewport.convertToPdfPoint(domX, domY);

          // X should be unchanged (at scale 1)
          expect(pdfX).toBe(domX);

          // Y should be flipped
          expect(pdfY).toBe(pageHeight - domY);
        }
      ),
      { numRuns: 50 }
    );
  });

  it('should apply scale to point conversion', () => {
    fc.assert(
      fc.property(
        fc.float({ min: 100, max: 500, noNaN: true }),
        fc.float({ min: 100, max: 500, noNaN: true }),
        fc.float({ min: 0.5, max: 3.0, noNaN: true }),
        fc.float({ min: 0, max: 200, noNaN: true }),
        fc.float({ min: 0, max: 200, noNaN: true }),
        (pageWidth, pageHeight, scale, domX, domY) => {
          const viewport = createMockViewport(pageWidth, pageHeight, scale);
          const [pdfX, pdfY] = viewport.convertToPdfPoint(domX, domY);

          // PDF coordinates should be unscaled
          expect(Math.abs(pdfX - domX / scale)).toBeLessThan(0.01);
        }
      ),
      { numRuns: 50 }
    );
  });
});
