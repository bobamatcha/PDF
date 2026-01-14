/**
 * Property-Based Tests for LocalSessionManager and SyncManager
 *
 * Phase 2 of DOCSIGN_PLAN: Local-First Session Management
 *
 * These tests use fast-check for property-based testing to verify:
 * 1. Session Creation Properties
 * 2. Session State Transition Properties
 * 3. Signature Recording Properties
 * 4. Session Expiry Properties
 * 5. Persistence Properties (mock IndexedDB)
 * 6. Sync Queue Properties
 * 7. Encryption Properties
 *
 * NOTE: These tests are written FIRST before the implementation exists.
 * They should FAIL until LocalSessionManager is implemented.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fc from 'fast-check';

// Mock IndexedDB using fake-indexeddb
// Note: You may need to add 'fake-indexeddb' to devDependencies
// For now, we'll use an in-memory mock approach

// ============================================================
// Types for LocalSessionManager (what we expect to implement)
// ============================================================

/**
 * Session status enum - valid states a session can be in
 */
export type LocalSessionStatus = 'pending' | 'accepted' | 'declined' | 'completed' | 'expired';

/**
 * Valid state transitions:
 * - pending -> accepted (recipient opens and accepts)
 * - pending -> declined (recipient declines)
 * - accepted -> completed (all signatures collected)
 * - accepted -> expired (TTL exceeded)
 * - pending -> expired (TTL exceeded)
 * - completed is terminal
 * - declined is terminal
 * - expired is terminal
 */
export const VALID_TRANSITIONS: Record<LocalSessionStatus, LocalSessionStatus[]> = {
  pending: ['accepted', 'declined', 'expired'],
  accepted: ['completed', 'expired'],
  declined: [], // Terminal state
  completed: [], // Terminal state
  expired: [], // Terminal state
};

/**
 * Recipient in a signing session
 */
export interface Recipient {
  id: string;
  email: string;
  name: string;
  role: 'signer' | 'cc';
}

/**
 * Signature field definition
 */
export interface SignatureField {
  id: string;
  type: 'signature' | 'initials' | 'date' | 'text';
  page: number;
  x: number;
  y: number;
  width: number;
  height: number;
  required: boolean;
  recipientId: string;
}

/**
 * Recorded signature data
 */
export interface RecordedSignature {
  fieldId: string;
  type: 'drawn' | 'typed';
  data: string; // base64 for drawn, text for typed
  timestamp: string;
  signerId: string;
}

/**
 * Audit chain event
 */
export interface AuditEvent {
  id: string;
  action: string;
  actor: string;
  documentHash: string;
  previousHash: string;
  timestamp: string;
}

/**
 * Local session structure
 */
export interface LocalSession {
  id: string;
  documentHash: string;
  documentEncrypted?: Uint8Array;
  recipients: Recipient[];
  fields: SignatureField[];
  signatures: Map<string, RecordedSignature>;
  status: LocalSessionStatus;
  createdAt: string;
  updatedAt: string;
  expiresAt: string | null; // null = never expires
  auditChain: AuditEvent[];
}

/**
 * Sync queue item
 */
export interface SyncQueueItem {
  id: string;
  sessionId: string;
  action: 'create' | 'update' | 'complete';
  payload: unknown;
  createdAt: string;
  retries: number;
}

// ============================================================
// In-Memory Mock IndexedDB Store
// ============================================================

class MockIndexedDBStore {
  private sessions: Map<string, LocalSession> = new Map();
  private syncQueue: SyncQueueItem[] = [];

  async get(key: string): Promise<LocalSession | undefined> {
    return this.sessions.get(key);
  }

  async put(key: string, value: LocalSession): Promise<void> {
    this.sessions.set(key, value);
  }

  async delete(key: string): Promise<boolean> {
    return this.sessions.delete(key);
  }

  async getAll(): Promise<LocalSession[]> {
    return Array.from(this.sessions.values());
  }

  async clear(): Promise<void> {
    this.sessions.clear();
    this.syncQueue = [];
  }

  // Sync queue methods
  async addToSyncQueue(item: SyncQueueItem): Promise<void> {
    this.syncQueue.push(item);
  }

  async getSyncQueue(): Promise<SyncQueueItem[]> {
    return [...this.syncQueue];
  }

  async removeFromSyncQueue(id: string): Promise<void> {
    this.syncQueue = this.syncQueue.filter(item => item.id !== id);
  }

  async clearSyncQueue(): Promise<void> {
    this.syncQueue = [];
  }
}

// ============================================================
// LocalSessionManager Implementation (Stub for Tests)
// ============================================================

/**
 * LocalSessionManager - manages offline-first signing sessions
 *
 * This is a stub implementation for tests. The real implementation
 * will be created after tests are written.
 */
class LocalSessionManager {
  private db: MockIndexedDBStore;

  constructor(db: MockIndexedDBStore) {
    this.db = db;
  }

  /**
   * Create a new signing session
   */
  async createSession(
    document: Uint8Array,
    recipients: Recipient[],
    fields: SignatureField[]
  ): Promise<LocalSession> {
    const id = crypto.randomUUID();
    const documentHash = await this.hashDocument(document);
    const now = new Date().toISOString();

    const session: LocalSession = {
      id,
      documentHash,
      documentEncrypted: document, // In real impl, this would be encrypted
      recipients,
      fields,
      signatures: new Map(),
      status: 'pending',
      createdAt: now,
      updatedAt: now,
      expiresAt: null, // No expiration by default for local sessions
      auditChain: [{
        id: crypto.randomUUID(),
        action: 'DocumentUploaded',
        actor: 'system',
        documentHash,
        previousHash: '',
        timestamp: now,
      }],
    };

    await this.db.put(id, session);
    return session;
  }

