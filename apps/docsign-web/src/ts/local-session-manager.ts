/**
 * Local Session Manager - IndexedDB-based session storage
 *
 * DOCSIGN_PLAN Phase 2: Local-first session management
 *
 * This module provides local-first session storage using IndexedDB:
 * - Store and retrieve signing sessions locally
 * - Cache server responses for offline use
 * - Store PDF documents locally with optional encryption
 * - Queue signatures for later sync
 * - **NEW: Encryption at rest using Web Crypto API (AES-GCM)**
 *
 * Key design principles:
 * - Always save locally first
 * - Fall back to server only if local not found AND online
 * - Cache server responses for offline access
 * - Never lose user data due to network issues
 * - Encrypt sensitive data at rest (signatures, PDF data)
 */

import {
  encryptString,
  decryptString,
  encryptBytes,
  decryptBytes,
  isEncrypted,
  isEncryptionAvailable,
  type EncryptedData,
} from './crypto-utils';
import { createLogger } from './logger';

const log = createLogger('LocalSessionManager');

// ============================================================
// Types
// ============================================================

/**
 * Session status - matches the status in session.ts
 */
export type SessionStatus = 'pending' | 'in_progress' | 'completed' | 'declined' | 'expired';

/**
 * Recipient status in a signing session
 */
export type RecipientStatus = 'pending' | 'sent' | 'viewed' | 'signed' | 'declined';

/**
 * Signing field definition
 */
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
  isOwn?: boolean;
  ownerName?: string;
}

/**
 * Recipient in a signing session
 */
export interface Recipient {
  id: string | number;
  firstName: string;
  lastName: string;
  email?: string;
  name?: string;
  order?: number;
  status?: RecipientStatus;
}

/**
 * Signature data captured for a field
 */
export interface SignatureData {
  fieldId: string;
  type: 'draw' | 'type' | 'upload';
  data: string; // Base64 encoded signature image or typed text
  timestamp: string;
  recipientId: string;
}

/**
 * Full session data - the main Session interface
 */
export interface Session {
  id: string;
  documentHash?: string;
  documentData?: Uint8Array;
  recipients: Recipient[];
  fields: SigningField[];
  completedFields?: Map<string, SignatureData>;
  status: SessionStatus;
  createdAt: string;
  updatedAt?: string;
  expiresAt: string | null;
}

/**
 * Summary of a session for listing (without full document data)
 */
export interface SessionSummary {
  id: string;
  documentHash?: string;
  recipientCount: number;
  fieldCount: number;
  completedFieldCount: number;
  status: SessionStatus;
  createdAt: string;
  updatedAt?: string;
  expiresAt: string | null;
}

export interface LocalSession {
  /** Unique session identifier */
  sessionId: string;
  /** Recipient ID */
  recipientId: string;
  /** Document name for display */
  documentName: string;
  /** Session metadata (sender info, dates, etc.) */
  metadata?: SessionMetadata;
  /** Fields for this recipient to sign */
  fields: SigningField[];
  /** All recipients in the session */
  recipients?: Recipient[];
  /** PDF document data (base64 or Uint8Array serialized) */
  pdfData?: string;
  /** Session status */
  status: SessionStatus;
  /** When session was created */
  createdAt: string;
  /** When session expires (null = never) */
  expiresAt?: string | null;
  /** Completed signatures */
  signatures?: Record<string, unknown>;
  /** Whether this was cached from server */
  isServerCached?: boolean;
  /** When this was last synced from server */
  lastSyncedAt?: string;
}

export interface SessionMetadata {
  filename?: string;
  created_by?: string;
  sender_email?: string;
  created_at?: string;
}

export interface QueuedSignature {
  sessionId: string;
  recipientId: string;
  signingKey: string;
  signatures: Record<string, unknown>;
  completedAt: string;
  timestamp: number;
  retryCount?: number;
}

// ============================================================
// IndexedDB Configuration
// ============================================================

const DB_NAME = 'docsign_local';
const DB_VERSION = 1;

const STORES = {
  SESSIONS: 'sessions',
  PDF_CACHE: 'pdf_cache',
  SIGNATURE_QUEUE: 'signature_queue',
} as const;

