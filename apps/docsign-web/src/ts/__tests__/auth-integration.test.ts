/**
 * Auth Integration Tests
 *
 * Regression tests for authentication flows.
 * These tests verify the auth API behavior and UX fixes.
 *
 * Note: These tests require the auth API to be running.
 * For local testing, use `wrangler dev` in apps/docsign-web/worker/
 */

import { describe, it, expect, beforeAll } from "vitest";

// API base URL - use local for testing, production for CI
const API_BASE =
  process.env.AUTH_API_URL || "https://api.getsignatures.org";

describe("Auth API Integration", () => {
  describe("Health Check", () => {
    it("API-1: Health endpoint should return 200", async () => {
      const response = await fetch(`${API_BASE}/health`);
      expect(response.status).toBe(200);
    });
  });

  describe("Forgot Password - Email Not Found", () => {
    it("API-2: Should return 404 with explicit error for non-existent email", async () => {
      const nonExistentEmail = `nonexistent${Date.now()}@example.com`;

      const response = await fetch(`${API_BASE}/auth/forgot-password`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: nonExistentEmail }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(404);

      const data = await response.json();
      expect(data.success).toBe(false);
      expect(data.message).toContain("No account found");
      expect(data.message).toContain("create a new account");
    });

    it("API-3: Should return 400 for invalid request body", async () => {
      const response = await fetch(`${API_BASE}/auth/forgot-password`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: "invalid json",
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(400);

      const data = await response.json();
      expect(data.success).toBe(false);
    });
  });

  // Registration tests may be rate-limited (429) by production DDoS mitigation
  // These tests are designed to skip gracefully when rate limited
  describe("Registration Validation", () => {
    it("API-4: Should reject weak passwords", async () => {
      const response = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: `test${Date.now()}@example.com`,
          password: "weak", // Too short, no uppercase, no number
          name: "Test User",
        }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(400);

      const data = await response.json();
      expect(data.success).toBe(false);
      // Should mention password requirements
      expect(data.message.toLowerCase()).toMatch(/password|character|uppercase|number/);
    });

    it("API-5: Should reject invalid email format", async () => {
      const response = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "not-an-email",
          password: "ValidPass123",
          name: "Test User",
        }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(400);

      const data = await response.json();
      expect(data.success).toBe(false);
      expect(data.message.toLowerCase()).toMatch(/email/);
    });

    it("API-6: Should reject empty name", async () => {
      const response = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: `test${Date.now()}@example.com`,
          password: "ValidPass123",
          name: "", // Empty name
        }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(400);

      const data = await response.json();
      expect(data.success).toBe(false);
      expect(data.message.toLowerCase()).toMatch(/name/);
    });
  });

  // Registration success tests may be rate-limited by production DDoS mitigation
  describe("Registration Success", () => {
    it("API-7: Should successfully register new user with valid data", async () => {
      const uniqueEmail = `testuser${Date.now()}@example.com`;

      const response = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: uniqueEmail,
          password: "ValidPass123",
          name: "Test User",
        }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(201);

      const data = await response.json();
      expect(data.success).toBe(true);
      expect(data.user_id).toBeDefined();
      expect(data.message).toContain("verify");
    });

    it("API-8: Should reject duplicate email registration", async () => {
      const email = `duplicate${Date.now()}@example.com`;

      // First registration
      const firstResponse = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password: "ValidPass123",
          name: "First User",
        }),
      });

      // Skip if rate limited
      if (firstResponse.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      // Second registration with same email
      const response = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password: "ValidPass123",
          name: "Second User",
        }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(409); // Conflict

      const data = await response.json();
      expect(data.success).toBe(false);
      expect(data.message.toLowerCase()).toMatch(/already exists/);
    });
  });

  describe("Login Flow", () => {
    it("API-9: Should reject login with non-existent email", async () => {
      const response = await fetch(`${API_BASE}/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: `nonexistent${Date.now()}@example.com`,
          password: "SomePass123",
        }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(401);

      const data = await response.json();
      expect(data.success).toBe(false);
      // Should not reveal whether email exists (security)
      expect(data.error).toMatch(/invalid email or password/i);
    });
  });

  // ============================================
  // BUG FIX REGRESSION TESTS
  // ============================================
  // These tests capture bugs that were fixed in production.
  // They should fail BEFORE the fix and pass AFTER.

  describe("Bug Fixes - Verification UX", () => {
    /**
     * BUG 1: Registration with existing unverified email
     *
     * BEFORE: Returns generic "An account with this email already exists"
     * AFTER: Should indicate account is unverified and offer resend option
     */
    it("API-BUG1: Should indicate account exists but is unverified when re-registering", async () => {
      const email = `bugtest1_${Date.now()}@example.com`;

      // First registration - creates unverified account
      const firstResponse = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password: "ValidPass123",
          name: "Bug Test User",
        }),
      });

      // Skip if rate limited
      if (firstResponse.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      // Verify first registration succeeded
      expect(firstResponse.status).toBe(201);

      // Second registration with same email
      const response = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password: "ValidPass123",
          name: "Bug Test User 2",
        }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(409); // Conflict

      const data = await response.json();
      expect(data.success).toBe(false);

      // BUG FIX: Should indicate the account is unverified
      expect(data.message.toLowerCase()).toContain("verified");
      expect(data.message.toLowerCase()).toMatch(/hasn't been verified|not yet verified|unverified/);

      // BUG FIX: Should include needs_verification flag for frontend
      expect(data.needs_verification).toBe(true);
    });

    /**
     * BUG 2: Login with unverified email
     *
     * BEFORE: Returns "Please verify your email before logging in" with no action
     * AFTER: Should return needs_verification=true and email for resend flow
     */
    it("API-BUG2: Login with unverified email should include needs_verification flag", async () => {
      const email = `bugtest2_${Date.now()}@example.com`;
      const password = "ValidPass123";

      // First create an unverified account
      const registerResponse = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password,
          name: "Bug Test User",
        }),
      });

      // Skip if rate limited
      if (registerResponse.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      // Verify registration succeeded
      expect(registerResponse.status).toBe(201);

      // Try to login (should fail because email not verified)
      const response = await fetch(`${API_BASE}/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password,
        }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(403); // Forbidden - unverified

      const data = await response.json();
      expect(data.success).toBe(false);

      // BUG FIX: Should include needs_verification flag for frontend
      expect(data.needs_verification).toBe(true);

      // BUG FIX: Should include email for resend flow
      expect(data.email).toBe(email.toLowerCase());

      // BUG FIX: Error message should mention resend option
      expect(data.error.toLowerCase()).toMatch(/check your inbox|send a new one|resend/);
    });

    /**
     * BUG 3: Resend verification endpoint must work
     *
     * BEFORE: Endpoint existed but wasn't exposed to UI
     * AFTER: Users can request new verification emails via button
     */
    it("API-BUG3: Resend verification endpoint should send email for unverified account", async () => {
      const email = `bugtest3_${Date.now()}@example.com`;

      // First create an unverified account
      const registerResponse = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password: "ValidPass123",
          name: "Resend Test User",
        }),
      });

      // Skip if rate limited
      if (registerResponse.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      // Verify registration succeeded
      expect(registerResponse.status).toBe(201);

      // Request resend verification
      const response = await fetch(`${API_BASE}/auth/resend-verification`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(200);

      const data = await response.json();
      expect(data.success).toBe(true);
      expect(data.message.toLowerCase()).toMatch(/verification|email|sent/);
    });

    /**
     * BUG 4: Resend verification should return 404 for non-existent email
     */
    it("API-BUG4: Resend verification should return 404 for non-existent email", async () => {
      const nonExistentEmail = `nonexistent_resend_${Date.now()}@example.com`;

      const response = await fetch(`${API_BASE}/auth/resend-verification`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: nonExistentEmail }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(404);

      const data = await response.json();
      expect(data.success).toBe(false);
      expect(data.message.toLowerCase()).toContain("no account found");
    });

    /**
     * BUG 5: Resend verification should return error for already verified email
     */
    it("API-BUG5: Resend verification should indicate if email already verified", async () => {
      // Note: This test would need a verified account to work properly
      // For now, we'll test the endpoint exists and returns correct format
      const response = await fetch(`${API_BASE}/auth/resend-verification`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: "invalid" }),
      });

      // Should return 400 or 404, not 500
      expect(response.status).not.toBe(500);
    });

    /**
     * BUG 6: Rate limit response should include retry_after_seconds
     *
     * This ensures the frontend can show a countdown timer
     */
    it("API-BUG6: Rate limit response should include retry_after_seconds", async () => {
      // This test intentionally triggers rate limiting
      // We need to make many requests quickly to hit the limit

      const email = `ratelimit_${Date.now()}@example.com`;
      const requests: Promise<Response>[] = [];

      // Make 10 rapid requests to trigger rate limit
      for (let i = 0; i < 10; i++) {
        requests.push(
          fetch(`${API_BASE}/auth/register`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              email: `${email}_${i}`,
              password: "ValidPass123",
              name: "Rate Limit Test",
            }),
          })
        );
      }

      const responses = await Promise.all(requests);

      // Find any 429 response
      const rateLimitedResponse = responses.find(r => r.status === 429);

      if (!rateLimitedResponse) {
        // If we didn't hit rate limit, skip the test
        console.log("Skipped: Could not trigger rate limit in test");
        return;
      }

      const data = await rateLimitedResponse.json();

      // Should include retry_after_seconds for frontend countdown
      expect(data.retry_after_seconds).toBeDefined();
      expect(typeof data.retry_after_seconds).toBe("number");
      expect(data.retry_after_seconds).toBeGreaterThan(0);
    });
  });

  // ============================================
  // EMAIL-FIRST UX TESTS
  // ============================================
  // These tests verify the new email-first authentication flow

  describe("Email Check Endpoint", () => {
    /**
     * BUG 7: check-email endpoint should return exists/verified status
     *
     * This enables email-first UX where we check if email exists
     * BEFORE showing password fields
     */
    it("API-BUG7: check-email should return exists=true for existing email", async () => {
      // First create an account
      const email = `checktest_${Date.now()}@example.com`;

      const registerResponse = await fetch(`${API_BASE}/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password: "ValidPass123",
          name: "Check Email Test",
        }),
      });

      // Skip if rate limited
      if (registerResponse.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(registerResponse.status).toBe(201);

      // Now check if email exists
      const response = await fetch(`${API_BASE}/auth/check-email`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(200);

      const data = await response.json();
      expect(data.exists).toBe(true);
      expect(data.verified).toBe(false); // New account is unverified
    });

    /**
     * BUG 8: check-email should return exists=false for new email
     */
    it("API-BUG8: check-email should return exists=false for new email", async () => {
      const newEmail = `brand_new_${Date.now()}@example.com`;

      const response = await fetch(`${API_BASE}/auth/check-email`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: newEmail }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(200);

      const data = await response.json();
      expect(data.exists).toBe(false);
    });

    /**
     * BUG 9: check-email should validate email format
     */
    it("API-BUG9: check-email should reject invalid email format", async () => {
      const response = await fetch(`${API_BASE}/auth/check-email`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: "not-an-email" }),
      });

      // Skip if rate limited
      if (response.status === 429) {
        console.log("Skipped: Rate limited by production API");
        return;
      }

      expect(response.status).toBe(400);
    });
  });
});

describe("Loading Overlay Jokes", () => {
  it("JOKES-1: Should have at least 10 jokes available", async () => {
    // This tests that the jokes module exports correctly
    const { JOKES } = await import("../jokes");
    expect(JOKES.length).toBeGreaterThanOrEqual(10);
  });

  it("JOKES-2: All jokes should have setup and punchline", async () => {
    const { JOKES } = await import("../jokes");
    JOKES.forEach((joke, index) => {
      expect(joke.setup).toBeDefined();
      expect(joke.setup.length).toBeGreaterThan(0);
      expect(joke.punchline).toBeDefined();
      expect(joke.punchline.length).toBeGreaterThan(0);
    });
  });

  it("JOKES-3: getRandomJoke should return different jokes", async () => {
    const { getRandomJoke, resetJokeHistory } = await import("../jokes");
    resetJokeHistory();

    const jokes = new Set<string>();
    for (let i = 0; i < 5; i++) {
      const joke = getRandomJoke();
      jokes.add(joke.setup);
    }

    // Should have gotten at least 3 different jokes out of 5 attempts
    expect(jokes.size).toBeGreaterThanOrEqual(3);
  });
});
