//! `lists` and `list-rm`.

use crate::features::calendar::calendars;
use crate::error::AppError;

use super::super::args::Cli;
use super::super::output::print_json;
use super::super::session::open;
use super::resolve_list;

/// # Errors
///
/// Propagates session and output failures.
pub async fn list_calendars() -> Result<(), AppError> {
    let session = open().await?;
    print_json(&session.store.calendars)
}

/// # Errors
///
/// Propagates session, list-resolution and delete failures.
pub async fn remove_list(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let needle = cli.require_positional(1, "<list>")?;
    let (href, _component) = resolve_list(&session.store, needle)?;

    calendars::delete(&session.dir, &session.client, &mut session.store, &href).await?;
    print_json(&serde_json::json!({ "deleted": href }))
}
