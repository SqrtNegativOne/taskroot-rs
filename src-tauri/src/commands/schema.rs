//! Column schemas for the generic task/event query UI.

#[tauri::command]
#[must_use]
pub fn get_task_schema() -> Vec<crate::domain::AppTaskColumnDef> {
    crate::domain::AppTask::get_schema()
}

#[tauri::command]
#[must_use]
pub fn get_event_schema() -> Vec<crate::domain::AppEventColumnDef> {
    crate::domain::AppEvent::get_schema()
}
