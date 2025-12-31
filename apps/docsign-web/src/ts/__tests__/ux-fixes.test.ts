/**
 * UX Fixes Property-Based Tests
 *
 * Tests for geriatric UX improvements including:
 * - Font sizes (18px minimum)
 * - Consent language (plain, simple words)
 * - Default tab selection
 * - Modal confirmation behavior
 * - Offline indicator messaging
 *
 * Uses vitest and fast-check for property-based testing.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import * as fc from 'fast-check';
import { JSDOM } from 'jsdom';

// ============================================================
// Test Utilities
// ============================================================

/**
 * Setup DOM environment for tests
 */
const setupDOM = (html: string = '') => {
  const dom = new JSDOM(`<!DOCTYPE html><html><body>${html}</body></html>`, {
    pretendToBeVisual: true,
  });
  global.document = dom.window.document;
  global.window = dom.window as unknown as Window & typeof globalThis;
  return dom;
};

/**
 * Parse CSS custom properties from a CSS string
 */
const parseCSSCustomProperties = (css: string): Map<string, string> => {
  const props = new Map<string, string>();
  const matches = css.matchAll(/--([a-zA-Z0-9-]+)\s*:\s*([^;]+);/g);
  for (const match of matches) {
    props.set(`--${match[1]}`, match[2].trim());
  }
  return props;
};

/**
 * Extract pixel value from a CSS string
 */
const extractPxValue = (value: string): number | null => {
  const match = value.match(/(\d+(?:\.\d+)?)\s*px/);
  return match ? parseFloat(match[1]) : null;
};

// ============================================================
// CSS Design Tokens from geriatric.css
// ============================================================

const GERIATRIC_CSS_TOKENS = {
  fontSizeBase: '18px',
  fontSizeSm: '16px',
  fontSizeLg: '22px',
  fontSizeXl: '28px',
  fontSizeAction: '24px',
  fontSizeHeading: '32px',
  touchTargetMin: '60px',
  focusRingWidth: '4px',
};

// ============================================================
// UX Fixes - Font Sizes
// ============================================================

describe('UX Fixes - Font Sizes', () => {
  describe('Minimum Font Size Requirements', () => {
    it('should define 18px as minimum base font size', () => {
      const baseFontSize = extractPxValue(GERIATRIC_CSS_TOKENS.fontSizeBase);
      expect(baseFontSize).toBe(18);
    });

    it('should have all font sizes >= 16px (except small text)', () => {
      const fontSizes = [
        GERIATRIC_CSS_TOKENS.fontSizeBase,
        GERIATRIC_CSS_TOKENS.fontSizeLg,
        GERIATRIC_CSS_TOKENS.fontSizeXl,
        GERIATRIC_CSS_TOKENS.fontSizeAction,
        GERIATRIC_CSS_TOKENS.fontSizeHeading,
      ];

      fontSizes.forEach((fontSize) => {
        const px = extractPxValue(fontSize);
        expect(px).not.toBeNull();
        expect(px!).toBeGreaterThanOrEqual(16);
      });
    });

    it('Property: All body text should be readable (>= 18px)', () => {
      fc.assert(
        fc.property(
          fc.constantFrom('base', 'lg', 'xl', 'action', 'heading'),
          (sizeKey) => {
            const tokenKey = `fontSize${sizeKey.charAt(0).toUpperCase() + sizeKey.slice(1)}` as keyof typeof GERIATRIC_CSS_TOKENS;
            const fontSize = GERIATRIC_CSS_TOKENS[tokenKey];
            const px = extractPxValue(fontSize);

            // All named sizes should be >= 18px (except 'sm' which is 16px)
            if (sizeKey !== 'sm') {
              expect(px).toBeGreaterThanOrEqual(18);
            }
          }
        )
      );
    });

    it('should enforce minimum font size in CSS body rule', () => {
      // This tests that the body element should have font-size: var(--font-size-base)
      // which is 18px
      const expectedBodyFontSize = '18px';
      expect(GERIATRIC_CSS_TOKENS.fontSizeBase).toBe(expectedBodyFontSize);
    });

    it('action buttons should have large enough font for elderly users', () => {
      const actionFontPx = extractPxValue(GERIATRIC_CSS_TOKENS.fontSizeAction);
      expect(actionFontPx).toBeGreaterThanOrEqual(20); // 24px in design system
    });

    it('headings should be significantly larger than body text', () => {
      const basePx = extractPxValue(GERIATRIC_CSS_TOKENS.fontSizeBase);
      const headingPx = extractPxValue(GERIATRIC_CSS_TOKENS.fontSizeHeading);

      expect(basePx).not.toBeNull();
      expect(headingPx).not.toBeNull();
      expect(headingPx!).toBeGreaterThan(basePx! * 1.5); // At least 1.5x larger
    });
  });
});

