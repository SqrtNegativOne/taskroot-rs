//! Calendar metadata shared by discovery, `account.json` and the frontend.
//!
//! A discovered collection is reduced to the handful of properties the app
//! needs: its href, a display name, a colour, the component kind it stores and
//! whether the current user may write to it.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The component kind stored by a calendar collection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/CalendarComponent.generated.ts"
)]
pub enum CalendarComponent {
    #[default]
    #[serde(rename = "VEVENT")]
    VEvent,
    #[serde(rename = "VTODO")]
    VTodo,
}

impl CalendarComponent {
    /// The value a CalDAV `comp` element carries.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VEvent => "VEVENT",
            Self::VTodo => "VTODO",
        }
    }
}

impl std::fmt::Display for CalendarComponent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One calendar collection discovered on the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/CalendarInfo.generated.ts")]
#[serde(rename_all = "camelCase", default)]
pub struct CalendarInfo {
    /// Absolute path on the server, e.g. `/me/2f07…/`.
    pub href: String,
    pub display_name: String,
    #[ts(optional)]
    pub color: Option<String>,
    pub component: CalendarComponent,
    pub writable: bool,
}

impl Default for CalendarInfo {
    fn default() -> Self {
        Self {
            href: String::new(),
            display_name: String::new(),
            color: None,
            component: CalendarComponent::default(),
            writable: true,
        }
    }
}