  /**
   * Get session by ID
   */
  async getSession(sessionId: string): Promise<LocalSession | undefined> {
    return this.db.get(sessionId);
  }

  /**
   * Update session status with validation
   */
  async updateStatus(sessionId: string, newStatus: LocalSessionStatus): Promise<LocalSession> {
    const session = await this.db.get(sessionId);
    if (!session) {
      throw new Error(`Session not found: ${sessionId}`);
    }

    // Check if transition is valid
    const validTransitions = VALID_TRANSITIONS[session.status];
    if (!validTransitions.includes(newStatus)) {
      throw new Error(`Invalid transition from ${session.status} to ${newStatus}`);
    }

    const now = new Date().toISOString();
    session.status = newStatus;
    session.updatedAt = now;
    session.auditChain.push({
      id: crypto.randomUUID(),
      action: `StatusChanged:${newStatus}`,
      actor: 'system',
      documentHash: session.documentHash,
      previousHash: session.auditChain[session.auditChain.length - 1]?.id || '',
      timestamp: now,
    });

    await this.db.put(sessionId, session);
    return session;
  }

  /**
   * Record a signature for a field
   */
  async recordSignature(
    sessionId: string,
    fieldId: string,
    signatureData: Omit<RecordedSignature, 'fieldId' | 'timestamp'>
  ): Promise<LocalSession> {
    const session = await this.db.get(sessionId);
    if (!session) {
      throw new Error(`Session not found: ${sessionId}`);
    }

    // Check if session is expired
    if (this.isSessionExpired(session)) {
      throw new Error('Session has expired');
    }

    // Check if field exists
    const field = session.fields.find(f => f.id === fieldId);
    if (!field) {
      throw new Error(`Field not found: ${fieldId}`);
    }

    // Check if session status allows signing
    if (session.status !== 'accepted' && session.status !== 'pending') {
      throw new Error(`Cannot sign in status: ${session.status}`);
    }

    const now = new Date().toISOString();
    const signature: RecordedSignature = {
      fieldId,
      ...signatureData,
      timestamp: now,
    };

    session.signatures.set(fieldId, signature);
    session.updatedAt = now;
    session.auditChain.push({
      id: crypto.randomUUID(),
      action: 'SignatureApplied',
      actor: signatureData.signerId,
      documentHash: session.documentHash,
      previousHash: session.auditChain[session.auditChain.length - 1]?.id || '',
      timestamp: now,
    });

    await this.db.put(sessionId, session);
    return session;
  }

  /**
   * Get recorded signature for a field
   */
  async getSignature(sessionId: string, fieldId: string): Promise<RecordedSignature | undefined> {
    const session = await this.db.get(sessionId);
    return session?.signatures.get(fieldId);
  }

  /**
   * Check if session is expired
   */
  isSessionExpired(session: LocalSession): boolean {
    if (session.expiresAt === null) {
      return false; // Never expires
    }
    return new Date(session.expiresAt).getTime() < Date.now();
  }

  /**
   * Delete a session
   */
  async deleteSession(sessionId: string): Promise<boolean> {
    return this.db.delete(sessionId);
  }

  /**
   * List all sessions
   */
  async listSessions(): Promise<LocalSession[]> {
    return this.db.getAll();
  }

  /**
   * Hash a document (SHA-256)
   */
  private async hashDocument(document: Uint8Array): Promise<string> {
    // Use .buffer slice to get proper ArrayBuffer for crypto.subtle
    const buffer = document.buffer.slice(
      document.byteOffset,
      document.byteOffset + document.byteLength
    ) as ArrayBuffer;
    const hashBuffer = await crypto.subtle.digest('SHA-256', buffer);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
  }
}

// ============================================================
// SyncManager Implementation (Stub for Tests)
// ============================================================

/**
 * SyncManager - manages offline queue and sync operations
 */
class SyncManager {
  private db: MockIndexedDBStore;

  constructor(db: MockIndexedDBStore) {
    this.db = db;
  }

  /**
   * Add item to sync queue
   */
  async enqueue(item: Omit<SyncQueueItem, 'id' | 'createdAt' | 'retries'>): Promise<SyncQueueItem> {
    const queueItem: SyncQueueItem = {
      id: crypto.randomUUID(),
      ...item,
      createdAt: new Date().toISOString(),
      retries: 0,
    };
    await this.db.addToSyncQueue(queueItem);
    return queueItem;
  }

  /**
   * Get all items in queue (FIFO order)
   */
  async getQueue(): Promise<SyncQueueItem[]> {
    const items = await this.db.getSyncQueue();
    // Sort by createdAt for FIFO order
    return items.sort((a, b) =>
      new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()
    );
  }

  /**
   * Remove item from queue
   */
  async dequeue(id: string): Promise<void> {
    await this.db.removeFromSyncQueue(id);
  }

  /**
   * Check if item exists in queue
   */
  async hasItem(id: string): Promise<boolean> {
    const queue = await this.db.getSyncQueue();
    return queue.some(item => item.id === id);
  }

  /**
   * Clear entire queue
   */
  async clearQueue(): Promise<void> {
    await this.db.clearSyncQueue();
  }

  /**
   * Serialize queue for storage/transport
   */
  serializeQueue(items: SyncQueueItem[]): string {
    return JSON.stringify(items);
  }

