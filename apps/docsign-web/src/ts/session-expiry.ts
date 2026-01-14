/**
 * Session Expiry Warning System
 *
 * Provides proactive warnings to users before their signing session expires.
 * Designed for elderly users with clear, reassuring messages.
 *
 * Warning intervals:
 * - 1 hour before expiry
 * - 30 minutes before expiry
 * - 10 minutes before expiry
 * - 5 minutes before expiry (urgent)
 */

import { showErrorToast } from './error-ui';
import { createLogger } from './logger';

const log = createLogger('SessionExpiry');

// ============================================================
// Types
// ============================================================

export interface SessionExpiryConfig {
  /** Session creation time (Unix ms or ISO string) */
  createdAt: number | string;
  /** Session TTL in milliseconds (default: 7 days) */
  ttlMs?: number;
  /** Callback when session expires */
  onExpire?: () => void;
  /** Callback when warning is shown (for UI updates) */
  onWarning?: (minutesRemaining: number) => void;
}

interface WarningThreshold {
  minutesBefore: number;
  message: string;
  type: 'warning' | 'error';
  shown: boolean;
}

// ============================================================
// Constants
// ============================================================

const DEFAULT_TTL_MS = 7 * 24 * 60 * 60 * 1000; // 7 days

const WARNING_THRESHOLDS: WarningThreshold[] = [
  {
    minutesBefore: 60,
    message: 'Your signing session will expire in about 1 hour. Please complete your signature soon.',
    type: 'warning',
    shown: false,
  },
  {
    minutesBefore: 30,
    message: 'Your signing session will expire in 30 minutes. Please complete your signature now.',
    type: 'warning',
    shown: false,
  },
  {
    minutesBefore: 10,
    message: 'Your signing session expires in 10 minutes! Please sign the document now.',
    type: 'warning',
    shown: false,
  },
  {
    minutesBefore: 5,
    message: 'URGENT: Your signing session expires in 5 minutes! Complete your signature immediately.',
    type: 'error',
    shown: false,
  },
];

// ============================================================
// Session Expiry Watcher
// ============================================================

/**
 * Manages session expiry monitoring and warnings
 */
export class SessionExpiryWatcher {
  private config: Required<SessionExpiryConfig>;
  private expiryTime: number;
  private checkInterval: ReturnType<typeof setInterval> | null = null;
  private thresholds: WarningThreshold[];
  private isExpired = false;

  constructor(config: SessionExpiryConfig) {
    this.config = {
      createdAt: config.createdAt,
      ttlMs: config.ttlMs ?? DEFAULT_TTL_MS,
      onExpire: config.onExpire ?? (() => {}),
      onWarning: config.onWarning ?? (() => {}),
    };

    // Calculate expiry time
    const createdTime =
      typeof this.config.createdAt === 'string'
        ? new Date(this.config.createdAt).getTime()
        : this.config.createdAt;

    this.expiryTime = createdTime + this.config.ttlMs;

    // Clone thresholds to track shown state per instance
    this.thresholds = WARNING_THRESHOLDS.map((t) => ({ ...t }));

    log.debug('SessionExpiryWatcher created', {
      createdAt: this.config.createdAt,
      expiryTime: new Date(this.expiryTime).toISOString(),
      ttlMs: this.config.ttlMs,
    });
  }

  /**
   * Start monitoring for session expiry
   * Checks every 30 seconds
   */
  start(): void {
    if (this.checkInterval) {
      log.warn('SessionExpiryWatcher already started');
      return;
    }

    log.info('Starting session expiry monitoring');

    // Check immediately
    this.checkExpiry();

    // Then check every 30 seconds
    this.checkInterval = setInterval(() => {
      this.checkExpiry();
    }, 30 * 1000);
  }

  /**
   * Stop monitoring
   */
  stop(): void {
    if (this.checkInterval) {
      clearInterval(this.checkInterval);
      this.checkInterval = null;
      log.info('Stopped session expiry monitoring');
    }
  }

  /**
   * Get remaining time until expiry in milliseconds
   */
  getTimeRemaining(): number {
    return Math.max(0, this.expiryTime - Date.now());
  }

