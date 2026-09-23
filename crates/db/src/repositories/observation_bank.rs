use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;
use crate::repositories::official_meta::{self, BankVersion, OfficialMetaRepository};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationBankEntry {
    pub id: String,
    pub bank_key: String,
    pub topic: String,
    pub label: String,
    pub text: String,
    /// This row's own edit count, independent of the dataset-wide version
    /// tracked in `official_meta` (see `BankVersion`).
    pub version: i64,
}

/// CRUD + versioning for `observation_bank` (Module 4). Every write to a
/// bank's published content — a new row or an edit to an existing one —
/// bumps that `bank_key`'s `official_meta` version, so later modules (KLE's
/// staleness checks in Module 9, Admin's "unused official rows" stats) have
/// a single source of truth for "has this bank changed since I last looked."
pub struct ObservationBankRepository<'a> {
    db: &'a Database,
}

impl<'a> ObservationBankRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        bank_key: &str,
        topic: &str,
        label: &str,
        text: &str,
    ) -> Result<ObservationBankEntry, DbError> {
        if bank_key.trim().is_empty() {
            return Err(DbError::InvalidInput("bank_key must not be empty".into()));
        }

        let id = Uuid::new_v4().to_string();
        log::debug!("creating observation_bank row {id} in bank '{bank_key}'");

        let tx = self.db.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO observation_bank (id, bank_key, topic, label, text, version)
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            params![id, bank_key, topic, label, text],
        )?;
        official_meta::bump_bank_version(&tx, bank_key)?;
        tx.commit()?;

        self.get(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: &str) -> Result<Option<ObservationBankEntry>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, bank_key, topic, label, text, version FROM observation_bank WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// All rows published under `bank_key`, alphabetical by label.
    pub fn list_for_bank(&self, bank_key: &str) -> Result<Vec<ObservationBankEntry>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, bank_key, topic, label, text, version
             FROM observation_bank WHERE bank_key = ?1 ORDER BY label ASC",
        )?;
        let rows = stmt.query_map([bank_key], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Every distinct bank_key with at least one row — e.g. for an Admin
    /// bank picker (Module 10).
    pub fn list_bank_keys(&self) -> Result<Vec<String>, DbError> {
        let conn = self.db.conn();
        let mut stmt =
            conn.prepare("SELECT DISTINCT bank_key FROM observation_bank ORDER BY bank_key ASC")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Edits an existing row's text, bumping both the row's own `version`
    /// and its bank's `official_meta` version.
    pub fn update_text(&self, id: &str, new_text: &str) -> Result<ObservationBankEntry, DbError> {
        let existing = self.get(id)?.ok_or(DbError::NotFound)?;

        let tx = self.db.conn().unchecked_transaction()?;
        tx.execute(
            "UPDATE observation_bank SET text = ?1, version = version + 1 WHERE id = ?2",
            params![new_text, id],
        )?;
        official_meta::bump_bank_version(&tx, &existing.bank_key)?;
        tx.commit()?;

        self.get(id)?.ok_or(DbError::NotFound)
    }

    /// The published version of an entire `bank_key` dataset, or `None` if
    /// nothing has ever been published under that key.
    pub fn bank_version(&self, bank_key: &str) -> Result<Option<BankVersion>, DbError> {
        OfficialMetaRepository::new(self.db).get(bank_key)
    }

    /// Looks up which dataset a row belongs to — used by
    /// `RecommendationBankRepository` (Module 5) to find the right
    /// `official_meta` row to bump when a linked recommendation changes.
    pub(crate) fn bank_key_of(&self, id: &str) -> Result<Option<String>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT bank_key FROM observation_bank WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(DbError::from)
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<ObservationBankEntry> {
        Ok(ObservationBankEntry {
            id: row.get(0)?,
            bank_key: row.get(1)?,
            topic: row.get(2)?,
            label: row.get(3)?,
            text: row.get(4)?,
            version: row.get(5)?,
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
        let repo = ObservationBankRepository::new(&db);

        let created = repo
            .create(
                "fire_extinguisher_is2190",
                "Fire Protection",
                "Missing pressure gauge",
                "Fire extinguisher pressure gauge is missing or unreadable.",
            )
            .unwrap();
        let fetched = repo.get(&created.id).unwrap().unwrap();

        assert_eq!(created, fetched);
        assert_eq!(fetched.version, 1);
    }

    #[test]
    fn empty_bank_key_is_rejected() {
        let db = test_db();
        let repo = ObservationBankRepository::new(&db);
        let result = repo.create("  ", "Topic", "Label", "Text");
        assert!(matches!(result, Err(DbError::InvalidInput(_))));
    }

    #[test]
    fn creating_a_row_publishes_bank_version_one() {
        let db = test_db();
        let repo = ObservationBankRepository::new(&db);
        repo.create("sprinkler_is15105", "Fire Protection", "Label", "Text")
            .unwrap();

        let version = repo.bank_version("sprinkler_is15105").unwrap().unwrap();
        assert_eq!(version.current_version, 1);
        assert!(version.published_at.is_some());
    }

    #[test]
    fn a_second_row_in_the_same_bank_bumps_the_bank_version_again() {
        let db = test_db();
        let repo = ObservationBankRepository::new(&db);
        repo.create("sprinkler_is15105", "Topic", "Label A", "Text A")
            .unwrap();
        repo.create("sprinkler_is15105", "Topic", "Label B", "Text B")
            .unwrap();

        let version = repo.bank_version("sprinkler_is15105").unwrap().unwrap();
        assert_eq!(version.current_version, 2);
    }

    #[test]
    fn different_banks_version_independently() {
        let db = test_db();
        let repo = ObservationBankRepository::new(&db);
        repo.create("bank_a", "Topic", "Label", "Text").unwrap();
        repo.create("bank_a", "Topic", "Label 2", "Text 2").unwrap();
        repo.create("bank_b", "Topic", "Label", "Text").unwrap();

        assert_eq!(
            repo.bank_version("bank_a")
                .unwrap()
                .unwrap()
                .current_version,
            2
        );
        assert_eq!(
            repo.bank_version("bank_b")
                .unwrap()
                .unwrap()
                .current_version,
            1
        );
    }

    #[test]
    fn bank_version_is_none_for_a_bank_that_has_never_been_written_to() {
        let db = test_db();
        let repo = ObservationBankRepository::new(&db);
        assert!(repo.bank_version("never_created").unwrap().is_none());
    }

    #[test]
    fn update_text_bumps_row_version_and_bank_version() {
        let db = test_db();
        let repo = ObservationBankRepository::new(&db);
        let created = repo
            .create("bank_a", "Topic", "Label", "Original text")
            .unwrap();
        assert_eq!(
            repo.bank_version("bank_a")
                .unwrap()
                .unwrap()
                .current_version,
            1
        );

        let updated = repo.update_text(&created.id, "Revised text").unwrap();

        assert_eq!(updated.text, "Revised text");
        assert_eq!(updated.version, 2);
        assert_eq!(
            repo.bank_version("bank_a")
                .unwrap()
                .unwrap()
                .current_version,
            2
        );
    }

    #[test]
    fn update_text_on_unknown_id_errors() {
        let db = test_db();
        let repo = ObservationBankRepository::new(&db);
        assert!(matches!(
            repo.update_text("does-not-exist", "text"),
            Err(DbError::NotFound)
        ));
    }

    #[test]
    fn list_for_bank_only_returns_that_banks_rows_alphabetically() {
        let db = test_db();
        let repo = ObservationBankRepository::new(&db);
        repo.create("bank_a", "Topic", "Zebra finding", "Text")
            .unwrap();
        repo.create("bank_a", "Topic", "Apple finding", "Text")
            .unwrap();
        repo.create("bank_b", "Topic", "Other bank", "Text")
            .unwrap();

        let rows = repo.list_for_bank("bank_a").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "Apple finding");
        assert_eq!(rows[1].label, "Zebra finding");
    }

    #[test]
    fn list_bank_keys_returns_distinct_keys_sorted() {
        let db = test_db();
        let repo = ObservationBankRepository::new(&db);
        repo.create("sprinkler", "Topic", "A", "Text").unwrap();
        repo.create("fire_extinguisher", "Topic", "B", "Text")
            .unwrap();
        repo.create("sprinkler", "Topic", "C", "Text").unwrap();

        assert_eq!(
            repo.list_bank_keys().unwrap(),
            vec!["fire_extinguisher".to_string(), "sprinkler".to_string()]
        );
    }
}
