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

/** API base URL - Cloudflare Worker (production) */
const API_BASE = "https://api.getsignatures.org";

// ============================================
// Types
// ============================================

/** User tier for rate limiting */
export type UserTier = "free" | "pro";

/** Public user info (returned from login) */
export interface User {
  id: string;
  email: string;
  first_name: string;
  middle_initial?: string;
  last_name: string;
  tier: UserTier;
  /** Weekly documents remaining (renamed from daily) */
  weekly_documents_remaining: number;
  /** Backward compat alias */
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
  /** True if account exists but needs email verification */
  needs_verification?: boolean;
}

/** Login response */
export interface LoginResponse {
  success: boolean;
  access_token?: string;
  refresh_token?: string;
  expires_in?: number;
  user?: User;
  error?: string;
  /** True if login failed due to unverified email */
  needs_verification?: boolean;
  /** Email for resend verification flow */
  email?: string;
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
  /** Seconds until rate limit resets (for rate limit errors) */
  retry_after_seconds?: number;
}

/**
 * Format a user-friendly error message from API response
 *
 * Best practices (WCAG 3.3.1, 3.3.3):
 * - Be specific about what went wrong
 * - Provide actionable guidance on how to fix it
 * - Never blame the user for server-side issues
 * - Use plain language, no jargon
 */
