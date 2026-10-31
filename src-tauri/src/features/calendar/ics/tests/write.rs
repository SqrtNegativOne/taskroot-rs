//! Writing: merging edits into the stored body, multi-component sets and
//! brand-new resources.

use super::fixtures::*;
use crate::features::calendar::domain::{
    AppEvent, AppTaskStatus, EventId, GeoPoint, TaskId, Tag, TaskPriority,
};
use crate::features::calendar::ics::{
    create_event_resource, create_task_resource, delete_event_component, delete_task_component,
    read_event_resource, read_task_resource, write_event_resource, write_task_resource,
};

// -------------------------------------------------------- I1 round-trips -----

#[test]
fn event_round_trip_is_byte_identical() {
    for name in [MOZILLA, ALL_DAY] {
        let raw = fixture(name);
        let event = read_one(&raw);
        assert_eq!(
            write_event_resource(&raw, &event).unwrap(),
            raw,
            "{name} must survive an unedited write unchanged"
        );
    }
}

#[test]
fn an_event_task_link_survives_a_write_and_can_be_cleared() {
    let raw = fixture(MOZILLA);
    let mut event = read_one(&raw);
    assert_eq!(event.task_id, None);

    event.task_id = Some(TaskId::from("task-1"));
    let written = write_event_resource(&raw, &event).unwrap();
    assert!(
        written.contains("\r\nX-TASKROOT-TASK-ID:task-1\r\n"),
        "link must be written: {written}"
    );
    assert_eq!(read_one(&written).task_id, Some(TaskId::from("task-1")));

    let created = create_event_resource(&event).unwrap();
    assert!(
        created.contains("X-TASKROOT-TASK-ID:task-1"),
        "a newly built event keeps the link: {created}"
    );

    event.task_id = None;
    let cleared = write_event_resource(&written, &event).unwrap();
    assert!(
        !cleared.contains("X-TASKROOT-TASK-ID"),
        "clearing the link must remove the property: {cleared}"
    );
    assert_eq!(read_one(&cleared).task_id, None);
}

#[test]
fn task_round_trip_only_touches_owned_lines() {
    let raw = fixture(TASK);
    let task = read_task(&raw);

    let written = write_task_resource(&raw, &task).unwrap();

    assert_only_owned_changed(&raw, &written, TASK_PROPERTIES);
    assert!(
        written.contains("X-APPLE-SORT-ORDER:813839139"),
        "unknown X- properties must survive"
    );
    assert!(
        written.contains("PRIORITY:7"),
        "PRIORITY:9 is the Low bucket, written back as 7"
    );
    assert!(written.contains("STATUS:NEEDS-ACTION"));
}

#[test]
fn moving_a_timed_event_only_rewrites_dtstart_and_dtend() {
    let raw = fixture(MOZILLA);
    let mut event = read_one(&raw);
    event.start_time = "2026-10-20T15:30:00".to_string();
    event.end_time = "2026-10-20T16:30:00".to_string();

    let written = write_event_resource(&raw, &event).unwrap();

    assert_only_owned_changed(&raw, &written, EVENT_PROPERTIES);
    assert!(written.contains("DTSTART;TZID=Asia/Kolkata:20261020T153000"));
    assert!(written.contains("DTEND;TZID=Asia/Kolkata:20261020T163000"));
    assert!(
        written.contains("BEGIN:VTIMEZONE"),
        "VTIMEZONE must survive"
    );
    assert!(
        written.contains("X-MOZ-GENERATION:1"),
        "X-MOZ-GENERATION must survive"
    );
    assert!(
        written.contains("TRANSP:OPAQUE"),
        "unknown props must survive"
    );
    assert_eq!(read_one(&written).start_time, "2026-10-20T15:30:00");
}

#[test]
fn retitling_an_all_day_event_keeps_its_categories_line() {
    let raw = fixture(ALL_DAY);
    let mut event = read_one(&raw);
    event.title = "Durga Puja".to_string();

    let written = write_event_resource(&raw, &event).unwrap();

    assert_eq!(
        written.replace("SUMMARY:Durga Puja", "SUMMARY:First Day of Durga"),
        raw
    );
    assert!(written.contains("CATEGORIES:Holiday"));
}

