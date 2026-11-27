//! `VTODO` and `VALARM` projection from an `ical-rs` CST component.

use crate::error::AppError;
use crate::features::calendar::domain::{
    AppTask, AppTaskStatus, GeoPoint, TaskAlarmAnchor, TaskAlarmTrigger, TaskId,
};
use crate::features::calendar::ics::cst::{
    ALARM_KIND, child_components, is_dependency_relation, is_parent_relation, prop, props,
    text_prop,
};
use crate::features::calendar::ics::dates::{
    DateValue, absolute_trigger, canonical, duration_whole_minutes, is_date_time, iso_line,
    parse_date_line,
};
use crate::features::calendar::ics::read::{joined_rrule, priority_value, read_categories};
use ical::tree::cst::IcalCst;
use ical::tree::line::IcalLine;
use ical::tree::param::related::RELATED;
use ical::tree::param::value::VALUE;

pub(super) fn task_from_component(component: &IcalCst<'_>) -> Result<AppTask, AppError> {
    let uid = text_prop(component, "UID")
        .ok_or_else(|| AppError::InvalidInput("VTODO without UID".to_string()))?;
    let created = prop(component, "CREATED").and_then(iso_line);
    let stamp = prop(component, "DTSTAMP").and_then(iso_line);
    let due = prop(component, "DUE").and_then(parse_date_line);
    let start = prop(component, "DTSTART").and_then(parse_date_line);
    // One zone governs both lines, as tasks.org writes them: `DUE` wins.
    let timezone = due
        .as_ref()
        .and_then(DateValue::task_timezone)
        .or_else(|| start.as_ref().and_then(DateValue::task_timezone));

    Ok(AppTask {
        id: TaskId::from(uid),
        remote_id: None,
        remote_collection_id: None,
        title: text_prop(component, "SUMMARY").unwrap_or_default(),
        status: Some(read_task_status(component)),
        priority: prop(component, "PRIORITY").and_then(priority_value),
        tags: read_categories(component),
        parent_task: read_parent(component).map(TaskId::from),
        dependencies: read_dependencies(component),
        est: prop(component, "ESTIMATED-DURATION")
            .and_then(|line| duration_whole_minutes(&line.raw_value_str()))
            .filter(|minutes| *minutes > 0),
        added: created.or(stamp),
        notes: text_prop(component, "DESCRIPTION"),
        start: start
            .as_ref()
            .and_then(|value| canonical(value, timezone.as_deref())),
        due: due
            .as_ref()
            .and_then(|value| canonical(value, timezone.as_deref())),
        completed_at: prop(component, "COMPLETED").and_then(iso_line),
        rrule: joined_rrule(component),
        geo: read_geo(component),
        alarms: read_alarms(component),
        timezone,
        updated_at: prop(component, "LAST-MODIFIED").and_then(iso_line),
        etag: None,
    })
}

pub(super) fn read_alarms(component: &IcalCst<'_>) -> Option<Vec<TaskAlarmTrigger>> {
    let alarms: Vec<_> = child_components(component, ALARM_KIND)
        .filter_map(alarm_trigger_for_component)
        .collect();
    (!alarms.is_empty()).then_some(alarms)
}

pub(super) fn alarm_trigger_for_component(alarm: &IcalCst<'_>) -> Option<TaskAlarmTrigger> {
    prop(alarm, "TRIGGER").and_then(alarm_trigger)
}

fn read_task_status(component: &IcalCst<'_>) -> AppTaskStatus {
    match prop(component, "STATUS")
        .map(|line| line.raw_value_str().trim().to_ascii_uppercase())
        .as_deref()
    {
        Some("COMPLETED") => AppTaskStatus::Done,
        Some("CANCELLED") => AppTaskStatus::Cancelled,
        Some("IN-PROCESS") => AppTaskStatus::NextUp,
        _ => AppTaskStatus::Todo,
    }
}

fn read_parent(component: &IcalCst<'_>) -> Option<String> {
    props(component, "RELATED-TO")
        .find(|line| is_parent_relation(line))
        .map(|line| line.value.decode().into_owned())
}

fn read_dependencies(component: &IcalCst<'_>) -> Option<Vec<TaskId>> {
    let ids: Vec<_> = props(component, "RELATED-TO")
        .filter(|line| is_dependency_relation(line))
        .map(|line| line.value.decode().trim().to_string())
        .filter(|id| !id.is_empty())
        .map(TaskId::from)
        .collect();
    (!ids.is_empty()).then_some(ids)
}

fn read_geo(component: &IcalCst<'_>) -> Option<GeoPoint> {
    let line = prop(component, "GEO")?;
    Some(GeoPoint {
        latitude: line.value.decode_component(0).trim().parse().ok()?,
        longitude: line.value.decode_component(1).trim().parse().ok()?,
    })
}

fn alarm_trigger(trigger: &IcalLine<'_>) -> Option<TaskAlarmTrigger> {
    let raw_cow = trigger.raw_value_str();
    let raw = raw_cow.trim();
    let absolute = trigger
        .param::<VALUE>()
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
    let anchor = if trigger
        .param::<RELATED>()
        .is_some_and(|value| value.eq_ignore_ascii_case("END"))
    {
        TaskAlarmAnchor::End
    } else {
        TaskAlarmAnchor::Start
    };
    Some(TaskAlarmTrigger::Relative {
        minutes_before: duration_whole_minutes(raw)?.checked_neg()?,
        anchor,
    })
}
