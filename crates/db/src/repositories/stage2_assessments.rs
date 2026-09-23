use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    fn from_sql(value: &str) -> rusqlite::Result<Self> {
        match value {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            other => Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("unknown risk level '{other}'"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage2Assessment {
    pub id: String,
    pub observation_id: String,
    pub risk_level: RiskLevel,
    pub category: Option<String>,
    pub department: Option<String>,
    pub equipment: Option<String>,
    pub legal_clause_id: Option<String>,
}

/// CRUD for `stage2_assessments` (Module 10d). `observation_id` is unique
/// per schema — an auditor revising their risk rating before saving is the
/// common case, so `upsert` replaces any existing row for that observation
/// rather than erroring on a second save.
pub struct Stage2AssessmentsRepository<'a> {
    db: &'a Database,
}

impl<'a> Stage2AssessmentsRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert(
        &self,
        observation_id: &str,
        risk_level: RiskLevel,
        category: Option<&str>,
        department: Option<&str>,
        equipment: Option<&str>,
        legal_clause_id: Option<&str>,
    ) -> Result<Stage2Assessment, DbError> {
        let id = Uuid::new_v4().to_string();
        self.db.conn().execute(
            "INSERT INTO stage2_assessments
                (id, observation_id, risk_level, category, department, equipment, legal_clause_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(observation_id) DO UPDATE SET
                risk_level = excluded.risk_level,
                category = excluded.category,
                department = excluded.department,
                equipment = excluded.equipment,
                legal_clause_id = excluded.legal_clause_id",
            params![
                id,
                observation_id,
                risk_level.as_sql(),
                category,
                department,
                equipment,
                legal_clause_id
            ],
        )?;
        self.get_for_observation(observation_id)?
            .ok_or(DbError::NotFound)
    }

    pub fn get_for_observation(
        &self,
        observation_id: &str,
    ) -> Result<Option<Stage2Assessment>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, observation_id, risk_level, category, department, equipment, legal_clause_id
                 FROM stage2_assessments WHERE observation_id = ?1",
                [observation_id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// Every assessment for observations belonging to `audit_id` — joins
    /// through `observations` since this table only references
    /// `observation_id` directly, not the audit.
    pub fn list_for_audit(&self, audit_id: &str) -> Result<Vec<Stage2Assessment>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT s2.id, s2.observation_id, s2.risk_level, s2.category, s2.department,
                    s2.equipment, s2.legal_clause_id
             FROM stage2_assessments s2
             JOIN observations o ON o.id = s2.observation_id
             WHERE o.audit_id = ?1",
        )?;
        let rows = stmt.query_map([audit_id], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Stage2Assessment> {
        Ok(Stage2Assessment {
            id: row.get(0)?,
            observation_id: row.get(1)?,
            risk_level: RiskLevel::from_sql(&row.get::<_, String>(2)?)?,
            category: row.get(3)?,
            department: row.get(4)?,
            equipment: row.get(5)?,
            legal_clause_id: row.get(6)?,
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
    fn upsert_creates_then_get_for_observation_round_trips() {
        let db = test_db();
        let (_, observation_id) = test_observation(&db);
        let repo = Stage2AssessmentsRepository::new(&db);

        let created = repo
            .upsert(
                &observation_id,
                RiskLevel::High,
                Some("Fire Protection"),
                None,
                None,
                None,
            )
            .unwrap();
        let fetched = repo.get_for_observation(&observation_id).unwrap().unwrap();

        assert_eq!(created, fetched);
        assert_eq!(fetched.risk_level, RiskLevel::High);
    }

    #[test]
    fn upsert_twice_replaces_rather_than_duplicates() {
        let db = test_db();
        let (_, observation_id) = test_observation(&db);
        let repo = Stage2AssessmentsRepository::new(&db);

        let first = repo
            .upsert(&observation_id, RiskLevel::Low, None, None, None, None)
            .unwrap();
        let second = repo
            .upsert(
                &observation_id,
                RiskLevel::Critical,
                Some("Fire Protection"),
                None,
                None,
                None,
            )
            .unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(second.risk_level, RiskLevel::Critical);
        assert_eq!(second.category.as_deref(), Some("Fire Protection"));
    }

    #[test]
    fn get_for_observation_with_no_assessment_is_none() {
        let db = test_db();
        let (_, observation_id) = test_observation(&db);
        let repo = Stage2AssessmentsRepository::new(&db);
        assert!(repo.get_for_observation(&observation_id).unwrap().is_none());
    }

    #[test]
    fn list_for_audit_scopes_correctly() {
        let db = test_db();
        let (audit_id, observation_id) = test_observation(&db);
        let repo = Stage2AssessmentsRepository::new(&db);
        repo.upsert(&observation_id, RiskLevel::Medium, None, None, None, None)
            .unwrap();

        let rows = repo.list_for_audit(&audit_id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].observation_id, observation_id);
    }
}
