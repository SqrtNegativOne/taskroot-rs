//! Column schema for the generic task query UI.

#[tauri::command]
#[specta::specta]
#[must_use]
pub fn get_task_schema() -> Vec<crate::features::calendar::AppTaskColumnDef> {
    crate::features::calendar::AppTask::get_schema()
}
