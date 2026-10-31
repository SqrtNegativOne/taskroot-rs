use chrono::{DateTime, Days, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz as ChronoTz;
use color_eyre::Result;
use rrule::RRuleSet;
use std::str::FromStr;

use crate::features::calendar::domain::wall_clock_to_utc;
use crate::error::AppError;

const ICAL_DATE: &str = "%Y%m%d";
const ICAL_DATE_TIME: &str = "%Y%m%dT%H%M%S";
const ICAL_UTC: &str = "%Y%m%dT%H%M%SZ";
const DTO_DATE: &str = "%Y-%m-%d";
const DTO_DATE_TIME: &str = "%Y-%m-%dT%H:%M:%S";

/// Rewrite a series' `RRULE` so it ends immediately before `occurrence_start`.
///
/// `occurrence_start` is the canonical occurrence value: `YYYY-MM-DD` for an
/// all-day event, the wall clock in the event's zone for a timed one. `UNTIL`
/// carries the same value type as `DTSTART` — a date for an all-day series, UTC
/// for a zoned one, and a floating local time when the event has no zone.
///
/// # Errors
/// Returns an error when the occurrence start or the resulting rule is invalid.
pub fn truncate_before(
    rrule: &str,
    occurrence_start: &str,
    timezone: Option<&str>,
) -> Result<String, AppError> {
    let until = until_before(occurrence_start, timezone)?;
    Ok(rrule
        .lines()
        .map(|line| replace_until(line, &until))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn until_before(occurrence_start: &str, timezone: Option<&str>) -> Result<String, AppError> {
    if let Ok(date) = NaiveDate::parse_from_str(occurrence_start, DTO_DATE) {
        let previous = date
            .checked_sub_days(Days::new(1))
            .ok_or_else(|| invalid("the occurrence date cannot be moved back a day"))?;
        return Ok(previous.format(ICAL_DATE).to_string());
    }
    let wall = NaiveDateTime::parse_from_str(occurrence_start, DTO_DATE_TIME).map_err(|_| {
        invalid(format!(
            "`{occurrence_start}` is not a canonical occurrence start"
        ))
    })?;
    let last = wall
        .checked_sub_signed(chrono::Duration::seconds(1))
        .ok_or_else(|| invalid("the occurrence cannot be moved back a second"))?;
    match timezone {
        None => Ok(last.format(ICAL_DATE_TIME).to_string()),
        Some(zone) => {
            let instant = wall_clock_to_utc(&last.format(DTO_DATE_TIME).to_string(), Some(zone))
                .ok_or_else(|| invalid(format!("`{occurrence_start}` does not resolve in {zone}")))?;
            Ok(instant.format(ICAL_UTC).to_string())
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
    exdates: &[String],
    range_start: &DateTime<Utc>,
    range_end: &DateTime<Utc>,
) -> Result<Vec<DateTime<Utc>>> {
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
            format!("DTSTART:{}Z", dt_start.format(ICAL_DATE_TIME))
        };
        full_rrule = format!("{anchor}\n{full_rrule}");
    }
    for exdate in timed_exdates(exdates, tz, named) {
        full_rrule.push('\n');
        full_rrule.push_str(&exdate);
    }

    let rrule_set = RRuleSet::from_str(&full_rrule)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse rrule: {e:?}"))?
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
/// same anchor timezone. `exdates` holds canonical `YYYY-MM-DD` values.
///
/// # Errors
/// Returns an error if the rule cannot be parsed or the range is invalid.
pub fn get_all_day_occurrences(
    rrule_str: &str,
    start_date: NaiveDate,
    timezone: Option<&str>,
    exdates: &[String],
    range_start: NaiveDate,
    range_end_inclusive: NaiveDate,
) -> Result<Vec<NaiveDate>> {
    let tz = resolve_tz(timezone);

    let mut full_rrule = rrule_str.to_string();
    if !full_rrule.contains("DTSTART") {
        let anchor = format!(
            "DTSTART;TZID={}:{}T000000",
            tz.name(),
            start_date.format("%Y%m%d")
        );
        full_rrule = format!("{anchor}\n{full_rrule}");
    }
    for exdate in all_day_exdates(exdates, tz.name()) {
        full_rrule.push('\n');
        full_rrule.push_str(&exdate);
    }

    let rrule_set = RRuleSet::from_str(&full_rrule)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse rrule: {e:?}"))?
        .limit();

    let range_start_dt = midnight_in(tz, range_start)?;
    let range_end_exclusive = range_end_inclusive
        .checked_add_days(Days::new(1))
        .ok_or_else(|| color_eyre::eyre::eyre!("range end overflow"))?;
    let range_end_dt = midnight_in(tz, range_end_exclusive)?;

    Ok((&rrule_set)
        .into_iter()
        .skip_while(|dt| *dt < range_start_dt)
        .take_while(|dt| *dt < range_end_dt)
        .map(|dt| dt.date_naive())
        .collect())
}

/// Rebuild `EXDATE` lines for a timed recurrence from the canonical slice.
fn timed_exdates(exdates: &[String], tz: rrule::Tz, named: bool) -> Vec<String> {
    exdates
        .iter()
        .filter_map(|value| {
            let datetime = NaiveDateTime::parse_from_str(
                value.strip_suffix('Z').unwrap_or(value),
                DTO_DATE_TIME,
            )
            .or_else(|_| {
                NaiveDate::parse_from_str(value, DTO_DATE).map(|date| date.and_time(NaiveTime::MIN))
            })
            .ok()?;
            Some(if named {
                format!(
                    "EXDATE;TZID={}:{}",
                    tz.name(),
                    datetime.format(ICAL_DATE_TIME)
                )
            } else {
                format!("EXDATE:{}Z", datetime.format(ICAL_DATE_TIME))
            })
        })
        .collect()
}

/// Rebuild date-only `EXDATE` lines in the anchor timezone.
fn all_day_exdates(exdates: &[String], tz_name: &str) -> Vec<String> {
    exdates
        .iter()
        .filter_map(|value| {
            let date =
                NaiveDate::parse_from_str(value.get(..10).unwrap_or(value), DTO_DATE).ok()?;
            Some(format!(
                "EXDATE;TZID={tz_name}:{}T000000",
                date.format("%Y%m%d")
            ))
        })
        .collect()
}

fn resolve_tz(timezone: Option<&str>) -> rrule::Tz {
    timezone
        .and_then(|name| ChronoTz::from_str(name).ok())
        .map_or(rrule::Tz::UTC, rrule::Tz::from)
}

fn midnight_in(tz: rrule::Tz, date: NaiveDate) -> Result<DateTime<rrule::Tz>> {
    let naive = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| color_eyre::eyre::eyre!("invalid midnight"))?;
    tz.from_local_datetime(&naive)
        .earliest()
        .ok_or_else(|| color_eyre::eyre::eyre!("no local midnight for {date}"))
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
        let london = ChronoTz::from_str("Europe/London").unwrap();
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
            &["2026-09-02T14:30:00".to_string()],
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
            &["2026-09-14".to_string()],
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
    fn truncating_before_an_all_day_occurrence_ends_on_the_previous_day() {
        let rrule =
            truncate_before("RRULE:FREQ=WEEKLY;BYDAY=MO", "2026-10-26", Some("Asia/Kolkata"))
                .unwrap();

        assert_eq!(rrule, "RRULE:FREQ=WEEKLY;BYDAY=MO;UNTIL=20261025");
    }

    #[test]
    fn truncating_a_zoned_series_writes_until_in_utc() {
        // 11:00 in Asia/Kolkata is 05:30Z, so the last kept instant is 05:29:59Z.
        let rrule = truncate_before(
            "RRULE:FREQ=WEEKLY;COUNT=5",
            "2026-10-26T11:00:00",
            Some("Asia/Kolkata"),
        )
        .unwrap();

        assert_eq!(rrule, "RRULE:FREQ=WEEKLY;UNTIL=20261026T052959Z");
    }

    #[test]
    fn truncating_a_floating_series_keeps_until_floating() {
        let rrule =
            truncate_before("RRULE:FREQ=DAILY", "2026-10-26T09:00:00", None).unwrap();

        assert_eq!(rrule, "RRULE:FREQ=DAILY;UNTIL=20261026T085959");
    }

    #[test]
    fn a_truncated_series_stops_before_the_requested_occurrence() {
        let dt_start = wall_clock_to_utc("2026-10-19T11:00:00", Some("Asia/Kolkata")).unwrap();
        let rrule = truncate_before(
            "RRULE:FREQ=WEEKLY",
            "2026-10-26T11:00:00",
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
}