// ============================================================
// IndexedDB Helpers
// ============================================================

/**
 * Open the IndexedDB database
 */
function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.onerror = () => {
      log.error('Failed to open database:', request.error);
      reject(request.error);
    };

    request.onsuccess = () => {
      resolve(request.result);
    };

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result;

      // Sessions store - indexed by sessionId
      if (!db.objectStoreNames.contains(STORES.SESSIONS)) {
        const sessionsStore = db.createObjectStore(STORES.SESSIONS, { keyPath: 'sessionId' });
        sessionsStore.createIndex('recipientId', 'recipientId', { unique: false });
        sessionsStore.createIndex('status', 'status', { unique: false });
        sessionsStore.createIndex('createdAt', 'createdAt', { unique: false });
      }

      // PDF cache store - indexed by sessionId
      if (!db.objectStoreNames.contains(STORES.PDF_CACHE)) {
        db.createObjectStore(STORES.PDF_CACHE, { keyPath: 'sessionId' });
      }

      // Signature queue store - for offline submission
      if (!db.objectStoreNames.contains(STORES.SIGNATURE_QUEUE)) {
        const queueStore = db.createObjectStore(STORES.SIGNATURE_QUEUE, { keyPath: ['sessionId', 'recipientId'] });
        queueStore.createIndex('timestamp', 'timestamp', { unique: false });
      }

      log.info('Database schema created/upgraded');
    };
  });
}

/**
 * Generic get from store
 */
async function getFromStore<T>(storeName: string, key: IDBValidKey): Promise<T | undefined> {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(storeName, 'readonly');
    const store = transaction.objectStore(storeName);
    const request = store.get(key);

    request.onsuccess = () => {
      db.close();
      resolve(request.result as T | undefined);
    };

    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}

/**
 * Generic put to store
 */
async function putToStore<T>(storeName: string, value: T): Promise<void> {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(storeName, 'readwrite');
    const store = transaction.objectStore(storeName);
    const request = store.put(value);

    request.onsuccess = () => {
      db.close();
      resolve();
    };

    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}

/**
 * Generic delete from store
 */
async function deleteFromStore(storeName: string, key: IDBValidKey): Promise<void> {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(storeName, 'readwrite');
    const store = transaction.objectStore(storeName);
    const request = store.delete(key);

    request.onsuccess = () => {
      db.close();
      resolve();
    };

    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}

/**
 * Get all items from store
 */
async function getAllFromStore<T>(storeName: string): Promise<T[]> {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(storeName, 'readonly');
    const store = transaction.objectStore(storeName);
    const request = store.getAll();

    request.onsuccess = () => {
      db.close();
      resolve(request.result as T[]);
    };

    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}

// ============================================================
// Local Session Manager Class
// ============================================================

/**
 * LocalSessionManager - IndexedDB-based local-first session storage
 *
 * Usage:
 * ```typescript
 * // Get session (tries local first, then server)
 * const session = await LocalSessionManager.getSession(sessionId, recipientId);
 *
 * // Save session locally
 * await LocalSessionManager.saveSession(session);
 *
 * // Cache server session
 * await LocalSessionManager.cacheSession(serverResponse);
 *
 * // Save signatures locally
 * await LocalSessionManager.saveSignatures(sessionId, signatures);
 *
 * // Queue for sync
 * await LocalSessionManager.queueForSync(submission);
 * ```
 */
export class LocalSessionManager {
  /**
   * Get a session from local storage
   * @param sessionId Session ID to retrieve
   * @param _recipientId Optional recipient ID to filter fields (reserved for future use)
   * @returns The session if found, undefined otherwise
   */
  static async getSession(sessionId: string, _recipientId?: string): Promise<LocalSession | undefined> {
    try {
      const session = await getFromStore<LocalSession>(STORES.SESSIONS, sessionId);

      if (!session) {
        log.debug('Session not found locally:', sessionId);
        return undefined;
      }

      // Check if expired
      if (session.expiresAt) {
        const expiresAt = new Date(session.expiresAt).getTime();
        if (Date.now() > expiresAt) {
          log.debug('Session expired:', sessionId);
          session.status = 'expired';
          return session;
        }
      }

      log.debug('Session found locally:', sessionId);
      return session;
    } catch (err) {
      log.error('Error getting session:', err);
      return undefined;
    }
  }

