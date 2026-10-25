//! Whole-resource writes: read-modify-write over the raw body.
//!
//! Invariants I1–I3 from `caldav-migration-plan.md` §7:
//!
//! - **I1** — `PUT` replaces the resource, so a write starts from the body that was
//!   fetched and only the properties listed in [`events::event_specs`] /
//!   [`tasks::task_specs`] change. Everything else (VTIMEZONE, VALARM, ATTENDEE,
//!   `X-MOZ-GENERATION`, `X-APPLE-SORT-ORDER`, unknown `X-*`) is re-emitted verbatim.
//! - **I2** — a resource is a *set* of components sharing a UID. Editing or removing
//!   one leaves the master and the other exceptions alone; removing the last
//!   component returns `None` so the caller DELETEs the resource.
//! - **I3** — nothing is round-tripped through `icalendar`'s serializer except a
//!   brand-new resource, which has nothing to preserve.

use crate::domain::{AppEvent, AppTask, AppTaskStatus, EventStatus, TaskId, TaskPriority};
use crate::error::AppError;
use crate::ics::lines::{ComponentSpan, Document, OwnedKind, PropertySpec};
use crate::ics::read;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz as ChronoTz;
use icalendar::{Calendar, Event, Property, Todo};
use std::str::FromStr;

const ICAL_DATE: &str = "%Y%m%d";
const ICAL_DATE_TIME: &str = "%Y%m%dT%H%M%S";
const ICAL_UTC: &str = "%Y%m%dT%H%M%SZ";
const DTO_DATE: &str = "%Y-%m-%d";
const DTO_DATE_TIME: &str = "%Y-%m-%dT%H:%M:%S";

const EVENT_KIND: &str = "VEVENT";
const TASK_KIND: &str = "VTODO";

/// Merge an edited event into `raw`, adding its component when the resource does
/// not hold it yet (the importer uses that to append exception instances).
///
/// # Errors
/// Returns [`AppError::InvalidInput`] when `raw` is unparseable or already holds a
/// different UID.
pub fn write_event_resource(raw: &str, event: &AppEvent) -> Result<String, AppError> {
    let document = Document::parse(raw);
    let spans = document.components();
    let uid = event_uid(event);
    if let Some(position) = read::events(raw)?
        .iter()
        .position(|candidate| candidate.id == event.id)
    {
        let span = nth_kind(&spans, EVENT_KIND, position)?;
        return Ok(document.edit_component(span, &event_specs(event)));
    }
    ensure_single_uid(&spans, EVENT_KIND, &uid)?;
    let specs = event_specs(event);
    Ok(document.insert_component(&component_lines(
        EVENT_KIND,
        event_header_lines(event, &uid),
        &specs,
    )))
}

/// Merge an edited task into `raw` in place. A VTODO resource holds exactly one
/// component per UID, so a missing UID is a stale caller, not an insert.
///
/// # Errors
/// Returns [`AppError::InvalidInput`] when `raw` is unparseable, and
/// [`AppError::NotFound`] when no VTODO has this task's UID.
pub fn write_task_resource(raw: &str, task: &AppTask) -> Result<String, AppError> {
    let document = Document::parse(raw);
    let spans = document.components();
    let position = read::tasks(raw)?
        .iter()
        .position(|candidate| candidate.id == task.id)
        .ok_or_else(|| AppError::NotFound(format!("VTODO {} is not in this resource", task.id)))?;
    let span = nth_kind(&spans, TASK_KIND, position)?;
    Ok(document.edit_component(span, &task_specs(task)))
}

/// Remove one event component.
///
/// A master takes its whole series with it (exceptions have no meaning without
/// it). Returns `None` when no component remains, which means "DELETE the
/// resource".
///
/// # Errors
/// Returns [`AppError::InvalidInput`] when `raw` is unparseable.
pub fn delete_event_component(raw: &str, event: &AppEvent) -> Result<Option<String>, AppError> {
    let document = Document::parse(raw);
    let spans = document.components();
    let targets = if event.recurring_event_id.is_none() {
        let uid = event_uid(event);
        spans
            .iter()
            .filter(|span| span.kind == EVENT_KIND && span.uid.as_deref() == Some(uid.as_str()))
            .collect::<Vec<_>>()
    } else {
        let position = read::events(raw)?
            .iter()
            .position(|candidate| candidate.id == event.id);
        match position {
            Some(position) => vec![nth_kind(&spans, EVENT_KIND, position)?],
            None => Vec::new(),
        }
    };
    Ok(finish_delete(&document, &spans, EVENT_KIND, &targets, raw))
}

