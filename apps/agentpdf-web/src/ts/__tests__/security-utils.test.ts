/**
 * Tests for Security Utilities
 *
 * These tests verify the security-related utilities:
 * - XSS prevention
 * - Input validation (session params, email, file names)
 * - Size limit enforcement
 * - Rate limit feedback
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock the DOM for escapeHtml
const mockCreateElement = vi.fn(() => {
  let textContent = '';
  return {
    set textContent(value: string) {
      textContent = value;
    },
    get textContent() {
      return textContent;
    },
    get innerHTML() {
      // Simulate browser's HTML escaping
      return textContent
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#039;');
    },
  };
});

vi.stubGlobal('document', {
  createElement: mockCreateElement,
});

// Import after mocks are set up
import {
  escapeHtml,
  validateSessionParams,
  validateEmail,
  validateFileName,
  validateFileSize,
  SIZE_LIMITS,
  getRateLimitStatus,
  getRateLimitMessage,
  type ValidationResult,
  type RateLimitInfo,
  type RateLimitStatus,
} from '../security-utils';

describe('XSS Prevention', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('escapeHtml', () => {
    it('should escape HTML tags', () => {
      const result = escapeHtml('<script>alert("xss")</script>');
      expect(result).not.toContain('<script>');
      expect(result).toContain('&lt;script&gt;');
    });

    it('should escape angle brackets', () => {
      const result = escapeHtml('<div>content</div>');
      expect(result).toContain('&lt;');
      expect(result).toContain('&gt;');
    });

    it('should escape ampersands', () => {
      const result = escapeHtml('foo & bar');
      expect(result).toContain('&amp;');
    });

    it('should escape double quotes', () => {
      const result = escapeHtml('Hello "World"');
      expect(result).toContain('&quot;');
    });

    it('should escape single quotes', () => {
      const result = escapeHtml("Hello 'World'");
      expect(result).toContain('&#039;');
    });

    it('should handle empty strings', () => {
      const result = escapeHtml('');
      expect(result).toBe('');
    });

    it('should handle plain text without modification', () => {
      const plainText = 'Hello World 123';
      const result = escapeHtml(plainText);
      expect(result).toBe(plainText);
    });

    it('should escape nested script tags', () => {
      const malicious = '<script><script>alert(1)</script></script>';
      const result = escapeHtml(malicious);
      expect(result).not.toContain('<script>');
    });

    it('should escape event handlers in attributes', () => {
      const result = escapeHtml('<img src="x" onerror="alert(1)">');
      // The entire string is escaped, so < becomes &lt; preventing execution
      expect(result).not.toContain('<img');
      expect(result).toContain('&lt;img');
    });

    it('should escape javascript: URLs', () => {
      const result = escapeHtml('<a href="javascript:alert(1)">click</a>');
      expect(result).not.toContain('<a');
      expect(result).toContain('&lt;a');
    });

    it('should escape SVG-based XSS', () => {
      const svg = '<svg onload="alert(1)"></svg>';
      const result = escapeHtml(svg);
      expect(result).not.toContain('<svg');
      expect(result).toContain('&lt;svg');
    });

    it('should handle unicode characters correctly', () => {
      const unicode = 'Hello \u4e2d\u6587 World';
      const result = escapeHtml(unicode);
      expect(result).toBe(unicode);
    });
  });
});

describe('Session Parameter Validation', () => {
  describe('validateSessionParams', () => {
    it('should accept valid session params', () => {
      const result = validateSessionParams({
        sessionId: 'valid-session-id-12345',
        recipientId: 'valid-recipient-id',
        signingKey: 'signing-key-that-is-long-enough',
      });
      expect(result.valid).toBe(true);
      expect(result.error).toBeUndefined();
    });

    it('should reject missing sessionId', () => {
      const result = validateSessionParams({
        recipientId: 'valid-recipient',
        signingKey: 'valid-signing-key-12345',
      });
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Session ID');
    });

    it('should reject empty sessionId', () => {
      const result = validateSessionParams({
        sessionId: '',
        recipientId: 'valid-recipient',
        signingKey: 'valid-signing-key-12345',
      });
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Session ID');
    });

    it('should reject whitespace-only sessionId', () => {
      const result = validateSessionParams({
        sessionId: '   ',
        recipientId: 'valid-recipient',
        signingKey: 'valid-signing-key-12345',
      });
      expect(result.valid).toBe(false);
    });

    it('should reject short sessionId', () => {
      const result = validateSessionParams({
        sessionId: 'short',
        recipientId: 'valid-recipient',
        signingKey: 'valid-signing-key-12345',
      });
      expect(result.valid).toBe(false);
      expect(result.error).toContain('at least 8 characters');
    });

    it('should reject missing recipientId', () => {
      const result = validateSessionParams({
        sessionId: 'valid-session-id',
        signingKey: 'valid-signing-key-12345',
      });
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Recipient ID');
    });

    it('should reject short recipientId', () => {
      const result = validateSessionParams({
        sessionId: 'valid-session-id',
        recipientId: 'short',
        signingKey: 'valid-signing-key-12345',
      });
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Recipient ID');
    });

    it('should reject missing signingKey', () => {
      const result = validateSessionParams({
        sessionId: 'valid-session-id',
        recipientId: 'valid-recipient-id',
      });
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Signing key');
    });

    it('should reject short signingKey', () => {
      const result = validateSessionParams({
        sessionId: 'valid-session-id',
        recipientId: 'valid-recipient',
        signingKey: 'short-key',
      });
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Signing key');
      expect(result.error).toContain('16 characters');
    });

    it('should reject empty object', () => {
      const result = validateSessionParams({});
      expect(result.valid).toBe(false);
    });
  });
});

describe('Email Validation', () => {
  describe('validateEmail', () => {
    it('should accept valid email addresses', () => {
      const validEmails = [
        'user@example.com',
        'user.name@example.com',
        'user+tag@example.com',
        'user@subdomain.example.com',
        'user123@example.co.uk',
      ];

      for (const email of validEmails) {
        const result = validateEmail(email);
        expect(result.valid, `${email} should be valid`).toBe(true);
      }
    });

    it('should reject missing email', () => {
      // @ts-expect-error testing undefined
      const result = validateEmail(undefined);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('required');
    });

    it('should reject empty email', () => {
      const result = validateEmail('');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('required');
    });

    it('should reject whitespace-only email', () => {
      const result = validateEmail('   ');
      expect(result.valid).toBe(false);
    });

    it('should reject email without @', () => {
      const result = validateEmail('userexample.com');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Invalid email format');
    });

    it('should reject email without domain', () => {
      const result = validateEmail('user@');
      expect(result.valid).toBe(false);
    });

    it('should reject email without local part', () => {
      const result = validateEmail('@example.com');
      expect(result.valid).toBe(false);
    });

    it('should reject email without TLD', () => {
      const result = validateEmail('user@example');
      expect(result.valid).toBe(false);
    });

    it('should reject email with spaces', () => {
      const result = validateEmail('user @example.com');
      expect(result.valid).toBe(false);
    });

    it('should reject email exceeding max length', () => {
      const longLocal = 'a'.repeat(250);
      const result = validateEmail(`${longLocal}@example.com`);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('254 characters');
    });

    it('should trim whitespace before validation', () => {
      const result = validateEmail('  user@example.com  ');
      expect(result.valid).toBe(true);
    });
  });
});

describe('File Name Validation', () => {
  describe('validateFileName', () => {
    it('should accept valid file names', () => {
      const validNames = [
        'document.pdf',
        'my-file_2024.pdf',
        'Contract (Final).pdf',
        'file.name.with.dots.pdf',
      ];

      for (const name of validNames) {
        const result = validateFileName(name);
        expect(result.valid, `${name} should be valid`).toBe(true);
      }
    });

    it('should reject missing file name', () => {
      // @ts-expect-error testing undefined
      const result = validateFileName(undefined);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('required');
    });

    it('should reject empty file name', () => {
      const result = validateFileName('');
      expect(result.valid).toBe(false);
    });

    it('should reject path traversal with ..', () => {
      const maliciousNames = [
        '../etc/passwd',
        '..\\windows\\system32',
        'foo/../bar',
        'foo/..\\bar',
        '...',
      ];

      for (const name of maliciousNames) {
        const result = validateFileName(name);
        // Either contains .. or contains path separators
        expect(result.valid, `${name} should be invalid`).toBe(false);
      }
    });

    it('should reject absolute paths starting with /', () => {
      const result = validateFileName('/etc/passwd');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('absolute path');
    });

    it('should reject absolute paths starting with \\', () => {
      const result = validateFileName('\\windows\\system32');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('absolute path');
    });

    it('should reject Windows drive letters', () => {
      const result = validateFileName('C:\\Users\\file.pdf');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('drive letter');
    });

    it('should reject null bytes', () => {
      const result = validateFileName('file\0.pdf');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('null bytes');
    });

    it('should reject directory separators', () => {
      const names = ['path/to/file.pdf', 'path\\to\\file.pdf'];

      for (const name of names) {
        const result = validateFileName(name);
        expect(result.valid, `${name} should be invalid`).toBe(false);
        expect(result.error).toContain('directory separator');
      }
    });

    it('should reject dangerous characters', () => {
      const dangerous = ['<', '>', ':', '"', '|', '?', '*'];

      for (const char of dangerous) {
        const result = validateFileName(`file${char}name.pdf`);
        expect(result.valid, `${char} should be rejected`).toBe(false);
        expect(result.error).toContain(char);
      }
    });

    it('should reject file names exceeding max length', () => {
      const longName = 'a'.repeat(201) + '.pdf';
      const result = validateFileName(longName);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('200 characters');
    });

    it('should accept file names at max length', () => {
      const maxName = 'a'.repeat(196) + '.pdf'; // 200 chars
      const result = validateFileName(maxName);
      expect(result.valid).toBe(true);
    });
  });
});

describe('Size Limits', () => {
  describe('SIZE_LIMITS constants', () => {
    it('should have correct MAX_PDF_SIZE', () => {
      expect(SIZE_LIMITS.MAX_PDF_SIZE).toBe(50 * 1024 * 1024);
    });

    it('should have correct MAX_SIGNATURE_SIZE', () => {
      expect(SIZE_LIMITS.MAX_SIGNATURE_SIZE).toBe(100 * 1024);
    });

    it('should have correct MAX_FILENAME_LENGTH', () => {
      expect(SIZE_LIMITS.MAX_FILENAME_LENGTH).toBe(200);
    });

    it('should have correct MAX_EMAIL_LENGTH', () => {
      expect(SIZE_LIMITS.MAX_EMAIL_LENGTH).toBe(254);
    });

    it('should have correct MAX_RECIPIENTS_PER_REQUEST', () => {
      expect(SIZE_LIMITS.MAX_RECIPIENTS_PER_REQUEST).toBe(10);
    });
  });

  describe('validateFileSize', () => {
    it('should accept valid file sizes', () => {
      const validSizes = [1, 1024, 1024 * 1024, 10 * 1024 * 1024];

      for (const size of validSizes) {
        const result = validateFileSize(size);
        expect(result.valid, `${size} should be valid`).toBe(true);
      }
    });

    it('should accept file at exactly max size', () => {
      const result = validateFileSize(SIZE_LIMITS.MAX_PDF_SIZE);
      expect(result.valid).toBe(true);
    });

    it('should reject file exceeding max size', () => {
      const result = validateFileSize(SIZE_LIMITS.MAX_PDF_SIZE + 1);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('exceeds maximum');
    });

    it('should reject negative file sizes', () => {
      const result = validateFileSize(-1);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Invalid file size');
    });

    it('should reject zero file size', () => {
      const result = validateFileSize(0);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('empty');
    });

    it('should use custom max size when provided', () => {
      const customMax = 1024;
      const result = validateFileSize(2048, customMax);
      expect(result.valid).toBe(false);
    });

    it('should accept file under custom max size', () => {
      const customMax = 1024;
      const result = validateFileSize(512, customMax);
      expect(result.valid).toBe(true);
    });

    it('should show size in MB in error message', () => {
      const result = validateFileSize(100 * 1024 * 1024);
      expect(result.error).toContain('MB');
    });
  });
});

describe('Rate Limit Feedback', () => {
  describe('getRateLimitStatus', () => {
    it('should return "ok" when plenty of quota remains', () => {
      const info: RateLimitInfo = {
        remainingDaily: 80,
        remainingMonthly: 800,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      expect(getRateLimitStatus(info)).toBe('ok');
    });

    it('should return "warning" when 10-50% remains', () => {
      const info: RateLimitInfo = {
        remainingDaily: 30,
        remainingMonthly: 300,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      expect(getRateLimitStatus(info)).toBe('warning');
    });

    it('should return "critical" when less than 10% remains', () => {
      const info: RateLimitInfo = {
        remainingDaily: 5,
        remainingMonthly: 50,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      expect(getRateLimitStatus(info)).toBe('critical');
    });

    it('should return "exceeded" when daily limit reached', () => {
      const info: RateLimitInfo = {
        remainingDaily: 0,
        remainingMonthly: 500,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      expect(getRateLimitStatus(info)).toBe('exceeded');
    });

    it('should return "exceeded" when monthly limit reached', () => {
      const info: RateLimitInfo = {
        remainingDaily: 50,
        remainingMonthly: 0,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      expect(getRateLimitStatus(info)).toBe('exceeded');
    });

    it('should use lowest percentage between daily and monthly', () => {
      // Daily has 60% left, Monthly has 8% left -> critical
      const info: RateLimitInfo = {
        remainingDaily: 60,
        remainingMonthly: 80,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      expect(getRateLimitStatus(info)).toBe('critical');
    });

    it('should handle zero limits gracefully', () => {
      const info: RateLimitInfo = {
        remainingDaily: 0,
        remainingMonthly: 0,
        limitDaily: 0,
        limitMonthly: 0,
      };
      expect(getRateLimitStatus(info)).toBe('exceeded');
    });

    it('should handle negative remaining gracefully', () => {
      const info: RateLimitInfo = {
        remainingDaily: -5,
        remainingMonthly: 100,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      expect(getRateLimitStatus(info)).toBe('exceeded');
    });
  });

  describe('getRateLimitMessage', () => {
    it('should return ok message with both limits', () => {
      const info: RateLimitInfo = {
        remainingDaily: 80,
        remainingMonthly: 800,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      const message = getRateLimitMessage(info);
      expect(message).toContain('80');
      expect(message).toContain('800');
      expect(message).toContain('daily');
      expect(message).toContain('monthly');
    });

    it('should mention daily limit when it is exceeded', () => {
      const info: RateLimitInfo = {
        remainingDaily: 0,
        remainingMonthly: 500,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      const message = getRateLimitMessage(info);
      expect(message).toContain('Daily limit reached');
      expect(message).toContain('midnight');
    });

    it('should mention monthly limit when it is exceeded', () => {
      const info: RateLimitInfo = {
        remainingDaily: 50,
        remainingMonthly: 0,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      const message = getRateLimitMessage(info);
      expect(message).toContain('Monthly limit reached');
      expect(message).toContain('next month');
    });

    it('should show warning for critical status', () => {
      const info: RateLimitInfo = {
        remainingDaily: 5,
        remainingMonthly: 50,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      const message = getRateLimitMessage(info);
      expect(message).toContain('Warning');
      expect(message).toContain('Only');
    });

    it('should show daily remaining when daily is lower', () => {
      const info: RateLimitInfo = {
        remainingDaily: 20,
        remainingMonthly: 800,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      const message = getRateLimitMessage(info);
      expect(message).toContain('20');
      expect(message).toContain('100');
      expect(message).toContain('daily');
    });

    it('should show monthly remaining when monthly is lower', () => {
      const info: RateLimitInfo = {
        remainingDaily: 80,
        remainingMonthly: 200,
        limitDaily: 100,
        limitMonthly: 1000,
      };
      const message = getRateLimitMessage(info);
      expect(message).toContain('200');
      expect(message).toContain('1000');
      expect(message).toContain('monthly');
    });
  });
});

describe('Security Utils Integration', () => {
  it('should properly combine validation for a complete workflow', () => {
    // Validate session params
    const sessionResult = validateSessionParams({
      sessionId: 'session-12345678',
      recipientId: 'recipient-12345678',
      signingKey: 'signing-key-1234567890123456',
    });
    expect(sessionResult.valid).toBe(true);

    // Validate email
    const emailResult = validateEmail('user@example.com');
    expect(emailResult.valid).toBe(true);

    // Validate file name
    const fileResult = validateFileName('contract.pdf');
    expect(fileResult.valid).toBe(true);

    // Validate file size
    const sizeResult = validateFileSize(1024 * 1024);
    expect(sizeResult.valid).toBe(true);

    // Check rate limit
    const rateLimit: RateLimitInfo = {
      remainingDaily: 50,
      remainingMonthly: 500,
      limitDaily: 100,
      limitMonthly: 1000,
    };
    expect(getRateLimitStatus(rateLimit)).toBe('ok');
  });

  it('should reject malicious input at every layer', () => {
    // XSS attempt
    const xssInput = '<script>alert("xss")</script>';
    const escaped = escapeHtml(xssInput);
    expect(escaped).not.toContain('<script>');

    // Path traversal in file name
    const fileResult = validateFileName('../../../etc/passwd');
    expect(fileResult.valid).toBe(false);

    // Oversized file
    const sizeResult = validateFileSize(100 * 1024 * 1024);
    expect(sizeResult.valid).toBe(false);
  });
});
