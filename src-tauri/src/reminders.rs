//! Task reminder delivery.
//!
//! Reminders live in the VTODO as `VALARM` triggers (see [`crate::ics`]). This
//! module only watches the in-memory store and raises a desktop notification when
//! a trigger becomes due while the app is running; it never writes to a resource.
//!
//! Tasks are polled rather than scheduled: the store changes under every pull and
//! edit, so a per-alarm timer would have to be rebuilt on each sync anyway.

use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::domain::{AppTask, AppTaskStatus, TaskAlarmAnchor, TaskAlarmTrigger};

/// How late a trigger may be and still fire (covers sleep/suspend and the poll
/// interval). Older reminders are skipped instead of replayed.
const MISSED_WINDOW_MINUTES: i64 = 5;
const POLL_SECONDS: u64 = 30;
const DATE_ONLY_REMINDER_HOUR: u32 = 9;

/// Remembers which trigger instant was last delivered per task/alarm slot.
#[derive(Default)]
pub struct ReminderState(pub std::sync::Mutex<HashMap<String, DateTime<Utc>>>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueReminder {
    pub key: String,
    pub task_id: String,
    pub title: String,
    pub trigger: DateTime<Utc>,
}

/// Start the reminder poller. Safe to call once from `setup`.
pub fn start_reminder_engine(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tick(&app).await;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(POLL_SECONDS)).await;
            tick(&app).await;
        }
    });
}

async fn tick(app: &AppHandle) {
    let state = match crate::commands::session::store_state(app) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("[reminders] store unavailable: {error}");
            return;
        }
    };
    let now = Utc::now();
    let tasks = state.read(crate::store::Store::tasks).await;
    let due = alarms_due(&tasks, now);

    let reminder_state = app.state::<ReminderState>();
    for reminder in due {
        let already_sent = reminder_state
            .0
            .lock()
            .is_ok_and(|sent| sent.get(&reminder.key) == Some(&reminder.trigger));
        if already_sent {
            continue;
        }

        if let Err(error) = app
            .notification()
            .builder()
            .title("Taskroot")
            .body(&reminder.title)
            .show()
        {
            eprintln!("[reminders] could not show `{}`: {error}", reminder.title);
        }

        if let Ok(mut sent) = reminder_state.0.lock() {
            sent.insert(reminder.key, reminder.trigger);
        }
    }
}

/// Reminders whose trigger fell inside the missed window ending at `now`.
#[must_use]
pub fn alarms_due(tasks: &[AppTask], now: DateTime<Utc>) -> Vec<DueReminder> {
    let window_start = now
        .checked_sub_signed(Duration::minutes(MISSED_WINDOW_MINUTES))
        .unwrap_or(now);
    let mut due = Vec::new();

    for task in tasks {
        if task.status == Some(AppTaskStatus::Done) {
            continue;
        }
        for (index, trigger) in task.alarms.iter().flatten().enumerate() {
            let Some(trigger_at) = trigger_at(task, trigger) else {
                continue;
            };
            if trigger_at <= window_start || trigger_at > now {
                continue;
            }
            due.push(DueReminder {
                key: format!("{}#{index}", task.id),
                task_id: task.id.to_string(),
                title: task.title.clone(),
                trigger: trigger_at,
            });
        }
    }

    due
}

/// The instant one `VALARM` trigger names.
fn trigger_at(task: &AppTask, trigger: &TaskAlarmTrigger) -> Option<DateTime<Utc>> {
    match trigger {
        TaskAlarmTrigger::Absolute { at } => DateTime::parse_from_rfc3339(at)
            .ok()
            .map(|instant| instant.with_timezone(&Utc)),
        TaskAlarmTrigger::Relative {
            minutes_before,
            anchor,
        } => {
            let value = match anchor {
                TaskAlarmAnchor::Start => task.start.as_deref().or(task.due.as_deref())?,
                TaskAlarmAnchor::End => task.due.as_deref()?,
            };
            let instant = wall_clock(value, task.timezone.as_deref())?;
            Some(instant.checked_sub_signed(Duration::minutes(i64::from(*minutes_before)))?)
        }
    }
}

