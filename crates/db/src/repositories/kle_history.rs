use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::connection::Database;
use crate::error::DbError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEvent {
    pub id: i64,
    pub entry_hash: String,
    pub timestamp: String,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionEvent {
    pub id: i64,
    pub entry_hash: String,
    pub timestamp: String,
    pub chosen_bank_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectionEvent {
    pub id: i64,
    pub entry_hash: String,
    pub timestamp: String,
    pub rejected_bank_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditEvent {
    pub id: i64,
    pub entry_hash: String,
    pub timestamp: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalEvent {
    pub id: i64,
    pub entry_hash: String,
    pub timestamp: String,
    pub admin_user_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceEvent {
    pub id: i64,
    pub entry_hash: String,
    pub timestamp: String,
    /// Opaque JSON blob — `aditup-kle` defines the breakdown shape.
    pub breakdown_json: String,
}

/// The full recorded lifecycle of one `entry_hash` across every KLE history
/// table — what Admin's "History Search" (§4) looks up.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryTimeline {
    pub searches: Vec<SearchEvent>,
    pub selections: Vec<SelectionEvent>,
    pub rejections: Vec<RejectionEvent>,
    pub edits: Vec<EditEvent>,
    pub approvals: Vec<ApprovalEvent>,
    pub confidence_snapshots: Vec<ConfidenceEvent>,
}

/// Append-only logging for all six `kle_history_*` tables, plus lookup by
/// `entry_hash`. Every Stage 3 search/selection/rejection/edit and every
/// Admin approval and confidence snapshot passes through here — storage
/// only, `aditup-kle` decides *when* each of these fires.
pub struct KleHistoryRepository<'a> {
    db: &'a Database,
}

impl<'a> KleHistoryRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn log_search(&self, entry_hash: &str, query: &str) -> Result<(), DbError> {
        self.db.conn().execute(
            "INSERT INTO kle_history_search (entry_hash, query) VALUES (?1, ?2)",
            params![entry_hash, query],
        )?;
        Ok(())
    }

    pub fn log_selection(
        &self,
        entry_hash: &str,
        chosen_bank_key: Option<&str>,
    ) -> Result<(), DbError> {
        self.db.conn().execute(
            "INSERT INTO kle_history_selection (entry_hash, chosen_bank_key) VALUES (?1, ?2)",
            params![entry_hash, chosen_bank_key],
        )?;
        Ok(())
    }

    pub fn log_rejection(
        &self,
        entry_hash: &str,
        rejected_bank_key: Option<&str>,
    ) -> Result<(), DbError> {
        self.db.conn().execute(
            "INSERT INTO kle_history_rejection (entry_hash, rejected_bank_key) VALUES (?1, ?2)",
            params![entry_hash, rejected_bank_key],
        )?;
        Ok(())
    }

    pub fn log_edit(
        &self,
        entry_hash: &str,
        before: Option<&str>,
        after: Option<&str>,
    ) -> Result<(), DbError> {
        self.db.conn().execute(
            "INSERT INTO kle_history_edit (entry_hash, before, after) VALUES (?1, ?2, ?3)",
            params![entry_hash, before, after],
        )?;
        Ok(())
    }

    pub fn log_approval(
        &self,
        entry_hash: &str,
        admin_user_id: Option<&str>,
    ) -> Result<(), DbError> {
        self.db.conn().execute(
            "INSERT INTO kle_history_approval (entry_hash, admin_user_id) VALUES (?1, ?2)",
            params![entry_hash, admin_user_id],
        )?;
        Ok(())
    }

    pub fn log_confidence(&self, entry_hash: &str, breakdown_json: &str) -> Result<(), DbError> {
        self.db.conn().execute(
            "INSERT INTO kle_history_confidence (entry_hash, breakdown_json) VALUES (?1, ?2)",
            params![entry_hash, breakdown_json],
        )?;
        Ok(())
    }

    /// The rejection ratio across every logged decision:
    /// `rejections / (rejections + selections)`. `None` when nothing has
    /// been logged yet, rather than a misleading `0.0`.
    pub fn rejection_ratio(&self) -> Result<Option<f64>, DbError> {
        let selections: i64 =
            self.db
                .conn()
                .query_row("SELECT count(*) FROM kle_history_selection", [], |row| {
                    row.get(0)
                })?;
        let rejections: i64 =
            self.db
                .conn()
                .query_row("SELECT count(*) FROM kle_history_rejection", [], |row| {
                    row.get(0)
                })?;

        let total = selections + rejections;
        if total == 0 {
            return Ok(None);
        }
        Ok(Some(rejections as f64 / total as f64))
    }

    pub fn timeline_for_entry(&self, entry_hash: &str) -> Result<EntryTimeline, DbError> {
        Ok(EntryTimeline {
            searches: self.query_many(
                "SELECT id, entry_hash, timestamp, query FROM kle_history_search WHERE entry_hash = ?1 ORDER BY timestamp ASC",
                entry_hash,
                |row| {
                    Ok(SearchEvent {
                        id: row.get(0)?,
                        entry_hash: row.get(1)?,
                        timestamp: row.get(2)?,
                        query: row.get(3)?,
                    })
                },
            )?,
            selections: self.query_many(
                "SELECT id, entry_hash, timestamp, chosen_bank_key FROM kle_history_selection WHERE entry_hash = ?1 ORDER BY timestamp ASC",
                entry_hash,
                |row| {
                    Ok(SelectionEvent {
                        id: row.get(0)?,
                        entry_hash: row.get(1)?,
                        timestamp: row.get(2)?,
                        chosen_bank_key: row.get(3)?,
                    })
                },
            )?,
            rejections: self.query_many(
                "SELECT id, entry_hash, timestamp, rejected_bank_key FROM kle_history_rejection WHERE entry_hash = ?1 ORDER BY timestamp ASC",
                entry_hash,
                |row| {
                    Ok(RejectionEvent {
                        id: row.get(0)?,
                        entry_hash: row.get(1)?,
                        timestamp: row.get(2)?,
                        rejected_bank_key: row.get(3)?,
                    })
                },
            )?,
            edits: self.query_many(
                "SELECT id, entry_hash, timestamp, before, after FROM kle_history_edit WHERE entry_hash = ?1 ORDER BY timestamp ASC",
                entry_hash,
                |row| {
                    Ok(EditEvent {
                        id: row.get(0)?,
                        entry_hash: row.get(1)?,
                        timestamp: row.get(2)?,
                        before: row.get(3)?,
                        after: row.get(4)?,
                    })
                },
            )?,
            approvals: self.query_many(
                "SELECT id, entry_hash, timestamp, admin_user_id FROM kle_history_approval WHERE entry_hash = ?1 ORDER BY timestamp ASC",
                entry_hash,
                |row| {
                    Ok(ApprovalEvent {
                        id: row.get(0)?,
                        entry_hash: row.get(1)?,
                        timestamp: row.get(2)?,
                        admin_user_id: row.get(3)?,
                    })
                },
            )?,
            confidence_snapshots: self.query_many(
                "SELECT id, entry_hash, timestamp, breakdown_json FROM kle_history_confidence WHERE entry_hash = ?1 ORDER BY timestamp ASC",
                entry_hash,
                |row| {
                    Ok(ConfidenceEvent {
                        id: row.get(0)?,
                        entry_hash: row.get(1)?,
                        timestamp: row.get(2)?,
                        breakdown_json: row.get(3)?,
                    })
                },
            )?,
        })
    }

    fn query_many<T>(
        &self,
        sql: &str,
        entry_hash: &str,
        mapper: impl FnMut(&rusqlite::Row) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, DbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([entry_hash], mapper)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
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
    fn logs_every_event_kind_and_retrieves_them_in_one_timeline() {
        let db = test_db();
        let repo = KleHistoryRepository::new(&db);
        let hash = "entry-1";

        repo.log_search(hash, "fire extinguisher missing").unwrap();
        repo.log_selection(hash, Some("fire_extinguisher")).unwrap();
        repo.log_rejection(hash, Some("sprinkler")).unwrap();
        repo.log_edit(hash, Some("before text"), Some("after text"))
            .unwrap();
        repo.log_approval(hash, Some("admin-user-1")).unwrap();
        repo.log_confidence(hash, "{\"percent\":72.5}").unwrap();

        let timeline = repo.timeline_for_entry(hash).unwrap();

        assert_eq!(timeline.searches.len(), 1);
        assert_eq!(timeline.selections.len(), 1);
        assert_eq!(timeline.rejections.len(), 1);
        assert_eq!(timeline.edits.len(), 1);
        assert_eq!(timeline.approvals.len(), 1);
        assert_eq!(timeline.confidence_snapshots.len(), 1);
        assert_eq!(
            timeline.selections[0].chosen_bank_key.as_deref(),
            Some("fire_extinguisher")
        );
    }

    #[test]
    fn timeline_for_an_unknown_entry_hash_is_all_empty() {
        let db = test_db();
        let repo = KleHistoryRepository::new(&db);
        let timeline = repo.timeline_for_entry("never-logged").unwrap();
        assert_eq!(timeline, EntryTimeline::default());
    }

    #[test]
    fn rejection_ratio_is_none_when_nothing_logged() {
        let db = test_db();
        let repo = KleHistoryRepository::new(&db);
        assert_eq!(repo.rejection_ratio().unwrap(), None);
    }

    #[test]
    fn rejection_ratio_reflects_the_mix_of_selections_and_rejections() {
        let db = test_db();
        let repo = KleHistoryRepository::new(&db);
        repo.log_selection("a", Some("bank")).unwrap();
        repo.log_selection("b", Some("bank")).unwrap();
        repo.log_selection("c", Some("bank")).unwrap();
        repo.log_rejection("d", Some("bank")).unwrap();

        let ratio = repo.rejection_ratio().unwrap().unwrap();
        assert!((ratio - 0.25).abs() < 1e-9);
    }
}
