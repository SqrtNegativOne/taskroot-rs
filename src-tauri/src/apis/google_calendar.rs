use color_eyre::Result;
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
    summary: Option<String>,
    #[serde(rename = "backgroundColor")]
    background_color: Option<String>,
    primary: Option<bool>,
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
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
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
    recurrence: Option<Vec<String>>,
    #[serde(rename = "recurringEventId")]
    recurring_event_id: Option<String>,
    #[serde(rename = "originalStartTime")]
    original_start_time: Option<GoogleEventTime>,
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
    let time_min = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(90))
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339();

    // 1. Fetch calendar list
    let list_response = client
        .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
        .bearer_auth(access_token)
        .send()
        .await?;

    if !list_response.status().is_success() {
        let err = list_response.text().await?;
        return Err(color_eyre::eyre::eyre!(
            "Google Calendar API error (calendarList): {err}"
        ));
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

    // 3. Sync Calendars to DB and compute which ones to delete
    let mut fetched_cal_ids = Vec::new();
    for cal in &calendars {
        fetched_cal_ids.push(cal.id.clone());
        let color = cal
            .background_color
            .clone()
            .and_then(|c| crate::domain::Color::try_from(c).ok());

        let app_calendar = crate::domain::AppCalendar {
            id: crate::domain::CollectionId(cal.id.clone()),
            summary: cal.summary.clone().unwrap_or_else(|| cal.id.clone()),
            color,
            is_primary: cal.primary,
        };
        let _ = crate::db::upsert_calendar(pool, app_calendar).await;
    }

    if let Ok(existing_cals) = crate::db::get_calendars(pool).await {
        for local_cal in existing_cals {
            if !fetched_cal_ids.contains(&local_cal.id.0) {
                // Delete missing calendars and their events
                let _ = sqlx::query("DELETE FROM events WHERE remote_collection_id = ?")
                    .bind(&local_cal.id.0)
                    .execute(pool)
                    .await;
                let _ = crate::db::delete_calendar(pool, &local_cal.id.0).await;
            }
        }
    }

    // 4. Fetch events for each calendar
    for calendar in calendars {
        let cal_id = urlencoding::encode(&calendar.id);
        let mut page_token: Option<String> = None;

        loop {
            let mut url = url::Url::parse(&format!(
                "https://www.googleapis.com/calendar/v3/calendars/{cal_id}/events"
            ))?;
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("timeMin", &time_min);
                q.append_pair("maxResults", "500");
                q.append_pair("orderBy", "updated");
                if let Some(token) = &page_token {
                    q.append_pair("pageToken", token);
                }
            }

            let response = client
                .get(url.as_str())
                .bearer_auth(access_token)
                .send()
                .await?;

            if !response.status().is_success() {
                let err = response.text().await?;
                eprintln!(
                    "Google Calendar API error for calendar {}: {}",
                    calendar.id, err
                );
                break;
            }

            let mut event_list: GoogleEventList = response.json().await?;

            if let Some(events) = event_list.items.take() {
                for event in events {
                    let app_event_id = if let Ok(Some(existing_local)) =
                        crate::db::get_event_by_remote_id(pool, &event.id).await
                    {
                        existing_local.id.0
                    } else {
                        format!("google_{}", event.id)
                    };

                    let remote_updated_at = event
                        .updated
                        .clone()
                        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

                    if let Ok(Some(local_event)) = crate::db::get_event(pool, &app_event_id).await {
                        if let Some(local_updated) = &local_event.updated_at {
                            if local_updated > &remote_updated_at {
                                continue;
                            }
                        }
                    }

                    let title = event.summary.unwrap_or_else(|| "No Title".to_string());

                    let status = match event.status.as_deref() {
                        Some("cancelled") => Some(crate::domain::EventStatus::Cancelled),
                        Some("tentative") => Some(crate::domain::EventStatus::Tentative),
                        _ => Some(crate::domain::EventStatus::Confirmed),
                    };

                    let is_all_day = event
                        .start
                        .as_ref()
                        .is_some_and(|st| st.date.is_some() && st.date_time.is_none());

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

                    let original_start_time = event
                        .original_start_time
                        .map(|st| st.date_time.unwrap_or_else(|| st.date.unwrap_or_default()));

                    let rrule = event.recurrence.map(|r| r.join("\n"));

                    let mut color_str = calendar.background_color.clone();
                    if let Some(color_id) = &event.color_id {
                        if let Some(color_def) = event_colors.get(color_id) {
                            if let Some(bg) = &color_def.background {
                                color_str = Some(bg.clone());
                            }
                        }
                    }

                    let color = color_str.and_then(|c| crate::domain::Color::try_from(c).ok());

                    let app_event = crate::domain::AppEvent {
                        id: crate::domain::EventId(app_event_id),
                        remote_id: Some(crate::domain::RemoteId(event.id)),
                        remote_collection_id: Some(crate::domain::CollectionId(
                            calendar.id.clone(),
                        )),
                        task_id: None,
                        title,
                        description: event.description,
                        start_time,
                        end_time,
                        rrule,
                        exdates: None,
                        recurring_event_id: event.recurring_event_id.map(crate::domain::EventId),
                        original_start_time,
                        status,
                        updated_at: Some(remote_updated_at),
                        color,
                        etag: None,
                        dirty: Some(false),
                        is_all_day: Some(is_all_day),
                    };

                    if let Err(e) = crate::db::upsert_event(pool, app_event).await {
                        eprintln!("Failed to upsert Google event: {e}");
                    }
                }
            } // Ends if let Some(events)

            if let Some(next_token) = event_list.next_page_token {
                page_token = Some(next_token);
            } else {
                break;
            }
        } // Ends loop
    } // Ends for calendar

    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn publish(event: &crate::domain::AppEvent, access_token: &str) -> Result<String> {
    let client = Client::new();

    let is_all_day = event.is_all_day.unwrap_or(false);

    let status_str = match &event.status {
        Some(crate::domain::EventStatus::Cancelled) => "cancelled",
        Some(crate::domain::EventStatus::Tentative) => "tentative",
        _ => "confirmed",
    };

    let google_event = if is_all_day {
        let start_date = event
            .start_time
            .split('T')
            .next()
            .unwrap_or(&event.start_time);
        let end_date = event.end_time.split('T').next().unwrap_or(&event.end_time);
        serde_json::json!({
            "summary": event.title,
            "description": event.description,
            "start": { "date": start_date },
            "end": { "date": end_date },
            "status": status_str
        })
    } else {
        serde_json::json!({
            "summary": event.title,
            "description": event.description,
            "start": { "dateTime": event.start_time },
            "end": { "dateTime": event.end_time },
            "status": status_str
        })
    };

    let cal_id = event
        .remote_collection_id
        .as_deref()
        .map_or("primary", std::string::String::as_str);
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
        return Err(color_eyre::eyre::eyre!(
            "Failed to publish Google Event: {err}"
        ));
    }

    let created: GoogleEvent = response.json().await?;
    Ok(created.id)
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn delete(
    remote_id: &str,
    remote_collection_id: Option<&str>,
    access_token: &str,
) -> Result<()> {
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
        return Err(color_eyre::eyre::eyre!(
            "Failed to delete Google Event: {err}"
        ));
    }
    Ok(())
}
