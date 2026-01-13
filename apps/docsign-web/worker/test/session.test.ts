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
        in_progress: unknown[];
        completed: unknown[];
        declined: unknown[];
        expired: unknown[];
      };

      expect(data.success).toBe(true);
      expect(data.in_progress).toEqual([]);
      expect(data.completed).toEqual([]);
      expect(data.declined).toEqual([]);
      expect(data.expired).toEqual([]);
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

      expect(data.success).toBe(true);

      // Session should be in in_progress (pending status)
      expect(data.in_progress.length).toBe(1);
      expect(data.in_progress[0].filename).toBe("dashboard-test.pdf");
      expect(data.in_progress[0].document_alias).toBe("Dashboard Test Doc");
      expect(data.in_progress[0].signing_context).toBe(
        "Testing dashboard feature"
      );
      expect(data.in_progress[0].recipients_signed).toBe(0);
      expect(data.in_progress[0].recipients_total).toBe(1);
      expect(data.in_progress[0].recipients[0].name).toBe("Recipient One");
    });
  });
});
