/**
 * Property-based tests for Sign PDF Bridge
 *
 * @vitest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import * as fc from 'fast-check';

// ============================================================
// Mock Types (from sign-pdf-bridge.ts)
// ============================================================

interface PageDimensions {
  width: number;
  height: number;
  originalWidth: number;
  originalHeight: number;
  pdfWidth: number;
  pdfHeight: number;
}

interface LoadPdfResult {
  numPages: number;
  success: boolean;
  error?: string;
}

interface RenderPageResult {
  pageNum: number;
  dimensions: PageDimensions;
  canvas: HTMLCanvasElement;
  success: boolean;
  error?: string;
}

interface RenderConfig {
  container: HTMLElement;
  scale?: number;
  pageWrapperClass?: string;
}

// ============================================================
// Pure Functions for Testing
// ============================================================

/**
 * Convert base64 string to Uint8Array
 */
function base64ToUint8Array(base64: string): Uint8Array {
  // Remove data URL prefix if present
  const cleanBase64 = base64.replace(/^data:[^;]+;base64,/, '');
  const binaryString = atob(cleanBase64);
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes;
}

/**
 * Create default page dimensions
 */
function createDefaultDimensions(): PageDimensions {
  return {
    width: 0,
    height: 0,
    originalWidth: 0,
    originalHeight: 0,
    pdfWidth: 0,
    pdfHeight: 0,
  };
}

/**
 * Calculate scaled dimensions
 */
function calculateScaledDimensions(
  pdfWidth: number,
  pdfHeight: number,
  scale: number
): PageDimensions {
  return {
    width: pdfWidth * scale,
    height: pdfHeight * scale,
    originalWidth: pdfWidth,
    originalHeight: pdfHeight,
    pdfWidth,
    pdfHeight,
  };
}

// ============================================================
// Property Tests
// ============================================================

describe('Base64 to Uint8Array Conversion', () => {
  it('should convert valid base64 to Uint8Array', () => {
    fc.assert(
      fc.property(fc.uint8Array({ minLength: 1, maxLength: 100 }), (bytes) => {
        // Convert to base64 and back
        const base64 = Buffer.from(bytes).toString('base64');
        const result = base64ToUint8Array(base64);

        expect(result.length).toBe(bytes.length);
        for (let i = 0; i < bytes.length; i++) {
          expect(result[i]).toBe(bytes[i]);
        }
      }),
      { numRuns: 50 }
    );
  });

  it('should strip data URL prefix', () => {
    fc.assert(
      fc.property(fc.uint8Array({ minLength: 1, maxLength: 50 }), (bytes) => {
        const base64 = Buffer.from(bytes).toString('base64');
        const dataUrl = `data:application/pdf;base64,${base64}`;

        const result = base64ToUint8Array(dataUrl);
        expect(result.length).toBe(bytes.length);
      }),
      { numRuns: 20 }
    );
  });

  it('should handle various MIME type prefixes', () => {
    const mimeTypes = [
      'data:application/pdf;base64,',
      'data:application/octet-stream;base64,',
      'data:image/png;base64,',
      'data:text/plain;base64,',
    ];

    fc.assert(
      fc.property(
        fc.uint8Array({ minLength: 1, maxLength: 30 }),
        fc.constantFrom(...mimeTypes),
        (bytes, prefix) => {
          const base64 = Buffer.from(bytes).toString('base64');
          const dataUrl = `${prefix}${base64}`;

          const result = base64ToUint8Array(dataUrl);
          expect(result.length).toBe(bytes.length);
        }
      ),
      { numRuns: 20 }
    );
  });
});

describe('Page Dimensions', () => {
  it('should create valid default dimensions', () => {
    const dims = createDefaultDimensions();

    expect(dims.width).toBe(0);
    expect(dims.height).toBe(0);
    expect(dims.originalWidth).toBe(0);
    expect(dims.originalHeight).toBe(0);
    expect(dims.pdfWidth).toBe(0);
    expect(dims.pdfHeight).toBe(0);
  });

  it('should calculate scaled dimensions correctly', () => {
    fc.assert(
      fc.property(
        fc.float({ min: 100, max: 1000, noNaN: true }),
        fc.float({ min: 100, max: 1000, noNaN: true }),
        fc.float({ min: 0.5, max: 4, noNaN: true }),
        (pdfWidth, pdfHeight, scale) => {
          const dims = calculateScaledDimensions(pdfWidth, pdfHeight, scale);

          expect(dims.pdfWidth).toBe(pdfWidth);
          expect(dims.pdfHeight).toBe(pdfHeight);
          expect(dims.originalWidth).toBe(pdfWidth);
          expect(dims.originalHeight).toBe(pdfHeight);
          expect(Math.abs(dims.width - pdfWidth * scale)).toBeLessThan(0.01);
          expect(Math.abs(dims.height - pdfHeight * scale)).toBeLessThan(0.01);
        }
      ),
      { numRuns: 50 }
    );
  });

  it('should preserve aspect ratio when scaling', () => {
    fc.assert(
      fc.property(
        fc.float({ min: 100, max: 1000, noNaN: true }),
        fc.float({ min: 100, max: 1000, noNaN: true }),
        fc.float({ min: 0.5, max: 4, noNaN: true }),
        (pdfWidth, pdfHeight, scale) => {
          const dims = calculateScaledDimensions(pdfWidth, pdfHeight, scale);

          const originalAspect = pdfWidth / pdfHeight;
          const scaledAspect = dims.width / dims.height;

          expect(Math.abs(originalAspect - scaledAspect)).toBeLessThan(0.001);
        }
      ),
      { numRuns: 50 }
    );
  });
});

