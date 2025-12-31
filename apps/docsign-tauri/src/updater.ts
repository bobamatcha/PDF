/**
 * Auto-update module for the GetSignatures desktop application.
 *
 * This module provides TypeScript bindings for the native auto-update commands.
 * Designed with geriatric UX in mind:
 * - Clear, reassuring notifications
 * - Simple one-click update process
 * - Progress indication during download
 * - Safety messages about document preservation
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * Information about an available update.
 */
export interface UpdateInfo {
  /** The new version number (e.g., "1.2.0") */
  version: string;
  /** Release notes describing what's new (optional) */
  notes?: string;
  /** When the update was released (optional, formatted date string) */
  date?: string;
}

/**
 * Error thrown when an update operation fails.
 */
export class UpdateError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "UpdateError";
  }
}

/**
 * Checks if a newer version of the application is available.
 *
 * This contacts the update server to check for newer versions.
 * Safe to call frequently - it's a lightweight check.
 *
 * @returns UpdateInfo if an update is available, null if already up to date.
 * @throws UpdateError if the check fails (e.g., no internet connection).
 *
 * @example
 * ```typescript
 * const update = await checkForUpdates();
 * if (update) {
 *   console.log(`Update available: ${update.version}`);
 *   console.log(`Release notes: ${update.notes}`);
 * } else {
 *   console.log('You have the latest version!');
 * }
 * ```
 */
export async function checkForUpdates(): Promise<UpdateInfo | null> {
  try {
    return await invoke<UpdateInfo | null>("check_for_updates");
  } catch (error) {
    throw new UpdateError(formatUpdateError(error));
  }
}

/**
 * Downloads and installs an available update.
 *
 * This will:
 * 1. Download the update package (may take a few minutes)
 * 2. Verify the download is safe and complete
 * 3. Install the update
 * 4. Restart the application automatically
 *
 * IMPORTANT: Make sure to save any work before calling this function.
 * The application will restart after the update is installed.
 *
 * @throws UpdateError if the update fails.
 *
 * @example
 * ```typescript
 * try {
 *   // Show progress indicator to user
 *   showProgress('Downloading update...');
 *
 *   await installUpdate();
 *   // Note: This line won't be reached - app restarts after update
 * } catch (error) {
 *   showError('Update failed. Please try again later.');
 * }
 * ```
 */
export async function installUpdate(): Promise<void> {
  try {
    await invoke<void>("install_update");
  } catch (error) {
    throw new UpdateError(formatUpdateError(error));
  }
}

/**
 * Gets the currently installed version of the application.
 *
 * Useful for displaying "Current version: X.Y.Z" in settings or about dialogs.
 *
 * @returns The current version string (e.g., "0.1.0").
 *
 * @example
 * ```typescript
 * const version = await getCurrentVersion();
 * console.log(`GetSignatures version ${version}`);
 * ```
 */
export async function getCurrentVersion(): Promise<string> {
  try {
    return await invoke<string>("get_current_version");
  } catch {
    return "unknown";
  }
}

/**
 * Checks if running in a Tauri environment where updates are supported.
 *
 * Updates are only available in the desktop app, not the web version.
 *
 * @returns true if updates are supported, false otherwise.
 */
export function isUpdateSupported(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

// ============================================================================
// Geriatric UX Helper Functions
// ============================================================================

/**
 * User-friendly message strings for update UI.
 * Designed for clarity and reassurance.
 */
export const UpdateMessages = {
  /** Shown when an update is available */
  updateAvailable: (version: string) =>
    `A new version (${version}) is available. Would you like to update now?`,

  /** Shown during download */
  downloading: "Downloading update... This may take a few minutes.",

  /** Shown when download is complete */
  installing: "Installing update... The app will restart shortly.",

  /** Reassurance about document safety */
  documentsSafe:
    "Your documents are safe. They will still be here after the update.",

  /** When already on latest version */
  upToDate: "You have the latest version. No update needed.",

  /** When update check fails */
  checkFailed:
    "Could not check for updates. Please check your internet connection.",

  /** When update installation fails */
  installFailed: "Update failed. Please try again later.",

  /** Prompt before update */
  updatePrompt:
    "A new version is available with improvements and fixes. Updating takes just a few minutes.",

  /** Button text for update */
  updateButton: "Update Now",

  /** Button text for later */
  laterButton: "Remind Me Later",

  /** Button text for skip */
  skipButton: "Skip This Version",
} as const;

/**
 * Formats release notes for display to elderly users.
 *
 * Simplifies technical release notes into user-friendly language.
 *
 * @param notes - Raw release notes from the update server
 * @returns Simplified, user-friendly summary
 */
export function formatReleaseNotes(notes: string | undefined): string {
  if (!notes) {
    return "This update includes improvements and fixes.";
  }

  // Keep it simple - just show the notes, but clean them up
  return notes
    .replace(/#+\s*/g, "") // Remove markdown headers
    .replace(/\*\*/g, "") // Remove bold markers
    .replace(/`[^`]*`/g, "") // Remove code blocks
    .replace(/\n{3,}/g, "\n\n") // Collapse multiple newlines
    .trim();
}

/**
 * Checks for updates and returns a user-friendly status.
 *
 * This is a convenience function that wraps checkForUpdates with
 * geriatric-friendly error handling and messaging.
 *
 * @returns An object with status information suitable for UI display
 */
export async function checkUpdateStatus(): Promise<{
  hasUpdate: boolean;
  message: string;
  updateInfo?: UpdateInfo;
}> {
  if (!isUpdateSupported()) {
    return {
      hasUpdate: false,
      message: "Updates are only available in the desktop app.",
    };
  }

  try {
    const update = await checkForUpdates();

    if (update) {
      return {
        hasUpdate: true,
        message: UpdateMessages.updateAvailable(update.version),
        updateInfo: update,
      };
    }

    return {
      hasUpdate: false,
      message: UpdateMessages.upToDate,
    };
  } catch {
    return {
      hasUpdate: false,
      message: UpdateMessages.checkFailed,
    };
  }
}

/**
 * Formats an error from the update backend into a user-friendly message.
 */
function formatUpdateError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "object" && error !== null && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "An unexpected error occurred during the update";
}