  /**
   * Save a session to local storage
   * @param session The session to save
   */
  static async saveSession(session: LocalSession): Promise<void> {
    try {
      await putToStore(STORES.SESSIONS, session);
      log.debug('Session saved:', session.sessionId);
    } catch (err) {
      log.error('Error saving session:', err);
      throw err;
    }
  }

  /**
   * Cache a server response for offline use
   * @param serverResponse The server response to cache
   */
  static async cacheSession(serverResponse: Record<string, unknown>): Promise<void> {
    try {
      // Convert server response to LocalSession format
      const session: LocalSession = {
        sessionId: String(serverResponse.sessionId || serverResponse.session_id || ''),
        recipientId: String(serverResponse.recipientId || serverResponse.recipient_id || ''),
        documentName: String(serverResponse.documentName || serverResponse.document_name || 'Document'),
        metadata: serverResponse.metadata as SessionMetadata,
        fields: (serverResponse.fields || []) as SigningField[],
        recipients: serverResponse.recipients as Recipient[],
        status: (serverResponse.status as SessionStatus) || 'pending',
        createdAt: String(serverResponse.createdAt || serverResponse.created_at || new Date().toISOString()),
        expiresAt: (serverResponse.expiresAt as string | null) || (serverResponse.expires_at as string | null),
        isServerCached: true,
        lastSyncedAt: new Date().toISOString(),
      };

      await this.saveSession(session);
      log.debug('Server session cached:', session.sessionId);
    } catch (err) {
      log.error('Error caching server session:', err);
      // Don't throw - caching failure shouldn't break the flow
    }
  }

