/**
 * Rate Limit Visibility Module
 *
 * Displays rate limit information to users in a friendly way.
 * Parses rate limit data from API responses and shows appropriate warnings.
 */

import { showErrorToast, showErrorModal } from './error-ui';
import { createUserError } from './error-messages';
import { createLogger } from './logger';

const log = createLogger('RateLimit');

// ============================================================
// Types
// ============================================================

/**
 * Rate limit info from API response
 */
export interface RateLimitInfo {
  remainingToday: number | null;
  remainingMonth: number | null;
}

/**
 * Rate limit status
 */
export type RateLimitStatus =
  | 'ok' // Plenty of capacity
  | 'warning' // Approaching limits
  | 'critical' // Very close to limits
  | 'exceeded'; // Limit exceeded

/**
 * Parsed rate limit result
 */
export interface RateLimitResult {
  status: RateLimitStatus;
  info: RateLimitInfo;
  message: string | null;
}

// ============================================================
// Constants
// ============================================================

const DAILY_WARNING_THRESHOLD = 10; // Warn when < 10 remaining today
const DAILY_CRITICAL_THRESHOLD = 3; // Critical when < 3 remaining today
const MONTHLY_WARNING_THRESHOLD = 100; // Warn when < 100 remaining this month
const MONTHLY_CRITICAL_THRESHOLD = 20; // Critical when < 20 remaining this month

// ============================================================
// Parsing Functions
// ============================================================

/**
 * Parse rate limit info from API response
 */
export function parseRateLimitInfo(response: unknown): RateLimitInfo {
  if (typeof response !== 'object' || response === null) {
    return { remainingToday: null, remainingMonth: null };
  }

  const obj = response as Record<string, unknown>;

  return {
    remainingToday:
      typeof obj.remaining_today === 'number' ? obj.remaining_today : null,
    remainingMonth:
      typeof obj.remaining_month === 'number' ? obj.remaining_month : null,
  };
}

/**
 * Determine rate limit status from info
 */
export function getRateLimitStatus(info: RateLimitInfo): RateLimitStatus {
  // If we have no info, assume ok
  if (info.remainingToday === null && info.remainingMonth === null) {
    return 'ok';
  }

  // Check if exceeded (0 remaining)
  if (info.remainingToday === 0 || info.remainingMonth === 0) {
    return 'exceeded';
  }

  // Check critical thresholds
  if (
    (info.remainingToday !== null && info.remainingToday < DAILY_CRITICAL_THRESHOLD) ||
    (info.remainingMonth !== null && info.remainingMonth < MONTHLY_CRITICAL_THRESHOLD)
  ) {
    return 'critical';
  }

  // Check warning thresholds
  if (
    (info.remainingToday !== null && info.remainingToday < DAILY_WARNING_THRESHOLD) ||
    (info.remainingMonth !== null && info.remainingMonth < MONTHLY_WARNING_THRESHOLD)
  ) {
    return 'warning';
  }

  return 'ok';
}

/**
 * Generate user-friendly message for rate limit status
 */
export function getRateLimitMessage(info: RateLimitInfo, status: RateLimitStatus): string | null {
  switch (status) {
    case 'exceeded':
      if (info.remainingToday === 0) {
        return "You've reached your daily limit. Please try again tomorrow.";
      }
      if (info.remainingMonth === 0) {
        return "You've reached your monthly limit. Please try again next month.";
      }
      return "You've reached your sending limit. Please try again later.";

    case 'critical':
      if (info.remainingToday !== null && info.remainingToday < DAILY_CRITICAL_THRESHOLD) {
        return `Only ${info.remainingToday} sends remaining today.`;
      }
      if (info.remainingMonth !== null && info.remainingMonth < MONTHLY_CRITICAL_THRESHOLD) {
        return `Only ${info.remainingMonth} sends remaining this month.`;
      }
      return 'You are very close to your sending limit.';

    case 'warning':
      if (info.remainingToday !== null && info.remainingToday < DAILY_WARNING_THRESHOLD) {
        return `${info.remainingToday} sends remaining today.`;
      }
      if (info.remainingMonth !== null && info.remainingMonth < MONTHLY_WARNING_THRESHOLD) {
        return `${info.remainingMonth} sends remaining this month.`;
      }
      return 'You are approaching your sending limit.';

    case 'ok':
    default:
      return null;
  }
}

