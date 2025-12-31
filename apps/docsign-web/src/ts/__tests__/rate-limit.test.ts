/**
 * Tests for Rate Limit Visibility Module
 *
 * @vitest-environment jsdom
 */

import { describe, it, expect, beforeEach } from 'vitest';
import * as fc from 'fast-check';

import {
  parseRateLimitInfo,
  getRateLimitStatus,
  getRateLimitMessage,
  processRateLimitResponse,
  createRateLimitStatusHtml,
  updateRateLimitInfo,
  getLastKnownRateLimitInfo,
  processAndStoreRateLimitInfo,
  type RateLimitInfo,
  type RateLimitStatus,
} from '../rate-limit';

// ============================================================
// Tests: parseRateLimitInfo
// ============================================================

describe('parseRateLimitInfo', () => {
  it('should parse valid response', () => {
    const response = {
      success: true,
      remaining_today: 50,
      remaining_month: 2500,
    };

    const info = parseRateLimitInfo(response);
    expect(info.remainingToday).toBe(50);
    expect(info.remainingMonth).toBe(2500);
  });

  it('should handle missing fields', () => {
    const response = { success: true };
    const info = parseRateLimitInfo(response);
    expect(info.remainingToday).toBeNull();
    expect(info.remainingMonth).toBeNull();
  });

  it('should handle null response', () => {
    const info = parseRateLimitInfo(null);
    expect(info.remainingToday).toBeNull();
    expect(info.remainingMonth).toBeNull();
  });

  it('should handle undefined response', () => {
    const info = parseRateLimitInfo(undefined);
    expect(info.remainingToday).toBeNull();
    expect(info.remainingMonth).toBeNull();
  });

  it('should ignore non-number values', () => {
    const response = {
      remaining_today: 'invalid',
      remaining_month: null,
    };
    const info = parseRateLimitInfo(response);
    expect(info.remainingToday).toBeNull();
    expect(info.remainingMonth).toBeNull();
  });

  it('should parse zero values correctly', () => {
    const response = {
      remaining_today: 0,
      remaining_month: 0,
    };
    const info = parseRateLimitInfo(response);
    expect(info.remainingToday).toBe(0);
    expect(info.remainingMonth).toBe(0);
  });
});

// ============================================================
// Tests: getRateLimitStatus
// ============================================================

describe('getRateLimitStatus', () => {
  it('should return ok for null values', () => {
    const status = getRateLimitStatus({
      remainingToday: null,
      remainingMonth: null,
    });
    expect(status).toBe('ok');
  });

  it('should return exceeded for zero values', () => {
    expect(getRateLimitStatus({ remainingToday: 0, remainingMonth: 100 })).toBe('exceeded');
    expect(getRateLimitStatus({ remainingToday: 50, remainingMonth: 0 })).toBe('exceeded');
    expect(getRateLimitStatus({ remainingToday: 0, remainingMonth: 0 })).toBe('exceeded');
  });

  it('should return critical for very low values', () => {
    expect(getRateLimitStatus({ remainingToday: 1, remainingMonth: null })).toBe('critical');
    expect(getRateLimitStatus({ remainingToday: 2, remainingMonth: null })).toBe('critical');
    expect(getRateLimitStatus({ remainingToday: null, remainingMonth: 10 })).toBe('critical');
  });

  it('should return warning for low values', () => {
    expect(getRateLimitStatus({ remainingToday: 5, remainingMonth: null })).toBe('warning');
    expect(getRateLimitStatus({ remainingToday: 9, remainingMonth: null })).toBe('warning');
    expect(getRateLimitStatus({ remainingToday: null, remainingMonth: 50 })).toBe('warning');
  });

  it('should return ok for high values', () => {
    expect(getRateLimitStatus({ remainingToday: 50, remainingMonth: 2500 })).toBe('ok');
    expect(getRateLimitStatus({ remainingToday: 100, remainingMonth: null })).toBe('ok');
    expect(getRateLimitStatus({ remainingToday: null, remainingMonth: 1000 })).toBe('ok');
  });

  it('should use property-based testing for status determination', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 0, max: 100 }),
        fc.integer({ min: 0, max: 3000 }),
        (daily, monthly) => {
          const status = getRateLimitStatus({
            remainingToday: daily,
            remainingMonth: monthly,
          });

          // Status should always be one of the valid values
          expect(['ok', 'warning', 'critical', 'exceeded']).toContain(status);

          // If either is 0, should be exceeded
          if (daily === 0 || monthly === 0) {
            expect(status).toBe('exceeded');
          }

          // If both are high, should be ok
          if (daily >= 10 && monthly >= 100) {
            expect(status).toBe('ok');
          }
        }
      ),
      { numRuns: 50 }
    );
  });
});

