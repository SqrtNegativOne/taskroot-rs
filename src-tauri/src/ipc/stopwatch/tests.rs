//! Behavioral tests for crediting stopwatch work segments onto task `time_spent`.

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::features::calendar::{CalendarComponent, CalendarInfo, Store, StoreState};
use crate::features::stopwatch::history::{StopwatchHistoryEntry, StopwatchHistoryKind};
use crate::features::stopwatch::StopwatchLabelSource;
use crate::test_support::caldav::start_server;

use super::{aggregate_work_seconds, apply_time_spent_credits};

const TASK_COLLECTION: &str = "/cal/tasks/";

fn work_entry(task_id: Option<&str>, duration_ms: u64) -> StopwatchHistoryEntry {
    StopwatchHistoryEntry {
        kind: StopwatchHistoryKind::Work,
        task_id: task_id.map(ToString::to_string),
        label: "Work".to_string(),
        label_source: if task_id.is_some() {
            StopwatchLabelSource::Task
        } else {
            StopwatchLabelSource::Custom
        },
        started_at: 1_000,
        ended_at: 1_000_u64.saturating_add(duration_ms),
        duration_ms,
    }
}

fn break_entry(task_id: Option<&str>, duration_ms: u64) -> StopwatchHistoryEntry {
    StopwatchHistoryEntry {
        kind: StopwatchHistoryKind::Break,
        ..work_entry(task_id, duration_ms)
    }
}

#[test]
fn aggregates_work_per_task_skips_breaks_unlinked_and_sub_half_second() {
    let entries = vec![
        work_entry(Some("t1"), 400),
        work_entry(Some("t1"), 1_200), // sums to 1600 ms -> rounds to 2s
        work_entry(Some("t2"), 499),   // < 500 ms -> skipped
        work_entry(Some("t3"), 500),   // 500 ms -> rounds to 1s
        work_entry(None, 30_000),      // custom/unlinked -> ignored
        break_entry(Some("t1"), 60_000), // break -> ignored
    ];

    let credits = aggregate_work_seconds(&entries);

    assert_eq!(credits.get("t1"), Some(&2));
    assert_eq!(credits.get("t2"), None);
    assert_eq!(credits.get("t3"), Some(&1));
    assert_eq!(credits.len(), 2);
}

#[tokio::test]
async fn credits_accumulate_onto_existing_task_time_spent_over_caldav() {
    const HREF: &str = "/cal/tasks/t1.ics";
    const INITIAL: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:Focus task\r\nX-TASKROOT-TIME-SPENT:120\r\nX-APPLE-SORT-ORDER:77\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";

    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path(HREF))
        .and(header("if-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(204).insert_header("ETag", "\"v2\""))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    store.calendars = vec![CalendarInfo {
        href: TASK_COLLECTION.to_string(),
        display_name: "Tasks".to_string(),
        component: CalendarComponent::VTodo,
        writable: true,
        ..CalendarInfo::default()
    }];
    store
        .set_resource(HREF, TASK_COLLECTION, Some("\"v1\""), INITIAL)
        .unwrap();
    let state = StoreState::new(store);

    let credits = aggregate_work_seconds(&[
        work_entry(Some("t1"), 45_600), // 46s
        work_entry(Some("missing-task"), 10_000),
    ]);
    let updated = apply_time_spent_credits(&state, &client, &credits).await;

    assert!(updated);
    let task = state.read(|s| s.task("t1").unwrap().clone()).await;
    assert_eq!(task.time_spent, Some(166));
    let raw = state
        .read(|s| s.resources.get(HREF).unwrap().raw.clone())
        .await;
    assert!(raw.contains("X-TASKROOT-TIME-SPENT:166"), "{raw}");
    assert!(raw.contains("X-APPLE-SORT-ORDER:77"), "{raw}");
}