  /**
   * Cache PDF data for a session (with encryption at rest)
   * @param sessionId Session ID
   * @param pdfData PDF data as base64 string or Uint8Array
   */
  static async cachePdfData(sessionId: string, pdfData: string | Uint8Array): Promise<void> {
    try {
      // Convert to Uint8Array if base64 string
      let bytes: Uint8Array;
      if (typeof pdfData === 'string') {
        const binary = atob(pdfData);
        bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
          bytes[i] = binary.charCodeAt(i);
        }
      } else {
        bytes = pdfData;
      }

      // Encrypt if available
      let storedData: string | EncryptedData;
      let isEncryptedFlag = false;

      if (isEncryptionAvailable()) {
        try {
          storedData = await encryptBytes(bytes);
          isEncryptedFlag = true;
          log.debug('PDF data encrypted');
        } catch (encryptErr) {
          log.warn('Encryption failed, storing unencrypted:', encryptErr);
          storedData = btoa(String.fromCharCode(...bytes));
        }
      } else {
        storedData = btoa(String.fromCharCode(...bytes));
      }

      await putToStore(STORES.PDF_CACHE, {
        sessionId,
        pdfData: storedData,
        isEncrypted: isEncryptedFlag,
        cachedAt: new Date().toISOString(),
      });

      log.debug('PDF data cached for session:', sessionId);
    } catch (err) {
      log.error('Error caching PDF data:', err);
      // Don't throw - caching failure shouldn't break the flow
    }
  }

  /**
   * Get cached PDF data for a session (handles decryption)
   * @param sessionId Session ID
   * @returns PDF data as base64 string, or undefined if not cached
   */
  static async getCachedPdfData(sessionId: string): Promise<string | undefined> {
    try {
      const cached = await getFromStore<{
        sessionId: string;
        pdfData: string | EncryptedData;
        isEncrypted?: boolean;
      }>(STORES.PDF_CACHE, sessionId);

      if (!cached) return undefined;

      // Handle encrypted data
      if (cached.isEncrypted && isEncrypted(cached.pdfData)) {
        try {
          const decrypted = await decryptBytes(cached.pdfData);
          // Convert to base64
          return btoa(String.fromCharCode(...decrypted));
        } catch (decryptErr) {
          log.error('Failed to decrypt PDF data:', decryptErr);
          return undefined;
        }
      }

      // Legacy unencrypted data
      return cached.pdfData as string;
    } catch (err) {
      log.error('Error getting cached PDF:', err);
      return undefined;
    }
  }

  /**
   * Save signatures to the session (with encryption at rest)
   * @param sessionId Session ID
   * @param signatures Signatures record
   */
  static async saveSignatures(sessionId: string, signatures: Record<string, unknown>): Promise<void> {
    try {
      const session = await this.getSession(sessionId);
      if (session) {
        // Encrypt signature data if available
        let storedSignatures: Record<string, unknown> | EncryptedData;
        let isSignaturesEncrypted = false;

        const mergedSignatures = { ...session.signatures, ...signatures };

        if (isEncryptionAvailable()) {
          try {
            const sigJson = JSON.stringify(mergedSignatures);
            storedSignatures = await encryptString(sigJson);
            isSignaturesEncrypted = true;
            log.debug('Signatures encrypted');
          } catch (encryptErr) {
            log.warn('Signature encryption failed:', encryptErr);
            storedSignatures = mergedSignatures;
          }
        } else {
          storedSignatures = mergedSignatures;
        }

        session.signatures = storedSignatures as Record<string, unknown>;
        session.status = 'in_progress';
        // Store encryption flag in metadata
        if (!session.metadata) {
          session.metadata = {};
        }
        (session.metadata as Record<string, unknown>).signaturesEncrypted = isSignaturesEncrypted;
        await this.saveSession(session);
        log.debug('Signatures saved for session:', sessionId);
      } else {
        log.warn('Cannot save signatures - session not found:', sessionId);
      }
    } catch (err) {
      log.error('Error saving signatures:', err);
      throw err;
    }
  }

  /**
   * Get decrypted signatures from a session
   * @param sessionId Session ID
   * @returns Decrypted signatures record
   */
  static async getDecryptedSignatures(sessionId: string): Promise<Record<string, unknown> | undefined> {
    try {
      const session = await this.getSession(sessionId);
      if (!session?.signatures) return undefined;

      // Check if signatures are encrypted
      const isSignaturesEncrypted = (session.metadata as Record<string, unknown> | undefined)?.signaturesEncrypted;

      if (isSignaturesEncrypted && isEncrypted(session.signatures)) {
        try {
          const decrypted = await decryptString(session.signatures as EncryptedData);
          return JSON.parse(decrypted);
        } catch (decryptErr) {
          log.error('Failed to decrypt signatures:', decryptErr);
          return undefined;
        }
      }

      // Legacy unencrypted data
      return session.signatures as Record<string, unknown>;
    } catch (err) {
      log.error('Error getting signatures:', err);
      return undefined;
    }
  }

  /**
   * Mark session as completed
   * @param sessionId Session ID
   */
  static async completeSession(sessionId: string): Promise<void> {
    try {
      const session = await this.getSession(sessionId);
      if (session) {
        session.status = 'completed';
        await this.saveSession(session);
        log.debug('Session completed:', sessionId);
      }
    } catch (err) {
      log.error('Error completing session:', err);
      throw err;
    }
  }

  /**
   * Queue a signature submission for later sync
   * @param submission The queued signature submission
   */
  static async queueForSync(submission: QueuedSignature): Promise<void> {
    try {
      await putToStore(STORES.SIGNATURE_QUEUE, submission);
      log.debug('Submission queued for sync:', submission.sessionId);
    } catch (err) {
      log.error('Error queueing submission:', err);
      throw err;
    }
  }

  /**
   * Get all queued submissions
   * @returns Array of queued submissions
   */
  static async getQueuedSubmissions(): Promise<QueuedSignature[]> {
    try {
      return await getAllFromStore<QueuedSignature>(STORES.SIGNATURE_QUEUE);
    } catch (err) {
      log.error('Error getting queued submissions:', err);
      return [];
    }
  }

  /**
   * Remove a submission from the queue
   * @param sessionId Session ID
   * @param recipientId Recipient ID
   */
  static async removeFromQueue(sessionId: string, recipientId: string): Promise<void> {
    try {
      await deleteFromStore(STORES.SIGNATURE_QUEUE, [sessionId, recipientId]);
      log.debug('Removed from queue:', sessionId, recipientId);
    } catch (err) {
      log.error('Error removing from queue:', err);
    }
  }

  /**
   * Delete a session from local storage
   * @param sessionId Session ID to delete
   */
  static async deleteSession(sessionId: string): Promise<void> {
    try {
      await deleteFromStore(STORES.SESSIONS, sessionId);
      await deleteFromStore(STORES.PDF_CACHE, sessionId);
      log.debug('Session deleted:', sessionId);
    } catch (err) {
      log.error('Error deleting session:', err);
    }
  }

  /**
   * Get all sessions for a recipient
   * @param recipientId Recipient ID
   * @returns Array of sessions
   */
  static async getSessionsForRecipient(recipientId: string): Promise<LocalSession[]> {
    try {
      const allSessions = await getAllFromStore<LocalSession>(STORES.SESSIONS);
      return allSessions.filter(s => s.recipientId === recipientId);
    } catch (err) {
      log.error('Error getting sessions for recipient:', err);
      return [];
    }
  }

  /**
   * Clear all local data (for testing/debugging)
   */
  static async clearAll(): Promise<void> {
    try {
      const db = await openDatabase();

      await new Promise<void>((resolve, reject) => {
        const transaction = db.transaction(
          [STORES.SESSIONS, STORES.PDF_CACHE, STORES.SIGNATURE_QUEUE],
          'readwrite'
        );

        transaction.objectStore(STORES.SESSIONS).clear();
        transaction.objectStore(STORES.PDF_CACHE).clear();
        transaction.objectStore(STORES.SIGNATURE_QUEUE).clear();

        transaction.oncomplete = () => {
          db.close();
          resolve();
        };

        transaction.onerror = () => {
          db.close();
          reject(transaction.error);
        };
      });

      log.info('All local data cleared');
    } catch (err) {
      log.error('Error clearing all data:', err);
    }
  }

  /**
   * Check if IndexedDB is available
   * @returns true if IndexedDB is supported
   */
  static isAvailable(): boolean {
    return typeof indexedDB !== 'undefined';
  }
}

