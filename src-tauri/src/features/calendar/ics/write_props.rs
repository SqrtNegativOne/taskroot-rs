//! Owned property encoders and predicates for `VEVENT`, `VTODO` and `VALARM`.

use crate::features::calendar::domain::{
    AppEvent, AppTask, AppTaskStatus, EventStatus, Tag, TaskAlarmAnchor, TaskAlarmTrigger,
    TaskPriority,
};
use crate::features::calendar::ics::cst::{
    ALARM_KIND, encode, is_dependency_relation, is_parent_relation, strip_cr, text_line,
};
use crate::features::calendar::ics::dates::{
    DTO_DATE_LEN, ICAL_UTC, date_line, parse_datetime, stamp_line, task_date_line,
};
use chrono::Utc;
use ical::param::IcalParam;
use ical::prop::{IcalPropKind, IcalPropName};
use ical::tree::cst::IcalCst;
use ical::tree::line::IcalLine;
use ical::value::IcalValue;
use ical::value::datetime::IcalDateTime;
use ical::value::duration::IcalDuration;
use ical::value::geo::IcalGeo;
use ical::value::integer::IcalInteger;
use ical::value::recur::IcalRecur;
use ical::value::text::{IcalText, IcalTextList};
use std::borrow::Cow;

const EVENT_OWNED: &[&str] = &[
    "SUMMARY",
    "DESCRIPTION",
    "DTSTART",
    "DTEND",
    "PRIORITY",
    "CATEGORIES",
    "RRULE",
    "EXDATE",
    "STATUS",
    "COLOR",
    "X-TASKROOT-TASK-ID",
];

const TASK_OWNED: &[&str] = &[
    "SUMMARY",
    "DESCRIPTION",
    "DTSTART",
    "DUE",
    "RRULE",
    "STATUS",
    "COMPLETED",
    "PERCENT-COMPLETE",
    "PRIORITY",
    "CATEGORIES",
    "GEO",
    "ESTIMATED-DURATION",
    "CREATED",
    "LAST-MODIFIED",
    "DTSTAMP",
];

pub(super) fn is_owned_event_prop(line: &IcalLine<'_>) -> bool {
    EVENT_OWNED
        .iter()
        .any(|name| line.name.get().eq_ignore_ascii_case(name))
}

pub(super) fn is_owned_task_prop(line: &IcalLine<'_>) -> bool {
    TASK_OWNED
        .iter()
        .any(|name| line.name.get().eq_ignore_ascii_case(name))
        || is_parent_relation(line)
        || is_dependency_relation(line)
}

pub(super) fn is_all_day(event: &AppEvent) -> bool {
    event.is_all_day.unwrap_or_else(|| {
        let value = event.start_time.as_str();
        value.len() == DTO_DATE_LEN && !value.contains('T') && !value.contains(' ')
    })
}

pub(super) fn event_lines(event: &AppEvent) -> Vec<IcalLine<'static>> {
    let all_day = is_all_day(event);
    let zone = event.timezone.as_deref();
    let mut lines = vec![text_line(IcalPropKind::Summary, &event.title)];
    lines.extend(
        event
            .description
            .as_deref()
            .map(|text| text_line(IcalPropKind::Description, text)),
    );
    lines.extend(date_line(
        IcalPropKind::DtStart,
        &event.start_time,
        all_day,
        zone,
    ));
    lines.extend(date_line(IcalPropKind::DtEnd, &event.end_time, all_day, zone));
    lines.extend(priority_line(event.priority.as_ref()));
    lines.extend(category_line(event.tags.as_deref()));
    lines.extend(rrule_lines(event.rrule.as_deref()));
    for exdate in event.exdates.as_deref().unwrap_or_default() {
        lines.extend(date_line(IcalPropKind::ExDate, exdate, all_day, zone));
    }
    if let Some(status) = event.status.as_ref() {
        let label = match status {
            EventStatus::Confirmed => "CONFIRMED",
            EventStatus::Tentative => "TENTATIVE",
            EventStatus::Cancelled => "CANCELLED",
        };
        lines.push(text_line(IcalPropKind::Status, label));
    }
    lines.extend(
        event
            .color
            .as_ref()
            .map(|color| text_line(IcalPropKind::Color, &color.0)),
    );
    lines.extend(event.task_id.as_ref().map(|id| {
        encode(
            IcalPropName::Unknown(Cow::Borrowed("X-TASKROOT-TASK-ID")),
            Vec::new(),
            IcalValue::Text(IcalText(strip_cr(&id.0))),
        )
    }));
    lines
}

