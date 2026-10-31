//! Shared fixtures and line-comparison helpers for the I1–I3 tests.

use crate::features::calendar::domain::{AppEvent, AppTask};
use crate::features::calendar::ics::lines::property_name;
use crate::features::calendar::ics::{read_event_resource, read_task_resource};

pub(super) const MOZILLA: &str = "mozilla-event-with-vtimezone.ics";
pub(super) const ALL_DAY: &str = "all-day-event.ics";
pub(super) const TASK: &str = "tasks-org-task.ics";
pub(super) const TASK_ALARM: &str = "tasks-org-task-alarm.ics";
pub(super) const HREF: &str = "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/dsh-20261019-cn.ics";
pub(super) const COLLECTION: &str = "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/";
pub(super) const MASTER_UID: &str = "dsh-20261019-cn@dsh-import";

pub(super) const EVENT_PROPERTIES: &[&str] = &[
    "SUMMARY",
    "DESCRIPTION",
    "DTSTART",
    "DTEND",
    "RRULE",
    "EXDATE",
    "STATUS",
    "COLOR",
    "X-TASKROOT-TASK-ID",
];

pub(super) const TASK_PROPERTIES: &[&str] = &[
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

pub(super) fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/features/calendar/ics/fixtures")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture {} is unreadable: {error}", path.display()))
}

pub(super) fn read_one(raw: &str) -> AppEvent {
    let events = read_event_resource(raw, HREF, COLLECTION).unwrap();
    assert_eq!(events.len(), 1, "fixture holds exactly one VEVENT");
    events.into_iter().next().unwrap()
}

pub(super) fn read_task(raw: &str) -> AppTask {
    let tasks = read_task_resource(raw, HREF, COLLECTION).unwrap();
    assert_eq!(tasks.len(), 1, "fixture holds exactly one VTODO");
    tasks.into_iter().next().unwrap()
}

/// Unfolded content lines, so comparisons are about content, not folding.
pub(super) fn content_lines(raw: &str) -> Vec<String> {
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
pub(super) fn assert_only_owned_changed(before: &str, after: &str, owned: &[&str]) {
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

/// Lines inside a component that is nested in a resource component — a `VALARM`
/// inside a `VTODO`, a `STANDARD` inside a `VTIMEZONE`. The writer owns property
/// names only, so these lines are the ones a name filter would hide.
fn nested_component_lines(raw: &str) -> Vec<String> {
    let mut enclosing = 0_usize;
    let mut lines = Vec::new();
    for line in content_lines(raw) {
        if line.starts_with("END:") {
            if enclosing >= 3 {
                lines.push(line.clone());
            }
            enclosing = enclosing.saturating_sub(1);
            continue;
        }
        if enclosing >= 3 {
            lines.push(line.clone());
        }
        if line.starts_with("BEGIN:") {
            enclosing = enclosing.saturating_add(1);
        }
    }
    lines
}

pub(super) fn assert_nested_preserved(before: &str, after: &str) {
    assert_eq!(
        nested_component_lines(before),
        nested_component_lines(after),
        "a line inside a nested component changed"
    );
}

pub(super) fn append_component(raw: &str, component: &str) -> String {
    let marker = "END:VCALENDAR\r\n";
    let position = raw.find(marker).expect("resource has a calendar end");
    let mut out = String::with_capacity(raw.len().saturating_add(component.len()));
    out.push_str(raw.get(..position).unwrap_or(raw));
    out.push_str(component);
    out.push_str(raw.get(position..).unwrap_or_default());
    out
}

pub(super) fn series_with_exception() -> String {
    append_component(
        &fixture(MOZILLA),
        "BEGIN:VEVENT\r\nUID:dsh-20261019-cn@dsh-import\r\nRECURRENCE-ID;TZID=Asia/Kolkata:20261026T110000\r\nDTSTART;TZID=Asia/Kolkata:20261028T110000\r\nDTEND;TZID=Asia/Kolkata:20261028T120000\r\nSUMMARY:CN moved\r\nDTSTAMP:20261020T000000Z\r\nEND:VEVENT\r\n",
    )
}
