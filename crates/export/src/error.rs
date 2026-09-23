use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Audit not found: {0}")]
    AuditNotFound(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("File I/O error: {0}")]
    IoError(String),

    #[error("Format error: {0}")]
    FormatError(String),
}
