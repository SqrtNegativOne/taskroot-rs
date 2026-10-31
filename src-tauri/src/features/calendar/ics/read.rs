//! Resource body → [`AppEvent`]/[`AppTask`] projection (plan §7: read with
//! `icalendar`, never write with it).
//!
//! # Canonical value formats
//!
//! Every string this module produces is what the writer consumes, so a read/write
//! cycle is a no-op:
//!
//! - **All-day** event dates: floating `YYYY-MM-DD` (`2026-10-17`).
//! - **Timed** event wall clock: `YYYY-MM-DDTHH:MM:SS` with no offset, i.e. the
//!   local value exactly as the resource wrote it, paired with
//!   [`AppEvent::timezone`] = the `TZID` from `DTSTART` (IANA). A `DTSTART:...Z`
//!   resource has `timezone = None` and its wall clock is UTC.
//! - **`exdates`**: `YYYY-MM-DD` for all-day events and `YYYY-MM-DDTHH:MM:SS`
//!   wall clock in the event's zone for timed events — one entry per `EXDATE`
//!   value, comma-separated lists flattened across all `EXDATE` lines.
//! - **`original_start_time`** / `RECURRENCE-ID`: the same two shapes. An
//!   exception's `id` is `<UID>@<original_start_time>`, e.g.
//!   `dsh-abc@2026-10-19T11:00:00` or `dsh-abc@2026-10-19`.
//! - **`updated_at`** / `added`: ISO 8601 (`2026-10-19T06:49:00Z`).
//! - **`due`** / **`start`** (VTODO): `YYYY-MM-DD` for a `DUE;VALUE=DATE`, otherwise
//!   `YYYY-MM-DDTHH:MM:SS`; the date-only/timed distinction is preserved. Both are the
//!   wall clock of [`AppTask::timezone`], which the reader takes from `DUE` and then
//!   `DTSTART`. A `…Z` value keeps its UTC wall clock and sets `timezone = "UTC"`, so
//!   the writer can re-emit the same instant instead of floating it.
//! - **`completed_at`**: ISO 8601 UTC, the `COMPLETED` timestamp.
//! - **`rrule`** (VTODO): the wire form (`RRULE:FREQ=…`), one line per rule.
//! - **`geo`**: `GEO` as a latitude/longitude pair.
//! - **`alarms`**: one entry per `VALARM`, in document order. `minutes_before` is
//!   positive for "before the anchor", and an absolute trigger keeps its UTC instant.
//!   Alarms this module cannot express are omitted rather than approximated.

use crate::features::calendar::domain::{
    AppEvent, AppTask, AppTaskStatus, CollectionId, Color, EventId, EventStatus, GeoPoint,
    RemoteId, Tag, TaskAlarmAnchor, TaskAlarmTrigger, TaskId, TaskPriority,
};
use crate::error::AppError;
use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz as ChronoTz;
use icalendar::parser::{read_calendar, unfold, Component, Property};
use std::str::FromStr;

const ICAL_DATE: &str = "%Y%m%d";
const ICAL_DATE_TIME: &str = "%Y%m%dT%H%M%S";
const DTO_DATE: &str = "%Y-%m-%d";
const DTO_DATE_TIME: &str = "%Y-%m-%dT%H:%M:%S";
const DTO_ISO: &str = "%Y-%m-%dT%H:%M:%SZ";

/// Map every `VEVENT` in a resource body, in file order.
///
/// # Errors
/// Returns [`AppError::InvalidInput`] for an unparseable body or a `VEVENT`
/// without a `UID`/`DTSTART`.
pub fn read_event_resource(
    raw: &str,
    href: &str,
    collection_href: &str,
) -> Result<Vec<AppEvent>, AppError> {
    let mut events = events(raw)?;
    for event in &mut events {
        event.remote_id = Some(RemoteId::from(href));
        event.remote_collection_id = Some(CollectionId::from(collection_href));
    }
    Ok(events)
}

/// Map every `VTODO` in a resource body, in file order.
///
/// # Errors
/// Returns [`AppError::InvalidInput`] for an unparseable body or a `VTODO`
/// without a `UID`.
pub fn read_task_resource(
    raw: &str,
    href: &str,
    collection_href: &str,
) -> Result<Vec<AppTask>, AppError> {
    let mut tasks = tasks(raw)?;
    for task in &mut tasks {
        task.remote_id = Some(RemoteId::from(href));
        task.remote_collection_id = Some(CollectionId::from(collection_href));
    }
    Ok(tasks)
}

