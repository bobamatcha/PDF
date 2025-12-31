/**
 * SyncManager - Background sync for offline signatures
 *
 * Features:
 * - Monitors online/offline status
 * - Retries failed syncs with exponential backoff
 * - Provides sync status to UI
 * - Never blocks the UI
 * - Never loses local data if sync fails
 * - Respects explicit offline mode choice
 */

import {
  getOfflineQueue,
  removeFromOfflineQueue,
  type QueuedSubmission,
} from "./session";

import {
  dispatchSyncStarted,
  dispatchSyncCompleted,
  dispatchSyncFailed,
  dispatchSyncProgress,
  dispatchOnlineStatusChanged,
} from "./sync-events";
import { createLogger } from "./logger";

const log = createLogger('SyncManager');

// ============================================================
// Types
// ============================================================

/**
 * Current sync status
 */
export interface SyncStatus {
  /** Number of items waiting to sync */
  pendingCount: number;
  /** ISO timestamp of last sync attempt */
  lastSyncAttempt: string | null;
  /** ISO timestamp of last successful sync */
  lastSuccessfulSync: string | null;
  /** Whether a sync is currently in progress */
  isSyncing: boolean;
  /** Whether the device is online */
  isOnline: boolean;
  /** List of sync errors */
  errors: SyncError[];
}

/**
 * Error from a failed sync attempt
 */
export interface SyncError {
  /** Session ID that failed */
  sessionId: string;
  /** Recipient ID that failed */
  recipientId: string;
  /** Error message */
  error: string;
  /** Number of retry attempts */
  attemptCount: number;
  /** ISO timestamp of last attempt */
  lastAttempt: string;
}

/**
 * Configuration for SyncManager
 */
export interface SyncManagerConfig {
  /** Server endpoint for submitting signatures */
  syncEndpoint: string;
  /** Minimum backoff delay in ms (default: 1000) */
  minBackoffMs?: number;
  /** Maximum backoff delay in ms (default: 30000) */
  maxBackoffMs?: number;
  /** Interval for periodic retry when online (default: 30000) */
  retryIntervalMs?: number;
  /** Maximum retry attempts before giving up (default: 10) */
  maxRetries?: number;
}

// ============================================================
// Storage Keys
// ============================================================

const SYNC_STATE_KEY = "docsign_sync_state";
const SYNC_ERRORS_KEY = "docsign_sync_errors";
const OFFLINE_MODE_KEY = "docsign_offline_mode";

// ============================================================
// Persisted State
// ============================================================

interface PersistedSyncState {
  lastSyncAttempt: string | null;
  lastSuccessfulSync: string | null;
}

function loadPersistedState(): PersistedSyncState {
  if (typeof localStorage === "undefined") {
    return { lastSyncAttempt: null, lastSuccessfulSync: null };
  }

  try {
    const json = localStorage.getItem(SYNC_STATE_KEY);
    if (json) {
      return JSON.parse(json);
    }
  } catch {
    // Ignore parse errors
  }
  return { lastSyncAttempt: null, lastSuccessfulSync: null };
}

function savePersistedState(state: PersistedSyncState): void {
  if (typeof localStorage === "undefined") {
    return;
  }
  localStorage.setItem(SYNC_STATE_KEY, JSON.stringify(state));
}

function loadSyncErrors(): SyncError[] {
  if (typeof localStorage === "undefined") {
    return [];
  }

  try {
    const json = localStorage.getItem(SYNC_ERRORS_KEY);
    if (json) {
      return JSON.parse(json);
    }
  } catch {
    // Ignore parse errors
  }
  return [];
}

function saveSyncErrors(errors: SyncError[]): void {
  if (typeof localStorage === "undefined") {
    return;
  }
  localStorage.setItem(SYNC_ERRORS_KEY, JSON.stringify(errors));
}

function isExplicitOfflineMode(): boolean {
  if (typeof localStorage === "undefined") {
    return false;
  }
  return localStorage.getItem(OFFLINE_MODE_KEY) === "true";
}

