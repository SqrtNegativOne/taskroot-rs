// Tauri's IPC contract injects `AppHandle` by value and owns the deserialized
// command arguments, so clippy's borrow suggestion cannot be applied to a
// command wrapper. Keep the exception at the IPC boundary instead of on every
// handler.
#![allow(clippy::needless_pass_by_value)]

pub mod events;
pub mod session;
pub mod stopwatch;
pub mod sync;
pub mod tasks;
pub mod window;

#[cfg(test)]
mod tests;
