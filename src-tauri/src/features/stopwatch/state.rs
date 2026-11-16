//! Stopwatch state machine. No Tauri dependency.
//!
//! The transitions mirror the reference `StopwatchTK` clock: work counts up,
//! breaks count down as a fraction of the work just done, a finished break
//! waits in `Stopped` until the next work session starts fresh, and an active
//! break cannot be paused.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

use super::history::{self, StopwatchHistoryEntry, StopwatchHistoryKind};

/// Work in a session must reach this before a break can start.
pub const MIN_WORK_FOR_BREAK_MS: u64 = 5 * 60 * 1_000;
/// A session at or above this length earns the longer break divisor.
pub const LONG_BREAK_THRESHOLD_MS: u64 = 50 * 60 * 1_000;

const MILLIS_PER_MINUTE: u64 = 60_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(
    export,
    export_to = "../../src/lib/bindings/StopwatchPhase.generated.ts"
)]
pub enum StopwatchPhase {
    #[default]
    Paused,
    Running,
    Breaking,
    Stopped,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(
    export,
    export_to = "../../src/lib/bindings/StopwatchLabelSource.generated.ts"
)]
pub enum StopwatchLabelSource {
    #[default]
    Unset,
    Task,
    Custom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(
    export,
    export_to = "../../src/lib/bindings/StopwatchState.generated.ts"
)]
pub struct StopwatchState {
    pub phase: StopwatchPhase,
    /// Work accumulated in the current session, excluding the live run below.
    #[ts(type = "number")]
    pub work_elapsed: u64,
    /// Start of the live work run, when work is actually ticking.
    #[ts(type = "number | null")]
    pub work_running_since: Option<u64>,
    /// Total length of the current break countdown.
    #[ts(type = "number")]
    pub break_duration: u64,
    /// Start of the current break countdown.
    #[ts(type = "number | null")]
    pub break_started_at: Option<u64>,
    /// Timed pause (`P`); work stops ticking until this instant.
    #[ts(type = "number | null")]
    pub paused_until: Option<u64>,
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

    pub fn set_label(&mut self, label: Option<String>, source: StopwatchLabelSource) {
        let trimmed = label.and_then(|text| {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
        self.label_source = if trimmed.is_some() {
            source
        } else {
            StopwatchLabelSource::Unset
        };
        self.label = trimmed;
    }
}

pub struct StopwatchManager {
    state: Mutex<StopwatchState>,
    history_path: PathBuf,
}

impl StopwatchManager {
    #[must_use]
    pub fn load(data_dir: &Path) -> Self {
        Self {
            state: Mutex::new(StopwatchState::default()),
            history_path: history::history_path(data_dir),
        }
    }

    fn alter(
        &self,
        change: impl FnOnce(&mut StopwatchState) -> Result<Vec<StopwatchHistoryEntry>, AppError>,
    ) -> Result<StopwatchState, AppError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AppError::Internal("Stopwatch state is unavailable".to_string()))?;
        let entries = change(&mut state)?;
        let snapshot = state.clone();
        drop(state);
        if let Err(error) = history::append(&self.history_path, &entries) {
            eprintln!("Failed to record stopwatch history: {error}");
        }
        Ok(snapshot)
    }

    /// Read the current state, clearing an expired timed pause.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn snapshot(&self) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter(|state| {
            state.expire_pause(now);
            Ok(Vec::new())
        })
    }

    /// Start, pause or resume work.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails,
    /// or [`AppError::InvalidInput`] while a break is active.
    pub fn toggle_work(&self) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter(|state| state.toggle_work(now))
    }

    /// Start a break, or cancel an active break.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails,
    /// or [`AppError::InvalidInput`] when the break is not allowed.
    pub fn toggle_break(&self, short_divisor: u32) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter(|state| state.toggle_break(now, short_divisor))
    }

    /// Finish the current break countdown.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn complete_break(&self) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter(|state| Ok(state.complete_break(now)))
    }

    /// Pause or resume the timed pause.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn toggle_pause(&self, minutes: u64) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter(|state| Ok(state.toggle_pause(now, minutes)))
    }

    /// Extend or shorten an active timed pause.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn adjust_pause(&self, delta_minutes: i64) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter(|state| {
            state.adjust_pause(now, delta_minutes);
            Ok(Vec::new())
        })
    }

    /// Clear the clock and record the live segment.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock or system clock fails.
    pub fn reset(&self) -> Result<StopwatchState, AppError> {
        let now = current_epoch_millis()?;
        self.alter(|state| Ok(state.reset(now)))
    }

    /// Set or clear the mini tracker label.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the state lock fails.
    pub fn set_label(
        &self,
        label: Option<String>,
        source: StopwatchLabelSource,
    ) -> Result<StopwatchState, AppError> {
        self.alter(|state| {
            state.set_label(label, source);
            Ok(Vec::new())
        })
    }
}

