//! Typed calendar dates/times and canonical format/timezone helpers.
//!
//! Single source of truth for iCalendar and DTO date-time formats, wall-clock
//! timezone resolution, and the [`CalendarDateTime`] value carried on domain
//! DTOs.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, TimeZone, Utc};
use chrono_tz::Tz as ChronoTz;
use serde::{Deserialize, Serialize};
use specta::Type;

pub const ICAL_DATE: &str = "%Y%m%d";
pub const ICAL_DATE_TIME: &str = "%Y%m%dT%H%M%S";
pub const ICAL_UTC: &str = "%Y%m%dT%H%M%SZ";
pub const DTO_DATE: &str = "%Y-%m-%d";
pub const DTO_DATE_TIME: &str = "%Y-%m-%dT%H:%M:%S";
pub const DTO_ISO: &str = "%Y-%m-%dT%H:%M:%SZ";

const SPACE_DATE_TIME: &str = "%Y-%m-%d %H:%M:%S";

/// A calendar date or date-time carried on domain DTOs.
///
/// Serializes transparently as its canonical ISO-8601 string:
/// - [`Self::Date`][] — `YYYY-MM-DD` (floating calendar date)
/// - [`Self::WallClock`][] — `YYYY-MM-DDTHH:MM:SS` (local wall clock in the owning
///   item's `timezone`, falling back to UTC when absent)
/// - [`Self::Utc`][] — `YYYY-MM-DDTHH:MM:SSZ` (explicit UTC instant; RFC 3339
///   offsets are normalized to UTC on parse)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum CalendarDateTime {
    Date(NaiveDate),
    WallClock(NaiveDateTime),
    Utc(DateTime<Utc>),
}

mod wire {
    use specta::Type;

    #[derive(Type)]
    #[specta(transparent)]
    pub(super) struct CalendarDateTime(#[allow(dead_code)] String);
}

impl Type for CalendarDateTime {
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
        wire::CalendarDateTime::definition(types)
    }
}

impl CalendarDateTime {
    /// Whether this value is a floating calendar date with no time of day.
    #[must_use]
    pub const fn is_date(self) -> bool {
        matches!(self, Self::Date(_))
    }

    /// Whether this value carries a time of day (wall clock or UTC instant).
    #[must_use]
    pub const fn is_timed(self) -> bool {
        !self.is_date()
    }

    /// The calendar day of this value.
    #[must_use]
    pub fn date(self) -> NaiveDate {
        match self {
            Self::Date(date) => date,
            Self::WallClock(naive) => naive.date(),
            Self::Utc(instant) => instant.date_naive(),
        }
    }

    /// Project a UTC instant into a wall clock in `timezone` (UTC fallback).
    #[must_use]
    pub fn from_utc_in_zone(instant: DateTime<Utc>, timezone: Option<&str>) -> Self {
        Self::WallClock(utc_to_wall_clock(instant, timezone))
    }

    /// Resolve a timed value to a UTC instant in `timezone` (UTC fallback).
    /// Returns `None` for a floating date.
    #[must_use]
    pub fn to_utc(self, timezone: Option<&str>) -> Option<DateTime<Utc>> {
        match self {
            Self::Date(_) => None,
            Self::WallClock(naive) => Some(resolve_wall_clock(naive, timezone)),
            Self::Utc(instant) => Some(instant),
        }
    }

    /// Resolve any value to a UTC instant, using `default_time` for date-only values.
    #[must_use]
    pub fn to_utc_with_default_time(
        self,
        default_time: NaiveTime,
        timezone: Option<&str>,
    ) -> DateTime<Utc> {
        match self {
            Self::Date(date) => resolve_wall_clock(date.and_time(default_time), timezone),
            Self::WallClock(naive) => resolve_wall_clock(naive, timezone),
            Self::Utc(instant) => instant,
        }
    }

