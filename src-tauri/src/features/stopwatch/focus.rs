//! The local focus pointer persisted to `stopwatch-focus.json`.
//!
//! "Currently doing" is app state, not a VTODO status: starting a task must not
//! rewrite the calendar. This file keeps the pointer and its label across
//! restarts, and is the seed for the live focus feed.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::state::{StopwatchLabelSource, StopwatchState};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedFocus {
    pub(super) task_id: Option<String>,
    pub(super) label: Option<String>,
    #[serde(default)]
    pub(super) label_source: StopwatchLabelSource,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FocusOnDisk<'a> {
    task_id: Option<&'a str>,
    label: Option<&'a str>,
    label_source: StopwatchLabelSource,
}

#[must_use]
pub(super) fn focus_path(dir: &Path) -> PathBuf {
    dir.join("stopwatch-focus.json")
}

/// Read the stored focus pointer, ignoring a missing or malformed file.
pub(super) fn load(path: &Path) -> Option<PersistedFocus> {
    let raw = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&raw) {
        Ok(stored) => Some(stored),
        Err(error) => {
            eprintln!(
                "Failed to parse {}: {error}; starting without a focus task",
                path.display()
            );
            None
        }
    }
}

/// Write the focus pointer atomically so a reader never sees a partial file.
pub(super) fn save(path: &Path, state: &StopwatchState) -> Result<(), AppError> {
    let focus = FocusOnDisk {
        task_id: state.task_id.as_deref(),
        label: state.label.as_deref(),
        label_source: state.label_source,
    };
    let raw = serde_json::to_string_pretty(&focus)
        .map_err(|error| AppError::Store(format!("Failed to encode {}: {error}", path.display())))?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, raw)
        .map_err(|error| AppError::Store(format!("Failed to write {}: {error}", temp.display())))?;
    std::fs::rename(&temp, path)
        .map_err(|error| AppError::Store(format!("Failed to replace {}: {error}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(task_id: Option<&str>, label: Option<&str>, source: StopwatchLabelSource) -> StopwatchState {
        StopwatchState {
            task_id: task_id.map(str::to_string),
            label: label.map(str::to_string),
            label_source: source,
            ..StopwatchState::default()
        }
    }

    #[test]
    fn saves_and_loads_the_focus_pointer() {
        let dir = tempfile::tempdir().unwrap();
        let path = focus_path(dir.path());

        save(
            &path,
            &state(Some("task-1"), Some("Learn Python"), StopwatchLabelSource::Task),
        )
        .unwrap();
        let stored = load(&path).unwrap();

        assert_eq!(stored.task_id.as_deref(), Some("task-1"));
        assert_eq!(stored.label.as_deref(), Some("Learn Python"));
        assert_eq!(stored.label_source, StopwatchLabelSource::Task);
    }

    #[test]
    fn a_missing_file_loads_as_none() {
        let dir = tempfile::tempdir().unwrap();

        assert!(load(&focus_path(dir.path())).is_none());
    }

    #[test]
    fn a_malformed_file_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = focus_path(dir.path());
        std::fs::write(&path, "not json").unwrap();

        assert!(load(&path).is_none());
    }
}
