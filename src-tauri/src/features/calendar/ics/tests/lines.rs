//! Escaping, folding, unknown-line preservation and RFC 5545 edge cases.

use super::fixtures::*;
use crate::features::calendar::domain::{TaskAlarmAnchor, TaskAlarmTrigger, TaskId};
use crate::features::calendar::ics::{
    read_event_resource, write_event_resource, write_task_resource,
};
use proptest::prelude::*;

#[test]
fn escaping_round_trips_commas_semicolons_and_newlines() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.title = "Brush, then; floss\\rinse\nthen bed".to_string();

    let written = write_task_resource(&raw, &task).unwrap();

    assert!(written.contains(r"SUMMARY:Brush\, then\; floss\\rinse\nthen bed"));
    assert_eq!(read_task(&written).title, task.title);
}

#[test]
fn a_long_title_is_folded_at_seventy_five_octets_and_reads_back() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.title = "a".repeat(200);

    let written = write_task_resource(&raw, &task).unwrap();

    for line in written.split("\r\n") {
        assert!(line.len() <= 75, "line of {} octets: {line:?}", line.len());
    }
    assert!(written.contains("\r\n "), "the long line must be folded");
    assert_eq!(read_task(&written).title, task.title);
}

#[test]
fn an_empty_property_value_survives_a_write() {
    let raw = append_component(
        &fixture(TASK),
        "BEGIN:VEVENT\r\nUID:e1\r\nDTSTART;VALUE=DATE:20261020\r\nDTEND;VALUE=DATE:20261021\r\nDTSTAMP:20261020T000000Z\r\nX-EMPTY:\r\nEND:VEVENT\r\n",
    );
    let event = read_event_resource(&raw, HREF, COLLECTION).unwrap()[0].clone();

    let written = write_event_resource(&raw, &event).unwrap();

    assert!(written.contains("X-EMPTY:\r\n"));
}

#[test]
fn case_insensitive_components_properties_and_parameters_round_trip() {
    let raw = "begin:vcalendar\r\nversion:2.0\r\nbegin:vtodo\r\nuid:t-case\r\nsummary:Lower\r\nrelated-to;reltype=parent:p-1\r\nbegin:valarm\r\naction:display\r\ntrigger;related=end:-p1d\r\ndescription:Keep me\r\nend:valarm\r\nend:vtodo\r\nend:vcalendar\r\n";
    let mut task = read_task(raw);
    assert_eq!(task.id, TaskId::from("t-case"));
    assert_eq!(task.title, "Lower");
    assert_eq!(task.parent_task, Some(TaskId::from("p-1")));
    assert_eq!(
        task.alarms,
        Some(vec![TaskAlarmTrigger::Relative {
            minutes_before: 1440,
            anchor: TaskAlarmAnchor::End,
        }])
    );

    task.title = "Updated".to_string();
    let written = write_task_resource(raw, &task).unwrap();
    assert_eq!(read_task(&written).title, "Updated");
    assert_eq!(read_task(&written).parent_task, Some(TaskId::from("p-1")));
    assert!(written.contains("description:Keep me"));
    assert_eq!(
        written.matches("SUMMARY:").count() + written.matches("summary:").count(),
        1,
        "editing a lowercase property must replace it rather than duplicate it: {written}"
    );
}

#[test]
fn quoted_parameters_with_colons_and_semicolons_are_parsed_and_preserved() {
    let raw = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t-quote\r\nSUMMARY:Check\r\nATTENDEE;CN=\"Smith, John: Lead; Ops\":mailto:john@example.com\r\nRELATED-TO;X-NOTE=\"a:b;c\";RELTYPE=DEPENDS-ON:dep-1\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
    let mut task = read_task(raw);
    assert_eq!(task.dependencies, Some(vec![TaskId::from("dep-1")]));

    task.title = "Checked".to_string();
    task.dependencies = Some(vec![TaskId::from("dep-2")]);
    let written = write_task_resource(raw, &task).unwrap();

    assert!(written.contains("ATTENDEE;CN=\"Smith, John: Lead; Ops\":mailto:john@example.com"));
    assert!(!written.contains("dep-1"));
    assert_eq!(read_task(&written).dependencies, Some(vec![TaskId::from("dep-2")]));
}

#[test]
fn iso_duration_triggers_and_estimates_handle_weeks_days_hours_and_reject_invalid() {
    let raw = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t-dur\r\nSUMMARY:Dur\r\nESTIMATED-DURATION:PT1H30M\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER;RELATED=START:-P1W\r\nEND:VALARM\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER;RELATED=START:PT\r\nEND:VALARM\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
    let task = read_task(raw);
    assert_eq!(task.est, Some(90));
    assert_eq!(
        task.alarms,
        Some(vec![TaskAlarmTrigger::Relative {
            minutes_before: 7 * 24 * 60,
            anchor: TaskAlarmAnchor::Start,
        }])
    );
}

proptest! {
    #[test]
    fn arbitrary_unknown_properties_survive_the_editor(
        extra in prop::collection::vec(
            ("X-PROP-[A-Z0-9]{1,8}", "[A-Za-z0-9][ -~]{0,50}"),
            0..8,
        ),
    ) {
        let extra = extra
            .into_iter()
            .map(|(name, value)| format!("{name}:{value}"))
            .collect::<Vec<_>>();
        let raw = append_component(
            &fixture(TASK),
            &format!(
                "BEGIN:VEVENT\r\nUID:e1\r\nDTSTART;VALUE=DATE:20261020\r\nDTEND;VALUE=DATE:20261021\r\nDTSTAMP:20261020T000000Z\r\n{}\r\nEND:VEVENT\r\n",
                extra.join("\r\n")
            ),
        );
        let event = read_event_resource(&raw, HREF, COLLECTION).unwrap()[0].clone();
        let written = write_event_resource(&raw, &event).unwrap();

        let lines = content_lines(&written);
        for line in &extra {
            prop_assert!(
                lines.contains(line),
                "unknown property {line} was dropped"
            );
        }
        assert_only_owned_changed(&raw, &written, EVENT_PROPERTIES);
    }
}