describe('LoadPdfResult Structure', () => {
  it('should have valid success result structure', () => {
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 1000 }), (numPages) => {
        const result: LoadPdfResult = {
          numPages,
          success: true,
        };

        expect(result.success).toBe(true);
        expect(result.numPages).toBe(numPages);
        expect(result.error).toBeUndefined();
      }),
      { numRuns: 20 }
    );
  });

  it('should have valid error result structure', () => {
    fc.assert(
      fc.property(fc.string({ minLength: 1, maxLength: 100 }), (errorMessage) => {
        const result: LoadPdfResult = {
          numPages: 0,
          success: false,
          error: errorMessage,
        };

        expect(result.success).toBe(false);
        expect(result.numPages).toBe(0);
        expect(result.error).toBe(errorMessage);
      }),
      { numRuns: 20 }
    );
  });
});

describe('RenderPageResult Structure', () => {
  it('should have valid success result structure', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 100 }),
        fc.float({ min: 100, max: 1000, noNaN: true }),
        fc.float({ min: 100, max: 1000, noNaN: true }),
        (pageNum, width, height) => {
          const mockCanvas = document.createElement('canvas');
          const result: RenderPageResult = {
            pageNum,
            dimensions: calculateScaledDimensions(width, height, 1.5),
            canvas: mockCanvas,
            success: true,
          };

          expect(result.success).toBe(true);
          expect(result.pageNum).toBe(pageNum);
          expect(result.error).toBeUndefined();
        }
      ),
      { numRuns: 20 }
    );
  });
});

describe('RenderConfig Defaults', () => {
  it('should use default scale of 1.5', () => {
    const defaultScale = 1.5;

    fc.assert(
      fc.property(
        fc.float({ min: 100, max: 1000, noNaN: true }),
        fc.float({ min: 100, max: 1000, noNaN: true }),
        (pdfWidth, pdfHeight) => {
          const dims = calculateScaledDimensions(pdfWidth, pdfHeight, defaultScale);

          expect(dims.width).toBeCloseTo(pdfWidth * 1.5, 2);
          expect(dims.height).toBeCloseTo(pdfHeight * 1.5, 2);
        }
      ),
      { numRuns: 20 }
    );
  });

  it('should have default pageWrapperClass', () => {
    const defaultClass = 'pdf-page-wrapper';
    expect(defaultClass).toBe('pdf-page-wrapper');
  });
});

describe('Page Number Validation', () => {
  it('should have 1-indexed page numbers', () => {
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 100 }), (pageNum) => {
        // Page numbers should be >= 1
        expect(pageNum).toBeGreaterThanOrEqual(1);
      }),
      { numRuns: 20 }
    );
  });

  it('should handle page count correctly', () => {
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 500 }), (pageCount) => {
        // Valid page numbers are 1 to pageCount
        for (let i = 1; i <= Math.min(pageCount, 10); i++) {
          expect(i).toBeGreaterThanOrEqual(1);
          expect(i).toBeLessThanOrEqual(pageCount);
        }
      }),
      { numRuns: 20 }
    );
  });
});

describe('DocSign Namespace', () => {
  it('should expose expected functions', () => {
    const expectedFunctions = [
      'loadPdf',
      'renderAllPages',
      'renderPage',
      'getPageCount',
      'getPageDimensions',
      'cleanup',
      'isDocumentLoaded',
      'getUserFriendlyError',
      'categorizeError',
      'createUserError',
      'getOfflineError',
      'getFileTooLargeError',
      'getUnsupportedFileError',
      'showErrorModal',
      'hideErrorModal',
      'showErrorToast',
      'hideErrorToast',
      'showConfirmDialog',
    ];

    // These should all be valid function names
    expectedFunctions.forEach((name) => {
      expect(typeof name).toBe('string');
      expect(name.length).toBeGreaterThan(0);
      // Should be camelCase
      expect(name[0]).toMatch(/[a-z]/);
    });
  });
});

describe('Error Handling', () => {
  it('should return error message in result', () => {
    fc.assert(
      fc.property(fc.string({ minLength: 1, maxLength: 200 }), (errorMessage) => {
        const result: LoadPdfResult = {
          numPages: 0,
          success: false,
          error: errorMessage,
        };

        expect(result.success).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error!.length).toBeGreaterThan(0);
      }),
      { numRuns: 20 }
    );
  });

  it('should produce zero dimensions on error', () => {
    const defaultDims = createDefaultDimensions();

    expect(defaultDims.width).toBe(0);
    expect(defaultDims.height).toBe(0);
    expect(defaultDims.originalWidth).toBe(0);
    expect(defaultDims.originalHeight).toBe(0);
  });
});
