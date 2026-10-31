//! Task reminder delivery.
//!
//! [`policy`] decides which alarms are due; [`tauri`] polls the store and shows
//! the notifications. The Tauri half is wired only by `bootstrap`.

mod policy;
mod tauri;

pub use policy::{DueReminder, alarms_due};
pub use tauri::{ReminderState, start_reminder_engine};
