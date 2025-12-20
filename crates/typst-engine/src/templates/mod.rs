//! Template management and embedded templates

pub mod embedded;
pub mod registry;

pub use registry::{get_template_source, list_templates, TemplateInfo};