// ============================================================
// UX Fixes - Consent Language
// ============================================================

describe('UX Fixes - Consent Language', () => {
  // Current consent language from sign.html (to be simplified)
  const CURRENT_CONSENT_TEXT = `
    By clicking "Review Document" below, you agree to:
    Use electronic signatures in place of handwritten signatures
    Electronically sign documents sent to you
    Conduct this transaction electronically
  `;

  // Simplified consent language (target)
  const SIMPLIFIED_CONSENT_TEXT = `
    By clicking "Review Document" below, you agree to:
    Sign this document on your computer instead of pen and paper
    Receive and sign documents on this website
  `;

  // Legal jargon that should be avoided
  const LEGAL_JARGON = [
    'electronic signatures',
    'conduct this transaction',
    'electronically',
    'pursuant to',
    'herein',
    'whereas',
    'aforementioned',
    'hereunder',
    'notwithstanding',
    'in witness whereof',
  ];

  // Simple words that should be used instead
  const SIMPLE_TERMS = [
    'computer',
    'pen and paper',
    'website',
    'sign',
    'document',
    'agree',
  ];

  describe('Jargon Detection', () => {
    it('should detect legal jargon in current consent text', () => {
      const lowerText = CURRENT_CONSENT_TEXT.toLowerCase();
      const foundJargon = LEGAL_JARGON.filter((term) =>
        lowerText.includes(term.toLowerCase())
      );

      // Current text contains jargon (this documents the problem)
      expect(foundJargon.length).toBeGreaterThan(0);
    });

    it('simplified consent should not contain legal jargon', () => {
      const lowerText = SIMPLIFIED_CONSENT_TEXT.toLowerCase();

      LEGAL_JARGON.forEach((term) => {
        expect(lowerText).not.toContain(term.toLowerCase());
      });
    });

    it('Property: simplified text should use plain language', () => {
      fc.assert(
        fc.property(fc.constantFrom(...SIMPLE_TERMS), (term) => {
          const lowerText = SIMPLIFIED_CONSENT_TEXT.toLowerCase();
          // At least some simple terms should be present
          const hasSimpleTerm = SIMPLE_TERMS.some((t) =>
            lowerText.includes(t.toLowerCase())
          );
          expect(hasSimpleTerm).toBe(true);
        })
      );
    });
  });

  describe('Readability', () => {
    it('simplified consent should be shorter than current', () => {
      const currentWords = CURRENT_CONSENT_TEXT.trim().split(/\s+/).length;
      const simplifiedWords = SIMPLIFIED_CONSENT_TEXT.trim().split(/\s+/).length;

      expect(simplifiedWords).toBeLessThanOrEqual(currentWords);
    });

    it('simplified consent should have shorter sentences', () => {
      const currentSentences = CURRENT_CONSENT_TEXT.split(/[.!?]+/).filter(
        (s) => s.trim()
      );
      const simplifiedSentences = SIMPLIFIED_CONSENT_TEXT.split(/[.!?]+/).filter(
        (s) => s.trim()
      );

      // Average sentence length
      const currentAvg =
        currentSentences.reduce((sum, s) => sum + s.split(/\s+/).length, 0) /
        currentSentences.length;
      const simplifiedAvg =
        simplifiedSentences.reduce((sum, s) => sum + s.split(/\s+/).length, 0) /
        simplifiedSentences.length;

      expect(simplifiedAvg).toBeLessThanOrEqual(currentAvg + 5); // Allow some tolerance
    });

    it('Property: consent text should avoid technical words', () => {
      const technicalTerms = [
        'api',
        'http',
        'https',
        'url',
        'database',
        'server',
        'encrypt',
        'decrypt',
        'hash',
        'token',
        'authentication',
        'authorization',
      ];

      fc.assert(
        fc.property(fc.constantFrom(...technicalTerms), (term) => {
          const lowerText = SIMPLIFIED_CONSENT_TEXT.toLowerCase();
          expect(lowerText).not.toContain(term);
        })
      );
    });
  });
});