/// Remove one task component. Returns `None` when no `VTODO` remains.
///
/// # Errors
/// Returns [`AppError::InvalidInput`] when `raw` is unparseable.
pub fn delete_task_component(raw: &str, task: &AppTask) -> Result<Option<String>, AppError> {
    let document = Document::parse(raw);
    let spans = document.components();
    let position = read::tasks(raw)?
        .iter()
        .position(|candidate| candidate.id == task.id);
    let targets = match position {
        Some(position) => vec![nth_kind(&spans, TASK_KIND, position)?],
        None => Vec::new(),
    };
    Ok(finish_delete(&document, &spans, TASK_KIND, &targets, raw))
}

/// Build a new one-component VEVENT resource with `icalendar`'s builder.
///
/// The DTO's `id` is the UID; an exception (`recurring_event_id` set) also gets a
/// `RECURRENCE-ID`.
///
pub fn create_event_resource(event: &AppEvent) -> Result<String, AppError> {
    let mut component = Event::new();
    for line in event_header_lines(event, &event_uid(event)) {
        append_line(&mut component, &line);
    }
    append_specs(&mut component, &event_specs(event));
    Ok(render_calendar(component))
}

/// Build a new one-component VTODO resource with `icalendar`'s builder.
///
pub fn create_task_resource(task: &AppTask) -> Result<String, AppError> {
    let mut component = Todo::new();
    append_line(&mut component, &format!("UID:{}", task.id));
    append_line(
        &mut component,
        &format!("DTSTAMP:{}", Utc::now().format(ICAL_UTC)),
    );
    append_specs(&mut component, &task_specs(task));
    Ok(render_calendar(component))
}

fn render_calendar(component: impl Into<icalendar::CalendarComponent>) -> String {
    let mut calendar = Calendar::empty();
    calendar
        .append_property(Property::new("VERSION", "2.0"))
        .append_property(Property::new("PRODID", "-//Taskroot//Taskroot Rust//EN"))
        .append_property(Property::new("CALSCALE", "GREGORIAN"));
    calendar.push(component);
    calendar.to_string()
}

/// `UID`, an exception's `RECURRENCE-ID`, and a freshly generated `DTSTAMP`.
fn event_header_lines(event: &AppEvent, uid: &str) -> Vec<String> {
    let mut lines = vec![format!("UID:{uid}")];
    if let Some(original) = event.original_start_time.as_deref() {
        if let Some(line) = recurrence_id_line(event, original) {
            lines.push(line);
        }
    }
    lines.push(format!("DTSTAMP:{}", Utc::now().format(ICAL_UTC)));
    lines
}

fn append_specs(component: &mut impl icalendar::Component, specs: &[PropertySpec]) {
    for spec in specs {
        for value in &spec.values {
            append_line(component, value);
        }
    }
}

fn append_line(component: &mut impl icalendar::Component, line: &str) {
    let Some((name, value)) = line.split_once(':') else {
        return;
    };
    let Some(property) = property_from(name, value) else {
        return;
    };
    component.append_multi_property(property);
}

fn property_from(name: &str, value: &str) -> Option<Property> {
    let mut segments = name.split(';');
    let key = segments.next()?;
    let mut property = Property::new(key, value);
    for segment in segments {
        let (param, param_value) = segment.split_once('=')?;
        property.add_parameter(param, param_value);
    }
    Some(property)
}

fn component_lines(kind: &str, header: Vec<String>, specs: &[PropertySpec]) -> Vec<String> {
    let mut lines = vec![format!("BEGIN:{kind}")];
    lines.extend(header);
    for spec in specs {
        lines.extend(spec.values.iter().cloned());
    }
    lines.push(format!("END:{kind}"));
    lines
}

