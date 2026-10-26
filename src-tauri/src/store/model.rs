//! The in-memory resource store and its read-only snapshot.
//!
//! Radicale is the source of truth; this map is the view every command reads and
//! edits mutate before the resource is `PUT` back. `snapshot.json` is a cache
//! written after a successful load/edit and read only when startup cannot reach
//! the server.

use crate::caldav::model::{CalendarComponent, CalendarInfo};
use crate::domain::{AppEvent, AppTask};
use crate::error::AppError;
use crate::settings::AppSettings;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::persist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResource {
    pub collection: String,
    #[serde(default)]
    pub etag: Option<String>,
    pub raw: String,
}

pub type Snapshot = BTreeMap<String, SnapshotResource>;

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
    pub tokens: BTreeMap<String, String>,
    pub settings: AppSettings,
    pub ui_state: serde_json::Map<String, serde_json::Value>,
    pub offline: bool,
    pub last_synced: Option<DateTime<Utc>>,
}

impl Store {
    /// Load settings and UI state; the resource map starts empty and is filled
    /// from Radicale or `snapshot.json` by the caller.
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
            tokens: BTreeMap::new(),
            settings,
            ui_state,
            offline: false,
            last_synced: None,
        })
    }

    #[must_use]
    pub fn snapshot_path(&self) -> PathBuf {
        self.data_dir.join("snapshot.json")
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

    /// Write the whole store to `snapshot.json`.
    ///
    /// The snapshot is only the offline cache for an unreachable server, so a
    /// failed write is logged instead of failing the server-backed operation
    /// that triggered it.
    pub fn write_snapshot(&self) {
        let snapshot: Snapshot = self
            .resources
            .iter()
            .map(|(href, resource)| {
                (
                    href.clone(),
                    SnapshotResource {
                        collection: resource.collection.clone(),
                        etag: resource.etag.clone(),
                        raw: resource.raw.clone(),
                    },
                )
            })
            .collect();
        if let Err(error) = persist::write_json(&self.snapshot_path(), &snapshot) {
            eprintln!("Failed to write the offline snapshot: {error}");
        }
    }

    /// Fill the store from `snapshot.json`. A malformed resource is skipped, so
    /// one bad cache entry cannot brick an offline start.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot exists but cannot be decoded.
    pub fn load_snapshot(&mut self) -> Result<usize, AppError> {
        let Some(snapshot) = persist::read_optional_json::<Snapshot>(&self.snapshot_path())? else {
            return Ok(0);
        };
        for (href, entry) in snapshot {
            if let Err(error) = self.set_resource(&href, &entry.collection, entry.etag.as_deref(), &entry.raw)
            {
                eprintln!("Skipping unparseable snapshot resource {href}: {error}");
            }
        }
        Ok(self.resources.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    pub fn mark_synced(&mut self) {
        self.offline = false;
        self.last_synced = Some(Utc::now());
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

