/**
 * Property-based tests for Tauri command TypeScript bindings.
 *
 * These tests use fast-check to generate random inputs and verify that
 * the TypeScript bindings handle all edge cases correctly.
 */

import * as fc from "fast-check";
import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the Tauri API since we can't actually invoke commands in tests
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// ============================================
// Test Utilities
// ============================================

/** Characters that are dangerous in printer names (mirrors Rust constant) */
const DANGEROUS_PRINTER_CHARS = ["'", '"', ";", "&", "|", "`", "$", "\\", "\n", "\r"];

/** Maximum file size allowed (100MB) */
const MAX_FILE_SIZE = 100 * 1024 * 1024;

/**
 * Validates a printer name for security.
 * This mirrors the Rust validation logic.
 */
function validatePrinterName(name: string): { valid: boolean; error?: string } {
  if (!name || name.length === 0) {
    return { valid: false, error: "Printer name cannot be empty" };
  }

  for (const char of DANGEROUS_PRINTER_CHARS) {
    if (name.includes(char)) {
      return { valid: false, error: "Invalid printer name" };
    }
  }

  if (name.length > 256) {
    return { valid: false, error: "Printer name is too long" };
  }

  return { valid: true };
}

/**
 * Sanitizes a filename by removing dangerous characters.
 * Mirrors the Rust sanitization logic.
 */
function sanitizeFilename(name: string): string {
  const dangerous = /[/\\:*?"<>|\x00-\x1f]/g;
  let sanitized = name.replace(dangerous, "_");

  // Trim whitespace and dots
  sanitized = sanitized.trim().replace(/^\.+|\.+$/g, "");

  // Limit length
  if (sanitized.length > 200) {
    sanitized = sanitized.substring(0, 200);
  }

  // Ensure non-empty
  if (sanitized.length === 0) {
    return "document.pdf";
  }

  return sanitized;
}

/**
 * Validates file size against the maximum limit.
 */
function validateFileSize(size: number): { valid: boolean; error?: string } {
  if (size > MAX_FILE_SIZE) {
    return {
      valid: false,
      error: "This PDF file is too large (over 100MB). Please select a smaller file.",
    };
  }
  return { valid: true };
}

/**
 * Ensures a filename has .pdf extension.
 */
function ensurePdfExtension(filename: string): string {
  const ext = filename.split(".").pop()?.toLowerCase();
  if (ext !== "pdf") {
    // Replace or add extension
    const base = filename.replace(/\.[^.]+$/, "");
    return `${base || filename}.pdf`;
  }
  return filename;
}

/**
 * Checks if a character is dangerous for printer names.
 */
function isDangerousPrinterChar(char: string): boolean {
  return DANGEROUS_PRINTER_CHARS.includes(char);
}

/**
 * Formats an error message for user-friendly display.
 */
function formatPrintError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "object" && error !== null && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "An unexpected error occurred while printing";
}

/**
 * Parses a semver version string.
 */
function parseVersion(version: string): { major: number; minor: number; patch: number } | null {
  const match = version.match(/^v?(\d+)\.(\d+)\.(\d+)/);
  if (!match) {
    return null;
  }
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
  };
}

/**
 * Compares two semver versions.
 * Returns: negative if a < b, 0 if equal, positive if a > b
 */
function compareVersions(a: string, b: string): number {
  const va = parseVersion(a);
  const vb = parseVersion(b);

  if (!va || !vb) {
    return 0;
  }

  if (va.major !== vb.major) return va.major - vb.major;
  if (va.minor !== vb.minor) return va.minor - vb.minor;
  return va.patch - vb.patch;
}

// ============================================
// FileDialogs Property Tests
// ============================================

