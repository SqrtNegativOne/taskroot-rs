//! Pure mapping from a mirrored [`AppEvent`] to a Google Calendar write request.
//!
//! This module is deliberately network-free so the request shape can be unit
//! tested without an HTTP mock. `publish` in the parent module owns the
//! transport (auth, retries, response parsing).
//!
//! Exception instances (`recurringEventId` / `originalStartTime`) and the
//! remaining per-occurrence gaps are documented in `docs/google-calendar-write.md`.

use crate::domain::{AppEvent, EventStatus};
use reqwest::Method;
use serde_json::{Map, Value};

const GOOGLE_CALENDAR_BASE: &str = "https://www.googleapis.com/calendar/v3";
const PRIMARY_CALENDAR: &str = "primary";

/// A fully resolved Google Calendar write request: verb, URL and JSON body.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct GoogleEventWrite {
    pub(super) method: Method,
    pub(super) url: String,
    pub(super) body: Value,
}

/// Build the create/update request for `event`.
///
/// New events (no `remote_id`) are `events.insert` (`POST`). Existing events use
/// `events.patch` (`PATCH`), **not** `events.update` (`PUT`): Google's docs state
/// that `update` "does not support patch semantics and always updates the entire
/// event resource", so a `PUT` clears every field the body omits. PATCH only
/// changes the fields present in the body, which preserves everything the app
/// does not model (attendees, reminders, location, transparency, visibility,
/// conference data, ...).
///
/// `color` is **read-only**. The app stores the resolved background hex Google
/// returns from `/colors`, collapsed through the calendar fallback, but Google
/// writes colors as a `colorId` (`"1"`..`"11"`). There is no stable reverse map
/// (the palette is server-owned and several ids can share a background), so the
/// field is never written back.
#[must_use]
pub(super) fn build_write(event: &AppEvent) -> GoogleEventWrite {
    let is_create = event.remote_id.is_none();
    GoogleEventWrite {
        method: if is_create {
            Method::POST
        } else {
            Method::PATCH
        },
        url: write_url(event),
        body: build_body(event, is_create),
    }
}

fn write_url(event: &AppEvent) -> String {
    let calendar_id = urlencoding::encode(
        event
            .remote_collection_id
            .as_deref()
            .map_or(PRIMARY_CALENDAR, String::as_str),
    );
    event.remote_id.as_ref().map_or_else(
        || format!("{GOOGLE_CALENDAR_BASE}/calendars/{calendar_id}/events"),
        |remote_id| {
            format!("{GOOGLE_CALENDAR_BASE}/calendars/{calendar_id}/events/{remote_id}")
        },
    )
}

fn build_body(event: &AppEvent, is_create: bool) -> Value {
    let mut body = Map::new();
    body.insert("summary".into(), Value::String(event.title.clone()));
    body.insert(
        "description".into(),
        event
            .description
            .clone()
            .map_or(Value::Null, Value::String),
    );
    let (start, end) = timing_fields(event);
    body.insert("start".into(), start);
    body.insert("end".into(), end);
    body.insert("status".into(), Value::String(status_str(event.status.as_ref()).into()));
    body.insert("recurrence".into(), Value::Array(recurrence_lines(event)));

    // `recurringEventId` and `originalStartTime` are immutable on Google's side,
    // so they may only travel with an insert. PATCH already preserves them.
    if is_create {
        insert_exception_fields(event, &mut body);
    }

    Value::Object(body)
}

fn timing_fields(event: &AppEvent) -> (Value, Value) {
    if event.is_all_day.unwrap_or(false) {
        return (
            Value::Object(single_date(event, &event.start_time)),
            Value::Object(single_date(event, &event.end_time)),
        );
    }
    (
        Value::Object(timed(event, &event.start_time)),
        Value::Object(timed(event, &event.end_time)),
    )
}

fn single_date(event: &AppEvent, value: &str) -> Map<String, Value> {
    let date = value.split('T').next().unwrap_or(value);
    let mut fields = Map::new();
    fields.insert("date".into(), Value::String(date.to_string()));
    // Kept for symmetry with timed; Google ignores timeZone on date-only starts.
    if let Some(timezone) = &event.timezone {
        fields.insert("timeZone".into(), Value::String(timezone.clone()));
    }
    fields
}

fn timed(event: &AppEvent, value: &str) -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert("dateTime".into(), Value::String(value.to_string()));
    if let Some(timezone) = &event.timezone {
        fields.insert("timeZone".into(), Value::String(timezone.clone()));
    }
    fields
}

fn insert_exception_fields(event: &AppEvent, body: &mut Map<String, Value>) {
    let Some(recurring_id) = &event.recurring_event_id else {
        return;
    };
    body.insert(
        "recurringEventId".into(),
        Value::String(recurring_id.0.clone()),
    );
    let Some(original) = &event.original_start_time else {
        return;
    };
    let original_time = if event.is_all_day.unwrap_or(false) {
        single_date(event, original)
    } else {
        timed(event, original)
    };
    body.insert("originalStartTime".into(), Value::Object(original_time));
}

fn recurrence_lines(event: &AppEvent) -> Vec<Value> {
    event
        .rrule
        .as_deref()
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| Value::String(line.to_string()))
        .collect()
}

const fn status_str(status: Option<&EventStatus>) -> &'static str {
    match status {
        Some(EventStatus::Cancelled) => "cancelled",
        Some(EventStatus::Tentative) => "tentative",
        _ => "confirmed",
    }
}

#[cfg(test)]
mod tests;