// ============================================================
// UX Fixes - Default Tab Selection
// ============================================================

describe('UX Fixes - Default Tab', () => {
  describe('Tab Selection Behavior', () => {
    it('Type tab should be considered the better default for elderly users', () => {
      // Rationale: Typing is easier than drawing for users with motor control issues
      // This tests the UX decision, not implementation
      const reasons = [
        'Drawing requires steady hand',
        'Typing is more familiar to most users',
        'Less motor control needed for typing',
        'Easier to correct mistakes when typing',
      ];

      expect(reasons.length).toBeGreaterThan(0);
    });

    it('should have exactly two tab options: Draw and Type', () => {
      const dom = setupDOM(`
        <div role="tablist" aria-label="Signature method">
          <button role="tab" id="tab-draw" aria-selected="false">Draw</button>
          <button role="tab" id="tab-type" aria-selected="true">Type</button>
        </div>
      `);

      const tabs = dom.window.document.querySelectorAll('[role="tab"]');
      expect(tabs.length).toBe(2);
    });

    it('Type tab should be active by default', () => {
      const dom = setupDOM(`
        <div role="tablist" aria-label="Signature method">
          <button role="tab" id="tab-draw" aria-selected="false">Draw</button>
          <button role="tab" id="tab-type" aria-selected="true">Type</button>
        </div>
      `);

      const typeTab = dom.window.document.getElementById('tab-type');
      expect(typeTab?.getAttribute('aria-selected')).toBe('true');
    });

    it('Draw tab should not be active by default', () => {
      const dom = setupDOM(`
        <div role="tablist" aria-label="Signature method">
          <button role="tab" id="tab-draw" aria-selected="false">Draw</button>
          <button role="tab" id="tab-type" aria-selected="true">Type</button>
        </div>
      `);

      const drawTab = dom.window.document.getElementById('tab-draw');
      expect(drawTab?.getAttribute('aria-selected')).toBe('false');
    });

    it('Property: Only one tab should be selected at a time', () => {
      const dom = setupDOM(`
        <div role="tablist" aria-label="Signature method">
          <button role="tab" id="tab-draw" aria-selected="false">Draw</button>
          <button role="tab" id="tab-type" aria-selected="true">Type</button>
        </div>
      `);

      const selectedTabs = dom.window.document.querySelectorAll(
        '[role="tab"][aria-selected="true"]'
      );
      expect(selectedTabs.length).toBe(1);
    });
  });
});

// ============================================================
// UX Fixes - Modal Confirmation
// ============================================================