// ============================================================
// Tests: getRateLimitMessage
// ============================================================

describe('getRateLimitMessage', () => {
  it('should return null for ok status', () => {
    const message = getRateLimitMessage({ remainingToday: 50, remainingMonth: 2500 }, 'ok');
    expect(message).toBeNull();
  });

  it('should return message for exceeded daily', () => {
    const message = getRateLimitMessage({ remainingToday: 0, remainingMonth: 100 }, 'exceeded');
    expect(message).toContain('daily limit');
    expect(message).toContain('tomorrow');
  });

  it('should return message for exceeded monthly', () => {
    const message = getRateLimitMessage({ remainingToday: 50, remainingMonth: 0 }, 'exceeded');
    expect(message).toContain('monthly limit');
  });

  it('should return message for warning', () => {
    const message = getRateLimitMessage({ remainingToday: 5, remainingMonth: null }, 'warning');
    expect(message).toContain('5');
    expect(message).toContain('remaining');
  });

  it('should return message for critical', () => {
    const message = getRateLimitMessage({ remainingToday: 2, remainingMonth: null }, 'critical');
    expect(message).toContain('2');
    expect(message).toContain('remaining');
  });
});

// ============================================================
// Tests: processRateLimitResponse
// ============================================================

describe('processRateLimitResponse', () => {
  it('should process full response', () => {
    const response = {
      success: true,
      remaining_today: 5,
      remaining_month: 2500,
    };

    const result = processRateLimitResponse(response);
    expect(result.status).toBe('warning');
    expect(result.info.remainingToday).toBe(5);
    expect(result.info.remainingMonth).toBe(2500);
    expect(result.message).not.toBeNull();
  });

  it('should handle exceeded response', () => {
    const response = {
      success: false,
      remaining_today: 0,
      remaining_month: 2500,
    };

    const result = processRateLimitResponse(response);
    expect(result.status).toBe('exceeded');
    expect(result.message).toContain('daily');
  });
});

// ============================================================
// Tests: createRateLimitStatusHtml
// ============================================================

describe('createRateLimitStatusHtml', () => {
  it('should return empty string for null values', () => {
    const html = createRateLimitStatusHtml({
      remainingToday: null,
      remainingMonth: null,
    });
    expect(html).toBe('');
  });

  it('should include role=status for accessibility', () => {
    const html = createRateLimitStatusHtml({
      remainingToday: 50,
      remainingMonth: null,
    });
    expect(html).toContain('role="status"');
  });

  it('should include appropriate class for ok status', () => {
    const html = createRateLimitStatusHtml({
      remainingToday: 50,
      remainingMonth: 2500,
    });
    expect(html).toContain('rate-limit-ok');
    expect(html).toContain('50 today');
    expect(html).toContain('2500 this month');
  });

  it('should include appropriate class for warning status', () => {
    const html = createRateLimitStatusHtml({
      remainingToday: 5,
      remainingMonth: null,
    });
    expect(html).toContain('rate-limit-warning');
  });

  it('should include appropriate class for critical status', () => {
    const html = createRateLimitStatusHtml({
      remainingToday: 2,
      remainingMonth: null,
    });
    expect(html).toContain('rate-limit-critical');
  });

  it('should include appropriate class for exceeded status', () => {
    const html = createRateLimitStatusHtml({
      remainingToday: 0,
      remainingMonth: 0,
    });
    expect(html).toContain('rate-limit-exceeded');
  });
});

