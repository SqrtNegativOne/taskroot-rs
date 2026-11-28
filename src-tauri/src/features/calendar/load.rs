//! Apply a fetched `CalDAV` snapshot into the in-memory store.
//!
//! There is no background sync: the app loads once at startup, when an account
//! is connected, and when the user presses Refresh. Every edit still `PUT`s to
//! Radicale immediately.

use super::caldav::model::CalendarInfo;
use super::store::{ops, Store};
use ops::FetchedCollection;

/// A complete load that has not touched any store yet.
pub struct Fetched {
    pub calendars: Vec<CalendarInfo>,
    pub collections: Vec<FetchedCollection>,
}

/// Replace the store with a fetched snapshot.
pub fn apply(store: &mut Store, fetched: Fetched) {
    store.set_calendars(fetched.calendars);
    ops::apply_full_load(store, fetched.collections);
}
