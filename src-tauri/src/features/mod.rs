//! Behaviour, one bounded context per feature.
//!
//! Importing this root starts nothing: `bootstrap` calls each feature's runtime
//! entry point explicitly. Features never import `bootstrap` or `ipc`.

pub mod calendar;
pub mod reminders;
pub mod stopwatch;
