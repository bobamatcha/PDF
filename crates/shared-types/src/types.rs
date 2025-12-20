#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeaseDocument {
    pub id: String,
    pub filename: String,
    pub pages: u32,
    pub text_content: Vec<String>, // Per-page text
    pub created_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComplianceReport {
    pub document_id: String,
    pub violations: Vec<Violation>,
    pub checked_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextPosition {
    pub start_offset: usize, // Character offset in the page text
    pub end_offset: usize,   // End character offset
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub statute: String, // e.g., "83.47(1)(a)"
    pub severity: Severity,
    pub message: String,
    pub page: Option<u32>,
    pub text_snippet: Option<String>,
    pub text_position: Option<TextPosition>, // Position for highlighting
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}
