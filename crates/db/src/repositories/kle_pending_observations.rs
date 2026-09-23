use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationPendingStatus {
    Pending,
    Approved,
    Rejected,
    Merged,
}

impl ObservationPendingStatus {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Merged => "merged",
        }
    }

    fn from_sql(value: &str) -> rusqlite::Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "merged" => Ok(Self::Merged),
            other => Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("unknown pending-observation status '{other}'"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingObservation {
    pub id: i64,
    pub status: ObservationPendingStatus,
    pub bank_key_guess: Option<String>,
    pub source_entry_hash: String,
    /// Opaque JSON blob (the KLE crate defines and (de)serializes its own
    /// payload shape — this repository never interprets it).
    pub payload: String,
    pub created_at: String,
}

/// CRUD for `kle_pending_observations` — the staging queue for observations
/// that scored below the novelty threshold (Module 8). Storage only; the
/// decision of *when* to stage something and what to do once approved
/// belongs to `aditup-kle`.
pub struct KlePendingObservationsRepository<'a> {
    db: &'a Database,
}

impl<'a> KlePendingObservationsRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Inserts a new pending observation, or — if one already exists for
    /// this `source_entry_hash` and is still `Pending` — updates it in
    /// place instead of creating a duplicate. A hash that was already
    /// resolved (approved/rejected/merged) starts a fresh pending row if
    /// captured again, since its content may have changed since resolution.
    pub fn upsert(
        &self,
        source_entry_hash: &str,
        bank_key_guess: Option<&str>,
        payload: &str,
    ) -> Result<PendingObservation, DbError> {
        if let Some(existing) = self.get_pending_by_source_hash(source_entry_hash)? {
            self.db.conn().execute(
                "UPDATE kle_pending_observations SET bank_key_guess = ?1, payload = ?2 WHERE id = ?3",
                params![bank_key_guess, payload, existing.id],
            )?;
            return self.get(existing.id)?.ok_or(DbError::NotFound);
        }

        log::debug!("staging new pending observation for entry_hash {source_entry_hash}");
        self.db.conn().execute(
            "INSERT INTO kle_pending_observations (bank_key_guess, source_entry_hash, payload)
             VALUES (?1, ?2, ?3)",
            params![bank_key_guess, source_entry_hash, payload],
        )?;
        let id = self.db.conn().last_insert_rowid();
        self.get(id)?.ok_or(DbError::NotFound)
    }

    pub fn get(&self, id: i64) -> Result<Option<PendingObservation>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, status, bank_key_guess, source_entry_hash, payload, created_at
                 FROM kle_pending_observations WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    fn get_pending_by_source_hash(
        &self,
        source_entry_hash: &str,
    ) -> Result<Option<PendingObservation>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, status, bank_key_guess, source_entry_hash, payload, created_at
                 FROM kle_pending_observations
                 WHERE source_entry_hash = ?1 AND status = 'pending'",
                [source_entry_hash],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list_by_status(
        &self,
        status: ObservationPendingStatus,
    ) -> Result<Vec<PendingObservation>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, status, bank_key_guess, source_entry_hash, payload, created_at
             FROM kle_pending_observations WHERE status = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([status.as_sql()], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn set_status(&self, id: i64, status: ObservationPendingStatus) -> Result<(), DbError> {
        let updated = self.db.conn().execute(
            "UPDATE kle_pending_observations SET status = ?1 WHERE id = ?2",
            params![status.as_sql(), id],
        )?;
        if updated == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<PendingObservation> {
        Ok(PendingObservation {
            id: row.get(0)?,
            status: ObservationPendingStatus::from_sql(&row.get::<_, String>(1)?)?,
            bank_key_guess: row.get(2)?,
            source_entry_hash: row.get(3)?,
            payload: row.get(4)?,
            created_at: row.get(5)?,
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
    fn upsert_creates_a_new_pending_row() {
        let db = test_db();
        let repo = KlePendingObservationsRepository::new(&db);
        let created = repo.upsert("hash-1", Some("bank_a"), "{}").unwrap();
        assert_eq!(created.status, ObservationPendingStatus::Pending);
        assert_eq!(created.source_entry_hash, "hash-1");
    }

    #[test]
    fn upsert_on_the_same_hash_updates_the_existing_pending_row_not_a_duplicate() {
        let db = test_db();
        let repo = KlePendingObservationsRepository::new(&db);
        let first = repo.upsert("hash-1", Some("bank_a"), "{\"v\":1}").unwrap();
        let second = repo.upsert("hash-1", Some("bank_b"), "{\"v\":2}").unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(second.bank_key_guess.as_deref(), Some("bank_b"));
        assert_eq!(second.payload, "{\"v\":2}");
        assert_eq!(
            repo.list_by_status(ObservationPendingStatus::Pending)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn upsert_after_resolution_starts_a_fresh_pending_row() {
        let db = test_db();
        let repo = KlePendingObservationsRepository::new(&db);
        let first = repo.upsert("hash-1", None, "{}").unwrap();
        repo.set_status(first.id, ObservationPendingStatus::Approved)
            .unwrap();

        let second = repo.upsert("hash-1", None, "{}").unwrap();

        assert_ne!(first.id, second.id);
        assert_eq!(second.status, ObservationPendingStatus::Pending);
    }

    #[test]
    fn list_by_status_filters_correctly() {
        let db = test_db();
        let repo = KlePendingObservationsRepository::new(&db);
        let a = repo.upsert("hash-a", None, "{}").unwrap();
        repo.upsert("hash-b", None, "{}").unwrap();
        repo.set_status(a.id, ObservationPendingStatus::Rejected)
            .unwrap();

        assert_eq!(
            repo.list_by_status(ObservationPendingStatus::Pending)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            repo.list_by_status(ObservationPendingStatus::Rejected)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn set_status_on_unknown_id_errors() {
        let db = test_db();
        let repo = KlePendingObservationsRepository::new(&db);
        assert!(matches!(
            repo.set_status(9999, ObservationPendingStatus::Approved),
            Err(DbError::NotFound)
        ));
    }
}
