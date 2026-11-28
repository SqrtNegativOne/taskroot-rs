//! `lists` and `list-rm`.

use crate::error::AppError;
use crate::features::calendar::calendars;

use super::super::output::print_json;
use super::super::session::open;
use super::resolve_list;

/// # Errors
///
/// Propagates session and output failures.
pub async fn list_calendars() -> Result<(), AppError> {
    let session = open().await?;
    let calendars = session.store.read(|store| store.calendars.clone()).await;
    print_json(&calendars)
}

/// # Errors
///
/// Propagates session, list-resolution and delete failures.
pub async fn remove_list(needle: &str) -> Result<(), AppError> {
    let session = open().await?;
    let (href, _component) = session
        .store
        .read(|store| resolve_list(store, needle))
        .await?;

    calendars::delete(&session.dir, &session.client, &session.store, &href).await?;
    print_json(&serde_json::json!({ "deleted": href }))
}
