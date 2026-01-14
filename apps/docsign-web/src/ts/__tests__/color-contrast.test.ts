/**
 * Bug #24: Color Contrast Accessibility Tests
 *
 * Scans ALL HTML files in www/ directory to ensure form inputs
 * with background colors also have explicit text colors.
 *
 * This prevents white-on-white or dark-on-dark text issues
 * that make inputs unreadable for users.
 */

import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Find the www directory by looking for docsign-web package root
 */
function findWwwDir(): string {
  // Try relative from test file location
  const fromTests = path.resolve(__dirname, '../../../../www');
  if (fs.existsSync(fromTests)) {
    return fromTests;
  }

  // Try from process.cwd() (vitest runs from package root)
  const fromCwd = path.join(process.cwd(), 'www');
  if (fs.existsSync(fromCwd)) {
    return fromCwd;
  }

  // Fallback: search upward for www directory
  let current = __dirname;
  while (current !== path.dirname(current)) {
    const wwwPath = path.join(current, 'www');
    if (
      fs.existsSync(wwwPath) &&
      fs.existsSync(path.join(wwwPath, 'admin.html'))
    ) {
      return wwwPath;
    }
    current = path.dirname(current);
  }

  return path.resolve(__dirname, '../../../../www');
}

const WWW_DIR = findWwwDir();

/**
 * Get all HTML files from the www directory
 */
function getHtmlFiles(): string[] {
  if (!fs.existsSync(WWW_DIR)) {
    return [];
  }
  return fs
    .readdirSync(WWW_DIR)
    .filter((f) => f.endsWith('.html'))
    .map((f) => path.join(WWW_DIR, f));
}

/**
 * Extract CSS blocks from HTML content (inline <style> tags)
 */
function extractStyleBlocks(html: string): string[] {
  const styleRegex = /<style[^>]*>([\s\S]*?)<\/style>/gi;
  const blocks: string[] = [];
  let match;
  while ((match = styleRegex.exec(html)) !== null) {
    blocks.push(match[1]);
  }
  return blocks;
}

/**
 * Check if a CSS block contains form input styling with background but no color
 *
 * Looks for patterns like:
 *   .form-group input { background: ...; } <- MISSING color property
 *
 * This is a heuristic check - it's not a full CSS parser, but catches common issues.
 *
 * Exclusions (intentional background-only rules):
 * - Toggle switches (decorative, not text display)
 * - Checkbox/radio inputs (decorative, browser handles)
 * - Button states like :hover, :active, .selected (inherit text color)
 * - File inputs (browser handles text)
 */
function findInputsWithBackgroundNoColor(css: string): string[] {
  const issues: string[] = [];

  // Pattern: CSS rule for form inputs (input, select, textarea) with background but no color
  // This regex matches a CSS rule block and captures the selector and properties
  const ruleRegex =
    /([^{}]*(?:input|select|textarea)[^{}]*)\{([^}]+)\}/gi;

  // Patterns to EXCLUDE (decorative elements, not text inputs)
  const exclusionPatterns = [
    /toggle/i, // Toggle switches
    /slider/i, // Toggle sliders
    /checkbox/i, // Checkboxes
    /radio/i, // Radio buttons
    /type="file"/i, // File inputs (browser handles)
    /type="checkbox"/i,
    /type="radio"/i,
    /:checked/i, // Checkbox/toggle checked states
    /:hover/i, // Hover states (inherit color)
    /:active/i, // Active states
    /:focus/i, // Focus states (usually border, not background)
    /\.selected/i, // Selected states on buttons/tabs
    /\.active/i, // Active states
  ];

  let match;
  while ((match = ruleRegex.exec(css)) !== null) {
    const selector = match[1].trim();
    const properties = match[2];

    // Skip excluded patterns (decorative elements)
    if (exclusionPatterns.some((pattern) => pattern.test(selector))) {
      continue;
    }

    // Check if rule has background (or background-color) property
    const hasBackground = /background(?:-color)?:\s*[^;]+/i.test(properties);

    // Check if rule has explicit color property
    const hasColor = /(?:^|;\s*)color:\s*[^;]+/i.test(properties);

    if (hasBackground && !hasColor) {
      issues.push(`Selector "${selector}" has background but no color property`);
    }
  }

  return issues;
}

