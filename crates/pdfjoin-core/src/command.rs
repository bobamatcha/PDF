use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum PdfCommand {
    Merge {
        files: Vec<Vec<u8>>,
    },
    Split {
        file: Vec<u8>,
        ranges: Vec<(u32, u32)>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessResult {
    pub success: bool,
    /// Base64-encoded PDF data
    pub data: Option<String>,
    pub error: Option<String>,
    pub metrics: Option<ProcessMetrics>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessMetrics {
    pub input_size_bytes: usize,
    pub output_size_bytes: usize,
    pub page_count: u32,
    pub processing_time_ms: u64,
}
