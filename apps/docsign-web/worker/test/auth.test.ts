/**
 * Auth Integration Tests
 *
 * These tests run directly in the Workers runtime with isolated KV storage.
 * Each test gets fresh KV state - NO rate limiting between tests!
 *
 * Uses @cloudflare/vitest-pool-workers for true integration testing.
 */

import { describe, it, expect, beforeAll } from "vitest";
import { SELF, env } from "cloudflare:test";

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
          name: "Test User",
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
          name: "Test User",
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
          name: "", // Empty name
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
          name: "Test User",
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
          name: "First User",
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
          name: "Second User",
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
          name: "Login Test User",
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

  describe("Rate Limiting", () => {
    it("should have rate limit headers after multiple requests", async () => {
      // Make several requests to trigger rate limit tracking
      // With isolated storage, we start fresh each test
      for (let i = 0; i < 5; i++) {
        await SELF.fetch("https://worker/auth/register", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            email: `ratelimit${i}@example.com`,
            password: "ValidPass123",
            name: "Rate Test",
          }),
        });
      }

      // The 4th request should hit rate limit (RequestLink tier = 3/hour)
      // But with isolated storage, each test is fresh so this tests the mechanism
      const response = await SELF.fetch("https://worker/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "ratelimitfinal@example.com",
          password: "ValidPass123",
          name: "Rate Test",
        }),
      });

      // Should be rate limited after 3 requests (RequestLink tier)
      expect(response.status).toBe(429);

      const data = await response.json() as { error: string; retry_after_seconds: number };
      expect(data.error).toContain("Rate limit");
      expect(data.retry_after_seconds).toBeGreaterThan(0);
    });
  });
});
