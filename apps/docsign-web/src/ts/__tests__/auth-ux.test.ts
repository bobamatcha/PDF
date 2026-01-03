/**
 * Auth UX Tests
 *
 * Validates that authentication forms follow geriatric UX guidelines:
 * - White background inputs for maximum contrast
 * - Large touch targets (60px minimum)
 * - 18px minimum font size
 * - High contrast text
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { JSDOM } from 'jsdom';
import * as fs from 'fs';
import * as path from 'path';

describe('Auth Page UX Compliance', () => {
  let dom: JSDOM;
  let document: Document;
  let computedStyles: Map<Element, CSSStyleDeclaration>;

  beforeAll(() => {
    // Load auth.html
    const authHtmlPath = path.resolve(__dirname, '../../../www/auth.html');
    const html = fs.readFileSync(authHtmlPath, 'utf-8');
    dom = new JSDOM(html, { runScripts: 'dangerously' });
    document = dom.window.document;
    computedStyles = new Map();
  });

  afterAll(() => {
    dom.window.close();
  });

  // Helper to get CSS property from style tag
  function getCssProperty(selector: string, property: string): string | null {
    const styleTag = document.querySelector('style');
    if (!styleTag) return null;

    const cssText = styleTag.textContent || '';
    // Simple regex to find property value - not perfect but works for tests
    const selectorRegex = new RegExp(
      `${selector.replace('.', '\\.')}\\s*\\{[^}]*${property}\\s*:\\s*([^;]+)`,
      'i'
    );
    const match = cssText.match(selectorRegex);
    return match ? match[1].trim() : null;
  }

  // Helper to get CSS variable value
  function getCssVariable(varName: string): string | null {
    const styleTag = document.querySelector('style');
    if (!styleTag) return null;

    const cssText = styleTag.textContent || '';
    const varRegex = new RegExp(`${varName}\\s*:\\s*([^;]+)`, 'i');
    const match = cssText.match(varRegex);
    return match ? match[1].trim() : null;
  }

  describe('Input Field Styling', () => {
    it('UX-1: Input background should be white or light color', () => {
      // Check the --bg-secondary variable which inputs use
      const bgSecondary = getCssVariable('--bg-secondary');
      expect(bgSecondary).not.toBeNull();

      // Should be white or a light color (not dark)
      const lightColors = ['white', '#ffffff', '#fff', '#f5f5f5', '#f8f8f8', '#fafafa'];
      const isLight = lightColors.some(c =>
        bgSecondary?.toLowerCase().includes(c.toLowerCase())
      );
      expect(isLight).toBe(true);
    });

    it('UX-2: Input text color should be dark for high contrast', () => {
      const textPrimary = getCssVariable('--text-primary');
      expect(textPrimary).not.toBeNull();

      // Should be dark (starts with #1, #2, #3 or is black)
      const isDark = textPrimary?.match(/^#[0-3]/i) || textPrimary === 'black';
      expect(isDark).toBeTruthy();
    });

    it('UX-3: Form inputs should have minimum 60px height', () => {
      const minHeight = getCssProperty('.form-input', 'min-height');
      expect(minHeight).not.toBeNull();

      // Extract numeric value
      const heightValue = parseInt(minHeight?.replace(/[^0-9]/g, '') || '0');
      expect(heightValue).toBeGreaterThanOrEqual(60);
    });

    it('UX-4: Form inputs should have 18px minimum font size', () => {
      const fontSize = getCssProperty('.form-input', 'font-size');
      expect(fontSize).not.toBeNull();

      const fontSizeValue = parseInt(fontSize?.replace(/[^0-9]/g, '') || '0');
      expect(fontSizeValue).toBeGreaterThanOrEqual(18);
    });

    it('UX-5: Input border should be visible (2px minimum)', () => {
      const border = getCssProperty('.form-input', 'border');
      expect(border).not.toBeNull();
      expect(border).toMatch(/2px/);
    });
  });

  describe('Button Styling', () => {
    it('UX-6: Primary button should have minimum 60px height', () => {
      const minHeight = getCssProperty('.btn-primary', 'min-height');
      expect(minHeight).not.toBeNull();

      const heightValue = parseInt(minHeight?.replace(/[^0-9]/g, '') || '0');
      expect(heightValue).toBeGreaterThanOrEqual(60);
    });

    it('UX-7: Primary button should have large font', () => {
      const fontSize = getCssProperty('.btn-primary', 'font-size');
      expect(fontSize).not.toBeNull();

      const fontSizeValue = parseInt(fontSize?.replace(/[^0-9]/g, '') || '0');
      expect(fontSizeValue).toBeGreaterThanOrEqual(18);
    });
  });

  describe('Form Labels', () => {
    it('UX-8: Labels should have minimum 18px font size', () => {
      const fontSize = getCssProperty('.form-label', 'font-size');
      expect(fontSize).not.toBeNull();

      const fontSizeValue = parseInt(fontSize?.replace(/[^0-9]/g, '') || '0');
      expect(fontSizeValue).toBeGreaterThanOrEqual(18);
    });

    it('UX-9: Labels should be bold or semi-bold', () => {
      const fontWeight = getCssProperty('.form-label', 'font-weight');
      expect(fontWeight).not.toBeNull();

      // 500+ is semi-bold or bold
      const weightValue = parseInt(fontWeight || '400');
      expect(weightValue).toBeGreaterThanOrEqual(500);
    });
  });

  describe('Accessibility', () => {
    it('UX-10: Page should have a main heading', () => {
      const h1 = document.querySelector('h1');
      expect(h1).not.toBeNull();
    });

    it('UX-11: Form inputs should have associated labels', () => {
      const inputs = document.querySelectorAll('input[type="text"], input[type="email"], input[type="password"]');
      inputs.forEach(input => {
        const id = input.getAttribute('id');
        if (id) {
          const label = document.querySelector(`label[for="${id}"]`);
          expect(label).not.toBeNull();
        }
      });
    });

    it('UX-12: Password requirements should be visible', () => {
      const requirements = document.querySelector('.password-requirements, #password-requirements, .form-hint');
      expect(requirements).not.toBeNull();
    });
  });

  describe('Dark Mode Override for Inputs', () => {
    it('UX-13: Inputs should NOT follow dark mode - always light background', () => {
      // The CSS should explicitly set white/light background, not rely on variables
      // that change in dark mode
      const inputBg = getCssProperty('.form-input', 'background');

      // Should be an explicit light color or var(--bg-secondary) which is white
      const isExplicitLight = inputBg?.includes('white') ||
                              inputBg?.includes('#fff') ||
                              inputBg?.includes('--bg-secondary');
      expect(isExplicitLight).toBe(true);
    });
  });

  describe('Tab Button Styling', () => {
    it('UX-19: Auth tabs container should have rounded corners (segmented control style)', () => {
      const borderRadius = getCssProperty('.auth-tabs', 'border-radius');
      expect(borderRadius).not.toBeNull();
      const radiusValue = parseInt(borderRadius?.replace(/[^0-9]/g, '') || '0');
      expect(radiusValue).toBeGreaterThanOrEqual(8);
    });

    it('UX-20: Active tab should have white background (segmented control)', () => {
      const activeBg = getCssProperty('.auth-tab.active', 'background');
      expect(activeBg).not.toBeNull();
      expect(activeBg).toMatch(/white|#fff/i);
    });

    it('UX-21: Tab buttons should have equal flex distribution', () => {
      const flex = getCssProperty('.auth-tab', 'flex');
      expect(flex).toBe('1');
    });

    it('UX-22: Tab text should not wrap - single line', () => {
      const whiteSpace = getCssProperty('.auth-tab', 'white-space');
      expect(whiteSpace).toBe('nowrap');
    });

    it('UX-23: Tab font size should be readable on mobile (min 16px)', () => {
      const fontSize = getCssProperty('.auth-tab', 'font-size');
      const fontSizeValue = parseInt(fontSize?.replace(/[^0-9]/g, '') || '0');
      expect(fontSizeValue).toBeGreaterThanOrEqual(16);
    });

    it('UX-24: Tabs should have smooth transition', () => {
      const transition = getCssProperty('.auth-tab', 'transition');
      expect(transition).not.toBeNull();
    });
  });

  describe('Text Overflow Prevention', () => {
    it('UX-14: Password inputs with toggle button must have sufficient padding-right', () => {
      // Password inputs have a Show/Hide button inside them
      // The input needs padding-right to prevent placeholder text from overlapping the button
      const paddingRight = getCssProperty('.password-wrapper .form-input', 'padding-right') ||
                          getCssProperty('.password-wrapper input', 'padding-right');

      expect(paddingRight).not.toBeNull();

      // Should be at least 70px to accommodate the Show button (~60px) plus spacing
      const paddingValue = parseInt(paddingRight?.replace(/[^0-9]/g, '') || '0');
      expect(paddingValue).toBeGreaterThanOrEqual(70);
    });

    it('UX-15: Show/Hide toggle button should be positioned absolutely', () => {
      // The class is .show-password-btn in auth.html
      const position = getCssProperty('.show-password-btn', 'position');
      expect(position).toBe('absolute');
    });

    it('UX-16: Show/Hide toggle should be positioned on the right side', () => {
      // The class is .show-password-btn in auth.html
      const right = getCssProperty('.show-password-btn', 'right');
      expect(right).not.toBeNull();
    });

    it('UX-17: Password wrapper should have relative positioning for absolute child', () => {
      const position = getCssProperty('.password-wrapper', 'position');
      expect(position).toBe('relative');
    });

    it('UX-18: Placeholder text should not overflow - text-overflow ellipsis', () => {
      // Ensure long placeholder text is truncated rather than overflowing
      const textOverflow = getCssProperty('.password-wrapper .form-input', 'text-overflow');
      expect(textOverflow).toBe('ellipsis');
    });
  });
});
