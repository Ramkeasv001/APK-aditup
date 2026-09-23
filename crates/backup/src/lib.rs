//! Backup/Restore Engine — Module 12 of Phase 2 build.
//!
//! Provides encrypted, compressed database snapshots with integrity validation.
//! Backups are point-in-time snapshots that can be restored to recover from
//! data loss or corruption.

mod backup;
mod error;
mod manifest;
mod restore;

pub use backup::BackupEngine;
pub use error::BackupError;
pub use manifest::BackupManifest;
pub use restore::RestoreEngine;

pub type BackupResult<T> = Result<T, BackupError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_engine_exists() {
        let _ = BackupEngine;
    }

    #[test]
    fn restore_engine_exists() {
        let _ = RestoreEngine;
    }

    #[test]
    fn manifest_can_be_created() {
        let manifest = BackupManifest::new(
            "1.0".to_string(),
            1,
            1024,
            512,
            "abc123".to_string(),
            5,
            10,
        );

        assert_eq!(manifest.version, "1.0");
        assert_eq!(manifest.database_version, 1);
        assert_eq!(manifest.file_size, 1024);
        assert_eq!(manifest.compressed_size, 512);
        assert_eq!(manifest.audit_count, 5);
        assert_eq!(manifest.observation_count, 10);
    }

    #[test]
    fn manifest_with_description() {
        let manifest = BackupManifest::new(
            "1.0".to_string(),
            1,
            1024,
            512,
            "abc123".to_string(),
            0,
            0,
        )
        .with_description("Test backup".to_string());

        assert_eq!(manifest.description, Some("Test backup".to_string()));
    }
}
