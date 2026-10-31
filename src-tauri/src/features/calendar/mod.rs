//! Calendar feature: the `CalDAV`-backed task and event domain.
//!
//! Everything a consumer needs is re-exported here. The submodules stay private
//! so the feature has one façade; `calendar` itself has no Tauri dependency.

#[allow(unreachable_pub)]
pub(crate) mod caldav;
#[allow(unreachable_pub)]
pub(crate) mod domain;
#[allow(unreachable_pub)]
pub(crate) mod ics;
#[allow(unreachable_pub)]
pub(crate) mod load;
#[allow(unreachable_pub)]
pub(crate) mod occurrences;
#[allow(unreachable_pub)]
pub(crate) mod session;
#[allow(unreachable_pub)]
pub(crate) mod store;

pub use caldav::client::CalDavClient;
pub use caldav::model::{CalendarComponent, CalendarInfo};
pub use caldav::transport::HttpClient;
pub use caldav::{account, calendars, connect, discovery};
pub use domain::*;
pub use ics::{
    create_event_resource, create_task_resource, delete_event_component, delete_task_component,
    read_event_resource, read_task_resource, write_event_resource, write_task_resource,
};
pub use load::load_into;
pub use occurrences::{expand_event_instances, single_instance};
pub use session::{Session, app_calendar, open};
pub use store::persist;
pub use store::{Resource, ResourceItems, Store, StoreState, ops, query};
