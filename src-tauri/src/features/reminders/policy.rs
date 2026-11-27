//! Reminder policy: which `VALARM` triggers are due at a given instant.
//!
//! Reminders live in the VTODO as `VALARM` triggers (see
//! [`crate::features::calendar::ics`]). This module is pure and has no Tauri
//! dependency; [`super::tauri`] delivers the notifications.

use chrono::{DateTime, Duration, NaiveTime, Utc};

use crate::features::calendar::{
    AppTask, AppTaskStatus, TaskAlarmAnchor, TaskAlarmTrigger,
};

/// How late a trigger may be and still fire (covers sleep/suspend and the poll
/// interval). Older reminders are skipped instead of replayed.
const MISSED_WINDOW_MINUTES: i64 = 5;
pub(super) const POLL_SECONDS: u64 = 30;
const DATE_ONLY_REMINDER_HOUR: u32 = 9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueReminder {
    pub key: String,
    pub task_id: String,
    pub title: String,
    pub trigger: DateTime<Utc>,
}

/// Reminders whose trigger fell inside the missed window ending at `now`.
#[must_use]
pub fn alarms_due(tasks: &[AppTask], now: DateTime<Utc>) -> Vec<DueReminder> {
    let window_start = now
        .checked_sub_signed(Duration::minutes(MISSED_WINDOW_MINUTES))
        .unwrap_or(now);
    let mut due = Vec::new();

    for task in tasks {
        if matches!(
            task.status,
            Some(AppTaskStatus::Done | AppTaskStatus::Cancelled)
        ) {
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
        TaskAlarmTrigger::Absolute { at } => Some(*at),
        TaskAlarmTrigger::Relative {
            minutes_before,
            anchor,
        } => {
            let value = match anchor {
                TaskAlarmAnchor::Start => task.start.or(task.due)?,
                TaskAlarmAnchor::End => task.due?,
            };
            let morning = NaiveTime::from_hms_opt(DATE_ONLY_REMINDER_HOUR, 0, 0)?;
            let instant = value.to_utc_with_default_time(morning, task.timezone.as_deref());
            instant.checked_sub_signed(Duration::minutes(i64::from(*minutes_before)))
        }
    }
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
            remote_id: None,
            remote_collection_id: None,
            notes: None,
            start: None,
            due: Some(due.parse().unwrap()),
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
    fn a_completed_or_cancelled_task_never_reminds() {
        for status in [AppTaskStatus::Done, AppTaskStatus::Cancelled] {
            let mut item = task("2026-10-27T10:00:00", before(15), Some("UTC"));
            item.status = Some(status);

            let due = alarms_due(&[item], utc("2026-10-27T09:46:00Z"));
            assert!(due.is_empty(), "expected no due alarms, got {due:?}");
        }
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
        item.start = Some("2026-10-27T08:00:00".parse().unwrap());
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
            at: "2026-10-27T09:00:00Z".parse().unwrap(),
        };
        let tasks = [task("2026-10-27T10:00:00", alarm, Some("UTC"))];

        let due = alarms_due(&tasks, utc("2026-10-27T09:01:00Z"));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].trigger, utc("2026-10-27T09:00:00Z"));
    }
}
