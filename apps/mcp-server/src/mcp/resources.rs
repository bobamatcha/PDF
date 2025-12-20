//! MCP Resource providers

use super::protocol::{Resource, ResourceContent};
use crate::compiler::errors::ServerError;
use crate::templates;
use crate::world::fonts::global_font_cache;

/// Get all resource definitions
pub fn get_resource_definitions() -> Vec<Resource> {
    let mut resources = vec![];

    // Template resources
    for template in templates::list_templates() {
        resources.push(Resource {
            uri: template.uri.clone(),
            name: template.name.clone(),
            description: Some(template.description.clone()),
            mime_type: Some("text/x-typst".to_string()),
        });
    }

    // Font list resource
    resources.push(Resource {
        uri: "typst://fonts".to_string(),
        name: "Available Fonts".to_string(),
        description: Some("List of all fonts available for use in templates".to_string()),
        mime_type: Some("application/json".to_string()),
    });

    resources
}

/// Read a resource by URI
pub fn read_resource(uri: &str) -> Result<ResourceContent, ServerError> {
    if uri.starts_with("typst://templates/") {
        let name = uri
            .strip_prefix("typst://templates/")
            .ok_or_else(|| ServerError::ResourceNotFound(uri.to_string()))?;

        let content = templates::get_template_source(name)?;

        Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type: Some("text/x-typst".to_string()),
            text: Some(content),
            blob: None,
        })
    } else if uri == "typst://fonts" {
        let fonts = global_font_cache().list_font_families();
        let json = serde_json::to_string_pretty(&fonts)?;

        Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            text: Some(json),
            blob: None,
        })
    } else {
        Err(ServerError::ResourceNotFound(uri.to_string()))
    }
}
