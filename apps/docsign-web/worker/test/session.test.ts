/**
 * Session API Integration Tests
 *
 * Bug #0 Regression Tests: Tests for session creation endpoint
 * including handling of invalid field values (NaN, undefined, etc.)
 *
 * Uses @cloudflare/vitest-pool-workers for true integration testing.
 */

import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { SELF, env, fetchMock } from "cloudflare:test";

// Enable fetch mocking before all tests
beforeAll(() => {
  fetchMock.activate();
  fetchMock.disableNetConnect();
});

// Setup persistent mock for Resend API before each test
beforeEach(() => {
  fetchMock.deactivate();
  fetchMock.activate();
  fetchMock.disableNetConnect();

  // Mock Resend API
  fetchMock
    .get("https://api.resend.com")
    .intercept({ path: "/emails", method: "POST" })
    .reply(200, { id: "mock-email-id-" + Math.random().toString(36).slice(2) })
    .persist();
});

/**
 * Helper to register and verify a user for testing
 * Returns the access token for authenticated requests
 */
async function createVerifiedUserAndLogin(
  email: string,
  password: string = "ValidPass123"
): Promise<string> {
  // Register the user
  const registerResponse = await SELF.fetch("https://worker/auth/register", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      email,
      password,
      first_name: "Test",
      last_name: "User",
    }),
  });

  if (registerResponse.status !== 201) {
    throw new Error(`Registration failed: ${registerResponse.status}`);
  }

  const registerData = (await registerResponse.json()) as { user_id: string };
  const userId = registerData.user_id;

  // Directly update user in KV to mark as verified
  // This bypasses email verification for testing
  const usersKv = env.USERS;
  const userKey = `user:${userId}`;
  const userData = await usersKv.get(userKey);
  if (userData) {
    const user = JSON.parse(userData);
    user.email_verified = true;
    await usersKv.put(userKey, JSON.stringify(user));
  }

  // Login to get access token
  const loginResponse = await SELF.fetch("https://worker/auth/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
  });

  if (loginResponse.status !== 200) {
    throw new Error(`Login failed: ${loginResponse.status}`);
  }

  const loginData = (await loginResponse.json()) as { access_token: string };
  return loginData.access_token;
}