#[test]
fn exdates_become_exdate_lines_in_the_event_zone() {
    let raw = fixture(MOZILLA);
    let mut event = read_one(&raw);
    event.exdates = Some(vec![
        "2026-10-19T11:00:00".to_string(),
        "2026-10-26T11:00:00".to_string(),
    ]);

    let written = write_event_resource(&raw, &event).unwrap();

    assert!(written.contains(
        "EXDATE;TZID=Asia/Kolkata:20261019T110000\r\nEXDATE;TZID=Asia/Kolkata:20261026T110000\r\n"
    ));
    assert_eq!(
        read_one(&written).exdates,
        Some(vec![
            "2026-10-19T11:00:00".to_string(),
            "2026-10-26T11:00:00".to_string(),
        ])
    );
}

#[test]
fn clearing_every_exdate_removes_the_property() {
    let raw = fixture(MOZILLA);
    let mut event = read_one(&raw);
    event.exdates = None;

    let written = write_event_resource(&raw, &event).unwrap();

    assert!(!written.contains("EXDATE"));
    assert!(written.contains("RRULE:FREQ=WEEKLY"));
}

// ---------------------------------------------- I2 multi-component sets -----

#[test]
fn editing_an_exception_preserves_the_master_and_other_exceptions() {
    let raw = series_with_exception();
    let mut exception = read_event_resource(&raw, HREF, COLLECTION).unwrap()[1].clone();
    exception.title = "CN moved again".to_string();

    let written = write_event_resource(&raw, &exception).unwrap();

    assert_only_owned_changed(&raw, &written, EVENT_PROPERTIES);
    assert!(written.contains("SUMMARY:CN\r\n"), "master summary intact");
    assert!(written.contains("SUMMARY:CN moved again"));
    assert!(written.contains("RECURRENCE-ID;TZID=Asia/Kolkata:20261026T110000"));
    assert!(written.contains("BEGIN:VTIMEZONE"));
    assert!(!written.contains("SUMMARY:CN moved\r\n"));
    assert_eq!(
        read_event_resource(&written, HREF, COLLECTION)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn deleting_an_exception_restores_the_master_resource_exactly() {
    let raw = series_with_exception();
    let exception = read_event_resource(&raw, HREF, COLLECTION).unwrap()[1].clone();

    let written = delete_event_component(&raw, &exception)
        .unwrap()
        .expect("the master remains");

    assert_eq!(
        written,
        fixture(MOZILLA),
        "the master resource must be untouched"
    );
    assert_eq!(
        read_event_resource(&written, HREF, COLLECTION)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn deleting_a_master_takes_the_whole_series_and_returns_none() {
    let raw = series_with_exception();
    let master = read_event_resource(&raw, HREF, COLLECTION).unwrap()[0].clone();

    assert_eq!(delete_event_component(&raw, &master).unwrap(), None);
}

#[test]
fn deleting_the_only_component_returns_none() {
    let raw = fixture(ALL_DAY);
    let event = read_one(&raw);

    assert_eq!(delete_event_component(&raw, &event).unwrap(), None);
}

#[test]
fn deleting_a_task_returns_none_for_a_single_vtodo_resource() {
    let raw = fixture(TASK);
    let task = read_task(&raw);

    assert_eq!(delete_task_component(&raw, &task).unwrap(), None);
}

// ------------------------------------------------------------ task writes ----

#[test]
fn completing_a_task_writes_completed_and_percent_complete() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.status = Some(AppTaskStatus::Done);

    let written = write_task_resource(&raw, &task).unwrap();

    assert_only_owned_changed(&raw, &written, TASK_PROPERTIES);
    assert!(written.contains("STATUS:COMPLETED"));
    assert!(written.contains("PERCENT-COMPLETE:100"));
    assert!(
        written.contains("COMPLETED:"),
        "completion timestamp missing"
    );
    assert!(!written.contains("STATUS:NEEDS-ACTION"));
    assert!(written.contains("X-APPLE-SORT-ORDER:813839139"));
    assert_eq!(read_task(&written).status, Some(AppTaskStatus::Done));
}

#[test]
fn reopening_a_task_clears_the_completion_properties() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.status = Some(AppTaskStatus::Done);
    let completed = write_task_resource(&raw, &task).unwrap();

    task.status = Some(AppTaskStatus::NextUp);
    let reopened = write_task_resource(&completed, &task).unwrap();

    assert!(reopened.contains("STATUS:NEEDS-ACTION"));
    assert!(reopened.contains("X-TASKROOT-STATUS:next-up"));
    assert!(!reopened.contains("COMPLETED"));
    assert!(!reopened.contains("PERCENT-COMPLETE"));
}

#[test]
fn setting_a_date_only_due_date_keeps_the_value_date_form() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.due = Some("2026-11-05".to_string());

    let written = write_task_resource(&raw, &task).unwrap();

    assert_only_owned_changed(&raw, &written, TASK_PROPERTIES);
    assert!(written.contains("DUE;VALUE=DATE:20261105"));
    assert_eq!(read_task(&written).due.as_deref(), Some("2026-11-05"));
}

#[test]
fn a_timed_due_date_stays_a_date_time() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.due = Some("2026-11-05T18:30:00".to_string());

    let written = write_task_resource(&raw, &task).unwrap();

    assert!(written.contains("DUE:20261105T183000\r\n"));
    assert_eq!(
        read_task(&written).due.as_deref(),
        Some("2026-11-05T18:30:00")
    );
}