    /// Project a timed value to its wall-clock `NaiveDateTime` in `timezone`.
    /// Returns `None` for a floating date.
    #[must_use]
    pub fn to_wall_clock(self, timezone: Option<&str>) -> Option<NaiveDateTime> {
        match self {
            Self::Date(_) => None,
            Self::WallClock(naive) => Some(naive),
            Self::Utc(instant) => Some(utc_to_wall_clock(instant, timezone)),
        }
    }

    /// Project any value to a `NaiveDateTime` in `timezone`, defaulting dates to midnight.
    #[must_use]
    pub fn to_naive_datetime(self, timezone: Option<&str>) -> NaiveDateTime {
        match self {
            Self::Date(date) => date.and_time(NaiveTime::MIN),
            Self::WallClock(naive) => naive,
            Self::Utc(instant) => utc_to_wall_clock(instant, timezone),
        }
    }

    fn sort_key(self) -> (NaiveDate, Option<NaiveTime>, u8) {
        match self {
            Self::Date(date) => (date, None, 0),
            Self::WallClock(naive) => (naive.date(), Some(naive.time()), 1),
            Self::Utc(instant) => (instant.date_naive(), Some(instant.time()), 2),
        }
    }
}

impl Ord for CalendarDateTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl PartialOrd for CalendarDateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for CalendarDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Date(date) => write!(f, "{}", date.format(DTO_DATE)),
            Self::WallClock(naive) => write!(f, "{}", naive.format(DTO_DATE_TIME)),
            Self::Utc(instant) => {
                write!(f, "{}", instant.to_rfc3339_opts(SecondsFormat::Secs, true))
            }
        }
    }
}

impl FromStr for CalendarDateTime {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Ok(date) = NaiveDate::parse_from_str(value, DTO_DATE) {
            return Ok(Self::Date(date));
        }
        if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
            return Ok(Self::Utc(datetime.with_timezone(&Utc)));
        }
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, DTO_DATE_TIME)
            .or_else(|_| NaiveDateTime::parse_from_str(value, SPACE_DATE_TIME))
        {
            return Ok(Self::WallClock(naive));
        }
        Err(format!("invalid calendar date/time: '{value}'"))
    }
}

impl TryFrom<String> for CalendarDateTime {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl TryFrom<&str> for CalendarDateTime {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl From<CalendarDateTime> for String {
    fn from(value: CalendarDateTime) -> Self {
        value.to_string()
    }
}

/// Parse an optional IANA timezone name.
#[must_use]
pub fn parse_timezone(timezone: Option<&str>) -> Option<ChronoTz> {
    timezone.and_then(|name| ChronoTz::from_str(name).ok())
}

/// Interpret a naive wall-clock date-time in `timezone` (falling back to UTC).
///
/// A local time that does not exist (a DST spring-forward gap) degrades to UTC
/// rather than dropping the occurrence; an ambiguous fall-back time picks the
/// earliest instant.
#[must_use]
pub fn resolve_wall_clock(naive: NaiveDateTime, timezone: Option<&str>) -> DateTime<Utc> {
    let zone = parse_timezone(timezone).unwrap_or(ChronoTz::UTC);
    match zone.from_local_datetime(&naive) {
        chrono::LocalResult::Single(datetime) => datetime.with_timezone(&Utc),
        chrono::LocalResult::Ambiguous(earliest, _) => earliest.with_timezone(&Utc),
        chrono::LocalResult::None => naive.and_utc(),
    }
}

/// Project a UTC instant into a wall-clock `NaiveDateTime` in `timezone` (UTC fallback).
#[must_use]
pub fn utc_to_wall_clock(instant: DateTime<Utc>, timezone: Option<&str>) -> NaiveDateTime {
    parse_timezone(timezone).map_or_else(
        || instant.naive_utc(),
        |zone| instant.with_timezone(&zone).naive_local(),
    )
}

#[cfg(test)]
mod tests;
