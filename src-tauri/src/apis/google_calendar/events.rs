use super::types::{
    GoogleCalendarList, GoogleCalendarListEntry, GoogleColorDefinition, GoogleColors, GoogleEvent,
    GoogleEventList, GoogleEventTime,
};
use crate::domain::{AppCalendar, AppEvent, CollectionId, Color, EventId, EventStatus, RemoteId};
use color_eyre::Result;
use reqwest::Client;
use sqlx::SqlitePool;
use std::collections::HashMap;

const GOOGLE_CALENDAR_BASE: &str = "https://www.googleapis.com/calendar/v3";

enum PageError {
    /// Sync token expired: Google asks for a clean full resync.
    Gone,
    Failed(String),
}

/// Sync every calendar, using incremental tokens when available.
///
/// # Errors
///
/// Returns an error if the calendar list cannot be fetched.
pub async fn sync(pool: &SqlitePool, access_token: &str) -> Result<()> {
    let client = Client::new();
    let calendars = fetch_calendars(&client, access_token).await?;
    let colors = fetch_event_colors(&client, access_token).await;
    sync_calendar_list(pool, &calendars).await;

    for calendar in &calendars {
        if let Err(e) = sync_calendar_events(pool, &client, access_token, calendar, &colors).await {
            eprintln!("Google Calendar sync error for {}: {e}", calendar.id);
        }
    }
    Ok(())
}

async fn fetch_calendars(
    client: &Client,
    access_token: &str,
) -> Result<Vec<GoogleCalendarListEntry>> {
    let response = client
        .get(format!("{GOOGLE_CALENDAR_BASE}/users/me/calendarList"))
        .bearer_auth(access_token)
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await?;
        return Err(color_eyre::eyre::eyre!(
            "Google Calendar API error (calendarList): {err}"
        ));
    }

    let list: GoogleCalendarList = response.json().await?;
    Ok(list.items.unwrap_or_default())
}

async fn fetch_event_colors(client: &Client, access_token: &str) -> HashMap<String, String> {
    let Ok(response) = client
        .get(format!("{GOOGLE_CALENDAR_BASE}/colors"))
        .bearer_auth(access_token)
        .send()
        .await
    else {
        return HashMap::new();
    };
    if !response.status().is_success() {
        return HashMap::new();
    }
    let Ok(data) = response.json::<GoogleColors>().await else {
        return HashMap::new();
    };
    data.event
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(id, definition): (String, GoogleColorDefinition)| {
            definition.background.map(|background| (id, background))
        })
        .collect()
}

async fn sync_calendar_list(pool: &SqlitePool, calendars: &[GoogleCalendarListEntry]) {
    let mut fetched_ids = Vec::new();
    for cal in calendars {
        fetched_ids.push(cal.id.clone());
        let color = cal
            .background_color
            .clone()
            .and_then(|c| Color::try_from(c).ok());
        let app_calendar = AppCalendar {
            id: CollectionId(cal.id.clone()),
            summary: cal.summary.clone().unwrap_or_else(|| cal.id.clone()),
            color,
            is_primary: cal.primary,
            access_role: cal.access_role.clone(),
        };
        let _ = crate::db::upsert_calendar(pool, app_calendar).await;
    }

    let Ok(existing) = crate::db::get_calendars(pool).await else {
        return;
    };
    for local in existing {
        if fetched_ids.contains(&local.id.0) {
            continue;
        }
        // A calendar that vanished remotely takes its (non-dirty) events with it.
        let _ = crate::db::delete_synced_events_for_calendar(pool, &local.id.0).await;
        let _ = crate::db::delete_calendar(pool, &local.id.0).await;
    }
}