#[test]
fn attaching_a_subtask_writes_related_to_parent() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.parent_task = Some(TaskId::from("1004415175551987000"));

    let written = write_task_resource(&raw, &task).unwrap();

    assert_only_owned_changed(&raw, &written, TASK_PROPERTIES);
    assert!(written.contains("RELATED-TO;RELTYPE=PARENT:1004415175551987000"));
    assert_eq!(
        read_task(&written).parent_task,
        Some(TaskId::from("1004415175551987000"))
    );
}

#[test]
fn a_depends_on_relation_is_not_treated_as_the_parent() {
    let raw = append_component(
        &fixture(TASK),
        "BEGIN:VTODO\r\nUID:other\r\nDTSTAMP:20261020T000000Z\r\nSUMMARY:Other\r\nRELATED-TO;RELTYPE=DEPENDS-ON:parent-uid\r\nEND:VTODO\r\n",
    );
    let tasks = read_task_resource(&raw, HREF, COLLECTION).unwrap();

    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[1].parent_task, None);

    let mut task = tasks[1].clone();
    task.parent_task = Some(TaskId::from("real-parent"));
    let written = write_task_resource(&raw, &task).unwrap();

    assert!(written.contains("RELATED-TO;RELTYPE=DEPENDS-ON:parent-uid"));
    assert!(written.contains("RELATED-TO;RELTYPE=PARENT:real-parent"));
}

#[test]
fn tags_dependencies_and_canvas_fields_round_trip() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.tags = Some(vec![Tag {
        id: "lists".to_string(),
        name: "lists".to_string(),
        color: None,
    }]);
    task.dependencies = Some(vec![TaskId::from("a"), TaskId::from("b")]);
    task.est = Some(25);
    task.canvas_x = Some(12.5);
    task.canvas_y = Some(-3.0);
    task.on_canvas = Some(true);
    task.tabs = Some("work".to_string());

    let written = write_task_resource(&raw, &task).unwrap();

    assert_only_owned_changed(&raw, &written, TASK_PROPERTIES);
    let reread = read_task(&written);
    assert_eq!(
        reread
            .tags
            .map(|tags| tags.into_iter().map(|tag| tag.name).collect::<Vec<_>>()),
        Some(vec!["lists".to_string()])
    );
    assert_eq!(
        reread.dependencies,
        Some(vec![TaskId::from("a"), TaskId::from("b")])
    );
    assert_eq!(reread.est, Some(25));
    assert_eq!(reread.canvas_x, Some(12.5));
    assert_eq!(reread.canvas_y, Some(-3.0));
    assert_eq!(reread.on_canvas, Some(true));
    assert_eq!(reread.tabs.as_deref(), Some("work"));
}

// ---------------------------------------------------- tasks.org VTODO -------

#[test]
fn a_task_without_a_priority_gets_no_priority_line() {
    let raw = fixture(TASK_ALARM);
    let mut task = read_task(&raw);
    task.priority = None;

    let written = write_task_resource(&raw, &task).unwrap();

    assert!(!written.contains("PRIORITY"), "tasks.org omits 0");
    assert_eq!(read_task(&written).priority, None);
}

#[test]
fn every_priority_bucket_writes_the_tasks_org_value() {
    let raw = fixture(TASK);
    let cases = [
        (TaskPriority::Urgent, "PRIORITY:1"),
        (TaskPriority::High, "PRIORITY:3"),
        (TaskPriority::Medium, "PRIORITY:5"),
        (TaskPriority::Low, "PRIORITY:7"),
    ];
    for (priority, expected) in cases {
        let mut task = read_task(&raw);
        task.priority = Some(priority.clone());
        let written = write_task_resource(&raw, &task).unwrap();
        assert!(written.contains(expected), "{priority:?} → {expected}");
        assert_eq!(read_task(&written).priority, Some(priority));
    }
}

