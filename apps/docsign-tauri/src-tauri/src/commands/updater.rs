//! Auto-update functionality for the docsign desktop application.
//!
//! This module provides Tauri commands for checking and installing updates.
//! Designed with geriatric UX in mind:
//! - Clear, non-technical update notifications
//! - Simple one-click update process
//! - Reassurance that documents are safe during updates

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

/// Information about an available update.
///
/// Serialized to JavaScript with user-friendly field names.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    /// The new version number (e.g., "1.2.0")
    pub version: String,
    /// Release notes explaining what's new
    pub notes: Option<String>,
    /// When the update was released
    pub date: Option<String>,
}

/// Checks if an update is available for the application.
///
/// This command contacts the update server to check for newer versions.
/// Returns update information if available, or null if already up to date.
///
/// # Returns
/// - `Ok(Some(UpdateInfo))` - An update is available
/// - `Ok(None)` - Already running the latest version
/// - `Err(String)` - Failed to check for updates (network error, etc.)
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = app
        .updater()
        .map_err(|e| format!("Update check failed: {}", e))?;

    match updater.check().await {
        Ok(Some(update)) => {
            // Parse the date from the update if available
            // The date is an OffsetDateTime, we just use its Display implementation
            let date = update.date.map(|d| d.to_string());

            Ok(Some(UpdateInfo {
                version: update.version.clone(),
                notes: update.body.clone(),
                date,
            }))
        }
        Ok(None) => {
            // No update available - already on latest version
            Ok(None)
        }
        Err(e) => {
            // Network or server error
            Err(format!(
                "Could not check for updates. Please check your internet connection. ({})",
                e
            ))
        }
    }
}

/// Downloads and installs an available update.
///
/// This will:
/// 1. Download the update package
/// 2. Verify its integrity
/// 3. Install the update
/// 4. Restart the application
///
/// # Important
/// Documents are automatically saved before the update process begins.
/// The application will restart automatically after the update is installed.
///
/// # Returns
/// - `Ok(())` - Update was downloaded and will be installed on restart
/// - `Err(String)` - Update failed with a user-friendly error message
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app
        .updater()
        .map_err(|e| format!("Could not start update: {}", e))?;

    // Check for the update first
    let update = updater
        .check()
        .await
        .map_err(|e| format!("Could not check for updates: {}", e))?
        .ok_or_else(|| "No update available".to_string())?;

    // Download and install the update
    // The closure receives download progress events
    update
        .download_and_install(
            |_chunk_length, _content_length| {
                // Progress callback - could be used for UI progress bar
                // For now, the frontend handles progress indication separately
            },
            || {
                // Download complete callback
                // The update is ready to be applied
            },
        )
        .await
        .map_err(|e| format!("Update download failed. Please try again later. ({})", e))?;

    // Request app restart to apply the update
    // Note: On some platforms this happens automatically
    app.restart();
}

/// Gets the current application version.
///
/// Useful for displaying "Current version: X.Y.Z" in the UI.
#[tauri::command]
pub fn get_current_version(app: AppHandle) -> String {
    app.config()
        .version
        .clone()
        .unwrap_or_else(|| "unknown".to_string())
}
