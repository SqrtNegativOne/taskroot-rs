//! The in-memory resource store.
//!
//! Radicale is the source of truth; this map is the view every command reads and
//! edits mutate before the resource is `PUT` back. Nothing is cached on disk: a
//! startup or Refresh load fills the map from the server.

use crate::caldav::model::{CalendarComponent, CalendarInfo};
use crate::domain::{AppEvent, AppTask};
use crate::error::AppError;
use crate::settings::AppSettings;
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::persist;

#[derive(Debug, Clone)]
pub enum ResourceItems {
    Events(Vec<AppEvent>),
    Tasks(Vec<AppTask>),
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub href: String,
    pub collection: String,
    pub etag: Option<String>,
    pub raw: String,
    pub items: ResourceItems,
}

impl Resource {
    #[must_use]
    pub fn as_events(&self) -> Option<&[AppEvent]> {
        match &self.items {
            ResourceItems::Events(events) => Some(events),
            ResourceItems::Tasks(_) => None,
        }
    }

    #[must_use]
    pub fn as_tasks(&self) -> Option<&[AppTask]> {
        match &self.items {
            ResourceItems::Tasks(tasks) => Some(tasks),
            ResourceItems::Events(_) => None,
        }
    }
}

pub struct Store {
    pub data_dir: PathBuf,
    pub resources: BTreeMap<String, Resource>,
    pub calendars: Vec<CalendarInfo>,
    pub settings: AppSettings,
    pub ui_state: serde_json::Map<String, serde_json::Value>,
}

impl Store {
    /// Load settings and UI state; the resource map starts empty and is filled
    /// from Radicale by the caller.
    ///
    /// # Errors
    ///
    /// Returns an error if the data directory cannot be read.
    pub fn load(data_dir: PathBuf) -> Result<Self, AppError> {
        let settings = persist::read_settings(&data_dir);
        let ui_state = persist::read_ui_state(&data_dir);
        Ok(Self {
            data_dir,
            resources: BTreeMap::new(),
            calendars: Vec::new(),
            settings,
            ui_state,
        })
    }

    /// Replace (or insert) one resource and re-parse its components.
    ///
    /// # Errors
    ///
    /// Returns an error if the ICS body cannot be parsed.
    pub fn set_resource(
        &mut self,
        href: &str,
        collection: &str,
        etag: Option<&str>,
        raw: &str,
    ) -> Result<(), AppError> {
        let items = parse_items(raw, href, collection)?;
        self.resources.insert(
            href.to_string(),
            Resource {
                href: href.to_string(),
                collection: collection.to_string(),
                etag: etag.map(str::to_string),
                raw: raw.to_string(),
                items,
            },
        );
        Ok(())
    }

    pub fn remove_resource(&mut self, href: &str) {
        self.resources.remove(href);
    }

    #[must_use]
    pub fn resource_for_event(&self, id: &str) -> Option<&Resource> {
        self.resources.values().find(|resource| {
            resource
                .as_events()
                .is_some_and(|events| events.iter().any(|event| event.id.as_str() == id))
        })
    }

    #[must_use]
    pub fn resource_for_task(&self, id: &str) -> Option<&Resource> {
        self.resources.values().find(|resource| {
            resource
                .as_tasks()
                .is_some_and(|tasks| tasks.iter().any(|task| task.id.as_str() == id))
        })
    }

    #[must_use]
    pub fn event(&self, id: &str) -> Option<&AppEvent> {
        self.resources.values().find_map(|resource| {
            resource
                .as_events()
                .and_then(|events| events.iter().find(|event| event.id.as_str() == id))
        })
    }

    #[must_use]
    pub fn task(&self, id: &str) -> Option<&AppTask> {
        self.resources.values().find_map(|resource| {
            resource
                .as_tasks()
                .and_then(|tasks| tasks.iter().find(|task| task.id.as_str() == id))
        })
    }

    #[must_use]
    pub fn events(&self) -> Vec<AppEvent> {
        self.resources
            .values()
            .filter_map(Resource::as_events)
            .flatten()
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn tasks(&self) -> Vec<AppTask> {
        self.resources
            .values()
            .filter_map(Resource::as_tasks)
            .flatten()
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn calendar(&self, href: &str) -> Option<&CalendarInfo> {
        self.calendars.iter().find(|calendar| calendar.href == href)
    }

    /// The collection a new resource of `component` should target: the requested
    /// href when it is writable and of the right component, otherwise the first
    /// writable collection of that component.
    #[must_use]
    pub fn default_calendar(
        &self,
        component: CalendarComponent,
        requested: Option<&str>,
    ) -> Option<&CalendarInfo> {
        if let Some(requested) = requested {
            let match_calendar = self
                .calendars
                .iter()
                .find(|calendar| calendar.href == requested);
            if let Some(calendar) = match_calendar {
                if calendar.writable && calendar.component == component {
                    return Some(calendar);
                }
            }
        }
        self.calendars
            .iter()
            .find(|calendar| calendar.writable && calendar.component == component)
    }
}

fn parse_items(raw: &str, href: &str, collection: &str) -> Result<ResourceItems, AppError> {
    if raw.contains("BEGIN:VTODO") && !raw.contains("BEGIN:VEVENT") {
        return Ok(ResourceItems::Tasks(crate::ics::read_task_resource(
            raw, href, collection,
        )?));
    }
    Ok(ResourceItems::Events(crate::ics::read_event_resource(
        raw, href, collection,
    )?))
}
