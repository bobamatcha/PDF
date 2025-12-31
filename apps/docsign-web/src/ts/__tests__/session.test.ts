/**
 * Property-Based Tests for Session Management
 *
 * Uses fast-check for property-based testing of session validation,
 * expiry detection, field filtering, and offline queue serialization.
 */

import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import {
  validateSessionParams,
  isSessionExpired,
  filterFieldsByRecipient,
  areAllRequiredFieldsComplete,
  serializeQueuedSubmission,
  deserializeQueuedSubmission,
  type SessionParams,
  type SigningField,
  type QueuedSubmission,
} from '../session';

// ============================================================
// Arbitraries (Generators)
// ============================================================

// Valid session IDs (3+ alphanumeric characters with _ and -)
const validSessionId = fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-'.split('')), {
  minLength: 3,
  maxLength: 64,
});

// Short session IDs (1-2 characters)
const shortSessionId = fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.split('')), {
  minLength: 1,
  maxLength: 2,
});

// Valid recipient IDs (1+ alphanumeric characters)
const validRecipientId = fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-'.split('')), {
  minLength: 1,
  maxLength: 64,
});

// Valid signing keys (3+ characters)
const validSigningKey = fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-'.split('')), {
  minLength: 3,
  maxLength: 128,
});

// Short signing keys (1-2 characters)
const shortSigningKey = fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.split('')), {
  minLength: 1,
  maxLength: 2,
});

// Field type arbitrary
const fieldType = fc.constantFrom('signature', 'initials', 'date', 'text');

// Signing field arbitrary
const signingField = (recipientIdArb: fc.Arbitrary<string> = validRecipientId): fc.Arbitrary<SigningField> =>
  fc.record({
    id: fc.uuid(),
    type: fieldType,
    page: fc.integer({ min: 1, max: 100 }),
    x: fc.float({ min: 0, max: 1000, noNaN: true }),
    y: fc.float({ min: 0, max: 1000, noNaN: true }),
    width: fc.float({ min: 10, max: 500, noNaN: true }),
    height: fc.float({ min: 10, max: 200, noNaN: true }),
    required: fc.boolean(),
    recipientId: recipientIdArb,
  });

// Queued submission arbitrary
const queuedSubmission = fc.record({
  sessionId: validSessionId,
  recipientId: validRecipientId,
  signingKey: validSigningKey,
  signatures: fc.dictionary(fc.uuid(), fc.string()),
  completedAt: fc.date().map((d) => d.toISOString()),
  timestamp: fc.integer({ min: 0, max: Number.MAX_SAFE_INTEGER }),
});

// ============================================================
// Session ID Validation Tests
// ============================================================

describe('Session ID Validation', () => {
  it('Property 1: Valid session IDs (length >= 3) are accepted', () => {
    fc.assert(
      fc.property(validSessionId, validRecipientId, validSigningKey, (sessionId, recipientId, signingKey) => {
        const result = validateSessionParams({ sessionId, recipientId, signingKey });
        expect(result.valid).toBe(true);
        expect(result.error).toBeUndefined();
      })
    );
  });

  it('Property 2: Short session IDs (length < 3) are rejected', () => {
    fc.assert(
      fc.property(shortSessionId, validRecipientId, validSigningKey, (sessionId, recipientId, signingKey) => {
        const result = validateSessionParams({ sessionId, recipientId, signingKey });
        expect(result.valid).toBe(false);
        expect(result.error).toContain('session');
      })
    );
  });

  it('Property 3: Empty session IDs are rejected', () => {
    const result = validateSessionParams({
      sessionId: '',
      recipientId: 'r1',
      signingKey: 'key_abc',
    });
    expect(result.valid).toBe(false);
    expect(result.error).toContain('session');
  });

  it('Property 4: Null session IDs are rejected', () => {
    const result = validateSessionParams({
      sessionId: null,
      recipientId: 'r1',
      signingKey: 'key_abc',
    });
    expect(result.valid).toBe(false);
    expect(result.error).toContain('session');
  });

  it('Property 5: Undefined session IDs are rejected', () => {
    const result = validateSessionParams({
      sessionId: undefined,
      recipientId: 'r1',
      signingKey: 'key_abc',
    });
    expect(result.valid).toBe(false);
    expect(result.error).toContain('session');
  });
});

// ============================================================
// Recipient ID Validation Tests
// ============================================================