describe("Session API Integration", () => {
  describe("Bug #0: Field Value Validation", () => {
    /**
     * This test captures Bug #0: NaN values in field positions cause BODY_PARSE_ERROR
     *
     * When field positions are calculated from undefined values, they become NaN:
     *   x_percent: undefined / canvasW * 100 = NaN
     *
     * Serde (Rust JSON parser) cannot parse NaN as f64, causing the request to fail
     * with "Invalid request format" error.
     *
     * Expected behavior: Server should either:
     * 1. Return a clear validation error explaining the issue, OR
     * 2. Accept the request with default values for invalid fields
     *
     * This test should FAIL until the bug is fixed.
     */
    it("should handle NaN values in field positions gracefully", async () => {
      const token = await createVerifiedUserAndLogin("nan-test@example.com");

      // Create a session request with NaN values (simulating the bug)
      // In JavaScript, null becomes NaN when used in arithmetic
      const sessionRequest = {
        encrypted_document: btoa("test document content"),
        metadata: {
          filename: "test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
        },
        recipients: [
          {
            id: "1",
            name: "Test Signer",
            email: "signer@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
        ],
        fields: [
          {
            id: "field_1",
            field_type: "signature",
            recipient_id: "1",
            page: 1,
            // These NaN values simulate the bug condition
            x_percent: NaN,
            y_percent: NaN,
            width_percent: NaN,
            height_percent: NaN,
            required: true,
            value: null,
          },
        ],
        expiry_hours: 168,
      };

      const response = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      // The bug causes a 400 BODY_PARSE_ERROR because Rust can't parse NaN
      // After the fix, this should either:
      // - Return 400 with a clear validation error about invalid field values, OR
      // - Return 200/201 with the session created (using default values)

      const data = (await response.json()) as {
        success: boolean;
        error_code?: string;
        message?: string;
        session_id?: string;
      };

      // Bug exists if we get BODY_PARSE_ERROR (serde can't parse NaN)
      // This assertion will FAIL while the bug exists
      expect(data.error_code).not.toBe("BODY_PARSE_ERROR");

      // After fix, should either succeed or have a specific validation error
      if (!data.success) {
        // If validation error, should be specific about field values
        expect(data.message).toMatch(/field|position|value|invalid/i);
      }
    });

    /**
     * Test that valid field values work correctly (regression test)
     */
    it("should accept valid field values", async () => {
      const token = await createVerifiedUserAndLogin("valid-fields@example.com");

      const sessionRequest = {
        encrypted_document: btoa("test document content"),
        metadata: {
          filename: "test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
        },
        recipients: [
          {
            id: "1",
            name: "Test Signer",
            email: "signer@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
        ],
        fields: [
          {
            id: "field_1",
            field_type: "signature",
            recipient_id: "1",
            page: 1,
            x_percent: 10.5,
            y_percent: 20.5,
            width_percent: 30.0,
            height_percent: 10.0,
            required: true,
            value: null,
          },
        ],
        expiry_hours: 168,
      };

      const response = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      expect(response.status).toBe(200);

      const data = (await response.json()) as {
        success: boolean;
        session_id: string;
      };

      expect(data.success).toBe(true);
      expect(data.session_id).toBeDefined();
    });

    /**
     * Test undefined field values (another variant of the bug)
     */
    it("should handle undefined/null field positions gracefully", async () => {
      const token = await createVerifiedUserAndLogin("undefined-test@example.com");

      // Create request where some fields are omitted (will be undefined in JSON)
      const sessionRequest = {
        encrypted_document: btoa("test document content"),
        metadata: {
          filename: "test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
        },
        recipients: [
          {
            id: "1",
            name: "Test Signer",
            email: "signer@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
        ],
        fields: [
          {
            id: "field_1",
            field_type: "signature",
            recipient_id: "1",
            page: 1,
            // Omitting x_percent, y_percent, width_percent, height_percent
            // These will be undefined in the JSON
            required: true,
            value: null,
          },
        ],
        expiry_hours: 168,
      };

      const response = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const data = (await response.json()) as {
        success: boolean;
        error_code?: string;
        message?: string;
      };

      // Should either succeed with defaults or return a clear validation error
      // Should NOT be a generic BODY_PARSE_ERROR
      if (!data.success) {
        expect(data.error_code).not.toBe("BODY_PARSE_ERROR");
        expect(data.message).toBeDefined();
      }
    });
  });

  /**
   * Bug #14: Signing Link Doesn't Load
   *
   * The signing flow fails with "OperationError" - a Web Crypto API error during
   * key import or decryption. This is caused by the frontend encryption/decryption
   * code, not the backend.
   *
   * Solution: Remove encryption temporarily to get the basic flow working.
   * The backend stores documents as opaque strings - it doesn't care if they're
   * encrypted or not.
   *
   * These tests verify the backend supports plain base64 document storage,
   * which is the expected format after removing frontend encryption.
   */
  describe("Bug #14: Non-Encrypted Document Flow", () => {
    /**
     * Test that a session can be created with a plain base64 document
     * (not encrypted). This is the expected format after removing frontend crypto.
     */
    it("should store plain base64 document and return it via KV", async () => {
      const token = await createVerifiedUserAndLogin("base64-doc-test@example.com");

      // Create a simple base64 encoded "document" (simulating non-encrypted PDF bytes)
      const originalContent = "This is the PDF content that would normally be encrypted";
      const documentBase64 = btoa(originalContent);

      const sessionRequest = {
        encrypted_document: documentBase64,  // Named "encrypted" but actually plain base64
        metadata: {
          filename: "test-unencrypted.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
        },
        recipients: [
          {
            id: "signer_1",
            name: "Test Signer",
            email: "signer@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
        ],
        fields: [
          {
            id: "sig_field_1",
            field_type: "signature",
            recipient_id: "signer_1",
            page: 1,
            x_percent: 50.0,
            y_percent: 80.0,
            width_percent: 20.0,
            height_percent: 5.0,
            required: true,
            value: null,
          },
        ],
        expiry_hours: 168,
      };

      // Create the session
      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      expect(createResponse.status).toBe(200);

      const createData = (await createResponse.json()) as {
        success: boolean;
        session_id: string;
      };

      expect(createData.success).toBe(true);
      expect(createData.session_id).toBeDefined();

      // Verify the session was stored correctly in KV
      const sessionsKv = env.SESSIONS;
      const sessionData = await sessionsKv.get(`session:${createData.session_id}`);
      expect(sessionData).not.toBeNull();

      const storedSession = JSON.parse(sessionData!) as {
        encrypted_document: string;
        metadata: { filename: string };
      };

      // Verify the document is stored exactly as sent (plain base64)
      expect(storedSession.encrypted_document).toBe(documentBase64);

      // Verify we can decode it back to original content
      const decodedContent = atob(storedSession.encrypted_document);
      expect(decodedContent).toBe(originalContent);
    });

    /**
     * Test that document data roundtrips correctly through the create/fetch flow.
     * This simulates what the signer would do when loading a signing session.
     *
     * NOTE: This test uses direct KV access since generating a valid signing token
     * requires the /invite flow. The full signer flow will be verified via Puppeteer.
     */
    it("should preserve document integrity through storage", async () => {
      const token = await createVerifiedUserAndLogin("roundtrip-test@example.com");

      // Simulate actual PDF bytes (a minimal PDF-like structure)
      const pdfLikeContent = "%PDF-1.4 fake pdf content for testing integrity";
      const documentBase64 = btoa(pdfLikeContent);

      const sessionRequest = {
        encrypted_document: documentBase64,
        metadata: {
          filename: "integrity-test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
        },
        recipients: [
          {
            id: "signer_1",
            name: "Test Signer",
            email: "signer@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
        ],
        fields: [],
        expiry_hours: 168,
      };

      // Create session
      const response = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const data = (await response.json()) as { session_id: string };

      // Read back from KV and verify integrity
      const sessionsKv = env.SESSIONS;
      const stored = await sessionsKv.get(`session:${data.session_id}`);
      const session = JSON.parse(stored!) as { encrypted_document: string };

      // Decode the stored base64 back to bytes
      const decodedBytes = atob(session.encrypted_document);

      // Verify integrity - content should match exactly
      expect(decodedBytes).toBe(pdfLikeContent);
      expect(decodedBytes.startsWith("%PDF")).toBe(true);
    });

    /**
     * Test that larger documents work correctly (more realistic PDF size)
     */
    it("should handle larger base64 documents", async () => {
      const token = await createVerifiedUserAndLogin("large-doc-test@example.com");

      // Create a larger "document" (~10KB of content)
      let largeContent = "%PDF-1.4\n";
      for (let i = 0; i < 100; i++) {
        largeContent += `Line ${i}: ${"x".repeat(100)}\n`;
      }
      const documentBase64 = btoa(largeContent);

      const sessionRequest = {
        encrypted_document: documentBase64,
        metadata: {
          filename: "large-test.pdf",
          page_count: 10,
          created_at: new Date().toISOString(),
          created_by: "Test User",
        },
        recipients: [
          {
            id: "signer_1",
            name: "Test Signer",
            email: "signer@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
        ],
        fields: [],
        expiry_hours: 168,
      };

      const response = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      expect(response.status).toBe(200);

      const data = (await response.json()) as {
        success: boolean;
        session_id: string;
      };

      expect(data.success).toBe(true);

      // Verify storage
      const sessionsKv = env.SESSIONS;
      const stored = await sessionsKv.get(`session:${data.session_id}`);
      const session = JSON.parse(stored!) as { encrypted_document: string };

      // Verify we can decode it back correctly
      const decoded = atob(session.encrypted_document);
      expect(decoded.length).toBe(largeContent.length);
      expect(decoded).toBe(largeContent);
    });
  });

  describe("Session Creation - Authentication", () => {
    it("should reject unauthenticated requests", async () => {
      const response = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          encrypted_document: btoa("test"),
          metadata: {
            filename: "test.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Test",
          },
          recipients: [],
          fields: [],
        }),
      });

      expect(response.status).toBe(401);
    });

    it("should reject requests with invalid token", async () => {
      const response = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: "Bearer invalid-token",
        },
        body: JSON.stringify({
          encrypted_document: btoa("test"),
          metadata: {
            filename: "test.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Test",
          },
          recipients: [],
          fields: [],
        }),
      });

      expect(response.status).toBe(401);
    });
  });
});
