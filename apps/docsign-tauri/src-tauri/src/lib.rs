//! Tauri docsign desktop application.
//!
//! This crate provides the native backend for the docsign desktop app,
//! including printing support, native file dialogs, system tray integration,
//! and other platform-specific features.

pub mod commands;
pub mod tray;

use commands::{
    check_for_updates, get_available_printers, get_current_version, install_update,
    open_multiple_pdfs, open_pdf_file, print_pdf, print_to_printer, save_signed_pdf,
};
use tauri::Manager;
use tray::{hide_to_tray, show_main_window, update_recent_documents};

/// Register all Tauri commands and run the application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // Setup application including system tray
        .setup(|app| {
            // Initialize the system tray
            if let Err(e) = tray::create_tray(app.handle()) {
                eprintln!("Failed to create system tray: {}", e);
                // Continue without tray - not a fatal error
            }

            // Get the main window and set up close handler
            if let Some(window) = app.get_webview_window("main") {
                // Set up close handler to minimize to tray instead of closing
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // Prevent the window from closing
                        api.prevent_close();
                        // Hide to tray instead
                        let _ = window_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Print commands
            print_pdf,
            get_available_printers,
            print_to_printer,
            // File dialog commands
            open_pdf_file,
            save_signed_pdf,
            open_multiple_pdfs,
            // Tray/window management commands
            show_main_window,
            hide_to_tray,
            update_recent_documents,
            // Auto-update commands
            check_for_updates,
            install_update,
            get_current_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
