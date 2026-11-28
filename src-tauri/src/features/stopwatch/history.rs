//! Append-only stopwatch history persisted to `stopwatch-history.json`.
//!
//! Each entry is one completed segment of work or break with the label that was
//! active at the time, so a future history screen can group study time and
//! breaks by task or custom label without querying `CalDAV`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;
use crate::storage;

use super::state::StopwatchLabelSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StopwatchHistoryKind {
    Work,
    Break,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StopwatchHistoryEntry {
    pub kind: StopwatchHistoryKind,
    #[serde(default)]
    #[specta(optional)]
    pub task_id: Option<String>,
    pub label: String,
    pub label_source: StopwatchLabelSource,
    pub started_at: u64,
    pub ended_at: u64,
    pub duration_ms: u64,
}

#[must_use]
pub fn history_path(dir: &Path) -> PathBuf {
    dir.join(storage::STOPWATCH_HISTORY_FILE)
}

/// Atomically replace the history file with `entries`.
///
/// # Errors
///
/// Returns [`AppError::Store`] when the file cannot be serialized or replaced.
pub(super) fn write(path: &Path, entries: &[StopwatchHistoryEntry]) -> Result<(), AppError> {
    storage::write_json_pretty_atomic(path, entries)
}

/// Append `entries` to the history file, creating it when absent.
///
/// A malformed history file is moved aside to `stopwatch-history.json.bad`
/// instead of being silently truncated.
///
/// # Errors
///
/// Returns [`AppError::Store`] when the file cannot be read or written.
pub fn append(path: &Path, entries: &[StopwatchHistoryEntry]) -> Result<(), AppError> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut all = read(path)?;
    all.extend_from_slice(entries);
    write(path, &all)
}

/// Read every persisted history entry from `path`.
///
/// Returns an empty list if the file does not exist yet. A malformed file is
/// moved aside to `stopwatch-history.json.bad` and treated as empty.
///
/// # Errors
///
/// Returns [`AppError::Store`] when an existing file cannot be read.
pub fn read(path: &Path) -> Result<Vec<StopwatchHistoryEntry>, AppError> {
    Ok(storage::read_json_or_quarantine(path)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        kind: StopwatchHistoryKind,
        task_id: Option<&str>,
        label: &str,
        duration_ms: u64,
    ) -> StopwatchHistoryEntry {
        StopwatchHistoryEntry {
            kind,
            task_id: task_id.map(ToString::to_string),
            label: label.to_string(),
            label_source: if task_id.is_some() {
                StopwatchLabelSource::Task
            } else {
                StopwatchLabelSource::Custom
            },
            started_at: 1_000,
            ended_at: 1_000_u64.saturating_add(duration_ms),
            duration_ms,
        }
    }

    #[test]
    fn appends_entries_across_calls() {
        let dir = tempfile::tempdir().unwrap();
        let path = history_path(dir.path());

        append(&path, &[entry(StopwatchHistoryKind::Work, Some("t1"), "a", 10)]).unwrap();
        append(&path, &[entry(StopwatchHistoryKind::Break, None, "a", 5)]).unwrap();

        let parsed = read(&path).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].kind, StopwatchHistoryKind::Work);
        assert_eq!(parsed[0].task_id.as_deref(), Some("t1"));
        assert_eq!(parsed[1].kind, StopwatchHistoryKind::Break);
        assert_eq!(parsed[1].task_id, None);
        assert_eq!(parsed[1].label, "a");
    }

    #[test]
    fn legacy_entries_without_task_id_deserialize_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = history_path(dir.path());
        std::fs::write(
            &path,
            r#"[{"kind":"work","label":"deep work","labelSource":"custom","startedAt":10,"endedAt":20,"durationMs":10}]"#,
        )
        .unwrap();

        let parsed = read(&path).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].task_id, None);
        assert_eq!(parsed[0].label, "deep work");
    }

    #[test]
    fn an_empty_append_does_not_create_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = history_path(dir.path());

        append(&path, &[]).unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn malformed_history_is_quarantined_before_appending_new_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = history_path(dir.path());
        std::fs::write(&path, "not json").unwrap();

        append(&path, &[entry(StopwatchHistoryKind::Work, Some("t1"), "a", 10)]).unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.path().join("stopwatch-history.json.bad")).unwrap(),
            "not json"
        );
        let parsed = read(&path).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].task_id.as_deref(), Some("t1"));
    }
}
