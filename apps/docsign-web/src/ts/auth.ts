/**
 * Authentication Module for DocSign
 *
 * Handles user authentication with JWT tokens stored in localStorage.
 * Maintains local-first architecture - auth is required for senders
 * to create signing sessions.
 *
 * @packageDocumentation
 * @module auth
 */

import { createLogger } from "./logger";

const log = createLogger("Auth");

// ============================================
// Constants
// ============================================

/** Storage key for access token */
const ACCESS_TOKEN_KEY = "docsign_access_token";

/** Storage key for refresh token */
const REFRESH_TOKEN_KEY = "docsign_refresh_token";

/** Storage key for user data */
const USER_KEY = "docsign_user";

/** API base URL - Cloudflare Worker */
const API_BASE = "https://docsign-worker.orlandodowntownhome.workers.dev";

// ============================================
// Types
// ============================================

/** User tier for rate limiting */
export type UserTier = "free" | "pro";

/** Public user info (returned from login) */
export interface User {
  id: string;
  email: string;
  name: string;
  tier: UserTier;
  daily_documents_remaining: number;
}

/** Auth tokens from login/refresh */
export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  expires_in: number;
}

/** Registration response */
export interface RegisterResponse {
  success: boolean;
  user_id?: string;
  message: string;
  error?: string;
}

/** Login response */
export interface LoginResponse {
  success: boolean;
  access_token?: string;
  refresh_token?: string;
  expires_in?: number;
  user?: User;
  error?: string;
}

/** Refresh response */
export interface RefreshResponse {
  success: boolean;
  access_token?: string;
  expires_in?: number;
  error?: string;
}

/** Generic auth response */
export interface AuthResponse {
  success: boolean;
  message: string;
}

/** Auth state change event */
export interface AuthStateChangeEvent {
  isAuthenticated: boolean;
  user: User | null;
}

// ============================================
// Event Handling
// ============================================

type AuthEventListener = (event: AuthStateChangeEvent) => void;
const authListeners: AuthEventListener[] = [];

/**
 * Subscribe to auth state changes
 */
export function onAuthStateChange(listener: AuthEventListener): () => void {
  authListeners.push(listener);
  return () => {
    const index = authListeners.indexOf(listener);
    if (index > -1) {
      authListeners.splice(index, 1);
    }
  };
}

/**
 * Emit auth state change event
 */
function emitAuthStateChange(): void {
  const event: AuthStateChangeEvent = {
    isAuthenticated: isAuthenticated(),
    user: getCurrentUser(),
  };
  authListeners.forEach((listener) => listener(event));
}

// ============================================
// Token Management
// ============================================

/**
 * Get current access token from localStorage
 */
export function getAccessToken(): string | null {
  try {
    return localStorage.getItem(ACCESS_TOKEN_KEY);
  } catch (e) {
    log.warn("Failed to get access token:", e);
    return null;
  }
}

/**
 * Get current refresh token from localStorage
 */
export function getRefreshToken(): string | null {
  try {
    return localStorage.getItem(REFRESH_TOKEN_KEY);
  } catch (e) {
    log.warn("Failed to get refresh token:", e);
    return null;
  }
}

/**
 * Store tokens in localStorage
 */
function storeTokens(accessToken: string, refreshToken: string): void {
  try {
    localStorage.setItem(ACCESS_TOKEN_KEY, accessToken);
    localStorage.setItem(REFRESH_TOKEN_KEY, refreshToken);
  } catch (e) {
    log.error("Failed to store tokens:", e);
  }
}

/**
 * Clear tokens from localStorage
 */
function clearTokens(): void {
  try {
    localStorage.removeItem(ACCESS_TOKEN_KEY);
    localStorage.removeItem(REFRESH_TOKEN_KEY);
    localStorage.removeItem(USER_KEY);
  } catch (e) {
    log.error("Failed to clear tokens:", e);
  }
}

/**
 * Store user data in localStorage
 */
function storeUser(user: User): void {
  try {
    localStorage.setItem(USER_KEY, JSON.stringify(user));
  } catch (e) {
    log.error("Failed to store user:", e);
  }
}

// ============================================
// User State
// ============================================

/**
 * Get current user from localStorage
 */
