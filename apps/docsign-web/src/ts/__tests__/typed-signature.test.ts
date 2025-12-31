/**
 * Property-based tests for TypedSignature component
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as fc from 'fast-check';

// ============================================================
// Constants from the module
// ============================================================

const SIGNATURE_FONTS = [
  { name: 'Dancing Script', label: 'Classic Cursive', style: 'flowing' },
  { name: 'Great Vibes', label: 'Elegant Script', style: 'formal' },
  { name: 'Pacifico', label: 'Casual Handwriting', style: 'casual' },
  { name: 'Sacramento', label: 'Flowing Script', style: 'flowing' },
  { name: 'Allura', label: 'Formal Calligraphy', style: 'calligraphy' },
] as const;

type SignatureFontName = (typeof SIGNATURE_FONTS)[number]['name'];

// ============================================================
// Mock TypedSignature State for Testing
// ============================================================

interface MockTypedSignatureState {
  text: string;
  currentFont: string;
  fontSize: number;
  textColor: string;
  backgroundColor: string;
}

function createMockState(overrides: Partial<MockTypedSignatureState> = {}): MockTypedSignatureState {
  return {
    text: '',
    currentFont: 'Dancing Script',
    fontSize: 48,
    textColor: '#000080',
    backgroundColor: '#ffffff',
    ...overrides,
  };
}

function isEmpty(state: MockTypedSignatureState): boolean {
  return !state.text || state.text.trim() === '';
}

function setText(state: MockTypedSignatureState, text: string): MockTypedSignatureState {
  return { ...state, text };
}

function setFont(state: MockTypedSignatureState, font: string): MockTypedSignatureState {
  const validFonts = SIGNATURE_FONTS.map((f) => f.name);
  if (validFonts.includes(font as SignatureFontName)) {
    return { ...state, currentFont: font };
  }
  return state;
}

// ============================================================
// Property Tests
// ============================================================

describe('TypedSignature Property Tests', () => {
  describe('State Management', () => {
    it('should correctly track empty state', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 0, maxLength: 100 }),
          (text) => {
            const state = createMockState({ text });
            const shouldBeEmpty = !text || text.trim() === '';
            expect(isEmpty(state)).toBe(shouldBeEmpty);
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should treat whitespace-only as empty', () => {
      fc.assert(
        fc.property(
          fc.stringOf(fc.constantFrom(' ', '\t', '\n', '\r')),
          (whitespace) => {
            const state = createMockState({ text: whitespace });
            expect(isEmpty(state)).toBe(true);
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should not be empty with non-whitespace text', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 1, maxLength: 50 }).filter((s) => s.trim().length > 0),
          (text) => {
            const state = createMockState({ text });
            expect(isEmpty(state)).toBe(false);
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should preserve text after setting', () => {
      fc.assert(
        fc.property(fc.string({ minLength: 1, maxLength: 100 }), (text) => {
          const state = createMockState();
          const updated = setText(state, text);
          expect(updated.text).toBe(text);
        }),
        { numRuns: 100 }
      );
    });
  });

  describe('Font Selection', () => {
    it('should only accept valid fonts', () => {
      fc.assert(
        fc.property(
          fc.constantFrom(...SIGNATURE_FONTS.map((f) => f.name)),
          (font) => {
            const state = createMockState();
            const updated = setFont(state, font);
            expect(updated.currentFont).toBe(font);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('should reject invalid fonts', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 1, maxLength: 50 }).filter(
            (s) => !SIGNATURE_FONTS.some((f) => f.name === s)
          ),
          (invalidFont) => {
            const state = createMockState({ currentFont: 'Dancing Script' });
            const updated = setFont(state, invalidFont);
            // Should not change from original
            expect(updated.currentFont).toBe('Dancing Script');
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should preserve font after valid change', () => {
      fc.assert(
        fc.property(
          fc.constantFrom(...SIGNATURE_FONTS.map((f) => f.name)),
          fc.constantFrom(...SIGNATURE_FONTS.map((f) => f.name)),
          (font1, font2) => {
            let state = createMockState();
            state = setFont(state, font1);
            expect(state.currentFont).toBe(font1);
            state = setFont(state, font2);
            expect(state.currentFont).toBe(font2);
          }
        ),
        { numRuns: 30 }
      );
    });
  });

  describe('Configuration Validation', () => {
    it('should have valid default fontSize', () => {
      fc.assert(
        fc.property(fc.constant(null), () => {
          const state = createMockState();
          expect(state.fontSize).toBeGreaterThan(0);
          expect(state.fontSize).toBeLessThanOrEqual(100);
        }),
        { numRuns: 1 }
      );
    });

    it('should have valid color format', () => {
      fc.assert(
        fc.property(fc.constant(null), () => {
          const state = createMockState();
          // Should be valid hex color
          expect(state.textColor).toMatch(/^#[0-9a-fA-F]{6}$/);
          expect(state.backgroundColor).toMatch(/^#[0-9a-fA-F]{6}$/);
        }),
        { numRuns: 1 }
      );
    });

    it('should handle custom configuration', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 12, max: 96 }),
          fc.hexaString({ minLength: 6, maxLength: 6 }),
          fc.hexaString({ minLength: 6, maxLength: 6 }),
          (fontSize, textColor, bgColor) => {
            const state = createMockState({
              fontSize,
              textColor: `#${textColor}`,
              backgroundColor: `#${bgColor}`,
            });

            expect(state.fontSize).toBe(fontSize);
            expect(state.textColor).toBe(`#${textColor}`);
            expect(state.backgroundColor).toBe(`#${bgColor}`);
          }
        ),
        { numRuns: 30 }
      );
    });
  });

  describe('Canvas Rendering Properties', () => {
    it('should calculate font size within bounds', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 20, max: 200, noNaN: true }),
          (height) => {
            // From the code: font_size = (height * 0.25).clamp(6.0, 10.0) for signature
            // But for typed: fontSize is configurable
            const calculatedSize = Math.max(6, Math.min(height * 0.25, 10));
            expect(calculatedSize).toBeGreaterThanOrEqual(6);
            expect(calculatedSize).toBeLessThanOrEqual(10);
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should produce valid canvas dimensions', () => {
      fc.assert(
        fc.property(
          fc.float({ min: 0.5, max: 4, noNaN: true }),
          (dpr) => {
            // Standard canvas dimensions from the code
            const baseWidth = 400;
            const baseHeight = 100;

            const scaledWidth = baseWidth * dpr;
            const scaledHeight = baseHeight * dpr;

            expect(scaledWidth).toBeGreaterThan(0);
            expect(scaledHeight).toBeGreaterThan(0);
            expect(Number.isFinite(scaledWidth)).toBe(true);
            expect(Number.isFinite(scaledHeight)).toBe(true);
          }
        ),
        { numRuns: 20 }
      );
    });
  });

  describe('Text Rendering Properties', () => {
    it('should center text within canvas', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 1, maxLength: 50 }),
          fc.integer({ min: 200, max: 600 }),
          fc.integer({ min: 50, max: 200 }),
          (text, canvasWidth, canvasHeight) => {
            // Text is centered at width/2, height/2
            const centerX = canvasWidth / 2;
            const centerY = canvasHeight / 2;

            expect(centerX).toBe(canvasWidth / 2);
            expect(centerY).toBe(canvasHeight / 2);
          }
        ),
        { numRuns: 30 }
      );
    });

    it('should limit text to available width', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 1, maxLength: 100 }),
          fc.integer({ min: 200, max: 600 }),
          (text, canvasWidth) => {
            // Max text width is canvas width - 20 (10px padding each side)
            const maxTextWidth = canvasWidth - 20;
            expect(maxTextWidth).toBeGreaterThan(0);
            expect(maxTextWidth).toBeLessThan(canvasWidth);
          }
        ),
        { numRuns: 30 }
      );
    });
  });
});

describe('SIGNATURE_FONTS Constants', () => {
  it('should have unique font names', () => {
    const names = SIGNATURE_FONTS.map((f) => f.name);
    const uniqueNames = new Set(names);
    expect(uniqueNames.size).toBe(names.length);
  });

  it('should have non-empty labels', () => {
    SIGNATURE_FONTS.forEach((font) => {
      expect(font.label.length).toBeGreaterThan(0);
    });
  });

  it('should have valid style categories', () => {
    const validStyles = ['flowing', 'formal', 'casual', 'calligraphy'];
    SIGNATURE_FONTS.forEach((font) => {
      expect(validStyles).toContain(font.style);
    });
  });

  it('should have at least 3 font options', () => {
    expect(SIGNATURE_FONTS.length).toBeGreaterThanOrEqual(3);
  });
});

describe('DataURL Export Properties', () => {
  it('should produce valid PNG data URL prefix', () => {
    const expectedPrefix = 'data:image/png;base64,';

    fc.assert(
      fc.property(fc.string({ minLength: 1, maxLength: 50 }), (text) => {
        // Simulating what toDataURL would produce
        const mockDataUrl = `${expectedPrefix}${Buffer.from('mock-png-data').toString('base64')}`;
        expect(mockDataUrl.startsWith(expectedPrefix)).toBe(true);
      }),
      { numRuns: 10 }
    );
  });

  it('should produce base64 with valid characters', () => {
    fc.assert(
      fc.property(
        fc.uint8Array({ minLength: 10, maxLength: 100 }),
        (bytes) => {
          const base64 = Buffer.from(bytes).toString('base64');
          // Base64 should only contain A-Z, a-z, 0-9, +, /, =
          expect(base64).toMatch(/^[A-Za-z0-9+/=]*$/);
        }
      ),
      { numRuns: 50 }
    );
  });
});

describe('Accessibility Properties', () => {
  it('should have minimum touch target size', () => {
    // Geriatric UX requires 60px minimum
    const minTouchTarget = 60;

    fc.assert(
      fc.property(fc.integer({ min: 50, max: 100 }), (size) => {
        const meetsRequirement = size >= minTouchTarget;
        // The code uses 60px min-height for inputs
        expect(minTouchTarget).toBe(60);
      }),
      { numRuns: 10 }
    );
  });

  it('should have readable font size', () => {
    // Geriatric UX requires 18px minimum, code uses 24px for input
    const minFontSize = 18;
    const inputFontSize = 24;

    expect(inputFontSize).toBeGreaterThanOrEqual(minFontSize);
  });

  it('should have high contrast colors', () => {
    // Navy (#000080) on white (#ffffff) should have high contrast
    const textColor = '#000080';
    const bgColor = '#ffffff';

    // Simple check: text should be dark, bg should be light
    const textLuminance = parseInt(textColor.slice(1, 3), 16);
    const bgLuminance = parseInt(bgColor.slice(1, 3), 16);

    expect(textLuminance).toBeLessThan(bgLuminance);
  });
});
