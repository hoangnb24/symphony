use std::path::Path;

use rusqlite::{params, Connection};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkError {
    #[error("harness database not found at {0}. Run: scripts/bin/harness-cli init")]
    MissingDatabase(String),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItem {
    pub id: String,
    pub status: String,
    pub lane: String,
    pub verify: String,
    pub runnable: String,
    pub reason: String,
}

pub fn list_work(db_path: &Path) -> Result<Vec<WorkItem>, WorkError> {
    if !db_path.exists() {
        return Err(WorkError::MissingDatabase(db_path.display().to_string()));
    }
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, status, risk_lane, verify_command
         FROM story
         ORDER BY id;",
    )?;
    let rows = statement.query_map(params![], |row| {
        let id = row.get::<_, String>(0)?;
        let status = row.get::<_, String>(1)?;
        let lane = row.get::<_, String>(2)?;
        let verify_command = row.get::<_, Option<String>>(3)?;
        Ok(classify(id, status, lane, verify_command))
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(WorkError::from)
}

fn classify(id: String, status: String, lane: String, verify_command: Option<String>) -> WorkItem {
    let has_verify = verify_command
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let verify = if has_verify { "configured" } else { "missing" }.to_owned();
    let (runnable, reason) = match status.as_str() {
        "planned" | "in_progress" if has_verify => ("yes", "ready"),
        "planned" | "in_progress" => ("warn", "proof command missing"),
        "implemented" => ("no", "already implemented"),
        "retired" => ("no", "retired"),
        "changed" => ("warn", "changed story needs human review"),
        _ => ("no", "unknown story status"),
    };

    WorkItem {
        id,
        status,
        lane,
        verify,
        runnable: runnable.to_owned(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_planned_story_with_verify_as_ready() {
        let item = classify(
            "US-1".to_owned(),
            "planned".to_owned(),
            "normal".to_owned(),
            Some("cargo test".to_owned()),
        );

        assert_eq!(item.verify, "configured");
        assert_eq!(item.runnable, "yes");
        assert_eq!(item.reason, "ready");
    }

    #[test]
    fn missing_verify_is_warning_not_status_change() {
        let item = classify(
            "US-2".to_owned(),
            "in_progress".to_owned(),
            "normal".to_owned(),
            None,
        );

        assert_eq!(item.status, "in_progress");
        assert_eq!(item.verify, "missing");
        assert_eq!(item.runnable, "warn");
        assert_eq!(item.reason, "proof command missing");
    }

    #[test]
    fn implemented_and_retired_are_not_runnable() {
        for status in ["implemented", "retired"] {
            let item = classify(
                "US-3".to_owned(),
                status.to_owned(),
                "normal".to_owned(),
                Some("true".to_owned()),
            );
            assert_eq!(item.runnable, "no");
        }
    }

    #[test]
    fn list_work_reads_story_rows_from_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("harness.db");
        let connection = Connection::open(&db_path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE story (
                    id TEXT PRIMARY KEY,
                    status TEXT NOT NULL,
                    risk_lane TEXT NOT NULL,
                    verify_command TEXT
                );
                INSERT INTO story (id, status, risk_lane, verify_command)
                VALUES
                    ('US-READY', 'planned', 'normal', 'cargo test'),
                    ('US-WARN', 'planned', 'normal', NULL),
                    ('US-DONE', 'implemented', 'normal', 'true');",
            )
            .unwrap();
        drop(connection);

        let items = list_work(&db_path).unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "US-DONE");
        assert_eq!(items[0].runnable, "no");
        assert_eq!(items[1].id, "US-READY");
        assert_eq!(items[1].runnable, "yes");
        assert_eq!(items[2].id, "US-WARN");
        assert_eq!(items[2].runnable, "warn");
    }
}
