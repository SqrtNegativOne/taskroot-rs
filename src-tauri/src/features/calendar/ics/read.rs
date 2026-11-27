//! Resource body → [`AppEvent`]/[`AppTask`] projection via `ical-rs`.
//!
//! Canonical value formats consumed symmetrically by the writer:
//! - All-day dates: `YYYY-MM-DD`; timed wall clocks: `YYYY-MM-DDTHH:MM:SS` paired
//!   with `timezone` (`TZID`, or `"UTC"` on VTODO `…Z` values).
//! - Exceptions: `id = <UID>@<original_start_time>`.
//! - Timestamps (`added`, `updated_at`, `completed_at`): ISO 8601 UTC.
//! - `rrule`: wire form (`RRULE:FREQ=…`), one line per rule.

use crate::error::AppError;
use crate::features::calendar::domain::{
    AppEvent, AppTask, CollectionId, Color, EventId, EventStatus, RemoteId, Tag, TaskId,
    TaskPriority,
};
use crate::features::calendar::ics::cst::{
    EVENT_KIND, TASK_KIND, child_components, parse_calendar, prop, props, text_prop,
};
use crate::features::calendar::ics::dates::{
    DTO_DATE, DateValue, all_day_exdate, canonical, iso_line, parse_date_line, parse_date_text,
};
use crate::features::calendar::ics::read_task::task_from_component;
use ical::tree::cst::IcalCst;
use ical::tree::line::IcalLine;
use ical::tree::param::tzid::TZID;
use ical::tree::param::value::VALUE;

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
    let calendar = parse_calendar(raw, "event resource")?;
    let mut events = child_components(&calendar, EVENT_KIND)
        .map(event_from_component)
        .collect::<Result<Vec<_>, _>>()?;
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
    let calendar = parse_calendar(raw, "task resource")?;
    let mut tasks = child_components(&calendar, TASK_KIND)
        .map(task_from_component)
        .collect::<Result<Vec<_>, _>>()?;
    for task in &mut tasks {
        task.remote_id = Some(RemoteId::from(href));
        task.remote_collection_id = Some(CollectionId::from(collection_href));
    }
    Ok(tasks)
}

pub(super) fn event_from_component(component: &IcalCst<'_>) -> Result<AppEvent, AppError> {
    let uid = text_prop(component, "UID")
        .ok_or_else(|| AppError::InvalidInput("VEVENT without UID".to_string()))?;
    let start = prop(component, "DTSTART")
        .and_then(parse_date_line)
        .ok_or_else(|| AppError::InvalidInput(format!("VEVENT {uid} has no parseable DTSTART")))?;
    let timezone = start.tzid().map(ToString::to_string);
    let is_all_day = start.is_date();
    let start_time = canonical(&start, timezone.as_deref())
        .ok_or_else(|| AppError::InvalidInput(format!("VEVENT {uid} has an invalid DTSTART")))?;
    let end_time = prop(component, "DTEND")
        .and_then(parse_date_line)
        .and_then(|end| canonical(&end, timezone.as_deref()))
        .unwrap_or_else(|| default_end(&start, &start_time, is_all_day));

    let recurrence = prop(component, "RECURRENCE-ID").and_then(parse_date_line);
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
        task_id: text_prop(component, "X-TASKROOT-TASK-ID").map(TaskId::from),
        title: text_prop(component, "SUMMARY").unwrap_or_default(),
        description: text_prop(component, "DESCRIPTION"),
        priority: prop(component, "PRIORITY").and_then(priority_value),
        tags: read_categories(component),
        start_time,
        end_time,
        rrule: joined_rrule(component),
        exdates: read_exdates(component, timezone.as_deref(), is_all_day),
        recurring_event_id,
        original_start_time,
        status: prop(component, "STATUS").and_then(event_status),
        updated_at: prop(component, "LAST-MODIFIED").and_then(iso_line),
        added: prop(component, "CREATED")
            .and_then(iso_line)
            .or_else(|| prop(component, "DTSTAMP").and_then(iso_line)),
        color: text_prop(component, "COLOR").and_then(|value| Color::try_from(value).ok()),
        etag: None,
        is_all_day: Some(is_all_day),
        timezone,
    })
}

pub(super) fn priority_value(line: &IcalLine<'_>) -> Option<TaskPriority> {
    match line.raw_value_str().trim().parse::<u32>().ok()? {
        1..=2 => Some(TaskPriority::Urgent),
        3..=4 => Some(TaskPriority::High),
        5 => Some(TaskPriority::Medium),
        6..=9 => Some(TaskPriority::Low),
        _ => None,
    }
}

pub(super) fn read_categories(component: &IcalCst<'_>) -> Option<Vec<Tag>> {
    let mut tags = Vec::new();
    for line in props(component, "CATEGORIES") {
        for raw in line.value.decode_list() {
            let name = raw.trim();
            if !name.is_empty() {
                tags.push(Tag {
                    id: name.to_string(),
                    name: name.to_string(),
                    color: None,
                });
            }
        }
    }
    (!tags.is_empty()).then_some(tags)
}

pub(super) fn joined_rrule(component: &IcalCst<'_>) -> Option<String> {
    let values: Vec<String> = props(component, "RRULE")
        .filter_map(|line| {
            let raw = line.raw_value_str();
            let value = raw.trim();
            if value.is_empty() {
                return None;
            }
            if value
                .get(..6)
                .is_some_and(|head| head.eq_ignore_ascii_case("RRULE:"))
            {
                Some(value.to_string())
            } else {
                Some(format!("RRULE:{value}"))
            }
        })
        .collect();
    (!values.is_empty()).then(|| values.join("\n"))
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
    component: &IcalCst<'_>,
    timezone: Option<&str>,
    is_all_day: bool,
) -> Option<Vec<String>> {
    let mut values = Vec::new();
    for line in props(component, "EXDATE") {
        let value_type = line.param::<VALUE>();
        let tzid = line.param::<TZID>();
        for raw in line.value.decode_list() {
            let Some(parsed) = parse_date_text(raw.trim(), value_type.as_deref(), tzid.as_deref())
            else {
                continue;
            };
            let formatted = if is_all_day {
                all_day_exdate(&parsed, timezone)
            } else {
                canonical(&parsed, timezone)
            };
            if let Some(value) = formatted {
                values.push(value);
            }
        }
    }
    (!values.is_empty()).then_some(values)
}

fn event_status(line: &IcalLine<'_>) -> Option<EventStatus> {
    match line.raw_value_str().trim().to_ascii_uppercase().as_str() {
        "CONFIRMED" => Some(EventStatus::Confirmed),
        "TENTATIVE" => Some(EventStatus::Tentative),
        "CANCELLED" => Some(EventStatus::Cancelled),
        _ => None,
    }
}
