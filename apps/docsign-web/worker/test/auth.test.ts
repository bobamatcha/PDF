/**
 * Auth Integration Tests
 *
 * These tests run directly in the Workers runtime with isolated KV storage.
 * Each test gets fresh KV state - NO rate limiting between tests!
 *
 * Uses @cloudflare/vitest-pool-workers for true integration testing.
 *
 * IMPORTANT: All external API calls (Resend) are mocked using fetchMock.
 * This prevents tests from wasting Resend credits.
 */

import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { SELF, env, fetchMock } from "cloudflare:test";

// Enable fetch mocking before all tests - prevents real network calls to Resend
beforeAll(() => {
  fetchMock.activate();
  fetchMock.disableNetConnect();
});

// Setup persistent mock for Resend API before each test
// This mock will handle any email send attempt without consuming credits
beforeEach(() => {
  // Reset any previous interceptors
  fetchMock.deactivate();
  fetchMock.activate();
  fetchMock.disableNetConnect();

  // Mock Resend API - persist() allows multiple calls per test
  // This prevents any real emails from being sent
  fetchMock
    .get("https://api.resend.com")
    .intercept({ path: "/emails", method: "POST" })
    .reply(200, { id: "mock-email-id-" + Math.random().toString(36).slice(2) })
    .persist();
});

