use rusqlite::{params, OptionalExtension};

use crate::connection::Database;
use crate::error::DbError;

/// Generic key/value access to `admin_settings`. This is where Module 3
/// stores `admin_pin_hash` and `recovery_key_hash`; later modules (14
/// Settings) use the same table for scoring weights/thresholds. The table
/// intentionally has no schema beyond key/value — validating what a given
/// key's value should look like is each caller's responsibility, not this
/// repository's.
pub struct AdminSettingsRepository<'a> {
    db: &'a Database,
}

impl<'a> AdminSettingsRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT value FROM admin_settings WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(DbError::from)
    }

    /// Insert-or-replace — callers decide whether "already set" should be an
    /// error (e.g. `aditup-security` refuses to overwrite an existing Admin
    /// PIN via `set_admin_pin`; `change_admin_pin` calls this only after
    /// verifying the old one).
    pub fn set(&self, key: &str, value: &str) -> Result<(), DbError> {
        self.db.conn().execute(
            "INSERT INTO admin_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), DbError> {
        self.db
            .conn()
            .execute("DELETE FROM admin_settings WHERE key = ?1", [key])?;
        Ok(())
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
    fn get_on_missing_key_returns_none() {
        let db = test_db();
        let repo = AdminSettingsRepository::new(&db);
        assert!(repo.get("admin_pin_hash").unwrap().is_none());
    }

    #[test]
    fn set_then_get_round_trips() {
        let db = test_db();
        let repo = AdminSettingsRepository::new(&db);
        repo.set("admin_pin_hash", "hash-value").unwrap();
        assert_eq!(
            repo.get("admin_pin_hash").unwrap().as_deref(),
            Some("hash-value")
        );
    }

    #[test]
    fn set_overwrites_an_existing_value() {
        let db = test_db();
        let repo = AdminSettingsRepository::new(&db);
        repo.set("admin_pin_hash", "first").unwrap();
        repo.set("admin_pin_hash", "second").unwrap();
        assert_eq!(
            repo.get("admin_pin_hash").unwrap().as_deref(),
            Some("second")
        );
    }

    #[test]
    fn delete_removes_the_key() {
        let db = test_db();
        let repo = AdminSettingsRepository::new(&db);
        repo.set("recovery_key_hash", "value").unwrap();
        repo.delete("recovery_key_hash").unwrap();
        assert!(repo.get("recovery_key_hash").unwrap().is_none());
    }
}