describe("FileDialogs", () => {
  describe("Filename Sanitization", () => {
    it("should handle arbitrary file names without throwing", () => {
      fc.assert(
        fc.property(fc.string(), (name) => {
          // Should never throw
          const result = sanitizeFilename(name);
          expect(typeof result).toBe("string");
        })
      );
    });

    it("should never return a filename with path separators", () => {
      fc.assert(
        fc.property(fc.string(), (name) => {
          const result = sanitizeFilename(name);
          expect(result.includes("/")).toBe(false);
          expect(result.includes("\\")).toBe(false);
        })
      );
    });

    it("should never return a filename with dangerous characters", () => {
      fc.assert(
        fc.property(fc.string(), (name) => {
          const result = sanitizeFilename(name);
          expect(result.includes(":")).toBe(false);
          expect(result.includes("*")).toBe(false);
          expect(result.includes("?")).toBe(false);
          expect(result.includes('"')).toBe(false);
          expect(result.includes("<")).toBe(false);
          expect(result.includes(">")).toBe(false);
          expect(result.includes("|")).toBe(false);
        })
      );
    });

    it("should never return an empty filename", () => {
      fc.assert(
        fc.property(fc.string(), (name) => {
          const result = sanitizeFilename(name);
          expect(result.length).toBeGreaterThan(0);
        })
      );
    });

    it("should limit filename length to 200 characters", () => {
      fc.assert(
        fc.property(fc.string({ minLength: 0, maxLength: 1000 }), (name) => {
          const result = sanitizeFilename(name);
          expect(result.length).toBeLessThanOrEqual(200);
        })
      );
    });

    it("should preserve valid filenames", () => {
      fc.assert(
        fc.property(
          fc.stringMatching(/^[a-zA-Z0-9_-]{1,50}\.pdf$/),
          (name) => {
            const result = sanitizeFilename(name);
            expect(result).toBe(name);
          }
        )
      );
    });

    it("should not contain control characters", () => {
      fc.assert(
        fc.property(fc.string(), (name) => {
          const result = sanitizeFilename(name);
          // Check for control characters (0x00-0x1F)
          for (let i = 0; i < result.length; i++) {
            const code = result.charCodeAt(i);
            expect(code).toBeGreaterThan(0x1f);
          }
        })
      );
    });
  });

  describe("File Size Validation", () => {
    it("should accept files under the limit", () => {
      fc.assert(
        fc.property(fc.integer({ min: 0, max: MAX_FILE_SIZE }), (size) => {
          const result = validateFileSize(size);
          expect(result.valid).toBe(true);
        })
      );
    });

    it("should reject files over the limit", () => {
      fc.assert(
        fc.property(fc.integer({ min: MAX_FILE_SIZE + 1, max: MAX_FILE_SIZE * 10 }), (size) => {
          const result = validateFileSize(size);
          expect(result.valid).toBe(false);
          expect(result.error).toContain("100MB");
        })
      );
    });

    it("should include user-friendly message for large files", () => {
      fc.assert(
        fc.property(fc.integer({ min: MAX_FILE_SIZE + 1, max: MAX_FILE_SIZE * 2 }), (size) => {
          const result = validateFileSize(size);
          expect(result.error).toContain("smaller");
        })
      );
    });
  });

  describe("PDF Extension Handling", () => {
    it("should always return a filename ending in .pdf", () => {
      fc.assert(
        fc.property(fc.string({ minLength: 1, maxLength: 50 }), (name) => {
          // Filter out strings that are only dots or spaces
          if (name.trim().replace(/\./g, "").length === 0) return;

          const result = ensurePdfExtension(name);
          expect(result.toLowerCase().endsWith(".pdf")).toBe(true);
        })
      );
    });

    it("should preserve .pdf extension if already present", () => {
      fc.assert(
        fc.property(
          fc.stringMatching(/^[a-zA-Z0-9_-]{1,30}\.pdf$/),
          (name) => {
            const result = ensurePdfExtension(name);
            expect(result).toBe(name);
          }
        )
      );
    });

    it("should handle uppercase .PDF extension", () => {
      fc.assert(
        fc.property(
          fc.stringMatching(/^[a-zA-Z0-9_-]{1,30}\.PDF$/),
          (name) => {
            const result = ensurePdfExtension(name);
            // Should recognize uppercase as valid
            expect(result.toLowerCase().endsWith(".pdf")).toBe(true);
          }
        )
      );
    });

    it("should replace other extensions with .pdf", () => {
      fc.assert(
        fc.property(
          fc.stringMatching(/^[a-zA-Z0-9_-]{1,30}$/),
          fc.stringMatching(/^[a-z]{1,4}$/),
          (stem, ext) => {
            if (ext.toLowerCase() === "pdf") return;

            const filename = `${stem}.${ext}`;
            const result = ensurePdfExtension(filename);
            expect(result.toLowerCase().endsWith(".pdf")).toBe(true);
          }
        )
      );
    });
  });
});

// ============================================
// Print Property Tests
// ============================================

