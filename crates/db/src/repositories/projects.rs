use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub client: Option<String>,
    pub created_at: String,
    pub archived_at: Option<String>,
}

/// CRUD access to `projects`. One repository per aggregate root, per §6/§11
/// of the architecture — nothing outside this file issues SQL against the
/// `projects` table.
pub struct ProjectsRepository<'a> {
    db: &'a Database,
}

impl<'a> ProjectsRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, name: &str, client: Option<&str>) -> Result<Project, DbError> {
        let id = Uuid::new_v4().to_string();
        log::debug!("creating project '{name}' ({id})");
        self.db.conn().execute(
            "INSERT INTO projects (id, name, client) VALUES (?1, ?2, ?3)",
            params![id, name, client],
        )?;
        self.get(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: &str) -> Result<Option<Project>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, name, client, created_at, archived_at FROM projects WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// Projects not yet archived, most recently created first.
    pub fn list_active(&self) -> Result<Vec<Project>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, client, created_at, archived_at
             FROM projects
             WHERE archived_at IS NULL
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn archive(&self, id: &str) -> Result<(), DbError> {
        log::info!("archiving project {id}");
        let updated = self.db.conn().execute(
            "UPDATE projects
             SET archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND archived_at IS NULL",
            [id],
        )?;
        if updated == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Project> {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            client: row.get(2)?,
            created_at: row.get(3)?,
            archived_at: row.get(4)?,
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
    fn create_then_get_round_trips() {
        let db = test_db();
        let repo = ProjectsRepository::new(&db);

        let created = repo
            .create("Riverside Plant Audit", Some("Acme Manufacturing"))
            .unwrap();
        let fetched = repo.get(&created.id).unwrap().unwrap();

        assert_eq!(created, fetched);
        assert_eq!(fetched.name, "Riverside Plant Audit");
        assert_eq!(fetched.client.as_deref(), Some("Acme Manufacturing"));
        assert!(fetched.archived_at.is_none());
    }

    #[test]
    fn get_returns_none_for_unknown_id() {
        let db = test_db();
        let repo = ProjectsRepository::new(&db);
        assert!(repo.get("does-not-exist").unwrap().is_none());
    }

    #[test]
    fn list_active_excludes_archived_projects() {
        let db = test_db();
        let repo = ProjectsRepository::new(&db);

        let keep = repo.create("Keep Me", None).unwrap();
        let drop = repo.create("Archive Me", None).unwrap();
        repo.archive(&drop.id).unwrap();

        let active = repo.list_active().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, keep.id);
    }

    #[test]
    fn archive_is_idempotent_and_errors_second_time() {
        let db = test_db();
        let repo = ProjectsRepository::new(&db);
        let project = repo.create("One-shot", None).unwrap();

        repo.archive(&project.id).unwrap();
        let second_attempt = repo.archive(&project.id);

        assert!(matches!(second_attempt, Err(DbError::NotFound)));
    }
}