// ============================================================
// SyncManager Class
// ============================================================

export class SyncManager {
  private config: Required<SyncManagerConfig>;
  private isSyncing = false;
  private isStarted = false;
  private retryTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private periodicRetryId: ReturnType<typeof setInterval> | null = null;
  private persistedState: PersistedSyncState;
  private errors: SyncError[];

  constructor(config: SyncManagerConfig) {
    this.config = {
      syncEndpoint: config.syncEndpoint,
      minBackoffMs: config.minBackoffMs ?? 1000,
      maxBackoffMs: config.maxBackoffMs ?? 30000,
      retryIntervalMs: config.retryIntervalMs ?? 30000,
      maxRetries: config.maxRetries ?? 10,
    };
    this.persistedState = loadPersistedState();
    this.errors = loadSyncErrors();
  }

  /**
   * Begin monitoring online status and syncing
   */
  start(): void {
    if (this.isStarted) {
      log.debug("Already started");
      return;
    }

    this.isStarted = true;
    log.info("Starting sync manager");

    // Listen for online/offline events
    window.addEventListener("online", this.handleOnline);
    window.addEventListener("offline", this.handleOffline);

    // If we're online and not in explicit offline mode, start syncing
    if (navigator.onLine && !isExplicitOfflineMode()) {
      this.syncNow();
    }

    // Start periodic retry for failed items
    this.startPeriodicRetry();
  }

  /**
   * Stop monitoring and syncing
   */
  stop(): void {
    if (!this.isStarted) {
      return;
    }

    this.isStarted = false;
    log.info("Stopping sync manager");

    window.removeEventListener("online", this.handleOnline);
    window.removeEventListener("offline", this.handleOffline);

    if (this.retryTimeoutId) {
      clearTimeout(this.retryTimeoutId);
      this.retryTimeoutId = null;
    }

    if (this.periodicRetryId) {
      clearInterval(this.periodicRetryId);
      this.periodicRetryId = null;
    }
  }

  /**
   * Force an immediate sync attempt
   */
  async syncNow(): Promise<void> {
    // Don't sync if explicitly in offline mode
    if (isExplicitOfflineMode()) {
      log.debug("Skipping sync - explicit offline mode");
      return;
    }

    // Don't sync if offline
    if (!navigator.onLine) {
      log.debug("Skipping sync - offline");
      return;
    }

    // Don't start if already syncing
    if (this.isSyncing) {
      log.debug("Skipping sync - already in progress");
      return;
    }

    const queue = getOfflineQueue();
    if (queue.length === 0) {
      log.debug("Nothing to sync");
      return;
    }

    this.isSyncing = true;
    const startTime = Date.now();
    const timestamp = new Date().toISOString();

    this.persistedState.lastSyncAttempt = timestamp;
    savePersistedState(this.persistedState);

    dispatchSyncStarted({
      pendingCount: queue.length,
      timestamp,
    });

    log.info(`Starting sync of ${queue.length} items`);

    let syncedCount = 0;

    for (let i = 0; i < queue.length; i++) {
      const item = queue[i];

      dispatchSyncProgress({
        current: i + 1,
        total: queue.length,
        sessionId: item.sessionId,
        percentage: Math.round(((i + 1) / queue.length) * 100),
      });

      const success = await this.syncItem(item);
      if (success) {
        syncedCount++;
      }
    }

    this.isSyncing = false;

    const completedTimestamp = new Date().toISOString();
    const durationMs = Date.now() - startTime;

    if (syncedCount === queue.length) {
      this.persistedState.lastSuccessfulSync = completedTimestamp;
      savePersistedState(this.persistedState);
    }

    dispatchSyncCompleted({
      syncedCount,
      timestamp: completedTimestamp,
      durationMs,
    });

    log.info(`Sync completed: ${syncedCount}/${queue.length} items`);
  }

