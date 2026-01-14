/**
 * Sync Events - Custom events for sync status changes
 *
 * These events allow UI components to react to sync status without
 * tight coupling to the SyncManager implementation.
 *
 * Event types:
 * - docsign:sync-started - Sync process has begun
 * - docsign:sync-completed - All pending items synced successfully
 * - docsign:sync-failed - Sync encountered an error
 * - docsign:sync-progress - Progress update during sync
 */

// ============================================================
// Event Types
// ============================================================

export const SYNC_EVENTS = {
  STARTED: "docsign:sync-started",
  COMPLETED: "docsign:sync-completed",
  FAILED: "docsign:sync-failed",
  PROGRESS: "docsign:sync-progress",
  ONLINE_STATUS_CHANGED: "docsign:online-status-changed",
} as const;

export type SyncEventType = (typeof SYNC_EVENTS)[keyof typeof SYNC_EVENTS];

// ============================================================
// Event Detail Interfaces
// ============================================================

/**
 * Detail for sync-started event
 */
export interface SyncStartedDetail {
  /** Number of items to sync */
  pendingCount: number;
  /** Timestamp when sync started */
  timestamp: string;
}

/**
 * Detail for sync-completed event
 */
export interface SyncCompletedDetail {
  /** Number of items successfully synced */
  syncedCount: number;
  /** Timestamp when sync completed */
  timestamp: string;
  /** Duration in milliseconds */
  durationMs: number;
}

/**
 * Detail for sync-failed event
 */
export interface SyncFailedDetail {
  /** Session ID that failed to sync */
  sessionId: string;
  /** Error message */
  error: string;
  /** Number of retry attempts so far */
  attemptCount: number;
  /** Timestamp of failure */
  timestamp: string;
  /** Whether retry is scheduled */
  willRetry: boolean;
}

/**
 * Detail for sync-progress event
 */
export interface SyncProgressDetail {
  /** Current item being synced (1-indexed) */
  current: number;
  /** Total items to sync */
  total: number;
  /** Session ID of current item */
  sessionId: string;
  /** Progress percentage (0-100) */
  percentage: number;
}

/**
 * Detail for online-status-changed event
 */
export interface OnlineStatusChangedDetail {
  /** Whether currently online */
  online: boolean;
  /** Timestamp of status change */
  timestamp: string;
}

// ============================================================
// Event Dispatchers
// ============================================================

/**
 * Dispatch sync-started event
 */
export function dispatchSyncStarted(detail: SyncStartedDetail): void {
  const event = new CustomEvent(SYNC_EVENTS.STARTED, {
    detail,
    bubbles: true,
  });
  window.dispatchEvent(event);
}

/**
 * Dispatch sync-completed event
 */
export function dispatchSyncCompleted(detail: SyncCompletedDetail): void {
  const event = new CustomEvent(SYNC_EVENTS.COMPLETED, {
    detail,
    bubbles: true,
  });
  window.dispatchEvent(event);
}

/**
 * Dispatch sync-failed event
 */
export function dispatchSyncFailed(detail: SyncFailedDetail): void {
  const event = new CustomEvent(SYNC_EVENTS.FAILED, {
    detail,
    bubbles: true,
  });
  window.dispatchEvent(event);
}

/**
 * Dispatch sync-progress event
 */
export function dispatchSyncProgress(detail: SyncProgressDetail): void {
  const event = new CustomEvent(SYNC_EVENTS.PROGRESS, {
    detail,
    bubbles: true,
  });
  window.dispatchEvent(event);
}

/**
 * Dispatch online-status-changed event
 */
export function dispatchOnlineStatusChanged(detail: OnlineStatusChangedDetail): void {
  const event = new CustomEvent(SYNC_EVENTS.ONLINE_STATUS_CHANGED, {
    detail,
    bubbles: true,
  });
  window.dispatchEvent(event);
}

// ============================================================
// Event Listener Helpers
// ============================================================

/**
 * Type-safe event listener for sync-started
 */
export function onSyncStarted(
  callback: (detail: SyncStartedDetail) => void
): () => void {
  const handler = (e: Event) => {
    callback((e as CustomEvent<SyncStartedDetail>).detail);
  };
  window.addEventListener(SYNC_EVENTS.STARTED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.STARTED, handler);
}

/**
 * Type-safe event listener for sync-completed
 */
export function onSyncCompleted(
  callback: (detail: SyncCompletedDetail) => void
): () => void {
  const handler = (e: Event) => {
    callback((e as CustomEvent<SyncCompletedDetail>).detail);
  };
  window.addEventListener(SYNC_EVENTS.COMPLETED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.COMPLETED, handler);
}

/**
 * Type-safe event listener for sync-failed
 */
export function onSyncFailed(
  callback: (detail: SyncFailedDetail) => void
): () => void {
  const handler = (e: Event) => {
    callback((e as CustomEvent<SyncFailedDetail>).detail);
  };
  window.addEventListener(SYNC_EVENTS.FAILED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.FAILED, handler);
}

/**
 * Type-safe event listener for sync-progress
 */
export function onSyncProgress(
  callback: (detail: SyncProgressDetail) => void
): () => void {
  const handler = (e: Event) => {
    callback((e as CustomEvent<SyncProgressDetail>).detail);
  };
  window.addEventListener(SYNC_EVENTS.PROGRESS, handler);
  return () => window.removeEventListener(SYNC_EVENTS.PROGRESS, handler);
}

/**
 * Type-safe event listener for online-status-changed
 */
export function onOnlineStatusChanged(
  callback: (detail: OnlineStatusChangedDetail) => void
): () => void {
  const handler = (e: Event) => {
    callback((e as CustomEvent<OnlineStatusChangedDetail>).detail);
  };
  window.addEventListener(SYNC_EVENTS.ONLINE_STATUS_CHANGED, handler);
  return () => window.removeEventListener(SYNC_EVENTS.ONLINE_STATUS_CHANGED, handler);
}