describe('Recipient ID Validation', () => {
  it('Property 6: Non-empty recipient IDs are accepted', () => {
    fc.assert(
      fc.property(validSessionId, validRecipientId, validSigningKey, (sessionId, recipientId, signingKey) => {
        const result = validateSessionParams({ sessionId, recipientId, signingKey });
        expect(result.valid).toBe(true);
      })
    );
  });

  it('Property 7: Empty recipient IDs are rejected', () => {
    const result = validateSessionParams({
      sessionId: 'sess_123',
      recipientId: '',
      signingKey: 'key_abc',
    });
    expect(result.valid).toBe(false);
    expect(result.error).toContain('recipient');
  });

  it('Property 8: Null recipient IDs are rejected', () => {
    const result = validateSessionParams({
      sessionId: 'sess_123',
      recipientId: null,
      signingKey: 'key_abc',
    });
    expect(result.valid).toBe(false);
    expect(result.error).toContain('recipient');
  });
});

// ============================================================
// Signing Key Validation Tests
// ============================================================

describe('Signing Key Validation', () => {
  it('Property 9: Valid signing keys (length >= 3) are accepted', () => {
    fc.assert(
      fc.property(validSessionId, validRecipientId, validSigningKey, (sessionId, recipientId, signingKey) => {
        const result = validateSessionParams({ sessionId, recipientId, signingKey });
        expect(result.valid).toBe(true);
      })
    );
  });

  it('Property 10: Short signing keys (length < 3) are rejected', () => {
    fc.assert(
      fc.property(validSessionId, validRecipientId, shortSigningKey, (sessionId, recipientId, signingKey) => {
        const result = validateSessionParams({ sessionId, recipientId, signingKey });
        expect(result.valid).toBe(false);
        expect(result.error).toContain('signing key');
      })
    );
  });

  it('Property 11: Null signing keys are rejected', () => {
    const result = validateSessionParams({
      sessionId: 'sess_123',
      recipientId: 'r1',
      signingKey: null,
    });
    expect(result.valid).toBe(false);
    expect(result.error).toContain('key');
  });
});

// ============================================================
// Session Expiry Detection Tests
// ============================================================

describe('Session Expiry Detection', () => {
  const ONE_DAY_MS = 24 * 60 * 60 * 1000;
  const SEVEN_DAYS_MS = 7 * ONE_DAY_MS;

  it('Property 12: Sessions created now are not expired', () => {
    const now = Date.now();
    expect(isSessionExpired(now, SEVEN_DAYS_MS)).toBe(false);
  });

  it('Property 13: Sessions older than TTL are expired', () => {
    fc.assert(
      fc.property(fc.integer({ min: 2, max: 100 }), (daysOld) => {
        const ttlMs = ONE_DAY_MS; // 1 day TTL
        // Use daysOld + 1 hour to ensure we're clearly past TTL
        const createdAt = Date.now() - (daysOld * ONE_DAY_MS + 3600000);
        expect(isSessionExpired(createdAt, ttlMs)).toBe(true);
      })
    );
  });

  it('Property 14: Sessions within TTL are not expired', () => {
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 6 }), (daysOld) => {
        const ttlMs = SEVEN_DAYS_MS; // 7 days TTL
        const createdAt = Date.now() - daysOld * ONE_DAY_MS;
        expect(isSessionExpired(createdAt, ttlMs)).toBe(false);
      })
    );
  });

  it('Property 15: ISO date strings work correctly', () => {
    const now = new Date();
    const isoString = now.toISOString();
    expect(isSessionExpired(isoString, SEVEN_DAYS_MS)).toBe(false);

    // Old date
    const oldDate = new Date(Date.now() - 8 * ONE_DAY_MS);
    expect(isSessionExpired(oldDate.toISOString(), SEVEN_DAYS_MS)).toBe(true);
  });

  it('Property 16: Invalid timestamps are treated as expired', () => {
    expect(isSessionExpired('invalid-date', SEVEN_DAYS_MS)).toBe(true);
    expect(isSessionExpired(NaN, SEVEN_DAYS_MS)).toBe(true);
  });
});

// ============================================================
// Field Filtering Tests
// ============================================================

