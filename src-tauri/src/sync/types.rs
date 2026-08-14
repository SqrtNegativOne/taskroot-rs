use serde::{Deserialize, Serialize};
use crate::domain::{AppEvent, AppTask};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncType {
    Task,
    Event,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncAction {
    Create,
    Update,
    Delete,
    Move,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SyncItemData {
    Task(AppTask),
    Event(AppEvent),
}

impl SyncItemData {
    #[must_use] 
    pub fn id(&self) -> String {
        match self {
            Self::Task(t) => t.id.clone(),
            Self::Event(e) => e.id.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncQueueItem {
    pub r#type: SyncType,
    pub action: SyncAction,
    pub item: SyncItemData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_calendar_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_fields: Option<Vec<String>>,
}