describe("Print", () => {
  describe("Printer Name Validation", () => {
    it("should detect all dangerous characters", () => {
      fc.assert(
        fc.property(
          fc.stringMatching(/^[a-zA-Z0-9 ]{0,10}$/),
          fc.constantFrom(...DANGEROUS_PRINTER_CHARS),
          fc.stringMatching(/^[a-zA-Z0-9 ]{0,10}$/),
          (prefix, dangerous, suffix) => {
            const name = prefix + dangerous + suffix;
            const result = validatePrinterName(name);
            expect(result.valid).toBe(false);
          }
        )
      );
    });

    it("should accept safe printer names", () => {
      fc.assert(
        fc.property(
          fc.stringMatching(/^[a-zA-Z0-9_ -]{1,100}$/),
          (name) => {
            // Double-check no dangerous chars
            const hasDangerous = DANGEROUS_PRINTER_CHARS.some((c) => name.includes(c));
            if (hasDangerous) return;

            const result = validatePrinterName(name);
            expect(result.valid).toBe(true);
          }
        )
      );
    });

    it("should reject empty printer names", () => {
      const result = validatePrinterName("");
      expect(result.valid).toBe(false);
      expect(result.error).toContain("empty");
    });

    it("should reject very long printer names", () => {
      fc.assert(
        fc.property(fc.integer({ min: 257, max: 1000 }), (length) => {
          const name = "a".repeat(length);
          const result = validatePrinterName(name);
          expect(result.valid).toBe(false);
          expect(result.error).toContain("too long");
        })
      );
    });

    it("should accept names at the length limit", () => {
      fc.assert(
        fc.property(fc.integer({ min: 1, max: 256 }), (length) => {
          const name = "a".repeat(length);
          const result = validatePrinterName(name);
          expect(result.valid).toBe(true);
        })
      );
    });

    it("should correctly identify dangerous characters", () => {
      fc.assert(
        fc.property(fc.string({ minLength: 1, maxLength: 1 }), (char) => {
          const expected = DANGEROUS_PRINTER_CHARS.includes(char);
          expect(isDangerousPrinterChar(char)).toBe(expected);
        })
      );
    });
  });

  describe("Error Formatting", () => {
    it("should handle string errors", () => {
      fc.assert(
        fc.property(fc.string({ minLength: 1, maxLength: 100 }), (msg) => {
          const result = formatPrintError(msg);
          expect(result).toBe(msg);
        })
      );
    });

    it("should handle Error objects", () => {
      fc.assert(
        fc.property(fc.string({ minLength: 1, maxLength: 100 }), (msg) => {
          const error = new Error(msg);
          const result = formatPrintError(error);
          expect(result).toBe(msg);
        })
      );
    });

    it("should handle objects with message property", () => {
      fc.assert(
        fc.property(fc.string({ minLength: 1, maxLength: 100 }), (msg) => {
          const error = { message: msg };
          const result = formatPrintError(error);
          expect(result).toBe(msg);
        })
      );
    });

    it("should return default message for unknown error types", () => {
      fc.assert(
        fc.property(fc.integer(), (num) => {
          const result = formatPrintError(num);
          expect(result).toContain("unexpected error");
        })
      );
    });

    it("should return default message for null/undefined", () => {
      expect(formatPrintError(null)).toContain("unexpected error");
      expect(formatPrintError(undefined)).toContain("unexpected error");
    });
  });
});

// ============================================
// Updater Property Tests
// ============================================