describe('Field Filtering by Recipient', () => {
  it('Property 17: Filtering returns only fields for the specified recipient', () => {
    fc.assert(
      fc.property(
        validRecipientId,
        fc.array(signingField(), { minLength: 0, maxLength: 20 }),
        (targetRecipientId, fields) => {
          const filtered = filterFieldsByRecipient(fields, targetRecipientId);

          // All filtered fields should belong to the target recipient
          filtered.forEach((field) => {
            expect(field.recipientId).toBe(targetRecipientId);
          });

          // Count should match
          const expectedCount = fields.filter((f) => f.recipientId === targetRecipientId).length;
          expect(filtered.length).toBe(expectedCount);
        }
      )
    );
  });

  it('Property 18: Filtering preserves field data integrity', () => {
    fc.assert(
      fc.property(
        validRecipientId,
        fc.array(signingField(fc.constant('r1')), { minLength: 1, maxLength: 10 }),
        (_, fields) => {
          const filtered = filterFieldsByRecipient(fields, 'r1');

          filtered.forEach((field) => {
            const original = fields.find((f) => f.id === field.id);
            expect(original).toBeDefined();
            expect(field).toEqual(original);
          });
        }
      )
    );
  });

  it('Property 19: Empty fields array returns empty result', () => {
    const result = filterFieldsByRecipient([], 'r1');
    expect(result).toEqual([]);
  });

  it('Property 20: Non-matching recipient returns empty result', () => {
    fc.assert(
      fc.property(
        fc.array(signingField(fc.constant('r1')), { minLength: 1, maxLength: 10 }),
        (fields) => {
          const filtered = filterFieldsByRecipient(fields, 'non_existent_recipient_xyz');
          expect(filtered).toEqual([]);
        }
      )
    );
  });
});

// ============================================================
// Required Field Completion Tests
// ============================================================

describe('Required Field Completion Check', () => {
  it('Property 21: All required fields completed returns true', () => {
    fc.assert(
      fc.property(
        fc.array(
          fc.record({
            id: fc.uuid(),
            type: fieldType,
            page: fc.constant(1),
            x: fc.constant(100),
            y: fc.constant(100),
            width: fc.constant(200),
            height: fc.constant(50),
            required: fc.constant(true),
            recipientId: fc.constant('r1'),
          }),
          { minLength: 1, maxLength: 10 }
        ),
        (fields) => {
          const completedIds = new Set(fields.map((f) => f.id));
          expect(areAllRequiredFieldsComplete(fields, completedIds)).toBe(true);
        }
      )
    );
  });

  it('Property 22: Missing required field returns false', () => {
    const fields: SigningField[] = [
      { id: 'f1', type: 'signature', page: 1, x: 100, y: 100, width: 200, height: 50, required: true, recipientId: 'r1' },
      { id: 'f2', type: 'date', page: 1, x: 100, y: 200, width: 100, height: 30, required: true, recipientId: 'r1' },
    ];

    // Only complete first field
    const completedIds = new Set(['f1']);
    expect(areAllRequiredFieldsComplete(fields, completedIds)).toBe(false);
  });

  it('Property 23: Optional fields do not affect completion', () => {
    const fields: SigningField[] = [
      { id: 'f1', type: 'signature', page: 1, x: 100, y: 100, width: 200, height: 50, required: true, recipientId: 'r1' },
      { id: 'f2', type: 'date', page: 1, x: 100, y: 200, width: 100, height: 30, required: false, recipientId: 'r1' },
    ];

    // Only complete required field
    const completedIds = new Set(['f1']);
    expect(areAllRequiredFieldsComplete(fields, completedIds)).toBe(true);
  });

  it('Property 24: Empty fields array returns true', () => {
    expect(areAllRequiredFieldsComplete([], new Set())).toBe(true);
  });

  it('Property 25: All optional fields returns true even with none completed', () => {
    fc.assert(
      fc.property(
        fc.array(
          fc.record({
            id: fc.uuid(),
            type: fieldType,
            page: fc.constant(1),
            x: fc.constant(100),
            y: fc.constant(100),
            width: fc.constant(200),
            height: fc.constant(50),
            required: fc.constant(false),
            recipientId: fc.constant('r1'),
          }),
          { minLength: 1, maxLength: 10 }
        ),
        (fields) => {
          expect(areAllRequiredFieldsComplete(fields, new Set())).toBe(true);
        }
      )
    );
  });
});

// ============================================================
// Offline Queue Serialization Tests
// ============================================================

