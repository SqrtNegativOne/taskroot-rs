//! `iCalendar` read/write for `CalDAV` resources.
//!
//! Three layers, never blurred:
//!
//! 1. **Resource body** — the exact bytes fetched from the server. Writes merge
//!    into these bytes; nothing is ever re-serialized from a DTO (**I1**, **I3**).
//! 2. **Components** — one resource is a *set* of `VEVENT`s or `VTODO`s sharing a
//!    UID (**I2**).
//! 3. **DTOs** — [`AppEvent`](crate::domain::AppEvent) /
//!    [`AppTask`](crate::domain::AppTask), see [`read`] for the canonical value
//!    formats.
//!
//! ```
//! use tauri_app_lib::ics::{read_event_resource, write_event_resource};
//!
//! let raw = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:one\r\nDTSTART;VALUE=DATE:20261017\r\nDTEND;VALUE=DATE:20261018\r\nSUMMARY:Standup\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
//! let events = read_event_resource(raw, "/cal/one.ics", "/cal/").unwrap();
//! let mut event = events[0].clone();
//! event.title = "Renamed".to_string();
//! let written = write_event_resource(raw, &event).unwrap();
//! assert!(written.contains("SUMMARY:Renamed"));
//! assert_eq!(written.replace("SUMMARY:Renamed", "SUMMARY:Standup"), raw);
//! ```
//!
//! Everything Taskroot does not own survives byte-identically: `VTIMEZONE`,
//! `ATTENDEE`, `X-MOZ-GENERATION`, `X-APPLE-SORT-ORDER`, `SEQUENCE`, `LOCATION`,
//! unknown `X-*`. A `VALARM` is rewritten only when the task's reminder list actually
//! changed, so a reminder this crate cannot model is preserved by every other edit.

mod lines;
pub mod read;
pub mod write;

#[cfg(test)]
mod tests;

pub use read::{read_event_resource, read_task_resource};
pub use write::{
    create_event_resource, create_task_resource, delete_event_component, delete_task_component,
    write_event_resource, write_task_resource,
};