/// Resource-body projection without transport identity; used by the writer to
/// locate the component an [`AppEvent`] came from.
pub fn events(raw: &str) -> Result<Vec<AppEvent>, AppError> {
    let unfolded = unfold(raw);
    let calendar = parse(&unfolded, "event resource")?;
    calendar
        .components
        .iter()
        .filter(|component| component.name.as_str() == "VEVENT")
        .map(event_from_component)
        .collect()
}

pub fn tasks(raw: &str) -> Result<Vec<AppTask>, AppError> {
    let unfolded = unfold(raw);
    let calendar = parse(&unfolded, "task resource")?;
    calendar
        .components
        .iter()
        .filter(|component| component.name.as_str() == "VTODO")
        .map(task_from_component)
        .collect()
}

fn parse<'a>(unfolded: &'a str, source: &str) -> Result<icalendar::parser::Calendar<'a>, AppError> {
    read_calendar(unfolded).map_err(|error| AppError::InvalidInput(format!("{source}: {error}")))
}

/// One `DATE` or `DATE-TIME` value as written in the resource.
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
    const fn is_date(&self) -> bool {
        matches!(self, Self::Date(_))
    }

    fn tzid(&self) -> Option<&str> {
        match self {
            Self::Date(_) => None,
            Self::Timed { tzid, .. } => tzid.as_deref(),
        }
    }
}

/// Parse a `DATE`/`DATE-TIME` property, honouring `VALUE=DATE` and `TZID`.
pub fn parse_date_value(property: &Property<'_>) -> Option<DateValue> {
    let value_type = parameter(property, "VALUE");
    parse_date_text(
        property.val.as_str(),
        value_type,
        parameter(property, "TZID"),
    )
}

/// Parse one date/date-time scalar with the parameters that were attached to it.
pub fn parse_date_text(
    raw: &str,
    value_type: Option<&str>,
    tzid: Option<&str>,
) -> Option<DateValue> {
    let date_shaped =
        value_type.is_some_and(|kind| kind.eq_ignore_ascii_case("DATE")) || raw.len() == 8;
    if date_shaped {
        return NaiveDate::parse_from_str(raw, ICAL_DATE)
            .ok()
            .map(DateValue::Date);
    }
    let utc = raw.ends_with('Z');
    let stamp = raw.strip_suffix('Z').unwrap_or(raw);
    let naive = NaiveDateTime::parse_from_str(stamp, ICAL_DATE_TIME).ok()?;
    Some(DateValue::Timed {
        naive,
        tzid: tzid.map(ToString::to_string),
        utc,
    })
}

/// The wall clock a resource value denotes, in `zone` for UTC-suffixed values.
pub fn wall_clock(value: &DateValue, zone: Option<&str>) -> Option<NaiveDateTime> {
    match value {
        DateValue::Date(_) => None,
        DateValue::Timed { naive, utc, .. } => {
            if *utc {
                Some(utc_naive_to_zone(*naive, zone))
            } else {
                Some(*naive)
            }
        }
    }
}

/// The canonical DTO string for a resource value: `YYYY-MM-DD` or a wall clock.
pub fn canonical(value: &DateValue, zone: Option<&str>) -> Option<String> {
    match value {
        DateValue::Date(date) => Some(date.format(DTO_DATE).to_string()),
        DateValue::Timed { .. } => {
            wall_clock(value, zone).map(|naive| naive.format(DTO_DATE_TIME).to_string())
        }
    }
}

fn utc_naive_to_zone(naive: NaiveDateTime, zone: Option<&str>) -> NaiveDateTime {
    let Some(tz) = zone.and_then(|name| ChronoTz::from_str(name).ok()) else {
        return naive;
    };
    Utc.from_utc_datetime(&naive)
        .with_timezone(&tz)
        .naive_local()
}

