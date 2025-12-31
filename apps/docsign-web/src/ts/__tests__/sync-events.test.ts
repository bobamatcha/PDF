/**
 * Property-based tests for Sync Events
 *
 * @vitest-environment jsdom
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as fc from 'fast-check';

// ============================================================
// Types (copied from sync-events.ts for testing)
// ============================================================

const SYNC_EVENTS = {
  STARTED: 'docsign:sync-started',
  COMPLETED: 'docsign:sync-completed',
  FAILED: 'docsign:sync-failed',
  PROGRESS: 'docsign:sync-progress',
  ONLINE_STATUS_CHANGED: 'docsign:online-status-changed',
} as const;

type SyncEventType = (typeof SYNC_EVENTS)[keyof typeof SYNC_EVENTS];

interface SyncStartedDetail {
  pendingCount: number;
  timestamp: string;
}

interface SyncCompletedDetail {
  syncedCount: number;
  timestamp: string;
  durationMs: number;
}

interface SyncFailedDetail {
  sessionId: string;
  error: string;
  attemptCount: number;
  timestamp: string;
  willRetry: boolean;
}

interface SyncProgressDetail {
  current: number;
  total: number;
  sessionId: string;
  percentage: number;
}

interface OnlineStatusChangedDetail {
  online: boolean;
  timestamp: string;
}

// ============================================================
// Event Dispatchers (mimicking sync-events.ts)
// ============================================================

function dispatchSyncStarted(detail: SyncStartedDetail): void {
  const event = new CustomEvent(SYNC_EVENTS.STARTED, {
    detail,
    bubbles: true,
  });
  window.dispatchEvent(event);
}

function dispatchSyncCompleted(detail: SyncCompletedDetail): void {
  const event = new CustomEvent(SYNC_EVENTS.COMPLETED, {
    detail,
    bubbles: true,
  });
  window.dispatchEvent(event);
}

function dispatchSyncFailed(detail: SyncFailedDetail): void {
  const event = new CustomEvent(SYNC_EVENTS.FAILED, {
    detail,
    bubbles: true,
  });
  window.dispatchEvent(event);
}

function dispatchSyncProgress(detail: SyncProgressDetail): void {
  const event = new CustomEvent(SYNC_EVENTS.PROGRESS, {
    detail,
    bubbles: true,
  });
  window.dispatchEvent(event);
}

function dispatchOnlineStatusChanged(detail: OnlineStatusChangedDetail): void {
  const event = new CustomEvent(SYNC_EVENTS.ONLINE_STATUS_CHANGED, {
    detail,
    bubbles: true,
  });
  window.dispatchEvent(event);
}

// ============================================================
// Event Listeners (mimicking sync-events.ts)
// ============================================================

function onSyncStarted(callback: (detail: SyncStartedDetail) => void): () => void {
  const handler = (e: Event) => {
    callback((e as CustomEvent<SyncStartedDetail>).detail);
  };
  window.addEventListener(SYNC_EVENTS.STARTED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.STARTED, handler);
}

function onSyncCompleted(callback: (detail: SyncCompletedDetail) => void): () => void {
  const handler = (e: Event) => {
    callback((e as CustomEvent<SyncCompletedDetail>).detail);
  };
  window.addEventListener(SYNC_EVENTS.COMPLETED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.COMPLETED, handler);
}

function onSyncFailed(callback: (detail: SyncFailedDetail) => void): () => void {
  const handler = (e: Event) => {
    callback((e as CustomEvent<SyncFailedDetail>).detail);
  };
  window.addEventListener(SYNC_EVENTS.FAILED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.FAILED, handler);
}

function onSyncProgress(callback: (detail: SyncProgressDetail) => void): () => void {
  const handler = (e: Event) => {
    callback((e as CustomEvent<SyncProgressDetail>).detail);
  };
  window.addEventListener(SYNC_EVENTS.PROGRESS, handler);
  return () => window.removeEventListener(SYNC_EVENTS.PROGRESS, handler);
}

function onOnlineStatusChanged(callback: (detail: OnlineStatusChangedDetail) => void): () => void {
  const handler = (e: Event) => {
    callback((e as CustomEvent<OnlineStatusChangedDetail>).detail);
  };
  window.addEventListener(SYNC_EVENTS.ONLINE_STATUS_CHANGED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.ONLINE_STATUS_CHANGED, handler);
}

// ============================================================
// Property Tests
// ============================================================

describe('Sync Events Constants', () => {
  it('should have unique event types', () => {
    const values = Object.values(SYNC_EVENTS);
    const uniqueValues = new Set(values);
    expect(uniqueValues.size).toBe(values.length);
  });

  it('should have docsign: prefix on all events', () => {
    Object.values(SYNC_EVENTS).forEach((eventType) => {
      expect(eventType.startsWith('docsign:')).toBe(true);
    });
  });

  it('should have 5 event types', () => {
    expect(Object.keys(SYNC_EVENTS).length).toBe(5);
  });
});

describe('Event Dispatch and Receive', () => {
  describe('SyncStarted', () => {
    it('should dispatch and receive sync-started events', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 100 }),
          fc.string({ minLength: 10, maxLength: 30 }),
          (pendingCount, timestamp) => {
            const received: SyncStartedDetail[] = [];
            const unsubscribe = onSyncStarted((detail) => received.push(detail));

            dispatchSyncStarted({ pendingCount, timestamp });

            expect(received.length).toBe(1);
            expect(received[0].pendingCount).toBe(pendingCount);
            expect(received[0].timestamp).toBe(timestamp);

            unsubscribe();
          }
        ),
        { numRuns: 20 }
      );
    });
  });

  describe('SyncCompleted', () => {
    it('should dispatch and receive sync-completed events', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 100 }),
          fc.string({ minLength: 10, maxLength: 30 }),
          fc.integer({ min: 0, max: 60000 }),
          (syncedCount, timestamp, durationMs) => {
            const received: SyncCompletedDetail[] = [];
            const unsubscribe = onSyncCompleted((detail) => received.push(detail));

            dispatchSyncCompleted({ syncedCount, timestamp, durationMs });

            expect(received.length).toBe(1);
            expect(received[0].syncedCount).toBe(syncedCount);
            expect(received[0].timestamp).toBe(timestamp);
            expect(received[0].durationMs).toBe(durationMs);

            unsubscribe();
          }
        ),
        { numRuns: 20 }
      );
    });
  });

  describe('SyncFailed', () => {
    it('should dispatch and receive sync-failed events', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 8, maxLength: 36 }),
          fc.string({ minLength: 5, maxLength: 100 }),
          fc.integer({ min: 0, max: 10 }),
          fc.string({ minLength: 10, maxLength: 30 }),
          fc.boolean(),
          (sessionId, error, attemptCount, timestamp, willRetry) => {
            const received: SyncFailedDetail[] = [];
            const unsubscribe = onSyncFailed((detail) => received.push(detail));

            dispatchSyncFailed({ sessionId, error, attemptCount, timestamp, willRetry });

            expect(received.length).toBe(1);
            expect(received[0].sessionId).toBe(sessionId);
            expect(received[0].error).toBe(error);
            expect(received[0].attemptCount).toBe(attemptCount);
            expect(received[0].willRetry).toBe(willRetry);

            unsubscribe();
          }
        ),
        { numRuns: 20 }
      );
    });
  });

  describe('SyncProgress', () => {
    it('should dispatch and receive sync-progress events', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 1, max: 100 }),
          fc.integer({ min: 1, max: 100 }),
          fc.string({ minLength: 8, maxLength: 36 }),
          (current, total, sessionId) => {
            // Ensure current <= total
            const actualCurrent = Math.min(current, total);
            const percentage = Math.round((actualCurrent / total) * 100);

            const received: SyncProgressDetail[] = [];
            const unsubscribe = onSyncProgress((detail) => received.push(detail));

            dispatchSyncProgress({
              current: actualCurrent,
              total,
              sessionId,
              percentage,
            });

            expect(received.length).toBe(1);
            expect(received[0].current).toBe(actualCurrent);
            expect(received[0].total).toBe(total);
            expect(received[0].sessionId).toBe(sessionId);
            expect(received[0].percentage).toBe(percentage);

            unsubscribe();
          }
        ),
        { numRuns: 20 }
      );
    });

    it('should have percentage between 0 and 100', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 1, max: 100 }),
          (total) => {
            // current should always be <= total for valid progress
            for (let current = 0; current <= total; current++) {
              const percentage = Math.round((current / total) * 100);
              expect(percentage).toBeGreaterThanOrEqual(0);
              expect(percentage).toBeLessThanOrEqual(100);
            }
          }
        ),
        { numRuns: 20 }
      );
    });
  });

  describe('OnlineStatusChanged', () => {
    it('should dispatch and receive online-status-changed events', () => {
      fc.assert(
        fc.property(fc.boolean(), fc.string({ minLength: 10, maxLength: 30 }), (online, timestamp) => {
          const received: OnlineStatusChangedDetail[] = [];
          const unsubscribe = onOnlineStatusChanged((detail) => received.push(detail));

          dispatchOnlineStatusChanged({ online, timestamp });

          expect(received.length).toBe(1);
          expect(received[0].online).toBe(online);
          expect(received[0].timestamp).toBe(timestamp);

          unsubscribe();
        }),
        { numRuns: 10 }
      );
    });
  });
});

describe('Event Listener Cleanup', () => {
  it('should not receive events after unsubscribe', () => {
    const received: SyncStartedDetail[] = [];
    const unsubscribe = onSyncStarted((detail) => received.push(detail));

    // First dispatch - should receive
    dispatchSyncStarted({ pendingCount: 1, timestamp: 'test1' });
    expect(received.length).toBe(1);

    // Unsubscribe
    unsubscribe();

    // Second dispatch - should NOT receive
    dispatchSyncStarted({ pendingCount: 2, timestamp: 'test2' });
    expect(received.length).toBe(1); // Still 1, not 2
  });

  it('should support multiple listeners for same event', () => {
    const received1: SyncStartedDetail[] = [];
    const received2: SyncStartedDetail[] = [];

    const unsubscribe1 = onSyncStarted((detail) => received1.push(detail));
    const unsubscribe2 = onSyncStarted((detail) => received2.push(detail));

    dispatchSyncStarted({ pendingCount: 5, timestamp: 'test' });

    expect(received1.length).toBe(1);
    expect(received2.length).toBe(1);

    // Unsubscribe first listener
    unsubscribe1();

    dispatchSyncStarted({ pendingCount: 10, timestamp: 'test2' });

    expect(received1.length).toBe(1); // Still 1
    expect(received2.length).toBe(2); // Now 2

    unsubscribe2();
  });
});

describe('Event Bubbling', () => {
  it('should create events with bubbles: true', () => {
    let bubbles = false;

    const handler = (e: Event) => {
      bubbles = e.bubbles;
    };

    window.addEventListener(SYNC_EVENTS.STARTED, handler);
    dispatchSyncStarted({ pendingCount: 1, timestamp: 'test' });
    window.removeEventListener(SYNC_EVENTS.STARTED, handler);

    expect(bubbles).toBe(true);
  });
});

describe('Detail Type Safety', () => {
  it('should preserve detail object structure', () => {
    fc.assert(
      fc.property(
        fc.record({
          pendingCount: fc.integer({ min: 0, max: 1000 }),
          timestamp: fc.string({ minLength: 1, maxLength: 50 }),
        }),
        (detail) => {
          let receivedDetail: SyncStartedDetail | null = null;
          const unsubscribe = onSyncStarted((d) => {
            receivedDetail = d;
          });

          dispatchSyncStarted(detail);

          expect(receivedDetail).not.toBeNull();
          expect(receivedDetail).toEqual(detail);

          unsubscribe();
        }
      ),
      { numRuns: 20 }
    );
  });
});
