use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub location: Option<String>,
}

/// CRUD access to `sites`, always scoped to a parent project.
pub struct SitesRepository<'a> {
    db: &'a Database,
}

impl<'a> SitesRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        project_id: &str,
        name: &str,
        location: Option<&str>,
    ) -> Result<Site, DbError> {
        let id = Uuid::new_v4().to_string();
        log::debug!("creating site '{name}' ({id}) under project {project_id}");
        self.db.conn().execute(
            "INSERT INTO sites (id, project_id, name, location) VALUES (?1, ?2, ?3, ?4)",
            params![id, project_id, name, location],
        )?;
        self.get(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: &str) -> Result<Option<Site>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, project_id, name, location FROM sites WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list_for_project(&self, project_id: &str) -> Result<Vec<Site>, DbError> {
        let conn = self.db.conn();
        let mut stmt =
            conn.prepare("SELECT id, project_id, name, location FROM sites WHERE project_id = ?1")?;
        let rows = stmt.query_map([project_id], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Site> {
        Ok(Site {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            location: row.get(3)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::projects::ProjectsRepository;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:"), "test-passphrase").unwrap()
    }

    #[test]
    fn create_then_get_round_trips() {
        let db = test_db();
        let project = ProjectsRepository::new(&db)
            .create("Project", None)
            .unwrap();
        let sites = SitesRepository::new(&db);

        let created = sites
            .create(&project.id, "Main Warehouse", Some("Bay 3"))
            .unwrap();
        let fetched = sites.get(&created.id).unwrap().unwrap();

        assert_eq!(created, fetched);
        assert_eq!(fetched.project_id, project.id);
    }

    #[test]
    fn list_for_project_only_returns_that_projects_sites() {
        let db = test_db();
        let projects = ProjectsRepository::new(&db);
        let sites = SitesRepository::new(&db);

        let project_a = projects.create("A", None).unwrap();
        let project_b = projects.create("B", None).unwrap();
        sites.create(&project_a.id, "A-Site-1", None).unwrap();
        sites.create(&project_a.id, "A-Site-2", None).unwrap();
        sites.create(&project_b.id, "B-Site-1", None).unwrap();

        assert_eq!(sites.list_for_project(&project_a.id).unwrap().len(), 2);
        assert_eq!(sites.list_for_project(&project_b.id).unwrap().len(), 1);
    }

    #[test]
    fn creating_a_site_under_unknown_project_fails_foreign_key_check() {
        let db = test_db();
        let sites = SitesRepository::new(&db);
        let result = sites.create("does-not-exist", "Orphan Site", None);
        assert!(result.is_err());
    }
}
