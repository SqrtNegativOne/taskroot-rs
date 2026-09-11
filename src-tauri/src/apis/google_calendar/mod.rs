mod events;
mod types;
mod write;

pub use events::sync;

use crate::domain::AppEvent;
use color_eyre::Result;
use reqwest::Client;
use types::GoogleEvent;
use write::build_write;

const GOOGLE_CALENDAR_BASE: &str = "https://www.googleapis.com/calendar/v3";

/// Create or update a Google Calendar event from the mirrored local row.
///
/// The request shape (verb, URL, body) is produced by the pure
/// [`write::build_write`] seam. Existing events are `PATCH`ed so Google keeps the
/// fields this app does not model; see that function for the full policy.
///
/// # Errors
///
/// Returns an error if the API call fails.
pub async fn publish(event: &AppEvent, access_token: &str) -> Result<String> {
    let request = build_write(event);

    let response = Client::new()
        .request(request.method, &request.url)
        .bearer_auth(access_token)
        .json(&request.body)
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

/// Move an event to another calendar.
///
/// Returns the id Google reports for the moved event (it may be re-keyed, so the
/// caller must persist it).
///
/// # Errors
///
/// Returns an error if the API call fails.
pub async fn move_event(
    remote_id: &str,
    source_calendar_id: &str,
    destination_calendar_id: &str,
    access_token: &str,
) -> Result<String> {
    let client = Client::new();
    let url = format!(
        "{GOOGLE_CALENDAR_BASE}/calendars/{}/events/{}/move?destination={}",
        urlencoding::encode(source_calendar_id),
        urlencoding::encode(remote_id),
        urlencoding::encode(destination_calendar_id),
    );

    let response = client
        .post(&url)
        .bearer_auth(access_token)
        .json(&serde_json::json!({}))
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await?;
        return Err(color_eyre::eyre::eyre!(
            "Failed to move Google Event: {err}"
        ));
    }

    let moved: GoogleEvent = response.json().await?;
    Ok(moved.id)
}

/// Finds which of `calendar_ids` currently holds `remote_id`.
///
/// Used to recover events whose local copy points at a calendar the remote copy
/// never reached (for example an interrupted move). Lookup failures are treated
/// as "not found here", so this never errors.
#[must_use]pub async fn locate_event(
    remote_id: &str,
    calendar_ids: &[String],
    access_token: &str,
) -> Option<String> {
    let client = Client::new();
    for calendar_id in calendar_ids {
        let url = format!(
            "{GOOGLE_CALENDAR_BASE}/calendars/{}/events/{}",
            urlencoding::encode(calendar_id),
            urlencoding::encode(remote_id),
        );
        let Ok(response) = client.get(&url).bearer_auth(access_token).send().await else {
            continue;
        };
        if response.status().is_success() {
            return Some(calendar_id.clone());
        }
    }
    None
}

/// Delete a Google Calendar event.
///
/// # Errors
///
/// Returns an error if the API call fails.
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
