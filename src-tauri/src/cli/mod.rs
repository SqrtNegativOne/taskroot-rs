//! Headless Taskroot CLI for agents and scripts.
//!
//! It is a *client of the app's own domain layer*: it loads `account.json`, the
//! keychain password and every collection through `store::ops::full_load`, then
//! reuses `store::ops` / `store::query` / `domain::projection` for reads and
//! writes. Nothing here parses or serialises `.ics` itself, so the line-preserving
//! write path (invariants I1–I3) still owns every resource.
//!
//! Output is JSON on stdout; failures are `AppError` text on stderr with a
//! non-zero exit code.

pub mod commands;
pub mod docs;
pub mod output;
pub mod session;
pub mod spec;

use clap::{CommandFactory, Parser};

use crate::error::AppError;
use spec::{Cli, Command};

/// Run one invocation from the process arguments.
///
/// # Errors
///
/// Propagates argument parsing and verb [`AppError`] failures; the binary turns
/// them into stderr text and a non-zero exit code.
pub async fn run() -> Result<(), AppError> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                return Ok(());
            }
            let rendered = error.to_string();
            let message = rendered
                .strip_prefix("error: ")
                .unwrap_or(&rendered)
                .trim_end();
            return Err(AppError::InvalidInput(message.to_string()));
        }
    };

    let Some(command) = cli.command else {
        let _ = Cli::command().print_help();
        println!();
        return Ok(());
    };

    match command {
        Command::Lists => commands::calendars::list_calendars().await,
        Command::ListRm { list } => commands::calendars::remove_list(&list).await,
        Command::Tasks(args) => commands::tasks::list_tasks(&args).await,
        Command::TaskShow { ids } => commands::tasks::show_task(&ids).await,
        Command::TaskAdd(args) => commands::tasks::add_task(&args).await,
        Command::TaskSet(args) => commands::tasks::set_task(&args).await,
        Command::TaskDone { ids } => commands::tasks::done_task(&ids).await,
        Command::TaskRm { ids } => commands::tasks::remove_task(&ids).await,
        Command::Events(args) => commands::events::list_events(&args).await,
        Command::EventAdd(args) => commands::events::add_event(&args).await,
        Command::EventRm { id } => commands::events::remove_event(&id).await,
    }
}
