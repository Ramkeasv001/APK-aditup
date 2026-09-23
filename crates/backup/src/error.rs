use thiserror::Error;

#[derive(Error, Debug)]
pub enum BackupError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Backup corrupted: {0}")]
    CorruptedBackup(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Invalid backup manifest: {0}")]
    InvalidManifest(String),

    #[error("Restore failed: {0}")]
    RestoreFailed(String),

    #[error("Integrity check failed: expected {expected}, got {actual}")]
    IntegrityCheckFailed { expected: String, actual: String },

    #[error("Backup path not found: {0}")]
    BackupNotFound(String),
}
