//! System tray integration for GetSignatures desktop app.
//!
//! Provides system tray icon, menu, and event handling for:
//! - Opening/focusing the main window
//! - Accessing recent documents
//! - Quitting the application

use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

/// Menu item IDs for the system tray
pub mod menu_ids {
    pub const OPEN: &str = "open";
    pub const RECENT_DOCS: &str = "recent_docs";
    pub const NO_RECENT: &str = "no_recent";
    pub const QUIT: &str = "quit";
}

/// Creates and initializes the system tray for the application.
///
/// # Arguments
/// * `app` - The Tauri application handle
///
/// # Returns
/// Result indicating success or failure of tray initialization
pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    // Load the tray icon
    let icon = load_tray_icon()?;

    // Build the tray menu
    let menu = build_tray_menu(app)?;

    // Create the tray icon with menu
    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("GetSignatures")
        .on_tray_icon_event(|tray, event| {
            handle_tray_icon_event(tray.app_handle(), event);
        })
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id.as_ref());
        })
        .build(app)?;

    Ok(())
}

/// Loads the tray icon from embedded resources.
fn load_tray_icon() -> Result<Image<'static>, Box<dyn std::error::Error>> {
    // Load icon from embedded bytes (PNG format)
    // The icon is embedded at compile time from src-tauri/icons/tray-icon.png
    let icon_bytes = include_bytes!("../icons/tray-icon.png");
    let icon = Image::from_bytes(icon_bytes)?;
    Ok(icon)
}

/// Builds the system tray menu with all menu items.
fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>, Box<dyn std::error::Error>> {
    // Create menu items
    let open_item = MenuItem::with_id(
        app,
        menu_ids::OPEN,
        "Open GetSignatures",
        true,
        None::<&str>,
    )?;

    // Create recent documents submenu
    let recent_submenu = build_recent_docs_submenu(app)?;

    // Create separator and quit items
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, menu_ids::QUIT, "Quit", true, None::<&str>)?;

    // Build the complete menu
    let menu = Menu::with_items(app, &[&open_item, &recent_submenu, &separator, &quit_item])?;

    Ok(menu)
}

/// Builds the "Recent Documents" submenu.
///
/// This submenu shows recently opened documents, or a placeholder
/// message if no recent documents exist.
fn build_recent_docs_submenu<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Submenu<R>, Box<dyn std::error::Error>> {
    // TODO: In a full implementation, this would query IndexedDB or local storage
    // for recently opened documents. For now, we show a placeholder.
    let no_recent = MenuItem::with_id(
        app,
        menu_ids::NO_RECENT,
        "No recent documents",
        false,
        None::<&str>,
    )?;

    let submenu = Submenu::with_items(app, "Recent Documents", true, &[&no_recent])?;

    Ok(submenu)
}

/// Handles tray icon events (clicks, double-clicks, etc.).
fn handle_tray_icon_event<R: Runtime>(app: &AppHandle<R>, event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => {
            // Single left click - show/focus main window
            show_main_window_internal(app);
        }
        TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } => {
            // Double-click left - show/focus main window
            show_main_window_internal(app);
        }
        _ => {
            // Other events (right-click menu is handled automatically)
        }
    }
}

/// Handles menu item click events.
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, menu_id: &str) {
    match menu_id {
        menu_ids::OPEN => {
            show_main_window_internal(app);
        }
        menu_ids::QUIT => {
            // Exit the application
            app.exit(0);
        }
        id if id.starts_with("recent_") => {
            // Handle recent document selection
            // TODO: Open the specific document
            let doc_id = id.strip_prefix("recent_").unwrap_or("");
            println!("Opening recent document: {}", doc_id);
        }
        _ => {
            // Unknown menu item
        }
    }
}

/// Internal function to show and focus the main window.
fn show_main_window_internal<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        // Unminimize if minimized
        let _ = window.unminimize();
        // Show the window if hidden
        let _ = window.show();
        // Bring to front and focus
        let _ = window.set_focus();
    }
}

// ============================================================================
// Tauri Commands for window management
// ============================================================================

/// Shows and focuses the main window.
///
/// This command can be invoked from the frontend to bring the window to the foreground.
#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.unminimize().map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

/// Hides the main window to the system tray.
///
/// The application continues running in the background and can be
/// restored via the tray icon.
#[tauri::command]
pub fn hide_to_tray(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

/// Updates the recent documents submenu with new entries.
///
/// # Arguments
/// * `app` - The application handle
/// * `documents` - List of (id, name) tuples for recent documents
#[tauri::command]
pub fn update_recent_documents(
    app: AppHandle,
    documents: Vec<(String, String)>,
) -> Result<(), String> {
    // In Tauri v2, we would need to rebuild the menu to update items
    // This is a simplified implementation that logs the update
    // A full implementation would store the documents and rebuild the tray menu

    if documents.is_empty() {
        println!("Recent documents cleared");
    } else {
        println!("Recent documents updated: {:?}", documents);
    }

    // TODO: Implement full menu rebuilding
    // This would involve:
    // 1. Getting the tray icon by ID
    // 2. Building a new menu with the updated recent documents
    // 3. Setting the new menu on the tray icon

    let _ = app; // Suppress unused warning until full implementation
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_ids_are_unique() {
        let ids = [
            menu_ids::OPEN,
            menu_ids::RECENT_DOCS,
            menu_ids::NO_RECENT,
            menu_ids::QUIT,
        ];
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "Menu IDs must be unique");
    }
}