fn event_from_component(component: &Component<'_>) -> Result<AppEvent, AppError> {
    let uid = property(component, "UID")
        .map(|property| property.val.as_str().to_string())
        .ok_or_else(|| AppError::InvalidInput("VEVENT without UID".to_string()))?;
    let start = property(component, "DTSTART")
        .and_then(parse_date_value)
        .ok_or_else(|| AppError::InvalidInput(format!("VEVENT {uid} has no parseable DTSTART")))?;
    let timezone = start.tzid().map(ToString::to_string);
    let is_all_day = start.is_date();
    let start_time = canonical(&start, timezone.as_deref())
        .ok_or_else(|| AppError::InvalidInput(format!("VEVENT {uid} has an invalid DTSTART")))?;
    let end_time = property(component, "DTEND")
        .and_then(parse_date_value)
        .and_then(|end| canonical(&end, timezone.as_deref()))
        .unwrap_or_else(|| default_end(&start, &start_time, is_all_day));

    let recurrence = property(component, "RECURRENCE-ID").and_then(parse_date_value);
    let original_start_time = recurrence.and_then(|value| canonical(&value, timezone.as_deref()));
    let id = original_start_time
        .as_ref()
        .map_or_else(|| uid.clone(), |original| format!("{uid}@{original}"));
    let recurring_event_id = original_start_time
        .as_ref()
        .map(|_| EventId::from(uid.clone()));

    Ok(AppEvent {
        id: EventId::from(id),
        remote_id: None,
        remote_collection_id: None,
        task_id: text_property(component, "X-TASKROOT-TASK-ID").map(TaskId::from),
        title: text_property(component, "SUMMARY").unwrap_or_default(),
        description: text_property(component, "DESCRIPTION"),
        start_time,
        end_time,
        rrule: joined_property(component, "RRULE"),
        exdates: read_exdates(component, timezone.as_deref(), is_all_day),
        recurring_event_id,
        original_start_time,
        status: property(component, "STATUS").and_then(event_status),
        updated_at: property(component, "LAST-MODIFIED").and_then(iso_value),
        color: text_property(component, "COLOR").and_then(|value| Color::try_from(value).ok()),
        etag: None,
        is_all_day: Some(is_all_day),
        timezone,
    })
}

fn default_end(start: &DateValue, start_time: &str, is_all_day: bool) -> String {
    if is_all_day {
        if let DateValue::Date(date) = start {
            if let Some(next) = date.succ_opt() {
                return next.format(DTO_DATE).to_string();
            }
        }
    }
    start_time.to_string()
}

