use std::path::PathBuf;

use aditup_export::{export_audit, ExportFormat};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportInput {
    pub audit_id: String,
    pub format: String, // "xlsx" or "csv"
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    /// Absolute path where the file was written. Frontend uses this to trigger
    /// a download via `@tauri-apps/plugin-opener` (open()) or similar.
    pub file_path: String,
    pub format: String,
    pub mime_type: String,
}

/// Module 11 — Export Engine: generates a shareable audit report in the
/// requested format (XLSX or CSV), writes it to a temporary location, and
/// returns the path so the frontend can trigger a download.
///
/// The frontend is responsible for initiating the actual file download
/// (via the Opener plugin or Save As dialog). This command just generates
/// the file.
#[tauri::command]
pub fn export_audit_report(
    input: ExportInput,
    state: State<AppState>,
) -> Result<ExportResult, String> {
    let format = match input.format.to_lowercase().as_str() {
        "xlsx" => ExportFormat::Xlsx,
        "csv" => ExportFormat::Csv,
        other => {
            return Err(format!(
                "unknown export format '{other}' (supported: xlsx, csv)"
            ))
        }
    };

    state.with_db(|db| {
        // Use a temp path — the frontend will handle the actual save destination.
        // For now, write to temp and return the path; a more complete implementation
        // would let the frontend specify the target path directly.
        let temp_dir = std::env::temp_dir();
        let filename = format!(
            "ADITUP-Audit-{}.{}",
            input.audit_id.chars().take(8).collect::<String>(),
            format.extension()
        );
        let output_path = temp_dir.join(&filename);

        export_audit(db, &input.audit_id, format, &output_path)
            .map_err(|e| e.to_string())
            .map(|path| ExportResult {
                file_path: path.to_string_lossy().to_string(),
                format: input.format.to_lowercase(),
                mime_type: format.mime_type().to_string(),
            })
    })
}
