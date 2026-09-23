use std::fs;
use std::path::Path;
use sha2::{Sha256, Digest};
use crate::error::BackupError;
use crate::manifest::BackupManifest;
use crate::BackupResult;

pub struct BackupEngine;

impl BackupEngine {
    /// Create a full database backup to the specified path
    pub fn create_backup(
        db_path: &Path,
        output_path: &Path,
        description: Option<String>,
    ) -> BackupResult<BackupManifest> {
        // Read the entire database file
        let db_data = fs::read(db_path)
            .map_err(|e| BackupError::IoError(format!("Failed to read database: {}", e)))?;

        let original_size = db_data.len() as u64;

        // Compress the database
        let compressed = zstd::encode_all(db_data.as_slice(), 0)
            .map_err(|e| BackupError::CompressionError(e.to_string()))?;

        let compressed_size = compressed.len() as u64;

        // Calculate SHA256 hash
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let hash = hex::encode(hasher.finalize());

        // Write backup file
        fs::write(output_path, &compressed)
            .map_err(|e| BackupError::IoError(format!("Failed to write backup: {}", e)))?;

        // Note: audit and observation counts are retrieved by the Tauri command
        // handler and passed separately. This function handles only the file
        // compression and integrity check. Manifest counts are set to 0 here.

        let manifest = BackupManifest::new(
            "1.0".to_string(),
            1,
            original_size,
            compressed_size,
            hash,
            0, // audit_count
            0, // observation_count
        );

        let manifest = match description {
            Some(desc) => manifest.with_description(desc),
            None => manifest,
        };

        Ok(manifest)
    }

    /// Verify backup integrity using SHA256
    pub fn verify_backup(backup_path: &Path, expected_hash: &str) -> BackupResult<()> {
        let data = fs::read(backup_path)
            .map_err(|e| BackupError::IoError(format!("Failed to read backup: {}", e)))?;

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let actual_hash = hex::encode(hasher.finalize());

        if actual_hash != expected_hash {
            return Err(BackupError::IntegrityCheckFailed {
                expected: expected_hash.to_string(),
                actual: actual_hash,
            });
        }

        Ok(())
    }
}
