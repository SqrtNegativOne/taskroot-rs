//! Thread-safe stopwatch manager with serialized disk I/O and cached history.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::AppError;

use super::super::focus;
use super::super::history::{self, StopwatchHistoryEntry};
use super::{current_epoch_millis, StopwatchLabelSource, StopwatchState};

/// Result of a state-machine transition: the updated snapshot plus any
/// history segments closed during that step.
#[derive(Debug, Clone)]
pub struct StopwatchTransition {
    pub state: StopwatchState,
    pub entries: Vec<StopwatchHistoryEntry>,
}

struct StopwatchPersistence {
    focus_path: PathBuf,
    history_path: PathBuf,
    history: Option<Vec<StopwatchHistoryEntry>>,
}

impl StopwatchPersistence {
    fn ensure_history_loaded(&mut self) -> Result<(), AppError> {
        if self.history.is_none() {
            self.history = Some(history::read(&self.history_path)?);
        }
        Ok(())
    }

    fn history_entries(&mut self) -> Result<&[StopwatchHistoryEntry], AppError> {
        self.ensure_history_loaded()?;
        self.history
            .as_deref()
            .ok_or_else(|| AppError::Internal("Stopwatch history cache unavailable".to_string()))
    }

    fn append_history(&mut self, entries: &[StopwatchHistoryEntry]) -> Result<(), AppError> {
        if entries.is_empty() {
            return Ok(());
        }
        self.ensure_history_loaded()?;
        let cached = self
            .history
            .as_mut()
            .ok_or_else(|| AppError::Internal("Stopwatch history cache unavailable".to_string()))?;
        let previous_len = cached.len();
        cached.extend_from_slice(entries);
        if let Err(error) = history::write(&self.history_path, cached) {
            cached.truncate(previous_len);
            return Err(error);
        }
        Ok(())
    }
}

/// Coordinates in-memory stopwatch state with strictly ordered persistence.
///
/// Lock order is strictly `io` -> `state`. Read-only state queries (`snapshot`,
/// `adjust_pause`) lock `state` alone and never block on disk I/O, while
/// transitions hold `io` across the state update and disk write so focus saves
/// and history appends always land in exact transition order.
pub struct StopwatchManager {
    state: Mutex<StopwatchState>,
    io: Mutex<StopwatchPersistence>,
}

impl StopwatchManager {
    #[must_use]
    pub fn load(data_dir: &Path) -> Self {
        let focus_path = focus::focus_path(data_dir);
        let mut state = StopwatchState::default();
        if let Some(stored) = focus::load(&focus_path) {
            state.task_id = stored.task_id;
            state.label = stored.label;
            state.label_source = stored.label_source;
        }
        state.normalize_focus();
        Self {
            state: Mutex::new(state),
            io: Mutex::new(StopwatchPersistence {
                focus_path,
                history_path: history::history_path(data_dir),
                history: None,
            }),
        }
    }

