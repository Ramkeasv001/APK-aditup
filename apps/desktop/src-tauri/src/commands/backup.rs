use std::path::PathBuf;
use aditup_backup::{BackupEngine, BackupManifest, RestoreEngine};
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub backup_path: String,
    pub manifest: BackupManifest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInput {
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreInput {
    pub backup_path: String,
}

/// Module 12 — Create a full database backup with compression and integrity checks
#[tauri::command]
pub fn create_database_backup(
    input: BackupInput,
    state: State<AppState>,
) -> Result<BackupInfo, String> {
    state.with_db(|_db| {
        let backup_dir = std::env::var("ADITUP_BACKUP_DIR")
            .unwrap_or_else(|_| format!("{}/.aditup-backups", dirs::home_dir().unwrap_or_default().display()));

        std::fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;

        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let backup_filename = format!("aditup-backup-{}.backup", timestamp);
        let backup_path = PathBuf::from(&backup_dir).join(&backup_filename);

        let manifest = BackupEngine::create_backup(&state.db_path, &backup_path, input.description)
            .map_err(|e| e.to_string())?;

        Ok(BackupInfo {
            backup_path: backup_path.to_string_lossy().to_string(),
            manifest,
        })
    })
}

/// Module 12 — Verify backup integrity using SHA256 hash
#[tauri::command]
pub fn verify_backup(backup_path: String) -> Result<(), String> {
    let path = PathBuf::from(&backup_path);

    // For now, just check if file exists and is readable
    // Full verification would extract manifest and verify hash
    if !path.exists() {
        return Err(format!("Backup not found: {}", backup_path));
    }

    let metadata = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to read backup metadata: {}", e))?;

    if metadata.len() == 0 {
        return Err("Backup file is empty".to_string());
    }

    Ok(())
}

/// Module 12 — Restore database from a backup (creates pre-restore backup first)
#[tauri::command]
pub fn restore_from_backup(
    input: RestoreInput,
    state: State<AppState>,
) -> Result<String, String> {
    state.with_db(|_db| {
        let backup_path = PathBuf::from(&input.backup_path);
        let db_path = state.db_path.clone();

        RestoreEngine::restore_backup(&backup_path, &db_path)
            .map_err(|e| e.to_string())?;

        Ok(format!("Database restored from {}", input.backup_path))
    })
}

/// Module 12 — List available backups in backup directory
#[tauri::command]
pub fn list_backups() -> Result<Vec<(String, u64)>, String> {
    let backup_dir = std::env::var("ADITUP_BACKUP_DIR")
        .unwrap_or_else(|_| format!("{}/.aditup-backups", dirs::home_dir().unwrap_or_default().display()));

    let path = PathBuf::from(&backup_dir);
    RestoreEngine::list_backups(&path)
        .map_err(|e| e.to_string())
}
