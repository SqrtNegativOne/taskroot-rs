//! Tag renames: rewrite every task and event that carries the old name.

use std::collections::HashSet;

use crate::error::AppError;
use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::domain::{EventPatch, Tag, TaskPatch};

use super::super::model::Store;
use super::super::state::StoreState;
use super::{update_event, update_task};

/// Rename one tag across every task and event in the store.
///
/// The new name is trimmed and must be non-empty. A rename that would collide
/// with an existing spelling merges the two groups instead of duplicating it.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] for an unusable name or a read-only
/// collection, and propagates the first `CalDAV` write failure. Renames are not
/// transactional across resources, so a transport failure can leave earlier
/// resources renamed.
pub async fn rename_tag(
    state: &StoreState,
    client: &CalDavClient,
    from: &str,
    to: &str,
) -> Result<(), AppError> {
    let to = validate_name(to)?;
    if from == to {
        return Ok(());
    }

    let (task_edits, event_edits) = state
        .read(|store| {
            let tasks: Vec<_> = store
                .tasks()
                .into_iter()
                .filter(|task| has_tag(task.tags.as_deref(), from))
                .collect();
            let events: Vec<_> = store
                .events()
                .into_iter()
                .filter(|event| has_tag(event.tags.as_deref(), from))
                .collect();

            for task in &tasks {
                ensure_writable(
                    store,
                    task.remote_collection_id.as_deref().map(String::as_str),
                )?;
            }
            for event in &events {
                ensure_writable(
                    store,
                    event.remote_collection_id.as_deref().map(String::as_str),
                )?;
            }

            let task_edits = tasks
                .into_iter()
                .map(|mut task| {
                    rename_in(&mut task.tags, from, to);
                    (task.id.0, task.tags)
                })
                .collect::<Vec<_>>();
            let event_edits = events
                .into_iter()
                .map(|mut event| {
                    rename_in(&mut event.tags, from, to);
                    (event.id.0, event.tags)
                })
                .collect::<Vec<_>>();

            Ok::<_, AppError>((task_edits, event_edits))
        })
        .await?;

    for (id, tags) in task_edits {
        update_task(state, client, &id, &[TaskPatch::Tags(tags)]).await?;
    }
    for (id, tags) in event_edits {
        update_event(state, client, &id, &[EventPatch::Tags(tags)]).await?;
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<&str, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidInput(
            "a tag name cannot be empty".to_string(),
        ));
    }
    if name.contains(',') || name.contains('\n') || name.contains('\r') {
        return Err(AppError::InvalidInput(
            "a tag name cannot contain commas or line breaks".to_string(),
        ));
    }
    Ok(name)
}

fn has_tag(tags: Option<&[Tag]>, name: &str) -> bool {
    tags.is_some_and(|tags| tags.iter().any(|tag| tag.name == name))
}

fn rename_in(tags: &mut Option<Vec<Tag>>, from: &str, to: &str) {
    let Some(tags) = tags.as_mut() else {
        return;
    };
    let mut seen = HashSet::with_capacity(tags.len());
    let mut renamed = Vec::with_capacity(tags.len());
    for tag in tags.drain(..) {
        let (id, name) = if tag.name == from {
            (to.to_string(), to.to_string())
        } else {
            (tag.id, tag.name)
        };
        if !seen.insert(name.clone()) {
            continue;
        }
        renamed.push(Tag {
            id,
            name,
            color: tag.color,
        });
    }
    *tags = renamed;
}

fn ensure_writable(store: &Store, collection: Option<&str>) -> Result<(), AppError> {
    let Some(href) = collection else {
        return Err(AppError::InvalidInput(
            "a tagged item has no calendar".to_string(),
        ));
    };
    let calendar = store
        .calendar(href)
        .ok_or_else(|| AppError::NotFound(format!("calendar {href}")))?;
    if calendar.writable {
        return Ok(());
    }
    Err(AppError::InvalidInput(format!(
        "the tag is also used in the read-only calendar \"{}\"",
        calendar.display_name
    )))
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    use super::*;
    use crate::features::calendar::caldav::model::{CalendarComponent, CalendarInfo};
    use crate::test_support::caldav::start_server;

    #[tokio::test]
    async fn rename_tag_updates_tasks_and_events_and_merges_duplicates() {
        const TASK_HREF: &str = "/cal/tasks/t1.ics";
        const EVENT_HREF: &str = "/cal/events/e1.ics";
        const TASK_RAW: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:T\r\nCATEGORIES:old,target\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
        const EVENT_RAW: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:E\r\nDTSTART:20261020T090000Z\r\nDTEND:20261020T100000Z\r\nCATEGORIES:old\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

        let (server, client) = start_server().await;
        Mock::given(method("PUT"))
            .and(path(TASK_HREF))
            .respond_with(ResponseTemplate::new(204).insert_header("ETag", "\"t2\""))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path(EVENT_HREF))
            .respond_with(ResponseTemplate::new(204).insert_header("ETag", "\"e2\""))
            .expect(1)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::load(dir.path().to_path_buf());
        store.calendars = vec![
            CalendarInfo {
                href: "/cal/tasks/".to_string(),
                display_name: "Tasks".to_string(),
                component: CalendarComponent::VTodo,
                writable: true,
                ..CalendarInfo::default()
            },
            CalendarInfo {
                href: "/cal/events/".to_string(),
                display_name: "Events".to_string(),
                component: CalendarComponent::VEvent,
                writable: true,
                ..CalendarInfo::default()
            },
        ];
        store
            .set_resource(TASK_HREF, "/cal/tasks/", Some("\"t1\""), TASK_RAW)
            .unwrap();
        store
            .set_resource(EVENT_HREF, "/cal/events/", Some("\"e1\""), EVENT_RAW)
            .unwrap();
        let state = StoreState::new(store);

        rename_tag(&state, &client, "old", " target ").await.unwrap();

        let (task_tags, event_tags) = state
            .read(|s| {
                let t = s
                    .task("t1")
                    .unwrap()
                    .tags
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|tag| tag.name.clone())
                    .collect::<Vec<_>>();
                let e = s
                    .event("e1")
                    .unwrap()
                    .tags
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|tag| tag.name.clone())
                    .collect::<Vec<_>>();
                (t, e)
            })
            .await;
        assert_eq!(task_tags, vec!["target".to_string()]);
        assert_eq!(event_tags, vec!["target".to_string()]);

        let err = rename_tag(&state, &client, "target", "   ")
            .await
            .unwrap_err();
        assert_eq!(err.code(), "invalid-input");
    }
}
