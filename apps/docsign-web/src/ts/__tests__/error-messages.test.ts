/**
 * Tests for Geriatric-Friendly Error Messages
 *
 * Tests the error categorization and user-friendly message generation.
 * Uses property-based testing with fast-check to ensure robustness.
 */

import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import {
  getUserFriendlyError,
  categorizeError,
  createUserError,
  getOfflineError,
  getFileTooLargeError,
  getUnsupportedFileError,
  type UserError,
  type ErrorCategory,
  type ErrorIcon,
} from '../error-messages';

// ============================================================
// Error Categorization Tests
// ============================================================

describe('Error Categorization', () => {
  describe('Network Errors', () => {
    const networkPatterns = [
      'network error',
      'Network Error',
      'NETWORK_ERROR',
      'Failed to fetch',
      'fetch failed',
      'offline',
      'connection refused',
      'internet connection',
      'failed to load',
      'Failed to load resource',
      'timeout',
      'request timeout',
      'ECONNREFUSED',
      'ENOTFOUND',
      'ERR_NETWORK',
      'net::ERR_INTERNET_DISCONNECTED',
    ];

    it('categorizes known network error patterns correctly', () => {
      networkPatterns.forEach((pattern) => {
        const category = categorizeError(new Error(pattern));
        expect(category).toBe('network');
      });
    });

    it('Property: Network error messages include "wifi-off" icon', () => {
      fc.assert(
        fc.property(fc.constantFrom(...networkPatterns), (pattern) => {
          const error = getUserFriendlyError(new Error(pattern));
          expect(error.icon).toBe('wifi-off');
          expect(error.title).toContain('Connection');
        })
      );
    });
  });

  describe('Password Protected PDF Errors', () => {
    const passwordPatterns = [
      'password required',
      'Password required',
      'encrypted PDF',
      'document is encrypted',
      'protected document',
      'decrypt failed',
      'file is locked',
      'access denied to PDF',
    ];

    it('categorizes known password error patterns correctly', () => {
      passwordPatterns.forEach((pattern) => {
        const category = categorizeError(new Error(pattern));
        expect(category).toBe('password-protected');
      });
    });

    it('Property: Password error messages include "lock" icon', () => {
      fc.assert(
        fc.property(fc.constantFrom(...passwordPatterns), (pattern) => {
          const error = getUserFriendlyError(new Error(pattern));
          expect(error.icon).toBe('lock');
          expect(error.title.toLowerCase()).toContain('password');
        })
      );
    });
  });

  describe('Signature Invalid Errors', () => {
    const signaturePatterns = [
      'signature is invalid',
      'invalid signature',
      'signature failed',
      'failed to apply signature',
      'signature error',
      'sign failed',
      'could not sign document',
      'signing error occurred',
    ];

    it('categorizes known signature error patterns correctly', () => {
      signaturePatterns.forEach((pattern) => {
        const category = categorizeError(new Error(pattern));
        expect(category).toBe('signature-invalid');
      });
    });

    it('Property: Signature error messages include "signature" icon', () => {
      fc.assert(
        fc.property(fc.constantFrom(...signaturePatterns), (pattern) => {
          const error = getUserFriendlyError(new Error(pattern));
          expect(error.icon).toBe('signature');
          expect(error.title.toLowerCase()).toContain('signature');
        })
      );
    });
  });

  describe('Session Expired Errors', () => {
    const sessionPatterns = [
      'session expired',
      'expired session',
      'link expired',
      'expired link',
      'session not found',
      'invalid session',
      '401 Unauthorized',
      '403 Forbidden',
      'unauthorized access',
    ];

    it('categorizes known session error patterns correctly', () => {
      sessionPatterns.forEach((pattern) => {
        const category = categorizeError(new Error(pattern));
        expect(category).toBe('session-expired');
      });
    });

    it('Property: Session error messages include "clock" icon', () => {
      fc.assert(
        fc.property(fc.constantFrom(...sessionPatterns), (pattern) => {
          const error = getUserFriendlyError(new Error(pattern));
          expect(error.icon).toBe('clock');
          expect(error.title.toLowerCase()).toContain('expir');
        })
      );
    });
  });

  describe('File Corrupt Errors', () => {
    const corruptPatterns = [
      'corrupted file',
      'file is corrupt',
      'invalid PDF',
      'PDF is invalid',
      'malformed document',
      'cannot read file',
      'parse error',
      'invalid document format',
      'damaged file',
    ];

    it('categorizes known corrupt file error patterns correctly', () => {
      corruptPatterns.forEach((pattern) => {
        const category = categorizeError(new Error(pattern));
        expect(category).toBe('file-corrupt');
      });
    });

    it('Property: File corrupt error messages include "file" icon', () => {
      fc.assert(
        fc.property(fc.constantFrom(...corruptPatterns), (pattern) => {
          const error = getUserFriendlyError(new Error(pattern));
          expect(error.icon).toBe('file');
          expect(error.title.toLowerCase()).toContain('document');
        })
      );
    });
  });

  describe('Authentication Errors', () => {
    const authPatterns = [
      'authentication failed',
      'login required',
      'invalid credentials',
      'identity verification failed',
      'verification failed for user',
    ];

    it('categorizes known authentication error patterns correctly', () => {
      authPatterns.forEach((pattern) => {
        const category = categorizeError(new Error(pattern));
        expect(category).toBe('authentication');
      });
    });

    it('Property: Auth error messages include "user" icon', () => {
      fc.assert(
        fc.property(fc.constantFrom(...authPatterns), (pattern) => {
          const error = getUserFriendlyError(new Error(pattern));
          expect(error.icon).toBe('user');
          const titleLower = error.title.toLowerCase();
          expect(titleLower.includes('identity') || titleLower.includes('verification')).toBe(true);
        })
      );
    });
  });

  describe('Generic/Unknown Errors', () => {
    const genericPatterns = [
      'unknown error',
      'something went wrong',
      'unexpected error',
      'internal error',
      'oops',
      'error code 500',
      'null reference exception',
    ];

    it('categorizes unknown error patterns as generic', () => {
      genericPatterns.forEach((pattern) => {
        const category = categorizeError(new Error(pattern));
        expect(category).toBe('generic');
      });
    });

    it('Property: Generic error messages include "alert" icon', () => {
      fc.assert(
        fc.property(fc.constantFrom(...genericPatterns), (pattern) => {
          const error = getUserFriendlyError(new Error(pattern));
          expect(error.icon).toBe('alert');
          expect(error.title).toBe('Something Went Wrong');
        })
      );
    });
  });
});

