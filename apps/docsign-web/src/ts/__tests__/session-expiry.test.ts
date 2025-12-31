/**
 * Tests for Session Expiry Warning System
 *
 * @vitest-environment jsdom
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as fc from 'fast-check';

import {
  SessionExpiryWatcher,
  getSessionTimeRemaining,
  formatTimeRemaining,
  createExpiryStatusHtml,
  initSessionExpiryMonitoring,
  stopSessionExpiryMonitoring,
  getSessionExpiryWatcher,
} from '../session-expiry';

// ============================================================
// Test Utilities
// ============================================================

function createMockDate(timestamp: number) {
  vi.useFakeTimers();
  vi.setSystemTime(timestamp);
}

function restoreRealTimers() {
  vi.useRealTimers();
}

// ============================================================
// Tests: getSessionTimeRemaining
// ============================================================

describe('getSessionTimeRemaining', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    restoreRealTimers();
  });

  it('should return positive value for non-expired session', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const createdAt = now - 1000; // Created 1 second ago
    const ttlMs = 60000; // 1 minute TTL

    const remaining = getSessionTimeRemaining(createdAt, ttlMs);
    expect(remaining).toBeGreaterThan(0);
    expect(remaining).toBeLessThanOrEqual(59000); // ~59 seconds remaining
  });

  it('should return 0 for expired session', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const createdAt = now - 120000; // Created 2 minutes ago
    const ttlMs = 60000; // 1 minute TTL

    const remaining = getSessionTimeRemaining(createdAt, ttlMs);
    expect(remaining).toBe(0);
  });

  it('should handle ISO string timestamps', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const createdAt = new Date(now - 30000).toISOString(); // 30 seconds ago
    const ttlMs = 60000; // 1 minute TTL

    const remaining = getSessionTimeRemaining(createdAt, ttlMs);
    expect(remaining).toBeGreaterThan(0);
    expect(remaining).toBeLessThanOrEqual(30000);
  });

  it('should return 0 for invalid timestamps', () => {
    expect(getSessionTimeRemaining('invalid')).toBe(0);
    expect(getSessionTimeRemaining(NaN)).toBe(0);
  });

  it('should use default 7-day TTL', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const createdAt = now - 1000; // Just created
    const remaining = getSessionTimeRemaining(createdAt);

    const sevenDaysMs = 7 * 24 * 60 * 60 * 1000;
    expect(remaining).toBeGreaterThan(sevenDaysMs - 2000);
    expect(remaining).toBeLessThanOrEqual(sevenDaysMs);
  });
});

// ============================================================
// Tests: formatTimeRemaining
// ============================================================

describe('formatTimeRemaining', () => {
  it('should format expired time', () => {
    expect(formatTimeRemaining(0)).toBe('Expired');
    expect(formatTimeRemaining(-1000)).toBe('Expired');
  });

  it('should format seconds as less than 1 minute', () => {
    expect(formatTimeRemaining(30 * 1000)).toBe('Less than 1 minute');
    expect(formatTimeRemaining(59 * 1000)).toBe('Less than 1 minute');
  });

  it('should format minutes', () => {
    expect(formatTimeRemaining(60 * 1000)).toBe('1 minute');
    expect(formatTimeRemaining(5 * 60 * 1000)).toBe('5 minutes');
    expect(formatTimeRemaining(45 * 60 * 1000)).toBe('45 minutes');
  });

  it('should format hours with minutes', () => {
    expect(formatTimeRemaining(60 * 60 * 1000)).toBe('1 hour');
    expect(formatTimeRemaining(90 * 60 * 1000)).toBe('1h 30m');
    expect(formatTimeRemaining(3 * 60 * 60 * 1000)).toBe('3 hours');
  });

  it('should format days with hours', () => {
    expect(formatTimeRemaining(24 * 60 * 60 * 1000)).toBe('1 day');
    expect(formatTimeRemaining(36 * 60 * 60 * 1000)).toBe('1d 12h');
    expect(formatTimeRemaining(7 * 24 * 60 * 60 * 1000)).toBe('7 days');
  });

  it('should handle property-based time values', () => {
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 10 * 24 * 60 }), (minutes) => {
        const ms = minutes * 60 * 1000;
        const formatted = formatTimeRemaining(ms);
        expect(typeof formatted).toBe('string');
        expect(formatted.length).toBeGreaterThan(0);
        expect(formatted).not.toBe('Expired');
      }),
      { numRuns: 30 }
    );
  });
});

// ============================================================
// Tests: createExpiryStatusHtml
// ============================================================

describe('createExpiryStatusHtml', () => {
  it('should create expired status', () => {
    const html = createExpiryStatusHtml(0);
    expect(html).toContain('expiry-status-expired');
    expect(html).toContain('Expired');
    expect(html).toContain('role="timer"');
  });

  it('should create urgent status for < 10 minutes', () => {
    const html = createExpiryStatusHtml(5 * 60 * 1000);
    expect(html).toContain('expiry-status-urgent');
    expect(html).toContain('5 minutes');
  });

  it('should create warning status for < 1 hour', () => {
    const html = createExpiryStatusHtml(30 * 60 * 1000);
    expect(html).toContain('expiry-status-warning');
    expect(html).toContain('30 minutes');
  });

  it('should create normal status for > 1 hour', () => {
    const html = createExpiryStatusHtml(2 * 60 * 60 * 1000);
    expect(html).toContain('expiry-status');
    expect(html).not.toContain('expiry-status-urgent');
    expect(html).not.toContain('expiry-status-warning');
    expect(html).toContain('2 hours');
  });

  it('should include aria-live for accessibility', () => {
    const html = createExpiryStatusHtml(60 * 60 * 1000);
    expect(html).toContain('aria-live="polite"');
  });
});

// ============================================================
// Tests: SessionExpiryWatcher
// ============================================================

describe('SessionExpiryWatcher', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    stopSessionExpiryMonitoring();
  });

  afterEach(() => {
    stopSessionExpiryMonitoring();
    restoreRealTimers();
  });

  it('should calculate time remaining correctly', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const createdAt = now;
    const ttlMs = 60 * 60 * 1000; // 1 hour

    const watcher = new SessionExpiryWatcher({ createdAt, ttlMs });
    const remaining = watcher.getTimeRemaining();

    expect(remaining).toBe(ttlMs);
  });

  it('should format time remaining', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const watcher = new SessionExpiryWatcher({
      createdAt: now,
      ttlMs: 2 * 60 * 60 * 1000, // 2 hours
    });

    expect(watcher.getTimeRemainingFormatted()).toBe('2 hours remaining');
  });

  it('should detect expired session', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const watcher = new SessionExpiryWatcher({
      createdAt: now - 2 * 60 * 60 * 1000, // 2 hours ago
      ttlMs: 60 * 60 * 1000, // 1 hour TTL
    });

    expect(watcher.hasExpired()).toBe(true);
    expect(watcher.getTimeRemainingFormatted()).toBe('Expired');
  });

  it('should call onExpire when session expires', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const onExpire = vi.fn();

    const watcher = new SessionExpiryWatcher({
      createdAt: now,
      ttlMs: 60 * 1000, // 1 minute
      onExpire,
    });

    watcher.start();

    // Advance past expiry
    vi.advanceTimersByTime(70 * 1000);

    expect(onExpire).toHaveBeenCalledTimes(1);
  });

  it('should call onWarning at thresholds', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const onWarning = vi.fn();

    const watcher = new SessionExpiryWatcher({
      createdAt: now,
      ttlMs: 2 * 60 * 60 * 1000, // 2 hours
      onWarning,
    });

    watcher.start();

    // Advance to 1 hour before expiry (should trigger 60 min warning)
    vi.advanceTimersByTime(60 * 60 * 1000 + 30 * 1000); // 1 hour + 30 seconds

    expect(onWarning).toHaveBeenCalled();
  });

  it('should stop monitoring when stopped', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const onWarning = vi.fn();

    const watcher = new SessionExpiryWatcher({
      createdAt: now,
      ttlMs: 2 * 60 * 60 * 1000,
      onWarning,
    });

    watcher.start();
    watcher.stop();

    // Advance time - should not trigger any warnings
    vi.advanceTimersByTime(2 * 60 * 60 * 1000);

    // The initial check when start() is called might have triggered a callback,
    // but no additional ones should be triggered after stop()
    const callCount = onWarning.mock.calls.length;
    vi.advanceTimersByTime(60 * 1000);
    expect(onWarning.mock.calls.length).toBe(callCount);
  });
});

// ============================================================
// Tests: Global Instance Management
// ============================================================

describe('Global Instance Management', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    stopSessionExpiryMonitoring();
  });

  afterEach(() => {
    stopSessionExpiryMonitoring();
    restoreRealTimers();
  });

  it('should initialize and return watcher', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const watcher = initSessionExpiryMonitoring({
      createdAt: now,
      ttlMs: 60 * 60 * 1000,
    });

    expect(watcher).toBeInstanceOf(SessionExpiryWatcher);
    expect(getSessionExpiryWatcher()).toBe(watcher);
  });

  it('should stop previous watcher when reinitializing', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    const watcher1 = initSessionExpiryMonitoring({
      createdAt: now,
      ttlMs: 60 * 60 * 1000,
    });

    const watcher2 = initSessionExpiryMonitoring({
      createdAt: now,
      ttlMs: 30 * 60 * 1000,
    });

    expect(getSessionExpiryWatcher()).toBe(watcher2);
    expect(getSessionExpiryWatcher()).not.toBe(watcher1);
  });

  it('should clear watcher on stop', () => {
    const now = Date.now();
    vi.setSystemTime(now);

    initSessionExpiryMonitoring({
      createdAt: now,
      ttlMs: 60 * 60 * 1000,
    });

    expect(getSessionExpiryWatcher()).not.toBeNull();

    stopSessionExpiryMonitoring();

    expect(getSessionExpiryWatcher()).toBeNull();
  });
});

// ============================================================
// Property-based Tests
// ============================================================

describe('Property-based Tests', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    restoreRealTimers();
  });

  it('time remaining should never be negative', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 0, max: 30 * 24 * 60 * 60 * 1000 }), // up to 30 days
        fc.integer({ min: 1000, max: 7 * 24 * 60 * 60 * 1000 }), // 1s to 7 days TTL
        (ageMs, ttlMs) => {
          const now = 1000000000000; // Fixed reference time
          vi.setSystemTime(now);

          const createdAt = now - ageMs;
          const remaining = getSessionTimeRemaining(createdAt, ttlMs);

          expect(remaining).toBeGreaterThanOrEqual(0);
        }
      ),
      { numRuns: 50 }
    );
  });

  it('formatted time should never be empty', () => {
    fc.assert(
      fc.property(fc.integer({ min: -1000, max: 10 * 24 * 60 * 60 * 1000 }), (ms) => {
        const formatted = formatTimeRemaining(ms);
        expect(typeof formatted).toBe('string');
        expect(formatted.length).toBeGreaterThan(0);
      }),
      { numRuns: 50 }
    );
  });

  it('expiry HTML should always contain required attributes', () => {
    fc.assert(
      fc.property(fc.integer({ min: 0, max: 7 * 24 * 60 * 60 * 1000 }), (ms) => {
        const html = createExpiryStatusHtml(ms);
        expect(html).toContain('class="expiry-status');
        expect(html).toContain('role="timer"');
        expect(html).toContain('aria-live="polite"');
      }),
      { numRuns: 30 }
    );
  });
});
