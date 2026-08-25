use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
struct GoogleCalendarList {
    items: Option<Vec<GoogleCalendarListEntry>>,
}

#[derive(Deserialize, Debug)]
struct GoogleCalendarListEntry {
    id: String,
    #[serde(rename = "backgroundColor")]
    background_color: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GoogleColors {
    event: Option<HashMap<String, GoogleColorDefinition>>,
}

#[derive(Deserialize, Debug)]
struct GoogleColorDefinition {
    background: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GoogleEventList {
    items: Option<Vec<GoogleEvent>>,
}

#[derive(Deserialize, Debug)]
struct GoogleEvent {
    id: String,
    summary: Option<String>,
    description: Option<String>,
    start: Option<GoogleEventTime>,
    end: Option<GoogleEventTime>,
    updated: Option<String>,
    status: Option<String>,
    #[serde(rename = "colorId")]
    color_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GoogleEventTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

/// # Errors
///
/// Returns an error if the operation fails.
#[allow(clippy::too_many_lines)]
pub async fn sync(pool: &SqlitePool, access_token: &str) -> Result<()> {
    let client = Client::new();
    let now = chrono::Utc::now().to_rfc3339();

    // 1. Fetch calendar list
    let list_response = client
        .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
        .bearer_auth(access_token)
        .send()
        .await?;

    if !list_response.status().is_success() {
        let err = list_response.text().await?;
        return Err(anyhow::anyhow!("Google Calendar API error (calendarList): {err}"));
    }

    let calendar_list: GoogleCalendarList = list_response.json().await?;
    let calendars = calendar_list.items.unwrap_or_default();

    // 2. Fetch colors
    let colors_response = client
        .get("https://www.googleapis.com/calendar/v3/colors")
        .bearer_auth(access_token)
        .send()
        .await?;
        
    let mut event_colors = HashMap::new();
    if colors_response.status().is_success() {
        if let Ok(colors_data) = colors_response.json::<GoogleColors>().await {
            if let Some(ec) = colors_data.event {
                event_colors = ec;
            }
        }
    }

    // 3. Fetch events for each calendar
    for calendar in calendars {
        let cal_id = urlencoding::encode(&calendar.id);
        let mut url = url::Url::parse(&format!("https://www.googleapis.com/calendar/v3/calendars/{cal_id}/events"))?;
        url.query_pairs_mut()
            .append_pair("timeMin", &now)
            .append_pair("maxResults", "50")
            .append_pair("singleEvents", "true")
            .append_pair("orderBy", "startTime");

        let response = client
            .get(url.as_str())
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let err = response.text().await?;
            eprintln!("Google Calendar API error for calendar {}: {}", calendar.id, err);
            continue;
        }

        let event_list: GoogleEventList = response.json().await?;

        if let Some(events) = event_list.items {
            for event in events {
                let app_event_id = format!("google_{}", event.id);
                let is_deleted = event.status.as_deref() == Some("cancelled");

                let remote_updated_at = event.updated.as_ref().map_or_else(
                    || chrono::Utc::now().timestamp_millis(),
                    |upd| {
                        chrono::DateTime::parse_from_rfc3339(upd)
                            .map_or(0, |dt| dt.timestamp_millis())
                    },
                );

                if let Ok(Some(local_event)) = crate::db::get_event(pool, &app_event_id).await {
                    if let Some(local_updated) = local_event.updated_at {
                        if local_updated > remote_updated_at {
                            continue;
                        }
                    }
                }

                if is_deleted {
                    let _ = crate::db::delete_event(pool, app_event_id).await;
                    continue;
                }

                let title = event.summary.unwrap_or_else(|| "No Title".to_string());

                let start_time = if let Some(st) = event.start {
                    st.date_time.unwrap_or_else(|| st.date.unwrap_or_default())
                } else {
                    continue;
                };

                let end_time = if let Some(et) = event.end {
                    et.date_time.unwrap_or_else(|| et.date.unwrap_or_default())
                } else {
                    start_time.clone()
                };

                let mut color = calendar.background_color.clone();
                if let Some(color_id) = &event.color_id {
                    if let Some(color_def) = event_colors.get(color_id) {
                        if let Some(bg) = &color_def.background {
                            color = Some(bg.clone());
                        }
                    }
                }

                let app_event = crate::domain::AppEvent {
                    id: app_event_id,
                    remote_id: Some(event.id),
                    remote_collection_id: Some(calendar.id.clone()),
                    task_id: None,
                    title,
                    description: event.description,
                    start_time,
                    end_time,

                    rrule: None,
                    exdates: None,
                    recurring_event_id: None,
                    original_start_time: None,
                    cancelled: Some(false),
                    updated_at: Some(remote_updated_at),
                    color,
                    deleted: Some(false),
                    etag: None,
                    dirty: Some(false),
                };

                if let Err(e) = crate::db::upsert_event(pool, app_event).await {
                    eprintln!("Failed to upsert Google event: {e}");
                }
            }
        }
    }

    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn publish(event: &crate::domain::AppEvent, access_token: &str) -> Result<String> {
    let client = Client::new();

    // We only send the fields we care about.
    let google_event = serde_json::json!({
        "summary": event.title,
        "description": event.description,
        "start": { "dateTime": event.start_time },
        "end": { "dateTime": event.end_time }
    });

    let cal_id = event.remote_collection_id.as_deref().unwrap_or("primary");
    let cal_id = urlencoding::encode(cal_id);

    let (url, method) =
        event.remote_id.as_ref().map_or_else(
            || {
                (
                    format!("https://www.googleapis.com/calendar/v3/calendars/{cal_id}/events"),
                    reqwest::Method::POST,
                )
            },
            |remote_id| {
                (
        format!("https://www.googleapis.com/calendar/v3/calendars/{cal_id}/events/{remote_id}"),
        reqwest::Method::PUT
    )
            },
        );

    let response = client
        .request(method, &url)
        .bearer_auth(access_token)
        .json(&google_event)
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await?;
        return Err(anyhow::anyhow!("Failed to publish Google Event: {err}"));
    }

    let created: GoogleEvent = response.json().await?;
    Ok(created.id)
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn delete(remote_id: &str, remote_collection_id: Option<&str>, access_token: &str) -> Result<()> {
    let client = Client::new();
    let cal_id = remote_collection_id.unwrap_or("primary");
    let cal_id = urlencoding::encode(cal_id);
    let url =
        format!("https://www.googleapis.com/calendar/v3/calendars/{cal_id}/events/{remote_id}");
    let response = client
        .request(reqwest::Method::DELETE, &url)
        .bearer_auth(access_token)
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await?;
        return Err(anyhow::anyhow!("Failed to delete Google Event: {err}"));
    }
    Ok(())
}
