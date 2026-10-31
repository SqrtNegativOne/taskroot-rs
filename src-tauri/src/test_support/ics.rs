//! ICS resources shared by the `CalDAV` and store tests.
//!
//! The bodies come from `src/ics/fixtures/`, so a test resource is a real
//! Radicale-style file instead of an inline string. [`with_exception`] appends
//! the moved occurrence the store's recurrence tests need.

/// The weekly 11:00 Asia/Kolkata series (`series.ics`).
#[must_use]
pub(crate) const fn series() -> &'static str {
    include_str!("../ics/fixtures/series.ics")
}

/// The all-day holiday resource (`all-day-event.ics`).
#[must_use]
pub(crate) const fn all_day() -> &'static str {
    include_str!("../ics/fixtures/all-day-event.ics")
}

/// The 2026-11-02 occurrence of [`series`], moved from 11:00 to 14:00.
const MOVED_OCCURRENCE: &str = "BEGIN:VEVENT\r\nUID:series\r\nRECURRENCE-ID;TZID=Asia/Kolkata:20261102T110000\r\nDTSTART;TZID=Asia/Kolkata:20261102T140000\r\nDTEND;TZID=Asia/Kolkata:20261102T143000\r\nSUMMARY:Standup\r\nEND:VEVENT\r\n";

/// `raw` with [`MOVED_OCCURRENCE`] appended as a `RECURRENCE-ID` exception.
#[must_use]
pub(crate) fn with_exception(raw: &str) -> String {
    raw.replacen(
        "END:VCALENDAR",
        &format!("{MOVED_OCCURRENCE}END:VCALENDAR"),
        1,
    )
}