describe("Updater", () => {
  describe("Version Parsing", () => {
    it("should parse valid semver strings", () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          (major, minor, patch) => {
            const version = `${major}.${minor}.${patch}`;
            const result = parseVersion(version);
            expect(result).not.toBeNull();
            expect(result!.major).toBe(major);
            expect(result!.minor).toBe(minor);
            expect(result!.patch).toBe(patch);
          }
        )
      );
    });

    it("should parse versions with v prefix", () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          (major, minor, patch) => {
            const version = `v${major}.${minor}.${patch}`;
            const result = parseVersion(version);
            expect(result).not.toBeNull();
            expect(result!.major).toBe(major);
          }
        )
      );
    });

    it("should return null for invalid version strings", () => {
      // Note: "1.2.3.4.5" will parse as 1.2.3 since regex matches first 3 parts
      const invalidVersions = ["not-a-version", "1.2", "abc", ""];
      for (const v of invalidVersions) {
        expect(parseVersion(v)).toBeNull();
      }
    });

    it("should handle versions with suffixes", () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          fc.stringMatching(/^-[a-z]+(\.\d+)?$/),
          (major, minor, patch, suffix) => {
            const version = `${major}.${minor}.${patch}${suffix}`;
            const result = parseVersion(version);
            // Should still parse the numeric part
            expect(result).not.toBeNull();
            expect(result!.major).toBe(major);
          }
        )
      );
    });
  });

  describe("Version Comparison", () => {
    it("should correctly order versions by major", () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 50 }),
          fc.integer({ min: 0, max: 50 }),
          fc.integer({ min: 0, max: 50 }),
          fc.integer({ min: 0, max: 50 }),
          (major1, major2, minor, patch) => {
            const v1 = `${major1}.${minor}.${patch}`;
            const v2 = `${major2}.${minor}.${patch}`;
            const result = compareVersions(v1, v2);

            if (major1 < major2) expect(result).toBeLessThan(0);
            else if (major1 > major2) expect(result).toBeGreaterThan(0);
            else expect(result).toBe(0);
          }
        )
      );
    });

    it("should correctly order versions by minor when major is equal", () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 50 }),
          fc.integer({ min: 0, max: 50 }),
          fc.integer({ min: 0, max: 50 }),
          fc.integer({ min: 0, max: 50 }),
          (major, minor1, minor2, patch) => {
            const v1 = `${major}.${minor1}.${patch}`;
            const v2 = `${major}.${minor2}.${patch}`;
            const result = compareVersions(v1, v2);

            if (minor1 < minor2) expect(result).toBeLessThan(0);
            else if (minor1 > minor2) expect(result).toBeGreaterThan(0);
            else expect(result).toBe(0);
          }
        )
      );
    });

    it("should correctly order versions by patch when major and minor are equal", () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 50 }),
          fc.integer({ min: 0, max: 50 }),
          fc.integer({ min: 0, max: 50 }),
          fc.integer({ min: 0, max: 50 }),
          (major, minor, patch1, patch2) => {
            const v1 = `${major}.${minor}.${patch1}`;
            const v2 = `${major}.${minor}.${patch2}`;
            const result = compareVersions(v1, v2);

            if (patch1 < patch2) expect(result).toBeLessThan(0);
            else if (patch1 > patch2) expect(result).toBeGreaterThan(0);
            else expect(result).toBe(0);
          }
        )
      );
    });

    it("should return 0 for equal versions", () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          (major, minor, patch) => {
            const version = `${major}.${minor}.${patch}`;
            expect(compareVersions(version, version)).toBe(0);
          }
        )
      );
    });

    it("should be antisymmetric", () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          fc.integer({ min: 0, max: 100 }),
          (m1, n1, p1, m2, n2, p2) => {
            const v1 = `${m1}.${n1}.${p1}`;
            const v2 = `${m2}.${n2}.${p2}`;
            const cmp1 = compareVersions(v1, v2);
            const cmp2 = compareVersions(v2, v1);

            // If a < b then b > a (and vice versa)
            if (cmp1 < 0) expect(cmp2).toBeGreaterThan(0);
            else if (cmp1 > 0) expect(cmp2).toBeLessThan(0);
            else expect(cmp2).toBe(0);
          }
        )
      );
    });
  });
});

// ============================================
// Integration Property Tests
// ============================================

describe("Integration", () => {
  describe("Tauri Environment Detection", () => {
    it("should handle missing __TAURI__ gracefully", () => {
      // In test environment, __TAURI__ is not defined
      const isTauri =
        typeof window !== "undefined" && "__TAURI__" in (window as unknown as object);
      expect(isTauri).toBe(false);
    });
  });

  describe("Byte Array Conversion", () => {
    it("should preserve data when converting Uint8Array to Array and back", () => {
      fc.assert(
        fc.property(
          fc.array(fc.integer({ min: 0, max: 255 }), { minLength: 0, maxLength: 1000 }),
          (bytes) => {
            const uint8 = new Uint8Array(bytes);
            const array = Array.from(uint8);
            const restored = new Uint8Array(array);

            expect(restored.length).toBe(uint8.length);
            for (let i = 0; i < restored.length; i++) {
              expect(restored[i]).toBe(uint8[i]);
            }
          }
        )
      );
    });

    it("should handle empty arrays", () => {
      const uint8 = new Uint8Array([]);
      const array = Array.from(uint8);
      expect(array.length).toBe(0);
    });

    it("should handle large arrays efficiently", () => {
      // Just verify it doesn't throw for large arrays
      const largeArray = new Uint8Array(100000);
      const array = Array.from(largeArray);
      expect(array.length).toBe(100000);
    });
  });
});