fn finish_delete(
    document: &Document,
    spans: &[ComponentSpan],
    kind: &str,
    targets: &[&ComponentSpan],
    raw: &str,
) -> Option<String> {
    if targets.is_empty() {
        return Some(raw.to_string());
    }
    let total = spans.iter().filter(|span| span.kind == kind).count();
    if total.saturating_sub(targets.len()) == 0 {
        return None;
    }
    Some(document.delete_components(targets))
}

fn nth_kind<'a>(
    spans: &'a [ComponentSpan],
    kind: &str,
    position: usize,
) -> Result<&'a ComponentSpan, AppError> {
    spans
        .iter()
        .filter(|span| span.kind == kind)
        .nth(position)
        .ok_or_else(|| {
            AppError::Internal(format!("{kind} #{position} vanished between parse passes"))
        })
}

fn ensure_single_uid(spans: &[ComponentSpan], kind: &str, uid: &str) -> Result<(), AppError> {
    let foreign = spans
        .iter()
        .any(|span| span.kind == kind && span.uid.as_deref() != Some(uid));
    if foreign {
        return Err(AppError::InvalidInput(format!(
            "a {kind} resource holds one UID; refusing to add {uid}"
        )));
    }
    Ok(())
}

/// Identity is the DTO id, which the reader derives from UID + RECURRENCE-ID, so
/// the DTO list and the component spans are index-aligned.
fn event_uid(event: &AppEvent) -> String {
    event
        .recurring_event_id
        .clone()
        .unwrap_or_else(|| event.id.clone())
        .0
}

fn event_specs(event: &AppEvent) -> Vec<PropertySpec> {
    let all_day = is_all_day(event);
    let zone = event.timezone.as_deref();
    vec![
        PropertySpec::new(
            OwnedKind::Name("SUMMARY"),
            vec![format!("SUMMARY:{}", escape_text(&event.title))],
        ),
        PropertySpec::optional(
            OwnedKind::Name("DESCRIPTION"),
            event
                .description
                .as_deref()
                .map(|description| format!("DESCRIPTION:{}", escape_text(description))),
        ),
        PropertySpec::optional(
            OwnedKind::Name("DTSTART"),
            date_line("DTSTART", &event.start_time, all_day, zone),
        ),
        PropertySpec::optional(
            OwnedKind::Name("DTEND"),
            date_line("DTEND", &event.end_time, all_day, zone),
        ),
        PropertySpec::new(
            OwnedKind::Name("RRULE"),
            rrule_lines(event.rrule.as_deref()),
        ),
        PropertySpec::new(
            OwnedKind::Name("EXDATE"),
            exdate_lines(event, all_day, zone),
        ),
        PropertySpec::optional(
            OwnedKind::Name("STATUS"),
            event
                .status
                .as_ref()
                .map(|status| format!("STATUS:{}", status_value(status))),
        ),
        PropertySpec::optional(
            OwnedKind::Name("COLOR"),
            event
                .color
                .as_ref()
                .map(|color| format!("COLOR:{}", color.0)),
        ),
    ]
}

