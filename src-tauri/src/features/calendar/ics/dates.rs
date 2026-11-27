//! Date, date-time and ISO-8601 duration conversions for iCalendar properties.

use crate::features::calendar::ics::cst::encode;
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz as ChronoTz;
use ical::param::IcalParam;
use ical::prop::IcalPropKind;
use ical::tree::line::IcalLine;
use ical::tree::param::tzid::TZID;
use ical::tree::param::value::VALUE;
use ical::value::IcalValue;
use ical::value::datetime::{IcalDate, IcalDateTime};
use ical::value::duration::IcalDuration;
use std::borrow::Cow;
use std::str::FromStr;

pub(super) const ICAL_DATE: &str = "%Y%m%d";
pub(super) const ICAL_DATE_TIME: &str = "%Y%m%dT%H%M%S";
pub(super) const ICAL_UTC: &str = "%Y%m%dT%H%M%SZ";
pub(super) const DTO_DATE: &str = "%Y-%m-%d";
pub(super) const DTO_DATE_LEN: usize = "YYYY-MM-DD".len();
pub(super) const DTO_DATE_TIME: &str = "%Y-%m-%dT%H:%M:%S";
pub(super) const DTO_ISO: &str = "%Y-%m-%dT%H:%M:%SZ";

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

pub(super) fn wall_clock(value: &DateValue, zone: Option<&str>) -> Option<NaiveDateTime> {
    match value {
        DateValue::Date(_) => None,
        DateValue::Timed { naive, utc, .. } => Some(if *utc {
            utc_naive_to_zone(*naive, zone)
        } else {
            *naive
        }),
    }
}

pub(super) fn canonical(value: &DateValue, zone: Option<&str>) -> Option<String> {
    match value {
        DateValue::Date(date) => Some(date.format(DTO_DATE).to_string()),
        DateValue::Timed { .. } => {
            wall_clock(value, zone).map(|naive| naive.format(DTO_DATE_TIME).to_string())
        }
    }
}

pub(super) fn all_day_exdate(value: &DateValue, zone: Option<&str>) -> Option<String> {
    match value {
        DateValue::Date(date) => Some(date.format(DTO_DATE).to_string()),
        DateValue::Timed { .. } => {
            wall_clock(value, zone).map(|naive| naive.date().format(DTO_DATE).to_string())
        }
    }
}

pub(super) fn iso_line(line: &IcalLine<'_>) -> Option<String> {
    match parse_date_line(line)? {
        DateValue::Date(date) => Some(date.format(DTO_DATE).to_string()),
        DateValue::Timed { naive, utc, .. } => Some(
            naive
                .format(if utc { DTO_ISO } else { DTO_DATE_TIME })
                .to_string(),
        ),
    }
}

pub(super) fn is_date_time(raw: &str) -> bool {
    NaiveDateTime::parse_from_str(raw.trim_suffix('Z'), ICAL_DATE_TIME).is_ok()
}

pub(super) fn absolute_trigger(raw: &str) -> Option<String> {
    let naive = NaiveDateTime::parse_from_str(raw.strip_suffix('Z')?, ICAL_DATE_TIME).ok()?;
    Some(naive.format(DTO_ISO).to_string())
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

pub(super) fn parse_datetime(value: &str) -> Option<NaiveDateTime> {
    if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
        return Some(datetime.with_timezone(&Utc).naive_utc());
    }
    NaiveDateTime::parse_from_str(value, DTO_DATE_TIME)
        .ok()
        .or_else(|| {
            NaiveDate::parse_from_str(value, DTO_DATE)
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
        })
}

pub(super) fn date_line(
    kind: IcalPropKind,
    value: &str,
    all_day: bool,
    zone: Option<&str>,
) -> Option<IcalLine<'static>> {
    if all_day {
        let date =
            NaiveDate::parse_from_str(value.get(..DTO_DATE_LEN).unwrap_or(value), DTO_DATE).ok()?;
        return Some(encode(
            kind,
            vec![IcalParam::Value(Cow::Borrowed("DATE"))],
            IcalValue::Date(IcalDate(Cow::Owned(date.format(ICAL_DATE).to_string()))),
        ));
    }
    let naive = write_wall_clock(value, zone)?;
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
    value: Option<&str>,
    zone: Option<&str>,
) -> Option<IcalLine<'static>> {
    let value = value?;
    date_line(
        kind,
        value,
        NaiveDate::parse_from_str(value, DTO_DATE).is_ok(),
        zone,
    )
}

pub(super) fn stamp_line(kind: IcalPropKind, value: &str) -> Option<IcalLine<'static>> {
    let datetime = parse_datetime(value)?;
    Some(encode(
        kind,
        Vec::new(),
        IcalValue::DateTime(IcalDateTime(Cow::Owned(
            datetime.format(ICAL_UTC).to_string(),
        ))),
    ))
}

fn write_wall_clock(value: &str, zone: Option<&str>) -> Option<NaiveDateTime> {
    if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
        let utc = datetime.with_timezone(&Utc);
        return Some(
            zone.and_then(|name| ChronoTz::from_str(name).ok())
                .map_or_else(
                    || utc.naive_utc(),
                    |timezone| utc.with_timezone(&timezone).naive_local(),
                ),
        );
    }
    NaiveDateTime::parse_from_str(value, DTO_DATE_TIME)
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S"))
        .ok()
}

fn utc_naive_to_zone(naive: NaiveDateTime, zone: Option<&str>) -> NaiveDateTime {
    let Some(tz) = zone.and_then(|name| ChronoTz::from_str(name).ok()) else {
        return naive;
    };
    Utc.from_utc_datetime(&naive)
        .with_timezone(&tz)
        .naive_local()
}
