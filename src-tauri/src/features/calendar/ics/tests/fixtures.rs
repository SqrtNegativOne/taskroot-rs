//! Shared fixtures and comparison helpers for the I1–I3 round-trip tests.

use crate::features::calendar::domain::{AppEvent, AppTask};
use crate::features::calendar::ics::{read_event_resource, read_task_resource};
use ical::tree::cst::{IcalCst, IcalItem};

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
    "PRIORITY",
    "CATEGORIES",
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
    "ESTIMATED-DURATION",
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

fn line_property_name(line: &str) -> &str {
    let end = line.find([';', ':']).unwrap_or(line.len());
    line.get(..end).unwrap_or(line)
}

/// Assert that every property outside `owned` is unchanged, in order.
pub(super) fn assert_only_owned_changed(before: &str, after: &str, owned: &[&str]) {
    let is_owned = |line: &str| {
        let name = line_property_name(line);
        owned.iter().any(|item| item.eq_ignore_ascii_case(name))
    };
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

fn collect_nested_components(cst: &IcalCst<'_>, depth: usize, out: &mut Vec<String>) {
    for item in &cst.items {
        if let IcalItem::Component(child) = item {
            if depth >= 1 {
                out.push(child.to_string());
            } else {
                collect_nested_components(child, depth.saturating_add(1), out);
            }
        }
    }
}

pub(super) fn assert_nested_preserved(before: &str, after: &str) {
    let before_cst = IcalCst::parse(before).expect("valid before ICS");
    let after_cst = IcalCst::parse(after).expect("valid after ICS");
    let mut before_nested = Vec::new();
    let mut after_nested = Vec::new();
    collect_nested_components(&before_cst, 0, &mut before_nested);
    collect_nested_components(&after_cst, 0, &mut after_nested);
    assert_eq!(
        before_nested, after_nested,
        "a nested component changed"
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
