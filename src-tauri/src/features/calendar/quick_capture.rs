//! Quick-capture parsing: sigils plus one date/time phrase.
//!
//! The parser is pure and never reads the clock: the caller supplies `today`.
//! Semantics:
//! - `next <weekday>` is the first such weekday strictly after `today` (when
//!   today is that weekday it means the following week).
//! - A bare `<weekday>` and `this <weekday>` are the next occurrence including
//!   today.
//! - A date-only phrase becomes a floating `YYYY-MM-DD`; a phrase with a time
//!   becomes a naive wall-clock `YYYY-MM-DDTHH:MM:SS`. The timezone is applied
//!   by the frontend, not here.
//! - A time with no date means today at that time.
//! - Only recognized phrase tokens are removed; the rest of the title stays.

use chrono::{Datelike, Days, NaiveDate, NaiveTime, Weekday};

use crate::features::calendar::domain::{parse_sigils, CalendarDateTime, TaskPriority};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickCaptureDraft {
    pub title: String,
    pub due: Option<CalendarDateTime>,
    pub tags: Vec<String>,
    pub priority: Option<TaskPriority>,
    pub estimate_minutes: Option<i32>,
    pub list_name: Option<String>,
}

#[must_use]
pub fn parse(input: &str, today: NaiveDate) -> QuickCaptureDraft {
    let parsed = parse_sigils(input);
    let words: Vec<&str> = parsed.clean_title.split_whitespace().collect();
    let phrase = detect_phrase(&words, today);

    let title = match &phrase {
        Some(phrase) => strip_words(&words, phrase.start, phrase.end),
        None => parsed.clean_title.clone(),
    };
    let due = phrase.and_then(|phrase| phrase.resolve(today));

    QuickCaptureDraft {
        title,
        due,
        tags: parsed.properties.tags,
        priority: parsed.properties.priority,
        estimate_minutes: parsed.properties.duration,
        list_name: parsed.properties.list,
    }
}

struct Phrase {
    start: usize,
    end: usize,
    date: Option<NaiveDate>,
    time: Option<NaiveTime>,
}

impl Phrase {
    const fn resolve(self, today: NaiveDate) -> Option<CalendarDateTime> {
        match (self.date, self.time) {
            (Some(date), Some(time)) => Some(CalendarDateTime::WallClock(date.and_time(time))),
            (Some(date), None) => Some(CalendarDateTime::Date(date)),
            (None, Some(time)) => Some(CalendarDateTime::WallClock(today.and_time(time))),
            (None, None) => None,
        }
    }
}

fn detect_phrase(words: &[&str], today: NaiveDate) -> Option<Phrase> {
    for (index, _) in words.iter().enumerate() {
        let Some((date, date_end)) = parse_date_at(words, index, today) else {
            continue;
        };
        let (time, end) = parse_time_after(words, date_end);
        return Some(Phrase {
            start: index,
            end,
            date: Some(date),
            time,
        });
    }

    for (index, word) in words.iter().enumerate() {
        let Some(time) = parse_time(word) else {
            continue;
        };
        let preceding_is_at = index
            .checked_sub(1)
            .and_then(|previous| words.get(previous))
            .is_some_and(|word| word.eq_ignore_ascii_case("at"));
        let start = if preceding_is_at {
            index.saturating_sub(1)
        } else {
            index
        };
        return Some(Phrase {
            start,
            end: index.saturating_add(1),
            date: None,
            time: Some(time),
        });
    }

    None
}

fn parse_date_at(words: &[&str], index: usize, today: NaiveDate) -> Option<(NaiveDate, usize)> {
    let word = words.get(index)?.to_ascii_lowercase();
    match word.as_str() {
        "today" => return Some((today, index.saturating_add(1))),
        "tomorrow" => {
            let date = today.checked_add_days(Days::new(1)).unwrap_or(today);
            return Some((date, index.saturating_add(1)));
        }
        "next" | "this" => {
            let weekday = words
                .get(index.saturating_add(1))
                .and_then(|next| parse_weekday(next))?;
            return Some((
                weekday_date(today, weekday, word == "next"),
                index.saturating_add(2),
            ));
        }
        _ => {}
    }
    let weekday = parse_weekday(&word)?;
    Some((weekday_date(today, weekday, false), index.saturating_add(1)))
}

fn parse_time_after(words: &[&str], index: usize) -> (Option<NaiveTime>, usize) {
    let Some(word) = words.get(index) else {
        return (None, index);
    };
    let (candidate, end) = if word.eq_ignore_ascii_case("at") {
        (words.get(index.saturating_add(1)), index.saturating_add(2))
    } else {
        (Some(word), index.saturating_add(1))
    };
    candidate
        .and_then(|candidate| parse_time(candidate))
        .map_or((None, index), |time| (Some(time), end))
}

