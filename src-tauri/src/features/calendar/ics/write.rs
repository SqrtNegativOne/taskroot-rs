//! Whole-resource writes: CST edits over the fetched body via `ical-rs`.
//!
//! - **I1 / I3** — Writes replace only owned properties on the `ical-rs` CST;
//!   `VTIMEZONE`, unmodelled `VALARM`s, `ATTENDEE` and unknown `X-*` survive.
//! - **I2** — Editing or removing one component leaves siblings alone; removing
//!   the last returns `None` so the caller DELETEs the resource.

use crate::error::AppError;
use crate::features::calendar::domain::{AppEvent, AppTask, TaskAlarmTrigger};
use crate::features::calendar::ics::cst::{
    ALARM_KIND, EVENT_KIND, TASK_KIND, child_components, is_component, parse_calendar, render_cst,
    replace_props, text_line, text_prop,
};
use crate::features::calendar::ics::dates::{date_line, stamp_line};
use crate::features::calendar::ics::read::event_from_component;
use crate::features::calendar::ics::read_task::{
    alarm_trigger_for_component, read_alarms, task_from_component,
};
use crate::features::calendar::ics::write_props::{
    alarm_trigger_line, build_alarm_component, event_lines, is_owned_event_prop,
    is_owned_task_prop, task_lines,
};
use chrono::Utc;
use ical::prop::IcalPropKind;
use ical::tree::cst::{IcalCst, IcalItem};

/// Merge an edited event into `raw`, appending its component when absent.
///
/// # Errors
/// Returns [`AppError::InvalidInput`] when `raw` is unparseable or holds another UID.
pub fn write_event_resource(raw: &str, event: &AppEvent) -> Result<String, AppError> {
    let mut calendar = parse_calendar(raw, "event resource")?;
    let uid = event_uid(event);
    if let Some(index) = find_event_index(&calendar, &event.id.0)? {
        if let Some(IcalItem::Component(component)) = calendar.items.get_mut(index) {
            replace_props(component, is_owned_event_prop, event_lines(event));
        }
        return Ok(render_cst(&calendar));
    }
    ensure_single_uid(&calendar, EVENT_KIND, &uid)?;
    calendar.push_component(build_event_component(event, &uid));
    Ok(render_cst(&calendar))
}

/// Merge an edited task into `raw` in place.
///
/// `VALARM`s are touched only when the modelled reminder list changed.
///
/// # Errors
/// Returns [`AppError::InvalidInput`] for an unparseable body, and
/// [`AppError::NotFound`] when no `VTODO` matches `task.id`.
pub fn write_task_resource(raw: &str, task: &AppTask) -> Result<String, AppError> {
    let mut calendar = parse_calendar(raw, "task resource")?;
    let index = find_task_index(&calendar, &task.id.0)?
        .ok_or_else(|| AppError::NotFound(format!("VTODO {} is not in this resource", task.id)))?;

    if let Some(IcalItem::Component(component)) = calendar.items.get_mut(index) {
        let alarms_changed = read_alarms(component) != task.alarms;
        replace_props(component, is_owned_task_prop, task_lines(task));
        if alarms_changed {
            apply_alarms(component, task.alarms.as_deref().unwrap_or_default());
        }
    }
    Ok(render_cst(&calendar))
}