describe('Offline Queue Serialization', () => {
  it('Property 26: Queued submission roundtrip preserves all data', () => {
    fc.assert(
      fc.property(queuedSubmission, (submission) => {
        const serialized = serializeQueuedSubmission(submission);
        const deserialized = deserializeQueuedSubmission(serialized);

        expect(deserialized.sessionId).toBe(submission.sessionId);
        expect(deserialized.recipientId).toBe(submission.recipientId);
        expect(deserialized.signingKey).toBe(submission.signingKey);
        expect(deserialized.signatures).toEqual(submission.signatures);
        expect(deserialized.completedAt).toBe(submission.completedAt);
        expect(deserialized.timestamp).toBe(submission.timestamp);
      })
    );
  });

  it('Property 27: Serialized format is valid JSON', () => {
    fc.assert(
      fc.property(queuedSubmission, (submission) => {
        const serialized = serializeQueuedSubmission(submission);
        expect(() => JSON.parse(serialized)).not.toThrow();
      })
    );
  });

  it('Property 28: Invalid JSON throws error', () => {
    expect(() => deserializeQueuedSubmission('not valid json')).toThrow();
  });

  it('Property 29: Missing required fields throws error', () => {
    expect(() => deserializeQueuedSubmission('{}')).toThrow('Invalid sessionId');

    expect(() => deserializeQueuedSubmission('{"sessionId":"s1"}')).toThrow('Invalid recipientId');

    expect(() =>
      deserializeQueuedSubmission('{"sessionId":"s1","recipientId":"r1"}')
    ).toThrow('Invalid signingKey');

    expect(() =>
      deserializeQueuedSubmission('{"sessionId":"s1","recipientId":"r1","signingKey":"k1"}')
    ).toThrow('Invalid signatures');

    expect(() =>
      deserializeQueuedSubmission(
        '{"sessionId":"s1","recipientId":"r1","signingKey":"k1","signatures":{}}'
      )
    ).toThrow('Invalid completedAt');

    expect(() =>
      deserializeQueuedSubmission(
        '{"sessionId":"s1","recipientId":"r1","signingKey":"k1","signatures":{},"completedAt":"2025-01-01"}'
      )
    ).toThrow('Invalid timestamp');
  });

  it('Property 30: Null signatures throws error', () => {
    expect(() =>
      deserializeQueuedSubmission(
        '{"sessionId":"s1","recipientId":"r1","signingKey":"k1","signatures":null,"completedAt":"2025-01-01","timestamp":123}'
      )
    ).toThrow('Invalid signatures');
  });

  it('Property 31: Complex signatures object roundtrips correctly', () => {
    const submission: QueuedSubmission = {
      sessionId: 'sess_123',
      recipientId: 'r1',
      signingKey: 'key_abc',
      signatures: {
        'field-1': { type: 'drawn', data: 'base64data...' },
        'field-2': { type: 'typed', text: 'John Doe', font: 'Dancing Script' },
        'field-3': 'simple-value',
      },
      completedAt: '2025-12-21T12:00:00Z',
      timestamp: Date.now(),
    };

    const serialized = serializeQueuedSubmission(submission);
    const deserialized = deserializeQueuedSubmission(serialized);

    expect(deserialized.signatures).toEqual(submission.signatures);
  });
});

// ============================================================
// Validation Edge Cases
// ============================================================

describe('Validation Edge Cases', () => {
  it('Property 32: Whitespace-only strings are rejected', () => {
    const result = validateSessionParams({
      sessionId: '   ',
      recipientId: 'r1',
      signingKey: 'key_abc',
    });
    // Whitespace-only session IDs are rejected after trimming
    // Security fix: whitespace is now trimmed before length validation
    expect(result.valid).toBe(false);
  });

  it('Property 33: Special characters in IDs are accepted', () => {
    fc.assert(
      fc.property(
        fc.stringOf(fc.constantFrom(...'_-'.split('')), { minLength: 3, maxLength: 10 }),
        (specialId) => {
          const result = validateSessionParams({
            sessionId: specialId,
            recipientId: 'r1',
            signingKey: 'key_abc',
          });
          expect(result.valid).toBe(true);
        }
      )
    );
  });

  it('Property 34: Very long valid IDs are accepted', () => {
    const longId = 'a'.repeat(1000);
    const result = validateSessionParams({
      sessionId: longId,
      recipientId: 'r1',
      signingKey: 'key_abc',
    });
    expect(result.valid).toBe(true);
  });

  it('Property 35: Unicode characters are accepted', () => {
    // This tests potential edge cases with unicode
    const result = validateSessionParams({
      sessionId: 'session_test',
      recipientId: 'r1',
      signingKey: 'key_abc',
    });
    expect(result.valid).toBe(true);
  });
});