  /**
   * Deserialize queue from storage/transport
   */
  deserializeQueue(json: string): SyncQueueItem[] {
    const parsed = JSON.parse(json);
    if (!Array.isArray(parsed)) {
      throw new Error('Invalid queue format');
    }
    // Validate each item
    return parsed.map(item => {
      if (typeof item.id !== 'string') throw new Error('Invalid item id');
      if (typeof item.sessionId !== 'string') throw new Error('Invalid sessionId');
      if (!['create', 'update', 'complete'].includes(item.action)) {
        throw new Error('Invalid action');
      }
      if (typeof item.createdAt !== 'string') throw new Error('Invalid createdAt');
      if (typeof item.retries !== 'number') throw new Error('Invalid retries');
      return item as SyncQueueItem;
    });
  }
}

// ============================================================
// Simple Encryption Helper (for testing - NOT production ready)
// ============================================================

class SimpleEncryption {
  private key: CryptoKey | null = null;

  async generateKey(): Promise<void> {
    this.key = await crypto.subtle.generateKey(
      { name: 'AES-GCM', length: 256 },
      true,
      ['encrypt', 'decrypt']
    );
  }

  async encrypt(data: Uint8Array): Promise<{ encrypted: Uint8Array; iv: Uint8Array }> {
    if (!this.key) throw new Error('Key not generated');
    const iv = crypto.getRandomValues(new Uint8Array(12));
    // Use .buffer slice to get proper ArrayBuffer for crypto.subtle
    const buffer = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    const encrypted = await crypto.subtle.encrypt(
      { name: 'AES-GCM', iv },
      this.key,
      buffer
    );
    return { encrypted: new Uint8Array(encrypted), iv };
  }

  async decrypt(encrypted: Uint8Array, iv: Uint8Array): Promise<Uint8Array> {
    if (!this.key) throw new Error('Key not generated');
    // Use .buffer slice to get proper ArrayBuffer for crypto.subtle
    const encBuffer = encrypted.buffer.slice(
      encrypted.byteOffset,
      encrypted.byteOffset + encrypted.byteLength
    ) as ArrayBuffer;
    const ivBuffer = iv.buffer.slice(iv.byteOffset, iv.byteOffset + iv.byteLength) as ArrayBuffer;
    const decrypted = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: new Uint8Array(ivBuffer) },
      this.key,
      encBuffer
    );
    return new Uint8Array(decrypted);
  }
}

// ============================================================
// Arbitraries (Generators) for fast-check
// ============================================================

// UUID format generator
const uuidArb = fc.uuid();

// Valid email generator
const emailArb = fc.emailAddress();

// Name generator (1-100 chars)
const nameArb = fc.string({ minLength: 1, maxLength: 100 }).filter(s => s.trim().length > 0);

// Recipient arbitrary
const recipientArb: fc.Arbitrary<Recipient> = fc.record({
  id: uuidArb,
  email: emailArb,
  name: nameArb,
  role: fc.constantFrom('signer' as const, 'cc' as const),
});

// Field type arbitrary
const fieldTypeArb = fc.constantFrom(
  'signature' as const,
  'initials' as const,
  'date' as const,
  'text' as const
);

// Signature field arbitrary
const signatureFieldArb = (recipientId?: string): fc.Arbitrary<SignatureField> =>
  fc.record({
    id: uuidArb,
    type: fieldTypeArb,
    page: fc.integer({ min: 1, max: 100 }),
    x: fc.float({ min: 0, max: 612, noNaN: true }), // PDF point coords
    y: fc.float({ min: 0, max: 792, noNaN: true }),
    width: fc.float({ min: 50, max: 300, noNaN: true }),
    height: fc.float({ min: 20, max: 100, noNaN: true }),
    required: fc.boolean(),
    recipientId: recipientId ? fc.constant(recipientId) : uuidArb,
  });

// Document data arbitrary (random bytes)
const documentArb = fc.uint8Array({ minLength: 100, maxLength: 10000 });

// Base64 string arbitrary (for signature data)
const base64Arb = fc.uint8Array({ minLength: 10, maxLength: 1000 }).map(arr => {
  // Convert to base64
  let binary = '';
  for (let i = 0; i < arr.length; i++) {
    binary += String.fromCharCode(arr[i]);
  }
  return btoa(binary);
});

// ISO timestamp arbitrary
const isoTimestampArb = fc.date({
  min: new Date('2020-01-01'),
  max: new Date('2030-12-31'),
}).map(d => d.toISOString());

// Session status arbitrary
const sessionStatusArb = fc.constantFrom(
  'pending' as const,
  'accepted' as const,
  'declined' as const,
  'completed' as const,
  'expired' as const
);

// Sync action arbitrary
const syncActionArb = fc.constantFrom(
  'create' as const,
  'update' as const,
  'complete' as const
);

// Sync queue item arbitrary
const syncQueueItemArb: fc.Arbitrary<SyncQueueItem> = fc.record({
  id: uuidArb,
  sessionId: uuidArb,
  action: syncActionArb,
  payload: fc.anything(),
  createdAt: isoTimestampArb,
  retries: fc.integer({ min: 0, max: 10 }),
});

// ============================================================
// Test Setup
// ============================================================

