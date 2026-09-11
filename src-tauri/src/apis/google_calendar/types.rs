use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub(super) struct GoogleCalendarList {
    pub(super) items: Option<Vec<GoogleCalendarListEntry>>,
}

#[derive(Deserialize, Debug)]
pub(super) struct GoogleCalendarListEntry {
    pub(super) id: String,
    pub(super) summary: Option<String>,
    #[serde(rename = "backgroundColor")]
    pub(super) background_color: Option<String>,
    pub(super) primary: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub(super) struct GoogleColors {
    pub(super) event: Option<HashMap<String, GoogleColorDefinition>>,
}

#[derive(Deserialize, Debug)]
pub(super) struct GoogleColorDefinition {
    pub(super) background: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct GoogleEventList {
    pub(super) items: Option<Vec<GoogleEvent>>,
    #[serde(rename = "nextPageToken")]
    pub(super) next_page_token: Option<String>,
    #[serde(rename = "nextSyncToken")]
    pub(super) next_sync_token: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct GoogleEvent {
    pub(super) id: String,
    pub(super) summary: Option<String>,
    pub(super) description: Option<String>,
    pub(super) start: Option<GoogleEventTime>,
    pub(super) end: Option<GoogleEventTime>,
    pub(super) updated: Option<String>,
    pub(super) status: Option<String>,
    #[serde(rename = "colorId")]
    pub(super) color_id: Option<String>,
    pub(super) recurrence: Option<Vec<String>>,
    #[serde(rename = "recurringEventId")]
    pub(super) recurring_event_id: Option<String>,
    #[serde(rename = "originalStartTime")]
    pub(super) original_start_time: Option<GoogleEventTime>,
}

#[derive(Deserialize, Debug)]
pub(super) struct GoogleEventTime {
    #[serde(rename = "dateTime")]
    pub(super) date_time: Option<String>,
    pub(super) date: Option<String>,
    #[serde(rename = "timeZone")]
    pub(super) time_zone: Option<String>,
}
