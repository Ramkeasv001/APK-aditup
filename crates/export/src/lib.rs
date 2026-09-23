//! Export Engine (Module 11).
//!
//! Generates shareable audit reports in XLSX (Excel), CSV, and (future) PDF/PPTX
//! formats. This crate owns all filesystem I/O and format generation. The Tauri
//! shell invokes these functions via IPC commands and returns file paths to the
//! renderer, which handles "Save As" UX.
//!
//! Each format is self-contained: `export_xlsx`, `export_csv`, etc. take an
//! `audit_id` and `output_path`, read from the database, and write the file.
//! They return the path (or error) so the caller knows what was written.

mod error;
pub mod formats;

pub use error::ExportError;
pub use formats::ExportFormat;

use aditup_db::{repositories::audits::AuditsRepository, Database};
use std::path::PathBuf;

pub type ExportResult<T> = Result<T, ExportError>;

/// Entry point: export an audit to the specified format at `output_path`.
/// Returns the path where the file was written on success.
pub fn export_audit(
    db: &Database,
    audit_id: &str,
    format: ExportFormat,
    output_path: &PathBuf,
) -> ExportResult<PathBuf> {
    // Verify the audit exists before attempting export
    let _audit = AuditsRepository::new(db)
        .get(audit_id)
        .map_err(|e| ExportError::Database(e.to_string()))?
        .ok_or_else(|| ExportError::AuditNotFound(audit_id.to_string()))?;

    match format {
        ExportFormat::Xlsx => formats::xlsx::export_xlsx(db, audit_id, output_path),
        ExportFormat::Csv => formats::csv::export_csv(db, audit_id, output_path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_format_enum_exists() {
        let _f = ExportFormat::Xlsx;
        let _f = ExportFormat::Csv;
    }

    #[test]
    fn export_format_xlsx_has_correct_extension() {
        assert_eq!(ExportFormat::Xlsx.extension(), "xlsx");
    }

    #[test]
    fn export_format_csv_has_correct_extension() {
        assert_eq!(ExportFormat::Csv.extension(), "csv");
    }

    #[test]
    fn export_format_xlsx_has_mime_type() {
        let mime = ExportFormat::Xlsx.mime_type();
        assert!(mime.contains("spreadsheet") || mime.contains("xlsx"));
    }

    #[test]
    fn export_format_csv_has_mime_type() {
        let mime = ExportFormat::Csv.mime_type();
        assert!(mime.contains("csv"));
    }

    #[test]
    fn export_error_displays_correctly() {
        let err = ExportError::Database("test error".to_string());
        assert_eq!(err.to_string(), "Database error: test error");

        let err2 = ExportError::AuditNotFound("audit-123".to_string());
        assert_eq!(err2.to_string(), "Audit not found: audit-123");
    }

    #[test]
    fn export_result_type_compiles() {
        fn test_fn() -> ExportResult<String> {
            Ok("success".to_string())
        }
        assert!(test_fn().is_ok());
    }
}