/// Remove one event component (a master removes its whole series).
/// Returns `None` when no `VEVENT` remains.
///
/// # Errors
/// Returns [`AppError::InvalidInput`] when `raw` is unparseable.
pub fn delete_event_component(raw: &str, event: &AppEvent) -> Result<Option<String>, AppError> {
    let mut calendar = parse_calendar(raw, "event resource")?;
    let targets = if event.recurring_event_id.is_none() {
        let uid = event_uid(event);
        calendar
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| match item {
                IcalItem::Component(child)
                    if is_component(child, EVENT_KIND)
                        && text_prop(child, "UID").as_deref() == Some(uid.as_str()) =>
                {
                    Some(idx)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    } else {
        find_event_index(&calendar, &event.id.0)?
            .into_iter()
            .collect()
    };
    Ok(finish_delete(&mut calendar, EVENT_KIND, &targets, raw))
}

/// Remove one task component. Returns `None` when no `VTODO` remains.
///
/// # Errors
/// Returns [`AppError::InvalidInput`] when `raw` is unparseable.
pub fn delete_task_component(raw: &str, task: &AppTask) -> Result<Option<String>, AppError> {
    let mut calendar = parse_calendar(raw, "task resource")?;
    let targets = find_task_index(&calendar, &task.id.0)?
        .into_iter()
        .collect::<Vec<_>>();
    Ok(finish_delete(&mut calendar, TASK_KIND, &targets, raw))
}

/// Build a new one-component `VEVENT` resource.
///
/// # Errors
/// Reserved for future validation; infallible today.
#[allow(clippy::unnecessary_wraps)]
pub fn create_event_resource(event: &AppEvent) -> Result<String, AppError> {
    Ok(render_new_calendar(build_event_component(
        event,
        &event_uid(event),
    )))
}

/// Build a new one-component `VTODO` resource.
///
/// # Errors
/// Reserved for future validation; infallible today.
#[allow(clippy::unnecessary_wraps)]
pub fn create_task_resource(task: &AppTask) -> Result<String, AppError> {
    let mut component = IcalCst::empty(TASK_KIND);
    component.push_line(text_line(IcalPropKind::Uid, &task.id.0));
    for line in task_lines(task) {
        component.push_line(line);
    }
    for alarm in task.alarms.as_deref().unwrap_or_default() {
        component.push_component(build_alarm_component(alarm));
    }
    Ok(render_new_calendar(component))
}

fn build_event_component(event: &AppEvent, uid: &str) -> IcalCst<'static> {
    let mut component = IcalCst::empty(EVENT_KIND);
    component.push_line(text_line(IcalPropKind::Uid, uid));
    if let Some(original) = event.original_start_time {
        if let Some(line) = date_line(
            IcalPropKind::RecurrenceId,
            original,
            event.is_all_day(),
            event.timezone.as_deref(),
        ) {
            component.push_line(line);
        }
    }
    component.push_line(stamp_line(IcalPropKind::DtStamp, Utc::now()));
    for line in event_lines(event) {
        component.push_line(line);
    }
    component
}

fn render_new_calendar(component: IcalCst<'static>) -> String {
    let mut calendar = IcalCst::v2();
    calendar.push_line(text_line(
        IcalPropKind::ProdId,
        "-//Taskroot//Taskroot Rust//EN",
    ));
    calendar.push_line(text_line(IcalPropKind::CalScale, "GREGORIAN"));
    calendar.push_component(component);
    render_cst(&calendar)
}

fn event_uid(event: &AppEvent) -> String {
    event
        .recurring_event_id
        .clone()
        .unwrap_or_else(|| event.id.clone())
        .0
}

fn find_event_index(calendar: &IcalCst<'_>, target_id: &str) -> Result<Option<usize>, AppError> {
    for (idx, item) in calendar.items.iter().enumerate() {
        if let IcalItem::Component(child) = item {
            if is_component(child, EVENT_KIND) && event_from_component(child)?.id.0 == target_id {
                return Ok(Some(idx));
            }
        }
    }
    Ok(None)
}

fn find_task_index(calendar: &IcalCst<'_>, target_id: &str) -> Result<Option<usize>, AppError> {
    for (idx, item) in calendar.items.iter().enumerate() {
        if let IcalItem::Component(child) = item {
            if is_component(child, TASK_KIND) && task_from_component(child)?.id.0 == target_id {
                return Ok(Some(idx));
            }
        }
    }
    Ok(None)
}

fn ensure_single_uid(calendar: &IcalCst<'_>, kind: &str, uid: &str) -> Result<(), AppError> {
    let foreign = child_components(calendar, kind)
        .any(|child| text_prop(child, "UID").as_deref() != Some(uid));
    if foreign {
        return Err(AppError::InvalidInput(format!(
            "a {kind} resource holds one UID; refusing to add {uid}"
        )));
    }
    Ok(())
}

fn finish_delete(
    calendar: &mut IcalCst<'_>,
    kind: &str,
    targets: &[usize],
    raw: &str,
) -> Option<String> {
    if targets.is_empty() {
        return Some(raw.to_string());
    }
    if child_components(calendar, kind).count().saturating_sub(targets.len()) == 0 {
        return None;
    }
    for &idx in targets.iter().rev() {
        calendar.items.remove(idx);
    }
    Some(render_cst(calendar))
}

fn apply_alarms(vtodo: &mut IcalCst<'_>, desired: &[TaskAlarmTrigger]) {
    let managed: Vec<usize> = vtodo
        .items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| match item {
            IcalItem::Component(child)
                if is_component(child, ALARM_KIND)
                    && alarm_trigger_for_component(child).is_some() =>
            {
                Some(idx)
            }
            _ => None,
        })
        .collect();

    let triggers: Vec<(&TaskAlarmTrigger, ical::tree::line::IcalLine<'static>)> = desired
        .iter()
        .map(|alarm| (alarm, alarm_trigger_line(alarm)))
        .collect();

    for (&idx, (_, trigger)) in managed.iter().zip(&triggers) {
        if let Some(IcalItem::Component(alarm_cst)) = vtodo.items.get_mut(idx) {
            replace_props(
                alarm_cst,
                |line| line.name.get().eq_ignore_ascii_case("TRIGGER"),
                vec![trigger.clone()],
            );
        }
    }
    for &idx in managed.iter().skip(triggers.len()).rev() {
        vtodo.items.remove(idx);
    }
    for (alarm, _) in triggers.into_iter().skip(managed.len()) {
        vtodo.push_component(build_alarm_component(alarm));
    }
}
