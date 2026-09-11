use chrono::{DateTime, Days, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz as ChronoTz;
use color_eyre::Result;
use rrule::RRuleSet;
use std::str::FromStr;

/// Parse an RRULE string and return the occurrences that fall within the
/// inclusive window `[range_start, range_end]`.
///
/// `dt_start` anchors the recurrence (the master event's original start), while
/// `range_start` / `range_end` bound the occurrences we actually want to render.
/// Anchoring the query on `range_start` (instead of `dt_start`) is what keeps
/// long-lived recurrences visible far from their original `DTSTART`.
///
/// # Errors
/// Returns an error if the `rrule_str` cannot be parsed.
pub fn get_occurrences(
    rrule_str: &str,
    dt_start: &DateTime<Utc>,
    range_start: &DateTime<Utc>,
    range_end: &DateTime<Utc>,
) -> Result<Vec<DateTime<rrule::Tz>>> {
    let mut full_rrule = rrule_str.to_string();
    if !full_rrule.contains("DTSTART") {
        let dt_start_str = dt_start.format("%Y%m%dT%H%M%SZ");
        full_rrule = format!("DTSTART:{dt_start_str}\n{full_rrule}");
    }

    // `.limit()` enables the crate's iteration guards so a pathological rule can
    // never spin forever while we walk from `DTSTART` up to the requested range.
    let rrule_set = RRuleSet::from_str(&full_rrule)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse rrule: {e:?}"))?
        .limit();

    let range_start_tz = range_start.with_timezone(&rrule::Tz::UTC);
    let range_end_tz = range_end.with_timezone(&rrule::Tz::UTC);

    let occurrences = (&rrule_set)
        .into_iter()
        .skip_while(|dt| *dt < range_start_tz)
        .take_while(|dt| *dt <= range_end_tz)
        .collect();

    Ok(occurrences)
}

/// Expand a **floating-date** all-day recurrence.
///
/// All-day events have no instant; they are anchored at midnight in the
/// event's IANA timezone (falling back to UTC) so `BYDAY=MO` keeps landing on
/// Mondays regardless of the host process timezone. The returned values are
/// dates in that same anchor timezone, never converted through a system-local
/// instant.
///
/// # Errors
/// Returns an error if the `rrule_str` cannot be parsed or the range is invalid.
pub fn get_all_day_occurrences(
    rrule_str: &str,
    start_date: NaiveDate,
    timezone: Option<&str>,
    range_start: NaiveDate,
    range_end_inclusive: NaiveDate,
) -> Result<Vec<NaiveDate>> {
    let tz = resolve_tz(timezone);

    let mut full_rrule = rewrite_all_day_exdates(rrule_str, tz.name());
    if !full_rrule.contains("DTSTART") {
        let dt_start_str = format!("{}T000000", start_date.format("%Y%m%d"));
        full_rrule = format!("DTSTART;TZID={}:{dt_start_str}\n{full_rrule}", tz.name());
    }

    let rrule_set = RRuleSet::from_str(&full_rrule)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse rrule: {e:?}"))?
        .limit();

    let range_start_dt = midnight_in(tz, range_start)?;
    let range_end_exclusive = range_end_inclusive
        .checked_add_days(Days::new(1))
        .ok_or_else(|| color_eyre::eyre::eyre!("range end overflow"))?;
    let range_end_dt = midnight_in(tz, range_end_exclusive)?;

    let occurrences = (&rrule_set)
        .into_iter()
        .skip_while(|dt| *dt < range_start_dt)
        .take_while(|dt| *dt < range_end_dt)
        .map(|dt| dt.date_naive())
        .collect();

    Ok(occurrences)
}

/// Rewrite date-only `EXDATE` values (Google's all-day form) into the anchor
/// timezone so the rule engine can match them against floating occurrences.
fn rewrite_all_day_exdates(rrule_str: &str, tz_name: &str) -> String {
    rrule_str
        .lines()
        .map(|line| rewrite_exdate_line(line, tz_name))
        .collect::<Vec<_>>()
        .join("\n")
}

fn rewrite_exdate_line(line: &str, tz_name: &str) -> String {
    let Some((prefix, value)) = line.split_once(':') else {
        return line.to_string();
    };
    if !prefix.to_ascii_uppercase().starts_with("EXDATE") || value.contains('T') {
        return line.to_string();
    }
    let dates = value
        .split(',')
        .map(|date| format!("{}T000000", date.trim()))
        .collect::<Vec<_>>()
        .join(",");
    format!("EXDATE;TZID={tz_name}:{dates}")
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
#[allow(clippy::unwrap_used)]
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

        let occ = get_occurrences(rule, &start, &start, &end).unwrap();
        assert_eq!(occ.len(), 5);
    }

    #[test]
    fn test_daily_rrule_still_occurs_long_after_dt_start() {
        let dt_start = Utc.with_ymd_and_hms(2025, 1, 1, 9, 0, 0).unwrap();
        let range_start = Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2026, 9, 7, 23, 59, 59).unwrap();

        let occ = get_occurrences("RRULE:FREQ=DAILY", &dt_start, &range_start, &range_end).unwrap();

        assert_eq!(occ.len(), 7, "one occurrence per day across the week");
    }

    #[test]
    fn test_occurrences_are_clipped_to_requested_range() {
        let dt_start = Utc.with_ymd_and_hms(2026, 9, 1, 9, 0, 0).unwrap();
        let range_start = Utc.with_ymd_and_hms(2026, 9, 3, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2026, 9, 4, 23, 59, 59).unwrap();

        let occ = get_occurrences("RRULE:FREQ=DAILY", &dt_start, &range_start, &range_end).unwrap();

        assert_eq!(occ.len(), 2);
    }

    fn assert_all_mondays(timezone: Option<&str>) {
        let start = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let range_end = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();

        let occ = get_all_day_occurrences(
            "RRULE:FREQ=WEEKLY;BYDAY=MO",
            start,
            timezone,
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
}
