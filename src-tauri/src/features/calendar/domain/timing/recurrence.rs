use chrono::{DateTime, Days, NaiveDate, TimeZone, Utc};
use rrule::RRuleSet;
use std::str::FromStr;

use crate::error::AppError;
use crate::features::calendar::domain::dates::{
    parse_timezone, resolve_wall_clock, CalendarDateTime, ICAL_DATE, ICAL_DATE_TIME, ICAL_UTC,
};

/// Rewrite a series' `RRULE` so it ends immediately before `occurrence_start`.
///
/// `occurrence_start` is the canonical occurrence value: a floating date for an
/// all-day event, the wall clock in the event's zone for a timed one. `UNTIL`
/// carries the same value type as `DTSTART` — a date for an all-day series, UTC
/// for a zoned one, and a floating local time when the event has no zone.
///
/// # Errors
/// Returns an error when the occurrence start or the resulting rule is invalid.
pub fn truncate_before(
    rrule: &str,
    occurrence_start: CalendarDateTime,
    timezone: Option<&str>,
) -> Result<String, AppError> {
    let until = until_before(occurrence_start, timezone)?;
    Ok(rrule
        .lines()
        .map(|line| replace_until(line, &until))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn until_before(
    occurrence_start: CalendarDateTime,
    timezone: Option<&str>,
) -> Result<String, AppError> {
    match occurrence_start {
        CalendarDateTime::Date(date) => {
            let previous = date
                .checked_sub_days(Days::new(1))
                .ok_or_else(|| invalid("the occurrence date cannot be moved back a day"))?;
            Ok(previous.format(ICAL_DATE).to_string())
        }
        CalendarDateTime::WallClock(wall) => {
            if let Some(zone) = timezone {
                let last = resolve_wall_clock(wall, Some(zone))
                    .checked_sub_signed(chrono::Duration::seconds(1))
                    .ok_or_else(|| invalid("the occurrence cannot be moved back a second"))?;
                return Ok(last.format(ICAL_UTC).to_string());
            }
            let last = wall
                .checked_sub_signed(chrono::Duration::seconds(1))
                .ok_or_else(|| invalid("the occurrence cannot be moved back a second"))?;
            Ok(last.format(ICAL_DATE_TIME).to_string())
        }
        CalendarDateTime::Utc(instant) => {
            let last = instant
                .checked_sub_signed(chrono::Duration::seconds(1))
                .ok_or_else(|| invalid("the occurrence cannot be moved back a second"))?;
            Ok(last.format(ICAL_UTC).to_string())
        }
    }
}

fn replace_until(line: &str, until: &str) -> String {
    let Some((name, value)) = line.split_once(':') else {
        return line.to_string();
    };
    if !name.trim().eq_ignore_ascii_case("RRULE") {
        return line.to_string();
    }
    let kept: Vec<&str> = value
        .split(';')
        .filter(|part| !is_until_or_count(part))
        .collect();
    if kept.is_empty() {
        return format!("{name}:UNTIL={until}");
    }
    format!("{name}:{};UNTIL={until}", kept.join(";"))
}

fn is_until_or_count(part: &str) -> bool {
    let name = part.split_once('=').map_or(part, |(name, _)| name).trim();
    name.eq_ignore_ascii_case("UNTIL") || name.eq_ignore_ascii_case("COUNT")
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::InvalidInput(message.into())
}

/// Expand a **timed** recurrence in the event's own timezone.
///
/// `DTSTART` is anchored as `DTSTART;TZID=<zone>`, so `BYDAY`/`BYHOUR` are resolved
/// against the wall clock and a weekly 09:00 event stays at 09:00 local across a DST
/// change (plan §8). Each occurrence is returned as a UTC instant for
/// [`EventInstance`](crate::features::calendar::domain::EventInstance).
///
/// `exdates` is the canonical `exdates` slice — wall-clock values in `zone`
/// (`YYYY-MM-DDTHH:MM:SS`); the exclusion lines are rebuilt from it rather than
/// scanned out of `rrule_str`, which now carries `RRULE` lines only.
///
/// Anchoring the query window on `range_start` instead of `dt_start` keeps
/// long-lived recurrences visible far from their original `DTSTART`.
///
/// # Errors
/// Returns an error if the rule cannot be parsed.
pub fn get_occurrences(
    rrule_str: &str,
    dt_start: &DateTime<Utc>,
    timezone: Option<&str>,
    exdates: &[CalendarDateTime],
    range_start: &DateTime<Utc>,
    range_end: &DateTime<Utc>,
) -> Result<Vec<DateTime<Utc>>, AppError> {
    let tz = resolve_tz(timezone);
    let named = timezone.is_some_and(|_| tz != rrule::Tz::UTC);
    let mut full_rrule = rrule_str.to_string();
    if !full_rrule.contains("DTSTART") {
        let anchor = if named {
            format!(
                "DTSTART;TZID={}:{}",
                tz.name(),
                dt_start.with_timezone(&tz).format(ICAL_DATE_TIME)
            )
        } else {
            format!("DTSTART:{}", dt_start.format(ICAL_UTC))
        };
        full_rrule = format!("{anchor}\n{full_rrule}");
    }
    for exdate in timed_exdates(exdates, tz, named, timezone) {
        full_rrule.push('\n');
        full_rrule.push_str(&exdate);
    }

    let rrule_set = RRuleSet::from_str(&full_rrule)
        .map_err(|e| invalid(format!("Failed to parse rrule: {e:?}")))?
        .limit();

    Ok((&rrule_set)
        .into_iter()
        .map(|dt| dt.with_timezone(&Utc))
        .skip_while(|dt| dt < range_start)
        .take_while(|dt| dt <= range_end)
        .collect())
}

/// Expand a **floating-date** all-day recurrence.
///
/// All-day events have no instant; they are anchored at midnight in the event's
/// IANA timezone (falling back to UTC) so `BYDAY=MO` keeps landing on Mondays
/// regardless of the host process timezone. The returned values are dates in that
/// same anchor timezone.
///
/// # Errors
/// Returns an error if the rule cannot be parsed or the range is invalid.
pub fn get_all_day_occurrences(
    rrule_str: &str,
    start_date: NaiveDate,
    _timezone: Option<&str>,
    exdates: &[CalendarDateTime],
    range_start: NaiveDate,
    range_end_inclusive: NaiveDate,
) -> Result<Vec<NaiveDate>, AppError> {
    // Floating all-day recurrences have no instant or DST transitions; anchoring
    // them at UTC midnight keeps weekdays independent of the host timezone and
    // avoids midnight spring-forward gaps (e.g. America/Havana, America/Santiago).
    let tz = rrule::Tz::UTC;

    let mut full_rrule = rrule_str.to_string();
    if !full_rrule.contains("DTSTART") {
        let anchor = format!(
            "DTSTART;TZID={}:{}T000000",
            tz.name(),
            start_date.format(ICAL_DATE)
        );
        full_rrule = format!("{anchor}\n{full_rrule}");
    }
    for exdate in all_day_exdates(exdates, tz.name()) {
        full_rrule.push('\n');
        full_rrule.push_str(&exdate);
    }

    let rrule_set = RRuleSet::from_str(&full_rrule)
        .map_err(|e| invalid(format!("Failed to parse rrule: {e:?}")))?
        .limit();

    let range_start_dt = midnight_in(tz, range_start)?;
    let range_end_exclusive = range_end_inclusive
        .checked_add_days(Days::new(1))
        .ok_or_else(|| invalid("range end overflow"))?;
    let range_end_dt = midnight_in(tz, range_end_exclusive)?;

    Ok((&rrule_set)
        .into_iter()
        .skip_while(|dt| *dt < range_start_dt)
        .take_while(|dt| *dt < range_end_dt)
        .map(|dt| dt.date_naive())
        .collect())
}

/// Rebuild `EXDATE` lines for a timed recurrence from the canonical slice.
fn timed_exdates(
    exdates: &[CalendarDateTime],
    tz: rrule::Tz,
    named: bool,
    timezone: Option<&str>,
) -> Vec<String> {
    exdates
        .iter()
        .map(|value| {
            let datetime = value.to_naive_datetime(timezone);
            if named {
                format!(
                    "EXDATE;TZID={}:{}",
                    tz.name(),
                    datetime.format(ICAL_DATE_TIME)
                )
            } else {
                format!("EXDATE:{}", datetime.format(ICAL_UTC))
            }
        })
        .collect()
}

/// Rebuild date-only `EXDATE` lines in the anchor timezone.
fn all_day_exdates(exdates: &[CalendarDateTime], tz_name: &str) -> Vec<String> {
    exdates
        .iter()
        .map(|value| {
            format!(
                "EXDATE;TZID={tz_name}:{}T000000",
                value.date().format(ICAL_DATE)
            )
        })
        .collect()
}

fn resolve_tz(timezone: Option<&str>) -> rrule::Tz {
    parse_timezone(timezone).map_or(rrule::Tz::UTC, rrule::Tz::from)
}

fn midnight_in(tz: rrule::Tz, date: NaiveDate) -> Result<DateTime<rrule::Tz>, AppError> {
    let naive = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| invalid("invalid midnight"))?;
    tz.from_local_datetime(&naive)
        .earliest()
        .ok_or_else(|| invalid(format!("no local midnight for {date}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use chrono::TimeZone;

    #[test]
    fn test_rrule_parsing() {
        let start = Utc.with_ymd_and_hms(2026, 9, 1, 9, 0, 0).unwrap();
        let end = start
            .checked_add_signed(chrono::Duration::try_days(10).unwrap())
            .unwrap();
        let rule = "RRULE:FREQ=DAILY;COUNT=5";

        let occ = get_occurrences(rule, &start, None, &[], &start, &end).unwrap();
        assert_eq!(occ.len(), 5);
    }

    #[test]
    fn test_daily_rrule_still_occurs_long_after_dt_start() {
        let dt_start = Utc.with_ymd_and_hms(2025, 1, 1, 9, 0, 0).unwrap();
        let range_start = Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2026, 9, 7, 23, 59, 59).unwrap();

        let occ = get_occurrences(
            "RRULE:FREQ=DAILY",
            &dt_start,
            None,
            &[],
            &range_start,
            &range_end,
        )
        .unwrap();

        assert_eq!(occ.len(), 7, "one occurrence per day across the week");
    }

    #[test]
    fn test_occurrences_are_clipped_to_requested_range() {
        let dt_start = Utc.with_ymd_and_hms(2026, 9, 1, 9, 0, 0).unwrap();
        let range_start = Utc.with_ymd_and_hms(2026, 9, 3, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2026, 9, 4, 23, 59, 59).unwrap();

        let occ = get_occurrences(
            "RRULE:FREQ=DAILY",
            &dt_start,
            None,
            &[],
            &range_start,
            &range_end,
        )
        .unwrap();

        assert_eq!(occ.len(), 2);
    }

    #[test]
    fn timed_recurrence_keeps_the_local_wall_clock_across_dst() {
        // Europe/London switches to BST on 2026-03-29: 09:00 local is 09:00Z
        // before it and 08:00Z after it. The local wall clock must not move.
        let dt_start = Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).unwrap();
        let range_start = Utc.with_ymd_and_hms(2026, 3, 16, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2026, 4, 6, 23, 59, 59).unwrap();

        let occ = get_occurrences(
            "RRULE:FREQ=WEEKLY;BYDAY=MO",
            &dt_start,
            Some("Europe/London"),
            &[],
            &range_start,
            &range_end,
        )
        .unwrap();

        let stamps = occ
            .iter()
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            stamps,
            vec![
                "2026-03-16T09:00:00Z",
                "2026-03-23T09:00:00Z",
                "2026-03-30T08:00:00Z",
                "2026-04-06T08:00:00Z",
            ]
        );
        let london = chrono_tz::Tz::from_str("Europe/London").unwrap();
        assert!(occ
            .iter()
            .all(|dt| dt.with_timezone(&london).format("%H:%M").to_string() == "09:00"));
    }

    #[test]
    fn an_exdate_wall_clock_removes_that_occurrence() {
        // 09:00Z is 14:30 in Asia/Kolkata, so the canonical exclusion value is
        // the local wall clock, not the UTC instant.
        let dt_start = Utc.with_ymd_and_hms(2026, 9, 1, 9, 0, 0).unwrap();
        let range_start = Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2026, 9, 3, 23, 59, 59).unwrap();

        let occ = get_occurrences(
            "RRULE:FREQ=DAILY;COUNT=3",
            &dt_start,
            Some("Asia/Kolkata"),
            &["2026-09-02T14:30:00".parse().unwrap()],
            &range_start,
            &range_end,
        )
        .unwrap();

        let stamps = occ
            .iter()
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .collect::<Vec<_>>();
        assert_eq!(stamps, vec!["2026-09-01", "2026-09-03"]);
    }

    fn assert_all_mondays(timezone: Option<&str>) {
        let start = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let range_end = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();

        let occ = get_all_day_occurrences(
            "RRULE:FREQ=WEEKLY;BYDAY=MO",
            start,
            timezone,
            &[],
            start,
            range_end,
        )
        .unwrap();

        assert_eq!(occ.len(), 4, "four Mondays between Sep 7 and Oct 4 2026");
        assert!(
            occ.iter().all(|d| d.weekday() == chrono::Weekday::Mon),
            "all-day weekly BYDAY=MO must stay on Mondays for timezone {timezone:?}, got {occ:?}"
        );
    }

    #[test]
    fn all_day_weekly_recurrence_stays_on_monday_for_positive_offset() {
        assert_all_mondays(Some("Asia/Kolkata"));
    }

    #[test]
    fn all_day_weekly_recurrence_stays_on_monday_for_negative_offset() {
        assert_all_mondays(Some("America/Los_Angeles"));
    }

    #[test]
    fn all_day_weekly_recurrence_stays_on_monday_in_utc() {
        assert_all_mondays(None);
    }

    #[test]
    fn an_all_day_exdate_removes_that_date() {
        let start = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let range_end = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();

        let occ = get_all_day_occurrences(
            "RRULE:FREQ=WEEKLY;BYDAY=MO",
            start,
            Some("Asia/Kolkata"),
            &["2026-09-14".parse().unwrap()],
            start,
            range_end,
        )
        .unwrap();

        assert_eq!(
            occ.iter()
                .map(|date| date.format("%Y-%m-%d").to_string())
                .collect::<Vec<_>>(),
            vec!["2026-09-07", "2026-09-21", "2026-09-28"]
        );
    }

    #[test]
    fn a_yearly_all_day_series_occurs_in_a_later_year() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 19).unwrap();
        let range_start = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let range_end = NaiveDate::from_ymd_opt(2027, 1, 31).unwrap();
        let occ = get_all_day_occurrences(
            "RRULE:FREQ=YEARLY",
            start,
            None,
            &[],
            range_start,
            range_end,
        )
        .unwrap();
        assert_eq!(occ, vec![NaiveDate::from_ymd_opt(2027, 1, 19).unwrap()]);
    }

    #[test]
    fn a_yearly_timed_series_occurs_in_a_later_year() {
        let start = Utc.with_ymd_and_hms(2026, 1, 19, 9, 0, 0).unwrap();
        let range_start = Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2027, 1, 31, 23, 59, 59).unwrap();
        let occ = get_occurrences(
            "RRULE:FREQ=YEARLY",
            &start,
            None,
            &[],
            &range_start,
            &range_end,
        )
        .unwrap();
        assert_eq!(occ.len(), 1);
        assert_eq!(occ[0].format("%Y-%m-%d").to_string(), "2027-01-19");
    }

    #[test]
    fn truncating_before_an_all_day_occurrence_ends_on_the_previous_day() {
        let rrule = truncate_before(
            "RRULE:FREQ=WEEKLY;BYDAY=MO",
            "2026-10-26".parse().unwrap(),
            Some("Asia/Kolkata"),
        )
        .unwrap();

        assert_eq!(rrule, "RRULE:FREQ=WEEKLY;BYDAY=MO;UNTIL=20261025");
    }

    #[test]
    fn truncating_a_zoned_series_writes_until_in_utc() {
        // 11:00 in Asia/Kolkata is 05:30Z, so the last kept instant is 05:29:59Z.
        let rrule = truncate_before(
            "RRULE:FREQ=WEEKLY;COUNT=5",
            "2026-10-26T11:00:00".parse().unwrap(),
            Some("Asia/Kolkata"),
        )
        .unwrap();

        assert_eq!(rrule, "RRULE:FREQ=WEEKLY;UNTIL=20261026T052959Z");
    }

    #[test]
    fn truncating_a_floating_series_keeps_until_floating() {
        let rrule = truncate_before(
            "RRULE:FREQ=DAILY",
            "2026-10-26T09:00:00".parse().unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(rrule, "RRULE:FREQ=DAILY;UNTIL=20261026T085959");
    }

    #[test]
    fn a_truncated_series_stops_before_the_requested_occurrence() {
        let dt_start = "2026-10-19T11:00:00"
            .parse::<CalendarDateTime>()
            .unwrap()
            .to_utc(Some("Asia/Kolkata"))
            .unwrap();
        let rrule = truncate_before(
            "RRULE:FREQ=WEEKLY",
            "2026-10-26T11:00:00".parse().unwrap(),
            Some("Asia/Kolkata"),
        )
        .unwrap();

        let occ = get_occurrences(
            &rrule,
            &dt_start,
            Some("Asia/Kolkata"),
            &[],
            &Utc.with_ymd_and_hms(2026, 10, 19, 0, 0, 0).unwrap(),
            &Utc.with_ymd_and_hms(2026, 11, 9, 23, 59, 59).unwrap(),
        )
        .unwrap();

        let days: Vec<String> = occ.iter().map(|dt| dt.format("%Y-%m-%d").to_string()).collect();
        assert_eq!(days, vec!["2026-10-19"], "the truncated occurrence is gone");
    }

    #[test]
    fn invalid_rrule_returns_invalid_input_error() {
        let start = Utc.with_ymd_and_hms(2026, 9, 1, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 9, 7, 23, 59, 59).unwrap();

        let timed_err =
            get_occurrences("RRULE:NOT_A_RULE", &start, None, &[], &start, &end).unwrap_err();
        assert_eq!(timed_err.code(), "invalid-input");

        let day = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let all_day_err =
            get_all_day_occurrences("RRULE:NOT_A_RULE", day, None, &[], day, day).unwrap_err();
        assert_eq!(all_day_err.code(), "invalid-input");
    }

    #[test]
    fn truncating_at_dst_spring_forward_excludes_that_occurrence() {
        // Europe/Berlin springs forward 02:00 -> 03:00 on 2026-03-29.
        // 03:00 local on 2026-03-29 is 01:00Z, while 02:59:59 local does not exist.
        let dt_start = "2026-03-22T03:00:00"
            .parse::<CalendarDateTime>()
            .unwrap()
            .to_utc(Some("Europe/Berlin"))
            .unwrap();
        let rrule = truncate_before(
            "RRULE:FREQ=WEEKLY",
            "2026-03-29T03:00:00".parse().unwrap(),
            Some("Europe/Berlin"),
        )
        .unwrap();

        let occ = get_occurrences(
            &rrule,
            &dt_start,
            Some("Europe/Berlin"),
            &[],
            &Utc.with_ymd_and_hms(2026, 3, 22, 0, 0, 0).unwrap(),
            &Utc.with_ymd_and_hms(2026, 4, 6, 23, 59, 59).unwrap(),
        )
        .unwrap();

        let days: Vec<String> = occ.iter().map(|dt| dt.format("%Y-%m-%d").to_string()).collect();
        assert_eq!(days, vec!["2026-03-22"]);
    }

    #[test]
    fn all_day_recurrence_survives_midnight_dst_transition_boundaries() {
        // America/Havana springs forward at midnight (00:00 -> 01:00) on 2026-03-08.
        let start = NaiveDate::from_ymd_opt(2026, 3, 7).unwrap();
        let range_start = NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
        let range_end = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();

        let occ = get_all_day_occurrences(
            "RRULE:FREQ=DAILY",
            start,
            Some("America/Havana"),
            &[],
            range_start,
            range_end,
        )
        .unwrap();

        assert_eq!(occ, vec![range_start, range_end]);
    }
}