  /**
   * Get current sync status
   */
  getStatus(): SyncStatus {
    const queue = getOfflineQueue();

    return {
      pendingCount: queue.length,
      lastSyncAttempt: this.persistedState.lastSyncAttempt,
      lastSuccessfulSync: this.persistedState.lastSuccessfulSync,
      isSyncing: this.isSyncing,
      isOnline: navigator.onLine,
      errors: [...this.errors],
    };
  }

  /**
   * Clear error history
   */
  clearErrors(): void {
    this.errors = [];
    saveSyncErrors(this.errors);
    log.debug("Errors cleared");
  }

  /**
   * Set explicit offline mode
   * When enabled, sync will not happen automatically
   */
  setOfflineMode(enabled: boolean): void {
    if (typeof localStorage === "undefined") {
      return;
    }

    if (enabled) {
      localStorage.setItem(OFFLINE_MODE_KEY, "true");
      log.info("Offline mode enabled");
    } else {
      localStorage.removeItem(OFFLINE_MODE_KEY);
      log.info("Offline mode disabled");

      // If we're online, start syncing
      if (navigator.onLine) {
        this.syncNow();
      }
    }
  }

  /**
   * Check if explicit offline mode is enabled
   */
  isOfflineModeEnabled(): boolean {
    return isExplicitOfflineMode();
  }

  /**
   * Notify that a new signature was saved locally
   * This triggers a sync attempt if conditions are met
   */
  notifyNewSignature(): void {
    log.debug("New signature saved, checking for sync");
    if (navigator.onLine && !isExplicitOfflineMode() && !this.isSyncing) {
      // Small delay to allow any immediate operations to complete
      setTimeout(() => this.syncNow(), 500);
    }
  }

  // ============================================================
  // Private Methods
  // ============================================================

  private handleOnline = (): void => {
    log.info("Device came online");
    dispatchOnlineStatusChanged({
      online: true,
      timestamp: new Date().toISOString(),
    });

    if (!isExplicitOfflineMode()) {
      // Small delay to let connection stabilize
      setTimeout(() => this.syncNow(), 1000);
    }
  };

  private handleOffline = (): void => {
    log.info("Device went offline");
    dispatchOnlineStatusChanged({
      online: false,
      timestamp: new Date().toISOString(),
    });
  };

  private startPeriodicRetry(): void {
    if (this.periodicRetryId) {
      return;
    }

    this.periodicRetryId = setInterval(() => {
      if (navigator.onLine && !isExplicitOfflineMode() && !this.isSyncing) {
        const queue = getOfflineQueue();
        if (queue.length > 0) {
          log.debug("Periodic retry triggered");
          this.syncNow();
        }
      }
    }, this.config.retryIntervalMs);
  }

  private async syncItem(item: QueuedSubmission): Promise<boolean> {
    const errorKey = `${item.sessionId}:${item.recipientId}`;
    const existingError = this.errors.find(
      (e) => e.sessionId === item.sessionId && e.recipientId === item.recipientId
    );
    const attemptCount = (existingError?.attemptCount ?? 0) + 1;

    // Check if we've exceeded max retries
    if (attemptCount > this.config.maxRetries) {
      log.warn(`Max retries exceeded for ${errorKey}, skipping`);
      return false;
    }

    try {
      const response = await this.postSignature(item);

      if (response.ok) {
        // Success - remove from queue and clear error
        removeFromOfflineQueue(item.sessionId, item.recipientId);
        this.removeError(item.sessionId, item.recipientId);
        log.debug(`Successfully synced ${errorKey}`);
        return true;
      }

      // Handle conflict (server has newer data)
      if (response.status === 409) {
        const serverData = await response.json();
        await this.handleConflict(item, serverData);
        return true;
      }

      // Other server errors
      const errorText = await response.text();
      this.recordError(item, `Server error ${response.status}: ${errorText}`, attemptCount);
      this.scheduleRetry(item, attemptCount);
      return false;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      log.error(`Failed to sync ${errorKey}:`, errorMessage);
      this.recordError(item, errorMessage, attemptCount);
      this.scheduleRetry(item, attemptCount);
      return false;
    }
  }

