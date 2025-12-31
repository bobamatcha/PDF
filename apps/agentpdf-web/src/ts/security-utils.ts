/**
 * Security Utilities for agentPDF
 *
 * This module provides security-related utilities including:
 * - XSS prevention
 * - Input validation
 * - Size limit enforcement
 * - Rate limit feedback
 */

// ============================================================================
// XSS Prevention
// ============================================================================

/**
 * Escape HTML entities to prevent XSS attacks.
 * Uses the browser's built-in text escaping via textContent.
 */
export function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// ============================================================================
// Input Validation
// ============================================================================

export interface ValidationResult {
  valid: boolean;
  error?: string;
}

/**
 * Validate session parameters for signing sessions.
 * Ensures all required parameters are present and have minimum length.
 */
export function validateSessionParams(params: {
  sessionId?: string;
  recipientId?: string;
  signingKey?: string;
}): ValidationResult {
  const { sessionId, recipientId, signingKey } = params;

  // Check sessionId
  if (!sessionId || sessionId.trim().length === 0) {
    return { valid: false, error: 'Session ID is required' };
  }
  if (sessionId.trim().length < 8) {
    return { valid: false, error: 'Session ID must be at least 8 characters' };
  }

  // Check recipientId
  if (!recipientId || recipientId.trim().length === 0) {
    return { valid: false, error: 'Recipient ID is required' };
  }
  if (recipientId.trim().length < 8) {
    return { valid: false, error: 'Recipient ID must be at least 8 characters' };
  }

  // Check signingKey
  if (!signingKey || signingKey.trim().length === 0) {
    return { valid: false, error: 'Signing key is required' };
  }
  if (signingKey.trim().length < 16) {
    return { valid: false, error: 'Signing key must be at least 16 characters' };
  }

  return { valid: true };
}

/**
 * Validate email format.
 * Uses a basic regex that covers most valid email addresses.
 */
export function validateEmail(email: string): ValidationResult {
  if (!email || email.trim().length === 0) {
    return { valid: false, error: 'Email is required' };
  }

  const trimmed = email.trim();

  if (trimmed.length > SIZE_LIMITS.MAX_EMAIL_LENGTH) {
    return { valid: false, error: `Email must be ${SIZE_LIMITS.MAX_EMAIL_LENGTH} characters or less` };
  }

  // Basic email regex - covers most valid addresses
  // Allows: local@domain.tld, local+tag@domain.tld, etc.
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

  if (!emailRegex.test(trimmed)) {
    return { valid: false, error: 'Invalid email format' };
  }

  return { valid: true };
}

/**
 * Validate file name for security.
 * Checks for path traversal characters and invalid patterns.
 */
export function validateFileName(name: string): ValidationResult {
  if (!name || name.trim().length === 0) {
    return { valid: false, error: 'File name is required' };
  }

  const trimmed = name.trim();

  if (trimmed.length > SIZE_LIMITS.MAX_FILENAME_LENGTH) {
    return { valid: false, error: `File name must be ${SIZE_LIMITS.MAX_FILENAME_LENGTH} characters or less` };
  }

  // Check for path traversal patterns
  if (trimmed.includes('..')) {
    return { valid: false, error: 'File name cannot contain path traversal sequences (..)' };
  }

  // Check for absolute paths
  if (trimmed.startsWith('/') || trimmed.startsWith('\\')) {
    return { valid: false, error: 'File name cannot be an absolute path' };
  }

  // Check for Windows drive letters (e.g., C:)
  if (/^[a-zA-Z]:/.test(trimmed)) {
    return { valid: false, error: 'File name cannot contain drive letters' };
  }

  // Check for null bytes
  if (trimmed.includes('\0')) {
    return { valid: false, error: 'File name cannot contain null bytes' };
  }

  // Check for directory separators in the middle
  if (trimmed.includes('/') || trimmed.includes('\\')) {
    return { valid: false, error: 'File name cannot contain directory separators' };
  }

  // Check for other dangerous characters
  const dangerousChars = ['<', '>', ':', '"', '|', '?', '*'];
  for (const char of dangerousChars) {
    if (trimmed.includes(char)) {
      return { valid: false, error: `File name cannot contain the character: ${char}` };
    }
  }

  return { valid: true };
}

