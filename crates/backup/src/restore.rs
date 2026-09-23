use std::fs;
use std::path::Path;
use crate::error::BackupError;
use crate::BackupResult;

pub struct RestoreEngine;

impl RestoreEngine {
    /// Restore database from a compressed backup file
    pub fn restore_backup(backup_path: &Path, target_path: &Path) -> BackupResult<()> {
        if !backup_path.exists() {
            return Err(BackupError::BackupNotFound(
                format!("Backup not found: {}", backup_path.display()),
            ));
        }

        let compressed_data = fs::read(backup_path)
            .map_err(|e| BackupError::IoError(format!("Failed to read backup: {}", e)))?;

        // Decompress the backup
        let decompressed = zstd::decode_all(compressed_data.as_slice())
            .map_err(|e| BackupError::CompressionError(format!("Failed to decompress: {}", e)))?;

        // Create backup of current database if it exists
        if target_path.exists() {
            let backup_ext = format!("{}.pre-restore", target_path.display());
            fs::copy(target_path, &backup_ext)
                .map_err(|e| BackupError::IoError(format!("Failed to backup current database: {}", e)))?;
        }

        // Write restored database
        fs::write(target_path, decompressed)
            .map_err(|e| BackupError::IoError(format!("Failed to write restored database: {}", e)))?;

        Ok(())
    }

    /// List available backups in a directory
    pub fn list_backups(backup_dir: &Path) -> BackupResult<Vec<(String, u64)>> {
        if !backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();

        for entry in fs::read_dir(backup_dir)
            .map_err(|e| BackupError::IoError(format!("Failed to read backup directory: {}", e)))?
        {
            let entry = entry
                .map_err(|e| BackupError::IoError(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "backup") {
                let metadata = fs::metadata(&path)
                    .map_err(|e| BackupError::IoError(format!("Failed to read file metadata: {}", e)))?;

                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                backups.push((filename, metadata.len()));
            }
        }

        backups.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(backups)
    }
}
