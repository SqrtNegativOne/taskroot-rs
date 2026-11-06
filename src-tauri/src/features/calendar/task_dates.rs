//! The single boundary that turns [`AppTask`] `start`/`due` strings into
//! render-ready [`TaskDateInstance`] markers.
//!
//! Nothing outside this module interprets a task's calendar values. A task with
//! both dates yields two markers, each keyed by `<task-id>@<start|due>`.

use crate::features::calendar::domain::{
    AppTask, TaskDateInstance, TaskDateKind, TaskDateTiming,
};
use crate::features::calendar::window::RenderWindow;

/// Expand tasks into `start`/`due` markers inside the requested window.
#[must_use]
pub fn expand_task_dates(
    tasks: &[AppTask],
    range_start_iso: &str,
    range_end_iso: &str,
) -> Vec<TaskDateInstance> {
    let Some(window) = RenderWindow::from_iso(range_start_iso, range_end_iso) else {
        return Vec::new();
    };

    let mut markers = Vec::new();
    for task in tasks {
        for kind in [TaskDateKind::Start, TaskDateKind::Due] {
            let Some(value) = task_date_value(task, kind) else {
                continue;
            };
            let Some(timing) = TaskDateTiming::from_canonical(value, task.timezone.as_deref())
            else {
                continue;
            };
            if !inside(&timing, &window) {
                continue;
            }
            markers.push(TaskDateInstance {
                task_id: task.id.clone(),
                kind,
                occurrence_key: format!("{}@{}", task.id, kind.as_str()),
                title: task.title.clone(),
                timing,
                status: task.status.clone(),
                calendar_id: task.remote_collection_id.clone(),
            });
        }
    }
    markers
}

fn task_date_value(task: &AppTask, kind: TaskDateKind) -> Option<&str> {
    match kind {
        TaskDateKind::Start => task.start.as_deref(),
        TaskDateKind::Due => task.due.as_deref(),
    }
}

fn inside(timing: &TaskDateTiming, window: &RenderWindow) -> bool {
    match timing {
        TaskDateTiming::AllDay { date } => window.contains_date(*date),
        TaskDateTiming::Timed { start, .. } => window.contains_instant(*start),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::calendar::domain::{AppTaskStatus, TaskId};

    fn task(id: &str, start: Option<&str>, due: Option<&str>) -> AppTask {
        AppTask {
            id: TaskId::from(id),
            title: format!("Task {id}"),
            status: Some(AppTaskStatus::Todo),
            priority: None,
            tags: None,
            parent_task: None,
            dependencies: None,
            est: None,
            added: None,
            canvas_x: None,
            canvas_y: None,
            on_canvas: None,
            remote_id: None,
            remote_collection_id: None,
            notes: None,
            tabs: None,
            start: start.map(str::to_string),
            due: due.map(str::to_string),
            completed_at: None,
            rrule: None,
            geo: None,
            alarms: None,
            timezone: None,
            updated_at: None,
            etag: None,
        }
    }

    #[test]
    fn a_task_with_both_dates_yields_two_markers() {
        let markers = expand_task_dates(
            &[task("a", Some("2026-11-05"), Some("2026-11-07"))],
            "2026-11-01",
            "2026-11-30",
        );

        let keys: Vec<&str> = markers
            .iter()
            .map(|marker| marker.occurrence_key.as_str())
            .collect();
        assert_eq!(keys, vec!["a@start", "a@due"]);
        assert_eq!(markers[0].kind, TaskDateKind::Start);
        assert_eq!(markers[1].kind, TaskDateKind::Due);
    }

    #[test]
    fn a_task_with_the_same_start_and_due_date_still_yields_two_markers() {
        let markers = expand_task_dates(
            &[task("a", Some("2026-11-05"), Some("2026-11-05"))],
            "2026-11-05",
            "2026-11-05",
        );

        assert_eq!(markers.len(), 2);
    }

    #[test]
    fn only_the_dates_inside_the_window_are_kept() {
        let markers = expand_task_dates(
            &[task("a", Some("2026-10-01"), Some("2026-11-20"))],
            "2026-11-01",
            "2026-11-30",
        );

        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].kind, TaskDateKind::Due);
    }

    #[test]
    fn tasks_without_dates_produce_no_markers() {
        let markers = expand_task_dates(&[task("a", None, None)], "2026-11-01", "2026-11-30");
        assert_eq!(markers.len(), 0);
    }
}
