//! `iCalendar` read/write for `CalDAV` resources, backed by `ical-rs`.
//!
//! - **I1 / I3** — Parsed into `ical-rs`'s [`ical::tree::cst::IcalCst`]. Writes
//!   replace only owned properties; unknown properties, parameters, `VTIMEZONE`,
//!   unmodelled `VALARM`s and sibling components survive intact.
//! - **I2** — One resource is a set of `VEVENT`s or `VTODO`s sharing a UID.
//!
//! ```
//! use taskroot_lib::features::calendar::{read_event_resource, write_event_resource};
//!
//! let raw = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:one\r\nDTSTART;VALUE=DATE:20261017\r\nDTEND;VALUE=DATE:20261018\r\nSUMMARY:Standup\r\nX-KEEP:yes\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
//! let mut event = read_event_resource(raw, "/cal/one.ics", "/cal/").unwrap().remove(0);
//! event.title = "Renamed".to_string();
//! let written = write_event_resource(raw, &event).unwrap();
//! assert!(written.contains("X-KEEP:yes"));
//! assert_eq!(read_event_resource(&written, "/cal/one.ics", "/cal/").unwrap()[0].title, "Renamed");
//! ```

mod cst;
mod dates;
pub mod read;
mod read_task;
pub mod write;
mod write_props;

#[cfg(test)]
mod tests;

pub use read::{read_event_resource, read_task_resource};
pub use write::{
    create_event_resource, create_task_resource, delete_event_component, delete_task_component,
    write_event_resource, write_task_resource,
};
