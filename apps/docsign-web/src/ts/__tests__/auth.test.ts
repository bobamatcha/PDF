/**
 * Property-Based Tests for Authentication Module
 *
 * Uses fast-check for property-based testing of:
 * - Email validation
 * - Password validation
 * - Token storage/retrieval
 * - Auth state management
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as fc from 'fast-check';
import {
  validateEmail,
  validatePassword,
  isAuthenticated,
  getCurrentUser,
  getAccessToken,
  getDocumentsRemaining,
} from '../auth';

// ============================================================
// Mock localStorage
// ============================================================

const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
  };
})();

beforeEach(() => {
  localStorageMock.clear();
  Object.defineProperty(global, 'localStorage', { value: localStorageMock, writable: true });
});

afterEach(() => {
  localStorageMock.clear();
});

// ============================================================
// Arbitraries (Generators)
// ============================================================

// Valid email addresses
const validEmail = fc.tuple(
  fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz0123456789._-'.split('')), { minLength: 1, maxLength: 20 }),
  fc.constantFrom('gmail.com', 'yahoo.com', 'hotmail.com', 'example.com', 'test.org')
).map(([local, domain]) => `${local}@${domain}`);

// Invalid emails (missing @)
const emailMissingAt = fc.stringOf(
  fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz0123456789.'.split('')),
  { minLength: 5, maxLength: 30 }
).filter(s => !s.includes('@'));

// Invalid emails (too short)
const shortEmail = fc.stringOf(
  fc.constantFrom(...'ab@.'.split('')),
  { minLength: 1, maxLength: 4 }
);

// Valid passwords (8+ chars, 1 upper, 1 lower, 1 number)
const validPassword = fc.tuple(
  fc.constantFrom(...'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('')),
  fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'.split('')),
  fc.constantFrom(...'0123456789'.split('')),
  fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%'.split('')), { minLength: 5, maxLength: 20 })
).map(([upper, lower, num, rest]) => `${upper}${lower}${num}${rest}`);

// Passwords without uppercase
const passwordNoUpper = fc.stringOf(
  fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz0123456789'.split('')),
  { minLength: 8, maxLength: 30 }
);

// Passwords without lowercase
const passwordNoLower = fc.stringOf(
  fc.constantFrom(...'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.split('')),
  { minLength: 8, maxLength: 30 }
);

// Passwords without numbers
const passwordNoNumber = fc.stringOf(
  fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('')),
  { minLength: 8, maxLength: 30 }
);

// Short passwords (< 8 chars)
const shortPassword = fc.stringOf(
  fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.split('')),
  { minLength: 1, maxLength: 7 }
);

// User tier
const userTier = fc.constantFrom('free', 'pro');

// Mock user object
const mockUser = fc.record({
  id: fc.uuid(),
  email: validEmail,
  name: fc.string({ minLength: 1, maxLength: 50 }),
  tier: userTier,
  daily_documents_remaining: fc.integer({ min: 0, max: 100 }),
});

// ============================================================
// Email Validation Tests
// ============================================================

describe('Email Validation', () => {
  it('Property 1: Valid emails with @ and domain are accepted', () => {
    fc.assert(
      fc.property(validEmail, (email) => {
        const result = validateEmail(email);
        expect(result).toBeNull();
      })
    );
  });

  it('Property 2: Emails without @ are rejected', () => {
    fc.assert(
      fc.property(emailMissingAt, (email) => {
        const result = validateEmail(email);
        expect(result).not.toBeNull();
        expect(result).toContain('valid email');
      })
    );
  });

  it('Property 3: Emails shorter than 5 characters are rejected', () => {
    fc.assert(
      fc.property(shortEmail, (email) => {
        const result = validateEmail(email);
        expect(result).not.toBeNull();
      })
    );
  });

  it('Property 4: Empty string is rejected', () => {
    const result = validateEmail('');
    expect(result).not.toBeNull();
  });

  it('Property 5: Whitespace-only strings are rejected', () => {
    fc.assert(
      fc.property(fc.stringOf(fc.constant(' '), { minLength: 1, maxLength: 10 }), (whitespace) => {
        const result = validateEmail(whitespace);
        expect(result).not.toBeNull();
      })
    );
  });

  it('Property 6: Emails with whitespace are trimmed', () => {
    fc.assert(
      fc.property(validEmail, (email) => {
        const paddedEmail = `  ${email}  `;
        const result = validateEmail(paddedEmail);
        expect(result).toBeNull();
      })
    );
  });
});

// ============================================================
// Password Validation Tests
// ============================================================

describe('Password Validation', () => {
  it('Property 7: Valid passwords (8+ chars, upper, lower, number) are accepted', () => {
    fc.assert(
      fc.property(validPassword, (password) => {
        const result = validatePassword(password);
        expect(result).toBeNull();
      })
    );
  });

  it('Property 8: Passwords without uppercase are rejected', () => {
    fc.assert(
      fc.property(passwordNoUpper, (password) => {
        // Only test if it actually has no uppercase
        if (!/[A-Z]/.test(password)) {
          const result = validatePassword(password);
          expect(result).not.toBeNull();
          expect(result).toContain('uppercase');
        }
      })
    );
  });

  it('Property 9: Passwords without lowercase are rejected', () => {
    fc.assert(
      fc.property(passwordNoLower, (password) => {
        // Only test if it actually has no lowercase
        if (!/[a-z]/.test(password)) {
          const result = validatePassword(password);
          expect(result).not.toBeNull();
          expect(result).toContain('lowercase');
        }
      })
    );
  });

  it('Property 10: Passwords without numbers are rejected', () => {
    // Test with passwords that have upper and lower but no number
    const validWithoutNumber = ['Abcdefgh', 'PasswordTest', 'HelloWorld', 'TestPassAbc'];
    for (const password of validWithoutNumber) {
      const result = validatePassword(password);
      expect(result).not.toBeNull();
      expect(result).toContain('number');
    }
  });

  it('Property 11: Short passwords (< 8 chars) are rejected', () => {
    fc.assert(
      fc.property(shortPassword, (password) => {
        const result = validatePassword(password);
        expect(result).not.toBeNull();
        expect(result).toContain('8 characters');
      })
    );
  });

  it('Property 12: Empty password is rejected', () => {
    const result = validatePassword('');
    expect(result).not.toBeNull();
    expect(result).toContain('8 characters');
  });
});

// ============================================================
// Auth State Tests
// ============================================================

describe('Auth State Management', () => {
  it('Property 13: isAuthenticated returns false when no token', () => {
    localStorageMock.clear();
    expect(isAuthenticated()).toBe(false);
  });

  it('Property 14: isAuthenticated returns true when token exists', () => {
    localStorageMock.setItem('docsign_access_token', 'test-token-123');
    expect(isAuthenticated()).toBe(true);
  });

  it('Property 15: getAccessToken returns stored token', () => {
    fc.assert(
      fc.property(fc.string({ minLength: 10, maxLength: 200 }), (token) => {
        localStorageMock.setItem('docsign_access_token', token);
        expect(getAccessToken()).toBe(token);
      })
    );
  });

  it('Property 16: getAccessToken returns null when no token', () => {
    localStorageMock.clear();
    expect(getAccessToken()).toBeNull();
  });

  it('Property 17: getCurrentUser returns null when no user stored', () => {
    localStorageMock.clear();
    expect(getCurrentUser()).toBeNull();
  });

  it('Property 18: getCurrentUser returns parsed user when stored', () => {
    fc.assert(
      fc.property(mockUser, (user) => {
        localStorageMock.setItem('docsign_user', JSON.stringify(user));
        const retrieved = getCurrentUser();
        expect(retrieved).not.toBeNull();
        expect(retrieved?.id).toBe(user.id);
        expect(retrieved?.email).toBe(user.email);
        expect(retrieved?.tier).toBe(user.tier);
      })
    );
  });

  it('Property 19: getCurrentUser handles invalid JSON gracefully', () => {
    localStorageMock.setItem('docsign_user', 'not-valid-json{');
    expect(getCurrentUser()).toBeNull();
  });

  it('Property 20: getDocumentsRemaining returns 0 when no user', () => {
    localStorageMock.clear();
    expect(getDocumentsRemaining()).toBe(0);
  });

  it('Property 21: getDocumentsRemaining returns correct count', () => {
    fc.assert(
      fc.property(mockUser, (user) => {
        localStorageMock.setItem('docsign_user', JSON.stringify(user));
        expect(getDocumentsRemaining()).toBe(user.daily_documents_remaining);
      })
    );
  });
});

// ============================================================
// Edge Cases
// ============================================================

describe('Edge Cases', () => {
  it('Property 22: Emails with unicode characters', () => {
    // Unicode emails should be handled gracefully
    const unicodeEmails = ['test@例え.jp', 'пользователь@example.com', '用户@测试.cn'];
    for (const email of unicodeEmails) {
      const result = validateEmail(email);
      // Should either accept or reject gracefully, not throw
      expect(typeof result === 'string' || result === null).toBe(true);
    }
  });

  it('Property 23: Passwords with special characters are accepted', () => {
    const specialPasswords = ['Password1!', 'Test@123#$', 'Secure^&*()1a'];
    for (const password of specialPasswords) {
      const result = validatePassword(password);
      expect(result).toBeNull();
    }
  });

  it('Property 24: Very long emails are handled', () => {
    fc.assert(
      fc.property(
        fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'.split('')), { minLength: 100, maxLength: 200 }),
        (longLocal) => {
          const email = `${longLocal}@example.com`;
          const result = validateEmail(email);
          // Should handle gracefully
          expect(typeof result === 'string' || result === null).toBe(true);
        }
      )
    );
  });

  it('Property 25: Very long passwords are handled', () => {
    fc.assert(
      fc.property(
        fc.stringOf(
          fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.split('')),
          { minLength: 100, maxLength: 500 }
        ),
        (longPassword) => {
          // Ensure it has required chars
          const password = 'Aa1' + longPassword;
          const result = validatePassword(password);
          expect(result).toBeNull();
        }
      )
    );
  });

  it('Property 26: Null bytes in strings are handled gracefully', () => {
    const nullByteEmail = 'test\x00@example.com';
    const result = validateEmail(nullByteEmail);
    // Should handle gracefully without crashing
    expect(typeof result === 'string' || result === null).toBe(true);
  });
});

// ============================================================
// Consistency Tests
// ============================================================

describe('Consistency', () => {
  it('Property 27: Validation is deterministic - same input gives same output', () => {
    fc.assert(
      fc.property(fc.string(), (input) => {
        const result1 = validateEmail(input);
        const result2 = validateEmail(input);
        expect(result1).toBe(result2);
      })
    );
  });

  it('Property 28: Password validation is deterministic', () => {
    fc.assert(
      fc.property(fc.string(), (input) => {
        const result1 = validatePassword(input);
        const result2 = validatePassword(input);
        expect(result1).toBe(result2);
      })
    );
  });

  it('Property 29: Auth state reflects localStorage changes', () => {
    expect(isAuthenticated()).toBe(false);
    localStorageMock.setItem('docsign_access_token', 'token');
    expect(isAuthenticated()).toBe(true);
    localStorageMock.removeItem('docsign_access_token');
    expect(isAuthenticated()).toBe(false);
  });
});
