use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeywordBankEntry {
    pub id: String,
    pub term: String,
    /// bank_keys this term has been seen in — stored as JSON text in the
    /// `contexts` column, deserialized here so callers never touch raw JSON.
    pub contexts: Vec<String>,
    /// Feedback-tuned relevance weight (Module 9's job to adjust); starts
    /// at 1.0 (neutral) for every newly admitted term.
    pub weight: f64,
}

/// CRUD for `keyword_bank`. Terms are admitted here by an Admin approving a
/// KLE-mined suggestion (Module 9) — this repository only knows how to
/// store/retrieve/reweight an already-decided term, not how to mine one.
pub struct KeywordBankRepository<'a> {
    db: &'a Database,
}

impl<'a> KeywordBankRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, term: &str, contexts: &[String]) -> Result<KeywordBankEntry, DbError> {
        let normalized = term.trim().to_lowercase();
        if normalized.is_empty() {
            return Err(DbError::InvalidInput("term must not be empty".into()));
        }

        let id = Uuid::new_v4().to_string();
        let contexts_json = serde_json::to_string(contexts)?;
        log::debug!("creating keyword_bank term '{normalized}' ({id})");
        self.db.conn().execute(
            "INSERT INTO keyword_bank (id, term, contexts, weight) VALUES (?1, ?2, ?3, 1.0)",
            params![id, normalized, contexts_json],
        )?;
        self.get(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: &str) -> Result<Option<KeywordBankEntry>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, term, contexts, weight FROM keyword_bank WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn get_by_term(&self, term: &str) -> Result<Option<KeywordBankEntry>, DbError> {
        let normalized = term.trim().to_lowercase();
        self.db
            .conn()
            .query_row(
                "SELECT id, term, contexts, weight FROM keyword_bank WHERE term = ?1",
                [normalized],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list_all(&self) -> Result<Vec<KeywordBankEntry>, DbError> {
        let conn = self.db.conn();
        let mut stmt =
            conn.prepare("SELECT id, term, contexts, weight FROM keyword_bank ORDER BY term ASC")?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Overwrites a term's feedback weight (Module 9's ranking-weight
    /// adjustment loop). Not exposed as an increment/decrement here — the
    /// caller computes the new value and this just persists it, keeping the
    /// adjustment policy entirely out of the storage layer.
    pub fn set_weight(&self, id: &str, new_weight: f64) -> Result<(), DbError> {
        let updated = self.db.conn().execute(
            "UPDATE keyword_bank SET weight = ?1 WHERE id = ?2",
            params![new_weight, id],
        )?;
        if updated == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<KeywordBankEntry> {
        let contexts_json: String = row.get(2)?;
        let contexts = serde_json::from_str(&contexts_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
        })?;
        Ok(KeywordBankEntry {
            id: row.get(0)?,
            term: row.get(1)?,
            contexts,
            weight: row.get(3)?,
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
    fn create_then_get_round_trips_with_neutral_weight() {
        let db = test_db();
        let repo = KeywordBankRepository::new(&db);

        let created = repo
            .create("toe-board", &["fire_hydrant".to_string()])
            .unwrap();
        let fetched = repo.get(&created.id).unwrap().unwrap();

        assert_eq!(created, fetched);
        assert_eq!(fetched.term, "toe-board");
        assert_eq!(fetched.contexts, vec!["fire_hydrant".to_string()]);
        assert_eq!(fetched.weight, 1.0);
    }

    #[test]
    fn terms_are_normalized_to_lowercase_and_trimmed() {
        let db = test_db();
        let repo = KeywordBankRepository::new(&db);
        let created = repo.create("  Scaffold  ", &[]).unwrap();
        assert_eq!(created.term, "scaffold");
    }

    #[test]
    fn empty_term_is_rejected() {
        let db = test_db();
        let repo = KeywordBankRepository::new(&db);
        assert!(matches!(
            repo.create("   ", &[]),
            Err(DbError::InvalidInput(_))
        ));
    }

    #[test]
    fn duplicate_term_is_rejected() {
        let db = test_db();
        let repo = KeywordBankRepository::new(&db);
        repo.create("scaffold", &[]).unwrap();
        assert!(repo.create("scaffold", &[]).is_err());
    }

    #[test]
    fn get_by_term_is_case_insensitive() {
        let db = test_db();
        let repo = KeywordBankRepository::new(&db);
        repo.create("scaffold", &[]).unwrap();
        assert!(repo.get_by_term("SCAFFOLD").unwrap().is_some());
    }

    #[test]
    fn set_weight_updates_the_stored_value() {
        let db = test_db();
        let repo = KeywordBankRepository::new(&db);
        let created = repo.create("scaffold", &[]).unwrap();

        repo.set_weight(&created.id, 1.35).unwrap();

        let fetched = repo.get(&created.id).unwrap().unwrap();
        assert_eq!(fetched.weight, 1.35);
    }

    #[test]
    fn set_weight_on_unknown_id_errors() {
        let db = test_db();
        let repo = KeywordBankRepository::new(&db);
        assert!(matches!(
            repo.set_weight("does-not-exist", 2.0),
            Err(DbError::NotFound)
        ));
    }

    #[test]
    fn list_all_is_sorted_alphabetically() {
        let db = test_db();
        let repo = KeywordBankRepository::new(&db);
        repo.create("zebra", &[]).unwrap();
        repo.create("apple", &[]).unwrap();

        let terms: Vec<_> = repo
            .list_all()
            .unwrap()
            .into_iter()
            .map(|e| e.term)
            .collect();
        assert_eq!(terms, vec!["apple".to_string(), "zebra".to_string()]);
    }
}
