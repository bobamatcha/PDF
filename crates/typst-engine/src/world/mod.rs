//! Typst World trait implementation for in-memory compilation

pub mod fonts;
pub mod virtual_fs;
pub mod virtual_world;

pub use fonts::FontCache;
pub use virtual_fs::VirtualFilesystem;
pub use virtual_world::VirtualWorld;
