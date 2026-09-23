use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeRequestStatus {
    Pending,
    Merged,
    KeptBoth,
    Discarded,
}

impl MergeRequestStatus {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Merged => "merged",
            Self::KeptBoth => "kept_both",
            Self::Discarded => "discarded",
        }
    }

    fn from_sql(value: &str) -> rusqlite::Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "merged" => Ok(Self::Merged),
            "kept_both" => Ok(Self::KeptBoth),
            "discarded" => Ok(Self::Discarded),
            other => Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("unknown merge-request status '{other}'"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeRequest {
    pub id: i64,
    pub candidate_a_id: i64,
    pub candidate_b_id: i64,
    pub similarity: f64,
    pub status: MergeRequestStatus,
}

/// CRUD for `kle_merge_requests` — suspected-duplicate pairs among pending
/// observations, surfaced by `aditup-kle`'s clustering pass for Admin
/// resolution. Two pending items are never auto-merged; this table is
/// exactly the human checkpoint that prevents that.
pub struct KleMergeRequestsRepository<'a> {
    db: &'a Database,
}

impl<'a> KleMergeRequestsRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Creates a request, unless an unresolved one already exists for this
    /// unordered pair — clustering can run repeatedly without piling up
    /// duplicate requests for the same two candidates.
    pub fn create_if_absent(
        &self,
        candidate_a_id: i64,
        candidate_b_id: i64,
        similarity: f64,
    ) -> Result<MergeRequest, DbError> {
        if let Some(existing) = self.find_pending_pair(candidate_a_id, candidate_b_id)? {
            return Ok(existing);
        }
        self.db.conn().execute(
            "INSERT INTO kle_merge_requests (candidate_a_id, candidate_b_id, similarity) VALUES (?1, ?2, ?3)",
            params![candidate_a_id, candidate_b_id, similarity],
        )?;
        let id = self.db.conn().last_insert_rowid();
        self.get(id)?.ok_or(DbError::NotFound)
    }

    fn find_pending_pair(&self, a: i64, b: i64) -> Result<Option<MergeRequest>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, candidate_a_id, candidate_b_id, similarity, status FROM kle_merge_requests
                 WHERE status = 'pending'
                   AND ((candidate_a_id = ?1 AND candidate_b_id = ?2)
                        OR (candidate_a_id = ?2 AND candidate_b_id = ?1))",
                params![a, b],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn get(&self, id: i64) -> Result<Option<MergeRequest>, DbError> {
        self.db
            .conn()
            .query_row(
                "SELECT id, candidate_a_id, candidate_b_id, similarity, status FROM kle_merge_requests WHERE id = ?1",
                [id],
                Self::from_row,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list_by_status(&self, status: MergeRequestStatus) -> Result<Vec<MergeRequest>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, candidate_a_id, candidate_b_id, similarity, status FROM kle_merge_requests
             WHERE status = ?1 ORDER BY similarity DESC",
        )?;
        let rows = stmt.query_map([status.as_sql()], Self::from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn set_status(&self, id: i64, status: MergeRequestStatus) -> Result<(), DbError> {
        let updated = self.db.conn().execute(
            "UPDATE kle_merge_requests SET status = ?1 WHERE id = ?2",
            params![status.as_sql(), id],
        )?;
        if updated == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<MergeRequest> {
        Ok(MergeRequest {
            id: row.get(0)?,
            candidate_a_id: row.get(1)?,
            candidate_b_id: row.get(2)?,
            similarity: row.get(3)?,
            status: MergeRequestStatus::from_sql(&row.get::<_, String>(4)?)?,
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
    fn create_if_absent_creates_a_new_request() {
        let db = test_db();
        let repo = KleMergeRequestsRepository::new(&db);
        let created = repo.create_if_absent(1, 2, 0.9).unwrap();
        assert_eq!(created.status, MergeRequestStatus::Pending);
    }

    #[test]
    fn create_if_absent_is_idempotent_regardless_of_argument_order() {
        let db = test_db();
        let repo = KleMergeRequestsRepository::new(&db);
        let first = repo.create_if_absent(1, 2, 0.9).unwrap();
        let second = repo.create_if_absent(2, 1, 0.95).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(
            repo.list_by_status(MergeRequestStatus::Pending)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn create_if_absent_after_resolution_creates_a_new_request() {
        let db = test_db();
        let repo = KleMergeRequestsRepository::new(&db);
        let first = repo.create_if_absent(1, 2, 0.9).unwrap();
        repo.set_status(first.id, MergeRequestStatus::Merged)
            .unwrap();

        let second = repo.create_if_absent(1, 2, 0.9).unwrap();
        assert_ne!(first.id, second.id);
    }

    #[test]
    fn list_by_status_sorts_by_similarity_descending() {
        let db = test_db();
        let repo = KleMergeRequestsRepository::new(&db);
        repo.create_if_absent(1, 2, 0.7).unwrap();
        repo.create_if_absent(3, 4, 0.95).unwrap();

        let results = repo.list_by_status(MergeRequestStatus::Pending).unwrap();
        assert_eq!(results[0].similarity, 0.95);
        assert_eq!(results[1].similarity, 0.7);
    }

    #[test]
    fn set_status_on_unknown_id_errors() {
        let db = test_db();
        let repo = KleMergeRequestsRepository::new(&db);
        assert!(matches!(
            repo.set_status(9999, MergeRequestStatus::Discarded),
            Err(DbError::NotFound)
        ));
    }
}