/// Date-only anchors mean "that morning"; timed anchors keep their wall clock in
/// the task's zone.
fn wall_clock(value: &str, timezone: Option<&str>) -> Option<DateTime<Utc>> {
    if let Ok(naive) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let morning = naive.and_hms_opt(DATE_ONLY_REMINDER_HOUR, 0, 0)?;
        return crate::domain::wall_clock_to_utc(
            &morning.format("%Y-%m-%dT%H:%M:%S").to_string(),
            timezone,
        );
    }
    crate::domain::wall_clock_to_utc(value, timezone)
}

#[cfg(test)]
mod tests {

    use super::*;

    fn task(due: &str, alarm: TaskAlarmTrigger, timezone: Option<&str>) -> AppTask {
        AppTask {
            id: "task-1".into(),
            title: "Brush".to_string(),
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
            start: None,
            due: Some(due.to_string()),
            completed_at: None,
            rrule: None,
            geo: None,
            alarms: Some(vec![alarm]),
            timezone: timezone.map(ToString::to_string),
            updated_at: None,
            etag: None,
        }
    }

    fn utc(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn before(minutes_before: i32) -> TaskAlarmTrigger {
        TaskAlarmTrigger::Relative {
            minutes_before,
            anchor: TaskAlarmAnchor::End,
        }
    }

    #[test]
    fn a_relative_reminder_fires_inside_its_window_only() {
        let tasks = [task("2026-10-27T10:00:00", before(15), Some("UTC"))];

        assert_eq!(
            alarms_due(&tasks, utc("2026-10-27T09:44:00Z")).len(),
            0,
            "not yet due"
        );
        let due = alarms_due(&tasks, utc("2026-10-27T09:46:00Z"));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].trigger, utc("2026-10-27T09:45:00Z"));
        assert_eq!(due[0].title, "Brush");

        assert_eq!(
            alarms_due(&tasks, utc("2026-10-27T10:00:00Z")).len(),
            0,
            "too late to replay"
        );
    }

    #[test]
    fn a_completed_task_never_reminds() {
        let mut item = task("2026-10-27T10:00:00", before(15), Some("UTC"));
        item.status = Some(AppTaskStatus::Done);

        let due = alarms_due(&[item], utc("2026-10-27T09:46:00Z"));
        assert!(due.is_empty(), "expected no due alarms, got {due:?}");
    }

    #[test]
    fn a_date_only_due_date_reminds_at_nine_in_the_task_zone() {
        let tasks = [task("2026-10-27", before(30), Some("Asia/Kolkata"))];

        // 08:30 IST == 03:00 UTC.
        let due = alarms_due(&tasks, utc("2026-10-27T03:01:00Z"));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].trigger, utc("2026-10-27T03:00:00Z"));
    }

    #[test]
    fn a_start_anchor_prefers_the_start_and_falls_back_to_due() {
        let mut item = task("2026-10-27T10:00:00", before(10), Some("UTC"));
        item.start = Some("2026-10-27T08:00:00".to_string());
        item.alarms = Some(vec![TaskAlarmTrigger::Relative {
            minutes_before: 10,
            anchor: TaskAlarmAnchor::Start,
        }]);

        let due = alarms_due(&[item], utc("2026-10-27T07:51:00Z"));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].trigger, utc("2026-10-27T07:50:00Z"));
    }

    #[test]
    fn an_absolute_trigger_uses_its_own_instant() {
        let alarm = TaskAlarmTrigger::Absolute {
            at: "2026-10-27T09:00:00Z".to_string(),
        };
        let tasks = [task("2026-10-27T10:00:00", alarm, Some("UTC"))];

        let due = alarms_due(&tasks, utc("2026-10-27T09:01:00Z"));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].trigger, utc("2026-10-27T09:00:00Z"));
    }
}
