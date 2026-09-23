use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Auditor,
    Admin,
}

impl UserRole {
    fn as_sql(self) -> &'static str {
        match self {
            UserRole::Auditor => "auditor",
            UserRole::Admin => "admin",
        }
    }

    fn from_sql(value: &str) -> rusqlite::Result<Self> {
        match value {
            "auditor" => Ok(UserRole::Auditor),
            "admin" => Ok(UserRole::Admin),
            other => Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("unknown user role '{other}'"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    /// Opaque Argon2id PHC string. This crate never hashes or verifies
    /// passwords itself — that's `aditup-security`'s job — it only stores
    /// and retrieves whatever string it's given.
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: String,
}

/// CRUD access to `users`. Password hashing/verification lives in
/// `aditup-security` (Module 3); this repository is pure storage.
pub struct UsersRepository<'a> {
    db: &'a Database,
}

impl<'a> UsersRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        username: &str,
        password_hash: &str,
        role: UserRole,
    ) -> Result<User, DbError> {
        let id = Uuid::new_v4().to_string();
        log::debug!("creating user '{username}' ({id}, role={:?})", role);
        self.db.conn().execute(
            "INSERT INTO users (id, username, password_hash, role) VALUES (?1, ?2, ?3, ?4)",
            params![id, username, password_hash, role.as_sql()],
        )?;
        self.get(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: &str) -> Result<Option<User>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, username, password_hash, role, created_at FROM users WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn get_by_username(&self, username: &str) -> Result<Option<User>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, username, password_hash, role, created_at FROM users WHERE username = ?1",
                [username],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// Total number of users — `0` means the app hasn't been set up yet.
    pub fn count(&self) -> Result<i64, DbError> {
        self.db
            .conn()
            .query_row("SELECT count(*) FROM users", [], |row| row.get(0))
            .map_err(DbError::from)
    }

    pub fn update_password_hash(&self, id: &str, new_password_hash: &str) -> Result<(), DbError> {
        let updated = self.db.conn().execute(
            "UPDATE users SET password_hash = ?1 WHERE id = ?2",
            params![new_password_hash, id],
        )?;
        if updated == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<User> {
        Ok(User {
            id: row.get(0)?,
            username: row.get(1)?,
            password_hash: row.get(2)?,
            role: UserRole::from_sql(&row.get::<_, String>(3)?)?,
            created_at: row.get(4)?,
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
        let repo = UsersRepository::new(&db);

        let created = repo
            .create("hse.admin", "argon2-hash", UserRole::Admin)
            .unwrap();
        let fetched = repo.get(&created.id).unwrap().unwrap();

        assert_eq!(created, fetched);
        assert_eq!(fetched.role, UserRole::Admin);
    }

    #[test]
    fn get_by_username_finds_the_right_user() {
        let db = test_db();
        let repo = UsersRepository::new(&db);
        repo.create("alice", "hash-a", UserRole::Auditor).unwrap();
        let bob = repo.create("bob", "hash-b", UserRole::Auditor).unwrap();

        let found = repo.get_by_username("bob").unwrap().unwrap();
        assert_eq!(found.id, bob.id);
        assert!(repo.get_by_username("carol").unwrap().is_none());
    }

    #[test]
    fn count_reflects_number_of_users() {
        let db = test_db();
        let repo = UsersRepository::new(&db);
        assert_eq!(repo.count().unwrap(), 0);
        repo.create("alice", "hash", UserRole::Admin).unwrap();
        assert_eq!(repo.count().unwrap(), 1);
    }

    #[test]
    fn duplicate_username_is_rejected() {
        let db = test_db();
        let repo = UsersRepository::new(&db);
        repo.create("alice", "hash-1", UserRole::Auditor).unwrap();
        let result = repo.create("alice", "hash-2", UserRole::Auditor);
        assert!(result.is_err());
    }

    #[test]
    fn update_password_hash_changes_the_stored_value() {
        let db = test_db();
        let repo = UsersRepository::new(&db);
        let user = repo.create("alice", "old-hash", UserRole::Admin).unwrap();

        repo.update_password_hash(&user.id, "new-hash").unwrap();

        let fetched = repo.get(&user.id).unwrap().unwrap();
        assert_eq!(fetched.password_hash, "new-hash");
    }

    #[test]
    fn update_password_hash_on_unknown_id_errors() {
        let db = test_db();
        let repo = UsersRepository::new(&db);
        assert!(matches!(
            repo.update_password_hash("does-not-exist", "x"),
            Err(DbError::NotFound)
        ));
    }
}
