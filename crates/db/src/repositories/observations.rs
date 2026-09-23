use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub id: String,
    pub audit_id: String,
    pub entry_hash: String,
    pub text: String,
    pub location: Option<String>,
    pub created_at: String,
}

/// Stable content hash of `location + text`, used as `entry_hash`.
///
/// This is deliberately a hash of content, not a surrogate ID: it must
/// produce the same value for the same observation across sessions, machines,
/// and export/import round-trips, because the Self-Learning Engine (Module 9)
/// links its staging/history tables back to observations by this hash rather
/// than a foreign key (see §6 of the architecture doc).
pub fn compute_entry_hash(location: Option<&str>, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(location.unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// CRUD access to `observations` (Stage 1 intake).
pub struct ObservationsRepository<'a> {
    db: &'a Database,
}

impl<'a> ObservationsRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        audit_id: &str,
        text: &str,
        location: Option<&str>,
    ) -> Result<Observation, DbError> {
        let id = Uuid::new_v4().to_string();
        let entry_hash = compute_entry_hash(location, text);
        log::debug!("creating observation {id} for audit {audit_id} (hash {entry_hash})");
        self.db.conn().execute(
            "INSERT INTO observations (id, audit_id, entry_hash, text, location)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, audit_id, entry_hash, text, location],
        )?;
        self.get(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: &str) -> Result<Option<Observation>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, audit_id, entry_hash, text, location, created_at
                 FROM observations WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list_for_audit(&self, audit_id: &str) -> Result<Vec<Observation>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, audit_id, entry_hash, text, location, created_at
             FROM observations WHERE audit_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([audit_id], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn update_text(&self, id: &str, text: &str) -> Result<Observation, DbError> {
        let existing = self.get(id)?.ok_or(DbError::NotFound)?;
        let entry_hash = compute_entry_hash(existing.location.as_deref(), text);
        self.db.conn().execute(
            "UPDATE observations SET text = ?1, entry_hash = ?2 WHERE id = ?3",
            params![text, entry_hash, id],
        )?;
        self.get(id)?.ok_or(DbError::NotFound)
    }

    pub fn delete(&self, id: &str) -> Result<(), DbError> {
        let deleted = self
            .db
            .conn()
            .execute("DELETE FROM observations WHERE id = ?1", [id])?;
        if deleted == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Observation> {
        Ok(Observation {
            id: row.get(0)?,
            audit_id: row.get(1)?,
            entry_hash: row.get(2)?,
            text: row.get(3)?,
            location: row.get(4)?,
            created_at: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::audits::AuditsRepository;
    use crate::repositories::projects::ProjectsRepository;
    use crate::repositories::sites::SitesRepository;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:"), "test-passphrase").unwrap()
    }

    fn test_audit(db: &Database) -> String {
        let project = ProjectsRepository::new(db).create("Project", None).unwrap();
        let site = SitesRepository::new(db)
            .create(&project.id, "Site", None)
            .unwrap();
        let user_id = Uuid::new_v4().to_string();
        db.conn()
            .execute(
                "INSERT INTO users (id, username, password_hash, role) VALUES (?1, 'a', 'x', 'auditor')",
                [&user_id],
            )
            .unwrap();
        AuditsRepository::new(db)
            .create(&site.id, &user_id, "2026-07-04", None)
            .unwrap()
            .id
    }

    #[test]
    fn entry_hash_is_deterministic_for_same_location_and_text() {
        let a = compute_entry_hash(Some("Bay 3"), "Fire extinguisher missing seal");
        let b = compute_entry_hash(Some("Bay 3"), "Fire extinguisher missing seal");
        assert_eq!(a, b);
    }

    #[test]
    fn entry_hash_differs_when_location_or_text_differ() {
        let base = compute_entry_hash(Some("Bay 3"), "Fire extinguisher missing seal");
        let diff_location = compute_entry_hash(Some("Bay 4"), "Fire extinguisher missing seal");
        let diff_text = compute_entry_hash(Some("Bay 3"), "Fire extinguisher expired");
        assert_ne!(base, diff_location);
        assert_ne!(base, diff_text);
    }

    #[test]
    fn create_then_get_round_trips_and_sets_entry_hash() {
        let db = test_db();
        let audit_id = test_audit(&db);
        let repo = ObservationsRepository::new(&db);

        let created = repo
            .create(
                &audit_id,
                "Scaffold missing toe-board",
                Some("East stairwell"),
            )
            .unwrap();
        let fetched = repo.get(&created.id).unwrap().unwrap();

        assert_eq!(created, fetched);
        assert_eq!(
            fetched.entry_hash,
            compute_entry_hash(Some("East stairwell"), "Scaffold missing toe-board")
        );
    }

    #[test]
    fn list_for_audit_returns_entries_in_creation_order() {
        let db = test_db();
        let audit_id = test_audit(&db);
        let repo = ObservationsRepository::new(&db);

        repo.create(&audit_id, "First", None).unwrap();
        repo.create(&audit_id, "Second", None).unwrap();

        let entries = repo.list_for_audit(&audit_id).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "First");
        assert_eq!(entries[1].text, "Second");
    }

    #[test]
    fn update_text_recomputes_entry_hash() {
        let db = test_db();
        let audit_id = test_audit(&db);
        let repo = ObservationsRepository::new(&db);
        let created = repo.create(&audit_id, "Original", Some("Loc")).unwrap();

        let updated = repo.update_text(&created.id, "Edited").unwrap();

        assert_eq!(updated.text, "Edited");
        assert_eq!(
            updated.entry_hash,
            compute_entry_hash(Some("Loc"), "Edited")
        );
        assert_ne!(updated.entry_hash, created.entry_hash);
    }

    #[test]
    fn delete_removes_the_row() {
        let db = test_db();
        let audit_id = test_audit(&db);
        let repo = ObservationsRepository::new(&db);
        let created = repo.create(&audit_id, "Temp", None).unwrap();

        repo.delete(&created.id).unwrap();

        assert!(repo.get(&created.id).unwrap().is_none());
        assert!(matches!(repo.delete(&created.id), Err(DbError::NotFound)));
    }

    #[test]
    fn creating_an_observation_for_unknown_audit_fails_foreign_key_check() {
        let db = test_db();
        let repo = ObservationsRepository::new(&db);
        let result = repo.create("does-not-exist", "Orphan", None);
        assert!(result.is_err());
    }
}