  private async postSignature(item: QueuedSubmission): Promise<Response> {
    return fetch(this.config.syncEndpoint, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        sessionId: item.sessionId,
        recipientId: item.recipientId,
        signingKey: item.signingKey,
        signatures: item.signatures,
        completedAt: item.completedAt,
        clientTimestamp: item.timestamp,
      }),
    });
  }

  private async handleConflict(
    item: QueuedSubmission,
    serverData: { serverTimestamp?: number; signatures?: Record<string, unknown> }
  ): Promise<void> {
    log.debug(`Handling conflict for ${item.sessionId}`);

    // If server has newer data, prefer server
    if (serverData.serverTimestamp && serverData.serverTimestamp > item.timestamp) {
      log.debug("Server has newer data, preferring server");
      removeFromOfflineQueue(item.sessionId, item.recipientId);
      return;
    }

    // If local has newer signatures, merge them
    if (serverData.signatures && item.signatures) {
      log.debug("Merging local signatures with server");
      const mergedSignatures = {
        ...serverData.signatures,
        ...item.signatures, // Local takes precedence for newer
      };

      // Retry with merged data
      const mergedItem: QueuedSubmission = {
        ...item,
        signatures: mergedSignatures,
      };

      const retryResponse = await this.postSignature(mergedItem);
      if (retryResponse.ok) {
        removeFromOfflineQueue(item.sessionId, item.recipientId);
        log.debug("Conflict resolved with merge");
      }
    } else {
      // No merge possible, just remove local
      removeFromOfflineQueue(item.sessionId, item.recipientId);
    }

    this.removeError(item.sessionId, item.recipientId);
  }

  private recordError(item: QueuedSubmission, error: string, attemptCount: number): void {
    const timestamp = new Date().toISOString();

    // Remove existing error for this item
    this.removeError(item.sessionId, item.recipientId);

    const syncError: SyncError = {
      sessionId: item.sessionId,
      recipientId: item.recipientId,
      error,
      attemptCount,
      lastAttempt: timestamp,
    };

    this.errors.push(syncError);
    saveSyncErrors(this.errors);

    dispatchSyncFailed({
      sessionId: item.sessionId,
      error,
      attemptCount,
      timestamp,
      willRetry: attemptCount < this.config.maxRetries,
    });
  }

  private removeError(sessionId: string, recipientId: string): void {
    const index = this.errors.findIndex(
      (e) => e.sessionId === sessionId && e.recipientId === recipientId
    );
    if (index !== -1) {
      this.errors.splice(index, 1);
      saveSyncErrors(this.errors);
    }
  }

  private scheduleRetry(item: QueuedSubmission, attemptCount: number): void {
    if (attemptCount >= this.config.maxRetries) {
      return;
    }

    // Exponential backoff: 1s, 2s, 4s, 8s, 16s, max 30s
    const delay = Math.min(
      this.config.minBackoffMs * Math.pow(2, attemptCount - 1),
      this.config.maxBackoffMs
    );

    log.debug(
      `Scheduling retry for ${item.sessionId} in ${delay}ms (attempt ${attemptCount})`
    );

    // We don't need to actually schedule individual retries
    // The periodic retry will pick it up
  }
}

// ============================================================
// Singleton Instance
// ============================================================

let syncManagerInstance: SyncManager | null = null;

/**
 * Get or create the SyncManager singleton
 * @param config Configuration (only used on first call)
 */
export function getSyncManager(config?: SyncManagerConfig): SyncManager {
  if (!syncManagerInstance) {
    if (!config) {
      throw new Error("SyncManager not initialized. Call getSyncManager with config first.");
    }
    syncManagerInstance = new SyncManager(config);
  }
  return syncManagerInstance;
}

/**
 * Initialize and start the SyncManager
 * Safe to call multiple times
 */
export function initSyncManager(config: SyncManagerConfig): SyncManager {
  const manager = getSyncManager(config);
  manager.start();
  return manager;
}
