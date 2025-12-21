//! Path helpers for finding project files

/// Get workspace root path
pub fn workspace_root() -> std::path::PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| {
            std::path::PathBuf::from(d)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}
