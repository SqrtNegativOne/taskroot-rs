//! The requested render window shared by the event and task projections.
//!
//! Callers pass two local calendar dates (`YYYY-MM-DD`) or RFC 3339 instants.
//! Date-only boundaries are interpreted in the host's local calendar (the
//! user's timezone); instants are used as-is.

use crate::features::calendar::domain::DTO_DATE;
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
    NaiveDate::parse_from_str(iso, DTO_DATE).ok()
}

fn parse_boundary_instant(iso: &str, end_of_day: bool) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        return Some(dt.with_timezone(&Utc));
    }
    let date = NaiveDate::parse_from_str(iso, DTO_DATE).ok()?;
    boundary_instant_in_tz(date, end_of_day, &chrono::Local)
}

fn boundary_instant_in_tz<Tz: chrono::TimeZone>(
    date: NaiveDate,
    end_of_day: bool,
    tz: &Tz,
) -> Option<DateTime<Utc>> {
    let time = if end_of_day {
        NaiveTime::from_hms_opt(23, 59, 59)?
    } else {
        NaiveTime::from_hms_opt(0, 0, 0)?
    };
    let naive = date.and_time(time);
    let resolved = match tz.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
        chrono::LocalResult::Ambiguous(earliest, latest) => {
            if end_of_day {
                latest.with_timezone(&Utc)
            } else {
                earliest.with_timezone(&Utc)
            }
        }
        chrono::LocalResult::None => {
            let step = if end_of_day { -30_i64 } else { 30_i64 };
            [1_i64, 2, 4]
                .into_iter()
                .find_map(|factor| {
                    let shifted = naive.checked_add_signed(chrono::Duration::minutes(
                        step.saturating_mul(factor),
                    ))?;
                    let res = tz.from_local_datetime(&shifted);
                    if end_of_day {
                        res.latest()
                    } else {
                        res.earliest()
                    }
                })
                .map_or_else(|| naive.and_utc(), |dt| dt.with_timezone(&Utc))
        }
    };
    Some(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_instant_handles_midnight_dst_gap_and_ambiguous_fall_back() {
        // America/Havana transitions at midnight:
        // - 2026-03-08 00:00 -> 01:00 (LocalResult::None)
        // - 2026-11-01 01:00 -> 00:00 (00:00 is LocalResult::Ambiguous)
        let havana = chrono_tz::America::Havana;
        let spring_forward = NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
        let fall_back = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();

        assert!(
            boundary_instant_in_tz(spring_forward, false, &havana).is_some(),
            "midnight DST spring-forward gap must not collapse the render window"
        );
        assert!(
            boundary_instant_in_tz(fall_back, false, &havana).is_some(),
            "ambiguous midnight DST fall-back must not collapse the render window"
        );
    }
}