// ============================================================
// Sync Manager - Background synchronization
// ============================================================

/**
 * Attempt to sync queued submissions to server
 * @param apiBase Base URL for API
 * @returns Number of successfully synced submissions
 */
export async function syncQueuedSubmissions(apiBase: string): Promise<number> {
  if (!navigator.onLine) {
    log.debug('Offline - skipping sync');
    return 0;
  }

  const queue = await LocalSessionManager.getQueuedSubmissions();
  let syncedCount = 0;

  for (const submission of queue) {
    try {
      const response = await fetch(`${apiBase}/session/${submission.sessionId}/signed`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Recipient-Id': submission.recipientId,
          'X-Signing-Key': submission.signingKey,
        },
        body: JSON.stringify({
          recipient_id: submission.recipientId,
          signatures: submission.signatures,
          completed_at: submission.completedAt,
        }),
      });

      if (response.ok) {
        await LocalSessionManager.removeFromQueue(submission.sessionId, submission.recipientId);
        await LocalSessionManager.completeSession(submission.sessionId);
        syncedCount++;
        log.debug('Synced submission:', submission.sessionId);
      } else {
        log.warn('Failed to sync submission:', response.status);
        // Update retry count
        submission.retryCount = (submission.retryCount || 0) + 1;
        await LocalSessionManager.queueForSync(submission);
      }
    } catch (err) {
      log.error('Error syncing submission:', err);
    }
  }

  return syncedCount;
}

/**
 * Setup automatic sync when coming online
 * @param apiBase Base URL for API
 */
export function setupAutoSync(apiBase: string): void {
  window.addEventListener('online', async () => {
    log.info('Online - attempting sync');
    const synced = await syncQueuedSubmissions(apiBase);
    if (synced > 0) {
      log.info(`Synced ${synced} submissions`);
    }
  });
}

// ============================================================
// Singleton Instance
// ============================================================

/**
 * Singleton instance of LocalSessionManager for convenience
 * This wraps the static class methods in an instance for easier use
 */
