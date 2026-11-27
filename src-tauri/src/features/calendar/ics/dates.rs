//! Date, date-time and ISO-8601 duration conversions for iCalendar properties.

use crate::features::calendar::domain::{
    utc_to_wall_clock, CalendarDateTime, ICAL_DATE, ICAL_DATE_TIME, ICAL_UTC,
};
use crate::features::calendar::ics::cst::encode;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use ical::param::IcalParam;
use ical::prop::IcalPropKind;
use ical::tree::line::IcalLine;
use ical::tree::param::tzid::TZID;
use ical::tree::param::value::VALUE;
use ical::value::datetime::{IcalDate, IcalDateTime};
use ical::value::duration::IcalDuration;
use ical::value::IcalValue;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateValue {
    Date(NaiveDate),
    Timed {
        naive: NaiveDateTime,
        tzid: Option<String>,
        utc: bool,
    },
}

impl DateValue {
    pub(super) const fn is_date(&self) -> bool {
        matches!(self, Self::Date(_))
    }

    pub(super) fn tzid(&self) -> Option<&str> {
        match self {
            Self::Date(_) => None,
            Self::Timed { tzid, .. } => tzid.as_deref(),
        }
    }

    pub(super) fn task_timezone(&self) -> Option<String> {
        match self {
            Self::Date(_) => None,
            Self::Timed { tzid, utc, .. } => {
                tzid.clone().or_else(|| utc.then(|| "UTC".to_string()))
            }
        }
    }

    pub(super) fn wall_clock(&self, zone: Option<&str>) -> Option<NaiveDateTime> {
        match self {
            Self::Date(_) => None,
            Self::Timed { naive, utc, .. } => Some(if *utc {
                utc_to_wall_clock(naive.and_utc(), zone)
            } else {
                *naive
            }),
        }
    }

    pub(super) fn canonical(&self, zone: Option<&str>) -> CalendarDateTime {
        match self {
            Self::Date(date) => CalendarDateTime::Date(*date),
            Self::Timed { .. } => CalendarDateTime::WallClock(
                self.wall_clock(zone).unwrap_or_else(|| match self {
                    Self::Timed { naive, .. } => *naive,
                    Self::Date(date) => date.and_time(chrono::NaiveTime::MIN),
                }),
            ),
        }
    }

    pub(super) fn all_day_exdate(&self, zone: Option<&str>) -> CalendarDateTime {
        match self {
            Self::Date(date) => CalendarDateTime::Date(*date),
            Self::Timed { .. } => CalendarDateTime::Date(self.canonical(zone).date()),
        }
    }
}

pub(super) fn parse_date_line(line: &IcalLine<'_>) -> Option<DateValue> {
    parse_date_text(
        line.raw_value_str().trim(),
        line.param::<VALUE>().as_deref(),
        line.param::<TZID>().as_deref(),
    )
}

pub(super) fn parse_date_text(
    raw: &str,
    value_type: Option<&str>,
    tzid: Option<&str>,
) -> Option<DateValue> {
    if value_type.is_some_and(|kind| kind.eq_ignore_ascii_case("DATE")) || raw.len() == 8 {
        return NaiveDate::parse_from_str(raw, ICAL_DATE)
            .ok()
            .map(DateValue::Date);
    }
    let utc = raw.ends_with('Z');
    let naive = NaiveDateTime::parse_from_str(raw.trim_suffix('Z'), ICAL_DATE_TIME).ok()?;
    Some(DateValue::Timed {
        naive,
        tzid: tzid.map(ToString::to_string),
        utc,
    })
}

pub(super) fn stamp_from_line(line: &IcalLine<'_>) -> Option<DateTime<Utc>> {
    match parse_date_line(line)? {
        DateValue::Date(date) => date.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc()),
        DateValue::Timed { naive, .. } => Some(naive.and_utc()),
    }
}

pub(super) fn is_date_time(raw: &str) -> bool {
    NaiveDateTime::parse_from_str(raw.trim_suffix('Z'), ICAL_DATE_TIME).is_ok()
}

pub(super) fn absolute_trigger(raw: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(raw.strip_suffix('Z')?, ICAL_DATE_TIME)
        .ok()
        .map(|naive| naive.and_utc())
}

/// Whole minutes in an ISO-8601 duration via `ical-rs`; non-whole-minute
/// values (such as `-PT45S`) return `None` so unmodelled triggers survive.
pub(super) fn duration_whole_minutes(raw: &str) -> Option<i32> {
    let trimmed = raw.trim();
    let valid_end = trimmed
        .bytes()
        .last()
        .is_some_and(|byte| byte.is_ascii_alphabetic())
        && !trimmed.ends_with(['P', 'p', 'T', 't']);
    if !valid_end {
        return None;
    }
    let seconds = IcalDuration::from(trimmed).seconds()?;
    (seconds.checked_rem(60)? == 0)
        .then(|| i32::try_from(seconds.checked_div(60)?).ok())
        .flatten()
}

pub(super) fn date_line(
    kind: IcalPropKind,
    value: CalendarDateTime,
    all_day: bool,
    zone: Option<&str>,
) -> Option<IcalLine<'static>> {
    if all_day {
        return Some(encode(
            kind,
            vec![IcalParam::Value(Cow::Borrowed("DATE"))],
            IcalValue::Date(IcalDate(Cow::Owned(
                value.date().format(ICAL_DATE).to_string(),
            ))),
        ));
    }
    let naive = value.to_wall_clock(zone)?;
    let params = zone.map_or_default(|tz| vec![IcalParam::TzId(Cow::Borrowed(tz))]);
    Some(encode(
        kind,
        params,
        IcalValue::DateTime(IcalDateTime(Cow::Owned(
            naive.format(ICAL_DATE_TIME).to_string(),
        ))),
    ))
}

pub(super) fn task_date_line(
    kind: IcalPropKind,
    value: Option<CalendarDateTime>,
    zone: Option<&str>,
) -> Option<IcalLine<'static>> {
    let value = value?;
    date_line(kind, value, value.is_date(), zone)
}

pub(super) fn stamp_line(kind: IcalPropKind, value: DateTime<Utc>) -> IcalLine<'static> {
    encode(
        kind,
        Vec::new(),
        IcalValue::DateTime(IcalDateTime(Cow::Owned(
            value.format(ICAL_UTC).to_string(),
        ))),
    )
}