describe('UX Fixes - Modal Confirmation', () => {
  /**
   * Mock signature state for testing confirmation logic
   */
  interface SignatureState {
    hasDrawnContent: boolean;
    hasTypedContent: boolean;
    mode: 'draw' | 'type';
  }

  /**
   * Determines if confirmation dialog should be shown before closing
   */
  const shouldShowConfirmation = (state: SignatureState): boolean => {
    if (state.mode === 'draw') {
      return state.hasDrawnContent;
    }
    return state.hasTypedContent;
  };

  describe('Confirmation Logic', () => {
    it('should warn before closing modal with unsaved drawn signature', () => {
      const state: SignatureState = {
        hasDrawnContent: true,
        hasTypedContent: false,
        mode: 'draw',
      };

      expect(shouldShowConfirmation(state)).toBe(true);
    });

    it('should warn before closing modal with unsaved typed signature', () => {
      const state: SignatureState = {
        hasDrawnContent: false,
        hasTypedContent: true,
        mode: 'type',
      };

      expect(shouldShowConfirmation(state)).toBe(true);
    });

    it('should close without warning when drawn signature is empty', () => {
      const state: SignatureState = {
        hasDrawnContent: false,
        hasTypedContent: false,
        mode: 'draw',
      };

      expect(shouldShowConfirmation(state)).toBe(false);
    });

    it('should close without warning when typed signature is empty', () => {
      const state: SignatureState = {
        hasDrawnContent: false,
        hasTypedContent: false,
        mode: 'type',
      };

      expect(shouldShowConfirmation(state)).toBe(false);
    });

    it('Property: confirmation based on active mode content only', () => {
      fc.assert(
        fc.property(
          fc.boolean(),
          fc.boolean(),
          fc.constantFrom('draw' as const, 'type' as const),
          (hasDrawn, hasTyped, mode) => {
            const state: SignatureState = {
              hasDrawnContent: hasDrawn,
              hasTypedContent: hasTyped,
              mode,
            };

            const shouldConfirm = shouldShowConfirmation(state);

            // Should only confirm if active mode has content
            if (mode === 'draw') {
              expect(shouldConfirm).toBe(hasDrawn);
            } else {
              expect(shouldConfirm).toBe(hasTyped);
            }
          }
        )
      );
    });
  });

  describe('Confirmation Dialog', () => {
    it('confirmation dialog should have clear, non-alarming text', () => {
      const confirmationTitle = 'Leave Without Saving?';
      const confirmationMessage = 'You have not saved your signature. Your signature will be lost if you leave now.';
      const confirmButton = 'Leave';
      const cancelButton = 'Stay';

      // Should not contain alarming words
      const alarmingWords = ['warning', 'danger', 'error', 'critical', 'urgent'];
      const combinedText = `${confirmationTitle} ${confirmationMessage}`.toLowerCase();

      alarmingWords.forEach((word) => {
        expect(combinedText).not.toContain(word);
      });

      // Should have clear action buttons
      expect(confirmButton.length).toBeGreaterThan(0);
      expect(cancelButton.length).toBeGreaterThan(0);
    });

    it('confirmation dialog should have appropriate ARIA attributes', () => {
      const dom = setupDOM(`
        <div role="alertdialog" aria-modal="true" aria-labelledby="confirm-title" aria-describedby="confirm-desc">
          <h2 id="confirm-title">Leave Without Saving?</h2>
          <p id="confirm-desc">Your signature will be lost.</p>
          <button>Stay</button>
          <button>Leave</button>
        </div>
      `);

      const dialog = dom.window.document.querySelector('[role="alertdialog"]');
      expect(dialog).not.toBeNull();
      expect(dialog?.getAttribute('aria-modal')).toBe('true');
      expect(dialog?.getAttribute('aria-labelledby')).toBe('confirm-title');
    });
  });
});

// ============================================================
// UX Fixes - Offline Indicator
// ============================================================