pub(super) fn task_lines(task: &AppTask) -> Vec<IcalLine<'static>> {
    let done = task.status == Some(AppTaskStatus::Done);
    let stamp = Utc::now().format(ICAL_UTC).to_string();
    let zone = task.timezone.as_deref();
    let status = match task.status.as_ref() {
        Some(AppTaskStatus::NextUp) => "IN-PROCESS",
        Some(AppTaskStatus::Done) => "COMPLETED",
        Some(AppTaskStatus::Cancelled) => "CANCELLED",
        Some(AppTaskStatus::Todo) | None => "NEEDS-ACTION",
    };

    let mut lines = vec![text_line(IcalPropKind::Summary, &task.title)];
    lines.extend(
        task.notes
            .as_deref()
            .map(|text| text_line(IcalPropKind::Description, text)),
    );
    lines.extend(task_date_line(
        IcalPropKind::DtStart,
        task.start.as_deref(),
        zone,
    ));
    lines.extend(task_date_line(IcalPropKind::Due, task.due.as_deref(), zone));
    lines.extend(rrule_lines(task.rrule.as_deref()));
    lines.push(text_line(IcalPropKind::Status, status));
    if done {
        lines.push(
            task.completed_at
                .as_deref()
                .and_then(|at| stamp_line(IcalPropKind::Completed, at))
                .unwrap_or_else(|| utc_line(IcalPropKind::Completed, &stamp)),
        );
        lines.push(encode(
            IcalPropKind::PercentComplete,
            Vec::new(),
            IcalValue::Integer(IcalInteger(Cow::Borrowed("100"))),
        ));
    }
    lines.extend(priority_line(task.priority.as_ref()));
    lines.extend(category_line(task.tags.as_deref()));
    lines.extend(task.geo.map(|geo| {
        encode(
            IcalPropKind::Geo,
            Vec::new(),
            IcalValue::Geo(IcalGeo {
                latitude: Cow::Owned(geo.latitude.to_string()),
                longitude: Cow::Owned(geo.longitude.to_string()),
            }),
        )
    }));
    lines.extend(task.parent_task.as_ref().map(|id| {
        encode(
            IcalPropKind::RelatedTo,
            vec![IcalParam::RelType(Cow::Borrowed("PARENT"))],
            IcalValue::Text(IcalText(strip_cr(&id.0))),
        )
    }));
    for dep in task.dependencies.as_deref().unwrap_or_default() {
        lines.push(encode(
            IcalPropKind::RelatedTo,
            vec![IcalParam::RelType(Cow::Borrowed("DEPENDS-ON"))],
            IcalValue::Text(IcalText(strip_cr(&dep.0))),
        ));
    }
    lines.extend(task.est.filter(|minutes| *minutes > 0).map(|minutes| {
        encode(
            IcalPropName::Unknown(Cow::Borrowed("ESTIMATED-DURATION")),
            Vec::new(),
            IcalValue::Duration(IcalDuration(Cow::Owned(format!("PT{minutes}M")))),
        )
    }));
    lines.extend(
        task.added
            .as_deref()
            .and_then(|added| stamp_line(IcalPropKind::Created, added)),
    );
    lines.extend(
        task.updated_at
            .as_deref()
            .and_then(|updated| stamp_line(IcalPropKind::LastModified, updated)),
    );
    lines.push(utc_line(IcalPropKind::DtStamp, &stamp));
    lines
}

pub(super) fn alarm_trigger_line(alarm: &TaskAlarmTrigger) -> Option<IcalLine<'static>> {
    match alarm {
        TaskAlarmTrigger::Relative {
            minutes_before,
            anchor,
        } => {
            let related = match anchor {
                TaskAlarmAnchor::Start => "START",
                TaskAlarmAnchor::End => "END",
            };
            let dur = if *minutes_before > 0 {
                format!("-PT{minutes_before}M")
            } else {
                format!("PT{}M", minutes_before.unsigned_abs())
            };
            Some(encode(
                IcalPropKind::Trigger,
                vec![IcalParam::Related(Cow::Borrowed(related))],
                IcalValue::Duration(IcalDuration(Cow::Owned(dur))),
            ))
        }
        TaskAlarmTrigger::Absolute { at } => {
            let stamp = parse_datetime(at)?;
            Some(encode(
                IcalPropKind::Trigger,
                vec![IcalParam::Value(Cow::Borrowed("DATE-TIME"))],
                IcalValue::DateTime(IcalDateTime(Cow::Owned(stamp.format(ICAL_UTC).to_string()))),
            ))
        }
    }
}

pub(super) fn build_alarm_component(alarm: &TaskAlarmTrigger) -> Option<IcalCst<'static>> {
    let mut component = IcalCst::empty(ALARM_KIND);
    component.push_line(alarm_trigger_line(alarm)?);
    component.push_line(text_line(IcalPropKind::Action, "DISPLAY"));
    component.push_line(text_line(
        IcalPropKind::Description,
        "Default Tasks.org description",
    ));
    Some(component)
}

pub(super) fn utc_line(kind: IcalPropKind, stamp: &str) -> IcalLine<'static> {
    encode(
        kind,
        Vec::new(),
        IcalValue::DateTime(IcalDateTime(Cow::Owned(stamp.to_string()))),
    )
}

fn priority_line(priority: Option<&TaskPriority>) -> Option<IcalLine<'static>> {
    let value = match priority {
        None | Some(TaskPriority::None) => return None,
        Some(TaskPriority::Low) => "7",
        Some(TaskPriority::Medium) => "5",
        Some(TaskPriority::High) => "3",
        Some(TaskPriority::Urgent) => "1",
    };
    Some(encode(
        IcalPropKind::Priority,
        Vec::new(),
        IcalValue::Integer(IcalInteger(Cow::Borrowed(value))),
    ))
}

fn category_line(tags: Option<&[Tag]>) -> Option<IcalLine<'static>> {
    let names: Vec<Cow<'_, str>> = tags?
        .iter()
        .map(|tag| strip_cr(&tag.name))
        .collect();
    (!names.is_empty()).then(|| {
        encode(
            IcalPropKind::Categories,
            Vec::new(),
            IcalValue::TextList(IcalTextList(names)),
        )
    })
}

fn rrule_lines(rrule: Option<&str>) -> Vec<IcalLine<'static>> {
    rrule
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            let body = line
                .get(..6)
                .filter(|head| head.eq_ignore_ascii_case("RRULE:"))
                .and_then(|_| line.get(6..))?
                .trim();
            (!body.is_empty()).then(|| {
                encode(
                    IcalPropKind::RRule,
                    Vec::new(),
                    IcalValue::Recur(IcalRecur(Cow::Borrowed(body))),
                )
            })
        })
        .collect()
}