fn task_specs(task: &AppTask) -> Vec<PropertySpec> {
    let done = task.status == Some(AppTaskStatus::Done);
    let stamp = Utc::now().format(ICAL_UTC).to_string();
    vec![
        PropertySpec::new(
            OwnedKind::Name("SUMMARY"),
            vec![format!("SUMMARY:{}", escape_text(&task.title))],
        ),
        PropertySpec::optional(
            OwnedKind::Name("DESCRIPTION"),
            task.notes
                .as_deref()
                .map(|notes| format!("DESCRIPTION:{}", escape_text(notes))),
        ),
        PropertySpec::optional(
            OwnedKind::Name("DUE"),
            task.due.as_deref().and_then(due_line),
        ),
        PropertySpec::new(
            OwnedKind::Name("STATUS"),
            vec![format!(
                "STATUS:{}",
                if done { "COMPLETED" } else { "NEEDS-ACTION" }
            )],
        ),
        PropertySpec::optional(
            OwnedKind::Name("COMPLETED"),
            done.then(|| format!("COMPLETED:{stamp}")),
        ),
        PropertySpec::optional(
            OwnedKind::Name("PERCENT-COMPLETE"),
            done.then(|| "PERCENT-COMPLETE:100".to_string()),
        ),
        PropertySpec::new(
            OwnedKind::Name("PRIORITY"),
            vec![format!(
                "PRIORITY:{}",
                priority_value(task.priority.as_ref())
            )],
        ),
        PropertySpec::new(
            OwnedKind::Name("CATEGORIES"),
            category_lines(task.tags.as_deref()),
        ),
        PropertySpec::new(
            OwnedKind::ParentRelation,
            related_lines(task.parent_task.as_ref()),
        ),
        PropertySpec::new(
            OwnedKind::Name("X-TASKROOT-STATUS"),
            vec![format!(
                "X-TASKROOT-STATUS:{}",
                workflow_value(task.status.as_ref())
            )],
        ),
        PropertySpec::optional(
            OwnedKind::Name("X-TASKROOT-DEPENDENCIES"),
            dependency_line(task.dependencies.as_deref()),
        ),
        PropertySpec::optional(
            OwnedKind::Name("X-TASKROOT-EST"),
            task.est.map(|est| format!("X-TASKROOT-EST:{est}")),
        ),
        PropertySpec::optional(
            OwnedKind::Name("X-TASKROOT-CANVAS-X"),
            task.canvas_x.map(|x| format!("X-TASKROOT-CANVAS-X:{x}")),
        ),
        PropertySpec::optional(
            OwnedKind::Name("X-TASKROOT-CANVAS-Y"),
            task.canvas_y.map(|y| format!("X-TASKROOT-CANVAS-Y:{y}")),
        ),
        PropertySpec::optional(
            OwnedKind::Name("X-TASKROOT-ON-CANVAS"),
            task.on_canvas
                .map(|on| format!("X-TASKROOT-ON-CANVAS:{on}")),
        ),
        PropertySpec::optional(
            OwnedKind::Name("X-TASKROOT-TABS"),
            task.tabs
                .as_deref()
                .map(|tabs| format!("X-TASKROOT-TABS:{}", escape_text(tabs))),
        ),
        PropertySpec::optional(
            OwnedKind::Name("CREATED"),
            task.added
                .as_deref()
                .and_then(|added| stamp_line("CREATED", added)),
        ),
        PropertySpec::optional(
            OwnedKind::Name("LAST-MODIFIED"),
            task.updated_at
                .as_deref()
                .and_then(|updated| stamp_line("LAST-MODIFIED", updated)),
        ),
        PropertySpec::new(OwnedKind::Name("DTSTAMP"), vec![format!("DTSTAMP:{stamp}")]),
    ]
}

fn is_all_day(event: &AppEvent) -> bool {
    event.is_all_day.unwrap_or_else(|| {
        let value = event.start_time.as_str();
        value.len() == DTO_DATE.len() && !value.contains('T') && !value.contains(' ')
    })
}

fn date_line(name: &str, value: &str, all_day: bool, zone: Option<&str>) -> Option<String> {
    if all_day {
        let date = NaiveDate::parse_from_str(value, DTO_DATE).ok()?;
        return Some(format!("{name};VALUE=DATE:{}", date.format(ICAL_DATE)));
    }
    let naive = wall_clock(value, zone)?;
    let stamp = naive.format(ICAL_DATE_TIME);
    Some(zone.map_or_else(
        || format!("{name}:{stamp}"),
        |timezone| format!("{name};TZID={timezone}:{stamp}"),
    ))
}

fn recurrence_id_line(event: &AppEvent, original: &str) -> Option<String> {
    if is_all_day(event) {
        let date = NaiveDate::parse_from_str(date_prefix(original), DTO_DATE).ok()?;
        return Some(format!(
            "RECURRENCE-ID;VALUE=DATE:{}",
            date.format(ICAL_DATE)
        ));
    }
    let naive = wall_clock(original, event.timezone.as_deref())?;
    Some(event.timezone.as_deref().map_or_else(
        || format!("RECURRENCE-ID:{}", naive.format(ICAL_DATE_TIME)),
        |timezone| {
            format!(
                "RECURRENCE-ID;TZID={timezone}:{}",
                naive.format(ICAL_DATE_TIME)
            )
        },
    ))
}

