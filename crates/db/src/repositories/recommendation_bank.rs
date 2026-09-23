use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;
use crate::repositories::observation_bank::ObservationBankRepository;
use crate::repositories::official_meta;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecommendationBankEntry {
    pub id: String,
    /// Nullable per schema: a recommendation doesn't have to be linked to a
    /// specific observation row (e.g. a general-purpose one), but when it
    /// is, writing it also counts as a change to that observation's bank.
    pub observation_bank_id: Option<String>,
    pub topic: String,
    pub text: String,
    pub version: i64,
}

/// CRUD + versioning for `recommendation_bank` (Module 5). Unlike
/// `observation_bank`, this table has no `bank_key` column of its own — a
/// linked row's dataset is whatever `bank_key` its parent `observation_bank`
/// row belongs to. Writing a linked recommendation is still a change to
/// that dataset's *published* content (Stage 3 surfaces the
/// observation+recommendation pair together), so it bumps the same
/// `official_meta` row `observation_bank`'s own writes do.
pub struct RecommendationBankRepository<'a> {
    db: &'a Database,
}

impl<'a> RecommendationBankRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        observation_bank_id: Option<&str>,
        topic: &str,
        text: &str,
    ) -> Result<RecommendationBankEntry, DbError> {
        let bank_key = self.resolve_bank_key(observation_bank_id)?;

        let id = Uuid::new_v4().to_string();
        log::debug!(
            "creating recommendation_bank row {id} (observation_bank_id={:?})",
            observation_bank_id
        );

        let tx = self.db.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO recommendation_bank (id, observation_bank_id, topic, text, version)
             VALUES (?1, ?2, ?3, ?4, 1)",
            params![id, observation_bank_id, topic, text],
        )?;
        if let Some(bank_key) = &bank_key {
            official_meta::bump_bank_version(&tx, bank_key)?;
        }
        tx.commit()?;

        self.get(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: &str) -> Result<Option<RecommendationBankEntry>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, observation_bank_id, topic, text, version
                 FROM recommendation_bank WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// Every recommendation linked to a given `observation_bank` row — the
    /// candidate set Stage 3 (and later the Search Engine) would draw from
    /// for a matched observation.
    pub fn list_for_observation(
        &self,
        observation_bank_id: &str,
    ) -> Result<Vec<RecommendationBankEntry>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, observation_bank_id, topic, text, version
             FROM recommendation_bank WHERE observation_bank_id = ?1",
        )?;
        let rows = stmt.query_map([observation_bank_id], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn update_text(
        &self,
        id: &str,
        new_text: &str,
    ) -> Result<RecommendationBankEntry, DbError> {
        let existing = self.get(id)?.ok_or(DbError::NotFound)?;
        let bank_key = self.resolve_bank_key(existing.observation_bank_id.as_deref())?;

        let tx = self.db.conn().unchecked_transaction()?;
        tx.execute(
            "UPDATE recommendation_bank SET text = ?1, version = version + 1 WHERE id = ?2",
            params![new_text, id],
        )?;
        if let Some(bank_key) = &bank_key {
            official_meta::bump_bank_version(&tx, bank_key)?;
        }
        tx.commit()?;

        self.get(id)?.ok_or(DbError::NotFound)
    }

    /// `None` when there's no `observation_bank_id` to resolve from, or when
    /// it points at a row that doesn't exist (shouldn't happen given the
    /// foreign key, but this repository doesn't assume the FK is the only
    /// thing keeping it honest).
    fn resolve_bank_key(
        &self,
        observation_bank_id: Option<&str>,
    ) -> Result<Option<String>, DbError> {
        match observation_bank_id {
            Some(obs_id) => ObservationBankRepository::new(self.db).bank_key_of(obs_id),
            None => Ok(None),
        }
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<RecommendationBankEntry> {
        Ok(RecommendationBankEntry {
            id: row.get(0)?,
            observation_bank_id: row.get(1)?,
            topic: row.get(2)?,
            text: row.get(3)?,
            version: row.get(4)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::official_meta::OfficialMetaRepository;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:"), "test-passphrase").unwrap()
    }

    fn seed_observation(db: &Database, bank_key: &str) -> String {
        ObservationBankRepository::new(db)
            .create(bank_key, "Topic", "Label", "Observation text")
            .unwrap()
            .id
    }

    #[test]
    fn create_then_get_round_trips_with_version_one() {
        let db = test_db();
        let obs_id = seed_observation(&db, "fire_extinguisher_is2190");
        let repo = RecommendationBankRepository::new(&db);

        let created = repo
            .create(
                Some(&obs_id),
                "Fire Protection",
                "Replace the extinguisher immediately.",
            )
            .unwrap();
        let fetched = repo.get(&created.id).unwrap().unwrap();

        assert_eq!(created, fetched);
        assert_eq!(fetched.version, 1);
        assert_eq!(
            fetched.observation_bank_id.as_deref(),
            Some(obs_id.as_str())
        );
    }

    #[test]
    fn unlinked_recommendation_is_allowed_and_does_not_touch_official_meta() {
        let db = test_db();
        let repo = RecommendationBankRepository::new(&db);

        let created = repo
            .create(None, "General", "General housekeeping recommendation.")
            .unwrap();

        assert!(created.observation_bank_id.is_none());
        // No bank_key exists to have been bumped.
        assert!(OfficialMetaRepository::new(&db)
            .get("fire_extinguisher_is2190")
            .unwrap()
            .is_none());
    }

    #[test]
    fn linked_recommendation_bumps_the_parent_banks_version() {
        let db = test_db();
        let obs_id = seed_observation(&db, "sprinkler_is15105");
        // Creating the observation already put the bank at version 1.
        assert_eq!(
            OfficialMetaRepository::new(&db)
                .get("sprinkler_is15105")
                .unwrap()
                .unwrap()
                .current_version,
            1
        );

        RecommendationBankRepository::new(&db)
            .create(Some(&obs_id), "Topic", "Text")
            .unwrap();

        assert_eq!(
            OfficialMetaRepository::new(&db)
                .get("sprinkler_is15105")
                .unwrap()
                .unwrap()
                .current_version,
            2
        );
    }

    #[test]
    fn update_text_bumps_row_version_and_parent_bank_version() {
        let db = test_db();
        let obs_id = seed_observation(&db, "bank_a");
        let repo = RecommendationBankRepository::new(&db);
        let created = repo.create(Some(&obs_id), "Topic", "Original").unwrap();
        // observation create (v1) + recommendation create (v2)
        assert_eq!(
            OfficialMetaRepository::new(&db)
                .get("bank_a")
                .unwrap()
                .unwrap()
                .current_version,
            2
        );

        let updated = repo.update_text(&created.id, "Revised").unwrap();

        assert_eq!(updated.text, "Revised");
        assert_eq!(updated.version, 2);
        assert_eq!(
            OfficialMetaRepository::new(&db)
                .get("bank_a")
                .unwrap()
                .unwrap()
                .current_version,
            3
        );
    }

    #[test]
    fn update_text_on_unknown_id_errors() {
        let db = test_db();
        let repo = RecommendationBankRepository::new(&db);
        assert!(matches!(
            repo.update_text("does-not-exist", "text"),
            Err(DbError::NotFound)
        ));
    }

    #[test]
    fn list_for_observation_only_returns_linked_recommendations() {
        let db = test_db();
        let obs_a = seed_observation(&db, "bank_a");
        let obs_b = seed_observation(&db, "bank_b");
        let repo = RecommendationBankRepository::new(&db);

        repo.create(Some(&obs_a), "Topic", "Rec A1").unwrap();
        repo.create(Some(&obs_a), "Topic", "Rec A2").unwrap();
        repo.create(Some(&obs_b), "Topic", "Rec B1").unwrap();
        repo.create(None, "Topic", "Unlinked rec").unwrap();

        assert_eq!(repo.list_for_observation(&obs_a).unwrap().len(), 2);
        assert_eq!(repo.list_for_observation(&obs_b).unwrap().len(), 1);
    }

    #[test]
    fn creating_a_recommendation_for_unknown_observation_fails_foreign_key_check() {
        let db = test_db();
        let repo = RecommendationBankRepository::new(&db);
        let result = repo.create(Some("does-not-exist"), "Topic", "Text");
        assert!(result.is_err());
    }
}