async fn sync_calendar_events(
    pool: &SqlitePool,
    client: &Client,
    access_token: &str,
    calendar: &GoogleCalendarListEntry,
    colors: &HashMap<String, String>,
) -> Result<()> {
    let mut sync_token = crate::db::get_calendar_sync_token(pool, &calendar.id)
        .await
        .ok()
        .flatten();
    let mut page_token: Option<String> = None;

    loop {
        match fetch_events_page(
            client,
            access_token,
            &calendar.id,
            sync_token.as_deref(),
            page_token.as_deref(),
        )
        .await
        {
            Ok(page) => {
                apply_events(pool, calendar, page.items, colors).await;
                if let Some(next) = page.next_page_token {
                    page_token = Some(next);
                    continue;
                }
                if let Some(token) = page.next_sync_token {
                    let _ =
                        crate::db::set_calendar_sync_token(pool, &calendar.id, Some(&token)).await;
                }
                return Ok(());
            }
            Err(PageError::Gone) => {
                let _ = crate::db::set_calendar_sync_token(pool, &calendar.id, None).await;
                let _ = crate::db::delete_synced_events_for_calendar(pool, &calendar.id).await;
                sync_token = None;
                page_token = None;
            }
            Err(PageError::Failed(message)) => return Err(color_eyre::eyre::eyre!(message)),
        }
    }
}

async fn fetch_events_page(
    client: &Client,
    access_token: &str,
    calendar_id: &str,
    sync_token: Option<&str>,
    page_token: Option<&str>,
) -> Result<GoogleEventList, PageError> {
    let encoded = urlencoding::encode(calendar_id);
    let endpoint = format!("{GOOGLE_CALENDAR_BASE}/calendars/{encoded}/events");
    let mut url = url::Url::parse(&endpoint).map_err(|e| PageError::Failed(e.to_string()))?;

    {
        let mut query = url.query_pairs_mut();
        query.append_pair("maxResults", "500");
        // Deleted entries only surface when requested; not allowed to be false
        // while using a sync token.
        query.append_pair("showDeleted", "true");
        if let Some(token) = sync_token {
            query.append_pair("syncToken", token);
        } else {
            query.append_pair("timeMin", &time_min());
            query.append_pair("orderBy", "updated");
        }
        if let Some(token) = page_token {
            query.append_pair("pageToken", token);
        }
    }

    let response = client
        .get(url.as_str())
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| PageError::Failed(e.to_string()))?;

    if response.status() == reqwest::StatusCode::GONE {
        return Err(PageError::Gone);
    }
    if !response.status().is_success() {
        let err = response.text().await.unwrap_or_default();
        return Err(PageError::Failed(format!(
            "Google Calendar API error for {calendar_id}: {err}"
        )));
    }

    response
        .json()
        .await
        .map_err(|e| PageError::Failed(e.to_string()))
}

fn time_min() -> String {
    chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(90))
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339()
}

async fn apply_events(
    pool: &SqlitePool,
    calendar: &GoogleCalendarListEntry,
    events: Option<Vec<GoogleEvent>>,
    colors: &HashMap<String, String>,
) {
    let Some(events) = events else {
        return;
    };
    for event in events {
        if let Err(e) = apply_event(pool, calendar, event, colors).await {
            eprintln!("Failed to upsert Google event: {e}");
        }
    }
}

async fn apply_event(
    pool: &SqlitePool,
    calendar: &GoogleCalendarListEntry,
    event: GoogleEvent,
    colors: &HashMap<String, String>,
) -> Result<(), sqlx::Error> {
    let local_id = local_event_id(pool, &event.id).await?;
    let local = crate::db::get_event(pool, &local_id).await?;

    let remote_updated = event
        .updated
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    if let Some(local) = &local {
        // Offline-first: a locally edited row outranks the remote copy.
        if local.dirty == Some(true) {
            return Ok(());
        }
        if local
            .updated_at
            .as_deref()
            .is_some_and(|updated| updated > remote_updated.as_str())
        {
            return Ok(());
        }
    }

    // A cancelled standalone event or recurring master is a tombstone: drop it.
    if event.status.as_deref() == Some("cancelled") && event.recurring_event_id.is_none() {
        return crate::db::delete_event(pool, local_id).await;
    }

    let Some(app_event) = build_app_event(calendar, &event, local_id, remote_updated, colors) else {
        return Ok(());
    };
    crate::db::upsert_event(pool, app_event).await
}

