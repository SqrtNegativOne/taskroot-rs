use chrono::{DateTime, Utc};
use color_eyre::Result;
use rrule::RRuleSet;
use std::str::FromStr;

/// Parse an RRULE string and return a list of occurrences between `start` and `end`.
///
/// # Errors
/// Returns an error if the `rrule_str` cannot be parsed.
pub fn get_occurrences(
    rrule_str: &str,
    dt_start: &DateTime<Utc>,
    end_date: &DateTime<Utc>,
) -> Result<Vec<DateTime<rrule::Tz>>> {
    let mut full_rrule = rrule_str.to_string();
    if !full_rrule.contains("DTSTART") {
        let dt_start_str = dt_start.format("%Y%m%dT%H%M%SZ");
        full_rrule = format!("DTSTART:{dt_start_str}\n{full_rrule}");
    }

    let rrule_set = RRuleSet::from_str(&full_rrule)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse rrule: {e:?}"))?;

    let dt_start_tz = dt_start.with_timezone(&rrule::Tz::UTC);
    let end_date_tz = end_date.with_timezone(&rrule::Tz::UTC);

    let rrule_set = rrule_set.after(dt_start_tz).before(end_date_tz);

    let occurrences = rrule_set.all(100).dates;

    Ok(occurrences)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_rrule_parsing() {
        let start = Utc::now();
        let end = start
            .checked_add_signed(chrono::Duration::try_days(10).unwrap())
            .unwrap();
        let rule = format!(
            "DTSTART:{}\nRRULE:FREQ=DAILY;COUNT=5",
            start.format("%Y%m%dT%H%M%SZ")
        );

        let occ = get_occurrences(&rule, &start, &end).unwrap();
        assert!(!occ.is_empty());
    }
}