export function getCurrentUser(): User | null {
  try {
    const userJson = localStorage.getItem(USER_KEY);
    if (!userJson) return null;
    return JSON.parse(userJson) as User;
  } catch (e) {
    log.warn("Failed to get user:", e);
    return null;
  }
}

/**
 * Check if user is authenticated
 */
export function isAuthenticated(): boolean {
  return getAccessToken() !== null;
}

/**
 * Get remaining documents for today
 */
export function getDocumentsRemaining(): number {
  const user = getCurrentUser();
  if (!user) return 0;
  return user.daily_documents_remaining;
}

// ============================================
// API Calls
// ============================================

/**
 * Register a new user account
 *
 * @param email - User's email address
 * @param password - User's password (min 8 chars, 1 upper, 1 lower, 1 number)
 * @param name - User's display name
 * @returns Registration result
 */
export async function register(
  email: string,
  password: string,
  name: string
): Promise<RegisterResponse> {
  try {
    log.info("Registering new user:", email);

    const response = await fetch(`${API_BASE}/auth/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password, name }),
    });

    const data = await response.json();

    if (!response.ok) {
      log.warn("Registration failed:", data.message);
      return {
        success: false,
        message: data.message || "Registration failed",
        error: data.message,
      };
    }

    log.info("Registration successful, verification email sent");
    return {
      success: true,
      user_id: data.user_id,
      message: data.message,
    };
  } catch (error) {
    log.error("Registration error:", error);
    return {
      success: false,
      message: "Network error. Please check your connection and try again.",
      error: String(error),
    };
  }
}

/**
 * Login with email and password
 *
 * @param email - User's email address
 * @param password - User's password
 * @returns Login result with tokens and user info
 */
export async function login(email: string, password: string): Promise<LoginResponse> {
  try {
    log.info("Logging in user:", email);

    const response = await fetch(`${API_BASE}/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password }),
    });

    const data = await response.json();

    if (!response.ok) {
      log.warn("Login failed:", data.error);
      return {
        success: false,
        error: data.error || "Login failed",
      };
    }

    // Store tokens and user
    if (data.access_token && data.refresh_token && data.user) {
      storeTokens(data.access_token, data.refresh_token);
      storeUser(data.user);
      emitAuthStateChange();
      log.info("Login successful:", data.user.email);
    }

    return {
      success: true,
      access_token: data.access_token,
      refresh_token: data.refresh_token,
      expires_in: data.expires_in,
      user: data.user,
    };
  } catch (error) {
    log.error("Login error:", error);
    return {
      success: false,
      error: "Network error. Please check your connection and try again.",
    };
  }
}

/**
 * Logout current user
 *
 * Clears local tokens and notifies server (best effort).
 */
export async function logout(): Promise<void> {
  const token = getAccessToken();

  if (token) {
    try {
      await fetch(`${API_BASE}/auth/logout`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
      });
    } catch (error) {
      log.warn("Logout API call failed:", error);
    }
  }

  // Clear local storage regardless of API success
  clearTokens();
  emitAuthStateChange();
  log.info("User logged out");
}

/**
 * Refresh access token using refresh token
 *
 * @returns true if refresh was successful
 */
export async function refreshToken(): Promise<boolean> {
  const token = getRefreshToken();

  if (!token) {
    log.debug("No refresh token available");
    return false;
  }

  try {
    log.debug("Refreshing access token");

    const response = await fetch(`${API_BASE}/auth/refresh`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ refresh_token: token }),
    });

    if (!response.ok) {
      log.warn("Token refresh failed, logging out");
      await logout();
      return false;
    }

    const data: RefreshResponse = await response.json();

    if (data.access_token) {
      localStorage.setItem(ACCESS_TOKEN_KEY, data.access_token);
      log.debug("Token refreshed successfully");
      return true;
    }

    return false;
  } catch (error) {
    log.error("Token refresh error:", error);
    return false;
  }
}

/**
 * Make authenticated API request
 *
 * Automatically adds Authorization header and handles token refresh.
 *
 * @param url - Request URL
 * @param options - Fetch options
 * @returns Response
 * @throws Error if not authenticated
 */
