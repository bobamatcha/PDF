use thiserror::Error;

#[derive(Error, Debug)]
pub enum PdfJoinError {
    #[error("Failed to parse PDF: {0}")]
    ParseError(String),

    #[error("Invalid page range: {0}")]
    InvalidRange(String),

    #[error("PDF operation failed: {0}")]
    OperationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}