// ============================================================
// UserError Structure Tests
// ============================================================

describe('UserError Structure', () => {
  it('Property: All errors have required fields', () => {
    const errorMessages = [
      'network error',
      'password required',
      'signature invalid',
      'session expired',
      'file corrupt',
      'auth failed',
      'unknown error',
    ];

    fc.assert(
      fc.property(fc.constantFrom(...errorMessages), (message) => {
        const error = getUserFriendlyError(new Error(message));

        // All required fields present
        expect(error.title).toBeDefined();
        expect(error.message).toBeDefined();
        expect(error.action).toBeDefined();
        expect(error.icon).toBeDefined();

        // Title is non-empty and reasonable length
        expect(error.title.length).toBeGreaterThan(0);
        expect(error.title.length).toBeLessThan(100);

        // Message is non-empty and provides detail
        expect(error.message.length).toBeGreaterThan(20);

        // Action is a short button label
        expect(error.action.length).toBeGreaterThan(0);
        expect(error.action.length).toBeLessThan(30);

        // Icon is a valid type
        const validIcons: ErrorIcon[] = [
          'wifi-off',
          'lock',
          'signature',
          'clock',
          'file',
          'alert',
          'user',
        ];
        expect(validIcons).toContain(error.icon);
      })
    );
  });

  it('Property: Error messages use plain language (no technical jargon)', () => {
    const technicalTerms = [
      'exception',
      'null',
      'undefined',
      'stack',
      'trace',
      'debug',
      'http',
      'api',
      'json',
      'parse',
      'syntax',
      'runtime',
      'module',
      'import',
      'export',
      'async',
      'promise',
      'callback',
    ];

    const errorMessages = [
      'network error',
      'password required',
      'signature invalid',
      'session expired',
      'file corrupt',
      'unknown error',
    ];

    errorMessages.forEach((msg) => {
      const error = getUserFriendlyError(new Error(msg));
      const combinedText = `${error.title} ${error.message} ${error.action}`.toLowerCase();

      technicalTerms.forEach((term) => {
        expect(combinedText).not.toContain(term);
      });
    });
  });

  it('Property: Error messages are reassuring and not alarming', () => {
    const alarmingPhrases = [
      'fatal',
      'critical',
      'severe',
      'disaster',
      'catastrophic',
      'danger',
      'emergency',
      'urgent',
      'panic',
      'crash',
      'destroyed',
      'lost forever',
    ];

    const errorMessages = [
      'network error',
      'password required',
      'signature invalid',
      'session expired',
      'file corrupt',
      'unknown error',
    ];

    errorMessages.forEach((msg) => {
      const error = getUserFriendlyError(new Error(msg));
      const combinedText = `${error.title} ${error.message}`.toLowerCase();

      alarmingPhrases.forEach((phrase) => {
        expect(combinedText).not.toContain(phrase);
      });
    });
  });

  it('Property: Error messages mention document safety where appropriate', () => {
    const safetyMessages = ['network error', 'unknown error'];

    safetyMessages.forEach((msg) => {
      const error = getUserFriendlyError(new Error(msg));
      expect(error.message.toLowerCase()).toContain('safe');
    });
  });
});

