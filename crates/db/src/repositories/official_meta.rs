use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankVersion {
    pub bank_key: String,
    pub current_version: i64,
    pub published_at: Option<String>,
}

/// Read access to `official_meta` — shared by every repository that
/// publishes into a `bank_key`-versioned dataset (`observation_bank` in
/// Module 4, `recommendation_bank` here in Module 5, and later the keyword,
/// synonym, and KLE-publish paths).
pub struct OfficialMetaRepository<'a> {
    db: &'a Database,
}

impl<'a> OfficialMetaRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn get(&self, bank_key: &str) -> Result<Option<BankVersion>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT bank_key, current_version, published_at FROM official_meta WHERE bank_key = ?1",
                [bank_key],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<BankVersion> {
        Ok(BankVersion {
            bank_key: row.get(0)?,
            current_version: row.get(1)?,
            published_at: row.get(2)?,
        })
    }
}

/// Bumps (or creates) `bank_key`'s published version.
///
/// Takes a raw `&Connection` — which a `&rusqlite::Transaction` derefs to —
/// rather than `&Database`, so callers compose this inside their *own*
/// transaction alongside the content write it corresponds to. A bank's
/// published content and its version must commit or roll back together;
/// this must never be called as a separate, independent write.
pub(crate) fn bump_bank_version(conn: &Connection, bank_key: &str) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO official_meta (bank_key, current_version, published_at)
         VALUES (?1, 1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT(bank_key) DO UPDATE SET
            current_version = current_version + 1,
            published_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        [bank_key],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:"), "test-passphrase").unwrap()
    }

    #[test]
    fn get_on_unknown_bank_key_returns_none() {
        let db = test_db();
        let repo = OfficialMetaRepository::new(&db);
        assert!(repo.get("never_created").unwrap().is_none());
    }

    #[test]
    fn bump_creates_version_one_then_increments() {
        let db = test_db();
        bump_bank_version(db.conn(), "bank_a").unwrap();
        let first = OfficialMetaRepository::new(&db)
            .get("bank_a")
            .unwrap()
            .unwrap();
        assert_eq!(first.current_version, 1);

        bump_bank_version(db.conn(), "bank_a").unwrap();
        let second = OfficialMetaRepository::new(&db)
            .get("bank_a")
            .unwrap()
            .unwrap();
        assert_eq!(second.current_version, 2);
    }
}
