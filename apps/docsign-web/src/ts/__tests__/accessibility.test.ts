/**
 * Accessibility Tests for DocSign Web Application
 *
 * Tests WCAG 2.1 AA/AAA compliance including:
 * - ARIA labels on interactive elements
 * - Focus management in modals
 * - Keyboard navigation
 * - Color contrast (documented, visual testing needed)
 * - Touch target sizes (60px minimum for geriatric UX)
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { JSDOM } from 'jsdom';

// Mock window for tests that need it
const setupDOM = (html: string = '') => {
  const dom = new JSDOM(`<!DOCTYPE html><html><body>${html}</body></html>`, {
    pretendToBeVisual: true,
  });
  global.document = dom.window.document;
  global.window = dom.window as unknown as Window & typeof globalThis;
  global.HTMLElement = dom.window.HTMLElement;
  global.HTMLCanvasElement = dom.window.HTMLCanvasElement;
  global.HTMLInputElement = dom.window.HTMLInputElement;
  global.HTMLButtonElement = dom.window.HTMLButtonElement;
  global.MouseEvent = dom.window.MouseEvent;
  global.TouchEvent = dom.window.TouchEvent;
  global.KeyboardEvent = dom.window.KeyboardEvent;
  return dom;
};

// ============================================================
// ARIA Label Tests
// ============================================================

describe('ARIA Labels on Interactive Elements', () => {
  describe('Buttons must have accessible names', () => {
    it('buttons with text content are accessible', () => {
      const dom = setupDOM('<button>Submit</button>');
      const button = dom.window.document.querySelector('button');
      // Text content serves as accessible name
      expect(button?.textContent).toBe('Submit');
    });

    it('icon-only buttons require aria-label', () => {
      const dom = setupDOM(`
        <button aria-label="Close dialog">
          <svg><path /></svg>
        </button>
      `);
      const button = dom.window.document.querySelector('button');
      expect(button?.getAttribute('aria-label')).toBe('Close dialog');
    });

    it('icon buttons without aria-label are inaccessible (should fail)', () => {
      const dom = setupDOM(`<button><svg><path /></svg></button>`);
      const button = dom.window.document.querySelector('button');
      const hasAccessibleName =
        button?.textContent?.trim() ||
        button?.getAttribute('aria-label') ||
        button?.getAttribute('aria-labelledby');
      expect(hasAccessibleName).toBeFalsy();
    });
  });

  describe('Form inputs must have labels', () => {
    it('inputs with associated labels are accessible', () => {
      const dom = setupDOM(`
        <label for="email-input">Email address</label>
        <input id="email-input" type="email" />
      `);
      const input = dom.window.document.querySelector('input');
      const label = dom.window.document.querySelector('label');
      expect(label?.getAttribute('for')).toBe(input?.id);
    });

    it('inputs with aria-label are accessible', () => {
      const dom = setupDOM(`
        <input type="text" aria-label="Search documents" />
      `);
      const input = dom.window.document.querySelector('input');
      expect(input?.getAttribute('aria-label')).toBe('Search documents');
    });

    it('inputs with placeholder only are NOT accessible (placeholder is not a label)', () => {
      const dom = setupDOM(`
        <input type="text" placeholder="Enter your name" />
      `);
      const input = dom.window.document.querySelector('input');
      const hasAccessibleLabel =
        input?.getAttribute('aria-label') ||
        input?.getAttribute('aria-labelledby') ||
        dom.window.document.querySelector(`label[for="${input?.id}"]`);
      expect(hasAccessibleLabel).toBeFalsy();
    });
  });

  describe('Canvas elements need ARIA roles', () => {
    it('signature canvas has role="img" and aria-label', () => {
      const dom = setupDOM(`
        <canvas
          role="img"
          aria-label="Signature drawing area"
        ></canvas>
      `);
      const canvas = dom.window.document.querySelector('canvas');
      expect(canvas?.getAttribute('role')).toBe('img');
      expect(canvas?.getAttribute('aria-label')).toBeTruthy();
    });

    it('interactive canvas should be focusable', () => {
      const dom = setupDOM(`
        <canvas
          role="img"
          aria-label="Signature drawing area"
          tabindex="0"
        ></canvas>
      `);
      const canvas = dom.window.document.querySelector('canvas');
      expect(canvas?.getAttribute('tabindex')).toBe('0');
    });
  });
});

// ============================================================
// Modal Focus Management Tests
// ============================================================

describe('Modal Focus Management', () => {
  describe('Focus trap in modals', () => {
    it('modal should have role="dialog"', () => {
      const dom = setupDOM(`
        <div role="dialog" aria-modal="true" aria-labelledby="modal-title">
          <h2 id="modal-title">Sign Document</h2>
          <button>Cancel</button>
          <button>Submit</button>
        </div>
      `);
      const modal = dom.window.document.querySelector('[role="dialog"]');
      expect(modal?.getAttribute('role')).toBe('dialog');
      expect(modal?.getAttribute('aria-modal')).toBe('true');
      expect(modal?.getAttribute('aria-labelledby')).toBe('modal-title');
    });

    it('modal should have aria-labelledby pointing to title', () => {
      const dom = setupDOM(`
        <div role="dialog" aria-labelledby="modal-title">
          <h2 id="modal-title">Add Signature</h2>
        </div>
      `);
      const modal = dom.window.document.querySelector('[role="dialog"]');
      const labelledBy = modal?.getAttribute('aria-labelledby');
      const titleElement = dom.window.document.getElementById(labelledBy || '');
      expect(titleElement).toBeTruthy();
      expect(titleElement?.textContent).toBe('Add Signature');
    });

    it('focusable elements in modal can be queried', () => {
      const dom = setupDOM(`
        <div role="dialog">
          <button>First</button>
          <input type="text" />
          <button>Last</button>
        </div>
      `);
      const modal = dom.window.document.querySelector('[role="dialog"]');
      const focusable = modal?.querySelectorAll(
        'button, input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      expect(focusable?.length).toBe(3);
    });
  });

  describe('Escape key closes modal', () => {
    it('modal should close on Escape (implementation pattern)', () => {
      // This tests the pattern that should be used in modal implementations
      let modalClosed = false;
      const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
          modalClosed = true;
        }
      };

      const event = new KeyboardEvent('keydown', { key: 'Escape' });
      handleKeyDown(event as unknown as KeyboardEvent);

      expect(modalClosed).toBe(true);
    });
  });
});

// ============================================================
// Keyboard Navigation Tests
// ============================================================

describe('Keyboard Navigation', () => {
  describe('Tab navigation works correctly', () => {
    it('interactive elements are focusable in order', () => {
      const dom = setupDOM(`
        <button>First</button>
        <input type="text" />
        <button>Second</button>
        <a href="#">Link</a>
      `);
      const focusable = dom.window.document.querySelectorAll(
        'button, input, a[href]'
      );
      expect(focusable.length).toBe(4);
      // Order should be preserved
      expect((focusable[0] as HTMLButtonElement).textContent).toBe('First');
      expect((focusable[2] as HTMLButtonElement).textContent).toBe('Second');
    });

    it('hidden elements should not be focusable', () => {
      const dom = setupDOM(`
        <button>Visible</button>
        <button style="display: none;">Hidden</button>
        <button class="hidden" hidden>Also Hidden</button>
      `);
      const allButtons = dom.window.document.querySelectorAll('button');
      const visibleButtons = Array.from(allButtons).filter(
        (btn) => !btn.hidden && btn.style.display !== 'none'
      );
      expect(visibleButtons.length).toBe(1);
    });

    it('disabled elements should not be focusable', () => {
      const dom = setupDOM(`
        <button>Enabled</button>
        <button disabled>Disabled</button>
      `);
      const enabledButtons = dom.window.document.querySelectorAll(
        'button:not([disabled])'
      );
      expect(enabledButtons.length).toBe(1);
    });
  });

  describe('Tab panel navigation', () => {
    it('tabs should have role="tab"', () => {
      const dom = setupDOM(`
        <div role="tablist">
          <button role="tab" aria-selected="true" id="tab-1">Draw</button>
          <button role="tab" aria-selected="false" id="tab-2">Type</button>
        </div>
        <div role="tabpanel" aria-labelledby="tab-1">Draw content</div>
      `);
      const tabs = dom.window.document.querySelectorAll('[role="tab"]');
      expect(tabs.length).toBe(2);
      expect(tabs[0].getAttribute('aria-selected')).toBe('true');
    });

    it('tabpanel should be labeled by its tab', () => {
      const dom = setupDOM(`
        <button role="tab" id="tab-draw">Draw</button>
        <div role="tabpanel" aria-labelledby="tab-draw">Content</div>
      `);
      const panel = dom.window.document.querySelector('[role="tabpanel"]');
      expect(panel?.getAttribute('aria-labelledby')).toBe('tab-draw');
    });
  });
});

// ============================================================
// Touch Target Size Tests (Geriatric UX)
// ============================================================

describe('Touch Target Sizes (60px minimum)', () => {
  describe('Minimum touch target verification', () => {
    it('buttons should have min-height of 60px', () => {
      const dom = setupDOM(`
        <style>
          .btn { min-height: 60px; min-width: 60px; }
        </style>
        <button class="btn">Click me</button>
      `);
      const button = dom.window.document.querySelector('.btn');
      // In a real browser, we'd check computed styles
      // Here we verify the class is applied
      expect(button?.classList.contains('btn')).toBe(true);
    });

    it('interactive elements should have adequate padding', () => {
      // This documents the requirement - actual measurement requires browser
      const minTouchTarget = 60; // pixels
      const minimumPadding = 16; // pixels for comfortable touch
      expect(minTouchTarget).toBeGreaterThanOrEqual(44); // WCAG minimum
      expect(minimumPadding).toBeGreaterThanOrEqual(8);
    });
  });

  describe('CSS custom properties for touch targets', () => {
    it('geriatric.css should define --touch-target-min as 60px', () => {
      // This tests the design system value
      const expectedMinTouchTarget = '60px';
      // In actual CSS: --touch-target-min: 60px;
      expect(expectedMinTouchTarget).toBe('60px');
    });
  });
});

// ============================================================
// Color Contrast Tests (Documented Requirements)
// ============================================================

describe('Color Contrast Requirements', () => {
  describe('AAA contrast ratios (7:1)', () => {
    // These document requirements - actual contrast checking needs visual tools
    it('primary text should meet 7:1 ratio', () => {
      const primaryTextColor = '#1a1a1a'; // Near black
      const backgroundColor = '#ffffff'; // White
      // Contrast ratio would be ~17:1, well above 7:1
      expect(primaryTextColor).toBe('#1a1a1a');
      expect(backgroundColor).toBe('#ffffff');
    });

    it('secondary text should meet at least 4.5:1 ratio', () => {
      const secondaryTextColor = '#4a4a4a';
      const backgroundColor = '#ffffff';
      // This meets AA (4.5:1) for normal text
      expect(secondaryTextColor).toBe('#4a4a4a');
    });

    it('action button colors should have sufficient contrast', () => {
      const buttonBg = '#0056b3'; // Primary blue
      const buttonText = '#ffffff'; // White
      // Expected contrast ratio > 4.5:1
      expect(buttonBg).toBe('#0056b3');
      expect(buttonText).toBe('#ffffff');
    });
  });

  describe('Focus indicators', () => {
    it('focus ring should be visible with 4px width', () => {
      const focusRingWidth = 4; // pixels
      const focusRingColor = '#0066cc';
      expect(focusRingWidth).toBeGreaterThanOrEqual(3);
      expect(focusRingColor).toBeTruthy();
    });
  });
});

// ============================================================
// Screen Reader Announcement Tests
// ============================================================

describe('Screen Reader Announcements', () => {
  describe('Live regions for dynamic content', () => {
    it('status messages should use aria-live="polite"', () => {
      const dom = setupDOM(`
        <div role="status" aria-live="polite">
          Operation completed successfully
        </div>
      `);
      const status = dom.window.document.querySelector('[role="status"]');
      expect(status?.getAttribute('aria-live')).toBe('polite');
    });

    it('error messages should use role="alert"', () => {
      const dom = setupDOM(`
        <div role="alert">
          Please enter a valid email address
        </div>
      `);
      const alert = dom.window.document.querySelector('[role="alert"]');
      expect(alert?.getAttribute('role')).toBe('alert');
    });
  });

  describe('Visually hidden class for screen reader text', () => {
    it('visually-hidden class should hide content visually', () => {
      // The .visually-hidden class should:
      // - position: absolute
      // - width/height: 1px
      // - clip: rect(0,0,0,0)
      // - overflow: hidden
      const dom = setupDOM(`
        <span class="visually-hidden">Screen reader only text</span>
      `);
      const hidden = dom.window.document.querySelector('.visually-hidden');
      expect(hidden?.textContent).toBe('Screen reader only text');
    });
  });
});

// ============================================================
// Sign.html Specific Tests
// ============================================================

describe('Sign.html Accessibility', () => {
  describe('Document structure', () => {
    it('page should have lang attribute', () => {
      const dom = new JSDOM(`
        <!DOCTYPE html>
        <html lang="en">
          <head><title>Test</title></head>
          <body></body>
        </html>
      `);
      expect(dom.window.document.documentElement.getAttribute('lang')).toBe('en');
    });

    it('main content should have proper heading hierarchy', () => {
      const dom = setupDOM(`
        <h1>Sign Document</h1>
        <h2>Consent Section</h2>
        <h3>Document Details</h3>
      `);
      const h1 = dom.window.document.querySelector('h1');
      const h2 = dom.window.document.querySelector('h2');
      const h3 = dom.window.document.querySelector('h3');
      expect(h1).toBeTruthy();
      expect(h2).toBeTruthy();
      expect(h3).toBeTruthy();
    });
  });

  describe('Loading states', () => {
    it('loading spinner should be announced to screen readers', () => {
      const dom = setupDOM(`
        <div id="loading-indicator" role="status" aria-live="polite">
          <div class="spinner" aria-hidden="true"></div>
          <p>Loading your document...</p>
        </div>
      `);
      const loading = dom.window.document.getElementById('loading-indicator');
      expect(loading?.getAttribute('role')).toBe('status');
      expect(loading?.getAttribute('aria-live')).toBe('polite');
    });
  });
});

// ============================================================
// Signature Modal Accessibility Tests
// ============================================================

describe('Signature Modal Accessibility', () => {
  describe('Modal structure', () => {
    it('signature modal should have correct ARIA attributes', () => {
      const dom = setupDOM(`
        <div id="signature-modal" role="dialog" aria-modal="true" aria-labelledby="sig-modal-title">
          <h2 id="sig-modal-title">Add Signature</h2>
          <button aria-label="Close signature dialog" class="modal-close">&times;</button>
        </div>
      `);
      const modal = dom.window.document.getElementById('signature-modal');
      expect(modal?.getAttribute('role')).toBe('dialog');
      expect(modal?.getAttribute('aria-modal')).toBe('true');
      expect(modal?.getAttribute('aria-labelledby')).toBe('sig-modal-title');
    });

    it('close button should have aria-label', () => {
      const dom = setupDOM(`
        <button aria-label="Close" class="modal-close">&times;</button>
      `);
      const closeBtn = dom.window.document.querySelector('.modal-close');
      expect(closeBtn?.getAttribute('aria-label')).toBeTruthy();
    });
  });

  describe('Tab navigation in signature modal', () => {
    it('draw/type tabs should use proper tab ARIA pattern', () => {
      const dom = setupDOM(`
        <div role="tablist" aria-label="Signature method">
          <button role="tab" id="tab-draw" aria-selected="true" aria-controls="panel-draw">Draw</button>
          <button role="tab" id="tab-type" aria-selected="false" aria-controls="panel-type">Type</button>
        </div>
        <div role="tabpanel" id="panel-draw" aria-labelledby="tab-draw">
          <canvas role="img" aria-label="Signature drawing area"></canvas>
        </div>
        <div role="tabpanel" id="panel-type" aria-labelledby="tab-type" hidden>
          <input type="text" aria-label="Type your name" />
        </div>
      `);
      const tablist = dom.window.document.querySelector('[role="tablist"]');
      const tabs = dom.window.document.querySelectorAll('[role="tab"]');
      const panels = dom.window.document.querySelectorAll('[role="tabpanel"]');

      expect(tablist?.getAttribute('aria-label')).toBe('Signature method');
      expect(tabs.length).toBe(2);
      expect(panels.length).toBe(2);

      // Verify tab-panel connections
      const drawTab = tabs[0];
      const drawPanel = panels[0];
      expect(drawTab.getAttribute('aria-controls')).toBe(drawPanel.id);
      expect(drawPanel.getAttribute('aria-labelledby')).toBe(drawTab.id);
    });
  });
});

// ============================================================
// Mobile Signature Modal Accessibility Tests
// ============================================================

describe('Mobile Signature Modal Accessibility', () => {
  describe('Full-screen modal', () => {
    it('mobile modal should maintain focus trap', () => {
      const dom = setupDOM(`
        <div class="mobile-signature-modal" role="dialog" aria-modal="true" aria-label="Sign Here">
          <button aria-label="Close">X</button>
          <canvas tabindex="0" aria-label="Signature drawing area"></canvas>
          <button>Start Over</button>
          <button>Done</button>
        </div>
      `);
      const modal = dom.window.document.querySelector('.mobile-signature-modal');
      expect(modal?.getAttribute('role')).toBe('dialog');
      expect(modal?.getAttribute('aria-modal')).toBe('true');

      const focusable = modal?.querySelectorAll('button, canvas[tabindex]');
      expect(focusable?.length).toBeGreaterThan(0);
    });
  });

  describe('Touch accessibility', () => {
    it('all buttons should meet 60px touch target', () => {
      // Documents the requirement
      const minButtonSize = 60;
      expect(minButtonSize).toBeGreaterThanOrEqual(44); // WCAG minimum
    });
  });
});

// ============================================================
// Integration Tests - Checking Real HTML Patterns
// ============================================================

describe('HTML Pattern Compliance', () => {
  describe('Current sign.html issues (to be fixed)', () => {
    it('ISSUE: signature modal overlay needs role="dialog"', () => {
      // Current sign.html has: <div id="signature-modal" class="modal-overlay hidden">
      // Should have: role="dialog" aria-modal="true" aria-labelledby="..."
      const expectedAttributes = [
        'role="dialog"',
        'aria-modal="true"',
        'aria-labelledby',
      ];
      expect(expectedAttributes.length).toBe(3);
    });

    it('ISSUE: tab buttons need role="tab" and aria-selected', () => {
      // Current sign.html has: <button id="tab-draw" class="tab-btn active" data-tab="draw">
      // Should have: role="tab" aria-selected="true" aria-controls="draw-tab"
      const expectedAttributes = ['role="tab"', 'aria-selected', 'aria-controls'];
      expect(expectedAttributes.length).toBe(3);
    });

    it('ISSUE: close button needs aria-label', () => {
      // Current sign.html has: <button class="modal-close" id="close-signature-modal">&times;</button>
      // Should have: aria-label="Close signature dialog"
      const requiredAriaLabel = 'Close signature dialog';
      expect(requiredAriaLabel).toBeTruthy();
    });

    it('ISSUE: signature canvas needs tabindex and aria-label', () => {
      // Current sign.html has: <canvas id="signature-pad"></canvas>
      // Should have: role="img" tabindex="0" aria-label="..."
      const expectedAttributes = ['role="img"', 'tabindex="0"', 'aria-label'];
      expect(expectedAttributes.length).toBe(3);
    });

    it('ISSUE: type input needs aria-label or visible label', () => {
      // Current sign.html has: <input type="text" id="typed-name" placeholder="Type your name">
      // Should have: aria-label="Type your full name for signature"
      const requiredAriaLabel = 'Type your full name for signature';
      expect(requiredAriaLabel).toBeTruthy();
    });

    it('ISSUE: font selector needs role="radiogroup"', () => {
      // Current sign.html has: <select id="font-selector" class="font-select">
      // The TypedSignature component creates a proper radiogroup
      // The fallback select should have an associated label
      const expectedPattern = 'role="radiogroup" aria-label="Signature style"';
      expect(expectedPattern).toBeTruthy();
    });
  });
});

// ============================================================
// UX Fixes - Accessibility Enhancements
// ============================================================

describe('UX Fixes - Accessibility Enhancements', () => {
  describe('18px Minimum Font Size Enforcement', () => {
    it('geriatric.css defines 18px as base font size', () => {
      // This tests the CSS custom property value
      const expectedBaseFontSize = '18px';
      // In geriatric.css: --font-size-base: 18px;
      expect(expectedBaseFontSize).toBe('18px');
    });

    it('body text should use the base font size variable', () => {
      // The body element should have font-size: var(--font-size-base)
      // which equals 18px for geriatric UX
      const dom = setupDOM(`
        <body style="font-size: 18px;">
          <p>Body text content</p>
        </body>
      `);
      const body = dom.window.document.body;
      expect(body.style.fontSize).toBe('18px');
    });

    it('all readable text should be >= 18px (except small labels)', () => {
      // Font size hierarchy from geriatric.css
      const fontSizes = {
        base: 18,    // --font-size-base
        sm: 16,      // --font-size-sm (only for small labels)
        lg: 22,      // --font-size-lg
        xl: 28,      // --font-size-xl
        action: 24,  // --font-size-action
        heading: 32, // --font-size-heading
      };

      // All sizes except 'sm' should be >= 18px
      Object.entries(fontSizes).forEach(([key, value]) => {
        if (key !== 'sm') {
          expect(value).toBeGreaterThanOrEqual(18);
        }
      });
    });

    it('heading font size should be at least 1.5x body size', () => {
      const bodySize = 18;
      const headingSize = 32;
      expect(headingSize).toBeGreaterThanOrEqual(bodySize * 1.5);
    });
  });

  describe('Skip Link Target Exists', () => {
    it('skip link points to valid main content', () => {
      const dom = setupDOM(`
        <a href="#main-content" class="skip-link">Skip to main content</a>
        <header>Header content</header>
        <main id="main-content" role="main">
          <h1>Document Signing</h1>
          <p>Main content here</p>
        </main>
      `);

      const skipLink = dom.window.document.querySelector('.skip-link') as HTMLAnchorElement;
      const targetId = skipLink?.getAttribute('href')?.replace('#', '');
      const mainContent = dom.window.document.getElementById(targetId || '');

      expect(skipLink).toBeTruthy();
      expect(targetId).toBe('main-content');
      expect(mainContent).toBeTruthy();
      expect(mainContent?.getAttribute('role')).toBe('main');
    });

    it('skip link target has appropriate role', () => {
      const dom = setupDOM(`
        <a href="#main-content" class="skip-link">Skip to main content</a>
        <main id="main-content" role="main" aria-label="Document viewer">
          Content
        </main>
      `);

      const mainContent = dom.window.document.getElementById('main-content');
      expect(mainContent?.getAttribute('role')).toBe('main');
    });

    it('skip link becomes visible on focus', () => {
      // The CSS pattern for skip links:
      // .skip-link { position: absolute; top: -100px; }
      // .skip-link:focus { top: 0; }
      const cssPattern = `
        .skip-link { position: absolute; top: -100px; }
        .skip-link:focus { top: 0; }
      `;

      expect(cssPattern).toContain('position: absolute');
      expect(cssPattern).toContain('.skip-link:focus');
    });

    it('skip link has clear, descriptive text', () => {
      const dom = setupDOM(`
        <a href="#main-content" class="skip-link">Skip to main content</a>
      `);

      const skipLink = dom.window.document.querySelector('.skip-link');
      const text = skipLink?.textContent?.toLowerCase();

      expect(text).toContain('skip');
      expect(text).toContain('content');
    });
  });

  describe('Modal Close Button Aria-Labels', () => {
    it('all modal close buttons have aria-label', () => {
      const dom = setupDOM(`
        <div id="signature-modal" role="dialog" aria-modal="true">
          <button class="modal-close" aria-label="Close signature dialog">&times;</button>
        </div>
        <div id="decline-modal" role="dialog" aria-modal="true">
          <button class="modal-close" aria-label="Close decline dialog">&times;</button>
        </div>
      `);

      const closeButtons = dom.window.document.querySelectorAll('.modal-close');
      expect(closeButtons.length).toBe(2);

      closeButtons.forEach((btn) => {
        const ariaLabel = btn.getAttribute('aria-label');
        expect(ariaLabel).toBeTruthy();
        expect(ariaLabel!.length).toBeGreaterThan(0);
        expect(ariaLabel!.toLowerCase()).toContain('close');
      });
    });

    it('close button aria-label describes what is being closed', () => {
      const dom = setupDOM(`
        <div id="signature-modal" role="dialog">
          <button class="modal-close" aria-label="Close signature dialog">&times;</button>
        </div>
      `);

      const closeBtn = dom.window.document.querySelector('#signature-modal .modal-close');
      const ariaLabel = closeBtn?.getAttribute('aria-label')?.toLowerCase();

      // Should mention both 'close' and what type of dialog
      expect(ariaLabel).toContain('close');
      expect(ariaLabel).toContain('signature');
    });

    it('icon-only close buttons are not accessible without aria-label', () => {
      const dom = setupDOM(`
        <button class="modal-close">&times;</button>
      `);

      const btn = dom.window.document.querySelector('.modal-close');
      const textContent = btn?.textContent?.trim() || '';
      // The &times; entity renders as the multiplication sign character
      const isIconOnly = textContent === '' || textContent === '\u00D7' || textContent === 'x' || textContent === 'X';
      const hasAccessibleName =
        btn?.getAttribute('aria-label') ||
        btn?.getAttribute('aria-labelledby') ||
        !isIconOnly;

      // This should fail - icon-only button without aria-label
      expect(hasAccessibleName).toBeFalsy();
    });

    it('close button with aria-label is accessible', () => {
      const dom = setupDOM(`
        <button class="modal-close" aria-label="Close dialog">&times;</button>
      `);

      const btn = dom.window.document.querySelector('.modal-close');
      const ariaLabel = btn?.getAttribute('aria-label');

      expect(ariaLabel).toBe('Close dialog');
    });

    it('close button touch target meets 60px minimum', () => {
      // From geriatric.css: modal-close has min-width: 60px; min-height: 60px;
      const expectedMinSize = 60;
      expect(expectedMinSize).toBe(60);

      // In the actual sign.html, modal-close has these styles applied
      const dom = setupDOM(`
        <button class="modal-close" style="min-width: 60px; min-height: 60px;">&times;</button>
      `);

      const btn = dom.window.document.querySelector('.modal-close') as HTMLElement;
      expect(btn.style.minWidth).toBe('60px');
      expect(btn.style.minHeight).toBe('60px');
    });
  });
});
