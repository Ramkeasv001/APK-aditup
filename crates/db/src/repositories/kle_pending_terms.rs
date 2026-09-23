use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermPendingStatus {
    Pending,
    Approved,
    Rejected,
}

impl TermPendingStatus {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    fn from_sql(value: &str) -> rusqlite::Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            other => Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("unknown pending-term status '{other}'"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingKeyword {
    pub id: i64,
    pub term: String,
    pub contexts: Vec<String>,
    pub frequency_observed: i64,
    pub status: TermPendingStatus,
}

/// CRUD for `kle_pending_keywords` — mined candidate terms awaiting Admin
/// approval into `keyword_bank` (Module 6).
pub struct KlePendingKeywordsRepository<'a> {
    db: &'a Database,
}

impl<'a> KlePendingKeywordsRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Records a sighting of `term`. If it's already pending, bumps
    /// `frequency_observed` and folds `context` into the existing list
    /// instead of creating a duplicate row — repeated sightings are exactly
    /// the signal that makes a mined term worth an Admin's attention.
    pub fn observe(&self, term: &str, context: Option<&str>) -> Result<PendingKeyword, DbError> {
        let normalized = term.trim().to_lowercase();
        if let Some(existing) = self.get_pending_by_term(&normalized)? {
            let mut contexts = existing.contexts.clone();
            if let Some(c) = context {
                if !contexts.iter().any(|existing_c| existing_c == c) {
                    contexts.push(c.to_string());
                }
            }
            let contexts_json = serde_json::to_string(&contexts)?;
            self.db.conn().execute(
                "UPDATE kle_pending_keywords SET frequency_observed = frequency_observed + 1, contexts = ?1 WHERE id = ?2",
                params![contexts_json, existing.id],
            )?;
            return self.get(existing.id)?.ok_or(DbError::NotFound);
        }

        let contexts_json =
            serde_json::to_string(&context.map(|c| vec![c.to_string()]).unwrap_or_default())?;
        self.db.conn().execute(
            "INSERT INTO kle_pending_keywords (term, contexts, frequency_observed) VALUES (?1, ?2, 1)",
            params![normalized, contexts_json],
        )?;
        let id = self.db.conn().last_insert_rowid();
        self.get(id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: i64) -> Result<Option<PendingKeyword>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, term, contexts, frequency_observed, status FROM kle_pending_keywords WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    fn get_pending_by_term(&self, term: &str) -> Result<Option<PendingKeyword>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, term, contexts, frequency_observed, status FROM kle_pending_keywords
                 WHERE term = ?1 AND status = 'pending'",
                [term],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list_by_status(
        &self,
        status: TermPendingStatus,
    ) -> Result<Vec<PendingKeyword>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, term, contexts, frequency_observed, status FROM kle_pending_keywords
             WHERE status = ?1 ORDER BY frequency_observed DESC",
        )?;
        let rows = stmt.query_map([status.as_sql()], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn set_status(&self, id: i64, status: TermPendingStatus) -> Result<(), DbError> {
        let updated = self.db.conn().execute(
            "UPDATE kle_pending_keywords SET status = ?1 WHERE id = ?2",
            params![status.as_sql(), id],
        )?;
        if updated == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<PendingKeyword> {
        let contexts_json: String = row.get(2)?;
        let contexts = serde_json::from_str(&contexts_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
        })?;
        Ok(PendingKeyword {
            id: row.get(0)?,
            term: row.get(1)?,
            contexts,
            frequency_observed: row.get(3)?,
            status: TermPendingStatus::from_sql(&row.get::<_, String>(4)?)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingSynonym {
    pub id: i64,
    pub term_a: String,
    pub term_b: String,
    pub confidence: f64,
    pub status: TermPendingStatus,
}

/// CRUD for `kle_pending_synonyms` — mined candidate synonym pairs awaiting
/// Admin approval into `synonym_bank` (Module 6).
pub struct KlePendingSynonymsRepository<'a> {
    db: &'a Database,
}

impl<'a> KlePendingSynonymsRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        term_a: &str,
        term_b: &str,
        confidence: f64,
    ) -> Result<PendingSynonym, DbError> {
        let a = term_a.trim().to_lowercase();
        let b = term_b.trim().to_lowercase();
        if a.is_empty() || b.is_empty() || a == b {
            return Err(DbError::InvalidInput(
                "term_a/term_b must be non-empty and distinct".into(),
            ));
        }
        self.db.conn().execute(
            "INSERT INTO kle_pending_synonyms (term_a, term_b, confidence) VALUES (?1, ?2, ?3)",
            params![a, b, confidence],
        )?;
        let id = self.db.conn().last_insert_rowid();
        self.get(id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: i64) -> Result<Option<PendingSynonym>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, term_a, term_b, confidence, status FROM kle_pending_synonyms WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list_by_status(
        &self,
        status: TermPendingStatus,
    ) -> Result<Vec<PendingSynonym>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, term_a, term_b, confidence, status FROM kle_pending_synonyms WHERE status = ?1",
        )?;
        let rows = stmt.query_map([status.as_sql()], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn set_status(&self, id: i64, status: TermPendingStatus) -> Result<(), DbError> {
        let updated = self.db.conn().execute(
            "UPDATE kle_pending_synonyms SET status = ?1 WHERE id = ?2",
            params![status.as_sql(), id],
        )?;
        if updated == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<PendingSynonym> {
        Ok(PendingSynonym {
            id: row.get(0)?,
            term_a: row.get(1)?,
            term_b: row.get(2)?,
            confidence: row.get(3)?,
            status: TermPendingStatus::from_sql(&row.get::<_, String>(4)?)?,
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
    fn observe_creates_a_new_pending_keyword_with_frequency_one() {
        let db = test_db();
        let repo = KlePendingKeywordsRepository::new(&db);
        let created = repo.observe("toe-board", Some("fire_hydrant")).unwrap();
        assert_eq!(created.term, "toe-board");
        assert_eq!(created.frequency_observed, 1);
        assert_eq!(created.contexts, vec!["fire_hydrant".to_string()]);
    }

    #[test]
    fn observing_the_same_term_again_bumps_frequency_instead_of_duplicating() {
        let db = test_db();
        let repo = KlePendingKeywordsRepository::new(&db);
        let first = repo.observe("scaffold", Some("bank_a")).unwrap();
        let second = repo.observe("scaffold", Some("bank_b")).unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(second.frequency_observed, 2);
        assert_eq!(second.contexts.len(), 2);
        assert_eq!(
            repo.list_by_status(TermPendingStatus::Pending)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn observing_the_same_context_twice_does_not_duplicate_it() {
        let db = test_db();
        let repo = KlePendingKeywordsRepository::new(&db);
        repo.observe("scaffold", Some("bank_a")).unwrap();
        let second = repo.observe("scaffold", Some("bank_a")).unwrap();
        assert_eq!(second.contexts, vec!["bank_a".to_string()]);
        assert_eq!(second.frequency_observed, 2);
    }

    #[test]
    fn pending_keyword_set_status_on_unknown_id_errors() {
        let db = test_db();
        let repo = KlePendingKeywordsRepository::new(&db);
        assert!(matches!(
            repo.set_status(9999, TermPendingStatus::Approved),
            Err(DbError::NotFound)
        ));
    }

    #[test]
    fn pending_synonym_create_then_get_round_trips() {
        let db = test_db();
        let repo = KlePendingSynonymsRepository::new(&db);
        let created = repo
            .create("extinguisher", "fire extinguisher", 0.8)
            .unwrap();
        let fetched = repo.get(created.id).unwrap().unwrap();
        assert_eq!(created, fetched);
        assert_eq!(fetched.status, TermPendingStatus::Pending);
    }

    #[test]
    fn pending_synonym_rejects_self_pairing() {
        let db = test_db();
        let repo = KlePendingSynonymsRepository::new(&db);
        assert!(repo.create("scaffold", "Scaffold", 1.0).is_err());
    }

    #[test]
    fn pending_synonym_list_by_status_filters() {
        let db = test_db();
        let repo = KlePendingSynonymsRepository::new(&db);
        let a = repo.create("a", "b", 0.5).unwrap();
        repo.create("c", "d", 0.5).unwrap();
        repo.set_status(a.id, TermPendingStatus::Approved).unwrap();

        assert_eq!(
            repo.list_by_status(TermPendingStatus::Pending)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            repo.list_by_status(TermPendingStatus::Approved)
                .unwrap()
                .len(),
            1
        );
    }
}
