use thiserror::Error;

/// All failure modes the storage layer can produce. Callers (Tauri commands
/// in later modules) match on this instead of the raw `rusqlite::Error` so
/// the IPC layer can decide what's safe to show the user versus what only
/// belongs in the log.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("database key was rejected — wrong password or a corrupted file")]
    InvalidKey,

    #[error("record not found")]
    NotFound,

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("migration {version} ({name}) failed: {source}")]
    Migration {
        version: i64,
        name: &'static str,
        #[source]
        source: rusqlite::Error,
    },

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("invalid data in row: {0}")]
    Serialization(#[from] serde_json::Error),
}
