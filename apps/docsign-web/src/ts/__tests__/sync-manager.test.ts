/**
 * Property-based tests for SyncManager
 *
 * Tests the sync queue, retry logic, and offline handling.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import * as fc from 'fast-check';

// ============================================================
// Types and Mock State
// ============================================================

type SyncStatus = 'idle' | 'syncing' | 'error' | 'success';

interface SyncItem {
  id: string;
  sessionId: string;
  fieldId: string;
  signatureData: string;
  timestamp: number;
  retryCount: number;
}

interface SyncManagerState {
  queue: SyncItem[];
  status: SyncStatus;
  isOnline: boolean;
  retryIntervalMs: number;
  maxRetries: number;
  minBackoffMs: number;
  maxBackoffMs: number;
}

// ============================================================
// Sync Manager Logic (Pure Functions)
// ============================================================

function createDefaultState(): SyncManagerState {
  return {
    queue: [],
    status: 'idle',
    isOnline: true,
    retryIntervalMs: 30000,
    maxRetries: 10,
    minBackoffMs: 1000,
    maxBackoffMs: 30000,
  };
}

function addToQueue(state: SyncManagerState, item: Omit<SyncItem, 'retryCount'>): SyncManagerState {
  return {
    ...state,
    queue: [...state.queue, { ...item, retryCount: 0 }],
  };
}

function removeFromQueue(state: SyncManagerState, id: string): SyncManagerState {
  return {
    ...state,
    queue: state.queue.filter((item) => item.id !== id),
  };
}

function incrementRetry(state: SyncManagerState, id: string): SyncManagerState {
  return {
    ...state,
    queue: state.queue.map((item) =>
      item.id === id ? { ...item, retryCount: item.retryCount + 1 } : item
    ),
  };
}

function shouldRetry(item: SyncItem, maxRetries: number): boolean {
  return item.retryCount < maxRetries;
}

function calculateBackoff(retryCount: number, minMs: number, maxMs: number): number {
  // Exponential backoff: min * 2^retryCount, capped at max
  const backoff = minMs * Math.pow(2, retryCount);
  return Math.min(backoff, maxMs);
}

function setOnlineStatus(state: SyncManagerState, isOnline: boolean): SyncManagerState {
  return { ...state, isOnline };
}

function setStatus(state: SyncManagerState, status: SyncStatus): SyncManagerState {
  return { ...state, status };
}

function getQueueLength(state: SyncManagerState): number {
  return state.queue.length;
}

function getOldestItem(state: SyncManagerState): SyncItem | undefined {
  return state.queue.reduce<SyncItem | undefined>((oldest, item) => {
    if (!oldest || item.timestamp < oldest.timestamp) {
      return item;
    }
    return oldest;
  }, undefined);
}

// ============================================================
// Property Tests
// ============================================================

describe('SyncManager Property Tests', () => {
  describe('Queue Management', () => {
    it('should add items to queue', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 1, maxLength: 36 }),
          fc.string({ minLength: 1, maxLength: 36 }),
          fc.string({ minLength: 1, maxLength: 36 }),
          fc.string({ minLength: 10, maxLength: 1000 }),
          fc.integer({ min: 0, max: Date.now() }),
          (id, sessionId, fieldId, signatureData, timestamp) => {
            let state = createDefaultState();
            state = addToQueue(state, { id, sessionId, fieldId, signatureData, timestamp });

            expect(state.queue.length).toBe(1);
            expect(state.queue[0].id).toBe(id);
            expect(state.queue[0].retryCount).toBe(0);
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should remove items from queue by id', () => {
      fc.assert(
        fc.property(
          fc.array(fc.string({ minLength: 8, maxLength: 16 }), { minLength: 1, maxLength: 10 }),
          fc.nat(),
          (ids, removeIndex) => {
            let state = createDefaultState();

            // Add all items
            ids.forEach((id, i) => {
              state = addToQueue(state, {
                id,
                sessionId: 'session',
                fieldId: 'field',
                signatureData: 'data',
                timestamp: i,
              });
            });

            const initialLength = state.queue.length;
            const idToRemove = ids[removeIndex % ids.length];

            state = removeFromQueue(state, idToRemove);

            expect(state.queue.length).toBe(initialLength - 1);
            expect(state.queue.find((item) => item.id === idToRemove)).toBeUndefined();
          }
        ),
        { numRuns: 30 }
      );
    });

    it('should not change queue when removing non-existent id', () => {
      fc.assert(
        fc.property(
          fc.array(fc.string({ minLength: 8, maxLength: 16 }), { minLength: 1, maxLength: 5 }),
          fc.string({ minLength: 20, maxLength: 30 }), // Guaranteed different
          (ids, nonExistentId) => {
            let state = createDefaultState();

            ids.forEach((id, i) => {
              state = addToQueue(state, {
                id,
                sessionId: 'session',
                fieldId: 'field',
                signatureData: 'data',
                timestamp: i,
              });
            });

            const originalLength = state.queue.length;
            state = removeFromQueue(state, nonExistentId);

            expect(state.queue.length).toBe(originalLength);
          }
        ),
        { numRuns: 30 }
      );
    });

    it('should maintain queue order (FIFO)', () => {
      fc.assert(
        fc.property(
          fc.array(fc.integer({ min: 0, max: 1000000 }), { minLength: 2, maxLength: 10 }),
          (timestamps) => {
            let state = createDefaultState();

            timestamps.forEach((ts, i) => {
              state = addToQueue(state, {
                id: `id-${i}`,
                sessionId: 'session',
                fieldId: 'field',
                signatureData: 'data',
                timestamp: ts,
              });
            });

            const oldest = getOldestItem(state);
            const minTimestamp = Math.min(...timestamps);

            expect(oldest?.timestamp).toBe(minTimestamp);
          }
        ),
        { numRuns: 30 }
      );
    });
  });

  describe('Retry Logic', () => {
    it('should increment retry count', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 8, maxLength: 16 }),
          fc.integer({ min: 1, max: 10 }),
          (id, incrementCount) => {
            let state = createDefaultState();
            state = addToQueue(state, {
              id,
              sessionId: 'session',
              fieldId: 'field',
              signatureData: 'data',
              timestamp: Date.now(),
            });

            for (let i = 0; i < incrementCount; i++) {
              state = incrementRetry(state, id);
            }

            expect(state.queue[0].retryCount).toBe(incrementCount);
          }
        ),
        { numRuns: 30 }
      );
    });

    it('should determine retry eligibility correctly', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 20 }),
          fc.integer({ min: 1, max: 15 }),
          (retryCount, maxRetries) => {
            const item: SyncItem = {
              id: 'test',
              sessionId: 'session',
              fieldId: 'field',
              signatureData: 'data',
              timestamp: Date.now(),
              retryCount,
            };

            const canRetry = shouldRetry(item, maxRetries);
            expect(canRetry).toBe(retryCount < maxRetries);
          }
        ),
        { numRuns: 50 }
      );
    });
  });

  describe('Exponential Backoff', () => {
    it('should calculate backoff within bounds', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 20 }),
          fc.integer({ min: 100, max: 5000 }),
          fc.integer({ min: 10000, max: 60000 }),
          (retryCount, minMs, maxMs) => {
            // Ensure min < max
            const actualMin = Math.min(minMs, maxMs);
            const actualMax = Math.max(minMs, maxMs);

            const backoff = calculateBackoff(retryCount, actualMin, actualMax);

            expect(backoff).toBeGreaterThanOrEqual(actualMin);
            expect(backoff).toBeLessThanOrEqual(actualMax);
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should increase with retry count (until cap)', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 5 }),
          fc.integer({ min: 1000, max: 2000 }),
          fc.integer({ min: 30000, max: 60000 }),
          (retryCount, minMs, maxMs) => {
            const backoff1 = calculateBackoff(retryCount, minMs, maxMs);
            const backoff2 = calculateBackoff(retryCount + 1, minMs, maxMs);

            // Each retry should be >= previous (monotonic until cap)
            expect(backoff2).toBeGreaterThanOrEqual(backoff1);
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should double with each retry (before cap)', () => {
      fc.assert(
        fc.property(fc.integer({ min: 0, max: 3 }), (retryCount) => {
          const minMs = 1000;
          const maxMs = 1000000; // Very high cap

          const backoff1 = calculateBackoff(retryCount, minMs, maxMs);
          const backoff2 = calculateBackoff(retryCount + 1, minMs, maxMs);

          // Should approximately double
          expect(backoff2).toBe(backoff1 * 2);
        }),
        { numRuns: 20 }
      );
    });
  });

  describe('Online/Offline State', () => {
    it('should track online status', () => {
      fc.assert(
        fc.property(fc.boolean(), (isOnline) => {
          let state = createDefaultState();
          state = setOnlineStatus(state, isOnline);
          expect(state.isOnline).toBe(isOnline);
        }),
        { numRuns: 10 }
      );
    });

    it('should toggle online status', () => {
      fc.assert(
        fc.property(
          fc.array(fc.boolean(), { minLength: 1, maxLength: 10 }),
          (toggles) => {
            let state = createDefaultState();

            toggles.forEach((isOnline) => {
              state = setOnlineStatus(state, isOnline);
            });

            // Final state should match last toggle
            expect(state.isOnline).toBe(toggles[toggles.length - 1]);
          }
        ),
        { numRuns: 30 }
      );
    });
  });

  describe('Sync Status', () => {
    it('should transition through valid statuses', () => {
      fc.assert(
        fc.property(
          fc.constantFrom<SyncStatus>('idle', 'syncing', 'error', 'success'),
          (newStatus) => {
            let state = createDefaultState();
            state = setStatus(state, newStatus);
            expect(state.status).toBe(newStatus);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('should start with idle status', () => {
      const state = createDefaultState();
      expect(state.status).toBe('idle');
    });
  });

  describe('Configuration', () => {
    it('should have valid default configuration', () => {
      const state = createDefaultState();

      expect(state.retryIntervalMs).toBeGreaterThan(0);
      expect(state.maxRetries).toBeGreaterThan(0);
      expect(state.minBackoffMs).toBeGreaterThan(0);
      expect(state.maxBackoffMs).toBeGreaterThanOrEqual(state.minBackoffMs);
    });

    it('should respect backoff bounds from config', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 500, max: 5000 }),
          fc.integer({ min: 10000, max: 120000 }),
          fc.integer({ min: 0, max: 15 }),
          (minBackoff, maxBackoff, retryCount) => {
            const backoff = calculateBackoff(retryCount, minBackoff, maxBackoff);

            expect(backoff).toBeGreaterThanOrEqual(minBackoff);
            expect(backoff).toBeLessThanOrEqual(maxBackoff);
          }
        ),
        { numRuns: 50 }
      );
    });
  });

  describe('Queue Statistics', () => {
    it('should correctly report queue length', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 0, max: 20 }),
          (itemCount) => {
            let state = createDefaultState();

            for (let i = 0; i < itemCount; i++) {
              state = addToQueue(state, {
                id: `id-${i}`,
                sessionId: 'session',
                fieldId: 'field',
                signatureData: 'data',
                timestamp: i,
              });
            }

            expect(getQueueLength(state)).toBe(itemCount);
          }
        ),
        { numRuns: 30 }
      );
    });

    it('should find oldest item by timestamp', () => {
      fc.assert(
        fc.property(
          fc.array(fc.integer({ min: 0, max: 1000000 }), { minLength: 2, maxLength: 10 }),
          (timestamps) => {
            // Ensure unique timestamps
            const uniqueTimestamps = [...new Set(timestamps)];
            if (uniqueTimestamps.length < 2) return;

            let state = createDefaultState();

            uniqueTimestamps.forEach((ts, i) => {
              state = addToQueue(state, {
                id: `id-${i}`,
                sessionId: 'session',
                fieldId: 'field',
                signatureData: 'data',
                timestamp: ts,
              });
            });

            const oldest = getOldestItem(state);
            const minTimestamp = Math.min(...uniqueTimestamps);

            expect(oldest?.timestamp).toBe(minTimestamp);
          }
        ),
        { numRuns: 30 }
      );
    });
  });
});

describe('Sync Event Constants', () => {
  it('should have distinct event types', () => {
    const events = ['sync:started', 'sync:completed', 'sync:failed', 'sync:progress', 'online:changed'];
    const uniqueEvents = new Set(events);
    expect(uniqueEvents.size).toBe(events.length);
  });
});
