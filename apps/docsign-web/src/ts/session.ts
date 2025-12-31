/**
 * Session Management Module
 *
 * Provides session validation and management logic for the docsign web app.
 * This TypeScript module mirrors the Rust session logic for use in pure JS contexts.
 */

// ============================================================
// Types
// ============================================================

export type SessionStatus = 'pending' | 'accepted' | 'declined' | 'completed' | 'expired';

export interface SessionValidation {
  valid: boolean;
  error?: string;
}

export interface SessionParams {
  sessionId: string | null | undefined;
  recipientId: string | null | undefined;
  signingKey: string | null | undefined;
}

export interface SigningField {
  id: string;
  type: string;
  page: number;
  x: number;
  y: number;
  width: number;
  height: number;
  required: boolean;
  recipientId: string;
}

export interface QueuedSubmission {
  sessionId: string;
  recipientId: string;
  signingKey: string;
  signatures: Record<string, unknown>;
  completedAt: string;
  timestamp: number;
}

// ============================================================
// Session Validation
// ============================================================

/**
 * Validate session URL parameters
 * Returns validation result - NEVER falls back to mock data
 *
 * Validation rules:
 * - Session ID must be non-empty and at least 3 non-whitespace characters
 * - Recipient ID must be non-empty (after trimming)
 * - Signing key must be non-empty and at least 3 non-whitespace characters
 *
 * Security: All inputs are trimmed and validated to prevent whitespace-only attacks
 */
export function validateSessionParams(params: SessionParams): SessionValidation {
  const { sessionId, recipientId, signingKey } = params;

  // Check session ID (trimmed for security)
  if (sessionId === null || sessionId === undefined || sessionId === '') {
    return { valid: false, error: 'Missing required parameter: session' };
  }
  const trimmedSessionId = sessionId.trim();
  if (trimmedSessionId.length < 3) {
    return { valid: false, error: 'Invalid session ID format' };
  }

  // Check recipient ID (trimmed for security)
  if (recipientId === null || recipientId === undefined || recipientId === '') {
    return { valid: false, error: 'Missing required parameter: recipient' };
  }
  const trimmedRecipientId = recipientId.trim();
  if (trimmedRecipientId.length === 0) {
    return { valid: false, error: 'Missing required parameter: recipient' };
  }

  // Check signing key (trimmed for security)
  if (signingKey === null || signingKey === undefined || signingKey === '') {
    return { valid: false, error: 'Missing required parameter: key' };
  }
  const trimmedSigningKey = signingKey.trim();
  if (trimmedSigningKey.length < 3) {
    return { valid: false, error: 'Invalid signing key format' };
  }

  return { valid: true };
}

// ============================================================
// Session Expiry Detection
// ============================================================

/**
 * Check if a timestamp is expired based on a TTL in milliseconds
 * @param createdAt - Creation timestamp (Unix ms or ISO string)
 * @param ttlMs - Time to live in milliseconds (default: 7 days)
 * @returns true if expired
 */
export function isSessionExpired(createdAt: number | string, ttlMs: number = 7 * 24 * 60 * 60 * 1000): boolean {
  const createdTime = typeof createdAt === 'string' ? new Date(createdAt).getTime() : createdAt;

  if (isNaN(createdTime)) {
    return true; // Invalid timestamp = expired
  }

  const now = Date.now();
  return now - createdTime > ttlMs;
}

// ============================================================
// Field Filtering
// ============================================================

/**
 * Filter fields by recipient ID
 * Returns only fields assigned to the specified recipient
 */
export function filterFieldsByRecipient(fields: SigningField[], recipientId: string): SigningField[] {
  return fields.filter((field) => field.recipientId === recipientId);
}

/**
 * Check if all required fields are completed
 */
export function areAllRequiredFieldsComplete(fields: SigningField[], completedFieldIds: Set<string>): boolean {
  return fields.filter((f) => f.required).every((f) => completedFieldIds.has(f.id));
}

// ============================================================
// Offline Queue Management
// ============================================================

const OFFLINE_QUEUE_KEY = 'docsign_offline_queue';

/**
 * Serialize a queued submission for storage
 */
export function serializeQueuedSubmission(submission: QueuedSubmission): string {
  return JSON.stringify(submission);
}

/**
 * Deserialize a queued submission from storage
 */
export function deserializeQueuedSubmission(json: string): QueuedSubmission {
  const parsed = JSON.parse(json);

  // Validate required fields
  if (typeof parsed.sessionId !== 'string') {
    throw new Error('Invalid sessionId in queued submission');
  }
  if (typeof parsed.recipientId !== 'string') {
    throw new Error('Invalid recipientId in queued submission');
  }
  if (typeof parsed.signingKey !== 'string') {
    throw new Error('Invalid signingKey in queued submission');
  }
  if (typeof parsed.signatures !== 'object' || parsed.signatures === null) {
    throw new Error('Invalid signatures in queued submission');
  }
  if (typeof parsed.completedAt !== 'string') {
    throw new Error('Invalid completedAt in queued submission');
  }
  if (typeof parsed.timestamp !== 'number') {
    throw new Error('Invalid timestamp in queued submission');
  }

  return parsed as QueuedSubmission;
}

/**
 * Get the offline queue from localStorage
 */
export function getOfflineQueue(): QueuedSubmission[] {
  if (typeof localStorage === 'undefined') {
    return [];
  }

  const json = localStorage.getItem(OFFLINE_QUEUE_KEY);
  if (!json) {
    return [];
  }

  try {
    const parsed = JSON.parse(json);
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed.map(deserializeQueuedSubmission);
  } catch {
    return [];
  }
}

/**
 * Add a submission to the offline queue
 */
export function addToOfflineQueue(submission: QueuedSubmission): void {
  if (typeof localStorage === 'undefined') {
    return;
  }

  const queue = getOfflineQueue();
  queue.push(submission);
  localStorage.setItem(OFFLINE_QUEUE_KEY, JSON.stringify(queue));
}

/**
 * Clear the offline queue
 */
export function clearOfflineQueue(): void {
  if (typeof localStorage === 'undefined') {
    return;
  }
  localStorage.removeItem(OFFLINE_QUEUE_KEY);
}

/**
 * Remove a specific submission from the queue
 */
export function removeFromOfflineQueue(sessionId: string, recipientId: string): void {
  if (typeof localStorage === 'undefined') {
    return;
  }

  const queue = getOfflineQueue();
  const filtered = queue.filter((s) => !(s.sessionId === sessionId && s.recipientId === recipientId));
  localStorage.setItem(OFFLINE_QUEUE_KEY, JSON.stringify(filtered));
}