/**
 * Process API response and return rate limit result
 */
export function processRateLimitResponse(response: unknown): RateLimitResult {
  const info = parseRateLimitInfo(response);
  const status = getRateLimitStatus(info);
  const message = getRateLimitMessage(info, status);

  return { status, info, message };
}

// ============================================================
// UI Functions
// ============================================================

/**
 * Show appropriate UI for rate limit status
 * Returns true if a blocking error was shown (exceeded)
 */
export function showRateLimitFeedback(result: RateLimitResult): boolean {
  const { status, message } = result;

  if (!message) {
    return false;
  }

  switch (status) {
    case 'exceeded':
      log.warn('Rate limit exceeded', result.info);
      showErrorModal(
        createUserError(
          'Sending Limit Reached',
          result.info.remainingToday === 0
            ? 'You have reached your daily sending limit. Your document is safe and will not be lost. You can try again tomorrow when your limit resets.'
            : 'You have reached your monthly sending limit. Your document is safe and will not be lost. Please contact support if you need to send more documents.',
          result.info.remainingToday === 0 ? 'Understood' : 'Contact Support',
          'clock'
        )
      );
      return true;

    case 'critical':
      log.warn('Rate limit critical', result.info);
      showErrorToast(message, 'error', 8000);
      return false;

    case 'warning':
      log.info('Rate limit warning', result.info);
      showErrorToast(message, 'warning', 5000);
      return false;

    default:
      return false;
  }
}

/**
 * Handle a 429 response from the API
 */
export function handleRateLimitExceeded(response?: unknown): void {
  const info = response ? parseRateLimitInfo(response) : { remainingToday: 0, remainingMonth: null };

  showErrorModal(
    createUserError(
      'Sending Limit Reached',
      info.remainingToday === 0
        ? 'You have reached your daily sending limit. Your document is completely safe. Please try again tomorrow after midnight when your limit resets automatically.'
        : info.remainingMonth === 0
          ? 'You have reached your monthly sending limit. Your document is completely safe. Please contact support if you need to send more documents this month.'
          : 'You have temporarily exceeded the sending limit. Please wait a moment and try again.',
      'Understood',
      'clock'
    )
  );
}

// ============================================================
// Status Display
// ============================================================

/**
 * Create HTML for rate limit status indicator
 */
export function createRateLimitStatusHtml(info: RateLimitInfo): string {
  const status = getRateLimitStatus(info);

  if (info.remainingToday === null && info.remainingMonth === null) {
    return '';
  }

  let className = 'rate-limit-status';
  let icon = '';

  switch (status) {
    case 'exceeded':
      className += ' rate-limit-exceeded';
      icon = '🚫';
      break;
    case 'critical':
      className += ' rate-limit-critical';
      icon = '⚠️';
      break;
    case 'warning':
      className += ' rate-limit-warning';
      icon = '⏳';
      break;
    default:
      className += ' rate-limit-ok';
      icon = '✓';
  }

  const parts: string[] = [];
  if (info.remainingToday !== null) {
    parts.push(`${info.remainingToday} today`);
  }
  if (info.remainingMonth !== null) {
    parts.push(`${info.remainingMonth} this month`);
  }

  return `<span class="${className}" role="status">${icon} ${parts.join(' / ')}</span>`;
}

// ============================================================
// State Management
// ============================================================

let lastKnownInfo: RateLimitInfo = { remainingToday: null, remainingMonth: null };

/**
 * Update the stored rate limit info
 */
export function updateRateLimitInfo(info: RateLimitInfo): void {
  lastKnownInfo = { ...info };
  log.debug('Rate limit info updated', info);
}

/**
 * Get the last known rate limit info
 */
export function getLastKnownRateLimitInfo(): RateLimitInfo {
  return { ...lastKnownInfo };
}

/**
 * Process an API response and update stored info
 */
export function processAndStoreRateLimitInfo(response: unknown): RateLimitResult {
  const result = processRateLimitResponse(response);
  updateRateLimitInfo(result.info);
  return result;
}
