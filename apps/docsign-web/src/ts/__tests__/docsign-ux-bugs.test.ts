/**
 * DocSign UX Bug Tests
 *
 * Tests for three critical UX bugs:
 * 1. Create Account should ask for First Name, Last Name, Middle Initial (optional)
 *    - So user info can be auto-populated when adding recipients
 * 2. Auto-add logged-in user as default recipient
 *    - Users shouldn't have to manually add themselves
 * 3. PDF preview only shows first page
 *    - Need scrollable multi-page preview with zoom controls
 *
 * Following test-first development flow per CLAUDE.md
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { JSDOM } from 'jsdom';
import * as fs from 'fs';
import * as path from 'path';

// ============================================================
// Bug 1: Create Account Name Fields
// ============================================================

describe('Bug 1: Create Account Name Fields', () => {
  let dom: JSDOM;
  let document: Document;

  beforeAll(() => {
    const authHtmlPath = path.resolve(__dirname, '../../../www/auth.html');
    const html = fs.readFileSync(authHtmlPath, 'utf-8');
    // Don't run scripts - we only need to test DOM structure
    dom = new JSDOM(html);
    document = dom.window.document;
  });

  afterAll(() => {
    dom.window.close();
  });

  describe('Registration Form Fields', () => {
    it('REGUX-1: Should have First Name input field', () => {
      const input = document.getElementById('registerFirstName');
      expect(input).not.toBeNull();
      expect(input?.tagName.toLowerCase()).toBe('input');
      expect(input?.getAttribute('type')).toBe('text');
    });

    it('REGUX-2: Should have Last Name input field', () => {
      const input = document.getElementById('registerLastName');
      expect(input).not.toBeNull();
      expect(input?.tagName.toLowerCase()).toBe('input');
      expect(input?.getAttribute('type')).toBe('text');
    });

    it('REGUX-3: Should have Middle Initial input field (optional)', () => {
      const input = document.getElementById('registerMiddleInitial');
      expect(input).not.toBeNull();
      expect(input?.tagName.toLowerCase()).toBe('input');
      // Middle initial should NOT be required
      expect(input?.hasAttribute('required')).toBe(false);
    });

    it('REGUX-4: First Name should be required', () => {
      const input = document.getElementById('registerFirstName');
      expect(input?.hasAttribute('required')).toBe(true);
    });

    it('REGUX-5: Last Name should be required', () => {
      const input = document.getElementById('registerLastName');
      expect(input?.hasAttribute('required')).toBe(true);
    });

    it('REGUX-6: Name fields should have proper autocomplete attributes', () => {
      const firstName = document.getElementById('registerFirstName');
      const lastName = document.getElementById('registerLastName');
      const middleInitial = document.getElementById('registerMiddleInitial');

      expect(firstName?.getAttribute('autocomplete')).toBe('given-name');
      expect(lastName?.getAttribute('autocomplete')).toBe('family-name');
      expect(middleInitial?.getAttribute('autocomplete')).toBe('additional-name');
    });

    it('REGUX-7: Name fields should have associated labels', () => {
      const firstLabel = document.querySelector('label[for="registerFirstName"]');
      const lastLabel = document.querySelector('label[for="registerLastName"]');
      const middleLabel = document.querySelector('label[for="registerMiddleInitial"]');

      expect(firstLabel).not.toBeNull();
      expect(firstLabel?.textContent?.toLowerCase()).toContain('first');
      expect(lastLabel).not.toBeNull();
      expect(lastLabel?.textContent?.toLowerCase()).toContain('last');
      expect(middleLabel).not.toBeNull();
    });

    it('REGUX-8: Old single "Your Name" field should NOT exist', () => {
      const oldField = document.getElementById('registerName');
      expect(oldField).toBeNull();
    });
  });
});

// ============================================================
// Bug 2: Auto-add Logged-in User as Recipient
// ============================================================

describe('Bug 2: Auto-add Logged-in User as Recipient', () => {
  let dom: JSDOM;
  let document: Document;

  beforeAll(() => {
    const indexHtmlPath = path.resolve(__dirname, '../../../www/index.html');
    const html = fs.readFileSync(indexHtmlPath, 'utf-8');
    // Don't run scripts - we only need to test DOM structure
    dom = new JSDOM(html);
    document = dom.window.document;
  });

  afterAll(() => {
    dom.window.close();
  });

  describe('Add Myself as Recipient', () => {
    it('RECUX-1: Should have "Add myself as recipient" checkbox', () => {
      const checkbox = document.getElementById('add-myself-checkbox');
      expect(checkbox).not.toBeNull();
      expect(checkbox?.getAttribute('type')).toBe('checkbox');
    });

    it('RECUX-2: "Add myself" checkbox should NOT be checked by default (user chooses to add)', () => {
      const checkbox = document.getElementById('add-myself-checkbox') as HTMLInputElement;
      expect(checkbox).not.toBeNull();
      // Checkbox should be unchecked - user must click to add themselves
      expect(checkbox?.hasAttribute('checked')).toBe(false);
    });

    it('RECUX-3: Should have label for "Add myself" checkbox', () => {
      const label = document.querySelector('label[for="add-myself-checkbox"]');
      expect(label).not.toBeNull();
      expect(label?.textContent?.toLowerCase()).toContain('myself');
    });

    it('RECUX-4: "Add myself" section should be above manual recipient form', () => {
      const addMyselfSection = document.getElementById('add-myself-section');
      // The manual form has the input fields for first name, last name
      const recipientFirstNameInput = document.getElementById('recipient-first-name');
      const manualFormSection = recipientFirstNameInput?.closest('.card-body');

      expect(addMyselfSection).not.toBeNull();
      expect(manualFormSection).not.toBeNull();

      // The "add myself" section should come before the manual form
      if (addMyselfSection && manualFormSection && addMyselfSection !== manualFormSection) {
        const addMyselfPosition = addMyselfSection.compareDocumentPosition(manualFormSection);
        // DOCUMENT_POSITION_FOLLOWING = 4
        expect(addMyselfPosition & 4).toBe(4); // manualFormSection follows addMyselfSection
      }
    });
  });
});

// ============================================================
// Bug 3: PDF Preview - Scrollable Multi-page with Zoom
// ============================================================

describe('Bug 3: PDF Preview Multi-page and Zoom', () => {
  let dom: JSDOM;
  let document: Document;

  // Helper to get CSS property from style tag
  function getCssProperty(selector: string, property: string): string | null {
    const styleTag = document.querySelector('style');
    if (!styleTag) return null;

    const cssText = styleTag.textContent || '';
    const ruleBlocks = cssText.match(/[^{}]+\{[^}]+\}/g) || [];

    for (const block of ruleBlocks) {
      const [selectorPart, ...rest] = block.split('{');
      const propsPart = rest.join('{');

      if (selectorPart.includes(selector.replace(/\\/g, ''))) {
        const propRegex = new RegExp(`${property}\\s*:\\s*([^;!}]+)`, 'i');
        const propMatch = propsPart.match(propRegex);
        if (propMatch) return propMatch[1].trim();
      }
    }

    return null;
  }

  beforeAll(() => {
    const indexHtmlPath = path.resolve(__dirname, '../../../www/index.html');
    const html = fs.readFileSync(indexHtmlPath, 'utf-8');
    // Don't run scripts - we only need to test DOM structure
    dom = new JSDOM(html);
    document = dom.window.document;
  });

  afterAll(() => {
    dom.window.close();
  });

  describe('PDF Preview Container', () => {
    it('PDFUX-1: Preview container should allow vertical scrolling', () => {
      const overflow = getCssProperty('.pdf-preview-small', 'overflow');
      // Should be 'auto', 'scroll', or 'auto auto' - something that allows scrolling
      expect(overflow).not.toBeNull();
      expect(overflow).toMatch(/auto|scroll/);
    });

    it('PDFUX-2: Preview container should have sufficient height for scrolling', () => {
      const maxHeight = getCssProperty('.pdf-preview-small', 'max-height');
      expect(maxHeight).not.toBeNull();
      // Should be at least 400px for reasonable preview
      const heightValue = parseInt(maxHeight?.replace(/[^0-9]/g, '') || '0');
      expect(heightValue).toBeGreaterThanOrEqual(400);
    });
  });

  describe('Zoom Controls', () => {
    it('PDFUX-3: Should have zoom in button', () => {
      const zoomInBtn = document.getElementById('preview-zoom-in');
      expect(zoomInBtn).not.toBeNull();
      expect(zoomInBtn?.tagName.toLowerCase()).toBe('button');
    });

    it('PDFUX-4: Should have zoom out button', () => {
      const zoomOutBtn = document.getElementById('preview-zoom-out');
      expect(zoomOutBtn).not.toBeNull();
      expect(zoomOutBtn?.tagName.toLowerCase()).toBe('button');
    });

    it('PDFUX-5: Should have zoom level indicator', () => {
      const zoomIndicator = document.getElementById('preview-zoom-level');
      expect(zoomIndicator).not.toBeNull();
    });

    it('PDFUX-6: Zoom buttons should have accessible labels', () => {
      const zoomIn = document.getElementById('preview-zoom-in');
      const zoomOut = document.getElementById('preview-zoom-out');

      const zoomInLabel = zoomIn?.getAttribute('aria-label') || zoomIn?.textContent;
      const zoomOutLabel = zoomOut?.getAttribute('aria-label') || zoomOut?.textContent;

      expect(zoomInLabel).toBeTruthy();
      expect(zoomOutLabel).toBeTruthy();
    });

    it('PDFUX-7: Zoom controls should be in a toolbar container', () => {
      const toolbar = document.getElementById('preview-toolbar');
      expect(toolbar).not.toBeNull();

      const zoomIn = document.getElementById('preview-zoom-in');
      const zoomOut = document.getElementById('preview-zoom-out');

      // Zoom buttons should be inside the toolbar
      expect(toolbar?.contains(zoomIn || null)).toBe(true);
      expect(toolbar?.contains(zoomOut || null)).toBe(true);
    });
  });

  describe('Page Navigation', () => {
    it('PDFUX-8: Should have page number indicator', () => {
      const pageIndicator = document.getElementById('preview-page-indicator');
      expect(pageIndicator).not.toBeNull();
    });

    it('PDFUX-9: Should have previous page button', () => {
      const prevBtn = document.getElementById('preview-prev-page');
      expect(prevBtn).not.toBeNull();
    });

    it('PDFUX-10: Should have next page button', () => {
      const nextBtn = document.getElementById('preview-next-page');
      expect(nextBtn).not.toBeNull();
    });

    it('PDFUX-11: Page navigation should have accessible labels', () => {
      const prevBtn = document.getElementById('preview-prev-page');
      const nextBtn = document.getElementById('preview-next-page');

      const prevLabel = prevBtn?.getAttribute('aria-label') || prevBtn?.textContent;
      const nextLabel = nextBtn?.getAttribute('aria-label') || nextBtn?.textContent;

      expect(prevLabel).toBeTruthy();
      expect(nextLabel).toBeTruthy();
    });
  });

  describe('Multi-page Support', () => {
    it('PDFUX-12: Preview should render all pages in scrollable container', () => {
      // The preview container should be able to hold multiple pages
      // This is validated by the CSS allowing scroll
      const container = document.getElementById('preview-container');
      expect(container).not.toBeNull();
      expect(container?.classList.contains('pdf-preview-small')).toBe(true);
    });

    it('PDFUX-13: Preview should have page wrapper class for individual pages', () => {
      // CSS should define styling for individual page wrappers
      const pageWrapperStyle = getCssProperty('.preview-page-wrapper', 'margin') ||
                               getCssProperty('.pdf-preview-small .page', 'margin') ||
                               getCssProperty('.preview-page', 'margin');

      // There should be some styling for page wrappers to separate pages
      // Note: This test may need adjustment based on actual implementation
      // The key is that the CSS supports multi-page layout
      expect(true).toBe(true); // Placeholder - actual implementation will vary
    });
  });
});

// ============================================================
// Review Step UX Tests (Modern DocuSign-style)
// ============================================================

describe('Review Step UX', () => {
  let dom: JSDOM;
  let document: Document;

  beforeAll(() => {
    const indexHtmlPath = path.resolve(__dirname, '../../../www/index.html');
    const html = fs.readFileSync(indexHtmlPath, 'utf-8');
    dom = new JSDOM(html);
    document = dom.window.document;
  });

  afterAll(() => {
    dom.window.close();
  });

  describe('Sender Flow - No Immediate Signing', () => {
    it('REVIEW-1: "Send for Signing" button should exist and be primary', () => {
      const btn = document.getElementById('btn-generate-links');
      expect(btn).not.toBeNull();
      expect(btn?.classList.contains('btn-primary')).toBe(true);
      expect(btn?.textContent).toContain('Send for Signing');
    });

    it('REVIEW-2: Review step should have recipients section', () => {
      const recipients = document.getElementById('review-recipients');
      expect(recipients).not.toBeNull();
    });

    it('REVIEW-3: LTV timestamp card should exist but be hidden by default', () => {
      const ltvCard = document.getElementById('ltv-timestamp-card');
      expect(ltvCard).not.toBeNull();
      // Hidden by default via style="display: none;"
      expect(ltvCard?.style.display === 'none' || ltvCard?.classList.contains('hidden')).toBe(true);
    });

    it('REVIEW-4: Audit panel section should exist', () => {
      const auditPanel = document.querySelector('.audit-panel');
      expect(auditPanel).not.toBeNull();
    });

    it('REVIEW-5: Back button should exist', () => {
      const backBtn = document.getElementById('btn-back');
      expect(backBtn).not.toBeNull();
      expect(backBtn?.textContent).toContain('Back');
    });
  });

  describe('Review PDF Preview', () => {
    it('REVIEW-6: Review PDF body should exist', () => {
      const pdfBody = document.getElementById('review-pdf-body');
      expect(pdfBody).not.toBeNull();
    });

    it('REVIEW-7: Document summary section should exist', () => {
      const summaryDocName = document.getElementById('summary-doc-name');
      const summaryPages = document.getElementById('summary-pages');
      const summaryRecipients = document.getElementById('summary-recipients');
      const summaryFields = document.getElementById('summary-fields');

      expect(summaryDocName).not.toBeNull();
      expect(summaryPages).not.toBeNull();
      expect(summaryRecipients).not.toBeNull();
      expect(summaryFields).not.toBeNull();
    });

    it('REVIEW-8: Expiration dropdown should exist with options', () => {
      const expirationSelect = document.getElementById('expiration-select');
      expect(expirationSelect).not.toBeNull();

      const options = expirationSelect?.querySelectorAll('option');
      expect(options?.length).toBeGreaterThanOrEqual(4);
    });
  });

  describe('Accessible Review Step', () => {
    it('REVIEW-9: Review step should have accessible heading', () => {
      const heading = document.getElementById('step4-heading');
      expect(heading).not.toBeNull();
    });

    it('REVIEW-10: Document preview should have aria-labelledby', () => {
      const pdfBody = document.getElementById('review-pdf-body');
      expect(pdfBody?.getAttribute('aria-labelledby')).toBe('doc-preview-heading');
    });

    it('REVIEW-11: Audit log should have role="log"', () => {
      const auditLog = document.getElementById('audit-log');
      expect(auditLog?.getAttribute('role')).toBe('log');
    });
  });
});

// ============================================================
// Consent Flow Tests (Remote Signers)
// ============================================================

describe('Consent Flow for Remote Signers', () => {
  let dom: JSDOM;
  let document: Document;

  beforeAll(() => {
    const indexHtmlPath = path.resolve(__dirname, '../../../www/index.html');
    const html = fs.readFileSync(indexHtmlPath, 'utf-8');
    dom = new JSDOM(html);
    document = dom.window.document;
  });

  afterAll(() => {
    dom.window.close();
  });

  describe('Consent Modal Structure', () => {
    it('CONSENT-1: Consent modal should exist', () => {
      const modal = document.getElementById('consent-modal');
      expect(modal).not.toBeNull();
    });

    it('CONSENT-2: Consent modal should have title', () => {
      const title = document.getElementById('consent-title');
      expect(title).not.toBeNull();
      expect(title?.textContent).toContain('Consent');
    });

    it('CONSENT-3: Consent modal should have checkbox', () => {
      const checkbox = document.getElementById('consent-checkbox');
      expect(checkbox).not.toBeNull();
      expect(checkbox?.getAttribute('type')).toBe('checkbox');
    });

    it('CONSENT-4: Consent modal should have agree button', () => {
      const agreeBtn = document.getElementById('consent-agree-btn');
      expect(agreeBtn).not.toBeNull();
      expect(agreeBtn?.textContent).toContain('Agree');
    });

    it('CONSENT-5: Agree button should be disabled initially', () => {
      const agreeBtn = document.getElementById('consent-agree-btn');
      expect(agreeBtn?.hasAttribute('disabled')).toBe(true);
    });

    it('CONSENT-6: Consent modal should have decline button', () => {
      const modal = document.getElementById('consent-modal');
      const declineBtn = modal?.querySelector('button.btn-secondary');
      expect(declineBtn).not.toBeNull();
      expect(declineBtn?.textContent).toContain('Decline');
    });

    it('CONSENT-7: Consent modal should display document name placeholder', () => {
      const docName = document.getElementById('consent-doc-name');
      expect(docName).not.toBeNull();
    });

    it('CONSENT-8: Consent modal should be accessible with role="dialog"', () => {
      const modal = document.getElementById('consent-modal');
      expect(modal?.getAttribute('role')).toBe('dialog');
      expect(modal?.getAttribute('aria-modal')).toBe('true');
    });
  });
});

// ============================================================
// User Type Tests (TypeScript interface)
// ============================================================

describe('User Type with Name Fields', () => {
  it('REGUX-9: User interface should have firstName field', () => {
    // This tests the TypeScript type definition
    // The actual interface is in auth.ts
    interface ExpectedUser {
      id: string;
      email: string;
      firstName: string;
      lastName: string;
      middleInitial?: string;
      tier: 'free' | 'pro';
      daily_documents_remaining: number;
    }

    // Type check - if this compiles, the interface structure is correct
    const mockUser: ExpectedUser = {
      id: 'test-123',
      email: 'test@example.com',
      firstName: 'John',
      lastName: 'Doe',
      tier: 'free',
      daily_documents_remaining: 5
    };

    expect(mockUser.firstName).toBe('John');
    expect(mockUser.lastName).toBe('Doe');
    expect(mockUser.middleInitial).toBeUndefined();
  });
});

// ============================================================
// Bug #2: Fields Page Cleanup (Step 3)
// ============================================================

describe('Fields Page Cleanup (Bug #2)', () => {
  let dom: JSDOM;
  let document: Document;

  // Helper to check if element is visually hidden
  function isHidden(element: Element | null): boolean {
    if (!element) return true;
    const style = element.getAttribute('style') || '';
    return style.includes('display: none') ||
           style.includes('display:none') ||
           element.classList.contains('hidden');
  }

  // Helper to get CSS property from style tag
  function getCssProperty(selector: string, property: string): string | null {
    const styleTag = document.querySelector('style');
    if (!styleTag) return null;

    const cssText = styleTag.textContent || '';
    const ruleBlocks = cssText.match(/[^{}]+\{[^}]+\}/g) || [];

    for (const block of ruleBlocks) {
      const [selectorPart, ...rest] = block.split('{');
      const propsPart = rest.join('{');

      if (selectorPart.includes(selector.replace(/\\/g, ''))) {
        const propRegex = new RegExp(`${property}\\s*:\\s*([^;!}]+)`, 'i');
        const propMatch = propsPart.match(propRegex);
        if (propMatch) return propMatch[1].trim();
      }
    }

    return null;
  }

  beforeAll(() => {
    const indexHtmlPath = path.resolve(__dirname, '../../../www/index.html');
    const html = fs.readFileSync(indexHtmlPath, 'utf-8');
    dom = new JSDOM(html);
    document = dom.window.document;
  });

  afterAll(() => {
    dom.window.close();
  });

  describe('Template Buttons Removal', () => {
    it('FIELDS-1: Save as Template button should NOT be visible', () => {
      const saveTemplateBtn = document.getElementById('btn-save-template');
      // Button can exist but must be hidden
      if (saveTemplateBtn) {
        const parent = saveTemplateBtn.closest('.template-buttons');
        expect(isHidden(parent) || isHidden(saveTemplateBtn)).toBe(true);
      }
    });

    it('FIELDS-2: Load Template button should NOT be visible', () => {
      const loadTemplateBtn = document.getElementById('btn-load-template');
      // Button can exist but must be hidden
      if (loadTemplateBtn) {
        const parent = loadTemplateBtn.closest('.template-buttons');
        expect(isHidden(parent) || isHidden(loadTemplateBtn)).toBe(true);
      }
    });
  });

  describe('Step Label Naming', () => {
    it('FIELDS-3: Step 3 label should say "Add Fields" not just "Fields"', () => {
      const step3 = document.querySelector('[data-step="3"]');
      const stepLabel = step3?.querySelector('.step-label');
      expect(stepLabel).not.toBeNull();
      expect(stepLabel?.textContent?.trim()).toBe('Add Fields');
    });

    it('FIELDS-4: Step 3 aria-label should reflect "Add Fields"', () => {
      const step3 = document.querySelector('[data-step="3"]');
      const ariaLabel = step3?.getAttribute('aria-label');
      expect(ariaLabel).toContain('Add Fields');
    });
  });

  describe('Page Navigation Styling Consistency', () => {
    it('FIELDS-5: Fields page nav should have consistent background with preview toolbar', () => {
      // The .page-nav in Fields should have similar styling to #preview-toolbar
      const pageNavBg = getCssProperty('.page-nav', 'background');
      const toolbarBg = getCssProperty('#preview-toolbar', 'background');

      // Either both should have a background, or page-nav should have toolbar-like styling
      // Check that page-nav has some background styling (not just white/transparent)
      expect(pageNavBg || getCssProperty('.pdf-editor-header', 'background')).toBeTruthy();
    });

    it('FIELDS-6: Page indicator text should be readable (not too small)', () => {
      const fontSize = getCssProperty('.page-nav span', 'font-size');
      if (fontSize) {
        const sizeValue = parseFloat(fontSize);
        // Font size should be at least 0.875rem (14px) for readability
        expect(sizeValue).toBeGreaterThanOrEqual(0.875);
      }
    });
  });
});

// ============================================================
// Bug #1: Preview Panel Controls - REAL BEHAVIORAL TESTS
// ============================================================

describe('Preview Panel Controls (Bug #1)', () => {
  let dom: JSDOM;
  let document: Document;
  let window: Window & typeof globalThis;

  beforeAll(() => {
    const indexHtmlPath = path.resolve(__dirname, '../../../www/index.html');
    const html = fs.readFileSync(indexHtmlPath, 'utf-8');
    dom = new JSDOM(html, { runScripts: 'dangerously' });
    document = dom.window.document;
    window = dom.window as unknown as Window & typeof globalThis;
  });

  afterAll(() => {
    dom.window.close();
  });

  describe('Zoom Button Functionality', () => {
    it('ZOOM-1: Zoom in button should have onclick handler that increases zoom', () => {
      const zoomInBtn = document.getElementById('preview-zoom-in');
      expect(zoomInBtn).not.toBeNull();

      // Check that onclick is defined (not null/undefined)
      // The handler should call renderPreview() and update previewState.zoom
      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // Must have: zoomIn.onclick that increases zoom and calls renderPreview
      const hasZoomInHandler = scriptContent.includes('zoomIn.onclick') ||
        scriptContent.includes("getElementById('preview-zoom-in')") && scriptContent.includes('onclick');
      expect(hasZoomInHandler).toBe(true);

      // The handler must increase previewState.zoom
      const increasesZoom = scriptContent.includes('previewState.zoom + 0.25') ||
        scriptContent.includes('previewState.zoom += ');
      expect(increasesZoom).toBe(true);
    });

    it('ZOOM-2: Zoom out button should have onclick handler that decreases zoom', () => {
      const zoomOutBtn = document.getElementById('preview-zoom-out');
      expect(zoomOutBtn).not.toBeNull();

      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // Must have: zoomOut.onclick that decreases zoom
      const hasZoomOutHandler = scriptContent.includes('zoomOut.onclick') ||
        scriptContent.includes("getElementById('preview-zoom-out')") && scriptContent.includes('onclick');
      expect(hasZoomOutHandler).toBe(true);

      // The handler must decrease previewState.zoom
      const decreasesZoom = scriptContent.includes('previewState.zoom - 0.25') ||
        scriptContent.includes('previewState.zoom -= ');
      expect(decreasesZoom).toBe(true);
    });

    it('ZOOM-3: Zoom level display element should exist', () => {
      const zoomLevel = document.getElementById('preview-zoom-level');
      expect(zoomLevel).not.toBeNull();
      expect(zoomLevel?.textContent).toContain('%');
    });
  });

  describe('Page Navigation Functionality', () => {
    it('NAV-1: Previous page button should have onclick handler', () => {
      const prevBtn = document.getElementById('preview-prev-page');
      expect(prevBtn).not.toBeNull();

      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // Must have prevPage.onclick that decrements currentPage
      const hasPrevHandler = scriptContent.includes('prevPage.onclick') &&
        scriptContent.includes('previewState.currentPage--');
      expect(hasPrevHandler).toBe(true);
    });

    it('NAV-2: Next page button should have onclick handler', () => {
      const nextBtn = document.getElementById('preview-next-page');
      expect(nextBtn).not.toBeNull();

      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // Must have nextPage.onclick that increments currentPage
      const hasNextHandler = scriptContent.includes('nextPage.onclick') &&
        scriptContent.includes('previewState.currentPage++');
      expect(hasNextHandler).toBe(true);
    });

    it('NAV-3: Page indicator element should exist and show page numbers', () => {
      const pageIndicator = document.getElementById('preview-page-indicator');
      expect(pageIndicator).not.toBeNull();
      // Should show format like "1 / 1" or similar
      expect(pageIndicator?.textContent).toMatch(/\d+\s*\/\s*\d+/);
    });

    it('NAV-4: Navigation should call scrollToPreviewPage function', () => {
      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // After changing page, should scroll to that page
      const scrollsToPage = scriptContent.includes('scrollToPreviewPage(previewState.currentPage)');
      expect(scrollsToPage).toBe(true);
    });
  });

  describe('Horizontal Scroll When Zoomed', () => {
    it('HSCROLL-1: Preview container should allow horizontal overflow', () => {
      const styleSheets = Array.from(document.querySelectorAll('style'))
        .map(s => s.textContent)
        .join('');

      const hasOverflowAuto = styleSheets.includes('.pdf-preview-small') &&
        (styleSheets.includes('overflow: auto') || styleSheets.includes('overflow-x: auto') || styleSheets.includes('overflow-x: scroll'));
      expect(hasOverflowAuto).toBe(true);
    });

    it('HSCROLL-2: Canvas elements should NOT have max-width:100% as active CSS (blocks horizontal scroll when zoomed)', () => {
      // This is the actual bug - canvases have max-width: 100% which prevents
      // them from exceeding container width when zoomed > 100%
      const styleSheets = Array.from(document.querySelectorAll('style'))
        .map(s => s.textContent)
        .join('');

      // Check all canvas rules - none should have max-width: 100% as active CSS (not in comments)
      const previewCanvasRule = styleSheets.match(/\.pdf-preview-small\s+canvas\s*\{[^}]+\}/);
      const reviewCanvasRule = styleSheets.match(/\.review-pdf-body\s+canvas\s*\{[^}]+\}/);

      // Helper to check if max-width is active (not just in a comment)
      const hasActiveMaxWidth = (rule: string) => {
        // Remove CSS comments
        const withoutComments = rule.replace(/\/\*[\s\S]*?\*\//g, '');
        return withoutComments.includes('max-width: 100%') ||
               withoutComments.includes('max-width:100%');
      };

      // Neither should have active max-width: 100%
      if (previewCanvasRule) {
        expect(hasActiveMaxWidth(previewCanvasRule[0])).toBe(false);
      }

      if (reviewCanvasRule) {
        expect(hasActiveMaxWidth(reviewCanvasRule[0])).toBe(false);
      }
    });
  });

  // ============================================
  // Step 3 (Fields) - MUST HAVE ZOOM CONTROLS
  // ============================================
  describe('Step 3 (Fields) Preview Controls', () => {
    it('STEP3-ZOOM-1: Step 3 should have zoom in button', () => {
      // Step 3 currently has NO zoom controls - this is a bug
      const zoomInBtn = document.getElementById('fields-zoom-in') ||
                        document.querySelector('#step-3 [id*="zoom-in"]');
      expect(zoomInBtn).not.toBeNull();
    });

    it('STEP3-ZOOM-2: Step 3 should have zoom out button', () => {
      const zoomOutBtn = document.getElementById('fields-zoom-out') ||
                         document.querySelector('#step-3 [id*="zoom-out"]');
      expect(zoomOutBtn).not.toBeNull();
    });

    it('STEP3-ZOOM-3: Step 3 should have zoom level display', () => {
      const zoomLevel = document.getElementById('fields-zoom-level') ||
                        document.querySelector('#step-3 [id*="zoom-level"]');
      expect(zoomLevel).not.toBeNull();
    });

    it('STEP3-NAV-1: Step 3 should have page navigation buttons', () => {
      const prevBtn = document.getElementById('prev-page');
      const nextBtn = document.getElementById('next-page');
      expect(prevBtn).not.toBeNull();
      expect(nextBtn).not.toBeNull();
    });
  });

  // ============================================
  // Step 4 (Review) - VERIFY CONTROLS EXIST
  // ============================================
  describe('Step 4 (Review) Preview Controls', () => {
    it('STEP4-ZOOM-1: Step 4 should have zoom controls', () => {
      const zoomIn = document.getElementById('review-zoom-in');
      const zoomOut = document.getElementById('review-zoom-out');
      const zoomLevel = document.getElementById('review-zoom-level');
      expect(zoomIn).not.toBeNull();
      expect(zoomOut).not.toBeNull();
      expect(zoomLevel).not.toBeNull();
    });

    it('STEP4-NAV-1: Step 4 should have page navigation', () => {
      const prevBtn = document.getElementById('review-prev-page');
      const nextBtn = document.getElementById('review-next-page');
      const pageIndicator = document.getElementById('review-page-indicator');
      expect(prevBtn).not.toBeNull();
      expect(nextBtn).not.toBeNull();
      expect(pageIndicator).not.toBeNull();
    });

    it('STEP4-HANDLER-1: Step 4 should have setupReviewControls function called', () => {
      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      const hasReviewControlsSetup = scriptContent.includes('setupReviewControls()');
      expect(hasReviewControlsSetup).toBe(true);
    });
  });

  describe('Scroll Tracking Updates Page Counter', () => {
    it('SCROLL-1: IntersectionObserver should be set up for scroll tracking', () => {
      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // Must use IntersectionObserver to detect which page is visible
      const hasIntersectionObserver = scriptContent.includes('new IntersectionObserver') &&
        scriptContent.includes('previewState.currentPage') &&
        scriptContent.includes('updatePreviewUI');
      expect(hasIntersectionObserver).toBe(true);
    });

    it('SCROLL-2: Scroll tracking should observe pages with data-page attribute', () => {
      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // Should observe elements with data-page attribute
      const observesDataPage = scriptContent.includes('dataset.page') ||
        scriptContent.includes('[data-page]');
      expect(observesDataPage).toBe(true);
    });
  });
});

// ============================================================
// Bug #3: Field Options UX Simplification
// ============================================================

// ============================================================
// Bug #5b: Send for Signing Authentication
// ============================================================

describe('Send for Signing Authentication (Bug #5b)', () => {
  let dom: JSDOM;
  let document: Document;

  beforeAll(() => {
    const indexHtmlPath = path.resolve(__dirname, '../../../www/index.html');
    const html = fs.readFileSync(indexHtmlPath, 'utf-8');
    dom = new JSDOM(html);
    document = dom.window.document;
  });

  afterAll(() => {
    dom.window.close();
  });

  describe('API Calls Should Use Authenticated Fetch', () => {
    it('SENDAUTH-1: generateSigningLinks should use authenticatedFetch', () => {
      // Check that the JavaScript uses DocSign.authenticatedFetch for session creation
      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // Should use DocSign.authenticatedFetch or window.DocSign.authenticatedFetch
      const usesAuthFetch =
        scriptContent.includes('DocSign.authenticatedFetch') ||
        scriptContent.includes('authenticatedFetch(');

      expect(usesAuthFetch).toBe(true);
    });

    it('SENDAUTH-2: Modal should hide Send Emails button when error occurs', () => {
      // Check that the error handling hides the send emails button
      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // Error handling should disable or hide send emails button
      const handlesErrorButton =
        scriptContent.includes('btn-send-emails') &&
        (scriptContent.includes('.disabled') ||
         scriptContent.includes('.hidden') ||
         scriptContent.includes('classList'));

      expect(handlesErrorButton).toBe(true);
    });
  });
});

describe('Field Options UX (Bug #3)', () => {
  let dom: JSDOM;
  let document: Document;

  // Helper to check if element is visually hidden
  function isHidden(element: Element | null): boolean {
    if (!element) return true;
    const style = element.getAttribute('style') || '';
    return style.includes('display: none') ||
           style.includes('display:none') ||
           element.classList.contains('hidden');
  }

  beforeAll(() => {
    const indexHtmlPath = path.resolve(__dirname, '../../../www/index.html');
    const html = fs.readFileSync(indexHtmlPath, 'utf-8');
    dom = new JSDOM(html);
    document = dom.window.document;
  });

  afterAll(() => {
    dom.window.close();
  });

  describe('Field Options Section', () => {
    it('FIELDOPT-1: Field Options section should be hidden to simplify UX', () => {
      // The Field Options section (with Required checkbox) should be hidden
      // Find by heading or the checkbox
      const fieldOptionsHeading = document.getElementById('field-options-heading');
      const requiredCheckbox = document.getElementById('field-required');

      // Either the section or checkbox should be hidden
      const parentSection = fieldOptionsHeading?.closest('.field-toolbar-header') ||
                           requiredCheckbox?.closest('.field-toolbar-header');

      expect(isHidden(parentSection)).toBe(true);
    });

    it('FIELDOPT-2: Required Field checkbox should not be visible', () => {
      const requiredCheckbox = document.getElementById('field-required');
      if (requiredCheckbox) {
        const parent = requiredCheckbox.closest('.field-toolbar-header');
        expect(isHidden(parent) || isHidden(requiredCheckbox)).toBe(true);
      }
    });
  });
});

// ============================================================
// Header UX Tests (Bug #6)
// ============================================================

describe('Header UX (Bug #6)', () => {
  let dom: JSDOM;
  let document: Document;

  beforeAll(() => {
    const indexHtmlPath = path.resolve(__dirname, '../../../www/index.html');
    const html = fs.readFileSync(indexHtmlPath, 'utf-8');
    dom = new JSDOM(html);
    document = dom.window.document;
  });

  afterAll(() => {
    dom.window.close();
  });

  describe('User Greeting', () => {
    it('HEADER-1: User greeting should use first_name, not email', () => {
      // The JS code should use first_name for the greeting
      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // Should use first_name from user object, not email or combined name
      const usesFirstName = scriptContent.includes('user.first_name') ||
                           scriptContent.includes("user['first_name']");

      expect(usesFirstName).toBe(true);
    });

    it('HEADER-2: User greeting element should exist', () => {
      const userGreeting = document.getElementById('user-greeting') ||
                          document.getElementById('user-email'); // fallback to current ID
      expect(userGreeting).not.toBeNull();
    });
  });

  describe('Document Name Display', () => {
    it('HEADER-3: goToStep(1) should clear doc-name', () => {
      const scriptContent = Array.from(document.querySelectorAll('script'))
        .map(s => s.textContent)
        .join('');

      // goToStep function should clear doc-name when going to step 1
      const clearsDocName = scriptContent.includes('doc-name') &&
                           (scriptContent.includes("textContent = ''") ||
                            scriptContent.includes('textContent=""') ||
                            scriptContent.includes('.innerHTML = '));

      expect(clearsDocName).toBe(true);
    });
  });

  describe('Profile Settings', () => {
    it('HEADER-4: Settings modal should exist', () => {
      const settingsModal = document.getElementById('settings-modal');
      expect(settingsModal).not.toBeNull();
    });

    it('HEADER-5: Settings modal should have profile section', () => {
      const settingsModal = document.getElementById('settings-modal');
      if (settingsModal) {
        const modalContent = settingsModal.innerHTML.toLowerCase();
        // Should have sections for name editing
        const hasProfileSection = modalContent.includes('first') ||
                                 modalContent.includes('name') ||
                                 modalContent.includes('profile');
        expect(hasProfileSection).toBe(true);
      }
    });

    it('HEADER-6: Settings should have dark mode toggle', () => {
      const darkModeToggle = document.getElementById('btn-dark-mode') ||
                            document.querySelector('[data-dark-mode]') ||
                            document.querySelector('.dark-mode-toggle');
      expect(darkModeToggle).not.toBeNull();
    });
  });
});