// ============================================================
// String vs Error Input Tests
// ============================================================

describe('String vs Error Input', () => {
  it('handles Error objects correctly', () => {
    const error = new Error('network connection failed');
    const result = getUserFriendlyError(error);
    expect(result.icon).toBe('wifi-off');
  });

  it('handles string input correctly', () => {
    const result = getUserFriendlyError('network connection failed');
    expect(result.icon).toBe('wifi-off');
  });

  it('Property: Same result for Error and string with same message', () => {
    const messages = [
      'network error',
      'password required',
      'signature invalid',
      'session expired',
    ];

    messages.forEach((msg) => {
      const fromError = getUserFriendlyError(new Error(msg));
      const fromString = getUserFriendlyError(msg);
      expect(fromError).toEqual(fromString);
    });
  });
});

// ============================================================
// Helper Function Tests
// ============================================================

describe('createUserError', () => {
  it('creates custom UserError with all fields', () => {
    const error = createUserError(
      'Custom Title',
      'Custom message for the user.',
      'Custom Action',
      'file'
    );

    expect(error.title).toBe('Custom Title');
    expect(error.message).toBe('Custom message for the user.');
    expect(error.action).toBe('Custom Action');
    expect(error.icon).toBe('file');
  });

  it('uses default icon when not specified', () => {
    const error = createUserError(
      'Title',
      'Message',
      'Action'
    );
    expect(error.icon).toBe('alert');
  });

  it('Property: All icons are valid', () => {
    const validIcons: ErrorIcon[] = [
      'wifi-off',
      'lock',
      'signature',
      'clock',
      'file',
      'alert',
      'user',
    ];

    fc.assert(
      fc.property(fc.constantFrom(...validIcons), (icon) => {
        const error = createUserError('Title', 'Message', 'Action', icon);
        expect(error.icon).toBe(icon);
      })
    );
  });
});

describe('getOfflineError', () => {
  it('returns correct offline error', () => {
    const error = getOfflineError();

    expect(error.title).toContain('Offline');
    expect(error.icon).toBe('wifi-off');
    expect(error.message.toLowerCase()).toContain('saved');
  });
});

