//! The requested render window shared by the event and task projections.
//!
//! Callers pass two local calendar dates (`YYYY-MM-DD`) or RFC 3339 instants.
//! Date-only boundaries are interpreted in the host's local calendar (the
//! user's timezone); instants are used as-is.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

pub struct RenderWindow {
    pub start_instant: DateTime<Utc>,
    pub end_instant: DateTime<Utc>,
    pub start_date: NaiveDate,
    pub end_date_inclusive: NaiveDate,
}

impl RenderWindow {
    /// Parse a request window, or `None` when either boundary is malformed.
    #[must_use]
    pub fn from_iso(range_start_iso: &str, range_end_iso: &str) -> Option<Self> {
        Some(Self {
            start_instant: parse_boundary_instant(range_start_iso, false)?,
            end_instant: parse_boundary_instant(range_end_iso, true)?,
            start_date: parse_boundary_date(range_start_iso)?,
            end_date_inclusive: parse_boundary_date(range_end_iso)?,
        })
    }

    /// Whether a floating date falls inside the window (inclusive).
    #[must_use]
    pub fn contains_date(&self, date: NaiveDate) -> bool {
        date >= self.start_date && date <= self.end_date_inclusive
    }

    /// Whether an instant falls inside the window (inclusive).
    #[must_use]
    pub fn contains_instant(&self, instant: DateTime<Utc>) -> bool {
        instant >= self.start_instant && instant <= self.end_instant
    }
}

fn parse_boundary_date(iso: &str) -> Option<NaiveDate> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        return Some(dt.with_timezone(&chrono::Local).date_naive());
    }
    NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()
}

fn parse_boundary_instant(iso: &str, end_of_day: bool) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        return Some(dt.with_timezone(&Utc));
    }
    let date = NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()?;
    let time = if end_of_day {
        NaiveTime::from_hms_opt(23, 59, 59)?
    } else {
        NaiveTime::from_hms_opt(0, 0, 0)?
    };
    date.and_time(time)
        .and_local_timezone(chrono::Local)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
}
