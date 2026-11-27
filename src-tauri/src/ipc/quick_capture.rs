use chrono::NaiveDate;
use serde::Serialize;
use specta::Type;

use crate::error::AppError;
use crate::features::calendar::{
    app_calendar, parse_quick_capture as parse_draft, AppCalendar, CalendarComponent,
    CalendarDateTime, CalendarInfo, Store,
};
use crate::ipc::session::store_state;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QuickCapturePreview {
    pub title: String,
    pub due: Option<CalendarDateTime>,
    pub has_time: bool,
    pub tags: Vec<String>,
    pub priority: Option<crate::features::calendar::TaskPriority>,
    pub estimate_minutes: Option<i32>,
    pub list: Option<AppCalendar>,
    pub list_query: Option<String>,
    pub list_error: Option<String>,
}

/// Parse a quick-capture line and resolve its target list. Read-only.
///
/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
#[specta::specta]
pub async fn parse_quick_capture(
    app: tauri::AppHandle,
    text: String,
) -> Result<QuickCapturePreview, AppError> {
    let state = store_state(&app)?;
    let today = chrono::Local::now().date_naive();
    Ok(state.read(|store| preview(store, &text, today)).await)
}

fn preview(store: &Store, text: &str, today: NaiveDate) -> QuickCapturePreview {
    let draft = parse_draft(text, today);
    let has_time = draft.due.is_some_and(CalendarDateTime::is_timed);
    let (list, list_error) = resolve_list(store, draft.list_name.as_deref());

    QuickCapturePreview {
        title: draft.title,
        due: draft.due,
        has_time,
        tags: draft.tags,
        priority: draft.priority,
        estimate_minutes: draft.estimate_minutes,
        list,
        list_query: draft.list_name,
        list_error,
    }
}

fn resolve_list(store: &Store, query: Option<&str>) -> (Option<AppCalendar>, Option<String>) {
    let Some(query) = query else {
        return (
            store
                .default_calendar(CalendarComponent::VTodo, None)
                .map(app_calendar),
            None,
        );
    };

    let writable: Vec<&CalendarInfo> = store
        .calendars
        .iter()
        .filter(|calendar| calendar.writable && calendar.component == CalendarComponent::VTodo)
        .collect();
    let needle = query.to_lowercase();

    let exact: Vec<&CalendarInfo> = writable
        .iter()
        .copied()
        .filter(|calendar| calendar.display_name.to_lowercase() == needle)
        .collect();
    match exact.as_slice() {
        [calendar] => return (Some(app_calendar(calendar)), None),
        [] => {}
        _ => return (None, Some(ambiguous_message(query))),
    }

    let prefix: Vec<&CalendarInfo> = writable
        .iter()
        .copied()
        .filter(|calendar| calendar.href.to_lowercase().starts_with(&needle))
        .collect();
    match prefix.as_slice() {
        [calendar] => (Some(app_calendar(calendar)), None),
        [] => (None, Some(format!("Unknown task list \"{query}\""))),
        _ => (None, Some(ambiguous_message(query))),
    }
}

fn ambiguous_message(query: &str) -> String {
    format!("Multiple task lists match \"{query}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calendar(
        href: &str,
        summary: &str,
        component: CalendarComponent,
        writable: bool,
    ) -> CalendarInfo {
        CalendarInfo {
            href: href.to_string(),
            display_name: summary.to_string(),
            component,
            writable,
            ..CalendarInfo::default()
        }
    }

    fn store_with_lists() -> Store {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut store = Store::load(dir.path().to_path_buf());
        store.calendars = vec![
            calendar("/cal/tasks/", "Tasks", CalendarComponent::VTodo, true),
            calendar("/cal/work/", "Work", CalendarComponent::VTodo, true),
            calendar(
                "/cal/work-extra/",
                "Work Extra",
                CalendarComponent::VTodo,
                true,
            ),
            calendar("/cal/dup-a/", "Dup", CalendarComponent::VTodo, true),
            calendar("/cal/dup-b/", "Dup", CalendarComponent::VTodo, true),
            calendar(
                "/cal/readonly/",
                "Read Only",
                CalendarComponent::VTodo,
                false,
            ),
            calendar("/cal/events/", "Events", CalendarComponent::VEvent, true),
        ];
        store
    }

    fn summary(result: (Option<AppCalendar>, Option<String>)) -> (Option<String>, Option<String>) {
        (result.0.map(|calendar| calendar.summary), result.1)
    }

    #[test]
    fn resolves_an_exact_summary_case_insensitively() {
        let store = store_with_lists();

        assert_eq!(
            summary(resolve_list(&store, Some("work"))),
            (Some("Work".to_string()), None)
        );
    }

    #[test]
    fn resolves_a_unique_href_prefix() {
        let store = store_with_lists();

        assert_eq!(
            summary(resolve_list(&store, Some("/cal/TASKS"))),
            (Some("Tasks".to_string()), None)
        );
    }

    #[test]
    fn reports_ambiguous_prefixes_and_summaries() {
        let store = store_with_lists();

        assert_eq!(
            summary(resolve_list(&store, Some("/cal/work"))),
            (
                None,
                Some("Multiple task lists match \"/cal/work\"".to_string())
            )
        );
        assert_eq!(
            summary(resolve_list(&store, Some("dup"))),
            (None, Some("Multiple task lists match \"dup\"".to_string()))
        );
    }

    #[test]
    fn ignores_unknown_read_only_and_wrong_component_lists() {
        let store = store_with_lists();

        for query in ["missing", "Read Only", "Events"] {
            let (list, error) = resolve_list(&store, Some(query));
            assert!(list.is_none(), "{query} must not resolve to a task list");
            assert_eq!(error, Some(format!("Unknown task list \"{query}\"")));
        }
    }

    #[test]
    fn no_query_uses_the_configured_default_then_the_first_writable_list() {
        let mut store = store_with_lists();
        store.settings.default_task_list = "/cal/work/".to_string();

        assert_eq!(
            summary(resolve_list(&store, None)),
            (Some("Work".to_string()), None)
        );

        store.settings.default_task_list.clear();
        assert_eq!(
            summary(resolve_list(&store, None)),
            (Some("Tasks".to_string()), None)
        );
    }

    #[test]
    fn preview_exposes_the_draft_and_resolved_list() {
        let store = store_with_lists();
        let today = NaiveDate::from_ymd_opt(2026, 11, 19).expect("valid date");

        let result = preview(&store, "Finish PCC next wed 3pm #work p1 +work", today);

        assert_eq!(result.title, "Finish PCC");
        assert_eq!(
            result.due.map(|value| value.to_string()).as_deref(),
            Some("2026-11-25T15:00:00")
        );
        assert!(result.has_time);
        assert_eq!(result.tags, vec!["work"]);
        assert_eq!(
            result.priority,
            Some(crate::features::calendar::TaskPriority::Urgent)
        );
        assert_eq!(
            result.list.map(|calendar| calendar.summary),
            Some("Work".to_string())
        );
        assert_eq!(result.list_query.as_deref(), Some("work"));
        assert_eq!(result.list_error, None);
    }
}
