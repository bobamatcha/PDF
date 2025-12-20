//! Output format handling (PDF, SVG, PNG)

use serde::{Deserialize, Serialize};

/// Output format for rendered documents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Pdf,
    Svg,
    Png,
}

impl OutputFormat {
    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            OutputFormat::Pdf => "application/pdf",
            OutputFormat::Svg => "image/svg+xml",
            OutputFormat::Png => "image/png",
        }
    }

    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Pdf => "pdf",
            OutputFormat::Svg => "svg",
            OutputFormat::Png => "png",
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Pdf => write!(f, "pdf"),
            OutputFormat::Svg => write!(f, "svg"),
            OutputFormat::Png => write!(f, "png"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pdf" => Ok(OutputFormat::Pdf),
            "svg" => Ok(OutputFormat::Svg),
            "png" => Ok(OutputFormat::Png),
            other => Err(format!("Unknown output format: {}", other)),
        }
    }
}