fn build_app_event(
    calendar: &GoogleCalendarListEntry,
    event: &GoogleEvent,
    local_id: String,
    remote_updated: String,
    colors: &HashMap<String, String>,
) -> Option<AppEvent> {
    let is_all_day = event
        .start
        .as_ref()
        .is_some_and(|start| start.date.is_some() && start.date_time.is_none());

    let start_time = extract_time(event.start.as_ref())
        .or_else(|| extract_time(event.original_start_time.as_ref()))?;
    let end_time = extract_time(event.end.as_ref()).unwrap_or_else(|| start_time.clone());

    let original_start_time = extract_time(event.original_start_time.as_ref());
    let timezone = event.start.as_ref().and_then(|start| start.time_zone.clone());
    let rrule = event.recurrence.clone().map(|lines| lines.join("\n"));

    // The master `rrule` string carries EXDATE/RDATE suppression and round-trips
    // through the write path. `exdates` is a separate, unwired column: Google
    // never populates it on read and the write path does not send it, so
    // per-occurrence deletion is a known follow-up. See
    // `docs/google-calendar-write.md`.

    let status = match event.status.as_deref() {
        Some("cancelled") => EventStatus::Cancelled,
        Some("tentative") => EventStatus::Tentative,
        _ => EventStatus::Confirmed,
    };

    let mut color_str = calendar.background_color.clone();
    if let Some(color_id) = &event.color_id {
        if let Some(background) = colors.get(color_id) {
            color_str = Some(background.clone());
        }
    }

    Some(AppEvent {
        id: EventId(local_id),
        remote_id: Some(RemoteId(event.id.clone())),
        remote_collection_id: Some(CollectionId(calendar.id.clone())),
        task_id: None,
        title: event.summary.clone().unwrap_or_else(|| "No Title".to_string()),
        description: event.description.clone(),
        start_time,
        end_time,
        rrule,
        exdates: None,
        recurring_event_id: event.recurring_event_id.clone().map(EventId),
        original_start_time,
        status: Some(status),
        updated_at: Some(remote_updated),
        color: color_str.and_then(|c| Color::try_from(c).ok()),
        etag: None,
        dirty: Some(false),
        is_all_day: Some(is_all_day),
        timezone,
    })
}

fn extract_time(time: Option<&GoogleEventTime>) -> Option<String> {
    let time = time?;
    time.date_time.clone().or_else(|| time.date.clone())
}

