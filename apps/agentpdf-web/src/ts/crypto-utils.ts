/**
 * Crypto Utilities for IndexedDB Encryption
 *
 * Uses Web Crypto API for AES-GCM encryption at rest.
 *
 * Key storage strategy:
 * - A master key is derived from a device-specific seed
 * - The seed is stored in localStorage (not in IndexedDB to avoid circular dependency)
 * - The key never leaves the device
 *
 * Security notes:
 * - This protects against physical access to the device's IndexedDB files
 * - It does NOT protect against malicious JavaScript in the same origin
 * - For maximum security, users should use a Tauri desktop app (which has OS-level encryption)
 */

// ============================================================
// Constants
// ============================================================

import { createLogger } from './logger';

const log = createLogger('CryptoUtils');

const SEED_STORAGE_KEY = 'agentpdf_crypto_seed';
const KEY_ALGORITHM = 'AES-GCM';
const KEY_LENGTH = 256;
const IV_LENGTH = 12; // 96 bits for AES-GCM

// ============================================================
// Types
// ============================================================

export interface EncryptedData {
  /** Base64 encoded ciphertext */
  ciphertext: string;
  /** Base64 encoded IV */
  iv: string;
  /** Version for future compatibility */
  version: 1;
}

// ============================================================
// Key Management
// ============================================================

let cachedKey: CryptoKey | null = null;

/**
 * Get or create a device-specific encryption seed
 */
function getOrCreateSeed(): string {
  if (typeof localStorage === 'undefined') {
    throw new Error('localStorage is not available');
  }

  let seed = localStorage.getItem(SEED_STORAGE_KEY);
  if (!seed) {
    // Generate a new random seed
    const randomBytes = new Uint8Array(32);
    crypto.getRandomValues(randomBytes);
    seed = Array.from(randomBytes)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');
    localStorage.setItem(SEED_STORAGE_KEY, seed);
    log.debug('Generated new device seed');
  }
  return seed;
}

/**
 * Derive an encryption key from the device seed
 */
async function deriveKey(): Promise<CryptoKey> {
  if (cachedKey) {
    return cachedKey;
  }

  const seed = getOrCreateSeed();

  // Convert seed to bytes
  const seedBytes = new Uint8Array(
    seed.match(/.{2}/g)!.map((byte) => parseInt(byte, 16))
  );

  // Import seed as raw key material
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    seedBytes,
    'PBKDF2',
    false,
    ['deriveKey']
  );

  // Derive the actual encryption key
  // Using a fixed salt is okay here because the seed itself is random
  const salt = new TextEncoder().encode('agentpdf-indexeddb-encryption-v1');

  cachedKey = await crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt,
      iterations: 100000,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: KEY_ALGORITHM, length: KEY_LENGTH },
    false,
    ['encrypt', 'decrypt']
  );

  log.debug('Derived encryption key');
  return cachedKey;
}

// ============================================================
// Encryption/Decryption
// ============================================================

/**
 * Encrypt a string using AES-GCM
 * @param plaintext The string to encrypt
 * @returns Encrypted data object
 */
export async function encryptString(plaintext: string): Promise<EncryptedData> {
  const key = await deriveKey();

  // Generate random IV
  const iv = crypto.getRandomValues(new Uint8Array(IV_LENGTH));

  // Encode plaintext
  const encodedText = new TextEncoder().encode(plaintext);

  // Encrypt
  const ciphertext = await crypto.subtle.encrypt(
    { name: KEY_ALGORITHM, iv },
    key,
    encodedText
  );

  // Convert to base64
  const ciphertextBase64 = btoa(
    String.fromCharCode(...new Uint8Array(ciphertext))
  );
  const ivBase64 = btoa(String.fromCharCode(...iv));

  return {
    ciphertext: ciphertextBase64,
    iv: ivBase64,
    version: 1,
  };
}

/**
 * Decrypt an encrypted data object
 * @param encrypted The encrypted data
 * @returns The decrypted plaintext
 */
