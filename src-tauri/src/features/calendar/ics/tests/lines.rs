//! The line editor: escaping, folding and preservation of unknown lines.

use super::fixtures::*;
use crate::features::calendar::ics::{read_event_resource, write_event_resource, write_task_resource};
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

proptest! {
    #[test]
    fn arbitrary_unknown_properties_survive_the_line_editor(
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