    fn alter_state_only(
        &self,
        change: impl FnOnce(&mut StopwatchState),
    ) -> Result<StopwatchState, AppError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AppError::Internal("Stopwatch state is unavailable".to_string()))?;
        change(&mut state);
        Ok(state.clone())
    }

    fn alter_transition(
        &self,
        change: impl FnOnce(&mut StopwatchState) -> Result<Vec<StopwatchHistoryEntry>, AppError>,
    ) -> Result<StopwatchTransition, AppError> {
        let mut io = self
            .io
            .lock()
            .map_err(|_| AppError::Internal("Stopwatch storage is unavailable".to_string()))?;
        let (snapshot, focus_changed, entries) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AppError::Internal("Stopwatch state is unavailable".to_string()))?;
            let focus_before = focus_key(&state);
            let entries = change(&mut state)?;
            state.normalize_focus();
            let focus_changed = focus_before != focus_key(&state);
            (state.clone(), focus_changed, entries)
        };
        if focus_changed {
            if let Err(error) = focus::save(&io.focus_path, &snapshot) {
                eprintln!("Failed to persist stopwatch focus: {error}");
            }
        }
        if let Err(error) = io.append_history(&entries) {
            eprintln!("Failed to record stopwatch history: {error}");
        }
        drop(io);
        Ok(StopwatchTransition {
            state: snapshot,
            entries,
        })
    }

    /// Read the current state, clearing an expired timed pause.
    /// # Errors
    /// Fails if the state lock or system clock is unavailable.
    pub fn snapshot(&self) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter_state_only(|state| {
            state.expire_pause(now);
        })
    }

    /// Read every persisted stopwatch history entry (served from cache after first load).
    /// # Errors
    /// Fails if the storage lock is poisoned or the history file cannot be read.
    pub fn history(&self) -> Result<Vec<StopwatchHistoryEntry>, AppError> {
        let mut io = self
            .io
            .lock()
            .map_err(|_| AppError::Internal("Stopwatch storage is unavailable".to_string()))?;
        Ok(io.history_entries()?.to_vec())
    }

    /// Start, pause or resume work.
    /// # Errors
    /// Fails on lock/clock error or [`AppError::InvalidInput`] during a break.
    pub fn toggle_work(&self) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| state.toggle_work(now))
    }

    /// Start a break, or cancel an active break.
    /// # Errors
    /// Fails on lock/clock error or [`AppError::InvalidInput`] when disallowed.
    pub fn toggle_break(&self, short_divisor: u32) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| state.toggle_break(now, short_divisor))
    }

    /// Finish the current break countdown.
    /// # Errors
    /// Fails if the lock or system clock is unavailable.
    pub fn complete_break(&self) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.complete_break(now)))
    }

    /// Pause or resume the timed pause.
    /// # Errors
    /// Fails if the lock or system clock is unavailable.
    pub fn toggle_pause(&self, minutes: u64) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.toggle_pause(now, minutes)))
    }

    /// Extend or shorten an active timed pause.
    /// # Errors
    /// Fails if the lock or system clock is unavailable.
    pub fn adjust_pause(&self, delta_minutes: i64) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter_state_only(|state| {
            state.adjust_pause(now, delta_minutes);
        })
    }

    /// Clear the clock and record the live segment.
    /// # Errors
    /// Fails if the lock or system clock is unavailable.
    pub fn reset(&self) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.reset(now)))
    }

    /// Set or clear the mini tracker label.
    /// # Errors
    /// Fails if the lock or system clock is unavailable.
    pub fn set_label(
        &self,
        label: Option<String>,
        source: StopwatchLabelSource,
    ) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.set_label(label, source, now)))
    }

    /// Point the stopwatch at a task, or clear the focus pointer.
    /// # Errors
    /// Fails if the lock or system clock is unavailable.
    pub fn set_task(
        &self,
        task_id: Option<String>,
        label: Option<String>,
    ) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.set_task(task_id, label, now)))
    }

    /// Flush any running work segment on application exit and pause the clock.
    /// # Errors
    /// Fails if the lock or system clock is unavailable.
    pub fn flush_for_exit(&self) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.flush_running_work(now)))
    }

    /// Clear the focus when it points at `task_id`, stopping a running clock.
    /// # Errors
    /// Fails if the lock or system clock is unavailable.
    pub fn clear_task_if(&self, task_id: &str) -> Result<Option<StopwatchTransition>, AppError> {
        let now = current_epoch_millis()?;
        let mut cleared = false;
        let transition = self.alter_transition(|state| {
            if state.task_id.as_deref() != Some(task_id) {
                return Ok(Vec::new());
            }
            let mut entries = state.flush_running_work(now);
            entries.extend(state.set_task(None, None, now));
            cleared = true;
            Ok(entries)
        })?;
        Ok(cleared.then_some(transition))
    }
}

type FocusKey = (Option<String>, Option<String>, StopwatchLabelSource);

fn focus_key(state: &StopwatchState) -> FocusKey {
    (
        state.task_id.clone(),
        state.label.clone(),
        state.label_source,
    )
}