export async function authenticatedFetch(
  url: string,
  options: RequestInit = {}
): Promise<Response> {
  let token = getAccessToken();

  if (!token) {
    throw new Error("Not authenticated");
  }

  const headers = new Headers(options.headers);
  headers.set("Authorization", `Bearer ${token}`);

  let response = await fetch(url, { ...options, headers });

  // If 401, try to refresh token and retry
  if (response.status === 401) {
    const refreshed = await refreshToken();
    if (refreshed) {
      token = getAccessToken();
      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
        response = await fetch(url, { ...options, headers });
      }
    }
  }

  return response;
}

/**
 * Request password reset email
 *
 * @param email - Email address to send reset link
 * @returns Result (always success to prevent email enumeration)
 */
export async function forgotPassword(email: string): Promise<AuthResponse> {
  try {
    log.info("Requesting password reset for:", email);

    const response = await fetch(`${API_BASE}/auth/forgot-password`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email }),
    });

    const data = await response.json();
    return {
      success: true,
      message: data.message || "If an account exists, a reset link has been sent.",
    };
  } catch (error) {
    log.error("Forgot password error:", error);
    return {
      success: false,
      message: "Network error. Please try again.",
    };
  }
}

/**
 * Reset password with token from email
 *
 * @param token - Reset token from email link
 * @param newPassword - New password to set
 * @returns Result
 */
export async function resetPassword(
  token: string,
  newPassword: string
): Promise<AuthResponse> {
  try {
    log.info("Resetting password");

    const response = await fetch(`${API_BASE}/auth/reset-password`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ token, new_password: newPassword }),
    });

    const data = await response.json();

    if (!response.ok) {
      return {
        success: false,
        message: data.message || "Password reset failed",
      };
    }

    log.info("Password reset successful");
    return {
      success: true,
      message: data.message || "Password reset successfully",
    };
  } catch (error) {
    log.error("Reset password error:", error);
    return {
      success: false,
      message: "Network error. Please try again.",
    };
  }
}

// ============================================
// Password Validation
// ============================================

/**
 * Validate password strength
 *
 * Requirements:
 * - At least 8 characters
 * - At least one uppercase letter
 * - At least one lowercase letter
 * - At least one number
 *
 * @param password - Password to validate
 * @returns Error message or null if valid
 */
export function validatePassword(password: string): string | null {
  if (password.length < 8) {
    return "Password must be at least 8 characters long";
  }
  if (!/[A-Z]/.test(password)) {
    return "Password must contain at least one uppercase letter";
  }
  if (!/[a-z]/.test(password)) {
    return "Password must contain at least one lowercase letter";
  }
  if (!/[0-9]/.test(password)) {
    return "Password must contain at least one number";
  }
  return null;
}

/**
 * Validate email format
 *
 * @param email - Email to validate
 * @returns Error message or null if valid
 */
export function validateEmail(email: string): string | null {
  const trimmed = email.trim();
  if (trimmed.length < 5) {
    return "Email is too short";
  }
  if (!trimmed.includes("@")) {
    return "Please enter a valid email address";
  }
  const parts = trimmed.split("@");
  if (parts.length !== 2 || !parts[0] || !parts[1].includes(".")) {
    return "Please enter a valid email address";
  }
  return null;
}

// ============================================
// Exports for window.DocSign
// ============================================

/**
 * Initialize auth module on window.DocSign
 */
export function initAuthNamespace(): void {
  if (typeof window !== "undefined" && window.DocSign) {
    const docSign = window.DocSign as unknown as Record<string, unknown>;

    // Auth state
    docSign.isAuthenticated = isAuthenticated;
    docSign.getCurrentUser = getCurrentUser;
    docSign.getAccessToken = getAccessToken;
    docSign.getDocumentsRemaining = getDocumentsRemaining;

    // Auth actions
    docSign.register = register;
    docSign.login = login;
    docSign.logout = logout;
    docSign.refreshToken = refreshToken;
    docSign.forgotPassword = forgotPassword;
    docSign.resetPassword = resetPassword;

    // Authenticated fetch
    docSign.authenticatedFetch = authenticatedFetch;

    // Validation
    docSign.validatePassword = validatePassword;
    docSign.validateEmail = validateEmail;

    // Event subscription
    docSign.onAuthStateChange = onAuthStateChange;

    log.debug("Auth module initialized on window.DocSign");
  }
}
