//! I1–I3 round-trip tests over real Radicale resources.
//!
//! The fixtures are byte-for-byte copies of files under `C:\radicale`; the
//! exception tests build a multi-component resource in-memory instead (plan §13).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::lines::property_name;
use super::*;
use crate::domain::{
    AppEvent, AppTask, AppTaskStatus, EventId, EventStatus, Tag, TaskId, TaskPriority,
};
use proptest::prelude::*;

const MOZILLA: &str = "mozilla-event-with-vtimezone.ics";
const ALL_DAY: &str = "all-day-event.ics";
const TASK: &str = "tasks-org-task.ics";
const HREF: &str = "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/dsh-20261019-cn.ics";
const COLLECTION: &str = "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/";
const MASTER_UID: &str = "dsh-20261019-cn@dsh-import";

const EVENT_PROPERTIES: &[&str] = &[
    "SUMMARY",
    "DESCRIPTION",
    "DTSTART",
    "DTEND",
    "RRULE",
    "EXDATE",
    "STATUS",
    "COLOR",
];

const TASK_PROPERTIES: &[&str] = &[
    "SUMMARY",
    "DESCRIPTION",
    "DUE",
    "STATUS",
    "COMPLETED",
    "PERCENT-COMPLETE",
    "PRIORITY",
    "CATEGORIES",
    "RELATED-TO",
    "X-TASKROOT-STATUS",
    "X-TASKROOT-DEPENDENCIES",
    "X-TASKROOT-EST",
    "X-TASKROOT-CANVAS-X",
    "X-TASKROOT-CANVAS-Y",
    "X-TASKROOT-ON-CANVAS",
    "X-TASKROOT-TABS",
    "CREATED",
    "LAST-MODIFIED",
    "DTSTAMP",
];

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/ics/fixtures")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture {} is unreadable: {error}", path.display()))
}

fn read_one(raw: &str) -> AppEvent {
    let events = read_event_resource(raw, HREF, COLLECTION).unwrap();
    assert_eq!(events.len(), 1, "fixture holds exactly one VEVENT");
    events.into_iter().next().unwrap()
}

fn read_task(raw: &str) -> AppTask {
    let tasks = read_task_resource(raw, HREF, COLLECTION).unwrap();
    assert_eq!(tasks.len(), 1, "fixture holds exactly one VTODO");
    tasks.into_iter().next().unwrap()
}

/// Unfolded content lines, so comparisons are about content, not folding.
fn content_lines(raw: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for line in raw.split("\r\n") {
        match line.strip_prefix([' ', '\t']) {
            Some(continuation) => {
                if let Some(last) = lines.last_mut() {
                    last.push_str(continuation);
                }
            }
            None => lines.push(line.to_string()),
        }
    }
    lines
}

/// Assert that every line outside `owned` is unchanged, in order.
fn assert_only_owned_changed(before: &str, after: &str, owned: &[&str]) {
    let is_owned = |line: &str| owned.contains(&property_name(line));
    let kept = |raw: &str| {
        content_lines(raw)
            .into_iter()
            .filter(|line| !is_owned(line))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        kept(before),
        kept(after),
        "a non-owned line changed; owned = {owned:?}"
    );
}

fn append_component(raw: &str, component: &str) -> String {
    let marker = "END:VCALENDAR\r\n";
    let position = raw.find(marker).expect("resource has a calendar end");
    let mut out = String::with_capacity(raw.len().saturating_add(component.len()));
    out.push_str(raw.get(..position).unwrap_or(raw));
    out.push_str(component);
    out.push_str(raw.get(position..).unwrap_or_default());
    out
}

fn series_with_exception() -> String {
    append_component(
        &fixture(MOZILLA),
        "BEGIN:VEVENT\r\nUID:dsh-20261019-cn@dsh-import\r\nRECURRENCE-ID;TZID=Asia/Kolkata:20261026T110000\r\nDTSTART;TZID=Asia/Kolkata:20261028T110000\r\nDTEND;TZID=Asia/Kolkata:20261028T120000\r\nSUMMARY:CN moved\r\nDTSTAMP:20261020T000000Z\r\nEND:VEVENT\r\n",
    )
}

