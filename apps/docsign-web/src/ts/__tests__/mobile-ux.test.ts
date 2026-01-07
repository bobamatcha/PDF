/**
 * Mobile & Elder UX Tests
 *
 * Comprehensive tests for mobile friendliness and geriatric UX:
 * - Touch targets meet accessibility minimums (44x44px)
 * - Text fits within buttons without overflow
 * - Readable font sizes for elderly users (16px minimum)
 * - Clean display across viewport sizes
 * - Easy navigation between forms
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { JSDOM } from 'jsdom';
import * as fs from 'fs';
import * as path from 'path';

describe('Mobile & Elder UX Compliance', () => {
  let dom: JSDOM;
  let document: Document;

  beforeAll(() => {
    const authHtmlPath = path.resolve(__dirname, '../../../www/auth.html');
    const html = fs.readFileSync(authHtmlPath, 'utf-8');
    dom = new JSDOM(html, { runScripts: 'dangerously' });
    document = dom.window.document;
  });

  afterAll(() => {
    dom.window.close();
  });

  // Helper to extract CSS property from embedded style tags
  function getCssProperty(selector: string, property: string): string | null {
    const styleTag = document.querySelector('style');
    if (!styleTag) return null;

    const cssText = styleTag.textContent || '';

    // Strategy 1: Direct selector match (handles .form-input { ... })
    const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const directRegex = new RegExp(
      `${escapedSelector}[^{]*\\{[^}]*${property}\\s*:\\s*([^;!}]+)`,
      'im'
    );
    let match = cssText.match(directRegex);
    if (match) return match[1].trim();

    // Strategy 2: Multi-selector match (handles .a,\n.b,\n.c { ... })
    // Find any rule block that contains our selector
    const ruleBlocks = cssText.match(/[^{}]+\{[^}]+\}/g) || [];
    for (const block of ruleBlocks) {
      const [selectorPart, ...rest] = block.split('{');
      const propsPart = rest.join('{');

      // Check if our selector is in the selector list
      if (selectorPart.includes(selector.replace(/\\/g, ''))) {
        const propRegex = new RegExp(`${property}\\s*:\\s*([^;!}]+)`, 'i');
        const propMatch = propsPart.match(propRegex);
        if (propMatch) return propMatch[1].trim();
      }
    }

    return null;
  }

  // Helper to check if min-height meets accessibility requirements
  function getMinHeight(selector: string): number {
    const minHeight = getCssProperty(selector, 'min-height');
    if (!minHeight) return 0;
    return parseInt(minHeight.replace(/[^0-9]/g, '')) || 0;
  }

  // Helper to get font size in pixels
  function getFontSize(selector: string): number {
    const fontSize = getCssProperty(selector, 'font-size');
    if (!fontSize) return 0;
    return parseInt(fontSize.replace(/[^0-9]/g, '')) || 0;
  }

  describe('Touch Target Accessibility (WCAG 2.5.5)', () => {
    it('MOB-1: Primary buttons should have minimum 44px touch target', () => {
      const minHeight = getMinHeight('.btn-primary');
      expect(minHeight).toBeGreaterThanOrEqual(44);
    });

    it('MOB-2: Form inputs should have minimum 44px touch target', () => {
      const minHeight = getMinHeight('.form-input');
      expect(minHeight).toBeGreaterThanOrEqual(44);
    });

    it('MOB-3: Tab buttons should have minimum 44px height', () => {
      const minHeight = getMinHeight('.auth-tab');
      expect(minHeight).toBeGreaterThanOrEqual(44);
    });

    it('MOB-4: Show password checkbox should be at least 24px', () => {
      const size = getCssProperty('.show-password-toggle input[type="checkbox"]', 'width');
      const sizeValue = parseInt(size?.replace(/[^0-9]/g, '') || '0');
      expect(sizeValue).toBeGreaterThanOrEqual(24);
    });
  });

  describe('Elder-Friendly Font Sizes', () => {
    it('MOB-5: Body font size should be at least 18px', () => {
      const fontSize = getCssProperty('body', 'font-size');
      const fontSizeValue = parseInt(fontSize?.replace(/[^0-9]/g, '') || '0');
      expect(fontSizeValue).toBeGreaterThanOrEqual(18);
    });

    it('MOB-6: Form labels should be at least 18px', () => {
      const fontSize = getFontSize('.form-label');
      expect(fontSize).toBeGreaterThanOrEqual(18);
    });

    it('MOB-7: Form inputs should have at least 18px font', () => {
      const fontSize = getFontSize('.form-input');
      expect(fontSize).toBeGreaterThanOrEqual(18);
    });

    it('MOB-8: Primary button text should be at least 18px', () => {
      const fontSize = getFontSize('.btn-primary');
      expect(fontSize).toBeGreaterThanOrEqual(18);
    });

    it('MOB-9: Alert messages should be readable (at least 18px)', () => {
      const fontSize = getFontSize('.alert');
      expect(fontSize).toBeGreaterThanOrEqual(18);
    });

    it('MOB-10: Tab buttons should have readable font (at least 16px)', () => {
      const fontSize = getFontSize('.auth-tab');
      expect(fontSize).toBeGreaterThanOrEqual(16);
    });
  });

  describe('Text Overflow Prevention', () => {
    it('MOB-11: Tab buttons should prevent text wrapping', () => {
      const whiteSpace = getCssProperty('.auth-tab', 'white-space');
      expect(whiteSpace).toBe('nowrap');
    });

    it('MOB-12: Buttons should have adequate padding for text', () => {
      const padding = getCssProperty('.btn', 'padding');
      expect(padding).not.toBeNull();
      // Should have at least 1rem padding
      expect(padding).toMatch(/1rem|16px/);
    });

    it('MOB-13: Container should have mobile-safe padding', () => {
      const padding = getCssProperty('.container', 'padding');
      expect(padding).not.toBeNull();
    });

    it('MOB-14: Auth card should have adequate padding', () => {
      const padding = getCssProperty('.auth-card', 'padding');
      expect(padding).not.toBeNull();
      // Should be at least 1.5rem or equivalent
      expect(padding).toMatch(/2rem|1\.5rem|24px|32px/);
    });
  });

  describe('Form Navigation & Switching', () => {
    it('MOB-15: Auth tabs container should exist', () => {
      const tabs = document.querySelector('.auth-tabs');
      expect(tabs).not.toBeNull();
    });

    it('MOB-16: Both Sign In and Create Account tabs should exist', () => {
      const loginTab = document.querySelector('#loginTab');
      const registerTab = document.querySelector('#registerTab');
      expect(loginTab).not.toBeNull();
      expect(registerTab).not.toBeNull();
    });

    it('MOB-17: Tabs should have proper ARIA roles', () => {
      const tablist = document.querySelector('[role="tablist"]');
      expect(tablist).not.toBeNull();

      const tabs = document.querySelectorAll('[role="tab"]');
      expect(tabs.length).toBeGreaterThanOrEqual(2);
    });

    it('MOB-18: Forms should have proper tabpanel role', () => {
      const tabpanels = document.querySelectorAll('[role="tabpanel"]');
      expect(tabpanels.length).toBeGreaterThanOrEqual(2);
    });

    it('MOB-19: Tab switching should use onclick handlers', () => {
      const tabs = document.querySelectorAll('.auth-tab');
      tabs.forEach(tab => {
        const onclick = tab.getAttribute('onclick');
        expect(onclick).toContain('showForm');
      });
    });

    it('MOB-20: Tabs should have equal distribution (flex: 1)', () => {
      const flex = getCssProperty('.auth-tab', 'flex');
      expect(flex).toBe('1');
    });
  });

  describe('Visual Hierarchy & Contrast', () => {
    it('MOB-21: Active tab should have distinct visual style', () => {
      const activeBg = getCssProperty('.auth-tab.active', 'background');
      expect(activeBg).not.toBeNull();
      expect(activeBg).toMatch(/white|#fff/i);
    });

    it('MOB-22: Primary button should have high contrast background', () => {
      const bg = getCssProperty('.btn-primary', 'background');
      expect(bg).not.toBeNull();
      // Should be a dark/accent color, not light
      expect(bg).toMatch(/--accent-primary|#0056b3|blue/i);
    });

    it('MOB-23: Form inputs should have visible borders', () => {
      const border = getCssProperty('.form-input', 'border');
      expect(border).not.toBeNull();
      expect(border).toMatch(/2px/);
    });

    it('MOB-24: Error states should be clearly visible', () => {
      const errorBorder = getCssProperty('.form-input.error', 'border-color');
      expect(errorBorder).not.toBeNull();
    });
  });

  describe('Mobile Layout', () => {
    it('MOB-25: Container should have max-width for readability', () => {
      const maxWidth = getCssProperty('.container', 'max-width');
      expect(maxWidth).not.toBeNull();
      const widthValue = parseInt(maxWidth?.replace(/[^0-9]/g, '') || '0');
      expect(widthValue).toBeLessThanOrEqual(600);
    });

    it('MOB-26: Container should be centered (margin auto)', () => {
      const margin = getCssProperty('.container', 'margin');
      expect(margin).toMatch(/auto/);
    });

    it('MOB-27: Form groups should have adequate spacing', () => {
      const marginBottom = getCssProperty('.form-group', 'margin-bottom');
      expect(marginBottom).not.toBeNull();
      // Should be at least 1rem
      expect(marginBottom).toMatch(/1\.5rem|1rem|16px|24px/);
    });

    it('MOB-28: Auth card should have rounded corners for modern look', () => {
      const borderRadius = getCssProperty('.auth-card', 'border-radius');
      expect(borderRadius).not.toBeNull();
      const radiusValue = parseInt(borderRadius?.replace(/[^0-9]/g, '') || '0');
      expect(radiusValue).toBeGreaterThanOrEqual(8);
    });
  });

  describe('Accessibility Features', () => {
    it('MOB-29: Page should have main heading (h1)', () => {
      const h1 = document.querySelector('h1');
      expect(h1).not.toBeNull();
    });

    it('MOB-30: All form inputs should have labels', () => {
      const inputs = document.querySelectorAll('input[type="text"], input[type="email"], input[type="password"]');
      inputs.forEach(input => {
        const id = input.getAttribute('id');
        if (id) {
          const label = document.querySelector(`label[for="${id}"]`);
          expect(label).not.toBeNull();
        }
      });
    });

    it('MOB-31: Alert should have ARIA live region', () => {
      const alert = document.querySelector('#alert');
      expect(alert?.getAttribute('aria-live')).toBe('polite');
      expect(alert?.getAttribute('role')).toBe('alert');
    });

    it('MOB-32: Password requirements hint should exist', () => {
      const requirements = document.querySelector('.password-requirements');
      expect(requirements).not.toBeNull();
    });

    it('MOB-33: Focus states should be visible', () => {
      const focusOutline = getCssProperty('.btn:focus', 'outline');
      expect(focusOutline).not.toBeNull();
    });

    it('MOB-34: Keyboard focus should be visible on tabs', () => {
      const focusVisible = getCssProperty('.auth-tab:focus-visible', 'outline');
      expect(focusVisible).not.toBeNull();
    });
  });

  describe('Password Toggle UX', () => {
    it('MOB-35: Password wrapper should use column layout', () => {
      const flexDirection = getCssProperty('.password-wrapper', 'flex-direction');
      expect(flexDirection).toBe('column');
    });

    it('MOB-36: Show password toggle should have clickable label', () => {
      const cursor = getCssProperty('.show-password-toggle', 'cursor');
      expect(cursor).toBe('pointer');
    });

    it('MOB-37: Show password toggle should have adequate spacing', () => {
      const gap = getCssProperty('.password-wrapper', 'gap');
      expect(gap).not.toBeNull();
    });

    it('MOB-38: Checkbox should have proper accent color', () => {
      const accentColor = getCssProperty('.show-password-toggle input[type="checkbox"]', 'accent-color');
      expect(accentColor).not.toBeNull();
    });
  });

  describe('Loading States', () => {
    it('MOB-39: Loading spinner should have animation', () => {
      const animation = getCssProperty('.loading', 'animation');
      expect(animation).toMatch(/spin/);
    });

    it('MOB-40: Disabled button should have visual indication', () => {
      const disabledBg = getCssProperty('.btn-primary:disabled', 'background');
      expect(disabledBg).not.toBeNull();
      // Should be grayed out
      expect(disabledBg).toMatch(/#9ca3af|gray|grey/i);
    });

    it('MOB-41: Disabled button should change cursor', () => {
      const cursor = getCssProperty('.btn-primary:disabled', 'cursor');
      expect(cursor).toBe('not-allowed');
    });
  });

  describe('Form Structure', () => {
    it('MOB-42: Login form should have email and password fields', () => {
      const loginForm = document.querySelector('#loginForm');
      expect(loginForm?.querySelector('#loginEmail')).not.toBeNull();
      expect(loginForm?.querySelector('#loginPassword')).not.toBeNull();
    });

    it('MOB-43: Register form should have name, email, and password fields', () => {
      const registerForm = document.querySelector('#registerForm');
      // Name is now split into First Name, Middle Initial, Last Name
      expect(registerForm?.querySelector('#registerFirstName')).not.toBeNull();
      expect(registerForm?.querySelector('#registerLastName')).not.toBeNull();
      expect(registerForm?.querySelector('#registerEmail')).not.toBeNull();
      expect(registerForm?.querySelector('#registerPassword')).not.toBeNull();
    });

    it('MOB-44: Forgot password form should exist', () => {
      const forgotForm = document.querySelector('#forgotForm');
      expect(forgotForm).not.toBeNull();
    });

    it('MOB-45: Forgot password link should exist in login form', () => {
      const forgotLink = document.querySelector('.forgot-password-link button, .forgot-password-link a');
      expect(forgotLink).not.toBeNull();
    });
  });

  describe('Responsive Design Indicators', () => {
    it('MOB-46: Viewport meta tag should be present', () => {
      const viewport = document.querySelector('meta[name="viewport"]');
      expect(viewport).not.toBeNull();
      expect(viewport?.getAttribute('content')).toContain('width=device-width');
    });

    it('MOB-47: Page should use box-sizing border-box', () => {
      const boxSizing = getCssProperty('*', 'box-sizing');
      expect(boxSizing).toBe('border-box');
    });

    it('MOB-48: Body should have minimum height for full-page layout', () => {
      const minHeight = getCssProperty('body', 'min-height');
      expect(minHeight).toBe('100vh');
    });
  });
});
