//! VirtualWorld implementation of the Typst World trait
//!
//! This module provides an in-memory implementation of the Typst World trait,
//! allowing documents to be compiled without touching the filesystem.

use std::collections::HashMap;

use chrono::{Datelike, Timelike, Utc};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Array, Bytes, Datetime, Dict, Value};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

use super::fonts::{global_font_cache, FontCache};
use super::virtual_fs::VirtualFilesystem;
use crate::compiler::errors::ServerError;

/// A virtual world for in-memory Typst compilation
pub struct VirtualWorld {
    /// Virtual filesystem containing source and assets
    filesystem: VirtualFilesystem,
    /// Reference to the global font cache
    font_cache: &'static FontCache,
    /// System inputs (accessible via sys.inputs in Typst)
    inputs: Dict,
    /// Fixed timestamp for deterministic builds
    time: chrono::DateTime<Utc>,
    /// Pre-hashed standard library
    library: LazyHash<Library>,
}

impl VirtualWorld {
    /// Create a new VirtualWorld with the given source and inputs
    pub fn new(
        source: String,
        inputs: HashMap<String, serde_json::Value>,
        assets: HashMap<String, Bytes>,
    ) -> Result<Self, ServerError> {
        let mut filesystem = VirtualFilesystem::new();

        // Mount the main source file
        filesystem.mount_main(source);

        // Mount any provided assets
        for (path, content) in assets {
            filesystem.mount_file(&path, content)?;
        }

        // Convert JSON inputs to Typst Dict
        let inputs_dict = Self::convert_inputs(inputs)?;

        // Build the library with inputs
        let library = Self::build_library(inputs_dict.clone());

        Ok(Self {
            filesystem,
            font_cache: global_font_cache(),
            inputs: inputs_dict,
            time: Utc::now(),
            library: LazyHash::new(library),
        })
    }

    /// Build the Typst standard library with sys.inputs configured
    fn build_library(inputs: Dict) -> Library {
        Library::builder().with_inputs(inputs).build()
    }

    /// Convert JSON values to Typst Dict
    fn convert_inputs(inputs: HashMap<String, serde_json::Value>) -> Result<Dict, ServerError> {
        let mut dict = Dict::new();

        for (key, value) in inputs {
            let typst_value = Self::json_to_typst_value(&value)?;
            dict.insert(key.into(), typst_value);
        }

        Ok(dict)
    }

    /// Convert a JSON value to a Typst Value
    fn json_to_typst_value(json: &serde_json::Value) -> Result<Value, ServerError> {
        match json {
            serde_json::Value::Null => Ok(Value::None),
            serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::Int(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Value::Float(f))
                } else {
                    Err(ServerError::InvalidArgument(format!(
                        "Invalid number: {}",
                        n
                    )))
                }
            }
            serde_json::Value::String(s) => Ok(Value::Str(s.as_str().into())),
            serde_json::Value::Array(arr) => {
                let items: Vec<Value> = arr
                    .iter()
                    .map(Self::json_to_typst_value)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::Array(Array::from(items.as_slice())))
            }
            serde_json::Value::Object(obj) => {
                let mut dict = Dict::new();
                for (k, v) in obj {
                    let typst_value = Self::json_to_typst_value(v)?;
                    dict.insert(k.as_str().into(), typst_value);
                }
                Ok(Value::Dict(dict))
            }
        }
    }

    /// Get the inputs dictionary
    pub fn inputs(&self) -> &Dict {
        &self.inputs
    }

    /// Get the filesystem
    pub fn filesystem(&self) -> &VirtualFilesystem {
        &self.filesystem
    }
}

impl World for VirtualWorld {
    /// Get the standard library
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    /// Get the font book
    fn book(&self) -> &LazyHash<FontBook> {
        // FontBook is not LazyHash in our cache, so we need to create one
        // This is a workaround - ideally the font cache would store LazyHash<FontBook>
        static BOOK: std::sync::OnceLock<LazyHash<FontBook>> = std::sync::OnceLock::new();
        BOOK.get_or_init(|| LazyHash::new(self.font_cache.book().clone()))
    }

    /// Get the main source file ID
    fn main(&self) -> FileId {
        self.filesystem.main_id().expect("Main file not mounted")
    }

    /// Get a source file by ID
    fn source(&self, id: FileId) -> FileResult<Source> {
        self.filesystem
            .get_source(id)
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    /// Get a binary file by ID
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.filesystem
            .get_file(id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    /// Get a font by index
    fn font(&self, index: usize) -> Option<Font> {
        self.font_cache.font(index)
    }

    /// Get the current date/time
    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let offset_hours = offset.unwrap_or(0);
        let adjusted = self.time + chrono::Duration::hours(offset_hours);

        Datetime::from_ymd_hms(
            adjusted.year(),
            adjusted.month() as u8,
            adjusted.day() as u8,
            adjusted.hour() as u8,
            adjusted.minute() as u8,
            adjusted.second() as u8,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_world_creation() {
        let source = "Hello, World!".to_string();
        let world = VirtualWorld::new(source, HashMap::new(), HashMap::new());

        assert!(world.is_ok());
        let world = world.unwrap();

        // Check that we can get the main source
        let main_id = world.main();
        let source = world.source(main_id);
        assert!(source.is_ok());
    }

    #[test]
    fn test_input_conversion() {
        let mut inputs = HashMap::new();
        inputs.insert("name".to_string(), serde_json::json!("Alice"));
        inputs.insert("age".to_string(), serde_json::json!(30));
        inputs.insert("active".to_string(), serde_json::json!(true));

        let world = VirtualWorld::new("test".to_string(), inputs, HashMap::new()).unwrap();

        // Verify inputs are accessible
        let inputs_dict = world.inputs();
        assert!(!inputs_dict.is_empty());
    }

    #[test]
    fn test_nested_input_conversion() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "user".to_string(),
            serde_json::json!({
                "name": "Bob",
                "tags": ["admin", "user"]
            }),
        );

        let world = VirtualWorld::new("test".to_string(), inputs, HashMap::new()).unwrap();

        let inputs_dict = world.inputs();
        assert!(inputs_dict.contains("user"));
    }

    #[test]
    fn test_today_function() {
        let world = VirtualWorld::new("test".to_string(), HashMap::new(), HashMap::new()).unwrap();

        let datetime = world.today(None);
        assert!(datetime.is_some());
    }
}
