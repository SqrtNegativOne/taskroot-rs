//! Stopwatch feature.
//!
//! [`state`] is the pure clock/state machine (usable headlessly), [`history`]
//! persists completed work/break segments, and [`tauri`] is the adapter that
//! resolves the managed state from an `AppHandle`.

#[allow(unreachable_pub)]
mod feed;
#[allow(unreachable_pub)]
mod focus;
#[allow(unreachable_pub)]
pub mod history;
#[allow(unreachable_pub)]
mod state;
#[allow(unreachable_pub)]
mod tauri;

pub use feed::start as start_focus_feed;
pub use state::{
    StopwatchLabelSource, StopwatchManager, StopwatchPhase, StopwatchState, StopwatchTransition,
    LONG_BREAK_THRESHOLD_MS, MIN_WORK_FOR_BREAK_MS,
};
pub(crate) use tauri::with_manager;
