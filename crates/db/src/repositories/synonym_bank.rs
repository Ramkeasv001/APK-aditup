use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynonymBankEntry {
    pub id: String,
    pub term_a: String,
    pub term_b: String,
    pub confidence: f64,
}

/// CRUD for `synonym_bank` — learned/approved synonym pairs (distinct from
/// the small fixed abbreviation dictionary in `aditup-scoring-engine`; this
/// table grows over time via KLE admin approval, Module 9).
pub struct SynonymBankRepository<'a> {
    db: &'a Database,
}

impl<'a> SynonymBankRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        term_a: &str,
        term_b: &str,
        confidence: f64,
    ) -> Result<SynonymBankEntry, DbError> {
        let a = term_a.trim().to_lowercase();
        let b = term_b.trim().to_lowercase();
        if a.is_empty() || b.is_empty() {
            return Err(DbError::InvalidInput(
                "term_a/term_b must not be empty".into(),
            ));
        }
        if a == b {
            return Err(DbError::InvalidInput(
                "a term cannot be a synonym of itself".into(),
            ));
        }

        let id = Uuid::new_v4().to_string();
        log::debug!("creating synonym pair '{a}' <-> '{b}' ({id})");
        self.db.conn().execute(
            "INSERT INTO synonym_bank (id, term_a, term_b, confidence) VALUES (?1, ?2, ?3, ?4)",
            params![id, a, b, confidence],
        )?;
        self.get(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: &str) -> Result<Option<SynonymBankEntry>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, term_a, term_b, confidence FROM synonym_bank WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// Every term paired with `term`, in either column — the relation is
    /// conceptually symmetric even though each row stores a fixed
    /// (term_a, term_b) order.
    pub fn list_for_term(&self, term: &str) -> Result<Vec<SynonymBankEntry>, DbError> {
        let normalized = term.trim().to_lowercase();
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, term_a, term_b, confidence FROM synonym_bank
             WHERE term_a = ?1 OR term_b = ?1",
        )?;
        let rows = stmt.query_map([normalized], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn list_all(&self) -> Result<Vec<SynonymBankEntry>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, term_a, term_b, confidence FROM synonym_bank")?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<SynonymBankEntry> {
        Ok(SynonymBankEntry {
            id: row.get(0)?,
            term_a: row.get(1)?,
            term_b: row.get(2)?,
            confidence: row.get(3)?,
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
    fn create_then_get_round_trips_normalized() {
        let db = test_db();
        let repo = SynonymBankRepository::new(&db);

        let created = repo
            .create("Extinguisher", "FIRE EXTINGUISHER", 0.9)
            .unwrap();
        let fetched = repo.get(&created.id).unwrap().unwrap();

        assert_eq!(created, fetched);
        assert_eq!(fetched.term_a, "extinguisher");
        assert_eq!(fetched.term_b, "fire extinguisher");
        assert_eq!(fetched.confidence, 0.9);
    }

    #[test]
    fn a_term_cannot_be_a_synonym_of_itself() {
        let db = test_db();
        let repo = SynonymBankRepository::new(&db);
        assert!(matches!(
            repo.create("scaffold", "Scaffold", 1.0),
            Err(DbError::InvalidInput(_))
        ));
    }

    #[test]
    fn empty_terms_are_rejected() {
        let db = test_db();
        let repo = SynonymBankRepository::new(&db);
        assert!(matches!(
            repo.create("", "scaffold", 1.0),
            Err(DbError::InvalidInput(_))
        ));
    }

    #[test]
    fn duplicate_pair_is_rejected() {
        let db = test_db();
        let repo = SynonymBankRepository::new(&db);
        repo.create("a", "b", 1.0).unwrap();
        assert!(repo.create("a", "b", 1.0).is_err());
    }

    #[test]
    fn list_for_term_finds_pairs_regardless_of_column_order() {
        let db = test_db();
        let repo = SynonymBankRepository::new(&db);
        repo.create("extinguisher", "fire extinguisher", 0.9)
            .unwrap();
        repo.create("hydrant", "extinguisher", 0.4).unwrap();
        repo.create("unrelated", "other", 1.0).unwrap();

        let matches = repo.list_for_term("extinguisher").unwrap();
        assert_eq!(matches.len(), 2);
    }
}
