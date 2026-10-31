//! Column schemas for the generic task/event query UI.

#[tauri::command]
#[must_use]
pub fn get_task_schema() -> Vec<crate::features::calendar::AppTaskColumnDef> {
    crate::features::calendar::AppTask::get_schema()
}

#[tauri::command]
#[must_use]
pub fn get_event_schema() -> Vec<crate::features::calendar::AppEventColumnDef> {
    crate::features::calendar::AppEvent::get_schema()
}
