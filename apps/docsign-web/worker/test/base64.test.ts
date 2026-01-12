/**
 * Base64 Encoding/Decoding Tests
 *
 * Bug #14 Fix: Tests for large file base64 encoding
 *
 * Problem: btoa(String.fromCharCode(...largeArray)) causes
 * "Maximum call stack size exceeded" on large PDFs.
 *
 * Solution: Use native Uint8Array.toBase64() (Sept 2025+) with
 * iterative fallback for older browsers.
 *
 * These tests verify the encoding works for:
 * 1. Small arrays (basic functionality)
 * 2. Large arrays (no stack overflow)
 * 3. Roundtrip encode/decode (data integrity)
 */

import { describe, it, expect } from "vitest";

/**
 * SOTA base64 encoding for Uint8Array
 * Uses native toBase64() if available, falls back to iterative loop
 */
function uint8ArrayToBase64(bytes: Uint8Array): string {
  // Use native method if available (Sept 2025+)
  if (typeof (bytes as any).toBase64 === 'function') {
    return (bytes as any).toBase64();
  }
  // Fallback: iterative loop (safe for any size, no stack overflow)
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

/**
 * SOTA base64 decoding to Uint8Array
 * Uses native fromBase64() if available, falls back to iterative loop
 */
function base64ToUint8Array(base64: string): Uint8Array {
  // Use native method if available (Sept 2025+)
  if (typeof (Uint8Array as any).fromBase64 === 'function') {
    return (Uint8Array as any).fromBase64(base64);
  }
  // Fallback: iterative decode
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

describe("Base64 Encoding/Decoding", () => {
  describe("uint8ArrayToBase64", () => {
    it("should encode small arrays correctly", () => {
      const input = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"
      const result = uint8ArrayToBase64(input);
      expect(result).toBe("SGVsbG8=");
    });

    it("should encode empty array", () => {
      const input = new Uint8Array([]);
      const result = uint8ArrayToBase64(input);
      expect(result).toBe("");
    });

    it("should encode binary data with all byte values", () => {
      // Test with bytes 0-255
      const input = new Uint8Array(256);
      for (let i = 0; i < 256; i++) {
        input[i] = i;
      }
      const result = uint8ArrayToBase64(input);
      // Verify it's valid base64
      expect(result).toMatch(/^[A-Za-z0-9+/]+=*$/);
      expect(result.length).toBeGreaterThan(0);
    });

    /**
     * Critical test: Large arrays should NOT cause stack overflow
     * This was the bug - btoa(String.fromCharCode(...largeArray)) fails
     */
    it("should handle large arrays without stack overflow (100KB)", () => {
      // 100KB simulates a small PDF
      const size = 100 * 1024;
      const input = new Uint8Array(size);
      for (let i = 0; i < size; i++) {
        input[i] = i % 256;
      }

      // This should NOT throw "Maximum call stack size exceeded"
      expect(() => uint8ArrayToBase64(input)).not.toThrow();

      const result = uint8ArrayToBase64(input);
      expect(result.length).toBeGreaterThan(0);
    });

    it("should handle large arrays without stack overflow (1MB)", () => {
      // 1MB simulates a typical PDF
      const size = 1024 * 1024;
      const input = new Uint8Array(size);
      for (let i = 0; i < size; i++) {
        input[i] = i % 256;
      }

      expect(() => uint8ArrayToBase64(input)).not.toThrow();

      const result = uint8ArrayToBase64(input);
      expect(result.length).toBeGreaterThan(0);
    });

    it("should handle large arrays without stack overflow (5MB)", () => {
      // 5MB simulates a larger PDF
      const size = 5 * 1024 * 1024;
      const input = new Uint8Array(size);
      for (let i = 0; i < size; i++) {
        input[i] = i % 256;
      }

      expect(() => uint8ArrayToBase64(input)).not.toThrow();

      const result = uint8ArrayToBase64(input);
      expect(result.length).toBeGreaterThan(0);
    });
  });

  describe("base64ToUint8Array", () => {
    it("should decode small base64 correctly", () => {
      const input = "SGVsbG8="; // "Hello"
      const result = base64ToUint8Array(input);
      expect(Array.from(result)).toEqual([72, 101, 108, 108, 111]);
    });

    it("should decode empty string", () => {
      const result = base64ToUint8Array("");
      expect(result.length).toBe(0);
    });
  });

  describe("Roundtrip encode/decode", () => {
    it("should roundtrip small data correctly", () => {
      const original = new Uint8Array([1, 2, 3, 4, 5, 255, 0, 128]);
      const encoded = uint8ArrayToBase64(original);
      const decoded = base64ToUint8Array(encoded);
      expect(Array.from(decoded)).toEqual(Array.from(original));
    });

    it("should roundtrip PDF-like header correctly", () => {
      // PDF magic bytes: %PDF-1.4
      const pdfHeader = new Uint8Array([0x25, 0x50, 0x44, 0x46, 0x2D, 0x31, 0x2E, 0x34]);
      const encoded = uint8ArrayToBase64(pdfHeader);
      const decoded = base64ToUint8Array(encoded);
      expect(Array.from(decoded)).toEqual(Array.from(pdfHeader));
    });

    it("should roundtrip large data correctly (100KB)", () => {
      const size = 100 * 1024;
      const original = new Uint8Array(size);
      for (let i = 0; i < size; i++) {
        original[i] = i % 256;
      }

      const encoded = uint8ArrayToBase64(original);
      const decoded = base64ToUint8Array(encoded);

      expect(decoded.length).toBe(original.length);
      // Verify first and last bytes
      expect(decoded[0]).toBe(original[0]);
      expect(decoded[size - 1]).toBe(original[size - 1]);
      // Verify middle byte
      expect(decoded[Math.floor(size / 2)]).toBe(original[Math.floor(size / 2)]);
    });

    it("should roundtrip large data correctly (1MB)", () => {
      const size = 1024 * 1024;
      const original = new Uint8Array(size);
      for (let i = 0; i < size; i++) {
        original[i] = i % 256;
      }

      const encoded = uint8ArrayToBase64(original);
      const decoded = base64ToUint8Array(encoded);

      expect(decoded.length).toBe(original.length);
      expect(Array.from(decoded.slice(0, 100))).toEqual(Array.from(original.slice(0, 100)));
    });
  });
});
