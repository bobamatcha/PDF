use serde::{Deserialize, Serialize};

/// Core document representation with embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: DocumentMetadata,
    pub embedding: Option<Vec<f32>>,  // 1024-dim BGE-M3 vector
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: String,
    pub author: Option<String>,
    pub version: u32,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Template with variable placeholders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub content: String,
    pub variables: Vec<TemplateVariable>,
    pub conditionals: Vec<ConditionalSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub var_type: VariableType,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableType {
    Text,
    Number,
    Date,
    Boolean,
    List,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalSection {
    pub condition: String,
    pub content: String,
}
