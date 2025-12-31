//! Field Export Module
//!
//! This module provides WASM-exposed functions to flatten placed fields
//! (text, signatures, checkboxes, etc.) into the PDF using pdfjoin-core.

use pdfjoin_core::apply_operations::apply_operations;
use pdfjoin_core::operations::{EditOperation, OperationLog, PdfRect, TextStyle};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Field type enum matching TypeScript FieldType
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Signature,
    Initials,
    Checkbox,
    Date,
}

/// Field style matching TypeScript FieldStyle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldStyle {
    pub font_size: f64,
    pub font_family: String,
    pub is_bold: bool,
    pub is_italic: bool,
    pub color: String,
}

/// Placed field data matching TypeScript PlacedField
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacedField {
    pub id: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub page_num: u32,
    // PDF coordinates (for export)
    pub pdf_x: f64,
    pub pdf_y: f64,
    pub pdf_width: f64,
    pub pdf_height: f64,
    // Content
    pub value: String,
    pub style: FieldStyle,
    // For checkbox
    #[serde(default)]
    pub checked: Option<bool>,
    // For signature - base64 encoded image
    #[serde(default)]
    pub signature_data: Option<String>,
}

/// Export PDF with flattened fields
///
/// Takes the original PDF bytes and an array of placed fields,
/// returns a new PDF with the fields embedded into it.
///
/// # Arguments
/// * `pdf_bytes` - Original PDF as Uint8Array
/// * `fields_json` - JSON array of PlacedField objects
///
/// # Returns
/// New PDF bytes as Uint8Array with fields flattened into the document
#[wasm_bindgen]
pub fn export_pdf_with_fields(pdf_bytes: &[u8], fields_json: &str) -> Result<Vec<u8>, JsValue> {
    console_error_panic_hook::set_once();

    // Parse fields from JSON
    let fields: Vec<PlacedField> = serde_json::from_str(fields_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse fields: {}", e)))?;

    // Convert fields to pdfjoin-core operations
    let mut log = OperationLog::new();

    for field in fields.iter() {
        let rect = PdfRect {
            x: field.pdf_x,
            y: field.pdf_y,
            width: field.pdf_width,
            height: field.pdf_height,
        };

        let operation = match field.field_type {
            FieldType::Text | FieldType::Date | FieldType::Initials => {
                // Skip empty text fields
                if field.value.is_empty() {
                    continue;
                }

                let style = TextStyle {
                    font_size: field.style.font_size,
                    color: field.style.color.clone(),
                    font_name: Some(field.style.font_family.clone()),
                    is_bold: field.style.is_bold,
                    is_italic: field.style.is_italic,
                };

                EditOperation::AddText {
                    id: 0, // Will be assigned by OperationLog::add
                    page: field.page_num,
                    rect: rect.clone(),
                    text: field.value.clone(),
                    style,
                }
            }
            FieldType::Checkbox => {
                // For checkboxes, add text "X" or empty based on checked state
                let is_checked = field.checked.unwrap_or(false);
                if !is_checked {
                    continue; // Skip unchecked boxes
                }

                let style = TextStyle {
                    font_size: field.pdf_height * 0.7, // Scale X to fit box
                    color: "#000000".to_string(),
                    font_name: Some("sans-serif".to_string()),
                    is_bold: true,
                    is_italic: false,
                };

                EditOperation::AddText {
                    id: 0,
                    page: field.page_num,
                    rect: rect.clone(),
                    text: "X".to_string(),
                    style,
                }
            }
            FieldType::Signature => {
                // For signatures with image data, use AddImage
                if let Some(ref sig_data) = field.signature_data {
                    if !sig_data.is_empty() {
                        // Determine format from data prefix
                        let format = if sig_data.starts_with("/9j/") {
                            "jpeg".to_string()
                        } else {
                            "png".to_string() // Default to PNG
                        };

                        EditOperation::AddImage {
                            id: 0,
                            page: field.page_num,
                            rect: rect.clone(),
                            image_data: sig_data.clone(),
                            format,
                        }
                    } else {
                        // Empty signature - add placeholder text
                        let style = TextStyle {
                            font_size: 12.0,
                            color: "#666666".to_string(),
                            font_name: Some("serif".to_string()),
                            is_bold: false,
                            is_italic: true,
                        };
                        EditOperation::AddText {
                            id: 0,
                            page: field.page_num,
                            rect: rect.clone(),
                            text: "[Signature]".to_string(),
                            style,
                        }
                    }
                } else {
                    // No signature data - add placeholder
                    let style = TextStyle {
                        font_size: 12.0,
                        color: "#666666".to_string(),
                        font_name: Some("serif".to_string()),
                        is_bold: false,
                        is_italic: true,
                    };
                    EditOperation::AddText {
                        id: 0,
                        page: field.page_num,
                        rect: rect.clone(),
                        text: "[Signature]".to_string(),
                        style,
                    }
                }
            }
        };

        log.add(operation);
    }

    // Apply operations to PDF
    let result_bytes = apply_operations(pdf_bytes, &log)
        .map_err(|e| JsValue::from_str(&format!("Failed to apply operations: {}", e)))?;

    Ok(result_bytes)
}

/// Validate fields before export
///
/// Returns JSON with validation results (any errors or warnings)
#[wasm_bindgen]
pub fn validate_fields_for_export(fields_json: &str) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    let fields: Vec<PlacedField> = serde_json::from_str(fields_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse fields: {}", e)))?;

    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for field in &fields {
        // Check for empty required fields
        match field.field_type {
            FieldType::Text | FieldType::Date => {
                if field.value.is_empty() {
                    warnings.push(format!("Field {} has no value", field.id));
                }
            }
            FieldType::Signature => {
                if field.signature_data.is_none()
                    || field
                        .signature_data
                        .as_ref()
                        .map(|s| s.is_empty())
                        .unwrap_or(true)
                {
                    warnings.push(format!("Signature field {} has no signature", field.id));
                }
            }
            FieldType::Initials => {
                if field.value.is_empty() {
                    warnings.push(format!("Initials field {} is empty", field.id));
                }
            }
            FieldType::Checkbox => {
                // Checkboxes can be unchecked, no warning needed
            }
        }

        // Check for invalid coordinates
        if field.pdf_x < 0.0 || field.pdf_y < 0.0 {
            errors.push(format!("Field {} has negative coordinates", field.id));
        }

        // Check for zero-size fields
        if field.pdf_width <= 0.0 || field.pdf_height <= 0.0 {
            errors.push(format!("Field {} has zero or negative size", field.id));
        }
    }

    let result = serde_json::json!({
        "valid": errors.is_empty(),
        "errors": errors,
        "warnings": warnings,
        "field_count": fields.len()
    });

    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pdf() -> Vec<u8> {
        use lopdf::{dictionary, Document, Object};

        let mut doc = Document::with_version("1.7");
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        });
        if let Ok(page) = doc.get_object_mut(page_id) {
            if let Ok(dict) = page.as_dict_mut() {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn test_export_empty_fields() {
        let pdf = create_test_pdf();
        let fields_json = "[]";

        let result = export_pdf_with_fields(&pdf, fields_json);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_export_text_field() {
        let pdf = create_test_pdf();
        let fields_json = r##"[{
            "id": "field-1",
            "type": "text",
            "pageNum": 1,
            "pdfX": 100,
            "pdfY": 700,
            "pdfWidth": 200,
            "pdfHeight": 20,
            "value": "John Doe",
            "style": {
                "fontSize": 12,
                "fontFamily": "sans-serif",
                "isBold": false,
                "isItalic": false,
                "color": "#000000"
            }
        }]"##;

        let result = export_pdf_with_fields(&pdf, fields_json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_checkbox_checked() {
        let pdf = create_test_pdf();
        let fields_json = r##"[{
            "id": "field-1",
            "type": "checkbox",
            "pageNum": 1,
            "pdfX": 100,
            "pdfY": 700,
            "pdfWidth": 24,
            "pdfHeight": 24,
            "value": "Yes",
            "checked": true,
            "style": {
                "fontSize": 14,
                "fontFamily": "sans-serif",
                "isBold": false,
                "isItalic": false,
                "color": "#000000"
            }
        }]"##;

        let result = export_pdf_with_fields(&pdf, fields_json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_fields() {
        let fields_json = r##"[{
            "id": "field-1",
            "type": "text",
            "pageNum": 1,
            "pdfX": 100,
            "pdfY": 700,
            "pdfWidth": 200,
            "pdfHeight": 20,
            "value": "",
            "style": {
                "fontSize": 12,
                "fontFamily": "sans-serif",
                "isBold": false,
                "isItalic": false,
                "color": "#000000"
            }
        }]"##;

        let result = validate_fields_for_export(fields_json);
        assert!(result.is_ok());

        let validation: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(validation["valid"].as_bool().unwrap());
        assert_eq!(validation["warnings"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_validate_invalid_coordinates() {
        let fields_json = r##"[{
            "id": "field-1",
            "type": "text",
            "pageNum": 1,
            "pdfX": -10,
            "pdfY": 700,
            "pdfWidth": 200,
            "pdfHeight": 20,
            "value": "Test",
            "style": {
                "fontSize": 12,
                "fontFamily": "sans-serif",
                "isBold": false,
                "isItalic": false,
                "color": "#000000"
            }
        }]"##;

        let result = validate_fields_for_export(fields_json);
        assert!(result.is_ok());

        let validation: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(!validation["valid"].as_bool().unwrap());
        assert!(!validation["errors"].as_array().unwrap().is_empty());
    }
}