describe('getFileTooLargeError', () => {
  it('returns correct file size error with default size', () => {
    const error = getFileTooLargeError();

    expect(error.title).toContain('Large');
    expect(error.icon).toBe('file');
    expect(error.message).toContain('25');
  });

  it('returns correct file size error with custom size', () => {
    const error = getFileTooLargeError(50);

    expect(error.message).toContain('50');
  });
});

describe('getUnsupportedFileError', () => {
  it('returns correct unsupported file error', () => {
    const error = getUnsupportedFileError();

    expect(error.title).toContain('Unsupported');
    expect(error.icon).toBe('file');
    expect(error.message.toLowerCase()).toContain('pdf');
  });
});

// ============================================================
// Edge Cases and Robustness
// ============================================================

describe('Edge Cases', () => {
  it('handles empty string input', () => {
    const result = getUserFriendlyError('');
    expect(result.icon).toBe('alert'); // Should fall back to generic
    expect(result.title).toBe('Something Went Wrong');
  });

  it('handles empty Error message', () => {
    const result = getUserFriendlyError(new Error(''));
    expect(result.icon).toBe('alert');
  });

  it('handles very long error messages', () => {
    const longMessage = 'network error '.repeat(1000);
    const result = getUserFriendlyError(new Error(longMessage));
    expect(result.icon).toBe('wifi-off'); // Should still categorize correctly
  });

  it('handles mixed case error messages', () => {
    const variations = [
      'NETWORK ERROR',
      'Network Error',
      'network error',
      'NeTwOrK eRrOr',
    ];

    variations.forEach((msg) => {
      const result = getUserFriendlyError(msg);
      expect(result.icon).toBe('wifi-off');
    });
  });

  it('handles errors with special characters', () => {
    const result = getUserFriendlyError('network error: <script>alert("xss")</script>');
    expect(result.icon).toBe('wifi-off');
  });

  it('handles errors with unicode characters', () => {
    const result = getUserFriendlyError('network error: connection failed');
    expect(result.icon).toBe('wifi-off');
  });

  it('Property: Random strings fallback to generic', () => {
    fc.assert(
      fc.property(
        fc.string({ minLength: 0, maxLength: 100 }).filter((s) => {
          // Filter out strings that would match known patterns
          const lowerS = s.toLowerCase();
          return !lowerS.includes('network') &&
            !lowerS.includes('password') &&
            !lowerS.includes('signature') &&
            !lowerS.includes('session') &&
            !lowerS.includes('expired') &&
            !lowerS.includes('corrupt') &&
            !lowerS.includes('invalid') &&
            !lowerS.includes('auth') &&
            !lowerS.includes('fetch') &&
            !lowerS.includes('connection') &&
            !lowerS.includes('encrypt') &&
            !lowerS.includes('sign') &&
            !lowerS.includes('401') &&
            !lowerS.includes('403') &&
            !lowerS.includes('lock') &&
            !lowerS.includes('offline') &&
            !lowerS.includes('internet') &&
            !lowerS.includes('timeout');
        }),
        (randomString) => {
          const result = getUserFriendlyError(randomString);
          expect(result.icon).toBe('alert');
        }
      ),
      { numRuns: 50 } // Fewer runs since filtering reduces valid inputs
    );
  });
});

// ============================================================
// Immutability Tests
// ============================================================

describe('Immutability', () => {
  it('returns a new object each time', () => {
    const error1 = getUserFriendlyError('network error');
    const error2 = getUserFriendlyError('network error');

    expect(error1).not.toBe(error2);
    expect(error1).toEqual(error2);
  });

  it('returned objects can be modified without affecting future calls', () => {
    const error1 = getUserFriendlyError('network error');
    error1.title = 'Modified Title';

    const error2 = getUserFriendlyError('network error');
    expect(error2.title).not.toBe('Modified Title');
    expect(error2.title).toContain('Connection');
  });
});
