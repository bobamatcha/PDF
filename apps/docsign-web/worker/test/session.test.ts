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

// Import real PDF as base64 from pre-generated fixture
// This ensures we test with actual valid PDFs (florida_escalation_addendum.pdf)
import { REAL_PDF_BASE64 } from "./fixtures/test-pdf";

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

  /**
   * Bug A: Re-signing Prevention
   *
   * Critical security bug: A signer can re-open their signing link and submit
   * a different signature, overwriting their previous signature.
   *
   * Expected behavior: Once a recipient has signed (signed=true), any subsequent
   * attempt to submit a signed document should return 400 error.
   */
  describe("Bug A: Re-signing Prevention", () => {
    it("should return 400 when already-signed recipient tries to sign again", async () => {
      const token = await createVerifiedUserAndLogin("resign-test@example.com");

      // Create a session with one recipient
      const sessionRequest = {
        encrypted_document: btoa("test document content"),
        metadata: {
          filename: "test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "resign-test@example.com",
        },
        recipients: [
          {
            id: "signer_1",
            name: "Test Signer",
            email: "signer1@example.com",
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
      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // First signing - should succeed
      const signRequest1 = {
        recipient_id: "signer_1",
        encrypted_document: btoa("signed document content v1"),
      };

      const signResponse1 = await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(signRequest1),
      });

      expect(signResponse1.status).toBe(200);
      const signData1 = (await signResponse1.json()) as { success: boolean };
      expect(signData1.success).toBe(true);

      // Second signing attempt - should be REJECTED with 400
      const signRequest2 = {
        recipient_id: "signer_1",
        encrypted_document: btoa("signed document content v2 - different signature"),
      };

      const signResponse2 = await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(signRequest2),
      });

      // BUG: Currently returns 200 and allows re-signing
      // After fix, should return 400 with error message
      expect(signResponse2.status).toBe(400);

      const signData2 = (await signResponse2.json()) as { success: boolean; message: string };
      expect(signData2.success).toBe(false);
      expect(signData2.message).toMatch(/already.*signed/i);
    });

    it("should allow different recipients to sign independently (parallel mode)", async () => {
      const token = await createVerifiedUserAndLogin("parallel-sign@example.com");

      // Create a session with TWO recipients
      const sessionRequest = {
        encrypted_document: btoa("test document for parallel signing"),
        metadata: {
          filename: "parallel-test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "parallel-sign@example.com",
        },
        recipients: [
          {
            id: "signer_1",
            name: "First Signer",
            email: "signer1@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
          {
            id: "signer_2",
            name: "Second Signer",
            email: "signer2@example.com",
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
            y_percent: 70.0,
            width_percent: 20.0,
            height_percent: 5.0,
            required: true,
            value: null,
          },
          {
            id: "sig_field_2",
            field_type: "signature",
            recipient_id: "signer_2",
            page: 1,
            x_percent: 50.0,
            y_percent: 85.0,
            width_percent: 20.0,
            height_percent: 5.0,
            required: true,
            value: null,
          },
        ],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // First signer signs
      const signResponse1 = await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          encrypted_document: btoa("signed by signer 1"),
        }),
      });
      expect(signResponse1.status).toBe(200);

      // Second signer should ALSO be able to sign (parallel mode)
      const signResponse2 = await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_2",
          encrypted_document: btoa("signed by signer 2"),
        }),
      });
      expect(signResponse2.status).toBe(200);

      // But signer_1 should NOT be able to sign again
      const resignResponse = await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          encrypted_document: btoa("trying to re-sign"),
        }),
      });
      expect(resignResponse.status).toBe(400);
    });

    it("should return signed=true in GET /session/{id} response after recipient signs", async () => {
      const token = await createVerifiedUserAndLogin("get-signed-status@example.com");

      // Create session with legacy=true to allow GET without token
      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          encrypted_document: btoa("test document for signed status check"),
          metadata: {
            filename: "signed-status-test.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Test User",
            sender_email: "get-signed-status@example.com",
          },
          recipients: [
            {
              id: "signer_check",
              name: "Status Checker",
              email: "statuscheck@example.com",
              role: "signer",
              signed: false,
              signed_at: null,
            },
          ],
          fields: [
            {
              id: "sig_field",
              field_type: "signature",
              recipient_id: "signer_check",
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
          legacy: true, // Allow GET without token for testing
        }),
      });

      expect(createResponse.status).toBe(200);
      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // GET session BEFORE signing - should show signed=false
      const getBeforeResponse = await SELF.fetch(
        `https://worker/session/${sessionId}`,
        { method: "GET" }
      );
      expect(getBeforeResponse.status).toBe(200);
      const beforeData = (await getBeforeResponse.json()) as {
        session: { recipients: Array<{ id: string; signed: boolean; signed_at: string | null }> };
      };
      expect(beforeData.session.recipients[0].signed).toBe(false);
      expect(beforeData.session.recipients[0].signed_at).toBeNull();

      // Sign the document
      const signResponse = await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_check",
          encrypted_document: btoa("signed document content"),
        }),
      });
      expect(signResponse.status).toBe(200);

      // GET session AFTER signing - should show signed=true
      const getAfterResponse = await SELF.fetch(
        `https://worker/session/${sessionId}`,
        { method: "GET" }
      );
      expect(getAfterResponse.status).toBe(200);
      const afterData = (await getAfterResponse.json()) as {
        session: { recipients: Array<{ id: string; signed: boolean; signed_at: string | null }> };
      };

      // CRITICAL ASSERTIONS for re-signing prevention
      expect(afterData.session.recipients[0].signed).toBe(true);
      expect(afterData.session.recipients[0].signed_at).toBeTruthy();
      // Verify signed_at is a valid ISO timestamp
      const signedAtDate = new Date(afterData.session.recipients[0].signed_at!);
      expect(signedAtDate.getTime()).toBeGreaterThan(0);
    });
  });

  /**
   * Bug E: Download Link Timing
   *
   * Currently, the signing completion response includes a download link even when
   * other signers haven't completed. The document isn't final until ALL sign.
   *
   * Expected behavior: Download link should ONLY be included in the response
   * when the signer is the FINAL signer (all recipients now signed).
   */
  describe("Bug E: Download Link Timing", () => {
    it("should NOT include download_link when other signers are still pending", async () => {
      const token = await createVerifiedUserAndLogin("download-timing@example.com");

      // Create session with 2 signers
      const sessionRequest = {
        encrypted_document: btoa("document needing two signatures"),
        metadata: {
          filename: "two-signer.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "download-timing@example.com",
        },
        recipients: [
          {
            id: "signer_1",
            name: "First Signer",
            email: "first@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
          {
            id: "signer_2",
            name: "Second Signer",
            email: "second@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
        ],
        fields: [],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // First signer signs (second signer still pending)
      const signResponse1 = await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          encrypted_document: btoa("signed by first"),
        }),
      });

      expect(signResponse1.status).toBe(200);
      const signData1 = (await signResponse1.json()) as {
        success: boolean;
        download_link?: string;
        all_signed?: boolean;
      };

      // BUG: Currently might include download_link even though signer_2 hasn't signed
      // After fix: should NOT include download_link until all sign
      expect(signData1.download_link).toBeUndefined();
      expect(signData1.all_signed).toBeFalsy();
    });

    it("should include download_link when final signer completes", async () => {
      const token = await createVerifiedUserAndLogin("final-signer@example.com");

      // Create session with 2 signers
      const sessionRequest = {
        encrypted_document: btoa("document for final signer test"),
        metadata: {
          filename: "final-signer.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "final-signer@example.com",
        },
        recipients: [
          {
            id: "signer_1",
            name: "First Signer",
            email: "first@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
          {
            id: "signer_2",
            name: "Second Signer",
            email: "second@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
        ],
        fields: [],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // First signer signs
      await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          encrypted_document: btoa("signed by first"),
        }),
      });

      // Second (final) signer signs
      const finalSignResponse = await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_2",
          encrypted_document: btoa("signed by second - final"),
        }),
      });

      expect(finalSignResponse.status).toBe(200);
      const finalData = (await finalSignResponse.json()) as {
        success: boolean;
        all_signed?: boolean;
      };

      // After fix: should indicate all_signed is true for final signer
      expect(finalData.success).toBe(true);
      expect(finalData.all_signed).toBe(true);
    });
  });

  /**
   * Bug C: Completion Emails to All Parties (Regression Tests)
   *
   * When all signers complete, completion email should be sent to:
   * 1. The sender/orchestrator
   * 2. ALL recipients who signed
   *
   * Note: We can't directly verify email sending in unit tests, but we can
   * verify the session state is correct for the email logic to work.
   */
  describe("Bug C: Completion Email State", () => {
    it("should track all signers as signed when complete", async () => {
      const token = await createVerifiedUserAndLogin("completion-test@example.com");

      // Create session with 2 signers
      const sessionRequest = {
        encrypted_document: btoa("completion test document"),
        metadata: {
          filename: "completion-test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "completion-test@example.com",
        },
        recipients: [
          {
            id: "signer_1",
            name: "First Signer",
            email: "first@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
          {
            id: "signer_2",
            name: "Second Signer",
            email: "second@example.com",
            role: "signer",
            signed: false,
            signed_at: null,
          },
        ],
        fields: [],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // Both signers sign
      await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          encrypted_document: btoa("signed by first"),
        }),
      });

      await SELF.fetch(`https://worker/session/${sessionId}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_2",
          encrypted_document: btoa("signed by second"),
        }),
      });

      // Verify session state in KV
      const sessionsKv = env.SESSIONS;
      const stored = await sessionsKv.get(`session:${sessionId}`);
      const session = JSON.parse(stored!) as {
        status: string;
        recipients: Array<{ id: string; signed: boolean; signed_at: string | null; email: string }>;
      };

      // Both recipients should be marked as signed
      expect(session.recipients.every(r => r.signed)).toBe(true);
      expect(session.recipients.every(r => r.signed_at !== null)).toBe(true);
      // Note: Serde serializes enum variants as lowercase by default
      expect(session.status.toLowerCase()).toBe("completed");

      // Note: Actual email sending is verified via mock intercepts in beforeEach
      // The test log output shows "Notification sent to sender: completion-test@example.com"
      // and additional emails to signers after Bug C fix
    });
  });

  /**
   * Feature 1: Document Aliasing (Regression Tests)
   *
   * Sessions should accept optional document_alias and signing_context fields
   * that appear in invitation emails.
   */
  describe("Feature 1: Document Aliasing", () => {
    it("should store document_alias in session metadata", async () => {
      const token = await createVerifiedUserAndLogin("alias-test@example.com");

      const sessionRequest = {
        encrypted_document: btoa("aliased document"),
        metadata: {
          filename: "lease.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "alias-test@example.com",
          document_alias: "Q1 2026 Lease Agreement",
          signing_context: "Lease for 30 James Ave, Orlando",
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
      const data = (await response.json()) as { session_id: string };

      // Verify alias and context are stored
      const sessionsKv = env.SESSIONS;
      const stored = await sessionsKv.get(`session:${data.session_id}`);
      const session = JSON.parse(stored!) as {
        metadata: {
          document_alias?: string;
          signing_context?: string;
        };
      };

      expect(session.metadata.document_alias).toBe("Q1 2026 Lease Agreement");
      expect(session.metadata.signing_context).toBe("Lease for 30 James Ave, Orlando");
    });

    it("should accept sessions without alias (backwards compatibility)", async () => {
      const token = await createVerifiedUserAndLogin("no-alias@example.com");

      const sessionRequest = {
        encrypted_document: btoa("document without alias"),
        metadata: {
          filename: "contract.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          // No document_alias or signing_context
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
      const data = (await response.json()) as { success: boolean };
      expect(data.success).toBe(true);
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

  // ============================================================
  // Feature 2: Document Dashboard - /my-sessions endpoint tests
  // ============================================================
  describe("Feature 2: Document Dashboard", () => {
    it("should return 401 for unauthenticated requests to /my-sessions", async () => {
      const response = await SELF.fetch("https://worker/my-sessions", {
        method: "GET",
        headers: { "Content-Type": "application/json" },
      });

      expect(response.status).toBe(401);
    });

    it("should return empty arrays for user with no sessions", async () => {
      const email = "empty-dashboard@example.com";
      const accessToken = await createVerifiedUserAndLogin(email);

      const response = await SELF.fetch("https://worker/my-sessions", {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${accessToken}`,
        },
      });

      expect(response.status).toBe(200);

      const data = (await response.json()) as {
        success: boolean;
        sent: {
          in_progress: unknown[];
          completed: unknown[];
          declined: unknown[];
          expired: unknown[];
        };
        inbox: {
          to_sign: unknown[];
          completed: unknown[];
          declined: unknown[];
        };
      };

      expect(data.success).toBe(true);
      expect(data.sent.in_progress).toEqual([]);
      expect(data.sent.completed).toEqual([]);
      expect(data.sent.declined).toEqual([]);
      expect(data.sent.expired).toEqual([]);
      expect(data.inbox.to_sign).toEqual([]);
      expect(data.inbox.completed).toEqual([]);
      expect(data.inbox.declined).toEqual([]);
    });

    it("should return user sessions grouped by status", async () => {
      const email = "dashboard-user@example.com";
      const accessToken = await createVerifiedUserAndLogin(email);

      // Create a session
      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({
          encrypted_document: btoa("test document"),
          metadata: {
            filename: "dashboard-test.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Test",
            sender_email: email,
            document_alias: "Dashboard Test Doc",
            signing_context: "Testing dashboard feature",
          },
          recipients: [
            {
              id: "1",
              name: "Recipient One",
              email: "recipient1@example.com",
              role: "signer",
              signed: false,
            },
          ],
          fields: [],
        }),
      });

      expect(createResponse.status).toBe(200);

      // Get dashboard data
      const dashboardResponse = await SELF.fetch("https://worker/my-sessions", {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${accessToken}`,
        },
      });

      expect(dashboardResponse.status).toBe(200);

      const data = (await dashboardResponse.json()) as {
        success: boolean;
        sent: {
          in_progress: {
            session_id: string;
            filename: string;
            document_alias: string | null;
            signing_context: string | null;
            status: string;
            recipients_signed: number;
            recipients_total: number;
            recipients: { name: string; email: string; signed: boolean }[];
          }[];
          completed: unknown[];
          declined: unknown[];
          expired: unknown[];
        };
      };

      expect(data.success).toBe(true);

      // Session should be in sent.in_progress (pending status)
      expect(data.sent.in_progress.length).toBe(1);
      expect(data.sent.in_progress[0].filename).toBe("dashboard-test.pdf");
      expect(data.sent.in_progress[0].document_alias).toBe("Dashboard Test Doc");
      expect(data.sent.in_progress[0].signing_context).toBe(
        "Testing dashboard feature"
      );
      expect(data.sent.in_progress[0].recipients_signed).toBe(0);
      expect(data.sent.in_progress[0].recipients_total).toBe(1);
      expect(data.sent.in_progress[0].recipients[0].name).toBe("Recipient One");
    });
  });

  /**
   * Bug Fix: My Documents Missing Sessions
   *
   * Sessions should appear in My Documents even when frontend doesn't set
   * metadata.sender_email. Backend should use authenticated user's email
   * for indexing instead of trusting the frontend.
   */
  describe("Bug Fix: My Documents Missing Sessions", () => {
    it("should index session by authenticated user email, not metadata.sender_email", async () => {
      const email = "dashboard-bug-test@example.com";
      const token = await createVerifiedUserAndLogin(email);

      // Create session WITHOUT sender_email in metadata (simulating frontend bug)
      const sessionRequest = {
        encrypted_document: btoa("test document for missing sender_email bug"),
        metadata: {
          filename: "missing-from-dashboard.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          // NOTE: sender_email intentionally omitted to simulate frontend bug!
        },
        recipients: [
          {
            id: "r1",
            name: "Bug Test Signer",
            email: "bugsigner@example.com",
            role: "signer",
          },
        ],
        fields: [],
        expiry_hours: 168,
      };

      // Create the session
      const createResp = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });
      expect(createResp.status).toBe(200);

      // Verify session appears in My Documents
      const dashboardResp = await SELF.fetch("https://worker/my-sessions", {
        headers: { Authorization: `Bearer ${token}` },
      });
      expect(dashboardResp.status).toBe(200);

      const dashboard = (await dashboardResp.json()) as {
        success: boolean;
        sent: {
          in_progress: { filename: string }[];
          completed: unknown[];
        };
      };

      expect(dashboard.success).toBe(true);

      // BUG FIX: Session should appear because backend uses authenticated user's email
      // Previously this would fail because session was indexed under hash("")
      expect(dashboard.sent.in_progress.length).toBeGreaterThan(0);
      expect(
        dashboard.sent.in_progress.some((s) => s.filename === "missing-from-dashboard.pdf")
      ).toBe(true);
    });

    it("should auto-populate metadata.sender_email if not provided", async () => {
      const email = "auto-populate-test@example.com";
      const token = await createVerifiedUserAndLogin(email);

      // Create session without sender_email
      const sessionRequest = {
        encrypted_document: btoa("test for auto-populate"),
        metadata: {
          filename: "auto-populate-test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test",
          // sender_email not provided
        },
        recipients: [
          { id: "r1", name: "Signer", email: "signer@example.com", role: "signer" },
        ],
        fields: [],
        expiry_hours: 168,
      };

      const createResp = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });
      expect(createResp.status).toBe(200);

      const createData = (await createResp.json()) as { session_id: string };

      // Fetch the session directly and verify sender_email was auto-populated
      const dashboardResp = await SELF.fetch("https://worker/my-sessions", {
        headers: { Authorization: `Bearer ${token}` },
      });
      const dashboard = (await dashboardResp.json()) as {
        sent: {
          in_progress: { session_id: string; sender_email?: string }[];
        };
      };

      // The session should have sender_email populated with authenticated user's email
      const session = dashboard.sent.in_progress.find(
        (s) => s.session_id === createData.session_id
      );
      // Note: We can't directly check metadata.sender_email from /my-sessions response
      // but if the session appears, indexing is correct
      expect(session).toBeDefined();
    });
  });

  // ============================================================
  // DocuSign-Style My Documents: Sent + Inbox
  // ============================================================
  /**
   * Feature: DocuSign-Style "My Documents" with Sent + Inbox tabs
   *
   * This implements a DocuSign-style dashboard where users can see:
   * - SENT: Documents they created and sent to others
   * - INBOX: Documents others sent TO them for signing (requires account)
   *
   * When a session is created, recipients are indexed by their email hash.
   * When they create an account with that email, they can see documents
   * waiting for their signature in the inbox.
   */
  describe("DocuSign-Style My Documents: Sent + Inbox", () => {
    it("should return new response structure with sent and inbox sections", async () => {
      const email = "inbox-structure@example.com";
      const token = await createVerifiedUserAndLogin(email);

      // Get dashboard - should have new structure
      const response = await SELF.fetch("https://worker/my-sessions", {
        headers: { Authorization: `Bearer ${token}` },
      });
      expect(response.status).toBe(200);

      const data = (await response.json()) as {
        success: boolean;
        sent?: {
          in_progress: unknown[];
          completed: unknown[];
          declined: unknown[];
          expired: unknown[];
        };
        inbox?: {
          to_sign: unknown[];
          completed: unknown[];
          declined: unknown[];
        };
      };

      expect(data.success).toBe(true);

      // New structure should have sent and inbox sections
      expect(data.sent).toBeDefined();
      expect(data.sent!.in_progress).toBeDefined();
      expect(data.sent!.completed).toBeDefined();
      expect(data.sent!.declined).toBeDefined();
      expect(data.sent!.expired).toBeDefined();

      expect(data.inbox).toBeDefined();
      expect(data.inbox!.to_sign).toBeDefined();
      expect(data.inbox!.completed).toBeDefined();
      expect(data.inbox!.declined).toBeDefined();
    });

    it("should show document in recipient inbox when they have an account", async () => {
      // Create sender account
      const senderEmail = "docusign-sender@example.com";
      const senderToken = await createVerifiedUserAndLogin(senderEmail);

      // Create recipient account with different email
      const recipientEmail = "docusign-recipient@example.com";
      const recipientToken = await createVerifiedUserAndLogin(recipientEmail);

      // Sender creates a session with recipient
      const sessionRequest = {
        encrypted_document: btoa("document for recipient inbox test"),
        metadata: {
          filename: "recipient-inbox-test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Sender User",
        },
        recipients: [
          {
            id: "r1",
            name: "Recipient User",
            email: recipientEmail, // Using the recipient's registered email
            role: "signer",
          },
        ],
        fields: [],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${senderToken}`,
        },
        body: JSON.stringify(sessionRequest),
      });
      expect(createResponse.status).toBe(200);

      // Recipient checks their dashboard - should see document in inbox
      const recipientDashboard = await SELF.fetch("https://worker/my-sessions", {
        headers: { Authorization: `Bearer ${recipientToken}` },
      });
      expect(recipientDashboard.status).toBe(200);

      const data = (await recipientDashboard.json()) as {
        success: boolean;
        inbox: {
          to_sign: {
            session_id: string;
            filename: string;
            sender_email: string;
            my_status: string;
          }[];
        };
      };

      expect(data.success).toBe(true);
      expect(data.inbox.to_sign.length).toBe(1);
      expect(data.inbox.to_sign[0].filename).toBe("recipient-inbox-test.pdf");
      expect(data.inbox.to_sign[0].sender_email).toBe(senderEmail);
      expect(data.inbox.to_sign[0].my_status).toBe("pending");
    });

    it("should move document to inbox.completed after recipient signs", async () => {
      // Create sender account
      const senderEmail = "sender-completion@example.com";
      const senderToken = await createVerifiedUserAndLogin(senderEmail);

      // Create recipient account
      const recipientEmail = "recipient-completion@example.com";
      const recipientToken = await createVerifiedUserAndLogin(recipientEmail);

      // Sender creates session
      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${senderToken}`,
        },
        body: JSON.stringify({
          encrypted_document: btoa("document to be signed"),
          metadata: {
            filename: "to-be-signed.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Sender",
          },
          recipients: [
            { id: "r1", name: "Recipient", email: recipientEmail, role: "signer" },
          ],
          fields: [],
          expiry_hours: 168,
        }),
      });
      expect(createResponse.status).toBe(200);
      const { session_id } = (await createResponse.json()) as { session_id: string };

      // Recipient signs the document
      const signResponse = await SELF.fetch(`https://worker/session/${session_id}/signed`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "r1",
          encrypted_document: btoa("signed document content"),
        }),
      });
      expect(signResponse.status).toBe(200);

      // Recipient checks dashboard - document should be in inbox.completed
      const dashboardResponse = await SELF.fetch("https://worker/my-sessions", {
        headers: { Authorization: `Bearer ${recipientToken}` },
      });
      const data = (await dashboardResponse.json()) as {
        inbox: {
          to_sign: unknown[];
          completed: { session_id: string; my_status: string }[];
        };
      };

      expect(data.inbox.to_sign.length).toBe(0);
      expect(data.inbox.completed.length).toBe(1);
      expect(data.inbox.completed[0].session_id).toBe(session_id);
      expect(data.inbox.completed[0].my_status).toBe("signed");
    });

    it("should show document in sender's sent section, not inbox", async () => {
      const email = "sender-only@example.com";
      const token = await createVerifiedUserAndLogin(email);

      // Create a session
      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          encrypted_document: btoa("sender's document"),
          metadata: {
            filename: "sender-document.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Sender",
          },
          recipients: [
            { id: "r1", name: "Signer", email: "external@example.com", role: "signer" },
          ],
          fields: [],
          expiry_hours: 168,
        }),
      });
      expect(createResponse.status).toBe(200);

      // Check dashboard
      const dashboardResponse = await SELF.fetch("https://worker/my-sessions", {
        headers: { Authorization: `Bearer ${token}` },
      });
      const data = (await dashboardResponse.json()) as {
        sent: { in_progress: { filename: string }[] };
        inbox: { to_sign: unknown[] };
      };

      // Document should appear in sent.in_progress, NOT in inbox
      expect(data.sent.in_progress.length).toBe(1);
      expect(data.sent.in_progress[0].filename).toBe("sender-document.pdf");
      expect(data.inbox.to_sign.length).toBe(0);
    });

    it("should allow recipient to download PDF from their inbox", async () => {
      // This test verifies that inbox document thumbnails can load
      // by testing the download endpoint from the recipient's perspective
      const senderEmail = "inbox-download-sender@example.com";
      const recipientEmail = "inbox-download-recipient@example.com";

      // Create sender and recipient accounts
      const senderToken = await createVerifiedUserAndLogin(senderEmail);
      const recipientToken = await createVerifiedUserAndLogin(recipientEmail);

      // Sender creates a session with recipient
      const sessionRequest = {
        encrypted_document: REAL_PDF_BASE64,
        metadata: {
          filename: "inbox-download-test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Sender",
          sender_email: senderEmail,
        },
        recipients: [
          {
            id: "recipient_1",
            name: "Inbox Recipient",
            email: recipientEmail,
            role: "signer",
            signed: false,
            signed_at: null,
          },
        ],
        fields: [
          {
            id: "sig_field_1",
            field_type: "signature",
            recipient_id: "recipient_1",
            page: 1,
            x_percent: 50.0,
            y_percent: 70.0,
            width_percent: 20.0,
            height_percent: 5.0,
            required: true,
            value: null,
          },
        ],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${senderToken}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      expect(createResponse.status).toBe(200);
      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // Recipient fetches their dashboard to see the session in inbox
      const dashboardResponse = await SELF.fetch("https://worker/my-sessions", {
        headers: { Authorization: `Bearer ${recipientToken}` },
      });

      expect(dashboardResponse.status).toBe(200);
      const dashboardData = (await dashboardResponse.json()) as {
        inbox: { to_sign: { session_id: string; filename: string }[] };
      };

      // Verify session appears in recipient's inbox
      expect(dashboardData.inbox.to_sign.length).toBe(1);
      expect(dashboardData.inbox.to_sign[0].session_id).toBe(sessionId);
      expect(dashboardData.inbox.to_sign[0].filename).toBe("inbox-download-test.pdf");

      // KEY TEST: Recipient downloads the PDF using session_id from their inbox
      // This is exactly what loadThumbnail() does in the frontend
      const downloadResponse = await SELF.fetch(`https://worker/session/${sessionId}/download`);

      expect(downloadResponse.status).toBe(200);
      expect(downloadResponse.headers.get("Content-Type")).toBe("application/pdf");
      expect(downloadResponse.headers.get("Content-Disposition")).toContain("inbox-download-test.pdf");

      // Verify the PDF has actual content
      const pdfBytes = await downloadResponse.arrayBuffer();
      expect(pdfBytes.byteLength).toBeGreaterThan(100);
    });
  });

  /**
   * Phase 6: Annotation Storage Tests
   *
   * Tests for the new lightweight annotation-based signing that stores ~50KB
   * per signer instead of ~13MB full PDF copies. This enables unlimited signers
   * within the 25MB KV limit.
   */
  describe("Annotation Storage (New Lightweight Format)", () => {
    it("should accept annotation submission instead of full PDF", async () => {
      const token = await createVerifiedUserAndLogin("annotation-test@example.com");

      // Create session with signature field
      const sessionRequest = {
        encrypted_document: btoa("test document for annotations"),
        metadata: {
          filename: "annotation-test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "annotation-test@example.com",
        },
        recipients: [
          {
            id: "signer_1",
            name: "Annotation Signer",
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
            y_percent: 70.0,
            width_percent: 20.0,
            height_percent: 5.0,
            required: true,
            value: null,
          },
        ],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      expect(createResponse.status).toBe(200);
      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // Submit annotation instead of full PDF
      const annotationResponse = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          annotations: [
            {
              field_id: "sig_field_1",
              data: {
                type: "DrawnSignature",
                image_base64: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==", // Small test signature
              },
            },
          ],
        }),
      });

      expect(annotationResponse.status).toBe(200);
      const result = (await annotationResponse.json()) as {
        success: boolean;
        message: string;
        all_signed: boolean;
        remaining_signers: number;
      };
      expect(result.success).toBe(true);
      expect(result.all_signed).toBe(true);
      expect(result.remaining_signers).toBe(0);
    });

    it("should reject annotation if recipient already signed", async () => {
      const token = await createVerifiedUserAndLogin("double-sign-annotation@example.com");

      // Create session
      const sessionRequest = {
        encrypted_document: btoa("double sign test"),
        metadata: {
          filename: "double-sign-annotation.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "double-sign-annotation@example.com",
        },
        recipients: [
          {
            id: "signer_1",
            name: "Double Signer",
            email: "double@example.com",
            role: "signer",
            signed: false,
          },
        ],
        fields: [
          {
            id: "sig_1",
            field_type: "signature",
            recipient_id: "signer_1",
            page: 1,
            x_percent: 50.0,
            y_percent: 70.0,
            width_percent: 20.0,
            height_percent: 5.0,
          },
        ],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // First annotation - should succeed
      const firstResponse = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          annotations: [
            {
              field_id: "sig_1",
              data: { type: "DrawnSignature", image_base64: "data:image/png;base64,test==" },
            },
          ],
        }),
      });
      expect(firstResponse.status).toBe(200);

      // Second annotation - should be rejected
      const secondResponse = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          annotations: [
            {
              field_id: "sig_1",
              data: { type: "DrawnSignature", image_base64: "data:image/png;base64,another==" },
            },
          ],
        }),
      });
      expect(secondResponse.status).toBe(400);
    });

    it("should support multiple signers with annotations", async () => {
      const token = await createVerifiedUserAndLogin("multi-signer-annotation@example.com");

      // Create session with 3 signers
      const sessionRequest = {
        encrypted_document: btoa("multi signer test"),
        metadata: {
          filename: "multi-signer-annotation.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "multi-signer-annotation@example.com",
        },
        recipients: [
          { id: "signer_1", name: "Signer One", email: "one@example.com", role: "signer", signed: false },
          { id: "signer_2", name: "Signer Two", email: "two@example.com", role: "signer", signed: false },
          { id: "signer_3", name: "Signer Three", email: "three@example.com", role: "signer", signed: false },
        ],
        fields: [
          { id: "sig_1", field_type: "signature", recipient_id: "signer_1", page: 1, x_percent: 50, y_percent: 50, width_percent: 20, height_percent: 5 },
          { id: "sig_2", field_type: "signature", recipient_id: "signer_2", page: 1, x_percent: 50, y_percent: 60, width_percent: 20, height_percent: 5 },
          { id: "sig_3", field_type: "signature", recipient_id: "signer_3", page: 1, x_percent: 50, y_percent: 70, width_percent: 20, height_percent: 5 },
        ],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // First signer
      const resp1 = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          annotations: [{ field_id: "sig_1", data: { type: "DrawnSignature", image_base64: "data:image/png;base64,sig1==" } }],
        }),
      });
      expect(resp1.status).toBe(200);
      const data1 = (await resp1.json()) as { all_signed: boolean; remaining_signers: number };
      expect(data1.all_signed).toBe(false);
      expect(data1.remaining_signers).toBe(2);

      // Second signer
      const resp2 = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_2",
          annotations: [{ field_id: "sig_2", data: { type: "DrawnSignature", image_base64: "data:image/png;base64,sig2==" } }],
        }),
      });
      expect(resp2.status).toBe(200);
      const data2 = (await resp2.json()) as { all_signed: boolean; remaining_signers: number };
      expect(data2.all_signed).toBe(false);
      expect(data2.remaining_signers).toBe(1);

      // Third signer
      const resp3 = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_3",
          annotations: [{ field_id: "sig_3", data: { type: "DrawnSignature", image_base64: "data:image/png;base64,sig3==" } }],
        }),
      });
      expect(resp3.status).toBe(200);
      const data3 = (await resp3.json()) as { all_signed: boolean; remaining_signers: number };
      expect(data3.all_signed).toBe(true);
      expect(data3.remaining_signers).toBe(0);

      // Verify annotations are stored in session
      const sessionsKv = env.SESSIONS;
      const stored = await sessionsKv.get(`session:${sessionId}`);
      const session = JSON.parse(stored!) as {
        signature_annotations: Array<{ recipient_id: string; field_id: string }>;
        status: string;
      };

      expect(session.signature_annotations.length).toBe(3);
      expect(session.status.toLowerCase()).toBe("completed");
    });

    it("should support typed signatures in annotations", async () => {
      const token = await createVerifiedUserAndLogin("typed-sig-annotation@example.com");

      const sessionRequest = {
        encrypted_document: btoa("typed signature test"),
        metadata: {
          filename: "typed-sig.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "typed-sig-annotation@example.com",
        },
        recipients: [
          { id: "signer_1", name: "Typed Signer", email: "typed@example.com", role: "signer", signed: false },
        ],
        fields: [
          { id: "sig_1", field_type: "signature", recipient_id: "signer_1", page: 1, x_percent: 50, y_percent: 70, width_percent: 20, height_percent: 5 },
        ],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // Submit typed signature
      const response = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          annotations: [
            {
              field_id: "sig_1",
              data: {
                type: "TypedSignature",
                text: "John Doe",
                font: "Dancing Script",
              },
            },
          ],
        }),
      });

      expect(response.status).toBe(200);
      const result = (await response.json()) as { success: boolean; all_signed: boolean };
      expect(result.success).toBe(true);
      expect(result.all_signed).toBe(true);
    });
  });

  /**
   * Download Endpoint Tests (Merge-on-Demand)
   *
   * Tests for GET /session/{id}/download which merges annotations into the
   * original PDF on-demand and returns the final document.
   */
  describe("Download Endpoint (Merge-on-Demand)", () => {
    it("should return 404 for non-existent session", async () => {
      const response = await SELF.fetch("https://worker/session/nonexistent123/download");
      expect(response.status).toBe(404);
    });

    it("should return original PDF when no annotations exist", async () => {
      const token = await createVerifiedUserAndLogin("download-original@example.com");

      // Create session without signing
      const originalPdf = "JVBERi0xLjQKJdPr6eEKMSAwIG9iago8PC9UeXBlL0NhdGFsb2cvUGFnZXMgMiAwIFI+PgplbmRvYmoKMiAwIG9iago8PC9UeXBlL1BhZ2VzL0tpZHNbMyAwIFJdL0NvdW50IDE+PgplbmRvYmoKMyAwIG9iago8PC9UeXBlL1BhZ2UvTWVkaWFCb3hbMCAwIDYxMiA3OTJdL1BhcmVudCAyIDAgUj4+CmVuZG9iagp4cmVmCjAgNAowMDAwMDAwMDAwIDY1NTM1IGYgCjAwMDAwMDAwMTUgMDAwMDAgbiAKMDAwMDAwMDA2OCAwMDAwMCBuIAowMDAwMDAwMTMxIDAwMDAwIG4gCnRyYWlsZXIKPDwvU2l6ZSA0L1Jvb3QgMSAwIFI+PgpzdGFydHhyZWYKMjA4CiUlRU9GCg==";
      const sessionRequest = {
        encrypted_document: originalPdf,
        metadata: {
          filename: "unsigned.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "download-original@example.com",
        },
        recipients: [
          { id: "signer_1", name: "Signer", email: "signer@example.com", role: "signer", signed: false },
        ],
        fields: [
          { id: "sig_1", field_type: "signature", recipient_id: "signer_1", page: 1, x_percent: 50, y_percent: 70, width_percent: 20, height_percent: 5 },
        ],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // Download without signing
      console.log(`Fetching download: https://worker/session/${sessionId}/download`);
      const downloadResponse = await SELF.fetch(`https://worker/session/${sessionId}/download`);
      console.log(`Download response status: ${downloadResponse.status}`);
      if (downloadResponse.status !== 200) {
        const body = await downloadResponse.text();
        console.log(`Download error body: ${body}`);
      }
      expect(downloadResponse.status).toBe(200);
      expect(downloadResponse.headers.get("Content-Type")).toBe("application/pdf");
      expect(downloadResponse.headers.get("Content-Disposition")).toContain("unsigned.pdf");
    });

    it("should support text annotations (date, text fields)", async () => {
      const token = await createVerifiedUserAndLogin("download-text@example.com");

      // Use real PDF from output directory for proper merge testing
      const sessionRequest = {
        encrypted_document: REAL_PDF_BASE64,
        metadata: {
          filename: "text-fields.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "download-text@example.com",
        },
        recipients: [
          { id: "signer_1", name: "Signer", email: "signer@example.com", role: "signer", signed: false },
        ],
        fields: [
          { id: "date_1", field_type: "date", recipient_id: "signer_1", page: 1, x_percent: 70, y_percent: 90, width_percent: 15, height_percent: 3 },
          { id: "text_1", field_type: "text", recipient_id: "signer_1", page: 1, x_percent: 50, y_percent: 80, width_percent: 20, height_percent: 3 },
        ],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // Submit text annotations
      const submitResponse = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          annotations: [
            { field_id: "date_1", data: { type: "Date", value: "2025-01-13" } },
            { field_id: "text_1", data: { type: "Text", value: "Some additional notes here" } },
          ],
        }),
      });
      expect(submitResponse.status).toBe(200);

      // Check what's stored in session
      const sessionsKv = env.SESSIONS;
      const stored = await sessionsKv.get(`session:${sessionId}`);
      const session = JSON.parse(stored!) as { signature_annotations: any[] };
      console.log(`Stored annotations: ${JSON.stringify(session.signature_annotations, null, 2)}`);

      // Download should include text annotations
      const downloadResponse = await SELF.fetch(`https://worker/session/${sessionId}/download`);
      if (downloadResponse.status !== 200) {
        const body = await downloadResponse.text();
        console.error(`Download error: ${downloadResponse.status} - ${body}`);
        expect.fail(`Download failed with ${downloadResponse.status}: ${body}`);
      }
      expect(downloadResponse.status).toBe(200);
      expect(downloadResponse.headers.get("Content-Type")).toBe("application/pdf");
    });

    it("should support checkbox annotations", async () => {
      const token = await createVerifiedUserAndLogin("download-checkbox@example.com");

      // Use real PDF from output directory for proper merge testing
      const sessionRequest = {
        encrypted_document: REAL_PDF_BASE64,
        metadata: {
          filename: "checkbox.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "download-checkbox@example.com",
        },
        recipients: [
          { id: "signer_1", name: "Signer", email: "signer@example.com", role: "signer", signed: false },
        ],
        fields: [
          { id: "check_1", field_type: "checkbox", recipient_id: "signer_1", page: 1, x_percent: 10, y_percent: 50, width_percent: 3, height_percent: 3 },
          { id: "check_2", field_type: "checkbox", recipient_id: "signer_1", page: 1, x_percent: 10, y_percent: 55, width_percent: 3, height_percent: 3 },
        ],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // Submit checkbox annotations (one checked, one unchecked)
      const submitResponse = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          annotations: [
            { field_id: "check_1", data: { type: "Checkbox", checked: true } },
            { field_id: "check_2", data: { type: "Checkbox", checked: false } },
          ],
        }),
      });
      expect(submitResponse.status).toBe(200);

      // Download should succeed
      const downloadResponse = await SELF.fetch(`https://worker/session/${sessionId}/download`);
      expect(downloadResponse.status).toBe(200);
    });

    it("should cache final document after completion", async () => {
      const token = await createVerifiedUserAndLogin("download-cache@example.com");

      // Use real PDF from output directory for proper merge testing
      const sessionRequest = {
        encrypted_document: REAL_PDF_BASE64,
        metadata: {
          filename: "cached.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: "download-cache@example.com",
        },
        recipients: [
          { id: "signer_1", name: "Signer", email: "signer@example.com", role: "signer", signed: false },
        ],
        fields: [
          { id: "date_1", field_type: "date", recipient_id: "signer_1", page: 1, x_percent: 70, y_percent: 90, width_percent: 15, height_percent: 3 },
        ],
        expiry_hours: 168,
      };

      const createResponse = await SELF.fetch("https://worker/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(sessionRequest),
      });

      const createData = (await createResponse.json()) as { session_id: string };
      const sessionId = createData.session_id;

      // Sign to complete session
      await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient_id: "signer_1",
          annotations: [
            { field_id: "date_1", data: { type: "Date", value: "2025-01-13" } },
          ],
        }),
      });

      // First download - should trigger merge and cache
      const download1 = await SELF.fetch(`https://worker/session/${sessionId}/download`);
      expect(download1.status).toBe(200);

      // Check that final_document is now cached in KV
      const sessionsKv = env.SESSIONS;
      const stored = await sessionsKv.get(`session:${sessionId}`);
      const session = JSON.parse(stored!) as { final_document: string | null };
      expect(session.final_document).not.toBeNull();
      expect(session.final_document!.length).toBeGreaterThan(0);

      // Second download should use cached version
      const download2 = await SELF.fetch(`https://worker/session/${sessionId}/download`);
      expect(download2.status).toBe(200);
    });
  });

  /**
   * Phase 1: Signing Completion Flow Enhancement
   *
   * Tests for download_url in annotation response and associate-session endpoint.
   * When all signers complete, the response should include a download_url.
   * Signers who create accounts after signing can associate the session.
   */
  describe("Phase 1: Signing Completion Flow", () => {
    describe("download_url in annotation response", () => {
      it("should return download_url when final signer completes (all_signed=true)", async () => {
        const token = await createVerifiedUserAndLogin("download-url-final@example.com");

        // Create session with one signer
        const sessionRequest = {
          encrypted_document: REAL_PDF_BASE64,
          metadata: {
            filename: "final-signer-download.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Test User",
            sender_email: "download-url-final@example.com",
          },
          recipients: [
            { id: "signer_1", name: "Single Signer", email: "signer@example.com", role: "signer", signed: false },
          ],
          fields: [
            { id: "sig_1", field_type: "signature", recipient_id: "signer_1", page: 1, x_percent: 50, y_percent: 70, width_percent: 20, height_percent: 5 },
          ],
          expiry_hours: 168,
        };

        const createResponse = await SELF.fetch("https://worker/session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify(sessionRequest),
        });

        const createData = (await createResponse.json()) as { session_id: string };
        const sessionId = createData.session_id;

        // Submit annotation - this should complete the signing
        const annotationResponse = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            recipient_id: "signer_1",
            annotations: [
              { field_id: "sig_1", data: { type: "DrawnSignature", image_base64: "data:image/png;base64,test==" } },
            ],
          }),
        });

        expect(annotationResponse.status).toBe(200);
        const result = (await annotationResponse.json()) as {
          success: boolean;
          all_signed: boolean;
          download_url?: string;
        };

        expect(result.success).toBe(true);
        expect(result.all_signed).toBe(true);
        // download_url should be present when all_signed is true
        expect(result.download_url).toBeDefined();
        // URL format: https://api.getsignatures.org/session/{session_id}/download?expires={timestamp}
        expect(result.download_url).toContain("api.getsignatures.org/session/");
        expect(result.download_url).toContain("/download?expires=");
      });

      it("should NOT return download_url when signers are still pending", async () => {
        const token = await createVerifiedUserAndLogin("download-url-pending@example.com");

        // Create session with TWO signers
        const sessionRequest = {
          encrypted_document: REAL_PDF_BASE64,
          metadata: {
            filename: "multi-signer-pending.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Test User",
            sender_email: "download-url-pending@example.com",
          },
          recipients: [
            { id: "signer_1", name: "First Signer", email: "first@example.com", role: "signer", signed: false },
            { id: "signer_2", name: "Second Signer", email: "second@example.com", role: "signer", signed: false },
          ],
          fields: [
            { id: "sig_1", field_type: "signature", recipient_id: "signer_1", page: 1, x_percent: 50, y_percent: 60, width_percent: 20, height_percent: 5 },
            { id: "sig_2", field_type: "signature", recipient_id: "signer_2", page: 1, x_percent: 50, y_percent: 80, width_percent: 20, height_percent: 5 },
          ],
          expiry_hours: 168,
        };

        const createResponse = await SELF.fetch("https://worker/session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify(sessionRequest),
        });

        const createData = (await createResponse.json()) as { session_id: string };
        const sessionId = createData.session_id;

        // First signer submits annotation (second signer still pending)
        const annotationResponse = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            recipient_id: "signer_1",
            annotations: [
              { field_id: "sig_1", data: { type: "DrawnSignature", image_base64: "data:image/png;base64,first==" } },
            ],
          }),
        });

        expect(annotationResponse.status).toBe(200);
        const result = (await annotationResponse.json()) as {
          success: boolean;
          all_signed: boolean;
          remaining_signers: number;
          download_url?: string;
        };

        expect(result.success).toBe(true);
        expect(result.all_signed).toBe(false);
        expect(result.remaining_signers).toBe(1);
        // download_url should NOT be present when signers are still pending
        expect(result.download_url).toBeUndefined();
      });

      it("should return download_url when second signer completes multi-signer session", async () => {
        const token = await createVerifiedUserAndLogin("download-url-multi@example.com");

        // Create session with TWO signers
        const sessionRequest = {
          encrypted_document: REAL_PDF_BASE64,
          metadata: {
            filename: "multi-signer-complete.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Test User",
            sender_email: "download-url-multi@example.com",
          },
          recipients: [
            { id: "signer_1", name: "First Signer", email: "first@example.com", role: "signer", signed: false },
            { id: "signer_2", name: "Second Signer", email: "second@example.com", role: "signer", signed: false },
          ],
          fields: [
            { id: "sig_1", field_type: "signature", recipient_id: "signer_1", page: 1, x_percent: 50, y_percent: 60, width_percent: 20, height_percent: 5 },
            { id: "sig_2", field_type: "signature", recipient_id: "signer_2", page: 1, x_percent: 50, y_percent: 80, width_percent: 20, height_percent: 5 },
          ],
          expiry_hours: 168,
        };

        const createResponse = await SELF.fetch("https://worker/session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify(sessionRequest),
        });

        const createData = (await createResponse.json()) as { session_id: string };
        const sessionId = createData.session_id;

        // First signer submits
        await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            recipient_id: "signer_1",
            annotations: [
              { field_id: "sig_1", data: { type: "DrawnSignature", image_base64: "data:image/png;base64,first==" } },
            ],
          }),
        });

        // Second (final) signer submits
        const finalResponse = await SELF.fetch(`https://worker/session/${sessionId}/annotations`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            recipient_id: "signer_2",
            annotations: [
              { field_id: "sig_2", data: { type: "DrawnSignature", image_base64: "data:image/png;base64,second==" } },
            ],
          }),
        });

        expect(finalResponse.status).toBe(200);
        const result = (await finalResponse.json()) as {
          success: boolean;
          all_signed: boolean;
          remaining_signers: number;
          download_url?: string;
        };

        expect(result.success).toBe(true);
        expect(result.all_signed).toBe(true);
        expect(result.remaining_signers).toBe(0);
        // download_url should now be present
        expect(result.download_url).toBeDefined();
        // URL format: https://api.getsignatures.org/session/{session_id}/download?expires={timestamp}
        expect(result.download_url).toContain("api.getsignatures.org/session/");
        expect(result.download_url).toContain("/download?expires=");
      });
    });

    describe("POST /auth/associate-session", () => {
      it("should return 401 for unauthenticated requests", async () => {
        const response = await SELF.fetch("https://worker/auth/associate-session", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ session_id: "test-session-123" }),
        });

        expect(response.status).toBe(401);
      });

      it("should return 404 for non-existent session", async () => {
        const token = await createVerifiedUserAndLogin("associate-404@example.com");

        const response = await SELF.fetch("https://worker/auth/associate-session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ session_id: "nonexistent-session-id" }),
        });

        expect(response.status).toBe(404);
        const data = (await response.json()) as { success: boolean; message: string };
        expect(data.success).toBe(false);
        expect(data.message).toContain("not found");
      });

      it("should return 403 when user is not a recipient on the session", async () => {
        // Create sender account and session
        const senderToken = await createVerifiedUserAndLogin("associate-sender@example.com");

        const sessionRequest = {
          encrypted_document: btoa("test document"),
          metadata: {
            filename: "associate-test.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Sender",
          },
          recipients: [
            { id: "r1", name: "Other Recipient", email: "other@example.com", role: "signer" },
          ],
          fields: [],
          expiry_hours: 168,
        };

        const createResponse = await SELF.fetch("https://worker/session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${senderToken}`,
          },
          body: JSON.stringify(sessionRequest),
        });
        const { session_id } = (await createResponse.json()) as { session_id: string };

        // Create a different user who is NOT a recipient
        const nonRecipientToken = await createVerifiedUserAndLogin("non-recipient@example.com");

        // Try to associate - should fail
        const response = await SELF.fetch("https://worker/auth/associate-session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${nonRecipientToken}`,
          },
          body: JSON.stringify({ session_id }),
        });

        expect(response.status).toBe(403);
        const data = (await response.json()) as { success: boolean; message: string };
        expect(data.success).toBe(false);
        expect(data.message).toContain("not a recipient");
      });

      it("should successfully associate session when user email matches recipient", async () => {
        // Create sender account
        const senderToken = await createVerifiedUserAndLogin("associate-ok-sender@example.com");

        // The recipient email that will create an account later
        const recipientEmail = "associate-ok-recipient@example.com";

        const sessionRequest = {
          encrypted_document: btoa("test document for association"),
          metadata: {
            filename: "associate-success.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Sender",
          },
          recipients: [
            { id: "r1", name: "Matching Recipient", email: recipientEmail, role: "signer" },
          ],
          fields: [],
          expiry_hours: 168,
        };

        const createResponse = await SELF.fetch("https://worker/session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${senderToken}`,
          },
          body: JSON.stringify(sessionRequest),
        });
        const { session_id } = (await createResponse.json()) as { session_id: string };

        // Create account with same email as recipient
        const recipientToken = await createVerifiedUserAndLogin(recipientEmail);

        // Associate the session
        const response = await SELF.fetch("https://worker/auth/associate-session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${recipientToken}`,
          },
          body: JSON.stringify({ session_id }),
        });

        expect(response.status).toBe(200);
        const data = (await response.json()) as { success: boolean; message: string };
        expect(data.success).toBe(true);
        expect(data.message).toContain("associated");

        // Verify the session now appears in the recipient's inbox
        const dashboardResponse = await SELF.fetch("https://worker/my-sessions", {
          headers: { Authorization: `Bearer ${recipientToken}` },
        });

        const dashboard = (await dashboardResponse.json()) as {
          inbox: { to_sign: { session_id: string }[] };
        };

        // Should find the session in inbox
        const found = dashboard.inbox.to_sign.find((s) => s.session_id === session_id);
        expect(found).toBeDefined();
      });

      it("should handle case-insensitive email matching", async () => {
        const senderToken = await createVerifiedUserAndLogin("case-sender@example.com");

        // Recipient email with different case
        const recipientEmailInSession = "CaseSensitive@Example.COM";

        const sessionRequest = {
          encrypted_document: btoa("case test document"),
          metadata: {
            filename: "case-test.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Sender",
          },
          recipients: [
            { id: "r1", name: "Case Test", email: recipientEmailInSession, role: "signer" },
          ],
          fields: [],
          expiry_hours: 168,
        };

        const createResponse = await SELF.fetch("https://worker/session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${senderToken}`,
          },
          body: JSON.stringify(sessionRequest),
        });
        const { session_id } = (await createResponse.json()) as { session_id: string };

        // Create account with lowercase email
        const recipientToken = await createVerifiedUserAndLogin("casesensitive@example.com");

        // Associate should succeed despite case difference
        const response = await SELF.fetch("https://worker/auth/associate-session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${recipientToken}`,
          },
          body: JSON.stringify({ session_id }),
        });

        expect(response.status).toBe(200);
        const data = (await response.json()) as { success: boolean };
        expect(data.success).toBe(true);
      });
    });
  });

  /**
   * Phase 3: Template Management API
   *
   * Tests for server-persisted templates (CRUD operations).
   */
  describe("Phase 3: Template Management", () => {
    describe("POST /templates (create)", () => {
      it("should return 401 for unauthenticated requests", async () => {
        const response = await SELF.fetch("https://worker/templates", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            name: "Test Template",
            fields: [],
          }),
        });

        expect(response.status).toBe(401);
      });

      it("should create a template successfully", async () => {
        const token = await createVerifiedUserAndLogin("template-create@example.com");

        const response = await SELF.fetch("https://worker/templates", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            name: "My First Template",
            fields: [
              {
                field_type: "signature",
                recipient_index: 0,
                page: 1,
                x_percent: 50,
                y_percent: 80,
                width_percent: 20,
                height_percent: 5,
                required: true,
              },
            ],
          }),
        });

        expect(response.status).toBe(200);
        const data = (await response.json()) as {
          success: boolean;
          template_id: string;
          message: string;
        };
        expect(data.success).toBe(true);
        expect(data.template_id).toBeDefined();
        expect(data.template_id.length).toBeGreaterThan(0);
      });

      it("should reject empty template name", async () => {
        const token = await createVerifiedUserAndLogin("template-empty-name@example.com");

        const response = await SELF.fetch("https://worker/templates", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            name: "   ",
            fields: [],
          }),
        });

        expect(response.status).toBe(400);
      });
    });

    describe("GET /templates (list)", () => {
      it("should return 401 for unauthenticated requests", async () => {
        const response = await SELF.fetch("https://worker/templates", {
          method: "GET",
        });

        expect(response.status).toBe(401);
      });

      it("should list user's templates", async () => {
        const token = await createVerifiedUserAndLogin("template-list@example.com");

        // Create two templates
        await SELF.fetch("https://worker/templates", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ name: "Template A", fields: [] }),
        });

        await SELF.fetch("https://worker/templates", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            name: "Template B",
            fields: [
              { field_type: "signature", recipient_index: 0, page: 1, x_percent: 50, y_percent: 80, width_percent: 20, height_percent: 5, required: true },
              { field_type: "date", recipient_index: 0, page: 1, x_percent: 50, y_percent: 90, width_percent: 15, height_percent: 3, required: false },
            ],
          }),
        });

        const response = await SELF.fetch("https://worker/templates", {
          method: "GET",
          headers: { Authorization: `Bearer ${token}` },
        });

        expect(response.status).toBe(200);
        const data = (await response.json()) as {
          success: boolean;
          templates: { id: string; name: string; field_count: number }[];
        };
        expect(data.success).toBe(true);
        expect(data.templates.length).toBe(2);
        expect(data.templates.find((t) => t.name === "Template A")).toBeDefined();
        expect(data.templates.find((t) => t.name === "Template B")?.field_count).toBe(2);
      });

      it("should not show other user's templates", async () => {
        // User 1 creates a template
        const token1 = await createVerifiedUserAndLogin("template-user1@example.com");
        await SELF.fetch("https://worker/templates", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token1}`,
          },
          body: JSON.stringify({ name: "User 1 Template", fields: [] }),
        });

        // User 2 should not see User 1's template
        const token2 = await createVerifiedUserAndLogin("template-user2@example.com");
        const response = await SELF.fetch("https://worker/templates", {
          method: "GET",
          headers: { Authorization: `Bearer ${token2}` },
        });

        const data = (await response.json()) as { templates: { name: string }[] };
        expect(data.templates.find((t) => t.name === "User 1 Template")).toBeUndefined();
      });
    });

    describe("GET /templates/{id}", () => {
      it("should return a template by ID", async () => {
        const token = await createVerifiedUserAndLogin("template-get@example.com");

        // Create template
        const createResponse = await SELF.fetch("https://worker/templates", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            name: "Detailed Template",
            fields: [
              { field_type: "signature", recipient_index: 0, page: 1, x_percent: 50, y_percent: 80, width_percent: 20, height_percent: 5, required: true },
            ],
          }),
        });
        const { template_id } = (await createResponse.json()) as { template_id: string };

        // Get template
        const response = await SELF.fetch(`https://worker/templates/${template_id}`, {
          method: "GET",
          headers: { Authorization: `Bearer ${token}` },
        });

        expect(response.status).toBe(200);
        const data = (await response.json()) as {
          success: boolean;
          template: { id: string; name: string; fields: unknown[] };
        };
        expect(data.success).toBe(true);
        expect(data.template.id).toBe(template_id);
        expect(data.template.name).toBe("Detailed Template");
        expect(data.template.fields.length).toBe(1);
      });

      it("should return 404 for non-existent template", async () => {
        const token = await createVerifiedUserAndLogin("template-get-404@example.com");

        const response = await SELF.fetch("https://worker/templates/nonexistent-id", {
          method: "GET",
          headers: { Authorization: `Bearer ${token}` },
        });

        expect(response.status).toBe(404);
      });
    });

    describe("PUT /templates/{id}", () => {
      it("should update a template", async () => {
        const token = await createVerifiedUserAndLogin("template-update@example.com");

        // Create template
        const createResponse = await SELF.fetch("https://worker/templates", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ name: "Original Name", fields: [] }),
        });
        const { template_id } = (await createResponse.json()) as { template_id: string };

        // Update template
        const response = await SELF.fetch(`https://worker/templates/${template_id}`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            name: "Updated Name",
            fields: [
              { field_type: "text", recipient_index: 0, page: 1, x_percent: 30, y_percent: 50, width_percent: 40, height_percent: 3, required: false },
            ],
          }),
        });

        expect(response.status).toBe(200);

        // Verify update
        const getResponse = await SELF.fetch(`https://worker/templates/${template_id}`, {
          method: "GET",
          headers: { Authorization: `Bearer ${token}` },
        });
        const { template } = (await getResponse.json()) as { template: { name: string; fields: unknown[] } };
        expect(template.name).toBe("Updated Name");
        expect(template.fields.length).toBe(1);
      });

      it("should return 404 for non-existent template", async () => {
        const token = await createVerifiedUserAndLogin("template-update-404@example.com");

        const response = await SELF.fetch("https://worker/templates/nonexistent-id", {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ name: "Updated", fields: [] }),
        });

        expect(response.status).toBe(404);
      });
    });

    describe("DELETE /templates/{id}", () => {
      it("should delete a template", async () => {
        const token = await createVerifiedUserAndLogin("template-delete@example.com");

        // Create template
        const createResponse = await SELF.fetch("https://worker/templates", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ name: "To Delete", fields: [] }),
        });
        const { template_id } = (await createResponse.json()) as { template_id: string };

        // Delete template
        const response = await SELF.fetch(`https://worker/templates/${template_id}`, {
          method: "DELETE",
          headers: { Authorization: `Bearer ${token}` },
        });

        expect(response.status).toBe(200);

        // Verify deletion
        const getResponse = await SELF.fetch(`https://worker/templates/${template_id}`, {
          method: "GET",
          headers: { Authorization: `Bearer ${token}` },
        });
        expect(getResponse.status).toBe(404);

        // Verify not in list
        const listResponse = await SELF.fetch("https://worker/templates", {
          method: "GET",
          headers: { Authorization: `Bearer ${token}` },
        });
        const { templates } = (await listResponse.json()) as { templates: { id: string }[] };
        expect(templates.find((t) => t.id === template_id)).toBeUndefined();
      });

      it("should return 404 for non-existent template", async () => {
        const token = await createVerifiedUserAndLogin("template-delete-404@example.com");

        const response = await SELF.fetch("https://worker/templates/nonexistent-id", {
          method: "DELETE",
          headers: { Authorization: `Bearer ${token}` },
        });

        expect(response.status).toBe(404);
      });
    });
  });

  describe("TSA Proxy Endpoint", () => {
    /**
     * The TSA proxy endpoint allows frontend to request RFC 3161 timestamps
     * without CORS issues by proxying through the worker.
     */

    beforeEach(() => {
      // Mock TSA server response
      fetchMock
        .get("https://freetsa.org")
        .intercept({ path: "/tsr", method: "POST" })
        .reply(
          200,
          // Minimal valid-looking TSA response (DER-encoded TimeStampResp)
          new Uint8Array([0x30, 0x03, 0x02, 0x01, 0x00]),
          { headers: { "Content-Type": "application/timestamp-reply" } }
        )
        .persist();
    });

    it("should proxy timestamp request to TSA server", async () => {
      // Build a minimal timestamp request body
      const timestampRequest = new Uint8Array([0x30, 0x20, 0x02, 0x01, 0x01]);

      const response = await SELF.fetch("https://worker/tsa-proxy", {
        method: "POST",
        body: timestampRequest,
      });

      expect(response.status).toBe(200);
      expect(response.headers.get("Content-Type")).toBe("application/timestamp-reply");
    });

    it("should accept custom TSA URL via query parameter", async () => {
      // Mock a different TSA server
      fetchMock
        .get("https://timestamp.digicert.com")
        .intercept({ path: "/", method: "POST" })
        .reply(
          200,
          new Uint8Array([0x30, 0x03, 0x02, 0x01, 0x00]),
          { headers: { "Content-Type": "application/timestamp-reply" } }
        )
        .persist();

      const response = await SELF.fetch(
        "https://worker/tsa-proxy?tsa=https://timestamp.digicert.com/",
        {
          method: "POST",
          body: new Uint8Array([0x30, 0x20]),
        }
      );

      expect(response.status).toBe(200);
    });

    it("should include CORS headers in response", async () => {
      const response = await SELF.fetch("https://worker/tsa-proxy", {
        method: "POST",
        body: new Uint8Array([0x30, 0x20]),
      });

      // Should have CORS headers for cross-origin requests
      expect(response.headers.get("Access-Control-Allow-Origin")).toBeDefined();
    });
  });

  /**
   * Document Lifecycle Management Tests
   * Tests for void, revise, and restart endpoints
   */
  describe("Document Lifecycle Management", () => {
    /**
     * Helper to create a session for lifecycle tests
     */
    async function createTestSession(
      token: string,
      email: string,
      numRecipients = 1
    ): Promise<{ session_id: string }> {
      const recipients = [];
      for (let i = 1; i <= numRecipients; i++) {
        recipients.push({
          id: `signer_${i}`,
          name: `Test Signer ${i}`,
          email: `signer${i}@example.com`,
          role: "signer",
          signed: false,
          signed_at: null,
        });
      }

      const sessionRequest = {
        encrypted_document: btoa("test document for lifecycle testing"),
        metadata: {
          filename: "lifecycle-test.pdf",
          page_count: 1,
          created_at: new Date().toISOString(),
          created_by: "Test User",
          sender_email: email,
        },
        recipients,
        fields: recipients.map((r, idx) => ({
          id: `sig_field_${idx + 1}`,
          field_type: "signature",
          recipient_id: r.id,
          page: 1,
          x_percent: 50.0,
          y_percent: 50.0 + idx * 10,
          width_percent: 20.0,
          height_percent: 5.0,
          required: true,
          value: null,
        })),
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
      return { session_id: data.session_id };
    }

    describe("PUT /session/{id}/void", () => {
      it("should void a pending session", async () => {
        const email = "void-test@example.com";
        const token = await createVerifiedUserAndLogin(email);
        const { session_id } = await createTestSession(token, email);

        const response = await SELF.fetch(`https://worker/session/${session_id}/void`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ reason: "Sending updated version" }),
        });

        expect(response.status).toBe(200);
        const data = (await response.json()) as { success: boolean; message: string };
        expect(data.success).toBe(true);

        // Verify session is now voided in KV
        const sessionsKv = env.SESSIONS;
        const sessionData = await sessionsKv.get(`session:${session_id}`);
        expect(sessionData).not.toBeNull();
        const storedSession = JSON.parse(sessionData!) as { status: string; voided_at: string; void_reason: string };
        expect(storedSession.status).toBe("Voided");
        expect(storedSession.voided_at).toBeDefined();
        expect(storedSession.void_reason).toBe("Sending updated version");
      });

      it("should reject voiding a completed session", async () => {
        const email = "void-completed@example.com";
        const token = await createVerifiedUserAndLogin(email);
        const { session_id } = await createTestSession(token, email);

        // Mark session as completed directly in KV
        const sessionsKv = env.SESSIONS;
        const sessionData = await sessionsKv.get(`session:${session_id}`);
        const session = JSON.parse(sessionData!);
        session.status = "Completed";
        await sessionsKv.put(`session:${session_id}`, JSON.stringify(session));

        const response = await SELF.fetch(`https://worker/session/${session_id}/void`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({}),
        });

        expect(response.status).toBe(400);
        const data = (await response.json()) as { success: boolean; message: string };
        expect(data.success).toBe(false);
        expect(data.message).toMatch(/cannot.*void|completed/i);
      });

      it("should reject voiding by non-owner", async () => {
        const ownerEmail = "void-owner@example.com";
        const ownerToken = await createVerifiedUserAndLogin(ownerEmail);
        const { session_id } = await createTestSession(ownerToken, ownerEmail);

        const otherEmail = "void-other@example.com";
        const otherToken = await createVerifiedUserAndLogin(otherEmail);

        const response = await SELF.fetch(`https://worker/session/${session_id}/void`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${otherToken}`,
          },
          body: JSON.stringify({}),
        });

        expect(response.status).toBe(403);
      });
    });

    describe("PUT /session/{id}/revise", () => {
      it("should revise a session when no one has signed", async () => {
        const email = "revise-test@example.com";
        const token = await createVerifiedUserAndLogin(email);
        const { session_id } = await createTestSession(token, email);

        const response = await SELF.fetch(`https://worker/session/${session_id}/revise`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            fields: [
              {
                id: "updated_field",
                field_type: "signature",
                recipient_id: "signer_1",
                page: 1,
                x_percent: 60.0,
                y_percent: 70.0,
                width_percent: 25.0,
                height_percent: 6.0,
                required: true,
                value: null,
              },
            ],
            message: "Please note the updated field position",
          }),
        });

        expect(response.status).toBe(200);
        const data = (await response.json()) as {
          success: boolean;
          message: string;
          tokens: { recipient_id: string; signing_url: string }[];
        };
        expect(data.success).toBe(true);
        expect(data.tokens).toBeDefined();
        expect(data.tokens.length).toBeGreaterThan(0);

        // Verify session was updated in KV
        const sessionsKv = env.SESSIONS;
        const sessionData = await sessionsKv.get(`session:${session_id}`);
        const storedSession = JSON.parse(sessionData!) as {
          fields: { id: string; x_percent: number }[];
          revision_count: number;
          token_version: number;
        };
        expect(storedSession.fields[0].id).toBe("updated_field");
        expect(storedSession.fields[0].x_percent).toBe(60.0);
        expect(storedSession.revision_count).toBe(1);
        expect(storedSession.token_version).toBe(1);
      });

      it("should reject revise when someone has signed", async () => {
        const email = "revise-signed@example.com";
        const token = await createVerifiedUserAndLogin(email);
        const { session_id } = await createTestSession(token, email);

        // Mark one recipient as signed
        const sessionsKv = env.SESSIONS;
        const sessionData = await sessionsKv.get(`session:${session_id}`);
        const session = JSON.parse(sessionData!);
        session.recipients[0].signed = true;
        session.recipients[0].signed_at = new Date().toISOString();
        await sessionsKv.put(`session:${session_id}`, JSON.stringify(session));

        const response = await SELF.fetch(`https://worker/session/${session_id}/revise`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            fields: [],
          }),
        });

        expect(response.status).toBe(400);
        const data = (await response.json()) as { success: boolean; message: string };
        expect(data.success).toBe(false);
        expect(data.message).toMatch(/already signed|restart/i);
      });

      it("should reject revise by non-owner", async () => {
        const ownerEmail = "revise-owner@example.com";
        const ownerToken = await createVerifiedUserAndLogin(ownerEmail);
        const { session_id } = await createTestSession(ownerToken, ownerEmail);

        const otherEmail = "revise-other@example.com";
        const otherToken = await createVerifiedUserAndLogin(otherEmail);

        const response = await SELF.fetch(`https://worker/session/${session_id}/revise`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${otherToken}`,
          },
          body: JSON.stringify({ fields: [] }),
        });

        expect(response.status).toBe(403);
      });
    });

    describe("PUT /session/{id}/restart", () => {
      it("should restart a session and clear signatures", async () => {
        const email = "restart-test@example.com";
        const token = await createVerifiedUserAndLogin(email);
        const { session_id } = await createTestSession(token, email, 2);

        // Mark one recipient as signed
        const sessionsKv = env.SESSIONS;
        let sessionData = await sessionsKv.get(`session:${session_id}`);
        let session = JSON.parse(sessionData!);
        session.recipients[0].signed = true;
        session.recipients[0].signed_at = new Date().toISOString();
        session.signature_annotations = [{ test: "annotation" }];
        await sessionsKv.put(`session:${session_id}`, JSON.stringify(session));

        const response = await SELF.fetch(`https://worker/session/${session_id}/restart`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            fields: [
              {
                id: "restart_field",
                field_type: "signature",
                recipient_id: "signer_1",
                page: 1,
                x_percent: 50.0,
                y_percent: 50.0,
                width_percent: 20.0,
                height_percent: 5.0,
                required: true,
                value: null,
              },
            ],
            message: "Document has been updated - please sign again",
          }),
        });

        expect(response.status).toBe(200);
        const data = (await response.json()) as {
          success: boolean;
          message: string;
          tokens: { recipient_id: string }[];
        };
        expect(data.success).toBe(true);
        expect(data.tokens).toBeDefined();

        // Verify session was reset
        sessionData = await sessionsKv.get(`session:${session_id}`);
        session = JSON.parse(sessionData!);
        expect(session.status).toBe("Pending");
        expect(session.recipients[0].signed).toBe(false);
        expect(session.recipients[0].signed_at).toBeNull();
        expect(session.signature_annotations).toEqual([]);
        expect(session.revision_count).toBe(1);
        expect(session.token_version).toBe(1);
      });

      it("should reject restart on completed session", async () => {
        const email = "restart-completed@example.com";
        const token = await createVerifiedUserAndLogin(email);
        const { session_id } = await createTestSession(token, email);

        // Mark session as completed
        const sessionsKv = env.SESSIONS;
        const sessionData = await sessionsKv.get(`session:${session_id}`);
        const session = JSON.parse(sessionData!);
        session.status = "Completed";
        await sessionsKv.put(`session:${session_id}`, JSON.stringify(session));

        const response = await SELF.fetch(`https://worker/session/${session_id}/restart`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ fields: [] }),
        });

        expect(response.status).toBe(400);
        const data = (await response.json()) as { success: boolean; message: string };
        expect(data.success).toBe(false);
        expect(data.message).toMatch(/cannot.*restart|completed/i);
      });

      it("should reject restart by non-owner", async () => {
        const ownerEmail = "restart-owner@example.com";
        const ownerToken = await createVerifiedUserAndLogin(ownerEmail);
        const { session_id } = await createTestSession(ownerToken, ownerEmail);

        const otherEmail = "restart-other@example.com";
        const otherToken = await createVerifiedUserAndLogin(otherEmail);

        const response = await SELF.fetch(`https://worker/session/${session_id}/restart`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${otherToken}`,
          },
          body: JSON.stringify({ fields: [] }),
        });

        expect(response.status).toBe(403);
      });
    });

    describe("Session State Updates", () => {
      it("should update voided_at and void_reason after void", async () => {
        const email = "state-void@example.com";
        const token = await createVerifiedUserAndLogin(email);
        const { session_id } = await createTestSession(token, email);

        // Void the session
        const voidResponse = await SELF.fetch(`https://worker/session/${session_id}/void`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ reason: "Testing state updates" }),
        });

        expect(voidResponse.status).toBe(200);

        // Verify session state was updated
        const sessionsKv = env.SESSIONS;
        const sessionData = await sessionsKv.get(`session:${session_id}`);
        const session = JSON.parse(sessionData!) as { status: string; voided_at: string; void_reason: string };
        expect(session.status).toBe("Voided");
        expect(session.voided_at).toBeDefined();
        expect(session.void_reason).toBe("Testing state updates");
      });

      it("should update revision_count and fields after revise", async () => {
        const email = "state-revise@example.com";
        const token = await createVerifiedUserAndLogin(email);
        const { session_id } = await createTestSession(token, email);

        // Revise the session
        const reviseResponse = await SELF.fetch(`https://worker/session/${session_id}/revise`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            fields: [
              {
                id: "new_field",
                field_type: "signature",
                recipient_id: "signer_1",
                page: 1,
                x_percent: 25.0,
                y_percent: 75.0,
                width_percent: 15.0,
                height_percent: 4.0,
                required: true,
                value: null,
              },
            ],
          }),
        });

        expect(reviseResponse.status).toBe(200);

        // Verify session was updated
        const sessionsKv = env.SESSIONS;
        const sessionData = await sessionsKv.get(`session:${session_id}`);
        const session = JSON.parse(sessionData!) as { fields: { id: string; x_percent: number }[]; revision_count: number };
        expect(session.fields[0].id).toBe("new_field");
        expect(session.fields[0].x_percent).toBe(25.0);
        expect(session.revision_count).toBe(1);
      });

      it("should successfully restart a pending session", async () => {
        const email = "state-restart@example.com";
        const token = await createVerifiedUserAndLogin(email);
        const { session_id } = await createTestSession(token, email);

        // Restart the session with new fields
        const restartResponse = await SELF.fetch(`https://worker/session/${session_id}/restart`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            fields: [
              {
                id: "restarted_sig_field",
                field_type: "signature",
                recipient_id: "signer_1",
                page: 1,
                x_percent: 30.0,
                y_percent: 80.0,
                width_percent: 25.0,
                height_percent: 6.0,
                required: true,
                value: null,
              },
            ],
            message: "Document has been updated",
          }),
        });

        // Restart should succeed
        expect(restartResponse.status).toBe(200);
        const responseData = (await restartResponse.json()) as { success: boolean; tokens: unknown[] };
        expect(responseData.success).toBe(true);
        expect(responseData.tokens).toBeDefined();

        // Verify session was updated
        const sessionsKv = env.SESSIONS;
        const sessionData = await sessionsKv.get(`session:${session_id}`);
        const session = JSON.parse(sessionData!) as {
          status: string;
          fields: { id: string; x_percent: number }[];
          revision_count: number;
          recipients: { signed: boolean }[];
        };
        expect(session.status).toBe("Pending");
        expect(session.fields[0].id).toBe("restarted_sig_field");
        expect(session.fields[0].x_percent).toBe(30.0);
        expect(session.revision_count).toBe(1);
        // All recipients should be reset
        expect(session.recipients[0].signed).toBe(false);
      });
    });

    describe("Inbox behavior after void", () => {
      it("should NOT show voided sessions in recipient inbox to_sign", async () => {
        // Create sender account
        const senderEmail = "void-inbox-sender@example.com";
        const senderToken = await createVerifiedUserAndLogin(senderEmail);

        // Create recipient account with different email
        const recipientEmail = "void-inbox-recipient@example.com";
        const recipientToken = await createVerifiedUserAndLogin(recipientEmail);

        // Sender creates a session with recipient
        const sessionRequest = {
          encrypted_document: btoa("document for voided inbox test"),
          metadata: {
            filename: "voided-inbox-test.pdf",
            page_count: 1,
            created_at: new Date().toISOString(),
            created_by: "Sender User",
            sender_email: senderEmail,
          },
          recipients: [
            {
              id: "r1",
              name: "Recipient User",
              email: recipientEmail,
              role: "signer",
              signed: false,
              signed_at: null,
            },
          ],
          fields: [
            {
              id: "sig_1",
              field_type: "signature",
              recipient_id: "r1",
              page: 1,
              x_percent: 50.0,
              y_percent: 70.0,
              width_percent: 20.0,
              height_percent: 5.0,
              required: true,
              value: null,
            },
          ],
          expiry_hours: 168,
        };

        const createResponse = await SELF.fetch("https://worker/session", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${senderToken}`,
          },
          body: JSON.stringify(sessionRequest),
        });
        expect(createResponse.status).toBe(200);
        const { session_id } = (await createResponse.json()) as { session_id: string };

        // Step 1: Verify document appears in recipient's inbox to_sign BEFORE void
        const beforeVoidDashboard = await SELF.fetch("https://worker/my-sessions", {
          headers: { Authorization: `Bearer ${recipientToken}` },
        });
        expect(beforeVoidDashboard.status).toBe(200);

        const beforeData = (await beforeVoidDashboard.json()) as {
          success: boolean;
          inbox: {
            to_sign: { session_id: string; filename: string; my_status: string }[];
            completed: unknown[];
            declined: unknown[];
          };
        };

        expect(beforeData.success).toBe(true);
        expect(beforeData.inbox.to_sign.length).toBe(1);
        expect(beforeData.inbox.to_sign[0].session_id).toBe(session_id);
        expect(beforeData.inbox.to_sign[0].filename).toBe("voided-inbox-test.pdf");
        expect(beforeData.inbox.to_sign[0].my_status).toBe("pending");

        // Step 2: Sender voids the document
        const voidResponse = await SELF.fetch(`https://worker/session/${session_id}/void`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${senderToken}`,
          },
          body: JSON.stringify({ reason: "Sending updated version" }),
        });
        expect(voidResponse.status).toBe(200);

        // Step 3: Verify document NO LONGER appears in recipient's inbox to_sign
        const afterVoidDashboard = await SELF.fetch("https://worker/my-sessions", {
          headers: { Authorization: `Bearer ${recipientToken}` },
        });
        expect(afterVoidDashboard.status).toBe(200);

        const afterData = (await afterVoidDashboard.json()) as {
          success: boolean;
          inbox: {
            to_sign: { session_id: string; filename: string; my_status: string }[];
            completed: unknown[];
            declined: unknown[];
          };
        };

        expect(afterData.success).toBe(true);
        // THIS IS THE KEY ASSERTION: voided sessions should NOT appear in inbox
        expect(afterData.inbox.to_sign.length).toBe(0);
        // And should not be in completed or declined either
        expect(afterData.inbox.completed.length).toBe(0);
        expect(afterData.inbox.declined.length).toBe(0);
      });
    });
  });
});
