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

pub mod args;
pub mod commands;
pub mod help;
pub mod output;
pub mod session;

use args::Cli;
use error::AppError;

use crate::error;

/// Run one invocation from the process arguments.
///
/// # Errors
///
/// Propagates the verb's [`AppError`]; the binary turns it into stderr text and a
/// non-zero exit code.
pub async fn run() -> Result<(), AppError> {
    let cli = Cli::from_env();
    let Some(command) = cli.positional(0) else {
        help::print_help();
        return Ok(());
    };

    match command {
        "help" => {
            help::print_help();
            Ok(())
        }
        "lists" => commands::calendars::list_calendars().await,
        "list-rm" => commands::calendars::remove_list(&cli).await,
        "tasks" => commands::tasks::list_tasks(&cli).await,
        "task-show" => commands::tasks::show_task(&cli).await,
        "task-add" => commands::tasks::add_task(&cli).await,
        "task-set" => commands::tasks::set_task(&cli).await,
        "task-done" => commands::tasks::done_task(&cli).await,
        "task-rm" => commands::tasks::remove_task(&cli).await,
        "events" => commands::events::list_events(&cli).await,
        "event-add" => commands::events::add_event(&cli).await,
        "event-rm" => commands::events::remove_event(&cli).await,
        other => Err(AppError::InvalidInput(format!(
            "unknown command `{other}`; run `taskroot help`"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dispatched_verb_is_documented() {
        let source = include_str!("mod.rs");
        let arms = source
            .split_once("match command {")
            .and_then(|(_, rest)| rest.split_once("other =>"))
            .map(|(arms, _)| arms)
            .unwrap();

        let mut dispatched: Vec<&str> = arms
            .lines()
            .filter_map(|line| line.trim().strip_prefix('"'))
            .filter_map(|rest| rest.split_once("\" =>").map(|(name, _)| name))
            .collect();
        let mut documented: Vec<&str> = help::VERBS
            .iter()
            .filter_map(|verb| verb.usage.split_whitespace().next())
            .collect();
        dispatched.sort_unstable();
        documented.sort_unstable();

        assert_eq!(dispatched, documented);
    }
}
