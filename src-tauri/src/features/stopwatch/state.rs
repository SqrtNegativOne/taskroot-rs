//! Stopwatch state machine. No Tauri dependency.
//!
//! The transitions mirror the reference `StopwatchTK` clock: work counts up,
//! breaks count down as a fraction of the work just done, a finished break
//! waits in `Stopped` until the next work session starts fresh, and an active
//! break cannot be paused.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;

use super::history::{self, StopwatchHistoryEntry, StopwatchHistoryKind};

/// Work in a session must reach this before a break can start.
pub const MIN_WORK_FOR_BREAK_MS: u64 = 5 * 60 * 1_000;
/// A session at or above this length earns the longer break divisor.
pub const LONG_BREAK_THRESHOLD_MS: u64 = 50 * 60 * 1_000;

const MILLIS_PER_MINUTE: u64 = 60_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StopwatchPhase {
    #[default]
    Paused,
    Running,
    Breaking,
    Stopped,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StopwatchLabelSource {
    #[default]
    Unset,
    Task,
    Custom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StopwatchState {
    pub phase: StopwatchPhase,
    /// Work accumulated in the current session, excluding the live run below.
    pub work_elapsed: u64,
    /// Start of the live work run, when work is actually ticking.
    pub work_running_since: Option<u64>,
    /// Total length of the current break countdown.
    pub break_duration: u64,
    /// Start of the current break countdown.
    pub break_started_at: Option<u64>,
    /// Timed pause (`P`); work stops ticking until this instant.
    pub paused_until: Option<u64>,
    /// Local focus pointer: the task the user is tracking, if any. The task's
    /// own status is independent and never stored in iCalendar.
    pub task_id: Option<String>,
    /// Free-text label shown in the mini tracker and recorded in history.
    pub label: Option<String>,
    pub label_source: StopwatchLabelSource,
}

impl StopwatchState {
    #[must_use]
    pub fn work_ms(&self, now: u64) -> u64 {
        if self.phase != StopwatchPhase::Running {
            return self.work_elapsed;
        }
        if let Some(since) = self.work_running_since {
            return self.work_elapsed.saturating_add(now.saturating_sub(since));
        }
        // A timed pause freezes work; once it expires work resumes at the
        // instant it was supposed to end, even before the next command arrives.
        if let Some(until) = self.paused_until.filter(|until| *until <= now) {
            return self.work_elapsed.saturating_add(now.saturating_sub(until));
        }
        self.work_elapsed
    }

    #[must_use]
    pub fn break_elapsed_ms(&self, now: u64) -> u64 {
        if self.phase != StopwatchPhase::Breaking {
            return 0;
        }
        self.break_started_at
            .map_or(0, |started| now.saturating_sub(started))
    }

    #[must_use]
    pub fn break_remaining_ms(&self, now: u64) -> u64 {
        self.break_duration
            .saturating_sub(self.break_elapsed_ms(now))
    }

    /// Clear an expired timed pause, resuming work at the point it expired.
    pub fn expire_pause(&mut self, now: u64) -> bool {
        let Some(until) = self.paused_until else {
            return false;
        };
        if until > now {
            return false;
        }
        self.paused_until = None;
        if self.phase == StopwatchPhase::Running && self.work_running_since.is_none() {
            self.work_running_since = Some(until);
        }
        true
    }

    fn take_work_segment(&mut self, now: u64) -> Option<StopwatchHistoryEntry> {
        let started = self.work_running_since.take()?;
        let duration = now.saturating_sub(started);
        self.work_elapsed = self.work_elapsed.saturating_add(duration);
        (duration > 0).then(|| StopwatchHistoryEntry {
            kind: StopwatchHistoryKind::Work,
            task_id: self.task_id.clone(),
            label: self.label.clone().unwrap_or_default(),
            label_source: self.label_source,
            started_at: started,
            ended_at: now,
            duration_ms: duration,
        })
    }

    fn take_break_segment(&mut self, now: u64) -> Option<StopwatchHistoryEntry> {
        let started = self.break_started_at.take()?;
        let duration = now.saturating_sub(started).min(self.break_duration);
        (duration > 0).then(|| StopwatchHistoryEntry {
            kind: StopwatchHistoryKind::Break,
            task_id: self.task_id.clone(),
            label: self.label.clone().unwrap_or_default(),
            label_source: self.label_source,
            started_at: started,
            ended_at: started.saturating_add(duration),
            duration_ms: duration,
        })
    }

    /// Start, pause or resume work.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] when a break is active.
    pub fn toggle_work(&mut self, now: u64) -> Result<Vec<StopwatchHistoryEntry>, AppError> {
        let pause_was_active = self.paused_until.is_some_and(|until| until > now);
        self.expire_pause(now);

        if pause_was_active || self.paused_until.is_some() {
            self.paused_until = None;
            if self.phase == StopwatchPhase::Running && self.work_running_since.is_none() {
                self.work_running_since = Some(now);
            }
            return Ok(Vec::new());
        }

        match self.phase {
            StopwatchPhase::Paused => {
                self.phase = StopwatchPhase::Running;
                self.work_running_since = Some(now);
                Ok(Vec::new())
            }
            StopwatchPhase::Stopped => {
                self.work_elapsed = 0;
                self.break_duration = 0;
                self.break_started_at = None;
                self.phase = StopwatchPhase::Running;
                self.work_running_since = Some(now);
                Ok(Vec::new())
            }
            StopwatchPhase::Running => {
                let entry = self.take_work_segment(now);
                self.phase = StopwatchPhase::Paused;
                Ok(entry.into_iter().collect())
            }
            StopwatchPhase::Breaking => Err(AppError::InvalidInput(
                "You cannot pause during a break.".to_string(),
            )),
        }
    }

    /// Start a break from the work done so far, or cancel an active break.
    ///
    /// `short_divisor` comes from `flowtime_break_divisor`; the long break uses
    /// one less (so a default divisor of five gives the reference 5/4 split).
    ///
    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] when the session is too short or has
    /// already finished a break.
    pub fn toggle_break(
        &mut self,
        now: u64,
        short_divisor: u32,
    ) -> Result<Vec<StopwatchHistoryEntry>, AppError> {
        self.expire_pause(now);

        if self.phase == StopwatchPhase::Breaking {
            let entry = self.take_break_segment(now);
            self.phase = StopwatchPhase::Running;
            self.break_duration = 0;
            self.paused_until = None;
            self.work_running_since = Some(now);
            return Ok(entry.into_iter().collect());
        }

        if self.phase == StopwatchPhase::Stopped {
            return Err(AppError::InvalidInput(
                "A break has already finished. Start working again first.".to_string(),
            ));
        }

        let total_work = self.work_ms(now);
        if total_work < MIN_WORK_FOR_BREAK_MS {
            return Err(AppError::InvalidInput(
                "Work for at least 5 minutes before taking a break.".to_string(),
            ));
        }

        let short = u64::from(short_divisor).max(1);
        let long = short.saturating_sub(1).max(1);
        let divisor = if total_work >= LONG_BREAK_THRESHOLD_MS {
            long
        } else {
            short
        };
        let duration = total_work.checked_div(divisor).unwrap_or(0).max(1);

        let entry = self.take_work_segment(now);
        self.phase = StopwatchPhase::Breaking;
        self.break_duration = duration;
        self.break_started_at = Some(now);
        self.paused_until = None;
        self.work_running_since = None;
        Ok(entry.into_iter().collect())
    }

    /// End the current break. A finished break waits in `Stopped`, and the next
    /// work run begins a fresh session.
    pub fn complete_break(&mut self, now: u64) -> Vec<StopwatchHistoryEntry> {
        if self.phase != StopwatchPhase::Breaking {
            return Vec::new();
        }
        let entry = self.take_break_segment(now);
        self.phase = StopwatchPhase::Stopped;
        self.work_elapsed = 0;
        self.work_running_since = None;
        self.break_duration = 0;
        entry.into_iter().collect()
    }

    /// Pause or resume the timed pause (`P`).
    pub fn toggle_pause(&mut self, now: u64, minutes: u64) -> Vec<StopwatchHistoryEntry> {
        let pause_was_active = self.paused_until.is_some_and(|until| until > now);
        self.expire_pause(now);

        if pause_was_active {
            self.paused_until = None;
            if self.phase == StopwatchPhase::Running && self.work_running_since.is_none() {
                self.work_running_since = Some(now);
            }
            return Vec::new();
        }

        if self.phase != StopwatchPhase::Running || self.work_running_since.is_none() {
            return Vec::new();
        }

        let entry = self.take_work_segment(now);
        let duration = minutes.max(1).saturating_mul(MILLIS_PER_MINUTE);
        self.paused_until = Some(now.saturating_add(duration));
        entry.into_iter().collect()
    }

    /// Extend (`delta > 0`) or shorten an active timed pause.
    pub fn adjust_pause(&mut self, now: u64, delta_minutes: i64) {
        self.expire_pause(now);
        let Some(until) = self.paused_until else {
            return;
        };

        let delta = i128::from(delta_minutes).saturating_mul(i128::from(MILLIS_PER_MINUTE));
        let target = i128::from(until).saturating_add(delta);
        if target <= i128::from(now) {
            self.paused_until = None;
            if self.phase == StopwatchPhase::Running && self.work_running_since.is_none() {
                self.work_running_since = Some(now);
            }
        } else {
            self.paused_until = u64::try_from(target).ok();
        }
    }

    /// Clear the clock, recording whatever segment was running.
    pub fn reset(&mut self, now: u64) -> Vec<StopwatchHistoryEntry> {
        let entry = match self.phase {
            StopwatchPhase::Running => self.take_work_segment(now),
            StopwatchPhase::Breaking => self.take_break_segment(now),
            StopwatchPhase::Paused | StopwatchPhase::Stopped => None,
        };

        self.phase = StopwatchPhase::Paused;
        self.work_elapsed = 0;
        self.work_running_since = None;
        self.break_duration = 0;
        self.break_started_at = None;
        self.paused_until = None;
        entry.into_iter().collect()
    }

    /// Flush any active work segment and pause the clock (used on exit or focus clear).
    pub fn flush_running_work(&mut self, now: u64) -> Vec<StopwatchHistoryEntry> {
        if self.phase != StopwatchPhase::Running {
            return Vec::new();
        }
        self.expire_pause(now);
        self.paused_until = None;
        let entry = self.take_work_segment(now);
        self.phase = StopwatchPhase::Paused;
        entry.into_iter().collect()
    }

    /// Set or clear the custom focus label. When switching focus mid-run, any
    /// live work segment is closed under the previous focus and resumed from `now`.
    pub fn set_label(
        &mut self,
        label: Option<String>,
        source: StopwatchLabelSource,
        now: u64,
    ) -> Vec<StopwatchHistoryEntry> {
        self.apply_focus(None, label, source, now)
    }

    /// Point the stopwatch at a task, or clear the local focus pointer. When
    /// switching focus mid-run, any live work segment is closed under the
    /// previous focus and resumed from `now`.
    pub fn set_task(
        &mut self,
        task_id: Option<String>,
        label: Option<String>,
        now: u64,
    ) -> Vec<StopwatchHistoryEntry> {
        self.apply_focus(task_id, label, StopwatchLabelSource::Task, now)
    }

    fn apply_focus(
        &mut self,
        task_id: Option<String>,
        label: Option<String>,
        source: StopwatchLabelSource,
        now: u64,
    ) -> Vec<StopwatchHistoryEntry> {
        let (next_task_id, next_label, next_source) = normalize_focus_parts(task_id, label, source);
        let focus_changing = self.task_id != next_task_id
            || self.label != next_label
            || self.label_source != next_source;

        let mut entries = Vec::new();
        if self.phase == StopwatchPhase::Running {
            self.expire_pause(now);
            if focus_changing && self.work_running_since.is_some() {
                if let Some(entry) = self.take_work_segment(now) {
                    entries.push(entry);
                }
                self.work_running_since = Some(now);
            }
        }

        self.task_id = next_task_id;
        self.label = next_label;
        self.label_source = next_source;
        entries
    }

    fn normalize_focus(&mut self) {
        let (task_id, label, source) = normalize_focus_parts(
            self.task_id.take(),
            self.label.take(),
            self.label_source,
        );
        self.task_id = task_id;
        self.label = label;
        self.label_source = source;
    }
}

fn normalize_focus_parts(
    task_id: Option<String>,
    label: Option<String>,
    source: StopwatchLabelSource,
) -> (Option<String>, Option<String>, StopwatchLabelSource) {
    let task_id = trim_to_some(task_id);
    let label = trim_to_some(label);
    let source = if task_id.is_some() {
        StopwatchLabelSource::Task
    } else if label.is_some() {
        let _ = source;
        StopwatchLabelSource::Custom
    } else {
        StopwatchLabelSource::Unset
    };
    (task_id, label, source)
}

fn trim_to_some(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

/// Result of a state-machine transition: the updated snapshot plus any
/// history segments closed during that step.
#[derive(Debug, Clone)]
pub struct StopwatchTransition {
    pub state: StopwatchState,
    pub entries: Vec<StopwatchHistoryEntry>,
}

pub struct StopwatchManager {
    state: Mutex<StopwatchState>,
    history_path: PathBuf,
    focus_path: PathBuf,
}

impl StopwatchManager {
    #[must_use]
    pub fn load(data_dir: &Path) -> Self {
        let focus_path = super::focus::focus_path(data_dir);
        let mut state = StopwatchState::default();
        if let Some(stored) = super::focus::load(&focus_path) {
            state.task_id = stored.task_id;
            state.label = stored.label;
            state.label_source = stored.label_source;
        }
        state.normalize_focus();
        Self {
            state: Mutex::new(state),
            history_path: history::history_path(data_dir),
            focus_path,
        }
    }

    fn alter_transition(
        &self,
        change: impl FnOnce(&mut StopwatchState) -> Result<Vec<StopwatchHistoryEntry>, AppError>,
    ) -> Result<StopwatchTransition, AppError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AppError::Internal("Stopwatch state is unavailable".to_string()))?;
        let focus_before = focus_key(&state);
        let entries = change(&mut state)?;
        state.normalize_focus();
        let focus_changed = focus_before != focus_key(&state);
        let snapshot = state.clone();
        if focus_changed {
            if let Err(error) = super::focus::save(&self.focus_path, &snapshot) {
                eprintln!("Failed to persist stopwatch focus: {error}");
            }
        }
        if let Err(error) = history::append(&self.history_path, &entries) {
            eprintln!("Failed to record stopwatch history: {error}");
        }
        drop(state);
        Ok(StopwatchTransition {
            state: snapshot,
            entries,
        })
    }

    /// Read the current state, clearing an expired timed pause.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn snapshot(&self) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| {
            state.expire_pause(now);
            Ok(Vec::new())
        })
        .map(|transition| transition.state)
    }

    /// Read every persisted stopwatch history entry.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Store`] when the history file cannot be read.
    pub fn history(&self) -> Result<Vec<StopwatchHistoryEntry>, AppError> {
        history::read(&self.history_path)
    }

    /// Start, pause or resume work.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails,
    /// or [`AppError::InvalidInput`] while a break is active.
    pub fn toggle_work(&self) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| state.toggle_work(now))
    }

    /// Start a break, or cancel an active break.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails,
    /// or [`AppError::InvalidInput`] when the break is not allowed.
    pub fn toggle_break(&self, short_divisor: u32) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| state.toggle_break(now, short_divisor))
    }

    /// Finish the current break countdown.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn complete_break(&self) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.complete_break(now)))
    }

    /// Pause or resume the timed pause.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn toggle_pause(&self, minutes: u64) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.toggle_pause(now, minutes)))
    }

    /// Extend or shorten an active timed pause.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn adjust_pause(&self, delta_minutes: i64) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| {
            state.adjust_pause(now, delta_minutes);
            Ok(Vec::new())
        })
        .map(|transition| transition.state)
    }

    /// Clear the clock and record the live segment.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn reset(&self) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.reset(now)))
    }

    /// Set or clear the mini tracker label.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn set_label(
        &self,
        label: Option<String>,
        source: StopwatchLabelSource,
    ) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.set_label(label, source, now)))
    }

    /// Point the stopwatch at a task, or clear the focus pointer.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn set_task(
        &self,
        task_id: Option<String>,
        label: Option<String>,
    ) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.set_task(task_id, label, now)))
    }

    /// Flush any running work segment on application exit and pause the clock.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn flush_for_exit(&self) -> Result<StopwatchTransition, AppError> {
        let now = current_epoch_millis()?;
        self.alter_transition(|state| Ok(state.flush_running_work(now)))
    }

    /// Clear the focus when it points at `task_id`, stopping a running clock.
    ///
    /// Returns the transition when the focus matched, `None` otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
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

pub fn current_epoch_millis() -> Result<u64, AppError> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("System clock error: {e}")))?
        .as_millis();
    u64::try_from(millis).map_err(|_| AppError::Internal("Timestamp too large".to_string()))
}

#[cfg(test)]
mod tests;
