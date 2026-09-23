use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage3MatchStatus {
    Matched,
    Written,
    Escalated,
}

impl Stage3MatchStatus {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Written => "written",
            Self::Escalated => "escalated",
        }
    }

    fn from_sql(value: &str) -> rusqlite::Result<Self> {
        match value {
            "matched" => Ok(Self::Matched),
            "written" => Ok(Self::Written),
            "escalated" => Ok(Self::Escalated),
            other => Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("unknown stage3 match status '{other}'"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage3Match {
    pub id: String,
    pub observation_id: String,
    pub bank_key: Option<String>,
    pub matched_row_id: Option<String>,
    pub match_score: Option<f64>,
    pub status: Stage3MatchStatus,
    pub final_text: Option<String>,
}

/// CRUD for `stage3_matches` (Module 10e). A row starts `Escalated` (the
/// schema's own default) the moment it's created via `get_or_create` — an
/// observation with no row yet simply hasn't reached Stage 3 UI
/// interaction at all; one with an `Escalated` row has been looked at but
/// not resolved. "Resolved" for the Stage 3 completion gate means
/// `Matched` or `Written`.
pub struct Stage3MatchesRepository<'a> {
    db: &'a Database,
}

impl<'a> Stage3MatchesRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn get_or_create(&self, observation_id: &str) -> Result<Stage3Match, DbError> {
        if let Some(existing) = self.get_for_observation(observation_id)? {
            return Ok(existing);
        }
        let id = Uuid::new_v4().to_string();
        self.db.conn().execute(
            "INSERT INTO stage3_matches (id, observation_id) VALUES (?1, ?2)",
            params![id, observation_id],
        )?;
        self.get_for_observation(observation_id)?
            .ok_or(DbError::NotFound)
    }

    pub fn get_for_observation(
        &self,
        observation_id: &str,
    ) -> Result<Option<Stage3Match>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, observation_id, bank_key, matched_row_id, match_score, status, final_text
                 FROM stage3_matches WHERE observation_id = ?1",
                [observation_id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn mark_matched(
        &self,
        observation_id: &str,
        bank_key: &str,
        matched_row_id: &str,
        match_score: f64,
        final_text: &str,
    ) -> Result<Stage3Match, DbError> {
        self.get_or_create(observation_id)?;
        self.db.conn().execute(
            "UPDATE stage3_matches
             SET status = 'matched', bank_key = ?1, matched_row_id = ?2, match_score = ?3, final_text = ?4
             WHERE observation_id = ?5",
            params![bank_key, matched_row_id, match_score, final_text, observation_id],
        )?;
        self.get_for_observation(observation_id)?
            .ok_or(DbError::NotFound)
    }

    pub fn mark_written(
        &self,
        observation_id: &str,
        final_text: &str,
    ) -> Result<Stage3Match, DbError> {
        self.get_or_create(observation_id)?;
        self.db.conn().execute(
            "UPDATE stage3_matches
             SET status = 'written', bank_key = NULL, matched_row_id = NULL, match_score = NULL, final_text = ?1
             WHERE observation_id = ?2",
            params![final_text, observation_id],
        )?;
        self.get_for_observation(observation_id)?
            .ok_or(DbError::NotFound)
    }

    pub fn mark_escalated(&self, observation_id: &str) -> Result<Stage3Match, DbError> {
        self.get_or_create(observation_id)?;
        self.db.conn().execute(
            "UPDATE stage3_matches
             SET status = 'escalated', bank_key = NULL, matched_row_id = NULL, match_score = NULL, final_text = NULL
             WHERE observation_id = ?1",
            [observation_id],
        )?;
        self.get_for_observation(observation_id)?
            .ok_or(DbError::NotFound)
    }

    /// Updates only `final_text`, leaving status/bank_key/match_score as-is
    /// — used when an auditor edits already-matched or already-written
    /// text (§4: "editing the filled text after selection is allowed").
    pub fn update_final_text(
        &self,
        observation_id: &str,
        new_text: &str,
    ) -> Result<Stage3Match, DbError> {
        self.get_or_create(observation_id)?;
        self.db.conn().execute(
            "UPDATE stage3_matches SET final_text = ?1 WHERE observation_id = ?2",
            params![new_text, observation_id],
        )?;
        self.get_for_observation(observation_id)?
            .ok_or(DbError::NotFound)
    }

    pub fn list_for_audit(&self, audit_id: &str) -> Result<Vec<Stage3Match>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT s3.id, s3.observation_id, s3.bank_key, s3.matched_row_id, s3.match_score,
                    s3.status, s3.final_text
             FROM stage3_matches s3
             JOIN observations o ON o.id = s3.observation_id
             WHERE o.audit_id = ?1",
        )?;
        let rows = stmt.query_map([audit_id], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Stage3Match> {
        Ok(Stage3Match {
            id: row.get(0)?,
            observation_id: row.get(1)?,
            bank_key: row.get(2)?,
            matched_row_id: row.get(3)?,
            match_score: row.get(4)?,
            status: Stage3MatchStatus::from_sql(&row.get::<_, String>(5)?)?,
            final_text: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::audits::AuditsRepository;
    use crate::repositories::observations::ObservationsRepository;
    use crate::repositories::projects::ProjectsRepository;
    use crate::repositories::sites::SitesRepository;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:"), "test-passphrase").unwrap()
    }

    fn test_observation(db: &Database) -> (String, String) {
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
        let audit = AuditsRepository::new(db)
            .create(&site.id, &user_id, "2026-07-04", None)
            .unwrap();
        let observation = ObservationsRepository::new(db)
            .create(&audit.id, "Fire extinguisher missing seal", None)
            .unwrap();
        (audit.id, observation.id)
    }

    #[test]
    fn get_or_create_starts_escalated() {
        let db = test_db();
        let (_, observation_id) = test_observation(&db);
        let repo = Stage3MatchesRepository::new(&db);

        let created = repo.get_or_create(&observation_id).unwrap();
        assert_eq!(created.status, Stage3MatchStatus::Escalated);
        assert!(created.final_text.is_none());
    }

    #[test]
    fn get_or_create_is_idempotent() {
        let db = test_db();
        let (_, observation_id) = test_observation(&db);
        let repo = Stage3MatchesRepository::new(&db);

        let first = repo.get_or_create(&observation_id).unwrap();
        let second = repo.get_or_create(&observation_id).unwrap();
        assert_eq!(first.id, second.id);
    }

    #[test]
    fn mark_matched_sets_all_match_fields() {
        let db = test_db();
        let (_, observation_id) = test_observation(&db);
        let repo = Stage3MatchesRepository::new(&db);

        let updated = repo
            .mark_matched(
                &observation_id,
                "fire_extinguisher",
                "row-1",
                82.5,
                "Replace the seal.",
            )
            .unwrap();

        assert_eq!(updated.status, Stage3MatchStatus::Matched);
        assert_eq!(updated.bank_key.as_deref(), Some("fire_extinguisher"));
        assert_eq!(updated.match_score, Some(82.5));
        assert_eq!(updated.final_text.as_deref(), Some("Replace the seal."));
    }

    #[test]
    fn mark_written_clears_match_fields() {
        let db = test_db();
        let (_, observation_id) = test_observation(&db);
        let repo = Stage3MatchesRepository::new(&db);
        repo.mark_matched(&observation_id, "bank", "row", 90.0, "Matched text")
            .unwrap();

        let updated = repo
            .mark_written(&observation_id, "My own wording")
            .unwrap();

        assert_eq!(updated.status, Stage3MatchStatus::Written);
        assert!(updated.bank_key.is_none());
        assert!(updated.match_score.is_none());
        assert_eq!(updated.final_text.as_deref(), Some("My own wording"));
    }

    #[test]
    fn mark_escalated_clears_everything() {
        let db = test_db();
        let (_, observation_id) = test_observation(&db);
        let repo = Stage3MatchesRepository::new(&db);
        repo.mark_matched(&observation_id, "bank", "row", 90.0, "Matched text")
            .unwrap();

        let updated = repo.mark_escalated(&observation_id).unwrap();

        assert_eq!(updated.status, Stage3MatchStatus::Escalated);
        assert!(updated.final_text.is_none());
    }

    #[test]
    fn update_final_text_preserves_status_and_bank_key() {
        let db = test_db();
        let (_, observation_id) = test_observation(&db);
        let repo = Stage3MatchesRepository::new(&db);
        repo.mark_matched(&observation_id, "bank", "row", 90.0, "Original text")
            .unwrap();

        let updated = repo
            .update_final_text(&observation_id, "Edited text")
            .unwrap();

        assert_eq!(updated.status, Stage3MatchStatus::Matched);
        assert_eq!(updated.bank_key.as_deref(), Some("bank"));
        assert_eq!(updated.final_text.as_deref(), Some("Edited text"));
    }

    #[test]
    fn list_for_audit_scopes_correctly() {
        let db = test_db();
        let (audit_id, observation_id) = test_observation(&db);
        let repo = Stage3MatchesRepository::new(&db);
        repo.get_or_create(&observation_id).unwrap();

        let rows = repo.list_for_audit(&audit_id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].observation_id, observation_id);
    }
}