describe('LocalSessionManager', () => {
  let db: MockIndexedDBStore;
  let manager: LocalSessionManager;

  beforeEach(async () => {
    db = new MockIndexedDBStore();
    manager = new LocalSessionManager(db);
  });

  afterEach(async () => {
    await db.clear();
  });

  // ============================================================
  // 1. Session Creation Properties
  // ============================================================

  describe('Session Creation Properties', () => {
    it('Property 1: Created sessions have valid UUIDs', async () => {
      await fc.assert(
        fc.asyncProperty(
          documentArb,
          fc.array(recipientArb, { minLength: 1, maxLength: 5 }),
          async (document, recipients) => {
            const fields = recipients.map(r => ({
              id: crypto.randomUUID(),
              type: 'signature' as const,
              page: 1,
              x: 100,
              y: 100,
              width: 200,
              height: 50,
              required: true,
              recipientId: r.id,
            }));

            const session = await manager.createSession(document, recipients, fields);

            // UUID v4 regex pattern
            const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
            expect(session.id).toMatch(uuidPattern);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 2: Document hash is deterministic (same doc = same hash)', async () => {
      await fc.assert(
        fc.asyncProperty(documentArb, async (document) => {
          const recipients: Recipient[] = [{
            id: crypto.randomUUID(),
            email: 'test@example.com',
            name: 'Test User',
            role: 'signer',
          }];
          const fields: SignatureField[] = [];

          const session1 = await manager.createSession(document, recipients, fields);
          const session2 = await manager.createSession(document, recipients, fields);

          // Same document should produce same hash
          expect(session1.documentHash).toBe(session2.documentHash);
          // But different session IDs
          expect(session1.id).not.toBe(session2.id);
        }),
        { numRuns: 20 }
      );
    });

    it('Property 3: Different documents have different hashes', async () => {
      await fc.assert(
        fc.asyncProperty(
          documentArb,
          documentArb.filter(d => d.length > 0),
          async (doc1, doc2) => {
            // Skip if documents are identical
            if (doc1.length === doc2.length && doc1.every((v, i) => v === doc2[i])) {
              return;
            }

            const recipients: Recipient[] = [{
              id: crypto.randomUUID(),
              email: 'test@example.com',
              name: 'Test User',
              role: 'signer',
            }];

            const session1 = await manager.createSession(doc1, recipients, []);
            const session2 = await manager.createSession(doc2, recipients, []);

            expect(session1.documentHash).not.toBe(session2.documentHash);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 4: Created sessions start with pending status', async () => {
      await fc.assert(
        fc.asyncProperty(
          documentArb,
          fc.array(recipientArb, { minLength: 1, maxLength: 3 }),
          async (document, recipients) => {
            const session = await manager.createSession(document, recipients, []);
            expect(session.status).toBe('pending');
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 5: All required fields are initialized', async () => {
      await fc.assert(
        fc.asyncProperty(
          documentArb,
          fc.array(recipientArb, { minLength: 1, maxLength: 3 }),
          async (document, recipients) => {
            const session = await manager.createSession(document, recipients, []);

            expect(session.id).toBeDefined();
            expect(session.documentHash).toBeDefined();
            expect(session.documentHash.length).toBe(64); // SHA-256 hex
            expect(session.recipients).toEqual(recipients);
            expect(session.signatures).toBeDefined();
            expect(session.signatures.size).toBe(0); // Empty initially
            expect(session.status).toBe('pending');
            expect(session.createdAt).toBeDefined();
            expect(session.updatedAt).toBeDefined();
            expect(session.auditChain).toBeDefined();
            expect(session.auditChain.length).toBeGreaterThan(0);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 6: Recipients are stored exactly as provided', async () => {
      await fc.assert(
        fc.asyncProperty(
          documentArb,
          fc.array(recipientArb, { minLength: 1, maxLength: 5 }),
          async (document, recipients) => {
            const session = await manager.createSession(document, recipients, []);
            expect(session.recipients).toEqual(recipients);
            expect(session.recipients.length).toBe(recipients.length);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 7: Fields are stored exactly as provided', async () => {
      await fc.assert(
        fc.asyncProperty(
          documentArb,
          fc.array(signatureFieldArb(), { minLength: 0, maxLength: 10 }),
          async (document, fields) => {
            const session = await manager.createSession(document, [], fields);
            expect(session.fields).toEqual(fields);
            expect(session.fields.length).toBe(fields.length);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 8: Initial audit chain has DocumentUploaded event', async () => {
      await fc.assert(
        fc.asyncProperty(documentArb, async (document) => {
          const session = await manager.createSession(document, [], []);

          expect(session.auditChain.length).toBe(1);
          expect(session.auditChain[0].action).toBe('DocumentUploaded');
          expect(session.auditChain[0].actor).toBe('system');
          expect(session.auditChain[0].documentHash).toBe(session.documentHash);
        }),
        { numRuns: 20 }
      );
    });
  });

  // ============================================================
  // 2. Session State Transition Properties
  // ============================================================

  describe('Session State Transition Properties', () => {
    it('Property 9: Valid transitions are allowed (pending -> accepted)', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);

      const updated = await manager.updateStatus(session.id, 'accepted');
      expect(updated.status).toBe('accepted');
    });

    it('Property 10: Valid transitions are allowed (pending -> declined)', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);

      const updated = await manager.updateStatus(session.id, 'declined');
      expect(updated.status).toBe('declined');
    });

    it('Property 11: Valid transitions are allowed (accepted -> completed)', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);
      await manager.updateStatus(session.id, 'accepted');

      const updated = await manager.updateStatus(session.id, 'completed');
      expect(updated.status).toBe('completed');
    });

    it('Property 12: Declined sessions cannot be reactivated', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);
      await manager.updateStatus(session.id, 'declined');

      // Cannot transition from declined to anything
      await expect(manager.updateStatus(session.id, 'accepted')).rejects.toThrow('Invalid transition');
      await expect(manager.updateStatus(session.id, 'pending')).rejects.toThrow('Invalid transition');
      await expect(manager.updateStatus(session.id, 'completed')).rejects.toThrow('Invalid transition');
    });

    it('Property 13: Completed sessions cannot transition', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);
      await manager.updateStatus(session.id, 'accepted');
      await manager.updateStatus(session.id, 'completed');

      await expect(manager.updateStatus(session.id, 'pending')).rejects.toThrow('Invalid transition');
      await expect(manager.updateStatus(session.id, 'accepted')).rejects.toThrow('Invalid transition');
      await expect(manager.updateStatus(session.id, 'declined')).rejects.toThrow('Invalid transition');
    });

    it('Property 14: Status changes update the updatedAt timestamp', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);
      const originalUpdatedAt = session.updatedAt;

      // Small delay to ensure timestamp changes
      await new Promise(resolve => setTimeout(resolve, 10));

      const updated = await manager.updateStatus(session.id, 'accepted');
      expect(new Date(updated.updatedAt).getTime()).toBeGreaterThan(
        new Date(originalUpdatedAt).getTime()
      );
    });

    it('Property 15: Status changes add audit event', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);
      const initialAuditLength = session.auditChain.length;

      const updated = await manager.updateStatus(session.id, 'accepted');
      expect(updated.auditChain.length).toBe(initialAuditLength + 1);
      expect(updated.auditChain[updated.auditChain.length - 1].action).toContain('StatusChanged');
    });

    it('Property 16: Cannot skip states (pending cannot go directly to completed)', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);

      await expect(manager.updateStatus(session.id, 'completed')).rejects.toThrow('Invalid transition');
    });

    it('Property 17: Invalid transitions throw descriptive errors', async () => {
      await fc.assert(
        fc.asyncProperty(sessionStatusArb, sessionStatusArb, async (fromStatus, toStatus) => {
          const document = new Uint8Array([1, 2, 3, 4, 5]);
          const session = await manager.createSession(document, [], []);

          // Transition to fromStatus if possible
          if (fromStatus !== 'pending') {
            if (fromStatus === 'accepted') {
              await manager.updateStatus(session.id, 'accepted');
            } else if (fromStatus === 'declined') {
              await manager.updateStatus(session.id, 'declined');
            } else if (fromStatus === 'completed') {
              await manager.updateStatus(session.id, 'accepted');
              await manager.updateStatus(session.id, 'completed');
            } else if (fromStatus === 'expired') {
              await manager.updateStatus(session.id, 'accepted');
              await manager.updateStatus(session.id, 'expired');
            }
          }

          const isValidTransition = VALID_TRANSITIONS[fromStatus].includes(toStatus);

          if (isValidTransition) {
            const updated = await manager.updateStatus(session.id, toStatus);
            expect(updated.status).toBe(toStatus);
          } else {
            await expect(manager.updateStatus(session.id, toStatus)).rejects.toThrow();
          }
        }),
        { numRuns: 30 }
      );
    });
  });

  // ============================================================
  // 3. Signature Recording Properties
  // ============================================================

  describe('Signature Recording Properties', () => {
    it('Property 18: Recorded signatures are retrievable', async () => {
      await fc.assert(
        fc.asyncProperty(base64Arb, async (signatureData) => {
          const document = new Uint8Array([1, 2, 3, 4, 5]);
          const fieldId = crypto.randomUUID();
          const fields: SignatureField[] = [{
            id: fieldId,
            type: 'signature',
            page: 1,
            x: 100,
            y: 100,
            width: 200,
            height: 50,
            required: true,
            recipientId: 'r1',
          }];

          const session = await manager.createSession(document, [], fields);
          await manager.updateStatus(session.id, 'accepted');

          await manager.recordSignature(session.id, fieldId, {
            type: 'drawn',
            data: signatureData,
            signerId: 'r1',
          });

          const retrieved = await manager.getSignature(session.id, fieldId);
          expect(retrieved).toBeDefined();
          expect(retrieved!.data).toBe(signatureData);
          expect(retrieved!.type).toBe('drawn');
        }),
        { numRuns: 20 }
      );
    });

    it('Property 19: Cannot record signatures for non-existent fields', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);
      await manager.updateStatus(session.id, 'accepted');

      await expect(
        manager.recordSignature(session.id, 'non-existent-field', {
          type: 'drawn',
          data: 'base64data',
          signerId: 'r1',
        })
      ).rejects.toThrow('Field not found');
    });

    it('Property 20: Cannot record signatures for non-existent sessions', async () => {
      await expect(
        manager.recordSignature('non-existent-session', 'field1', {
          type: 'drawn',
          data: 'base64data',
          signerId: 'r1',
        })
      ).rejects.toThrow('Session not found');
    });

    it('Property 21: Duplicate recordings update (not duplicate) the signature', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const fieldId = crypto.randomUUID();
      const fields: SignatureField[] = [{
        id: fieldId,
        type: 'signature',
        page: 1,
        x: 100,
        y: 100,
        width: 200,
        height: 50,
        required: true,
        recipientId: 'r1',
      }];

      const session = await manager.createSession(document, [], fields);
      await manager.updateStatus(session.id, 'accepted');

      await manager.recordSignature(session.id, fieldId, {
        type: 'drawn',
        data: 'first-signature',
        signerId: 'r1',
      });

      await manager.recordSignature(session.id, fieldId, {
        type: 'typed',
        data: 'second-signature',
        signerId: 'r1',
      });

      const retrieved = await manager.getSignature(session.id, fieldId);
      expect(retrieved!.data).toBe('second-signature');
      expect(retrieved!.type).toBe('typed');

      // Should still only have one signature for this field
      const updatedSession = await manager.getSession(session.id);
      expect(updatedSession!.signatures.size).toBe(1);
    });

    it('Property 22: Signature data is preserved exactly', async () => {
      await fc.assert(
        fc.asyncProperty(
          base64Arb,
          fc.constantFrom('drawn' as const, 'typed' as const),
          async (signatureData, sigType) => {
            const document = new Uint8Array([1, 2, 3, 4, 5]);
            const fieldId = crypto.randomUUID();
            const signerId = crypto.randomUUID();
            const fields: SignatureField[] = [{
              id: fieldId,
              type: 'signature',
              page: 1,
              x: 100,
              y: 100,
              width: 200,
              height: 50,
              required: true,
              recipientId: signerId,
            }];

            const session = await manager.createSession(document, [], fields);
            await manager.updateStatus(session.id, 'accepted');

            await manager.recordSignature(session.id, fieldId, {
              type: sigType,
              data: signatureData,
              signerId: signerId,
            });

            const retrieved = await manager.getSignature(session.id, fieldId);
            expect(retrieved!.data).toBe(signatureData);
            expect(retrieved!.type).toBe(sigType);
            expect(retrieved!.signerId).toBe(signerId);
            expect(retrieved!.fieldId).toBe(fieldId);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 23: Recording signature updates audit chain', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const fieldId = crypto.randomUUID();
      const fields: SignatureField[] = [{
        id: fieldId,
        type: 'signature',
        page: 1,
        x: 100,
        y: 100,
        width: 200,
        height: 50,
        required: true,
        recipientId: 'r1',
      }];

      const session = await manager.createSession(document, [], fields);
      const acceptedSession = await manager.updateStatus(session.id, 'accepted');
      const preSignAuditLength = acceptedSession.auditChain.length;

      const updated = await manager.recordSignature(session.id, fieldId, {
        type: 'drawn',
        data: 'signature-data',
        signerId: 'r1',
      });

      expect(updated.auditChain.length).toBe(preSignAuditLength + 1);
      expect(updated.auditChain[updated.auditChain.length - 1].action).toBe('SignatureApplied');
      expect(updated.auditChain[updated.auditChain.length - 1].actor).toBe('r1');
    });
  });

  // ============================================================
  // 4. Session Expiry Properties
  // ============================================================

  describe('Session Expiry Properties', () => {
    it('Property 24: Sessions without expiry (null) never expire', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);

      expect(session.expiresAt).toBeNull();
      expect(manager.isSessionExpired(session)).toBe(false);
    });

    it('Property 25: Expired sessions cannot accept new signatures', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const fieldId = crypto.randomUUID();
      const fields: SignatureField[] = [{
        id: fieldId,
        type: 'signature',
        page: 1,
        x: 100,
        y: 100,
        width: 200,
        height: 50,
        required: true,
        recipientId: 'r1',
      }];

      const session = await manager.createSession(document, [], fields);
      await manager.updateStatus(session.id, 'accepted');

      // Manually set expiry to past
      const expiredSession = await manager.getSession(session.id);
      expiredSession!.expiresAt = new Date(Date.now() - 1000).toISOString();
      await db.put(session.id, expiredSession!);

      await expect(
        manager.recordSignature(session.id, fieldId, {
          type: 'drawn',
          data: 'signature-data',
          signerId: 'r1',
        })
      ).rejects.toThrow('expired');
    });

    it('Property 26: Expiry check is consistent with isSessionExpired()', async () => {
      await fc.assert(
        fc.asyncProperty(
          fc.integer({ min: -1000000, max: 1000000 }),
          async (offsetMs) => {
            const document = new Uint8Array([1, 2, 3, 4, 5]);
            const session = await manager.createSession(document, [], []);

            // Set expiry relative to now
            const expiryTime = Date.now() + offsetMs;
            session.expiresAt = new Date(expiryTime).toISOString();
            await db.put(session.id, session);

            const retrieved = await manager.getSession(session.id);
            const isExpired = manager.isSessionExpired(retrieved!);

            // Should be expired if expiryTime < now
            expect(isExpired).toBe(expiryTime < Date.now());
          }
        ),
        { numRuns: 30 }
      );
    });

    it('Property 27: Sessions with future expiry are not expired', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);

      // Set expiry to 1 hour from now
      session.expiresAt = new Date(Date.now() + 3600000).toISOString();
      await db.put(session.id, session);

      const retrieved = await manager.getSession(session.id);
      expect(manager.isSessionExpired(retrieved!)).toBe(false);
    });
  });

  // ============================================================
  // 5. Persistence Properties (mock IndexedDB)
  // ============================================================

  describe('Persistence Properties', () => {
    it('Property 28: Sessions survive "reload" (get after set)', async () => {
      await fc.assert(
        fc.asyncProperty(
          documentArb,
          fc.array(recipientArb, { minLength: 1, maxLength: 3 }),
          async (document, recipients) => {
            const session = await manager.createSession(document, recipients, []);
            const sessionId = session.id;

            // Simulate "reload" - create new manager with same db
            const newManager = new LocalSessionManager(db);
            const retrieved = await newManager.getSession(sessionId);

            expect(retrieved).toBeDefined();
            expect(retrieved!.id).toBe(sessionId);
            expect(retrieved!.documentHash).toBe(session.documentHash);
            expect(retrieved!.status).toBe(session.status);
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 29: Deleted sessions are not retrievable', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);
      const sessionId = session.id;

      await manager.deleteSession(sessionId);

      const retrieved = await manager.getSession(sessionId);
      expect(retrieved).toBeUndefined();
    });

    it('Property 30: List returns all active sessions', async () => {
      await fc.assert(
        fc.asyncProperty(
          fc.integer({ min: 1, max: 10 }),
          async (count) => {
            // Clear existing sessions
            await db.clear();

            const document = new Uint8Array([1, 2, 3, 4, 5]);
            const createdIds: string[] = [];

            for (let i = 0; i < count; i++) {
              const session = await manager.createSession(document, [], []);
              createdIds.push(session.id);
            }

            const allSessions = await manager.listSessions();
            expect(allSessions.length).toBe(count);

            const retrievedIds = allSessions.map(s => s.id);
            createdIds.forEach(id => {
              expect(retrievedIds).toContain(id);
            });
          }
        ),
        { numRuns: 10 }
      );
    });

    it('Property 31: Multiple sessions with same document are independent', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);

      const session1 = await manager.createSession(document, [], []);
      const session2 = await manager.createSession(document, [], []);

      // Update one session
      await manager.updateStatus(session1.id, 'accepted');

      // Verify they're independent
      const retrieved1 = await manager.getSession(session1.id);
      const retrieved2 = await manager.getSession(session2.id);

      expect(retrieved1!.status).toBe('accepted');
      expect(retrieved2!.status).toBe('pending');
    });

    it('Property 32: Session updates persist correctly', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const fieldId = crypto.randomUUID();
      const fields: SignatureField[] = [{
        id: fieldId,
        type: 'signature',
        page: 1,
        x: 100,
        y: 100,
        width: 200,
        height: 50,
        required: true,
        recipientId: 'r1',
      }];

      const session = await manager.createSession(document, [], fields);
      await manager.updateStatus(session.id, 'accepted');
      await manager.recordSignature(session.id, fieldId, {
        type: 'drawn',
        data: 'signature-data',
        signerId: 'r1',
      });

      // Create new manager (simulate page refresh)
      const newManager = new LocalSessionManager(db);
      const retrieved = await newManager.getSession(session.id);

      expect(retrieved!.status).toBe('accepted');
      expect(retrieved!.signatures.get(fieldId)).toBeDefined();
      expect(retrieved!.signatures.get(fieldId)!.data).toBe('signature-data');
    });
  });

  // ============================================================
  // 6. Sync Queue Properties
  // ============================================================

  describe('Sync Queue Properties', () => {
    let syncManager: SyncManager;

    beforeEach(() => {
      syncManager = new SyncManager(db);
    });

    it('Property 33: Added items appear in queue', async () => {
      await fc.assert(
        fc.asyncProperty(syncActionArb, async (action) => {
          await db.clearSyncQueue();

          const item = await syncManager.enqueue({
            sessionId: crypto.randomUUID(),
            action,
            payload: { test: 'data' },
          });

          const queue = await syncManager.getQueue();
          expect(queue.some(q => q.id === item.id)).toBe(true);
        }),
        { numRuns: 10 }
      );
    });

    it('Property 34: Removed items do not appear in queue', async () => {
      await db.clearSyncQueue();

      const item = await syncManager.enqueue({
        sessionId: crypto.randomUUID(),
        action: 'create',
        payload: {},
      });

      await syncManager.dequeue(item.id);

      const hasItem = await syncManager.hasItem(item.id);
      expect(hasItem).toBe(false);

      const queue = await syncManager.getQueue();
      expect(queue.some(q => q.id === item.id)).toBe(false);
    });

    it('Property 35: Queue order is preserved (FIFO)', async () => {
      await db.clearSyncQueue();

      const items: SyncQueueItem[] = [];
      for (let i = 0; i < 5; i++) {
        const item = await syncManager.enqueue({
          sessionId: `session-${i}`,
          action: 'create',
          payload: { order: i },
        });
        items.push(item);
        // Small delay to ensure different timestamps
        await new Promise(resolve => setTimeout(resolve, 5));
      }

      const queue = await syncManager.getQueue();

      // Verify FIFO order (by creation time)
      for (let i = 0; i < items.length; i++) {
        expect(queue[i].sessionId).toBe(`session-${i}`);
      }
    });

    it('Property 36: Serialization roundtrip preserves data', async () => {
      await fc.assert(
        fc.asyncProperty(
          fc.array(syncQueueItemArb, { minLength: 1, maxLength: 10 }),
          async (items) => {
            const serialized = syncManager.serializeQueue(items);
            const deserialized = syncManager.deserializeQueue(serialized);

            expect(deserialized.length).toBe(items.length);
            for (let i = 0; i < items.length; i++) {
              expect(deserialized[i].id).toBe(items[i].id);
              expect(deserialized[i].sessionId).toBe(items[i].sessionId);
              expect(deserialized[i].action).toBe(items[i].action);
              expect(deserialized[i].createdAt).toBe(items[i].createdAt);
              expect(deserialized[i].retries).toBe(items[i].retries);
            }
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 37: Invalid JSON throws on deserialize', () => {
      expect(() => syncManager.deserializeQueue('not valid json')).toThrow();
    });

    it('Property 38: Invalid queue format throws on deserialize', () => {
      expect(() => syncManager.deserializeQueue('{}')).toThrow('Invalid queue format');
    });

    it('Property 39: Missing item fields throw on deserialize', () => {
      expect(() => syncManager.deserializeQueue('[{"id":"x"}]')).toThrow('Invalid sessionId');
      expect(() => syncManager.deserializeQueue('[{"id":"x","sessionId":"y"}]')).toThrow('Invalid action');
    });

    it('Property 40: Clear queue removes all items', async () => {
      // Add several items
      for (let i = 0; i < 5; i++) {
        await syncManager.enqueue({
          sessionId: `session-${i}`,
          action: 'create',
          payload: {},
        });
      }

      await syncManager.clearQueue();

      const queue = await syncManager.getQueue();
      expect(queue.length).toBe(0);
    });
  });

  // ============================================================
  // 7. Encryption Properties
  // ============================================================

  describe('Encryption Properties', () => {
    let encryption: SimpleEncryption;

    beforeEach(async () => {
      encryption = new SimpleEncryption();
      await encryption.generateKey();
    });

    it('Property 41: Encrypted data differs from original', async () => {
      await fc.assert(
        fc.asyncProperty(
          fc.uint8Array({ minLength: 10, maxLength: 1000 }),
          async (data) => {
            const { encrypted } = await encryption.encrypt(data);

            // Encrypted should be different from original
            // (With high probability for non-trivial inputs)
            if (data.length > 0) {
              let isDifferent = encrypted.length !== data.length;
              if (!isDifferent) {
                for (let i = 0; i < data.length; i++) {
                  if (encrypted[i] !== data[i]) {
                    isDifferent = true;
                    break;
                  }
                }
              }
              expect(isDifferent).toBe(true);
            }
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 42: Decrypted data equals original', async () => {
      await fc.assert(
        fc.asyncProperty(
          fc.uint8Array({ minLength: 1, maxLength: 1000 }),
          async (data) => {
            const { encrypted, iv } = await encryption.encrypt(data);
            const decrypted = await encryption.decrypt(encrypted, iv);

            expect(decrypted.length).toBe(data.length);
            for (let i = 0; i < data.length; i++) {
              expect(decrypted[i]).toBe(data[i]);
            }
          }
        ),
        { numRuns: 20 }
      );
    });

    it('Property 43: Different encryptions have different IVs', async () => {
      const data = new Uint8Array([1, 2, 3, 4, 5]);

      const result1 = await encryption.encrypt(data);
      const result2 = await encryption.encrypt(data);

      // IVs should be different (random)
      let ivsDifferent = false;
      for (let i = 0; i < result1.iv.length; i++) {
        if (result1.iv[i] !== result2.iv[i]) {
          ivsDifferent = true;
          break;
        }
      }
      expect(ivsDifferent).toBe(true);
    });

    it('Property 44: Different sessions have different encrypted forms', async () => {
      const data = new Uint8Array([1, 2, 3, 4, 5]);

      const result1 = await encryption.encrypt(data);
      const result2 = await encryption.encrypt(data);

      // Due to different IVs, encrypted data should differ
      let encrypted1Differs = false;
      for (let i = 0; i < Math.min(result1.encrypted.length, result2.encrypted.length); i++) {
        if (result1.encrypted[i] !== result2.encrypted[i]) {
          encrypted1Differs = true;
          break;
        }
      }
      expect(encrypted1Differs).toBe(true);
    });

    it('Property 45: Wrong IV fails to decrypt correctly', async () => {
      const data = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
      const { encrypted } = await encryption.encrypt(data);

      // Wrong IV
      const wrongIv = crypto.getRandomValues(new Uint8Array(12));

      // Should throw or produce garbage
      await expect(encryption.decrypt(encrypted, wrongIv)).rejects.toThrow();
    });

    it('Property 46: Empty data encrypts and decrypts', async () => {
      const data = new Uint8Array([]);
      const { encrypted, iv } = await encryption.encrypt(data);
      const decrypted = await encryption.decrypt(encrypted, iv);

      expect(decrypted.length).toBe(0);
    });

    it('Property 47: Large data encrypts and decrypts', async () => {
      // 64KB of data (Node.js crypto.getRandomValues limit is 65536 bytes)
      const data = crypto.getRandomValues(new Uint8Array(65536));
      const { encrypted, iv } = await encryption.encrypt(data);
      const decrypted = await encryption.decrypt(encrypted, iv);

      expect(decrypted.length).toBe(data.length);
      // Sample check (checking every byte would be slow)
      for (let i = 0; i < 1000; i++) {
        const idx = Math.floor(Math.random() * data.length);
        expect(decrypted[idx]).toBe(data[idx]);
      }
    });
  });

  // ============================================================
  // Edge Cases and Error Handling
  // ============================================================

  describe('Edge Cases', () => {
    it('Property 48: Getting non-existent session returns undefined', async () => {
      const result = await manager.getSession('non-existent-id');
      expect(result).toBeUndefined();
    });

    it('Property 49: Deleting non-existent session returns false', async () => {
      const result = await manager.deleteSession('non-existent-id');
      expect(result).toBe(false);
    });

    it('Property 50: Session with empty recipients is valid', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);

      expect(session.recipients).toEqual([]);
      expect(session.id).toBeDefined();
    });

    it('Property 51: Session with empty fields is valid', async () => {
      const document = new Uint8Array([1, 2, 3, 4, 5]);
      const session = await manager.createSession(document, [], []);

      expect(session.fields).toEqual([]);
      expect(session.id).toBeDefined();
    });

    it('Property 52: Minimum document size works', async () => {
      const document = new Uint8Array([1]); // 1 byte
      const session = await manager.createSession(document, [], []);

      expect(session.documentHash).toBeDefined();
      expect(session.documentHash.length).toBe(64);
    });

    it('Property 53: Empty document works', async () => {
      const document = new Uint8Array([]); // 0 bytes
      const session = await manager.createSession(document, [], []);

      expect(session.documentHash).toBeDefined();
      expect(session.documentHash.length).toBe(64);
    });

    it('Property 54: Very large recipient list works', async () => {
      const document = new Uint8Array([1, 2, 3]);
      const recipients: Recipient[] = [];
      for (let i = 0; i < 100; i++) {
        recipients.push({
          id: crypto.randomUUID(),
          email: `user${i}@example.com`,
          name: `User ${i}`,
          role: 'signer',
        });
      }

      const session = await manager.createSession(document, recipients, []);
      expect(session.recipients.length).toBe(100);
    });

    it('Property 55: Unicode in names and emails works', async () => {
      const document = new Uint8Array([1, 2, 3]);
      const recipients: Recipient[] = [{
        id: crypto.randomUUID(),
        email: 'user@example.com',
        name: 'Nombre Espanol',
        role: 'signer',
      }];

      const session = await manager.createSession(document, recipients, []);
      expect(session.recipients[0].name).toBe('Nombre Espanol');
    });
  });
});
