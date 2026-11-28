//! Reminder policy: which `VALARM` triggers are due at a given instant and
//! bounded deduplication across poll ticks.
//!
//! Reminders live in the VTODO as `VALARM` triggers (see
//! [`crate::features::calendar::ics`]). This module is pure and has no Tauri
//! dependency; [`super::tauri`] delivers the notifications.

use std::collections::HashSet;

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
    pub task_id: String,
    pub title: String,
    pub trigger: DateTime<Utc>,
}

/// Stable deduplication identity for a delivered reminder.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReminderKey {
    pub scope_id: String,
    pub task_id: String,
    pub trigger: DateTime<Utc>,
}

/// Bounded record of reminders delivered inside the active missed window.
#[derive(Debug, Default)]
pub struct SentReminders {
    delivered: HashSet<ReminderKey>,
}

impl SentReminders {
    /// Prunes entries older than the missed window and returns reminders due at
    /// `now` that have not yet been delivered for `scope_id`, recording them as
    /// delivered.
    pub fn take_due(
        &mut self,
        scope_id: &str,
        tasks: &[AppTask],
        now: DateTime<Utc>,
    ) -> Vec<DueReminder> {
        self.prune(now);
        alarms_due(tasks, now)
            .into_iter()
            .filter(|reminder| {
                self.delivered.insert(ReminderKey {
                    scope_id: scope_id.to_string(),
                    task_id: reminder.task_id.clone(),
                    trigger: reminder.trigger,
                })
            })
            .collect()
    }

    /// Drops entries whose trigger is no longer inside the missed window at
    /// `now` and therefore cannot be returned by [`alarms_due`].
    pub fn prune(&mut self, now: DateTime<Utc>) {
        let window_start = missed_window_start(now);
        self.delivered.retain(|key| key.trigger > window_start);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.delivered.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.delivered.is_empty()
    }
}

/// Reminders whose trigger fell inside the missed window ending at `now`.
#[must_use]
pub fn alarms_due(tasks: &[AppTask], now: DateTime<Utc>) -> Vec<DueReminder> {
    let window_start = missed_window_start(now);
    let mut due = Vec::new();

    for task in tasks {
        if matches!(
            task.status,
            Some(AppTaskStatus::Done | AppTaskStatus::Cancelled)
        ) {
            continue;
        }
        for trigger in task.alarms.iter().flatten() {
            let Some(trigger_at) = trigger_at(task, trigger) else {
                continue;
            };
            if trigger_at <= window_start || trigger_at > now {
                continue;
            }
            due.push(DueReminder {
                task_id: task.id.to_string(),
                title: task.title.clone(),
                trigger: trigger_at,
            });
        }
    }

    due
}

fn missed_window_start(now: DateTime<Utc>) -> DateTime<Utc> {
    now.checked_sub_signed(Duration::minutes(MISSED_WINDOW_MINUTES))
        .unwrap_or(now)
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
mod tests;