async fn local_event_id(pool: &SqlitePool, remote_id: &str) -> Result<String, sqlx::Error> {
    Ok(crate::db::get_event_by_remote_id(pool, remote_id)
        .await?
        .map_or_else(|| format!("google_{remote_id}"), |existing| existing.id.0))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn google_event(id: &str, status: &str) -> GoogleEvent {
        GoogleEvent {
            id: id.into(),
            summary: Some("Meeting".into()),
            description: None,
            start: Some(GoogleEventTime {
                date_time: Some("2026-09-02T09:00:00Z".into()),
                date: None,
                time_zone: Some("UTC".into()),
            }),
            end: Some(GoogleEventTime {
                date_time: Some("2026-09-02T09:30:00Z".into()),
                date: None,
                time_zone: Some("UTC".into()),
            }),
            updated: Some("2026-09-02T10:00:00Z".into()),
            status: Some(status.into()),
            color_id: None,
            recurrence: None,
            recurring_event_id: None,
            original_start_time: None,
        }
    }

    fn calendar() -> GoogleCalendarListEntry {
        GoogleCalendarListEntry {
            id: "cal-1".into(),
            summary: Some("Primary".into()),
            background_color: Some("#123456".into()),
            primary: Some(true),
            access_role: Some("owner".into()),
        }
    }

    async fn seed(pool: &SqlitePool, event: AppEvent) {
        crate::db::create_event(pool, event).await.unwrap();
    }

    fn local_event(id: &str, remote_id: &str, dirty: bool) -> AppEvent {
        AppEvent {
            id: EventId(id.into()),
            remote_id: Some(RemoteId(remote_id.into())),
            remote_collection_id: Some(CollectionId("cal-1".into())),
            task_id: None,
            title: "Local".into(),
            description: None,
            start_time: "2026-09-02T09:00:00Z".into(),
            end_time: "2026-09-02T09:30:00Z".into(),
            rrule: None,
            exdates: None,
            recurring_event_id: None,
            original_start_time: None,
            status: Some(EventStatus::Confirmed),
            updated_at: Some("2026-09-01T00:00:00Z".into()),
            color: None,
            etag: None,
            dirty: Some(dirty),
            is_all_day: Some(false),
            timezone: None,
        }
    }

    #[tokio::test]
    async fn remotely_deleted_calendar_event_is_removed_locally() {
        let pool = crate::db::init_db("sqlite::memory:").await.unwrap();
        seed(&pool, local_event("local-1", "remote-1", false)).await;

        apply_event(&pool, &calendar(), google_event("remote-1", "cancelled"), &HashMap::new())
            .await
            .unwrap();

        assert!(
            crate::db::get_event(&pool, "local-1").await.unwrap().is_none(),
            "a cancelled standalone event must be deleted locally"
        );
    }

    #[tokio::test]
    async fn cancelled_recurring_instance_is_kept_as_an_exception() {
        let pool = crate::db::init_db("sqlite::memory:").await.unwrap();

        let mut override_event = google_event("inst-1", "cancelled");
        override_event.recurring_event_id = Some("master-1".into());
        override_event.original_start_time = Some(GoogleEventTime {
            date_time: Some("2026-09-02T09:00:00Z".into()),
            date: None,
            time_zone: None,
        });

        apply_event(&pool, &calendar(), override_event, &HashMap::new())
            .await
            .unwrap();

        let stored = crate::db::get_event(&pool, "google_inst-1")
            .await
            .unwrap()
            .expect("cancelled instance must persist to suppress its master slot");
        assert_eq!(stored.status, Some(EventStatus::Cancelled));
        assert_eq!(stored.recurring_event_id, Some(EventId("master-1".into())));
    }

    #[tokio::test]
    async fn locally_dirty_event_survives_a_remote_deletion() {
        let pool = crate::db::init_db("sqlite::memory:").await.unwrap();
        seed(&pool, local_event("local-1", "remote-1", true)).await;

        apply_event(&pool, &calendar(), google_event("remote-1", "cancelled"), &HashMap::new())
            .await
            .unwrap();

        assert!(
            crate::db::get_event(&pool, "local-1").await.unwrap().is_some(),
            "offline-first: locally dirty rows win over remote deletions"
        );
    }

    #[tokio::test]
    async fn sync_token_round_trips_through_the_calendar_row() {
        let pool = crate::db::init_db("sqlite::memory:").await.unwrap();
        crate::db::upsert_calendar(
            &pool,
            AppCalendar {
                id: CollectionId("cal-1".into()),
                summary: "Primary".into(),
                color: None,
                is_primary: Some(true),
                access_role: None,
            },
        )
        .await
        .unwrap();

        assert!(crate::db::get_calendar_sync_token(&pool, "cal-1").await.unwrap().is_none());
        crate::db::set_calendar_sync_token(&pool, "cal-1", Some("token-abc"))
            .await
            .unwrap();
        assert_eq!(
            crate::db::get_calendar_sync_token(&pool, "cal-1").await.unwrap().as_deref(),
            Some("token-abc")
        );
    }
}
