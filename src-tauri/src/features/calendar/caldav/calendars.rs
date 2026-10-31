//! Calendar collection lifecycle: create, rename, recolour and delete lists.
//!
//! Every operation writes the server first, then mirrors the resulting
//! collection list into the in-memory store and `account.json`, so a Refresh
//! does not have to re-discover the list.

use std::path::Path;

use uuid::Uuid;

use crate::features::calendar::domain::Color;
use crate::error::AppError;
use crate::features::calendar::store::Store;

use super::account;
use super::client::CalDavClient;
use super::discovery;
use super::model::{CalendarComponent, CalendarInfo};

/// Create a collection under the account's calendar home.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] for an empty name, [`AppError::Auth`] when
/// no account is stored, and propagates the `MKCALENDAR`/re-discovery failures.
pub async fn create(
    dir: &Path,
    client: &CalDavClient,
    store: &mut Store,
    display_name: &str,
    component: CalendarComponent,
) -> Result<(), AppError> {
    let name = require_name(display_name)?;
    let account = account::read_account(dir)?
        .ok_or_else(|| AppError::Auth("No CalDAV account is configured".to_string()))?;
    let href = format!(
        "{}/{}/",
        account.home_href.trim_end_matches('/'),
        Uuid::new_v4()
    );

    client.mkcalendar(&href, name, component).await?;
    let calendars = discovery::fetch_calendars(client, &account.home_href).await?;
    replace_calendars(dir, store, calendars)
}

/// Rename a collection.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] for an empty name or a read-only target,
/// [`AppError::NotFound`] for an unknown collection, and propagates the
/// `PROPPATCH` failure.
pub async fn rename(
    dir: &Path,
    client: &CalDavClient,
    store: &mut Store,
    href: &str,
    display_name: &str,
) -> Result<(), AppError> {
    let name = require_name(display_name)?;
    ensure_writable(store, href)?;

    client.set_display_name(href, name).await?;
    let mut calendars = store.calendars.clone();
    if let Some(calendar) = calendars.iter_mut().find(|calendar| calendar.href == href) {
        calendar.display_name = name.to_string();
    }
    replace_calendars(dir, store, calendars)
}

/// Set (or clear, with `None`) a collection's colour.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] for a malformed colour or a read-only
/// target, [`AppError::NotFound`] for an unknown collection, and propagates the
/// `PROPPATCH` failure.
pub async fn set_color(
    dir: &Path,
    client: &CalDavClient,
    store: &mut Store,
    href: &str,
    color: Option<String>,
) -> Result<(), AppError> {
    ensure_writable(store, href)?;
    if let Some(color) = color.as_deref() {
        Color::try_from(color.to_string()).map_err(AppError::InvalidInput)?;
    }

    client.set_calendar_color(href, color.as_deref()).await?;
    let mut calendars = store.calendars.clone();
    if let Some(calendar) = calendars.iter_mut().find(|calendar| calendar.href == href) {
        calendar.color = color;
    }
    replace_calendars(dir, store, calendars)
}

/// Delete a collection and every resource in it.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] for a read-only target,
/// [`AppError::NotFound`] for an unknown collection, and propagates the `DELETE`
/// failure.
pub async fn delete(
    dir: &Path,
    client: &CalDavClient,
    store: &mut Store,
    href: &str,
) -> Result<(), AppError> {
    ensure_writable(store, href)?;

    client.delete_collection(href).await?;
    store
        .resources
        .retain(|_, resource| resource.collection != href);
    let calendars = store
        .calendars
        .iter()
        .filter(|calendar| calendar.href != href)
        .cloned()
        .collect();
    replace_calendars(dir, store, calendars)?;
    Ok(())
}

fn require_name(display_name: &str) -> Result<&str, AppError> {
    let name = display_name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidInput("A list needs a name".to_string()));
    }
    Ok(name)
}

fn ensure_writable(store: &Store, href: &str) -> Result<(), AppError> {
    match store.calendar(href) {
        Some(calendar) if calendar.writable => Ok(()),
        Some(_) => Err(AppError::InvalidInput("That list is read-only".to_string())),
        None => Err(AppError::NotFound(format!("No list at {href}"))),
    }
}

fn replace_calendars(
    dir: &Path,
    store: &mut Store,
    calendars: Vec<CalendarInfo>,
) -> Result<(), AppError> {
    if let Some(mut account) = account::read_account(dir)? {
        account.calendars.clone_from(&calendars);
        account::write_account(dir, &account)?;
    }
    store.calendars = calendars;
    Ok(())
}
