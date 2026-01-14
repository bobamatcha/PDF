//! Tauri command modules for the docsign desktop app.
//!
//! This module provides native desktop functionality through Tauri commands,
//! allowing the web frontend to access OS-level features.

pub mod file_dialogs;
pub mod print;
pub mod updater;

// Re-export all commands for convenient registration in main.rs
pub use file_dialogs::{open_multiple_pdfs, open_pdf_file, save_signed_pdf};
pub use print::*;
pub use updater::{check_for_updates, get_current_version, install_update};
