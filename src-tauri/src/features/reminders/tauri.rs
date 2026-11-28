//! Tauri adapter: poll the store and raise desktop notifications.

use chrono::Utc;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::features::calendar::{account::AccountService, Store, StoreState};

use super::policy::{SentReminders, POLL_SECONDS};

/// Remembers which reminders have already been delivered in the active window.
#[derive(Default)]
pub struct ReminderState(std::sync::Mutex<SentReminders>);

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
    let Some(accounts) = app.try_state::<AccountService>() else {
        return;
    };
    let Ok(scope) = accounts.scope().await else {
        return;
    };
    let Some(state) = app.try_state::<StoreState>() else {
        eprintln!("[reminders] store unavailable");
        return;
    };
    let Some(reminder_state) = app.try_state::<ReminderState>() else {
        eprintln!("[reminders] reminder state unavailable");
        return;
    };

    let now = Utc::now();
    let tasks = state.read(Store::tasks).await;
    let due = match reminder_state.0.lock() {
        Ok(mut sent) => sent.take_due(&scope.id, &tasks, now),
        Err(error) => {
            eprintln!("[reminders] reminder state poisoned: {error}");
            return;
        }
    };

    for reminder in due {
        if let Err(error) = app
            .notification()
            .builder()
            .title("Taskroot")
            .body(&reminder.title)
            .show()
        {
            eprintln!("[reminders] could not show `{}`: {error}", reminder.title);
        }
    }
}