fn read_exdates(
    component: &Component<'_>,
    timezone: Option<&str>,
    is_all_day: bool,
) -> Option<Vec<String>> {
    let mut values = Vec::new();
    for property in properties(component, "EXDATE") {
        let value_type = parameter(property, "VALUE");
        let tzid = parameter(property, "TZID");
        for raw in property.val.as_str().split(',') {
            let Some(parsed) = parse_date_text(raw.trim(), value_type, tzid) else {
                continue;
            };
            let canonical = if is_all_day {
                all_day_exdate(&parsed, timezone)
            } else {
                canonical(&parsed, timezone)
            };
            if let Some(value) = canonical {
                values.push(value);
            }
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn all_day_exdate(value: &DateValue, zone: Option<&str>) -> Option<String> {
    match value {
        DateValue::Date(date) => Some(date.format(DTO_DATE).to_string()),
        DateValue::Timed { .. } => {
            wall_clock(value, zone).map(|naive| naive.date().format(DTO_DATE).to_string())
        }
    }
}

fn task_from_component(component: &Component<'_>) -> Result<AppTask, AppError> {
    let uid = property(component, "UID")
        .map(|property| property.val.as_str().to_string())
        .ok_or_else(|| AppError::InvalidInput("VTODO without UID".to_string()))?;
    let created = property(component, "CREATED").and_then(iso_value);
    let stamp = property(component, "DTSTAMP").and_then(iso_value);
    let due = property(component, "DUE").and_then(parse_date_value);
    let start = property(component, "DTSTART").and_then(parse_date_value);
    // One zone governs both lines, as tasks.org writes them: `DUE` wins because
    // it is the field every task has in the UI.
    let timezone = due
        .as_ref()
        .and_then(value_timezone)
        .or_else(|| start.as_ref().and_then(value_timezone));

    Ok(AppTask {
        id: TaskId::from(uid),
        remote_id: None,
        remote_collection_id: None,
        title: text_property(component, "SUMMARY").unwrap_or_default(),
        status: Some(read_task_status(component)),
        priority: property(component, "PRIORITY").and_then(priority_value),
        tags: read_categories(component),
        parent_task: read_parent(component).map(TaskId::from),
        dependencies: read_dependencies(component),
        est: property(component, "X-TASKROOT-EST")
            .and_then(|property| property.val.as_str().parse().ok()),
        added: created.or(stamp),
        canvas_x: number_property(component, "X-TASKROOT-CANVAS-X"),
        canvas_y: number_property(component, "X-TASKROOT-CANVAS-Y"),
        on_canvas: text_property(component, "X-TASKROOT-ON-CANVAS").map(|value| {
            value.eq_ignore_ascii_case("true") || value == "1" || value.eq_ignore_ascii_case("yes")
        }),
        notes: text_property(component, "DESCRIPTION"),
        tabs: text_property(component, "X-TASKROOT-TABS"),
        start: start
            .as_ref()
            .and_then(|value| canonical(value, timezone.as_deref())),
        due: due
            .as_ref()
            .and_then(|value| canonical(value, timezone.as_deref())),
        completed_at: property(component, "COMPLETED").and_then(iso_value),
        rrule: joined_property(component, "RRULE"),
        geo: read_geo(component),
        alarms: read_alarms(component),
        timezone,
        updated_at: property(component, "LAST-MODIFIED").and_then(iso_value),
        etag: None,
    })
}

fn read_task_status(component: &Component<'_>) -> AppTaskStatus {
    if property(component, "STATUS")
        .is_some_and(|property| property.val.as_str().eq_ignore_ascii_case("COMPLETED"))
    {
        return AppTaskStatus::Done;
    }
    match text_property(component, "X-TASKROOT-STATUS").as_deref() {
        Some("next-up") => AppTaskStatus::NextUp,
        Some("doing") => AppTaskStatus::Doing,
        Some("done") => AppTaskStatus::Done,
        _ => AppTaskStatus::Todo,
    }
}

fn priority_value(property: &Property<'_>) -> Option<TaskPriority> {
    match property.val.as_str().parse::<u32>().ok()? {
        1..=2 => Some(TaskPriority::Urgent),
        3..=4 => Some(TaskPriority::High),
        5 => Some(TaskPriority::Medium),
        6..=9 => Some(TaskPriority::Low),
        _ => None,
    }
}

fn read_categories(component: &Component<'_>) -> Option<Vec<Tag>> {
    let mut tags = Vec::new();
    for property in properties(component, "CATEGORIES") {
        for name in property.val.as_str().split(',') {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            tags.push(Tag {
                id: name.to_string(),
                name: name.to_string(),
                color: None,
            });
        }
    }
    if tags.is_empty() {
        None
    } else {
        Some(tags)
    }
}

fn read_parent(component: &Component<'_>) -> Option<String> {
    properties(component, "RELATED-TO")
        .find(|property| relation_is_parent(property))
        .map(|property| property.val.as_str().to_string())
}

fn relation_is_parent(property: &Property<'_>) -> bool {
    parameter(property, "RELTYPE").is_none_or(|kind| kind.eq_ignore_ascii_case("PARENT"))
}

fn read_dependencies(component: &Component<'_>) -> Option<Vec<TaskId>> {
    let raw = text_property(component, "X-TASKROOT-DEPENDENCIES")?;
    let ids: Vec<String> = serde_json::from_str(&raw).ok()?;
    if ids.is_empty() {
        return None;
    }
    Some(ids.into_iter().map(TaskId::from).collect())
}

/// The zone a `DTSTART`/`DUE` value is expressed in: its `TZID`, or `"UTC"` for a
/// `…Z` value so the writer can keep the instant instead of floating it.
fn value_timezone(value: &DateValue) -> Option<String> {
    match value {
        DateValue::Date(_) => None,
        DateValue::Timed { tzid, utc, .. } => {
            tzid.clone().or_else(|| utc.then(|| "UTC".to_string()))
        }
    }
}

fn read_geo(component: &Component<'_>) -> Option<GeoPoint> {
    let raw = text_property(component, "GEO")?;
    let (latitude, longitude) = raw.split_once(';')?;
    Some(GeoPoint {
        latitude: latitude.trim().parse().ok()?,
        longitude: longitude.trim().parse().ok()?,
    })
}

/// The `VALARM` triggers of a task, in document order.
///
/// Alarms this module cannot express are left out; the writer only rewrites the
/// alarm list when the DTO's list actually changed, so they survive otherwise.
fn read_alarms(component: &Component<'_>) -> Option<Vec<TaskAlarmTrigger>> {
    let alarms = component
        .components
        .iter()
        .filter(|child| child.name.as_str() == "VALARM")
        .filter_map(|child| property(child, "TRIGGER"))
        .filter_map(alarm_trigger)
        .collect::<Vec<_>>();
    if alarms.is_empty() {
        return None;
    }
    Some(alarms)
}

fn alarm_trigger(trigger: &Property<'_>) -> Option<TaskAlarmTrigger> {
    let raw = trigger.val.as_str().trim();
    let absolute = parameter(trigger, "VALUE")
        .is_some_and(|value| value.eq_ignore_ascii_case("DATE-TIME"))
        || is_date_time(raw);
    if absolute {
        let at = absolute_trigger(raw)?;
        // tasks.org writes this instant for "the reminder was cleared".
        if at == "1976-04-01T00:54:45Z" {
            return None;
        }
        return Some(TaskAlarmTrigger::Absolute { at });
    }
    let anchor =
        if parameter(trigger, "RELATED").is_some_and(|value| value.eq_ignore_ascii_case("END")) {
            TaskAlarmAnchor::End
        } else {
            TaskAlarmAnchor::Start
        };
    // The DTO counts minutes *before* the anchor, the wire carries a signed
    // duration: `-PT15M` is 15 minutes before.
    let minutes_before = duration_minutes(raw)?.checked_neg()?;
    Some(TaskAlarmTrigger::Relative {
        minutes_before,
        anchor,
    })
}

fn is_date_time(raw: &str) -> bool {
    let stamp = raw.strip_suffix('Z').unwrap_or(raw);
    NaiveDateTime::parse_from_str(stamp, ICAL_DATE_TIME).is_ok()
}

fn absolute_trigger(raw: &str) -> Option<String> {
    let naive = NaiveDateTime::parse_from_str(raw.strip_suffix('Z')?, ICAL_DATE_TIME).ok()?;
    Some(naive.format(DTO_ISO).to_string())
}

/// Whole minutes in an ISO 8601 duration such as `-PT15M` or `P1D`, signed as
/// written. A duration that is not a whole number of minutes (`-PT30S`) is not
/// representable here and is therefore not modelled.
fn duration_minutes(raw: &str) -> Option<i32> {
    let (sign, rest) = raw.strip_prefix('-').map_or_else(
        || (1, raw.strip_prefix('+').unwrap_or(raw)),
        |rest| (-1, rest),
    );
    let rest = rest.strip_prefix('P')?;
    let mut total = 0_i32;
    let mut number = String::new();
    let mut in_time = false;
    for character in rest.chars() {
        match character {
            'T' => in_time = true,
            '0'..='9' => number.push(character),
            unit => {
                let value: i32 = number.parse().ok()?;
                number.clear();
                let minutes = match (unit, in_time) {
                    ('W', false) => value.checked_mul(7 * 24 * 60)?,
                    ('D', false) => value.checked_mul(24 * 60)?,
                    ('H', true) => value.checked_mul(60)?,
                    ('M', true) => value,
                    _ => return None,
                };
                total = total.checked_add(minutes)?;
            }
        }
    }
    if !number.is_empty() {
        return None;
    }
    total.checked_mul(sign)
}

fn event_status(property: &Property<'_>) -> Option<EventStatus> {
    match property.val.as_str().to_ascii_uppercase().as_str() {
        "CONFIRMED" => Some(EventStatus::Confirmed),
        "TENTATIVE" => Some(EventStatus::Tentative),
        "CANCELLED" => Some(EventStatus::Cancelled),
        _ => None,
    }
}

fn iso_value(property: &Property<'_>) -> Option<String> {
    match parse_date_value(property)? {
        DateValue::Date(date) => Some(date.format(DTO_DATE).to_string()),
        DateValue::Timed { naive, utc, .. } => {
            if utc {
                Some(naive.format(DTO_ISO).to_string())
            } else {
                Some(naive.format(DTO_DATE_TIME).to_string())
            }
        }
    }
}

fn number_property(component: &Component<'_>, name: &str) -> Option<f64> {
    text_property(component, name).and_then(|value| value.parse().ok())
}

fn property<'a>(component: &'a Component<'a>, name: &str) -> Option<&'a Property<'a>> {
    component
        .properties
        .iter()
        .find(|property| property.name.as_str() == name)
}

fn properties<'a>(
    component: &'a Component<'a>,
    name: &'a str,
) -> impl Iterator<Item = &'a Property<'a>> {
    component
        .properties
        .iter()
        .filter(move |property| property.name.as_str() == name)
}

fn text_property(component: &Component<'_>, name: &str) -> Option<String> {
    property(component, name).map(|property| property.val.as_str().to_string())
}

fn joined_property(component: &Component<'_>, name: &str) -> Option<String> {
    let values = properties(component, name)
        .map(|property| {
            let value = property.val.as_str().trim();
            // `RRULE` values are bare (`FREQ=...`); the DTO keeps the wire form
            // (`RRULE:FREQ=...`) that the Google write path and the writer use.
            let prefixed = format!("{name}:{value}");
            if value.to_ascii_uppercase().starts_with(&format!("{name}:")) {
                value.to_string()
            } else {
                prefixed
            }
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.join("\n"))
    }
}

fn parameter<'a>(property: &'a Property<'a>, key: &str) -> Option<&'a str> {
    property
        .params
        .iter()
        .find(|param| param.key.as_str() == key)
        .and_then(|param| param.val.as_ref())
        .map(icalendar::parser::ParseString::as_str)
}
