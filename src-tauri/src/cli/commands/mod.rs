//! Verb bodies and the helpers they share.

pub mod calendars;
pub mod events;
pub mod tasks;

use crate::features::calendar::CalendarComponent;
use crate::features::calendar::AppTask;
use crate::error::AppError;
use crate::features::calendar::Store;

pub(crate) fn find_task(store: &Store, needle: &str) -> Result<AppTask, AppError> {
    let mut matches: Vec<AppTask> = store
        .tasks()
        .into_iter()
        .filter(|task| task.id.as_str().starts_with(needle))
        .collect();
    match matches.len() {
        0 => Err(AppError::NotFound(format!(
            "no task id starts with `{needle}`"
        ))),
        1 => Ok(matches.remove(0)),
        count => Err(AppError::InvalidInput(format!(
            "`{needle}` matches {count} tasks; use more of the id"
        ))),
    }
}

/// Find a list by display name or href, of any component kind.
///
/// The caller decides which component it needs, so a task list passed to an
/// event verb fails with a message that names the mismatch instead of "not found".
pub(crate) fn resolve_list(
    store: &Store,
    needle: &str,
) -> Result<(String, CalendarComponent), AppError> {
    let found = store.calendars.iter().find(|calendar| {
        calendar.display_name.eq_ignore_ascii_case(needle) || calendar.href.starts_with(needle)
    });
    found
        .map(|calendar| (calendar.href.clone(), calendar.component))
        .ok_or_else(|| AppError::NotFound(format!("no list named `{needle}`")))
}

pub(crate) fn require_component(
    found: CalendarComponent,
    wanted: CalendarComponent,
    list: &str,
) -> Result<(), AppError> {
    if found == wanted {
        return Ok(());
    }
    Err(AppError::InvalidInput(format!(
        "`{list}` holds {found} items; this command needs a {wanted} list"
    )))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::features::calendar::CalendarInfo;
    use crate::settings::AppSettings;
    use crate::features::calendar::{Resource, ResourceItems};

    fn store_with(task_ids: &[&str], calendars: Vec<CalendarInfo>) -> Store {
        let mut store = Store {
            data_dir: PathBuf::new(),
            resources: BTreeMap::new(),
            calendars,
            settings: AppSettings::default(),
            ui_state: serde_json::Map::new(),
        };
        for id in task_ids {
            store.resources.insert(
                format!("{id}.ics"),
                Resource {
                    href: format!("{id}.ics"),
                    collection: "/tasks/".to_string(),
                    etag: None,
                    raw: String::new(),
                    items: ResourceItems::Tasks(vec![super::tasks::empty_task(
                        (*id).to_string(),
                        "task".into(),
                    )]),
                },
            );
        }
        store
    }

    #[test]
    fn an_id_prefix_resolves_to_its_task() {
        let store = store_with(&["a2a8bd04-1111", "a2a8ffff-2222"], Vec::new());

        let found = find_task(&store, "a2a8bd").unwrap();

        assert_eq!(found.id.0, "a2a8bd04-1111");
    }

    #[test]
    fn an_ambiguous_prefix_is_rejected_instead_of_picking_one() {
        let store = store_with(&["a2a8bd04-1111", "a2a8ffff-2222"], Vec::new());

        let error = find_task(&store, "a2a8").unwrap_err();

        assert!(matches!(error, AppError::InvalidInput(_)));
    }

    #[test]
    fn an_unknown_id_prefix_is_not_found() {
        let store = store_with(&["a2a8bd04-1111"], Vec::new());

        assert!(matches!(
            find_task(&store, "zz"),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn a_list_name_is_matched_ignoring_case_and_reports_its_component() {
        let store = store_with(
            &[],
            vec![
                CalendarInfo {
                    href: "/me/cal/".to_string(),
                    display_name: "SqrtCal".to_string(),
                    component: CalendarComponent::VEvent,
                    ..CalendarInfo::default()
                },
                CalendarInfo {
                    href: "/me/tasks/".to_string(),
                    display_name: "SqrtTasks".to_string(),
                    component: CalendarComponent::VTodo,
                    ..CalendarInfo::default()
                },
            ],
        );

        assert_eq!(
            resolve_list(&store, "sqrtcal").unwrap(),
            ("/me/cal/".to_string(), CalendarComponent::VEvent)
        );
        assert!(require_component(
            CalendarComponent::VEvent,
            CalendarComponent::VTodo,
            "sqrtcal"
        )
        .is_err());
        assert!(require_component(
            CalendarComponent::VTodo,
            CalendarComponent::VTodo,
            "sqrttasks"
        )
        .is_ok());
    }
}