// ---------------------------------------------------------------- reading ----

#[test]
fn mozilla_event_reads_every_owned_field() {
    let event = read_one(&fixture(MOZILLA));

    assert_eq!(event.id, EventId::from(MASTER_UID));
    assert_eq!(event.title, "CN");
    assert_eq!(event.start_time, "2026-10-19T11:00:00");
    assert_eq!(event.end_time, "2026-10-19T12:00:00");
    assert_eq!(event.timezone.as_deref(), Some("Asia/Kolkata"));
    assert_eq!(event.is_all_day, Some(false));
    assert_eq!(event.status, Some(EventStatus::Confirmed));
    assert_eq!(
        event.rrule.as_deref(),
        Some("RRULE:FREQ=WEEKLY;UNTIL=20261218T235959Z")
    );
    assert_eq!(event.exdates, Some(vec!["2026-10-19T11:00:00".to_string()]));
    assert_eq!(event.recurring_event_id, None);
    assert_eq!(event.original_start_time, None);
    assert_eq!(event.updated_at.as_deref(), Some("2026-10-19T06:49:00Z"));
    assert_eq!(event.color, None);
    assert_eq!(event.remote_id.as_ref().map(|id| id.0.as_str()), Some(HREF));
    assert_eq!(
        event.remote_collection_id.as_ref().map(|id| id.0.as_str()),
        Some(COLLECTION)
    );
}

#[test]
fn all_day_event_stays_a_floating_date() {
    let event = read_one(&fixture(ALL_DAY));

    assert_eq!(
        event.id,
        EventId::from("dsh-20261017-first-day-of-durga@dsh-import")
    );
    assert_eq!(event.title, "First Day of Durga");
    assert_eq!(event.start_time, "2026-10-17");
    assert_eq!(event.end_time, "2026-10-18");
    assert_eq!(event.is_all_day, Some(true));
    assert_eq!(event.timezone, None);
    assert_eq!(event.rrule, None);
    assert_eq!(event.exdates, None);
    assert_eq!(event.description, None);
}

#[test]
fn tasks_org_task_maps_the_workflow_and_the_priority_bucket() {
    let task = read_task(&fixture(TASK));

    assert_eq!(task.id, TaskId::from("1004415175551987562"));
    assert_eq!(task.title, "Brush");
    assert_eq!(task.status, Some(AppTaskStatus::Todo));
    assert_eq!(task.priority, Some(TaskPriority::Low));
    assert_eq!(task.added.as_deref(), Some("2026-10-23T12:13:08Z"));
    assert_eq!(task.notes, None);
    assert_eq!(task.due, None);
    assert_eq!(task.tags, None);
    assert_eq!(task.parent_task, None);
    assert_eq!(task.dependencies, None);
    assert_eq!(
        task.remote_collection_id.as_deref().map(String::as_str),
        Some(COLLECTION)
    );
    assert_eq!(task.remote_id.as_ref().map(|id| id.0.as_str()), Some(HREF));
}

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
fn an_exception_is_identified_by_uid_at_recurrence_id() {
    let raw = series_with_exception();
    let events = read_event_resource(&raw, HREF, COLLECTION).unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, EventId::from(MASTER_UID));
    assert_eq!(
        events[1].id,
        EventId::from(format!("{MASTER_UID}@2026-10-26T11:00:00"))
    );
    assert_eq!(
        events[1].original_start_time.as_deref(),
        Some("2026-10-26T11:00:00")
    );
    assert_eq!(
        events[1]
            .recurring_event_id
            .as_ref()
            .map(|id| id.0.as_str()),
        Some(MASTER_UID)
    );
}

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

// ------------------------------------------------- surface preservation ------

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