  /**
   * Get remaining time formatted for display
   */
  getTimeRemainingFormatted(): string {
    const remaining = this.getTimeRemaining();

    if (remaining <= 0) {
      return 'Expired';
    }

    const minutes = Math.floor(remaining / (60 * 1000));
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (days > 0) {
      return `${days} day${days > 1 ? 's' : ''} remaining`;
    }
    if (hours > 0) {
      return `${hours} hour${hours > 1 ? 's' : ''} remaining`;
    }
    if (minutes > 0) {
      return `${minutes} minute${minutes > 1 ? 's' : ''} remaining`;
    }
    return 'Less than 1 minute remaining';
  }

  /**
   * Check if session has expired
   */
  hasExpired(): boolean {
    return this.isExpired || Date.now() >= this.expiryTime;
  }

  /**
   * Internal: Check expiry and show warnings
   */
  private checkExpiry(): void {
    const now = Date.now();
    const remaining = this.expiryTime - now;
    const minutesRemaining = remaining / (60 * 1000);

    // Check if expired
    if (remaining <= 0 && !this.isExpired) {
      this.isExpired = true;
      log.warn('Session has expired');
      this.stop();
      this.config.onExpire();
      return;
    }

    // Check warning thresholds
    for (const threshold of this.thresholds) {
      if (!threshold.shown && minutesRemaining <= threshold.minutesBefore && minutesRemaining > 0) {
        threshold.shown = true;
        log.info(`Showing expiry warning: ${threshold.minutesBefore} minutes`);

        showErrorToast(threshold.message, threshold.type, 10000);
        this.config.onWarning(Math.floor(minutesRemaining));
      }
    }
  }
}

// ============================================================
// Utility Functions
// ============================================================

/**
 * Calculate time remaining until expiry
 */
export function getSessionTimeRemaining(
  createdAt: number | string,
  ttlMs: number = DEFAULT_TTL_MS
): number {
  const createdTime = typeof createdAt === 'string' ? new Date(createdAt).getTime() : createdAt;

  if (isNaN(createdTime)) {
    return 0;
  }

  const expiryTime = createdTime + ttlMs;
  return Math.max(0, expiryTime - Date.now());
}

/**
 * Format remaining time for display
 */
export function formatTimeRemaining(remainingMs: number): string {
  if (remainingMs <= 0) {
    return 'Expired';
  }

  const seconds = Math.floor(remainingMs / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) {
    const remainingHours = hours % 24;
    if (remainingHours > 0) {
      return `${days}d ${remainingHours}h`;
    }
    return `${days} day${days > 1 ? 's' : ''}`;
  }

  if (hours > 0) {
    const remainingMinutes = minutes % 60;
    if (remainingMinutes > 0) {
      return `${hours}h ${remainingMinutes}m`;
    }
    return `${hours} hour${hours > 1 ? 's' : ''}`;
  }

  if (minutes > 0) {
    return `${minutes} minute${minutes > 1 ? 's' : ''}`;
  }

  return 'Less than 1 minute';
}

/**
 * Create an expiry status element for the UI
 * Returns an HTML string with appropriate styling
 */
export function createExpiryStatusHtml(remainingMs: number): string {
  const formatted = formatTimeRemaining(remainingMs);
  const minutes = remainingMs / (60 * 1000);

  let className = 'expiry-status';
  let icon = '⏱️';

  if (remainingMs <= 0) {
    className += ' expiry-status-expired';
    icon = '⚠️';
  } else if (minutes <= 10) {
    className += ' expiry-status-urgent';
    icon = '⚠️';
  } else if (minutes <= 60) {
    className += ' expiry-status-warning';
    icon = '⏰';
  }

  return `<span class="${className}" role="timer" aria-live="polite">${icon} ${formatted}</span>`;
}

// ============================================================
// Global Instance Management
// ============================================================

let globalWatcher: SessionExpiryWatcher | null = null;

/**
 * Initialize session expiry monitoring for the current session
 */
export function initSessionExpiryMonitoring(config: SessionExpiryConfig): SessionExpiryWatcher {
  // Stop existing watcher
  if (globalWatcher) {
    globalWatcher.stop();
  }

  globalWatcher = new SessionExpiryWatcher(config);
  globalWatcher.start();

  return globalWatcher;
}

/**
 * Stop session expiry monitoring
 */
export function stopSessionExpiryMonitoring(): void {
  if (globalWatcher) {
    globalWatcher.stop();
    globalWatcher = null;
  }
}

/**
 * Get the current session expiry watcher (if any)
 */
export function getSessionExpiryWatcher(): SessionExpiryWatcher | null {
  return globalWatcher;
}