// ============================================================
// Bug #24: Color Contrast Tests
// ============================================================

describe('Bug #24: Form Input Color Contrast', () => {
  const htmlFiles = getHtmlFiles();

  it('www directory exists and has HTML files', () => {
    expect(fs.existsSync(WWW_DIR)).toBe(true);
    expect(htmlFiles.length).toBeGreaterThan(0);
  });

  describe('All HTML files: inputs with background must have color', () => {
    htmlFiles.forEach((filePath) => {
      const fileName = path.basename(filePath);

      it(`${fileName}: form inputs with background should have explicit color`, () => {
        const content = fs.readFileSync(filePath, 'utf-8');
        const styleBlocks = extractStyleBlocks(content);
        const allIssues: string[] = [];

        for (const css of styleBlocks) {
          const issues = findInputsWithBackgroundNoColor(css);
          allIssues.push(...issues);
        }

        // This test should FAIL for admin.html (Bug #23) until fixed
        expect(allIssues).toEqual([]);
      });
    });
  });

  describe('Bug #23: admin.html specific check', () => {
    it('admin.html .form-group inputs must have color: var(--text-primary)', () => {
      const adminPath = path.join(WWW_DIR, 'admin.html');

      if (!fs.existsSync(adminPath)) {
        // Skip if admin.html doesn't exist
        return;
      }

      const content = fs.readFileSync(adminPath, 'utf-8');

      // Check that .form-group input/select/textarea rules have color property
      // This regex is more specific to the admin.html structure
      const formGroupInputRule =
        /\.form-group\s+(input|select|textarea)[^{]*\{([^}]+)\}/gi;

      let hasProperColorRule = false;
      let match;

      while ((match = formGroupInputRule.exec(content)) !== null) {
        const properties = match[2];
        const hasBackground = /background:\s*var\(--bg-tertiary\)/i.test(
          properties
        );
        const hasColor = /color:\s*var\(--text-primary\)/i.test(properties);

        if (hasBackground && hasColor) {
          hasProperColorRule = true;
        }
      }

      // This test should FAIL until Bug #23 is fixed
      expect(hasProperColorRule).toBe(true);
    });
  });
});

// ============================================================
// CSS Variable Contrast Analysis
// ============================================================

describe('CSS Variable Color Definitions', () => {
  it('should define contrasting text/background color pairs', () => {
    const adminPath = path.join(WWW_DIR, 'admin.html');
    if (!fs.existsSync(adminPath)) return;

    const content = fs.readFileSync(adminPath, 'utf-8');

    // Check that light mode has dark text on light backgrounds
    const lightModeVars = {
      '--bg-tertiary': /#f9fafb/i, // Light gray background
      '--text-primary': /#111827/i, // Dark text
    };

    for (const [varName, expectedPattern] of Object.entries(lightModeVars)) {
      const regex = new RegExp(`${varName}:\\s*([^;]+);`);
      const match = content.match(regex);
      expect(match).toBeTruthy();
      if (match) {
        expect(match[1].trim()).toMatch(expectedPattern);
      }
    }
  });

  it('should define contrasting dark mode colors', () => {
    const adminPath = path.join(WWW_DIR, 'admin.html');
    if (!fs.existsSync(adminPath)) return;

    const content = fs.readFileSync(adminPath, 'utf-8');

    // Check dark mode section exists with proper contrast
    const hasDarkMode = /body\.dark-mode\s*\{[\s\S]*?--text-primary:\s*#f9fafb/i.test(
      content
    );
    expect(hasDarkMode).toBe(true);
  });
});
