use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    Signature,
    Text,
    Date,
    Initials,
    Checkbox,
}

impl FieldType {
    /// Get default dimensions for a field type (width, height)
    fn default_dimensions(&self) -> (f64, f64) {
        match self {
            FieldType::Signature => (200.0, 50.0),
            FieldType::Text => (150.0, 30.0),
            FieldType::Date => (100.0, 30.0),
            FieldType::Initials => (50.0, 30.0),
            FieldType::Checkbox => (20.0, 20.0),
        }
    }

    /// Parse field type from string
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "signature" => Some(FieldType::Signature),
            "text" => Some(FieldType::Text),
            "date" => Some(FieldType::Date),
            "initials" => Some(FieldType::Initials),
            "checkbox" => Some(FieldType::Checkbox),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub id: String,
    pub field_type: FieldType,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub page: u32,
    pub value: Option<String>,
}

impl Field {
    /// Create a new field with default dimensions for the field type
    pub fn new(field_type: FieldType, x: f64, y: f64, page: u32) -> Self {
        let (width, height) = field_type.default_dimensions();
        Self {
            id: Uuid::new_v4().to_string(),
            field_type,
            x,
            y,
            width,
            height,
            page,
            value: None,
        }
    }

    /// Create a new field with custom dimensions
    pub fn new_with_size(
        field_type: FieldType,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        page: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            field_type,
            x,
            y,
            width,
            height,
            page,
            value: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldEditor {
    fields: Vec<Field>,
}

impl FieldEditor {
    /// Create a new field editor
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Place a new field and return its ID
    pub fn place_field(&mut self, field_type: FieldType, x: f64, y: f64, page: u32) -> String {
        let field = Field::new(field_type, x, y, page);
        let id = field.id.clone();
        self.fields.push(field);
        id
    }

    /// Move a field to a new position
    pub fn move_field(&mut self, id: &str, x: f64, y: f64) {
        if let Some(field) = self.fields.iter_mut().find(|f| f.id == id) {
            field.x = x;
            field.y = y;
        }
    }

    /// Resize a field
    pub fn resize_field(&mut self, id: &str, width: f64, height: f64) {
        if let Some(field) = self.fields.iter_mut().find(|f| f.id == id) {
            field.width = width;
            field.height = height;
        }
    }

    /// Delete a field by ID
    pub fn delete_field(&mut self, id: &str) {
        self.fields.retain(|f| f.id != id);
    }

    /// Get a field by ID
    pub fn get_field(&self, id: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.id == id)
    }

    /// Get a mutable reference to a field by ID
    pub fn get_field_mut(&mut self, id: &str) -> Option<&mut Field> {
        self.fields.iter_mut().find(|f| f.id == id)
    }

    /// Get all fields
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Get fields for a specific page
    pub fn fields_by_page(&self, page: u32) -> Vec<&Field> {
        self.fields.iter().filter(|f| f.page == page).collect()
    }

    /// Clear all fields
    pub fn clear(&mut self) {
        self.fields.clear();
    }

    /// Update field value
    pub fn set_field_value(&mut self, id: &str, value: String) {
        if let Some(field) = self.get_field_mut(id) {
            field.value = Some(value);
        }
    }
}

impl Default for FieldEditor {
    fn default() -> Self {
        Self::new()
    }
}

// WASM bindings
#[wasm_bindgen]
pub struct WasmFieldEditor {
    editor: FieldEditor,
}

#[allow(clippy::derivable_impls)]
impl Default for WasmFieldEditor {
    fn default() -> Self {
        Self {
            editor: FieldEditor::new(),
        }
    }
}

#[wasm_bindgen]
impl WasmFieldEditor {
    /// Create a new field editor
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Place a new field
    #[wasm_bindgen(js_name = placeField)]
    pub fn place_field(
        &mut self,
        field_type: &str,
        x: f64,
        y: f64,
        page: u32,
    ) -> Result<String, JsValue> {
        let ft = FieldType::from_str(field_type)
            .ok_or_else(|| JsValue::from_str(&format!("Invalid field type: {}", field_type)))?;
        Ok(self.editor.place_field(ft, x, y, page))
    }

