use rusqlite::{params, Connection};

use crate::error::DbError;

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

/// Ordered, append-only list of schema migrations. Never edit a migration
/// once it has shipped — add a new one instead, even to fix a mistake in an
/// earlier one, so `schema_migrations` stays a true history of what ran
/// against real project databases in the field.
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/0001_init.sql"),
}];

/// Applies every migration not yet recorded in `schema_migrations`, each in
/// its own transaction so a failure partway through a migration can't leave
/// the schema half-updated.
pub(crate) fn apply_all(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );",
    )?;

    for migration in MIGRATIONS {
        let already_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [migration.version],
            |row| row.get(0),
        )?;
        if already_applied {
            continue;
        }

        log::info!(
            "applying database migration {} ({})",
            migration.version,
            migration.name
        );

        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(migration.sql)
            .map_err(|source| DbError::Migration {
                version: migration.version,
                name: migration.name,
                source,
            })?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![migration.version, migration.name],
        )?;
        tx.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_all_migrations_and_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply_all(&conn).unwrap();
        // Running twice must not error or re-apply anything.
        apply_all(&conn).unwrap();

        let applied_count: i64 = conn
            .query_row("SELECT count(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(applied_count, MIGRATIONS.len() as i64);
    }

    #[test]
    fn creates_every_expected_table() {
        let conn = Connection::open_in_memory().unwrap();
        apply_all(&conn).unwrap();

        let expected_tables = [
            "users",
            "projects",
            "sites",
            "audits",
            "observations",
            "evidence",
            "stage2_assessments",
            "stage3_matches",
            "observation_bank",
            "recommendation_bank",
            "legal_bank",
            "keyword_bank",
            "synonym_bank",
            "official_meta",
            "kle_pending_observations",
            "kle_pending_keywords",
            "kle_pending_synonyms",
            "kle_merge_requests",
            "kle_history_search",
            "kle_history_selection",
            "kle_history_rejection",
            "kle_history_edit",
            "kle_history_approval",
            "kle_history_confidence",
            "admin_settings",
            "audit_log",
        ];

        for table in expected_tables {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists, "expected table `{table}` to exist after migration");
        }
    }
}
