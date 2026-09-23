use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditStatus {
    Draft,
    Stage2,
    Stage3,
    Finalized,
}

impl AuditStatus {
    fn as_sql(self) -> &'static str {
        match self {
            AuditStatus::Draft => "draft",
            AuditStatus::Stage2 => "stage2",
            AuditStatus::Stage3 => "stage3",
            AuditStatus::Finalized => "finalized",
        }
    }

    fn from_sql(value: &str) -> rusqlite::Result<Self> {
        match value {
            "draft" => Ok(AuditStatus::Draft),
            "stage2" => Ok(AuditStatus::Stage2),
            "stage3" => Ok(AuditStatus::Stage3),
            "finalized" => Ok(AuditStatus::Finalized),
            other => Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("unknown audit status '{other}'"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Audit {
    pub id: String,
    pub site_id: String,
    pub auditor_user_id: String,
    pub audit_date: String,
    pub status: AuditStatus,
    pub scope_notes: Option<String>,
}

/// The audit workflow this repository tracks is Stage 1 → 2 → 3 →
/// finalized, matching the User Journey (§4). `advance_status` only allows
/// the forward transitions the UI actually exposes — it isn't a general
/// "set any status" setter, so an invalid jump is a bug caught here rather
/// than in the UI layer.
pub struct AuditsRepository<'a> {
    db: &'a Database,
}

impl<'a> AuditsRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        site_id: &str,
        auditor_user_id: &str,
        audit_date: &str,
        scope_notes: Option<&str>,
    ) -> Result<Audit, DbError> {
        let id = Uuid::new_v4().to_string();
        log::debug!("creating audit {id} for site {site_id}");
        self.db.conn().execute(
            "INSERT INTO audits (id, site_id, auditor_user_id, audit_date, scope_notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, site_id, auditor_user_id, audit_date, scope_notes],
        )?;
        self.get(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: &str) -> Result<Option<Audit>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, site_id, auditor_user_id, audit_date, status, scope_notes
                 FROM audits WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list_for_site(&self, site_id: &str) -> Result<Vec<Audit>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, site_id, auditor_user_id, audit_date, status, scope_notes
             FROM audits WHERE site_id = ?1",
        )?;
        let rows = stmt.query_map([site_id], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Advances an audit exactly one step forward (Draft -> Stage2 -> Stage3
    /// -> Finalized). Returns `DbError::NotFound` if `id` doesn't exist and
    /// the underlying SQL `CHECK` constraint rejects anything but the four
    /// known statuses — there is deliberately no way to skip a stage or move
    /// backward through this method.
    pub fn advance_status(&self, id: &str) -> Result<Audit, DbError> {
        let current = self.get(id)?.ok_or(DbError::NotFound)?;
        let next = match current.status {
            AuditStatus::Draft => AuditStatus::Stage2,
            AuditStatus::Stage2 => AuditStatus::Stage3,
            AuditStatus::Stage3 => AuditStatus::Finalized,
            AuditStatus::Finalized => AuditStatus::Finalized,
        };
        log::info!("advancing audit {id}: {:?} -> {:?}", current.status, next);
        self.db.conn().execute(
            "UPDATE audits SET status = ?1 WHERE id = ?2",
            params![next.as_sql(), id],
        )?;
        self.get(id)?.ok_or(DbError::NotFound)
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Audit> {
        Ok(Audit {
            id: row.get(0)?,
            site_id: row.get(1)?,
            auditor_user_id: row.get(2)?,
            audit_date: row.get(3)?,
            status: AuditStatus::from_sql(&row.get::<_, String>(4)?)?,
            scope_notes: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::projects::ProjectsRepository;
    use crate::repositories::sites::SitesRepository;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:"), "test-passphrase").unwrap()
    }

    /// User creation/password-hashing belongs to Module 3 (Authentication).
    /// This inserts the bare row `audits.auditor_user_id`'s foreign key
    /// needs so Module 2's own tests don't depend on unwritten code.
    fn insert_test_user(db: &Database) -> String {
        let id = Uuid::new_v4().to_string();
        db.conn()
            .execute(
                "INSERT INTO users (id, username, password_hash, role) VALUES (?1, ?2, 'x', 'auditor')",
                params![id, format!("auditor-{id}")],
            )
            .unwrap();
        id
    }

    fn test_site(db: &Database) -> String {
        let project = ProjectsRepository::new(db).create("Project", None).unwrap();
        SitesRepository::new(db)
            .create(&project.id, "Site", None)
            .unwrap()
            .id
    }

    #[test]
    fn new_audit_starts_in_draft() {
        let db = test_db();
        let site_id = test_site(&db);
        let user_id = insert_test_user(&db);
        let repo = AuditsRepository::new(&db);

        let audit = repo
            .create(
                &site_id,
                &user_id,
                "2026-07-04",
                Some("Routine quarterly walkaround"),
            )
            .unwrap();

        assert_eq!(audit.status, AuditStatus::Draft);
        assert_eq!(audit.site_id, site_id);
    }

    #[test]
    fn advance_status_walks_through_every_stage_in_order() {
        let db = test_db();
        let site_id = test_site(&db);
        let user_id = insert_test_user(&db);
        let repo = AuditsRepository::new(&db);
        let audit = repo.create(&site_id, &user_id, "2026-07-04", None).unwrap();

        let after_1 = repo.advance_status(&audit.id).unwrap();
        assert_eq!(after_1.status, AuditStatus::Stage2);

        let after_2 = repo.advance_status(&audit.id).unwrap();
        assert_eq!(after_2.status, AuditStatus::Stage3);

        let after_3 = repo.advance_status(&audit.id).unwrap();
        assert_eq!(after_3.status, AuditStatus::Finalized);
    }

    #[test]
    fn advance_status_is_a_no_op_once_finalized() {
        let db = test_db();
        let site_id = test_site(&db);
        let user_id = insert_test_user(&db);
        let repo = AuditsRepository::new(&db);
        let audit = repo.create(&site_id, &user_id, "2026-07-04", None).unwrap();

        for _ in 0..3 {
            repo.advance_status(&audit.id).unwrap();
        }
        let finalized_again = repo.advance_status(&audit.id).unwrap();
        assert_eq!(finalized_again.status, AuditStatus::Finalized);
    }

    #[test]
    fn creating_an_audit_for_unknown_site_fails_foreign_key_check() {
        let db = test_db();
        let user_id = insert_test_user(&db);
        let repo = AuditsRepository::new(&db);
        let result = repo.create("does-not-exist", &user_id, "2026-07-04", None);
        assert!(result.is_err());
    }
}
