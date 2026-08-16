use chrono::{DateTime, Duration, Local, TimeZone, Utc};

#[must_use]
/// Get the current UTC time.
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

#[must_use]
/// Get the current local time.
pub fn now_local() -> DateTime<Local> {
    Local::now()
}

#[must_use]
/// Add days to a given date safely. Returns None if it overflows.
pub fn add_days<T: TimeZone>(date: &DateTime<T>, days: i64) -> Option<DateTime<T>> {
    date.clone().checked_add_signed(Duration::try_days(days)?)
}

#[must_use]
/// Start of the day for a given date in local time.
pub fn start_of_day_local(date: &DateTime<Local>) -> Option<DateTime<Local>> {
    date.date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|naive| Local.from_local_datetime(&naive).single())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_add_days() {
        let now = now_utc();
        let tomorrow = add_days(&now, 1).unwrap();
        assert!(tomorrow > now);
    }
}