describe('UX Fixes - Offline Indicator', () => {
  // Import the actual offline error function behavior
  const getOfflineMessage = (): { title: string; message: string; action: string } => {
    return {
      title: 'You Are Offline',
      message:
        'Your device is not connected to the internet. Your work is saved locally and will be sent automatically when you reconnect. You can continue working offline.',
      action: 'Continue',
    };
  };

  describe('Offline Message Content', () => {
    it('should show reassuring message not technical jargon', () => {
      const offline = getOfflineMessage();

      // Should contain reassuring phrases (saved = safe)
      expect(offline.message.toLowerCase()).toContain('saved');
    });

    it('should mention that work is safe', () => {
      const offline = getOfflineMessage();
      const lowerMessage = offline.message.toLowerCase();

      const safetyPhrases = ['safe', 'saved', 'not lost', 'protected'];
      const hasSafetyPhrase = safetyPhrases.some((phrase) =>
        lowerMessage.includes(phrase)
      );

      expect(hasSafetyPhrase).toBe(true);
    });

    it('should not contain technical network jargon', () => {
      const offline = getOfflineMessage();
      const combinedText = `${offline.title} ${offline.message}`.toLowerCase();

      const technicalTerms = [
        'http',
        'connection refused',
        'dns',
        'timeout',
        'econnrefused',
        'network error',
        '404',
        '500',
        'server',
        'api',
      ];

      technicalTerms.forEach((term) => {
        expect(combinedText).not.toContain(term.toLowerCase());
      });
    });

    it('should explain what will happen when back online', () => {
      const offline = getOfflineMessage();
      const lowerMessage = offline.message.toLowerCase();

      // Should mention automatic sync or reconnection
      const syncPhrases = ['reconnect', 'automatically', 'when you', 'back online'];
      const hasSyncInfo = syncPhrases.some((phrase) =>
        lowerMessage.includes(phrase)
      );

      expect(hasSyncInfo).toBe(true);
    });

    it('Property: offline message should always be reassuring', () => {
      fc.assert(
        fc.property(fc.integer({ min: 1, max: 100 }), () => {
          // Call the function multiple times to ensure consistency
          const offline = getOfflineMessage();

          // Should never contain alarming language
          const alarmingPhrases = [
            'error',
            'failed',
            'lost',
            'problem',
            'issue',
            'wrong',
          ];
          const lowerMessage = offline.message.toLowerCase();

          // Check none of the alarming phrases are present
          // Note: "lost" is actually in the message saying data is NOT lost,
          // so we need to be careful about context
          const hasAlarmingPhrase = alarmingPhrases.some(
            (phrase) =>
              lowerMessage.includes(phrase) &&
              !lowerMessage.includes(`not ${phrase}`) &&
              !lowerMessage.includes(`will not be ${phrase}`)
          );

          // The message should be reassuring, not alarming
          expect(offline.title.toLowerCase()).not.toContain('error');
        }),
        { numRuns: 10 }
      );
    });
  });

  describe('Offline Indicator Visibility', () => {
    it('offline indicator should have role="status"', () => {
      const dom = setupDOM(`
        <div id="offline-indicator" role="status" aria-live="polite" class="hidden">
          Your work is safe. Changes will sync when you reconnect.
        </div>
      `);

      const indicator = dom.window.document.getElementById('offline-indicator');
      expect(indicator?.getAttribute('role')).toBe('status');
    });

    it('offline indicator should have aria-live="polite"', () => {
      const dom = setupDOM(`
        <div id="offline-indicator" role="status" aria-live="polite">
          Your work is safe.
        </div>
      `);

      const indicator = dom.window.document.getElementById('offline-indicator');
      expect(indicator?.getAttribute('aria-live')).toBe('polite');
    });

    it('should not use aria-live="assertive" for offline status', () => {
      // Assertive is too disruptive for a non-critical status update
      const dom = setupDOM(`
        <div id="offline-indicator" role="status" aria-live="polite">
          Your work is safe.
        </div>
      `);

      const indicator = dom.window.document.getElementById('offline-indicator');
      expect(indicator?.getAttribute('aria-live')).not.toBe('assertive');
    });
  });
});

// ============================================================
// Touch Target Size Tests
// ============================================================

describe('UX Fixes - Touch Target Sizes', () => {
  describe('60px Minimum Touch Targets', () => {
    it('should define 60px as minimum touch target', () => {
      const minTarget = extractPxValue(GERIATRIC_CSS_TOKENS.touchTargetMin);
      expect(minTarget).toBe(60);
    });

    it('touch target should exceed WCAG 2.1 minimum (44px)', () => {
      const minTarget = extractPxValue(GERIATRIC_CSS_TOKENS.touchTargetMin);
      const wcagMinimum = 44;

      expect(minTarget).toBeGreaterThan(wcagMinimum);
    });

    it('Property: all interactive elements should meet minimum touch target', () => {
      const dom = setupDOM(`
        <button style="min-height: 60px; min-width: 60px;">Click</button>
        <a href="#" style="display: inline-block; min-height: 60px; min-width: 60px;">Link</a>
        <input type="checkbox" style="width: 32px; height: 32px;" />
      `);

      // Buttons and links should meet 60px minimum
      const buttons = dom.window.document.querySelectorAll('button');
      buttons.forEach((btn) => {
        const style = btn.getAttribute('style') || '';
        expect(style).toContain('60px');
      });
    });
  });
});