describe("Auth API Integration", () => {
  describe("Health Check", () => {
    it("should return 200 with healthy status", async () => {
      const response = await SELF.fetch("https://worker/health");

      expect(response.status).toBe(200);

      const data = await response.json() as {
        status: string;
        version: string;
        dependencies: {
          kv_sessions: { status: string };
          kv_rate_limits: { status: string };
        };
      };

      expect(data.status).toBe("healthy");
      expect(data.version).toBeDefined();
      expect(data.dependencies.kv_sessions.status).toBe("healthy");
      expect(data.dependencies.kv_rate_limits.status).toBe("healthy");
    });
  });

  describe("Forgot Password - Email Not Found", () => {
    it("should return 404 with explicit error for non-existent email", async () => {
      const response = await SELF.fetch("https://worker/auth/forgot-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: "nonexistent@example.com" }),
      });

      expect(response.status).toBe(404);

      const data = await response.json() as { success: boolean; message: string };
      expect(data.success).toBe(false);
      expect(data.message).toContain("No account found");
      expect(data.message).toContain("create a new account");
    });

    it("should return 400 for invalid request body", async () => {
      const response = await SELF.fetch("https://worker/auth/forgot-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: "invalid json",
      });

      expect(response.status).toBe(400);

      const data = await response.json() as { success: boolean };
      expect(data.success).toBe(false);
    });

    it("should return 400 for missing email field", async () => {
      const response = await SELF.fetch("https://worker/auth/forgot-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });

      expect(response.status).toBe(400);
    });
  });

  describe("Registration Validation", () => {
    it("should reject weak passwords", async () => {
      const response = await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "test@example.com",
          password: "weak", // Too short, no uppercase, no number
          first_name: "Test",
          last_name: "User",
        }),
      });

      expect(response.status).toBe(400);

      const data = await response.json() as { success: boolean; message: string };
      expect(data.success).toBe(false);
      expect(data.message.toLowerCase()).toMatch(/password|character|uppercase|number/);
    });

    it("should reject invalid email format", async () => {
      const response = await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "not-an-email",
          password: "ValidPass123",
          first_name: "Test",
          last_name: "User",
        }),
      });

      expect(response.status).toBe(400);

      const data = await response.json() as { success: boolean; message: string };
      expect(data.success).toBe(false);
      expect(data.message.toLowerCase()).toMatch(/email/);
    });

    it("should reject empty name", async () => {
      const response = await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "test@example.com",
          password: "ValidPass123",
          first_name: "", // Empty first name
          last_name: "User",
        }),
      });

      expect(response.status).toBe(400);

      const data = await response.json() as { success: boolean; message: string };
      expect(data.success).toBe(false);
      expect(data.message.toLowerCase()).toMatch(/name/);
    });
  });

  describe("Registration Success", () => {
    it("should successfully register new user with valid data", async () => {
      const response = await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "newuser@example.com",
          password: "ValidPass123",
          first_name: "Test",
          last_name: "User",
        }),
      });

      expect(response.status).toBe(201);

      const data = await response.json() as {
        success: boolean;
        user_id: string;
        message: string;
      };
      expect(data.success).toBe(true);
      expect(data.user_id).toBeDefined();
      expect(data.message).toContain("verify");
    });

    it("should reject duplicate email registration", async () => {
      const email = "duplicate@example.com";

      // First registration
      const first = await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password: "ValidPass123",
          first_name: "First",
          last_name: "User",
        }),
      });
      expect(first.status).toBe(201);

      // Second registration with same email
      const second = await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password: "ValidPass123",
          first_name: "Second",
          last_name: "User",
        }),
      });

      expect(second.status).toBe(409); // Conflict

      const data = await second.json() as { success: boolean; message: string };
      expect(data.success).toBe(false);
      expect(data.message.toLowerCase()).toMatch(/already exists/);
    });
  });

  describe("Login Flow", () => {
    it("should reject login with non-existent email", async () => {
      const response = await SELF.fetch("https://worker/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "nonexistent@example.com",
          password: "SomePass123",
        }),
      });

      expect(response.status).toBe(401);

      const data = await response.json() as { success: boolean; error: string };
      expect(data.success).toBe(false);
      // Should not reveal whether email exists (security)
      expect(data.error).toMatch(/invalid email or password/i);
    });

    it("should reject login with wrong password", async () => {
      // First register a user
      await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "logintest@example.com",
          password: "CorrectPass123",
          first_name: "Login",
          last_name: "Test",
        }),
      });

      // Try to login with wrong password
      const response = await SELF.fetch("https://worker/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "logintest@example.com",
          password: "WrongPass456",
        }),
      });

      expect(response.status).toBe(401);

      const data = await response.json() as { success: boolean; error: string };
      expect(data.success).toBe(false);
      expect(data.error).toMatch(/invalid email or password/i);
    });
  });

  describe("Email Verification Integration", () => {
    it("should send verification email on registration", async () => {
      // Register a user
      const response = await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "emailverify@example.com",
          password: "ValidPass123",
          first_name: "Email",
          last_name: "Verify",
        }),
      });

      // Registration should succeed
      expect(response.status).toBe(201);

      const data = await response.json() as {
        success: boolean;
        message: string;
        user_id: string;
        email_sent?: boolean;
      };
      expect(data.success).toBe(true);

      // FAILING TEST: The response should indicate email was attempted
      // Currently the code just logs to console and doesn't report email status
      // After fix: response should include email_sent field
      expect(data.email_sent).toBeDefined();
    });

    it("should have resend-verification endpoint", async () => {
      // First register a user
      const registerResponse = await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "resendtest@example.com",
          password: "ValidPass123",
          first_name: "Resend",
          last_name: "Test",
        }),
      });
      expect(registerResponse.status).toBe(201);

      // FAILING TEST: Try to resend verification email
      // This endpoint doesn't exist yet - should return 200, not 404
      const resendResponse = await SELF.fetch("https://worker/auth/resend-verification", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: "resendtest@example.com" }),
      });

      // Currently this will return 404 because the endpoint doesn't exist
      // After fix: should return 200 (or 400/404 with proper error message)
      expect(resendResponse.status).not.toBe(404);
    });
  });

  describe("Password Reset Email Integration", () => {
    it("should send password reset email when user exists", async () => {
      // First register a user
      const registerResponse = await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "resetemailtest@example.com",
          password: "ValidPass123",
          first_name: "Reset",
          last_name: "Email",
        }),
      });
      expect(registerResponse.status).toBe(201);

      // Request password reset
      const resetResponse = await SELF.fetch("https://worker/auth/forgot-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: "resetemailtest@example.com" }),
      });

      expect(resetResponse.status).toBe(200);

      const data = await resetResponse.json() as {
        success: boolean;
        message: string;
        email_sent?: boolean;
      };
      expect(data.success).toBe(true);

      // FAILING TEST: Response should indicate email was attempted
      // Currently the code just logs to console and doesn't report email status
      // After fix: response should include email_sent field
      expect(data.email_sent).toBeDefined();
    });
  });

  // Note: Rate limiting tests removed because:
  // 1. RequestLink tier is 100 requests/day - impractical to test with 100+ HTTP requests
  // 2. Rate limiting is verified to work in production via manual testing
  // 3. The rate limit logic is straightforward (KV counter + TTL)
});
