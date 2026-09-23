use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub id: Uuid,
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub database_version: i32,
    pub file_size: u64,
    pub compressed_size: u64,
    pub sha256_hash: String,
    pub description: Option<String>,
    pub audit_count: u32,
    pub observation_count: u32,
}

impl BackupManifest {
    pub fn new(
        version: String,
        database_version: i32,
        file_size: u64,
        compressed_size: u64,
        sha256_hash: String,
        audit_count: u32,
        observation_count: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            version,
            timestamp: Utc::now(),
            database_version,
            file_size,
            compressed_size,
            sha256_hash,
            description: None,
            audit_count,
            observation_count,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}