// ============================================================
// Tests: State Management
// ============================================================

describe('State Management', () => {
  beforeEach(() => {
    // Reset state
    updateRateLimitInfo({ remainingToday: null, remainingMonth: null });
  });

  it('should store and retrieve rate limit info', () => {
    updateRateLimitInfo({ remainingToday: 75, remainingMonth: 2800 });

    const info = getLastKnownRateLimitInfo();
    expect(info.remainingToday).toBe(75);
    expect(info.remainingMonth).toBe(2800);
  });

  it('should return a copy, not the original object', () => {
    updateRateLimitInfo({ remainingToday: 50, remainingMonth: 2500 });

    const info1 = getLastKnownRateLimitInfo();
    const info2 = getLastKnownRateLimitInfo();

    expect(info1).not.toBe(info2);
    expect(info1).toEqual(info2);
  });

  it('should process and store from response', () => {
    const response = {
      success: true,
      remaining_today: 45,
      remaining_month: 2200,
    };

    const result = processAndStoreRateLimitInfo(response);

    expect(result.info.remainingToday).toBe(45);
    expect(result.info.remainingMonth).toBe(2200);

    const stored = getLastKnownRateLimitInfo();
    expect(stored.remainingToday).toBe(45);
    expect(stored.remainingMonth).toBe(2200);
  });
});

// ============================================================
// Property-based Tests
// ============================================================

describe('Property-based Tests', () => {
  it('status should be consistent with message presence', () => {
    fc.assert(
      fc.property(
        fc.option(fc.integer({ min: 0, max: 100 }), { nil: null }),
        fc.option(fc.integer({ min: 0, max: 3000 }), { nil: null }),
        (daily, monthly) => {
          const info: RateLimitInfo = {
            remainingToday: daily,
            remainingMonth: monthly,
          };
          const status = getRateLimitStatus(info);
          const message = getRateLimitMessage(info, status);

          // ok status should have no message
          if (status === 'ok') {
            expect(message).toBeNull();
          }

          // non-ok status should have a message
          if (status !== 'ok') {
            expect(message).not.toBeNull();
            expect(typeof message).toBe('string');
            expect(message!.length).toBeGreaterThan(0);
          }
        }
      ),
      { numRuns: 50 }
    );
  });

  it('HTML should be valid for any status', () => {
    fc.assert(
      fc.property(
        fc.option(fc.integer({ min: 0, max: 100 }), { nil: null }),
        fc.option(fc.integer({ min: 0, max: 3000 }), { nil: null }),
        (daily, monthly) => {
          const info: RateLimitInfo = {
            remainingToday: daily,
            remainingMonth: monthly,
          };
          const html = createRateLimitStatusHtml(info);

          if (daily === null && monthly === null) {
            expect(html).toBe('');
          } else {
            expect(html).toContain('rate-limit-');
            expect(html).toContain('role="status"');
          }
        }
      ),
      { numRuns: 50 }
    );
  });

  it('parse and status should handle any input safely', () => {
    fc.assert(
      fc.property(fc.anything(), (input) => {
        // Should never throw
        const info = parseRateLimitInfo(input);
        const status = getRateLimitStatus(info);

        expect(typeof info.remainingToday === 'number' || info.remainingToday === null).toBe(true);
        expect(typeof info.remainingMonth === 'number' || info.remainingMonth === null).toBe(true);
        expect(['ok', 'warning', 'critical', 'exceeded']).toContain(status);
      }),
      { numRuns: 100 }
    );
  });
});
