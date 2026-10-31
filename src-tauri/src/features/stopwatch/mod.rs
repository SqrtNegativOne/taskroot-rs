//! Stopwatch feature.
//!
//! [`state`] is the pure clock/state machine (usable headlessly); [`tauri`] is
//! the adapter that resolves the managed state from an `AppHandle`.

#[allow(unreachable_pub)]
mod state;
#[allow(unreachable_pub)]
mod tauri;

pub(crate) use state::current_epoch_millis;
pub use state::{StopwatchManager, StopwatchState};
pub(crate) use tauri::with_locked_state;
