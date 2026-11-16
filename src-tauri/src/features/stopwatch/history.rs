//! Append-only stopwatch history persisted to `stopwatch-history.json`.
//!
//! Each entry is one completed segment of work or break with the label that was
//! active at the time, so a future history screen can group study time and
//! breaks by task or custom label without querying `CalDAV`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

use super::state::StopwatchLabelSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(
    export,
    export_to = "../../src/lib/bindings/StopwatchHistoryKind.generated.ts"
)]
pub enum StopwatchHistoryKind {
    Work,
    Break,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(
    export,
    export_to = "../../src/lib/bindings/StopwatchHistoryEntry.generated.ts"
)]
pub struct StopwatchHistoryEntry {
    pub kind: StopwatchHistoryKind,
    pub label: String,
    pub label_source: StopwatchLabelSource,
    #[ts(type = "number")]
    pub started_at: u64,
    #[ts(type = "number")]
    pub ended_at: u64,
    #[ts(type = "number")]
    pub duration_ms: u64,
}

#[must_use]
pub fn history_path(dir: &Path) -> PathBuf {
    dir.join("stopwatch-history.json")
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
    let raw = serde_json::to_string_pretty(&all)
        .map_err(|e| AppError::Store(format!("Failed to encode {}: {e}", path.display())))?;
    std::fs::write(path, raw)
        .map_err(|e| AppError::Store(format!("Failed to write {}: {e}", path.display())))
}

fn read(path: &Path) -> Result<Vec<StopwatchHistoryEntry>, AppError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(AppError::Store(format!(
                "Failed to read {}: {error}",
                path.display()
            )));
        }
    };

    serde_json::from_str(&raw).map_or_else(
        |error| {
            eprintln!(
                "Failed to parse {}: {error}; starting a fresh history",
                path.display()
            );
            mark_bad(path);
            Ok(Vec::new())
        },
        Ok,
    )
}

fn mark_bad(path: &Path) {
    let bad = path.with_extension("json.bad");
    let _ = std::fs::remove_file(&bad);
    if let Err(error) = std::fs::rename(path, &bad) {
        eprintln!(
            "Failed to rename {} to {}: {error}",
            path.display(),
            bad.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: StopwatchHistoryKind, label: &str, duration_ms: u64) -> StopwatchHistoryEntry {
        StopwatchHistoryEntry {
            kind,
            label: label.to_string(),
            label_source: StopwatchLabelSource::Custom,
            started_at: 1_000,
            ended_at: 1_000_u64.saturating_add(duration_ms),
            duration_ms,
        }
    }

    #[test]
    fn appends_entries_across_calls() {
        let dir = tempfile::tempdir().unwrap();
        let path = history_path(dir.path());

        append(&path, &[entry(StopwatchHistoryKind::Work, "a", 10)]).unwrap();
        append(&path, &[entry(StopwatchHistoryKind::Break, "a", 5)]).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let parsed: Vec<StopwatchHistoryEntry> = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].kind, StopwatchHistoryKind::Work);
        assert_eq!(parsed[1].kind, StopwatchHistoryKind::Break);
        assert_eq!(parsed[1].label, "a");
    }

    #[test]
    fn an_empty_append_does_not_create_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = history_path(dir.path());

        append(&path, &[]).unwrap();

        assert!(!path.exists());
    }
}
