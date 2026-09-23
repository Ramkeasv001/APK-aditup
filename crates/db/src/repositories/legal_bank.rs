use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegalBankEntry {
    pub id: String,
    pub standard: String,
    pub clause: String,
    pub text: String,
    pub version: i64,
}

/// CRUD for `legal_bank`. Deferred from Module 5 (Recommendation Bank) —
/// there's no foreign key between `legal_bank` and `recommendation_bank`,
/// and Module 8 (Rule-Based Engine) is the first module that actually needs
/// to query it, for clause suggestion against an observation's text.
/// `legal_bank` is organized by `standard` (e.g. "IS 2190:2024"), not a
/// `bank_key`-grouped dataset, so unlike `observation_bank` it doesn't
/// participate in `official_meta` versioning — each row just tracks its own
/// edit count.
pub struct LegalBankRepository<'a> {
    db: &'a Database,
}

impl<'a> LegalBankRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        standard: &str,
        clause: &str,
        text: &str,
    ) -> Result<LegalBankEntry, DbError> {
        if standard.trim().is_empty() || clause.trim().is_empty() {
            return Err(DbError::InvalidInput(
                "standard and clause must not be empty".into(),
            ));
        }

        let id = Uuid::new_v4().to_string();
        log::debug!("creating legal_bank row {id} ({standard} {clause})");
        self.db.conn().execute(
            "INSERT INTO legal_bank (id, standard, clause, text, version) VALUES (?1, ?2, ?3, ?4, 1)",
            params![id, standard, clause, text],
        )?;
        self.get(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: &str) -> Result<Option<LegalBankEntry>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, standard, clause, text, version FROM legal_bank WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list_by_standard(&self, standard: &str) -> Result<Vec<LegalBankEntry>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, standard, clause, text, version FROM legal_bank
             WHERE standard = ?1 ORDER BY clause ASC",
        )?;
        let rows = stmt.query_map([standard], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn list_all(&self) -> Result<Vec<LegalBankEntry>, DbError> {
        let conn = self.db.conn();
        let mut stmt =
            conn.prepare("SELECT id, standard, clause, text, version FROM legal_bank ORDER BY standard ASC, clause ASC")?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn update_text(&self, id: &str, new_text: &str) -> Result<LegalBankEntry, DbError> {
        let updated = self.db.conn().execute(
            "UPDATE legal_bank SET text = ?1, version = version + 1 WHERE id = ?2",
            params![new_text, id],
        )?;
        if updated == 0 {
            return Err(DbError::NotFound);
        }
        self.get(id)?.ok_or(DbError::NotFound)
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<LegalBankEntry> {
        Ok(LegalBankEntry {
            id: row.get(0)?,
            standard: row.get(1)?,
            clause: row.get(2)?,
            text: row.get(3)?,
            version: row.get(4)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:"), "test-passphrase").unwrap()
    }

    #[test]
    fn create_then_get_round_trips_with_version_one() {
        let db = test_db();
        let repo = LegalBankRepository::new(&db);

        let created = repo
            .create(
                "IS 2190:2024",
                "Clause 6.2",
                "Fire extinguishers shall be inspected monthly.",
            )
            .unwrap();
        let fetched = repo.get(&created.id).unwrap().unwrap();

        assert_eq!(created, fetched);
        assert_eq!(fetched.version, 1);
    }

    #[test]
    fn empty_standard_or_clause_is_rejected() {
        let db = test_db();
        let repo = LegalBankRepository::new(&db);
        assert!(repo.create("", "Clause 1", "text").is_err());
        assert!(repo.create("IS 2190:2024", " ", "text").is_err());
    }

    #[test]
    fn list_by_standard_only_returns_that_standards_clauses_sorted() {
        let db = test_db();
        let repo = LegalBankRepository::new(&db);
        repo.create("IS 2190:2024", "Clause 6.2", "text").unwrap();
        repo.create("IS 2190:2024", "Clause 4.1", "text").unwrap();
        repo.create("NBC Part F", "Clause 1", "text").unwrap();

        let rows = repo.list_by_standard("IS 2190:2024").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].clause, "Clause 4.1");
        assert_eq!(rows[1].clause, "Clause 6.2");
    }

    #[test]
    fn update_text_bumps_the_rows_own_version() {
        let db = test_db();
        let repo = LegalBankRepository::new(&db);
        let created = repo
            .create("IS 2190:2024", "Clause 6.2", "Original")
            .unwrap();

        let updated = repo.update_text(&created.id, "Revised").unwrap();

        assert_eq!(updated.text, "Revised");
        assert_eq!(updated.version, 2);
    }

    #[test]
    fn update_text_on_unknown_id_errors() {
        let db = test_db();
        let repo = LegalBankRepository::new(&db);
        assert!(matches!(
            repo.update_text("does-not-exist", "text"),
            Err(DbError::NotFound)
        ));
    }
}
