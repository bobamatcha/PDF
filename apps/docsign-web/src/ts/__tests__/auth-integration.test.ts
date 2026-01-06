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