function formatErrorMessage(data: Record<string, unknown>, fallback: string): string {
  // Handle rate limit responses with specific, helpful messaging
  if (data.retry_after_seconds && typeof data.retry_after_seconds === "number") {
    const seconds = data.retry_after_seconds as number;
    if (seconds >= 3600) {
      const hours = Math.ceil(seconds / 3600);
      return `You've reached the hourly limit for this action. You can try again in ${hours} hour${hours > 1 ? "s" : ""}.`;
    } else if (seconds >= 60) {
      const minutes = Math.ceil(seconds / 60);
      return `You've reached the limit for this action. You can try again in ${minutes} minute${minutes > 1 ? "s" : ""}.`;
    } else {
      return `Please wait ${seconds} seconds before trying again.`;
    }
  }
  // Check both message and error fields - prefer message as it's usually more descriptive
  return (data.message as string) || (data.error as string) || fallback;
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
 * Get remaining documents for this week
 */
export function getDocumentsRemaining(): number {
  const user = getCurrentUser();
  if (!user) return 0;
  return user.weekly_documents_remaining ?? user.daily_documents_remaining ?? 0;
}

// ============================================
// API Calls
// ============================================

/** Registration options with name parts */
export interface RegisterOptions {
  email: string;
  password: string;
  first_name: string;
  middle_initial?: string;
  last_name: string;
}

/**
 * Register a new user account
 *
 * @param options - Registration options with name parts
 * @returns Registration result
 */
export async function register(options: RegisterOptions): Promise<RegisterResponse> {
  try {
    log.info("Registering new user:", options.email);

    const response = await fetch(`${API_BASE}/auth/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: options.email,
        password: options.password,
        first_name: options.first_name,
        middle_initial: options.middle_initial || null,
        last_name: options.last_name,
      }),
    });

    const data = await response.json();

    if (!response.ok) {
      // Handle both message and error fields - API uses message for most errors,
      // but rate limit responses use error field
      const errorMsg = data.message || data.error || "Registration failed";
      log.warn("Registration failed:", errorMsg);
      return {
        success: false,
        message: errorMsg,
        error: errorMsg,
        // CRITICAL: Pass through needs_verification so UI can show resend button
        needs_verification: data.needs_verification,
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
      message: "Could not connect to the server. This may be a temporary issue—please try again in a moment.",
      error: String(error),
    };
  }
}

/**
 * Check if an email is already registered (email-first UX)
 *
 * @param email - Email address to check
 * @returns Whether email exists and if it's verified
 */
export interface CheckEmailResponse {
  exists: boolean;
  verified: boolean;
}

export async function checkEmail(email: string): Promise<CheckEmailResponse> {
  try {
    log.info("Checking email:", email);

    const response = await fetch(`${API_BASE}/auth/check-email`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email }),
    });

    const data = await response.json();

    if (!response.ok) {
      log.warn("Check email failed:", response.status);
      return { exists: false, verified: false };
    }

    return {
      exists: data.exists ?? false,
      verified: data.verified ?? false,
    };
  } catch (error) {
    log.error("Check email error:", error);
    return { exists: false, verified: false };
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
        // CRITICAL: Pass through needs_verification and email so UI can show resend button
        needs_verification: data.needs_verification,
        email: data.email,
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
      error: "Could not connect to the server. This may be a temporary issue—please try again in a moment.",
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
 * @returns Result with explicit error if email not found
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

    if (!response.ok) {
      const errorMsg = formatErrorMessage(data, "Unable to send reset email. Please try again.");
      log.warn("Forgot password failed:", errorMsg);
      return {
        success: false,
        message: errorMsg,
        retry_after_seconds: data.retry_after_seconds as number | undefined,
      };
    }

    return {
      success: true,
      message: data.message || "A password reset link has been sent to your email.",
    };
  } catch (error) {
    log.error("Forgot password error:", error);
    return {
      success: false,
      message: "Could not connect to the server. This may be a temporary issue—please try again in a moment.",
    };
  }
}

/**
 * Resend email verification link
 *
 * Use this when a user needs a new verification email (e.g., after login failure
 * due to unverified email, or re-registration with existing unverified account).
 *
 * @param email - Email address to send verification link
 * @returns Result with success/error message
 */
export async function resendVerification(email: string): Promise<AuthResponse> {
  try {
    log.info("Requesting verification resend for:", email);

    const response = await fetch(`${API_BASE}/auth/resend-verification`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email }),
    });

    const data = await response.json();

    if (!response.ok) {
      const errorMsg = formatErrorMessage(data, "Unable to send verification email. Please try again.");
      log.warn("Resend verification failed:", errorMsg);
      return {
        success: false,
        message: errorMsg,
        retry_after_seconds: data.retry_after_seconds as number | undefined,
      };
    }

    return {
      success: true,
      message: data.message || "Verification email sent! Please check your inbox.",
    };
  } catch (error) {
    log.error("Resend verification error:", error);
    return {
      success: false,
      message: "Could not connect to the server. This may be a temporary issue—please try again in a moment.",
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
      const errorMsg = formatErrorMessage(data, "Password reset failed. The link may have expired.");
      return {
        success: false,
        message: errorMsg,
        retry_after_seconds: data.retry_after_seconds as number | undefined,
      };
    }

    log.info("Password reset successful");
    return {
      success: true,
      message: data.message || "Password reset successfully! You can now sign in.",
    };
  } catch (error) {
    log.error("Reset password error:", error);
    return {
      success: false,
      message: "Could not connect to the server. This may be a temporary issue—please try again in a moment.",
    };
  }
}

/** Profile update options */
export interface UpdateProfileOptions {
  first_name?: string;
  middle_initial?: string;
  last_name?: string;
}

/** Profile update response */
export interface UpdateProfileResponse {
  success: boolean;
  message: string;
  user?: User;
}

/**
 * Update user profile (name parts)
 *
 * @param options - Profile fields to update
 * @returns Updated user info
 */
export async function updateProfile(options: UpdateProfileOptions): Promise<UpdateProfileResponse> {
  try {
    log.info("Updating profile");

    const response = await authenticatedFetch(`${API_BASE}/auth/profile`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(options),
    });

    const data = await response.json();

    if (!response.ok) {
      const errorMsg = formatErrorMessage(data, "Failed to update profile.");
      log.warn("Profile update failed:", errorMsg);
      return {
        success: false,
        message: errorMsg,
      };
    }

    // Update stored user if returned
    if (data.user) {
      storeUser(data.user);
      emitAuthStateChange();
    }

    log.info("Profile updated successfully");
    return {
      success: true,
      message: data.message || "Profile updated successfully.",
      user: data.user,
    };
  } catch (error) {
    log.error("Profile update error:", error);
    return {
      success: false,
      message: "Could not connect to the server. This may be a temporary issue—please try again in a moment.",
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
    docSign.resendVerification = resendVerification;
    docSign.checkEmail = checkEmail;
    docSign.updateProfile = updateProfile;

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
