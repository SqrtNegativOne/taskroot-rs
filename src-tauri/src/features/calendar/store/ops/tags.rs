//! Tag renames: rewrite every task and event that carries the old name.

use std::collections::HashSet;

use crate::error::AppError;
use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::domain::Tag;

use super::super::model::Store;
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
    store: &mut Store,
    client: &CalDavClient,
    from: &str,
    to: &str,
) -> Result<(), AppError> {
    let to = validate_name(to)?;
    if from == to {
        return Ok(());
    }

    let mut tasks: Vec<_> = store
        .tasks()
        .into_iter()
        .filter(|task| has_tag(task.tags.as_deref(), from))
        .collect();
    let mut events: Vec<_> = store
        .events()
        .into_iter()
        .filter(|event| has_tag(event.tags.as_deref(), from))
        .collect();

    if tasks.is_empty() && events.is_empty() {
        return Ok(());
    }

    for task in &tasks {
        ensure_writable(store, task.remote_collection_id.as_deref())?;
    }
    for event in &events {
        ensure_writable(store, event.remote_collection_id.as_deref())?;
    }

    for task in &mut tasks {
        rename_in(&mut task.tags, from, to);
    }
    for event in &mut events {
        rename_in(&mut event.tags, from, to);
    }

    for task in tasks {
        update_task(store, client, task).await?;
    }
    for event in events {
        update_event(store, client, event).await?;
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
    *tags = Some(renamed);
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