fn exdate_lines(event: &AppEvent, all_day: bool, zone: Option<&str>) -> Vec<String> {
    event
        .exdates
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|value| {
            if all_day {
                let date = NaiveDate::parse_from_str(date_prefix(value), DTO_DATE).ok()?;
                return Some(format!("EXDATE;VALUE=DATE:{}", date.format(ICAL_DATE)));
            }
            let naive = wall_clock(value, zone)?;
            Some(zone.map_or_else(
                || format!("EXDATE:{}", naive.format(ICAL_DATE_TIME)),
                |timezone| format!("EXDATE;TZID={timezone}:{}", naive.format(ICAL_DATE_TIME)),
            ))
        })
        .collect()
}

fn rrule_lines(rrule: Option<&str>) -> Vec<String> {
    rrule
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.get(..5)
                .is_some_and(|head| head.eq_ignore_ascii_case("RRULE"))
        })
        .map(ToString::to_string)
        .collect()
}

fn due_line(due: &str) -> Option<String> {
    if let Ok(date) = NaiveDate::parse_from_str(due, DTO_DATE) {
        return Some(format!("DUE;VALUE=DATE:{}", date.format(ICAL_DATE)));
    }
    wall_clock(due, None).map(|naive| format!("DUE:{}", naive.format(ICAL_DATE_TIME)))
}

fn stamp_line(name: &str, value: &str) -> Option<String> {
    let datetime = parse_datetime(value)?;
    Some(format!("{name}:{}", datetime.format(ICAL_UTC)))
}

fn parse_datetime(value: &str) -> Option<NaiveDateTime> {
    if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
        return Some(datetime.with_timezone(&Utc).naive_utc());
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(value, DTO_DATE_TIME) {
        return Some(naive);
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, DTO_DATE) {
        return date.and_hms_opt(0, 0, 0);
    }
    None
}

/// Interpret a DTO wall clock (or an offset-carrying timestamp) in `zone`.
fn wall_clock(value: &str, zone: Option<&str>) -> Option<NaiveDateTime> {
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

fn date_prefix(value: &str) -> &str {
    value.get(..DTO_DATE.len()).unwrap_or(value)
}

fn category_lines(tags: Option<&[crate::domain::Tag]>) -> Vec<String> {
    let names = tags
        .unwrap_or_default()
        .iter()
        .map(|tag| escape_text(&tag.name))
        .collect::<Vec<_>>();
    if names.is_empty() {
        Vec::new()
    } else {
        vec![format!("CATEGORIES:{}", names.join(","))]
    }
}

fn related_lines(parent: Option<&TaskId>) -> Vec<String> {
    parent
        .map(|id| format!("RELATED-TO;RELTYPE=PARENT:{}", escape_text(&id.0)))
        .into_iter()
        .collect()
}

fn dependency_line(dependencies: Option<&[TaskId]>) -> Option<String> {
    let ids = dependencies
        .unwrap_or_default()
        .iter()
        .map(|id| id.0.as_str())
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return None;
    }
    let json = serde_json::to_string(&ids).ok()?;
    Some(format!("X-TASKROOT-DEPENDENCIES:{}", escape_text(&json)))
}

const fn priority_value(priority: Option<&TaskPriority>) -> u32 {
    match priority {
        None | Some(TaskPriority::None) => 0,
        Some(TaskPriority::Low) => 7,
        Some(TaskPriority::Medium) => 5,
        Some(TaskPriority::High) => 3,
        Some(TaskPriority::Urgent) => 1,
    }
}

const fn workflow_value(status: Option<&AppTaskStatus>) -> &'static str {
    match status {
        Some(AppTaskStatus::NextUp) => "next-up",
        Some(AppTaskStatus::Doing) => "doing",
        Some(AppTaskStatus::Done) => "done",
        _ => "todo",
    }
}

const fn status_value(status: &EventStatus) -> &'static str {
    match status {
        EventStatus::Confirmed => "CONFIRMED",
        EventStatus::Tentative => "TENTATIVE",
        EventStatus::Cancelled => "CANCELLED",
    }
}

fn escape_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            ';' => escaped.push_str("\\;"),
            ',' => escaped.push_str("\\,"),
            '\n' => escaped.push_str("\\n"),
            '\r' => {}
            other => escaped.push(other),
        }
    }
    escaped
}