    /// Move a field
    #[wasm_bindgen(js_name = moveField)]
    pub fn move_field(&mut self, id: &str, x: f64, y: f64) {
        self.editor.move_field(id, x, y);
    }

    /// Resize a field
    #[wasm_bindgen(js_name = resizeField)]
    pub fn resize_field(&mut self, id: &str, width: f64, height: f64) {
        self.editor.resize_field(id, width, height);
    }

    /// Delete a field
    #[wasm_bindgen(js_name = deleteField)]
    pub fn delete_field(&mut self, id: &str) {
        self.editor.delete_field(id);
    }

    /// Get all fields as JSON
    #[wasm_bindgen(js_name = getFieldsJson)]
    pub fn get_fields_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.editor.fields)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize fields: {}", e)))
    }

    /// Get fields for a specific page as JSON
    #[wasm_bindgen(js_name = getFieldsByPageJson)]
    pub fn get_fields_by_page_json(&self, page: u32) -> Result<String, JsValue> {
        let fields = self.editor.fields_by_page(page);
        serde_json::to_string(&fields)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize fields: {}", e)))
    }

    /// Set field value
    #[wasm_bindgen(js_name = setFieldValue)]
    pub fn set_field_value(&mut self, id: &str, value: String) {
        self.editor.set_field_value(id, value);
    }

    /// Clear all fields
    #[wasm_bindgen(js_name = clearFields)]
    pub fn clear(&mut self) {
        self.editor.clear();
    }

    /// Get field count
    #[wasm_bindgen(js_name = getFieldCount)]
    pub fn get_field_count(&self) -> usize {
        self.editor.fields().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_creation() {
        let field = Field::new(FieldType::Signature, 100.0, 200.0, 1);
        assert_eq!(field.field_type, FieldType::Signature);
        assert_eq!(field.x, 100.0);
        assert_eq!(field.y, 200.0);
        assert_eq!(field.page, 1);
    }

    #[test]
    fn test_field_editor_place_field() {
        let mut editor = FieldEditor::new();
        editor.place_field(FieldType::Signature, 100.0, 200.0, 1);
        assert_eq!(editor.fields().len(), 1);
        assert_eq!(editor.fields()[0].page, 1);
    }

    #[test]
    fn test_field_editor_multiple_fields() {
        let mut editor = FieldEditor::new();
        editor.place_field(FieldType::Signature, 100.0, 200.0, 1);
        editor.place_field(FieldType::Text, 150.0, 250.0, 1);
        editor.place_field(FieldType::Date, 200.0, 300.0, 2);
        assert_eq!(editor.fields().len(), 3);
    }

    #[test]
    fn test_field_move() {
        let mut editor = FieldEditor::new();
        let id = editor.place_field(FieldType::Signature, 100.0, 200.0, 1);
        editor.move_field(&id, 150.0, 250.0);
        let field = editor.get_field(&id).unwrap();
        assert_eq!(field.x, 150.0);
        assert_eq!(field.y, 250.0);
    }

    #[test]
    fn test_field_resize() {
        let mut editor = FieldEditor::new();
        let id = editor.place_field(FieldType::Signature, 100.0, 200.0, 1);
        editor.resize_field(&id, 200.0, 100.0);
        let field = editor.get_field(&id).unwrap();
        assert_eq!(field.width, 200.0);
        assert_eq!(field.height, 100.0);
    }

    #[test]
    fn test_field_delete() {
        let mut editor = FieldEditor::new();
        let id = editor.place_field(FieldType::Signature, 100.0, 200.0, 1);
        assert_eq!(editor.fields().len(), 1);
        editor.delete_field(&id);
        assert_eq!(editor.fields().len(), 0);
    }

    #[test]
    fn test_fields_by_page() {
        let mut editor = FieldEditor::new();
        editor.place_field(FieldType::Signature, 100.0, 200.0, 1);
        editor.place_field(FieldType::Text, 150.0, 250.0, 1);
        editor.place_field(FieldType::Date, 200.0, 300.0, 2);
        let page1_fields = editor.fields_by_page(1);
        let page2_fields = editor.fields_by_page(2);
        assert_eq!(page1_fields.len(), 2);
        assert_eq!(page2_fields.len(), 1);
    }

    #[test]
    fn test_default_dimensions() {
        // Test signature field dimensions (200x50)
        let sig = Field::new(FieldType::Signature, 0.0, 0.0, 1);
        assert_eq!(sig.width, 200.0);
        assert_eq!(sig.height, 50.0);

        // Test text field dimensions (150x30)
        let text = Field::new(FieldType::Text, 0.0, 0.0, 1);
        assert_eq!(text.width, 150.0);
        assert_eq!(text.height, 30.0);

        // Test date field dimensions (100x30)
        let date = Field::new(FieldType::Date, 0.0, 0.0, 1);
        assert_eq!(date.width, 100.0);
        assert_eq!(date.height, 30.0);

        // Test initials field dimensions (50x30)
        let initials = Field::new(FieldType::Initials, 0.0, 0.0, 1);
        assert_eq!(initials.width, 50.0);
        assert_eq!(initials.height, 30.0);

        // Test checkbox field dimensions (20x20)
        let checkbox = Field::new(FieldType::Checkbox, 0.0, 0.0, 1);
        assert_eq!(checkbox.width, 20.0);
        assert_eq!(checkbox.height, 20.0);
    }

    #[test]
    fn test_field_value() {
        let mut editor = FieldEditor::new();
        let id = editor.place_field(FieldType::Text, 100.0, 200.0, 1);

        // Initially no value
        assert_eq!(editor.get_field(&id).unwrap().value, None);

        // Set value
        editor.set_field_value(&id, "Test Value".to_string());
        assert_eq!(
            editor.get_field(&id).unwrap().value,
            Some("Test Value".to_string())
        );
    }

    #[test]
    fn test_clear_fields() {
        let mut editor = FieldEditor::new();
        editor.place_field(FieldType::Signature, 100.0, 200.0, 1);
        editor.place_field(FieldType::Text, 150.0, 250.0, 1);
        editor.place_field(FieldType::Date, 200.0, 300.0, 2);

        assert_eq!(editor.fields().len(), 3);
        editor.clear();
        assert_eq!(editor.fields().len(), 0);
    }

    #[test]
    fn test_field_id_uniqueness() {
        let mut editor = FieldEditor::new();
        let id1 = editor.place_field(FieldType::Signature, 100.0, 200.0, 1);
        let id2 = editor.place_field(FieldType::Signature, 100.0, 200.0, 1);
        let id3 = editor.place_field(FieldType::Signature, 100.0, 200.0, 1);

        // All IDs should be unique
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_field_type_serialization() {
        // Test that FieldType serializes to a readable string format
        let field = Field::new(FieldType::Signature, 100.0, 200.0, 1);
        let json = serde_json::to_string(&field).unwrap();

        // Print to see actual format
        println!("Field JSON: {}", json);

        // Should contain "Signature" as a string, not an object
        assert!(
            json.contains("\"Signature\""),
            "Expected 'Signature' string in JSON: {}",
            json
        );

        // Should NOT contain field_type as an object like {"field_type":{}}
        assert!(
            !json.contains("\"field_type\":{}"),
            "field_type should not be empty object"
        );
    }

    #[test]
    fn test_fields_array_serialization() {
        let mut editor = FieldEditor::new();
        editor.place_field(FieldType::Signature, 100.0, 200.0, 1);
        editor.place_field(FieldType::Text, 150.0, 250.0, 1);

        let fields = editor.fields_by_page(1);
        let json = serde_json::to_string(&fields).unwrap();

        // Both field types should be readable strings
        assert!(
            json.contains("\"Signature\""),
            "Should contain Signature: {}",
            json
        );
        assert!(json.contains("\"Text\""), "Should contain Text: {}", json);
    }
}