#[test]
fn editing_a_completed_task_keeps_its_completion_time() {
    let raw = fixture(TASK_ALARM);
    let mut task = read_task(&raw);
    task.status = Some(AppTaskStatus::Done);
    let completed = write_task_resource(&raw, &task).unwrap();
    let stamp = read_task(&completed)
        .completed_at
        .expect("completing writes COMPLETED");

    let mut edited = read_task(&completed);
    edited.title = "Watered".to_string();
    let written = write_task_resource(&completed, &edited).unwrap();

    assert_eq!(
        read_task(&written).completed_at.as_deref(),
        Some(stamp.as_str()),
        "a retitle must not move the completion timestamp"
    );
}

#[test]
fn starting_a_task_writes_dtstart_in_the_task_zone() {
    let raw = fixture(TASK_ALARM);
    let mut task = read_task(&raw);
    task.start = Some("2026-11-06T07:30:00".to_string());

    let written = write_task_resource(&raw, &task).unwrap();

    assert!(written.contains("DTSTART;TZID=Asia/Kolkata:20261106T073000"));
    assert!(written.contains("DUE;TZID=Asia/Kolkata:20261105T180000"));
    assert_eq!(
        read_task(&written).start.as_deref(),
        Some("2026-11-06T07:30:00")
    );
}

#[test]
fn a_date_only_start_uses_value_date() {
    let raw = fixture(TASK_ALARM);
    let mut task = read_task(&raw);
    task.start = Some("2026-11-04".to_string());

    let written = write_task_resource(&raw, &task).unwrap();

    assert!(written.contains("DTSTART;VALUE=DATE:20261104"));
    assert_eq!(read_task(&written).start.as_deref(), Some("2026-11-04"));
}

#[test]
fn an_rrule_is_read_and_written_back_unchanged() {
    let raw = fixture(TASK_ALARM);
    let mut task = read_task(&raw);
    assert_eq!(task.rrule.as_deref(), Some("RRULE:FREQ=DAILY;COUNT=10"));

    task.rrule = Some("RRULE:FREQ=WEEKLY;BYDAY=MO,WE".to_string());
    let written = write_task_resource(&raw, &task).unwrap();

    assert!(written.contains("RRULE:FREQ=WEEKLY;BYDAY=MO,WE"));
    assert!(!written.contains("COUNT=10"));
    assert_eq!(
        read_task(&written).rrule.as_deref(),
        Some("RRULE:FREQ=WEEKLY;BYDAY=MO,WE")
    );

    task.rrule = None;
    let cleared = write_task_resource(&raw, &task).unwrap();
    assert!(!cleared.contains("RRULE"));
}

#[test]
fn a_utc_due_date_keeps_its_instant() {
    let raw = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:a\r\nDUE:20261105T123000Z\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
    let task = read_task(raw);

    assert_eq!(task.due.as_deref(), Some("2026-11-05T12:30:00"));
    assert_eq!(task.timezone.as_deref(), Some("UTC"));

    let written = write_task_resource(raw, &task).unwrap();
    assert!(written.contains("DUE;TZID=UTC:20261105T123000"));
    assert_eq!(
        read_task(&written).due.as_deref(),
        Some("2026-11-05T12:30:00")
    );
}

#[test]
fn a_geo_point_round_trips() {
    let raw = fixture(TASK);
    let mut task = read_task(&raw);
    task.geo = Some(GeoPoint {
        latitude: -33.868_82,
        longitude: 151.209_3,
    });

    let written = write_task_resource(&raw, &task).unwrap();

    assert!(written.contains("GEO:-33.86882;151.2093"));
    assert_eq!(read_task(&written).geo, task.geo);

    task.geo = None;
    let cleared = write_task_resource(&written, &task).unwrap();
    assert!(!cleared.contains("GEO"));
}

// -------------------------------------------------------------- creating -----