class LocalSessionManagerInstance {
  async createSession(
    document: Uint8Array,
    recipients: Recipient[],
    fields: SigningField[] = []
  ): Promise<Session> {
    const sessionId = crypto.randomUUID();
    const now = new Date().toISOString();

    const session: LocalSession = {
      sessionId,
      recipientId: recipients[0]?.id?.toString() || '',
      documentName: 'Untitled Document',
      fields,
      recipients,
      status: 'pending',
      createdAt: now,
      expiresAt: null,
    };

    // Store PDF data
    if (document.length > 0) {
      await LocalSessionManager.cachePdfData(sessionId, document);
    }

    await LocalSessionManager.saveSession(session);

    return {
      id: sessionId,
      recipients,
      fields,
      status: 'pending',
      createdAt: now,
      expiresAt: null,
    };
  }

  async getSession(sessionId: string): Promise<Session | null> {
    const localSession = await LocalSessionManager.getSession(sessionId);
    if (!localSession) return null;

    return {
      id: localSession.sessionId,
      recipients: localSession.recipients || [],
      fields: localSession.fields,
      status: localSession.status,
      createdAt: localSession.createdAt,
      expiresAt: localSession.expiresAt || null,
    };
  }

  async updateSessionStatus(sessionId: string, status: SessionStatus): Promise<void> {
    const session = await LocalSessionManager.getSession(sessionId);
    if (session) {
      session.status = status;
      await LocalSessionManager.saveSession(session);
    }
  }

  async recordSignature(
    sessionId: string,
    fieldId: string,
    signatureData: string,
    type: SignatureData['type'] = 'draw',
    recipientId: string = ''
  ): Promise<void> {
    const session = await LocalSessionManager.getSession(sessionId);
    if (session) {
      const signatures = session.signatures || {};
      signatures[fieldId] = {
        fieldId,
        type,
        data: signatureData,
        timestamp: new Date().toISOString(),
        recipientId,
      };
      await LocalSessionManager.saveSignatures(sessionId, signatures);
    }
  }

  async getSignedDocument(sessionId: string): Promise<Uint8Array | null> {
    const pdfData = await LocalSessionManager.getCachedPdfData(sessionId);
    if (!pdfData) return null;

    // Convert base64 to Uint8Array
    const binary = atob(pdfData);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes;
  }

  async deleteSession(sessionId: string): Promise<void> {
    await LocalSessionManager.deleteSession(sessionId);
  }

  async listSessions(): Promise<SessionSummary[]> {
    const sessions = await getAllFromStore<LocalSession>(STORES.SESSIONS);
    return sessions.map((s) => ({
      id: s.sessionId,
      recipientCount: s.recipients?.length || 0,
      fieldCount: s.fields.length,
      completedFieldCount: Object.keys(s.signatures || {}).length,
      status: s.status,
      createdAt: s.createdAt,
      expiresAt: s.expiresAt || null,
    }));
  }
}

/**
 * Singleton instance export
 */
export const localSessionManager = new LocalSessionManagerInstance();

// ============================================================
// Window Namespace Extension
// ============================================================

/**
 * Initialize LocalSessionManager on window.DocSign namespace
 */
export function initLocalSessionNamespace(): void {
  // Ensure DocSign namespace exists
  if (!window.DocSign) {
    (window as { DocSign?: Record<string, unknown> }).DocSign = {};
  }

  const docSign = window.DocSign as Record<string, unknown>;

  // Add LocalSessionManager class
  docSign.LocalSessionManager = LocalSessionManager;

  // Add singleton instance
  docSign.localSessionManager = localSessionManager;

  // Add convenience functions bound to the singleton
  docSign.createSession = localSessionManager.createSession.bind(localSessionManager);
  docSign.getSession = localSessionManager.getSession.bind(localSessionManager);
  docSign.updateSessionStatus = localSessionManager.updateSessionStatus.bind(localSessionManager);
  docSign.recordSignature = localSessionManager.recordSignature.bind(localSessionManager);
  docSign.getSignedDocument = localSessionManager.getSignedDocument.bind(localSessionManager);
  docSign.deleteSession = localSessionManager.deleteSession.bind(localSessionManager);
  docSign.listSessions = localSessionManager.listSessions.bind(localSessionManager);

  log.info('Session management initialized on window.DocSign');
}
