//! Tauri adapter: poll the store and raise desktop notifications.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::features::calendar::{Store, StoreState};

use super::policy::{POLL_SECONDS, alarms_due};

/// Remembers which trigger instant was last delivered per task/alarm slot.
#[derive(Default)]
pub struct ReminderState(pub std::sync::Mutex<HashMap<String, DateTime<Utc>>>);

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
    let Some(state) = app.try_state::<StoreState>() else {
        eprintln!("[reminders] store unavailable");
        return;
    };
    let now = Utc::now();
    let tasks = state.read(Store::tasks).await;
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
