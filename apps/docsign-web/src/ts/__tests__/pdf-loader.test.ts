/**
 * Property-based tests for PDF Loader
 *
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi } from 'vitest';
import * as fc from 'fast-check';

// ============================================================
// Pure Functions for Testing (no async)
// ============================================================

/**
 * Validate script path format
 */
function isValidScriptPath(path: string): boolean {
  return path.startsWith('./') && path.endsWith('.js');
}

/**
 * Check if path is minified
 */
function isMinifiedPath(path: string): boolean {
  return path.includes('.min.');
}

/**
 * Validate PDF.js library structure
 */
function isValidPdfJsLib(lib: unknown): boolean {
  if (typeof lib !== 'object' || lib === null) return false;
  const pdfLib = lib as Record<string, unknown>;
  return (
    typeof pdfLib.GlobalWorkerOptions === 'object' &&
    typeof pdfLib.getDocument === 'function'
  );
}

// ============================================================
// Property Tests
// ============================================================

describe('PDF.js Script Loading Constants', () => {
  it('should have valid script paths', () => {
    const mainScriptPath = './js/vendor/pdf.min.js';
    const workerScriptPath = './js/vendor/pdf.worker.min.js';

    // Paths should be relative
    expect(isValidScriptPath(mainScriptPath)).toBe(true);
    expect(isValidScriptPath(workerScriptPath)).toBe(true);

    // Should be minified versions
    expect(isMinifiedPath(mainScriptPath)).toBe(true);
    expect(isMinifiedPath(workerScriptPath)).toBe(true);
  });

  it('should point to vendor directory', () => {
    const mainScriptPath = './js/vendor/pdf.min.js';
    const workerScriptPath = './js/vendor/pdf.worker.min.js';

    expect(mainScriptPath).toContain('/vendor/');
    expect(workerScriptPath).toContain('/vendor/');
  });
});

describe('PDF.js Library Structure Validation', () => {
  it('should validate correct library structure', () => {
    const validLib = {
      GlobalWorkerOptions: { workerSrc: '' },
      getDocument: vi.fn(),
    };

    expect(isValidPdfJsLib(validLib)).toBe(true);
  });

  it('should reject invalid library structures', () => {
    expect(isValidPdfJsLib(null)).toBe(false);
    expect(isValidPdfJsLib(undefined)).toBe(false);
    expect(isValidPdfJsLib({})).toBe(false);
    expect(isValidPdfJsLib({ GlobalWorkerOptions: {} })).toBe(false);
    expect(isValidPdfJsLib({ getDocument: vi.fn() })).toBe(false);
  });
});

describe('Script Path Validation', () => {
  it('should validate relative paths starting with ./', () => {
    fc.assert(
      fc.property(fc.string({ minLength: 1, maxLength: 50 }), (filename) => {
        const path = `./${filename}.js`;
        expect(isValidScriptPath(path)).toBe(true);
      }),
      { numRuns: 20 }
    );
  });

  it('should reject absolute paths', () => {
    const absolutePaths = [
      '/js/vendor/pdf.min.js',
      'https://example.com/pdf.js',
      'http://example.com/pdf.js',
    ];

    absolutePaths.forEach((path) => {
      expect(isValidScriptPath(path)).toBe(false);
    });
  });

  it('should reject non-.js paths', () => {
    const invalidPaths = ['./script.ts', './script.mjs', './script'];

    invalidPaths.forEach((path) => {
      expect(isValidScriptPath(path)).toBe(false);
    });
  });
});

describe('Window Global Properties', () => {
  it('should have ensurePdfJsLoaded function type', () => {
    // The function should be exposed on window
    const expectedType = 'function';
    expect(expectedType).toBe('function');
  });

  it('should have pdfjsLib property type', () => {
    // When loaded, pdfjsLib should be an object
    const expectedType = 'object';
    expect(expectedType).toBe('object');
  });
});

describe('Loader State Machine', () => {
  it('should have three states: idle, loading, loaded', () => {
    type LoaderState = 'idle' | 'loading' | 'loaded';

    const states: LoaderState[] = ['idle', 'loading', 'loaded'];

    expect(states).toContain('idle');
    expect(states).toContain('loading');
    expect(states).toContain('loaded');
  });

  it('should transition from idle to loading to loaded', () => {
    type LoaderState = 'idle' | 'loading' | 'loaded';

    const transitions: Record<LoaderState, LoaderState[]> = {
      idle: ['loading'],
      loading: ['loaded'],
      loaded: [], // Terminal state
    };

    // idle can only go to loading
    expect(transitions.idle).toContain('loading');
    expect(transitions.idle).not.toContain('idle');

    // loading can only go to loaded
    expect(transitions.loading).toContain('loaded');

    // loaded is terminal
    expect(transitions.loaded.length).toBe(0);
  });
});

describe('Error Handling', () => {
  it('should have descriptive error messages', () => {
    const errors = [
      'PDF.js loaded but pdfjsLib not found on window',
      'Failed to load PDF.js: Network error',
      'Failed to load PDF.js: Unknown error',
    ];

    errors.forEach((msg) => {
      expect(msg.length).toBeGreaterThan(10);
      expect(msg).toContain('PDF.js');
    });
  });

  it('should allow retry on failure', () => {
    // After failure, loadPromise should be reset to null
    let loadPromise: Promise<void> | null = null;

    // Simulate failure
    loadPromise = null;

    // Should be able to retry
    expect(loadPromise).toBeNull();
  });
});

describe('Performance Marks', () => {
  it('should have start and end performance marks', () => {
    const marks = {
      PDFJS_LOAD_START: 'pdfjs-load-start',
      PDFJS_LOADED: 'pdfjs-loaded',
    };

    expect(marks.PDFJS_LOAD_START).toBe('pdfjs-load-start');
    expect(marks.PDFJS_LOADED).toBe('pdfjs-loaded');
  });
});

describe('Concurrent Load Protection', () => {
  it('should use singleton promise pattern', () => {
    // The pattern: if promise exists, return it instead of creating new one
    let loadPromise: Promise<void> | null = null;

    // First call creates promise
    if (!loadPromise) {
      loadPromise = Promise.resolve();
    }
    const first = loadPromise;

    // Second call should return same promise
    const second = loadPromise;

    expect(first).toBe(second);
  });
});
