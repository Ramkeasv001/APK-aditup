use std::path::Path;

use rusqlite::Connection;

use crate::error::DbError;
use crate::migrations;

/// An open, encrypted, migrated database handle. This is the only way to
/// get a `Connection` out of this crate — nothing outside `db` should ever
/// construct a raw `rusqlite::Connection` against a project file, so the
/// SQLCipher key + `foreign_keys` + `WAL` pragmas can never accidentally be
/// skipped.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens `path` (creating it if it doesn't exist), applies the SQLCipher
    /// key, verifies the key is actually correct, enables WAL + foreign keys,
    /// and brings the schema up to date.
    ///
    /// `passphrase` is used as-is as the SQLCipher key. Deriving it from a
    /// user's master password (Argon2id) is Module 3's job, not this crate's
    /// — `db` only knows how to use a key, not how to produce one.
    pub fn open(path: &Path, passphrase: &str) -> Result<Self, DbError> {
        log::debug!("opening database at {}", path.display());
        let conn = Connection::open(path)?;
        Self::init(conn, passphrase)
    }

    fn init(conn: Connection, passphrase: &str) -> Result<Self, DbError> {
        // PRAGMA key must be the first thing executed on the connection.
        conn.pragma_update(None, "key", passphrase)?;

        // SQLCipher doesn't validate the key eagerly — a wrong key only
        // surfaces once you try to actually read encrypted pages. Force
        // that check now so callers get a clear error immediately instead
        // of a confusing failure on some unrelated later query.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
            .map_err(|_| DbError::InvalidKey)?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;

        migrations::apply_all(&conn)?;

        log::info!("database ready");
        Ok(Self { conn })
    }

    /// Raw access for repositories in this crate. Intentionally not `pub` —
    /// external crates go through repository types, never the connection.
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_memory_and_runs_migrations() {
        let db = Database::open(Path::new(":memory:"), "test-passphrase").unwrap();
        let table_count: i64 = db
            .conn()
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(table_count > 0);
    }

    #[test]
    fn rejects_wrong_passphrase_on_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.sqlite");

        {
            let _db = Database::open(&path, "correct-horse-battery-staple").unwrap();
        }

        let reopened = Database::open(&path, "wrong-password");
        assert!(matches!(reopened, Err(DbError::InvalidKey)));
    }

    #[test]
    fn reopens_successfully_with_correct_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.sqlite");

        {
            let _db = Database::open(&path, "correct-horse-battery-staple").unwrap();
        }

        let reopened = Database::open(&path, "correct-horse-battery-staple");
        assert!(reopened.is_ok());
    }
}