fn parse_weekday(word: &str) -> Option<Weekday> {
    match word.to_ascii_lowercase().as_str() {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tues" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thur" | "thurs" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

fn weekday_date(today: NaiveDate, weekday: Weekday, strict: bool) -> NaiveDate {
    let today_index = today.weekday().num_days_from_monday();
    let target = weekday.num_days_from_monday();
    let mut ahead = target
        .saturating_add(7)
        .saturating_sub(today_index)
        .rem_euclid(7);
    if strict && ahead == 0 {
        ahead = 7;
    }
    today
        .checked_add_days(Days::new(u64::from(ahead)))
        .unwrap_or(today)
}

fn parse_time(token: &str) -> Option<NaiveTime> {
    let lower = token.to_ascii_lowercase();
    let (body, meridiem) = split_meridiem(&lower);

    let (hour_text, minute_text) = match body.split_once(':') {
        Some((hour, minute)) => (hour, Some(minute)),
        None => (body, None),
    };
    if minute_text.is_none() && meridiem.is_none() {
        return None;
    }

    let hour: u32 = hour_text.parse().ok()?;
    let minute: u32 = match minute_text {
        Some(minute) => minute.parse().ok()?,
        None => 0,
    };
    if minute > 59 {
        return None;
    }

    let hour = if let Some(is_pm) = meridiem {
        if !(1..=12).contains(&hour) {
            return None;
        }
        match (is_pm, hour) {
            (true, 12) => 12,
            (true, hour) => hour.saturating_add(12),
            (false, 12) => 0,
            (false, hour) => hour,
        }
    } else {
        if hour > 23 {
            return None;
        }
        hour
    };

    NaiveTime::from_hms_opt(hour, minute, 0)
}

fn split_meridiem(lower: &str) -> (&str, Option<bool>) {
    if let Some(rest) = lower.strip_suffix("am") {
        return (rest, Some(false));
    }
    if let Some(rest) = lower.strip_suffix("pm") {
        return (rest, Some(true));
    }
    (lower, None)
}

fn strip_words(words: &[&str], start: usize, end: usize) -> String {
    words
        .iter()
        .enumerate()
        .filter(|(index, _)| *index < start || *index >= end)
        .map(|(_, word)| *word)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }

    fn due_str(input: &str, today: NaiveDate) -> Option<String> {
        parse(input, today).due.map(|value| value.to_string())
    }

    #[test]
    fn parses_the_documented_example() {
        let today = date(2026, 11, 19);
        let draft = parse("Finish PCC next wed 3pm #work p1", today);

        assert_eq!(draft.title, "Finish PCC");
        assert_eq!(
            draft.due.map(|value| value.to_string()).as_deref(),
            Some("2026-11-25T15:00:00")
        );
        assert_eq!(draft.tags, vec!["work"]);
        assert_eq!(draft.priority, Some(TaskPriority::Urgent));
    }

    #[test]
    fn next_weekday_is_strict_while_bare_and_this_include_today() {
        let today = date(2026, 11, 25);

        assert_eq!(due_str("Sync next wed", today).as_deref(), Some("2026-12-02"));
        assert_eq!(due_str("Sync this wed", today).as_deref(), Some("2026-11-25"));
        assert_eq!(due_str("Sync wed", today).as_deref(), Some("2026-11-25"));
    }

    #[test]
    fn supports_full_and_abbreviated_weekday_names() {
        let today = date(2026, 11, 19);

        for token in ["mon", "monday"] {
            assert_eq!(
                due_str(&format!("Plan {token}"), today).as_deref(),
                Some("2026-11-23")
            );
        }
        for token in ["tue", "tues", "tuesday"] {
            assert_eq!(
                due_str(&format!("Plan {token}"), today).as_deref(),
                Some("2026-11-24")
            );
        }
        for token in ["thu", "thur", "thurs", "thursday"] {
            assert_eq!(
                due_str(&format!("Plan {token}"), today).as_deref(),
                Some("2026-11-19")
            );
        }
    }

    #[test]
    fn parses_today_tomorrow_and_times() {
        let today = date(2026, 11, 19);

        assert_eq!(
            due_str("Write report tomorrow", today).as_deref(),
            Some("2026-11-20")
        );
        assert_eq!(
            due_str("Standup tomorrow at 9:30am", today).as_deref(),
            Some("2026-11-20T09:30:00")
        );
        assert_eq!(
            due_str("Review today 15:30", today).as_deref(),
            Some("2026-11-19T15:30:00")
        );
    }

    #[test]
    fn time_without_a_date_uses_today() {
        let today = date(2026, 11, 19);

        let draft = parse("Call mom at 3pm", today);
        assert_eq!(draft.title, "Call mom");
        assert_eq!(
            draft.due.map(|value| value.to_string()).as_deref(),
            Some("2026-11-19T15:00:00")
        );

        let draft = parse("Call mom 12:05", today);
        assert_eq!(draft.title, "Call mom");
        assert_eq!(
            draft.due.map(|value| value.to_string()).as_deref(),
            Some("2026-11-19T12:05:00")
        );
    }

    #[test]
    fn bare_numbers_and_invalid_times_stay_in_the_title() {
        let today = date(2026, 11, 19);

        for input in ["Buy 3 apples", "Meet at 25:00", "Meet at 13pm"] {
            let draft = parse(input, today);
            assert_eq!(draft.title, input, "{input} must not be consumed");
            assert_eq!(draft.due, None);
        }
    }

    #[test]
    fn unrelated_words_do_not_invent_a_date() {
        let today = date(2026, 11, 19);
        let draft = parse("Wedding planning notes", today);

        assert_eq!(draft.title, "Wedding planning notes");
        assert_eq!(draft.due, None);
    }

    #[test]
    fn invalid_sigil_forms_stay_in_the_title() {
        let today = date(2026, 11, 19);

        for input in ["Buy milk +", "Buy milk p5", "Buy milk p0", "Buy milk !"] {
            let draft = parse(input, today);
            assert_eq!(draft.title, input, "{input} must not be consumed");
            assert_eq!(draft.list_name, None);
            assert_eq!(draft.priority, None);
        }
    }

    #[test]
    fn keeps_estimate_and_list_from_sigils() {
        let today = date(2026, 11, 19);
        let draft = parse("Write spec ^45m +errands", today);

        assert_eq!(draft.title, "Write spec");
        assert_eq!(draft.estimate_minutes, Some(45));
        assert_eq!(draft.list_name.as_deref(), Some("errands"));
        assert_eq!(draft.due, None);
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn doesnt_panic_on_any_input(ref input in ".*") {
            let today = date(2026, 11, 19);
            let _ = parse(input, today);
        }
    }
}
