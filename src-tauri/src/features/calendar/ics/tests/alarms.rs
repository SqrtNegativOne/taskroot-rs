//! `VALARM` handling: reminders are the only task lines rewritten on their own.

use super::fixtures::*;
use crate::features::calendar::domain::{TaskAlarmAnchor, TaskAlarmTrigger};
use crate::features::calendar::ics::{create_task_resource, write_task_resource};

#[test]
fn retitling_a_task_never_touches_its_alarm_or_its_timezone() {
    let raw = fixture(TASK_ALARM);
    let mut task = read_task(&raw);
    task.title = "Water the plants and the herbs".to_string();

    let written = write_task_resource(&raw, &task).unwrap();

    assert_only_owned_changed(&raw, &written, TASK_PROPERTIES);
    assert_nested_preserved(&raw, &written);
    assert!(written.contains("BEGIN:VTIMEZONE"));
    assert!(
        written.contains("DESCRIPTION:Default Tasks.org description"),
        "the VALARM DESCRIPTION must not become the task's notes"
    );
    assert!(
        written.contains("\r\nBEGIN:VALARM\r\nTRIGGER;RELATED=END:-PT15M\r\n"),
        "the alarm block must be untouched"
    );
    assert_eq!(read_task(&written).alarms, task.alarms);
}

#[test]
fn changing_a_reminder_rewrites_only_its_trigger() {
    let raw = fixture(TASK_ALARM);
    let mut task = read_task(&raw);
    task.alarms = Some(vec![TaskAlarmTrigger::Relative {
        minutes_before: 30,
        anchor: TaskAlarmAnchor::End,
    }]);

    let written = write_task_resource(&raw, &task).unwrap();

    // `TRIGGER` is owned only while the reminder list differs from the resource's.
    let owned = [TASK_PROPERTIES, &["TRIGGER"]].concat();
    assert_only_owned_changed(&raw, &written, &owned);
    assert!(written.contains("TRIGGER;RELATED=END:-PT30M"));
    assert!(!written.contains("-PT15M"));
    assert!(
        written.contains("ACTION:DISPLAY"),
        "the alarm's action is not a Taskroot property"
    );
    assert!(written.contains("DESCRIPTION:Default Tasks.org description"));
    assert_eq!(read_task(&written).alarms, task.alarms);
}

#[test]
fn clearing_the_reminders_removes_every_alarm() {
    let raw = fixture(TASK_ALARM);
    let mut task = read_task(&raw);
    task.alarms = None;

    let written = write_task_resource(&raw, &task).unwrap();

    assert!(!written.contains("VALARM"));
    assert!(written.contains("BEGIN:VTIMEZONE"));
    assert!(written.contains("SUMMARY:Water the plants"));
    assert!(written.contains("X-APPLE-SORT-ORDER:813839139"));
    assert!(written.contains("RRULE:FREQ=DAILY;COUNT=10"));
    assert_eq!(read_task(&written).alarms, None);
}

#[test]
fn adding_a_reminder_creates_a_tasks_org_alarm_block() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.alarms = Some(vec![TaskAlarmTrigger::Relative {
        minutes_before: 60,
        anchor: TaskAlarmAnchor::Start,
    }]);

    let written = write_task_resource(&raw, &task).unwrap();

    assert!(written.contains(
        "BEGIN:VALARM\r\nTRIGGER;RELATED=START:-PT60M\r\nACTION:DISPLAY\r\nDESCRIPTION:Default Tasks.org description\r\nEND:VALARM\r\nEND:VTODO\r\n"
    ));
    assert_eq!(read_task(&written).alarms, task.alarms);
}

#[test]
fn an_absolute_reminder_round_trips() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.alarms = Some(vec![TaskAlarmTrigger::Absolute {
        at: "2026-11-05T17:45:00Z".to_string(),
    }]);

    let written = write_task_resource(&raw, &task).unwrap();

    assert!(written.contains("TRIGGER;VALUE=DATE-TIME:20261105T174500Z"));
    assert_eq!(read_task(&written).alarms, task.alarms);
}

#[test]
fn an_unmodelled_alarm_survives_an_unrelated_edit() {
    let raw = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:a\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER;RELATED=END:-PT45S\r\nDESCRIPTION:odd\r\nEND:VALARM\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
    let mut task = read_task(raw);
    assert_eq!(task.alarms, None, "45 seconds is not a whole minute");
    task.title = "b".to_string();

    let written = write_task_resource(raw, &task).unwrap();

    assert_nested_preserved(raw, &written);
    assert!(written.contains("TRIGGER;RELATED=END:-PT45S"));
    assert!(written.contains("SUMMARY:b"));
}

#[test]
fn create_task_resource_emits_a_valarm_block() {
    let mut task = read_task(&fixture(TASK));
    task.alarms = Some(vec![TaskAlarmTrigger::Relative {
        minutes_before: 15,
        anchor: TaskAlarmAnchor::End,
    }]);

    let created = create_task_resource(&task).unwrap();

    assert!(created.contains(
        "BEGIN:VALARM\r\nTRIGGER;RELATED=END:-PT15M\r\nACTION:DISPLAY\r\nDESCRIPTION:Default Tasks.org description\r\nEND:VALARM\r\nEND:VTODO\r\n"
    ));
    assert_eq!(read_task(&created).alarms, task.alarms);
}