#[test]
fn create_event_resource_uses_the_dto_id_as_uid() {
    let template = read_one(&fixture(MOZILLA));
    let created = create_event_resource(&AppEvent {
        id: EventId::from("dsh-20261019-cn@dsh-import@taskroot-import"),
        ..template
    })
    .unwrap();

    assert!(created.starts_with("BEGIN:VCALENDAR\r\n"));
    assert_eq!(
        created
            .lines()
            .filter(|line| line.starts_with("UID:"))
            .collect::<Vec<_>>(),
        vec!["UID:dsh-20261019-cn@dsh-import@taskroot-import"],
        "a created resource must contain exactly the DTO's UID, not an auto-generated one"
    );
    assert_eq!(
        created
            .lines()
            .filter(|line| line.starts_with("DTSTAMP:"))
            .count(),
        1,
        "a created resource must contain exactly one DTSTAMP"
    );
    assert!(created.contains("DTSTART;TZID=Asia/Kolkata:20261019T110000"));
    assert!(created.contains("EXDATE;TZID=Asia/Kolkata:20261019T110000"));
    assert!(created.contains("RRULE:FREQ=WEEKLY;UNTIL=20261218T235959Z"));
    assert!(
        !created.contains("VTIMEZONE"),
        "new resources omit VTIMEZONE"
    );
    assert!(created.ends_with("END:VCALENDAR\r\n"));

    let reread = read_one(&created);
    assert_eq!(reread.start_time, "2026-10-19T11:00:00");
    assert_eq!(reread.exdates.unwrap().len(), 1);
}

#[test]
fn create_event_resource_emits_an_all_day_date_pair() {
    let created = create_event_resource(&read_one(&fixture(ALL_DAY))).unwrap();

    assert!(created.contains("DTSTART;VALUE=DATE:20261017"));
    assert!(created.contains("DTEND;VALUE=DATE:20261018"));
    assert_eq!(read_one(&created).start_time, "2026-10-17");
}

#[test]
fn create_task_resource_round_trips_through_the_reader() {
    let mut task = read_task(&fixture(TASK));
    task.title = "Floss".to_string();
    task.status = Some(AppTaskStatus::Done);
    task.due = Some("2026-11-05".to_string());

    let created = create_task_resource(&task).unwrap();

    assert!(created.contains("BEGIN:VTODO"));
    assert_eq!(
        created
            .lines()
            .filter(|line| line.starts_with("UID:"))
            .collect::<Vec<_>>(),
        vec!["UID:1004415175551987562"]
    );
    assert!(created.contains("DUE;VALUE=DATE:20261105"));
    assert_eq!(
        created
            .lines()
            .filter(|line| line.starts_with("DTSTAMP:"))
            .count(),
        1
    );
    let reread = read_task(&created);
    assert_eq!(reread.title, "Floss");
    assert_eq!(reread.status, Some(AppTaskStatus::Done));
}

// ------------------------------------------------------------- insertion -----

#[test]
fn writing_an_unknown_exception_inserts_a_component_after_the_master() {
    let raw = fixture(MOZILLA);
    let master = read_one(&raw);
    let exception = AppEvent {
        id: EventId::from(format!("{MASTER_UID}@2026-11-02T11:00:00")),
        recurring_event_id: Some(EventId::from(MASTER_UID)),
        original_start_time: Some("2026-11-02T11:00:00".to_string()),
        start_time: "2026-11-04T11:00:00".to_string(),
        end_time: "2026-11-04T12:00:00".to_string(),
        title: "CN moved".to_string(),
        ..master
    };

    let written = write_event_resource(&raw, &exception).unwrap();

    let untouched = raw.strip_suffix("END:VCALENDAR\r\n").unwrap();
    assert!(
        written.starts_with(untouched),
        "the fetched body must be preserved verbatim"
    );
    assert!(written.ends_with("END:VCALENDAR\r\n"));
    assert!(written.contains("RECURRENCE-ID;TZID=Asia/Kolkata:20261102T110000"));
    assert!(written.contains("DTSTART;TZID=Asia/Kolkata:20261104T110000"));

    let events = read_event_resource(&written, HREF, COLLECTION).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].id, exception.id);
    assert_eq!(events[1].start_time, "2026-11-04T11:00:00");
    assert_eq!(
        events[1].original_start_time.as_deref(),
        Some("2026-11-02T11:00:00")
    );
}

#[test]
fn inserting_a_component_for_a_foreign_uid_is_rejected() {
    let raw = fixture(MOZILLA);
    let template = read_one(&fixture(ALL_DAY));
    let foreign = AppEvent {
        id: EventId::from("someone-else"),
        ..template
    };

    let error = write_event_resource(&raw, &foreign).unwrap_err();
    assert_eq!(error.code(), "invalid-input");
}