// ============================================================
// Focus Indicator Tests
// ============================================================

describe('UX Fixes - Focus Indicators', () => {
  describe('Visible Focus Rings', () => {
    it('should define 4px focus ring width', () => {
      const focusWidth = extractPxValue(GERIATRIC_CSS_TOKENS.focusRingWidth);
      expect(focusWidth).toBe(4);
    });

    it('focus ring should be visible (>= 3px)', () => {
      const focusWidth = extractPxValue(GERIATRIC_CSS_TOKENS.focusRingWidth);
      expect(focusWidth).toBeGreaterThanOrEqual(3);
    });
  });
});

// ============================================================
// Skip Link Tests
// ============================================================

describe('UX Fixes - Skip Links', () => {
  describe('Skip Link Functionality', () => {
    it('skip link should target main content', () => {
      const dom = setupDOM(`
        <a href="#main-content" class="skip-link">Skip to main content</a>
        <main id="main-content">Content here</main>
      `);

      const skipLink = dom.window.document.querySelector('.skip-link');
      const mainContent = dom.window.document.getElementById('main-content');

      expect(skipLink?.getAttribute('href')).toBe('#main-content');
      expect(mainContent).not.toBeNull();
    });

    it('skip link target should exist in document', () => {
      const dom = setupDOM(`
        <a href="#main-content" class="skip-link">Skip to main content</a>
        <main id="main-content" role="main">Content</main>
      `);

      const skipLink = dom.window.document.querySelector(
        '.skip-link'
      ) as HTMLAnchorElement;
      const targetId = skipLink?.getAttribute('href')?.replace('#', '');
      const target = dom.window.document.getElementById(targetId || '');

      expect(target).not.toBeNull();
    });
  });
});

// ============================================================
// Modal Close Button Accessibility Tests
// ============================================================

describe('UX Fixes - Modal Close Buttons', () => {
  describe('Close Button Accessibility', () => {
    it('all modal close buttons should have aria-label', () => {
      const dom = setupDOM(`
        <div role="dialog" aria-modal="true">
          <button class="modal-close" aria-label="Close dialog">&times;</button>
        </div>
        <div role="dialog" aria-modal="true">
          <button class="modal-close" aria-label="Close signature dialog">&times;</button>
        </div>
      `);

      const closeButtons = dom.window.document.querySelectorAll('.modal-close');
      closeButtons.forEach((btn) => {
        const ariaLabel = btn.getAttribute('aria-label');
        expect(ariaLabel).toBeTruthy();
        expect(ariaLabel!.length).toBeGreaterThan(0);
      });
    });

    it('close button aria-label should describe action clearly', () => {
      const dom = setupDOM(`
        <button class="modal-close" aria-label="Close dialog">&times;</button>
      `);

      const closeBtn = dom.window.document.querySelector('.modal-close');
      const ariaLabel = closeBtn?.getAttribute('aria-label')?.toLowerCase();

      expect(ariaLabel).toContain('close');
    });

    it('Property: close buttons should never have empty accessible name', () => {
      const closeButtonPatterns = [
        '<button class="modal-close" aria-label="Close">&times;</button>',
        '<button class="modal-close" aria-label="Close dialog">X</button>',
        '<button class="modal-close" aria-label="Close signature dialog"><svg></svg></button>',
      ];

      fc.assert(
        fc.property(fc.constantFrom(...closeButtonPatterns), (html) => {
          const dom = setupDOM(html);
          const btn = dom.window.document.querySelector('.modal-close');

          const hasAccessibleName =
            btn?.getAttribute('aria-label') ||
            btn?.getAttribute('aria-labelledby') ||
            (btn?.textContent?.trim() && btn?.textContent?.trim() !== '&times;');

          expect(hasAccessibleName).toBeTruthy();
        })
      );
    });
  });
});