pub fn current_epoch_millis() -> Result<u64, AppError> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("System clock error: {e}")))?
        .as_millis();
    u64::try_from(millis).map_err(|_| AppError::Internal("Timestamp too large".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: u64 = 60_000;

    fn minutes(n: u64) -> u64 {
        n.saturating_mul(MINUTE)
    }

    fn state_running_since(started: u64) -> StopwatchState {
        StopwatchState {
            phase: StopwatchPhase::Running,
            work_running_since: Some(started),
            ..StopwatchState::default()
        }
    }

    #[test]
    fn starts_paused_and_pristine() {
        let state = StopwatchState::default();

        assert_eq!(state.phase, StopwatchPhase::Paused);
        assert_eq!(state.work_ms(1_000), 0);
        assert_eq!(state.label_source, StopwatchLabelSource::Unset);
    }

    #[test]
    fn work_accumulates_while_running_and_freezes_when_paused() {
        let mut state = state_running_since(1_000);
        assert_eq!(
            state.work_ms(1_000_u64.saturating_add(minutes(4))),
            minutes(4)
        );

        let entries = state
            .toggle_work(1_000_u64.saturating_add(minutes(4)))
            .unwrap();
        assert_eq!(state.phase, StopwatchPhase::Paused);
        assert_eq!(state.work_ms(9_999_999), minutes(4));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, StopwatchHistoryKind::Work);
    }

    #[test]
    fn resuming_work_keeps_the_accumulated_total() {
        let mut state = state_running_since(0);
        state.toggle_work(minutes(2)).unwrap();
        state.toggle_work(minutes(2)).unwrap();

        assert_eq!(state.work_ms(minutes(3)), minutes(3));
    }

    #[test]
    fn a_break_needs_five_minutes_of_work() {
        let mut state = state_running_since(0);

        let before = state.toggle_break(minutes(4), 5);
        assert!(before.is_err());

        let allowed = state.toggle_break(MIN_WORK_FOR_BREAK_MS, 5);
        assert!(allowed.is_ok());
        assert_eq!(state.phase, StopwatchPhase::Breaking);
    }

    #[test]
    fn a_short_break_is_one_fifth_of_the_work() {
        let mut state = state_running_since(0);

        state.toggle_break(minutes(30), 5).unwrap();

        assert_eq!(state.break_duration, minutes(6));
        assert_eq!(state.break_remaining_ms(minutes(30)), minutes(6));
        assert_eq!(state.break_remaining_ms(minutes(33)), minutes(3));
    }

    #[test]
    fn a_long_break_is_one_fourth_of_the_work() {
        let mut state = state_running_since(0);

        state.toggle_break(minutes(60), 5).unwrap();

        assert_eq!(state.break_duration, minutes(15));
    }

    #[test]
    fn a_finished_break_waits_at_zero_then_starts_a_fresh_session() {
        let mut state = state_running_since(0);
        state.toggle_break(minutes(30), 5).unwrap();
        let break_end = minutes(30).saturating_add(minutes(6));

        let entries = state.complete_break(break_end);
        assert_eq!(state.phase, StopwatchPhase::Stopped);
        assert_eq!(state.work_ms(break_end), 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, StopwatchHistoryKind::Break);
        assert_eq!(entries[0].duration_ms, minutes(6));

        state.toggle_work(break_end).unwrap();
        assert_eq!(state.phase, StopwatchPhase::Running);
        assert_eq!(state.work_ms(break_end), 0);
    }

    #[test]
    fn a_break_cannot_be_paused_and_stops_cannot_break() {
        let mut state = state_running_since(0);
        state.toggle_break(minutes(10), 5).unwrap();
        assert!(state.toggle_work(minutes(10)).is_err());

        let mut stopped = StopwatchState {
            phase: StopwatchPhase::Stopped,
            ..StopwatchState::default()
        };
        assert!(stopped.toggle_break(0, 5).is_err());
    }

    #[test]
    fn cancelling_a_break_resumes_work_and_records_the_break_taken() {
        let mut state = state_running_since(0);
        state.toggle_break(minutes(30), 5).unwrap();

        let entries = state.toggle_break(minutes(31), 5).unwrap();

        assert_eq!(state.phase, StopwatchPhase::Running);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, StopwatchHistoryKind::Break);
        assert_eq!(entries[0].duration_ms, MINUTE);
    }

    #[test]
    fn a_timed_pause_freezes_work_and_resumes_when_it_expires() {
        let mut state = state_running_since(0);

        let entries = state.toggle_pause(minutes(2), 10);
        assert_eq!(state.paused_until, Some(minutes(12)));
        assert_eq!(state.work_ms(minutes(5)), minutes(2));
        assert_eq!(entries.len(), 1);

        assert!(state.expire_pause(minutes(12)));
        assert_eq!(state.work_ms(minutes(13)), minutes(3));
    }

    #[test]
    fn toggling_work_clears_an_active_timed_pause() {
        let mut state = state_running_since(0);
        state.toggle_pause(minutes(2), 10);

        state.toggle_work(minutes(3)).unwrap();

        assert_eq!(state.phase, StopwatchPhase::Running);
        assert_eq!(state.paused_until, None);
    }

    #[test]
    fn reset_records_the_live_segment_and_keeps_the_label() {
        let mut state = state_running_since(1_000);
        state.set_label(Some("  maths  ".to_string()), StopwatchLabelSource::Custom);

        let entries = state.reset(1_000_u64.saturating_add(minutes(2)));

        assert_eq!(state.phase, StopwatchPhase::Paused);
        assert_eq!(state.work_ms(1_000_u64.saturating_add(minutes(2))), 0);
        assert_eq!(state.label.as_deref(), Some("maths"));
        assert_eq!(state.label_source, StopwatchLabelSource::Custom);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label, "maths");
    }

    #[test]
    fn setting_an_empty_label_clears_its_source() {
        let mut state = StopwatchState::default();
        state.set_label(Some("deep work".to_string()), StopwatchLabelSource::Task);
        state.set_label(Some("   ".to_string()), StopwatchLabelSource::Custom);

        assert_eq!(state.label, None);
        assert_eq!(state.label_source, StopwatchLabelSource::Unset);
    }
}