export async function decryptString(encrypted: EncryptedData): Promise<string> {
  const key = await deriveKey();

  // Decode from base64
  const ciphertext = Uint8Array.from(atob(encrypted.ciphertext), (c) =>
    c.charCodeAt(0)
  );
  const iv = Uint8Array.from(atob(encrypted.iv), (c) => c.charCodeAt(0));

  // Decrypt
  const plaintext = await crypto.subtle.decrypt(
    { name: KEY_ALGORITHM, iv },
    key,
    ciphertext
  );

  // Decode to string
  return new TextDecoder().decode(plaintext);
}

/**
 * Encrypt binary data (Uint8Array)
 * @param data The binary data to encrypt
 * @returns Encrypted data object
 */
export async function encryptBytes(data: Uint8Array): Promise<EncryptedData> {
  const key = await deriveKey();

  // Generate random IV
  const iv = crypto.getRandomValues(new Uint8Array(IV_LENGTH));

  // Create a copy in a regular ArrayBuffer for compatibility
  const dataBuffer = new ArrayBuffer(data.length);
  new Uint8Array(dataBuffer).set(data);

  // Encrypt
  const ciphertext = await crypto.subtle.encrypt(
    { name: KEY_ALGORITHM, iv },
    key,
    dataBuffer
  );

  // Convert to base64
  const ciphertextBase64 = btoa(
    String.fromCharCode(...new Uint8Array(ciphertext))
  );
  const ivBase64 = btoa(String.fromCharCode(...iv));

  return {
    ciphertext: ciphertextBase64,
    iv: ivBase64,
    version: 1,
  };
}

/**
 * Decrypt to binary data
 * @param encrypted The encrypted data
 * @returns The decrypted Uint8Array
 */
export async function decryptBytes(encrypted: EncryptedData): Promise<Uint8Array> {
  const key = await deriveKey();

  // Decode from base64
  const ciphertextBytes = Uint8Array.from(atob(encrypted.ciphertext), (c) =>
    c.charCodeAt(0)
  );
  const ivBytes = Uint8Array.from(atob(encrypted.iv), (c) => c.charCodeAt(0));

  // Create ArrayBuffer copies for compatibility
  const cipherBuffer = new ArrayBuffer(ciphertextBytes.length);
  new Uint8Array(cipherBuffer).set(ciphertextBytes);

  const ivBuffer = new ArrayBuffer(ivBytes.length);
  new Uint8Array(ivBuffer).set(ivBytes);

  // Decrypt
  const plaintext = await crypto.subtle.decrypt(
    { name: KEY_ALGORITHM, iv: new Uint8Array(ivBuffer) },
    key,
    cipherBuffer
  );

  return new Uint8Array(plaintext);
}

// ============================================================
// Utility Functions
// ============================================================

/**
 * Check if data is encrypted (has our format)
 */
export function isEncrypted(data: unknown): data is EncryptedData {
  if (!data || typeof data !== 'object') return false;
  const obj = data as Record<string, unknown>;
  return (
    typeof obj.ciphertext === 'string' &&
    typeof obj.iv === 'string' &&
    obj.version === 1
  );
}

/**
 * Check if encryption is available
 */
export function isEncryptionAvailable(): boolean {
  return (
    typeof crypto !== 'undefined' &&
    typeof crypto.subtle !== 'undefined' &&
    typeof localStorage !== 'undefined'
  );
}

/**
 * Clear the encryption key and seed
 * WARNING: This will make all encrypted data unrecoverable!
 */
export function clearEncryptionKey(): void {
  cachedKey = null;
  if (typeof localStorage !== 'undefined') {
    localStorage.removeItem(SEED_STORAGE_KEY);
  }
  log.warn('Encryption key cleared - encrypted data is now unrecoverable');
}

/**
 * Export the seed for backup
 * User should store this securely outside the browser
 */
export function exportSeed(): string | null {
  if (typeof localStorage === 'undefined') return null;
  return localStorage.getItem(SEED_STORAGE_KEY);
}

/**
 * Import a seed for recovery
 * @param seed The seed to import
 */
export function importSeed(seed: string): void {
  if (typeof localStorage === 'undefined') {
    throw new Error('localStorage is not available');
  }
  if (!/^[0-9a-f]{64}$/i.test(seed)) {
    throw new Error('Invalid seed format (must be 64 hex characters)');
  }
  localStorage.setItem(SEED_STORAGE_KEY, seed.toLowerCase());
  cachedKey = null; // Force re-derivation
  log.info('Seed imported successfully');
}
