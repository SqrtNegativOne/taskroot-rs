//! `clap` command definitions for the headless CLI.

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::features::calendar::{AppTaskStatus, TaskPriority};

pub const HELP_TAIL: &str = "\
An id may be a unique prefix. Verbs that take <id>... accept several ids and do
one load for all of them; they print a JSON array of results. Dates are
YYYY-MM-DD for all-day values, YYYY-MM-DDTHH:MM:SS wall clock for timed ones.

env: TASKROOT_DATA_DIR, TASKROOT_CALDAV_PASSWORD (keychain fallback)

Reads and writes go through Radicale via Taskroot's own store, so every edit is
a line-preserving merge into the fetched .ics resource.";

#[derive(Debug, Parser)]
#[command(
    name = "taskroot",
    about = "Headless Taskroot CLI (JSON on stdout)",
    after_help = HELP_TAIL
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List calendars this account can see
    Lists,
    /// Delete a list and everything in it
    ListRm {
        /// List display name or href prefix
        #[arg(value_name = "LIST")]
        list: String,
    },
    /// List tasks
    Tasks(ListTasksArgs),
    /// Print tasks exactly as stored
    TaskShow {
        /// Task id(s) or unique prefix(es)
        #[arg(required = true, value_name = "ID")]
        ids: Vec<String>,
    },
    /// Create a task
    TaskAdd(TaskAddArgs),
    /// Change tasks; --list moves them
    TaskSet(TaskSetArgs),
    /// Mark tasks done
    TaskDone {
        /// Task id(s) or unique prefix(es)
        #[arg(required = true, value_name = "ID")]
        ids: Vec<String>,
    },
    /// Delete tasks
    TaskRm {
        /// Task id(s) or unique prefix(es)
        #[arg(required = true, value_name = "ID")]
        ids: Vec<String>,
    },
    /// Event instances in a date range (default: today + 14 days)
    Events(ListEventsArgs),
    /// Create an event
    EventAdd(EventAddArgs),
    /// Delete an event
    EventRm {
        /// Event id
        #[arg(value_name = "ID")]
        id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliTaskStatus {
    #[value(name = "todo")]
    Todo,
    #[value(name = "next-up", aliases = ["nextup", "next"])]
    NextUp,
    #[value(name = "done", alias = "completed")]
    Done,
    #[value(name = "cancelled", alias = "canceled")]
    Cancelled,
}

impl From<CliTaskStatus> for AppTaskStatus {
    fn from(value: CliTaskStatus) -> Self {
        match value {
            CliTaskStatus::Todo => Self::Todo,
            CliTaskStatus::NextUp => Self::NextUp,
            CliTaskStatus::Done => Self::Done,
            CliTaskStatus::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliPriority {
    #[value(name = "none", alias = "0")]
    None,
    #[value(name = "low", alias = "1")]
    Low,
    #[value(name = "medium", alias = "2")]
    Medium,
    #[value(name = "high", alias = "3")]
    High,
    #[value(name = "urgent", alias = "4")]
    Urgent,
}

impl From<CliPriority> for TaskPriority {
    fn from(value: CliPriority) -> Self {
        match value {
            CliPriority::None => Self::None,
            CliPriority::Low => Self::Low,
            CliPriority::Medium => Self::Medium,
            CliPriority::High => Self::High,
            CliPriority::Urgent => Self::Urgent,
        }
    }
}

#[derive(Debug, Clone, Default, Args)]
pub struct ListTasksArgs {
    /// Substring filter
    #[arg(long, value_name = "TEXT")]
    pub search: Option<String>,
    /// Filter by task status
    #[arg(long, value_enum)]
    pub status: Option<CliTaskStatus>,
    /// Only tasks that are not done
    #[arg(long)]
    pub open: bool,
}

#[derive(Debug, Clone, Default, Args)]
pub struct TaskFields {
    /// Description
    #[arg(long, value_name = "TEXT")]
    pub notes: Option<String>,
    /// Task due date
    #[arg(long, value_name = "DATE")]
    pub due: Option<String>,
    /// Task start date
    #[arg(long, value_name = "DATE")]
    pub start: Option<String>,
    /// Priority (0-4 or none/low/medium/high/urgent)
    #[arg(long, value_enum)]
    pub priority: Option<CliPriority>,
    /// Task status
    #[arg(long, value_enum)]
    pub status: Option<CliTaskStatus>,
    /// IANA zone for timed values
    #[arg(long, value_name = "ZONE")]
    pub tz: Option<String>,
    /// Target list; on task-set this moves the task
    #[arg(long, value_name = "NAME")]
    pub list: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct TaskAddArgs {
    /// Task title
    #[arg(long, value_name = "TITLE")]
    pub title: String,
    #[command(flatten)]
    pub fields: TaskFields,
}

#[derive(Debug, Clone, Args)]
pub struct TaskSetArgs {
    /// Task id(s) or unique prefix(es)
    #[arg(required = true, value_name = "ID")]
    pub ids: Vec<String>,
    /// New task title
    #[arg(long, value_name = "TITLE")]
    pub title: Option<String>,
    /// Drop the due date
    #[arg(long)]
    pub clear_due: bool,
    /// Drop the start date
    #[arg(long)]
    pub clear_start: bool,
    #[command(flatten)]
    pub fields: TaskFields,
}

#[derive(Debug, Clone, Default, Args)]
pub struct ListEventsArgs {
    /// Range start (default: today)
    #[arg(long, value_name = "DATE")]
    pub start: Option<String>,
    /// Range end (default: today + 14 days)
    #[arg(long, value_name = "DATE")]
    pub end: Option<String>,
    /// Substring filter
    #[arg(long, value_name = "TEXT")]
    pub search: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct EventAddArgs {
    /// Event title
    #[arg(long, value_name = "TITLE")]
    pub title: String,
    /// Event start
    #[arg(long, value_name = "DATE")]
    pub start: String,
    /// Event end
    #[arg(long, value_name = "DATE")]
    pub end: String,
    /// Description
    #[arg(long, value_name = "TEXT")]
    pub notes: Option<String>,
    /// Priority (0-4 or none/low/medium/high/urgent)
    #[arg(long, value_enum)]
    pub priority: Option<CliPriority>,
    /// IANA zone for timed values
    #[arg(long, value_name = "ZONE")]
    pub tz: Option<String>,
    /// Target list
    #[arg(long, value_name = "NAME")]
    pub list: Option<String>,
    /// Event has no time of day
    #[arg(long)]
    pub all_day: bool,
    /// Recurrence rule
    #[arg(long, value_name = "RULE")]
    pub rrule: Option<String>,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(tokens: &[&str]) -> Result<Cli, clap::Error> {
        let mut argv = vec!["taskroot"];
        argv.extend_from_slice(tokens);
        Cli::try_parse_from(argv)
    }

    #[test]
    fn clap_definition_passes_internal_debug_assertions() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_key_equals_value_and_double_dash_terminator() {
        let parsed = parse(&[
            "task-add",
            "--title=--not-a-flag",
            "--due=2026-10-28",
            "--priority=3",
        ])
        .unwrap();

        let Some(Command::TaskAdd(args)) = parsed.command else {
            panic!("expected task-add");
        };
        assert_eq!(args.title, "--not-a-flag");
        assert_eq!(args.fields.due.as_deref(), Some("2026-10-28"));
        assert_eq!(args.fields.priority, Some(CliPriority::High));

        let show = parse(&["task-show", "--", "-dash-id", "b7c1"]).unwrap();
        let Some(Command::TaskShow { ids }) = show.command else {
            panic!("expected task-show");
        };
        assert_eq!(ids, vec!["-dash-id", "b7c1"]);
    }

    #[test]
    fn unknown_flag_typo_is_rejected_instead_of_ignored() {
        let err = parse(&["task-add", "--titl", "Oops"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn invalid_status_and_priority_values_are_rejected_at_parse_time() {
        assert!(parse(&["tasks", "--status", "later"]).is_err());
        assert!(parse(&["task-add", "--title", "x", "--priority", "9"]).is_err());
    }

    #[test]
    fn status_and_priority_aliases_are_accepted() {
        let parsed = parse(&[
            "task-set",
            "a2a8",
            "b7c1",
            "--clear-due",
            "--priority",
            "4",
            "--status",
            "next",
        ])
        .unwrap();

        let Some(Command::TaskSet(args)) = parsed.command else {
            panic!("expected task-set");
        };
        assert_eq!(args.ids, vec!["a2a8", "b7c1"]);
        assert!(args.clear_due);
        assert_eq!(args.fields.priority, Some(CliPriority::Urgent));
        assert_eq!(args.fields.status, Some(CliTaskStatus::NextUp));
    }

    #[test]
    fn task_show_requires_at_least_one_id_and_accepts_several() {
        assert!(parse(&["task-show"]).is_err());

        let parsed = parse(&["task-show", "id-1", "id-2"]).unwrap();
        let Some(Command::TaskShow { ids }) = parsed.command else {
            panic!("expected task-show");
        };
        assert_eq!(ids, vec!["id-1", "id-2"]);
    }
}