// ============================================================================
// Size Limit Constants
// ============================================================================

export const SIZE_LIMITS = {
  /** Maximum PDF file size: 50 MB */
  MAX_PDF_SIZE: 50 * 1024 * 1024,
  /** Maximum signature image size: 100 KB */
  MAX_SIGNATURE_SIZE: 100 * 1024,
  /** Maximum file name length */
  MAX_FILENAME_LENGTH: 200,
  /** Maximum email length (per RFC 5321) */
  MAX_EMAIL_LENGTH: 254,
  /** Maximum recipients per dispatch request */
  MAX_RECIPIENTS_PER_REQUEST: 10,
} as const;

/**
 * Validate file size against a maximum limit.
 */
export function validateFileSize(
  size: number,
  maxSize: number = SIZE_LIMITS.MAX_PDF_SIZE
): ValidationResult {
  if (size < 0) {
    return { valid: false, error: 'Invalid file size' };
  }

  if (size === 0) {
    return { valid: false, error: 'File is empty' };
  }

  if (size > maxSize) {
    const maxMB = (maxSize / (1024 * 1024)).toFixed(1);
    const actualMB = (size / (1024 * 1024)).toFixed(1);
    return {
      valid: false,
      error: `File size (${actualMB} MB) exceeds maximum allowed (${maxMB} MB)`,
    };
  }

  return { valid: true };
}

// ============================================================================
// Rate Limit Feedback
// ============================================================================

export interface RateLimitInfo {
  remainingDaily: number;
  remainingMonthly: number;
  limitDaily: number;
  limitMonthly: number;
}

export type RateLimitStatus = 'ok' | 'warning' | 'critical' | 'exceeded';

/**
 * Calculate rate limit status based on remaining quota.
 * - 'ok': More than 50% remaining
 * - 'warning': 10-50% remaining
 * - 'critical': Less than 10% remaining
 * - 'exceeded': No quota remaining
 */
export function getRateLimitStatus(info: RateLimitInfo): RateLimitStatus {
  const { remainingDaily, remainingMonthly, limitDaily, limitMonthly } = info;

  // Check if exceeded
  if (remainingDaily <= 0 || remainingMonthly <= 0) {
    return 'exceeded';
  }

  // Calculate percentages
  const dailyPercent = limitDaily > 0 ? (remainingDaily / limitDaily) * 100 : 100;
  const monthlyPercent = limitMonthly > 0 ? (remainingMonthly / limitMonthly) * 100 : 100;

  // Use the lower of the two percentages
  const lowestPercent = Math.min(dailyPercent, monthlyPercent);

  if (lowestPercent < 10) {
    return 'critical';
  }

  if (lowestPercent < 50) {
    return 'warning';
  }

  return 'ok';
}

/**
 * Get a user-friendly message about rate limit status.
 */
export function getRateLimitMessage(info: RateLimitInfo): string {
  const { remainingDaily, remainingMonthly, limitDaily, limitMonthly } = info;
  const status = getRateLimitStatus(info);

  // Calculate percentages to determine which limit is more critical
  const dailyPercent = limitDaily > 0 ? (remainingDaily / limitDaily) * 100 : 100;
  const monthlyPercent = limitMonthly > 0 ? (remainingMonthly / limitMonthly) * 100 : 100;
  const dailyIsLower = dailyPercent <= monthlyPercent;

  switch (status) {
    case 'exceeded':
      if (remainingDaily <= 0) {
        return `Daily limit reached (${limitDaily} requests). Resets at midnight.`;
      }
      return `Monthly limit reached (${limitMonthly} requests). Resets at the start of next month.`;

    case 'critical':
      if (dailyIsLower) {
        return `Warning: Only ${remainingDaily} of ${limitDaily} daily requests remaining.`;
      }
      return `Warning: Only ${remainingMonthly} of ${limitMonthly} monthly requests remaining.`;

    case 'warning':
      if (dailyIsLower) {
        return `${remainingDaily} of ${limitDaily} daily requests remaining.`;
      }
      return `${remainingMonthly} of ${limitMonthly} monthly requests remaining.`;

    case 'ok':
    default:
      return `${remainingDaily} daily / ${remainingMonthly} monthly requests remaining.`;
  }
}
