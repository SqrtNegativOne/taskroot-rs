// Tauri's IPC contract injects `AppHandle` by value and owns the deserialized
// command arguments, so clippy's borrow suggestion cannot be applied to a
// command wrapper. Keep the exception at the IPC boundary instead of on every
// handler.
#![allow(clippy::needless_pass_by_value)]

pub mod calendars;
pub mod events;
pub mod plan;
pub mod refresh;
pub mod schema;
pub mod session;
pub mod settings;
pub mod stopwatch;
pub mod tasks;
pub mod window;

#[cfg(test)]
mod tests;
