//! Calendar collection lifecycle routed through [`AccountService`].
//!
//! Every mutation writes the `CalDAV` server first without holding the store
//! lock, then persists the updated collection list to `account.json` through
//! `self.repository` under `self.gate` and mirrors it into the in-memory store.

use uuid::Uuid;

use crate::error::AppError;
use crate::features::calendar::domain::Color;
use crate::features::calendar::store::{Store, StoreState};

use super::super::discovery;
use super::super::model::{CalendarComponent, CalendarInfo};
use super::service::{no_account, AccountService};
use super::AccountRegistry;

impl AccountService {
    /// Create a collection under the active account's calendar home.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] for an empty name, [`AppError::Auth`]
    /// when no account is active, and propagates `MKCALENDAR`/discovery/storage
    /// failures.
    pub async fn create_calendar(
        &self,
        state: &StoreState,
        display_name: &str,
        component: CalendarComponent,
    ) -> Result<(), AppError> {
        let name = require_name(display_name)?;
        let _guard = self.gate.write().await;
        let mut registry = self.repository.read()?;
        let profile = registry.active().ok_or_else(no_account)?.clone();
        let client = self.profile_client(&profile)?;
        let href = format!(
            "{}/{}/",
            profile.account.home_href.trim_end_matches('/'),
            Uuid::new_v4()
        );

        client.mkcalendar(&href, name, component).await?;
        let calendars = discovery::fetch_calendars(&client, &profile.account.home_href).await?;
        self.commit_calendars(&mut registry, &profile.id, state, calendars, None)
            .await
    }

    /// Rename a writable collection on the active account.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] for an empty name or a read-only
    /// target, [`AppError::NotFound`] for an unknown collection, and propagates
    /// `PROPPATCH`/storage failures.
    pub async fn rename_calendar(
        &self,
        state: &StoreState,
        href: &str,
        display_name: &str,
    ) -> Result<(), AppError> {
        let name = require_name(display_name)?;
        let _guard = self.gate.write().await;
        state.read(|store| ensure_writable(store, href)).await?;
        let mut registry = self.repository.read()?;
        let profile = registry.active().ok_or_else(no_account)?.clone();
        let client = self.profile_client(&profile)?;

        client.set_display_name(href, name).await?;
        let name = name.to_string();
        let mut calendars = state.read(|store| store.calendars.clone()).await;
        if let Some(calendar) = calendars.iter_mut().find(|calendar| calendar.href == href) {
            calendar.display_name = name;
        }
        self.commit_calendars(&mut registry, &profile.id, state, calendars, None)
            .await
    }

    /// Set (or clear, with `None`) a writable collection's colour.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] for a malformed colour or a read-only
    /// target, [`AppError::NotFound`] for an unknown collection, and propagates
    /// `PROPPATCH`/storage failures.
    pub async fn set_calendar_color(
        &self,
        state: &StoreState,
        href: &str,
        color: Option<String>,
    ) -> Result<(), AppError> {
        if let Some(color) = color.as_deref() {
            Color::try_from(color.to_string()).map_err(AppError::InvalidInput)?;
        }
        let _guard = self.gate.write().await;
        state.read(|store| ensure_writable(store, href)).await?;
        let mut registry = self.repository.read()?;
        let profile = registry.active().ok_or_else(no_account)?.clone();
        let client = self.profile_client(&profile)?;

        client.set_calendar_color(href, color.as_deref()).await?;
        let mut calendars = state.read(|store| store.calendars.clone()).await;
        if let Some(calendar) = calendars.iter_mut().find(|calendar| calendar.href == href) {
            calendar.color = color;
        }
        self.commit_calendars(&mut registry, &profile.id, state, calendars, None)
            .await
    }

    /// Delete a writable collection and every resource in it.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] for a read-only target,
    /// [`AppError::NotFound`] for an unknown collection, and propagates
    /// `DELETE`/storage failures.
    pub async fn delete_calendar(&self, state: &StoreState, href: &str) -> Result<(), AppError> {
        let _guard = self.gate.write().await;
        state.read(|store| ensure_writable(store, href)).await?;
        let mut registry = self.repository.read()?;
        let profile = registry.active().ok_or_else(no_account)?.clone();
        let client = self.profile_client(&profile)?;

        client.delete_collection(href).await?;
        let calendars = state
            .read(|store| {
                store
                    .calendars
                    .iter()
                    .filter(|calendar| calendar.href != href)
                    .cloned()
                    .collect()
            })
            .await;
        self.commit_calendars(&mut registry, &profile.id, state, calendars, Some(href))
            .await
    }

    async fn commit_calendars(
        &self,
        registry: &mut AccountRegistry,
        profile_id: &str,
        state: &StoreState,
        calendars: Vec<CalendarInfo>,
        removed_href: Option<&str>,
    ) -> Result<(), AppError> {
        if let Some(active) = registry
            .profiles
            .iter_mut()
            .find(|candidate| candidate.id == profile_id)
        {
            active.account.calendars.clone_from(&calendars);
        }
        let mut store = state.lock().await;
        self.repository.write(registry)?;
        if let Some(href) = removed_href {
            store.remove_collection_resources(href);
        }
        store.set_calendars(calendars);
        drop(store);
        Ok(())
    }
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
