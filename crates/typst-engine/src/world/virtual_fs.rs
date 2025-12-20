//! Virtual filesystem for in-memory compilation
//!
//! This module provides a sandboxed virtual filesystem that allows
//! Typst to compile documents entirely in memory, without touching
//! the real filesystem.

use std::collections::HashMap;
use std::path::Path;

use typst::foundations::Bytes;
use typst::syntax::{FileId, Source, VirtualPath};

use crate::compiler::errors::ServerError;

/// A file stored in the virtual filesystem
#[derive(Debug, Clone)]
pub struct VirtualFile {
    /// The file content as bytes
    pub content: Bytes,
    /// The virtual path of this file
    pub path: VirtualPath,
    /// Unique identifier
    pub id: FileId,
}

/// A virtual filesystem for in-memory compilation
#[derive(Debug)]
pub struct VirtualFilesystem {
    /// Map of FileId to VirtualFile
    files: HashMap<FileId, VirtualFile>,
    /// Map of path string to FileId for lookups
    path_to_id: HashMap<String, FileId>,
    /// The main entry point file ID
    main_id: Option<FileId>,
}

impl VirtualFilesystem {
    /// Create a new empty virtual filesystem
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            path_to_id: HashMap::new(),
            main_id: None,
        }
    }

    /// Mount the main source file (main.typ)
    ///
    /// Returns the FileId of the mounted file
    pub fn mount_main(&mut self, content: String) -> FileId {
        let id = self.generate_file_id("/main.typ");
        let vpath = VirtualPath::new("/main.typ");

        let file = VirtualFile {
            content: content.as_bytes().into(),
            path: vpath,
            id,
        };

        self.files.insert(id, file);
        self.path_to_id.insert("/main.typ".to_string(), id);
        self.main_id = Some(id);

        id
    }

    /// Mount an additional file (e.g., an image or included template)
    pub fn mount_file(&mut self, path: &str, content: Bytes) -> Result<FileId, ServerError> {
        // Security check: reject path traversal attempts
        self.validate_path(path)?;

        let normalized = self.normalize_path(path);
        let id = self.generate_file_id(&normalized);
        let vpath = VirtualPath::new(&normalized);

        let file = VirtualFile {
            content,
            path: vpath,
            id,
        };

        self.files.insert(id, file);
        self.path_to_id.insert(normalized, id);

        Ok(id)
    }

    /// Get the main file ID
    pub fn main_id(&self) -> Option<FileId> {
        self.main_id
    }

    /// Get a source file by ID (for .typ files)
    pub fn get_source(&self, id: FileId) -> Option<Source> {
        self.files.get(&id).and_then(|file| {
            // Convert bytes to string for source files
            let text = std::str::from_utf8(&file.content).ok()?;
            Some(Source::new(id, text.to_string()))
        })
    }

    /// Get a binary file by ID (for images, etc.)
    pub fn get_file(&self, id: FileId) -> Option<&Bytes> {
        self.files.get(&id).map(|f| &f.content)
    }

    /// Resolve a path relative to a base file
    pub fn resolve_path(&self, _base: FileId, path: &str) -> Option<FileId> {
        // Normalize and look up
        let normalized = self.normalize_path(path);
        self.path_to_id.get(&normalized).copied()
    }

    /// Look up a file by path
    pub fn lookup_path(&self, path: &str) -> Option<FileId> {
        let normalized = self.normalize_path(path);
        self.path_to_id.get(&normalized).copied()
    }

    /// Generate a unique FileId for a path
    fn generate_file_id(&self, path: &str) -> FileId {
        // Use a package of None (no external packages in virtual fs)
        // and create a virtual path
        let vpath = VirtualPath::new(path);
        FileId::new(None, vpath)
    }

    /// Validate a path for security
    fn validate_path(&self, path: &str) -> Result<(), ServerError> {
        // Reject path traversal attempts
        if path.contains("..") {
            return Err(ServerError::PathSecurityViolation(
                "Path traversal with '..' is not allowed".to_string(),
            ));
        }

        // Reject absolute filesystem paths (not virtual paths)
        let p = Path::new(path);
        if p.is_absolute() && !path.starts_with('/') {
            return Err(ServerError::PathSecurityViolation(
                "Absolute filesystem paths are not allowed".to_string(),
            ));
        }

        Ok(())
    }

    /// Normalize a path to a consistent format
    fn normalize_path(&self, path: &str) -> String {
        let mut normalized = path.to_string();

        // Ensure it starts with /
        if !normalized.starts_with('/') {
            normalized = format!("/{}", normalized);
        }

        // Remove any double slashes
        while normalized.contains("//") {
            normalized = normalized.replace("//", "/");
        }

        normalized
    }
}

impl Default for VirtualFilesystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_main() {
        let mut fs = VirtualFilesystem::new();
        let id = fs.mount_main("Hello, World!".to_string());

        assert!(fs.main_id().is_some());
        assert_eq!(fs.main_id(), Some(id));

        let source = fs.get_source(id).unwrap();
        assert!(source.text().contains("Hello"));
    }

    #[test]
    fn test_path_traversal_blocked() {
        let mut fs = VirtualFilesystem::new();
        let result = fs.mount_file("../../../etc/passwd", Bytes::from_static(&[]));

        assert!(result.is_err());
        assert!(matches!(result, Err(ServerError::PathSecurityViolation(_))));
    }

    #[test]
    fn test_mount_asset() {
        let mut fs = VirtualFilesystem::new();
        fs.mount_main("test".to_string());

        let content: Bytes = vec![0x89u8, 0x50, 0x4E, 0x47].into(); // PNG header
        let id = fs.mount_file("images/logo.png", content.clone()).unwrap();

        let retrieved = fs.get_file(id).unwrap();
        assert_eq!(retrieved, &content);
    }
}
