//! The verb/option tables, the terminal help text and the generated `docs/cli.md`.

use std::fmt::Write;

/// One CLI verb.
///
/// [`help_text`] and `docs/cli.md` are both rendered from [`VERBS`], so the help
/// text cannot describe a command the dispatch does not have (the reverse is
/// covered by `every_dispatched_verb_is_documented`).
pub struct Verb {
    pub usage: &'static str,
    pub summary: &'static str,
}

pub const VERBS: &[Verb] = &[
    Verb {
        usage: "help",
        summary: "this text",
    },
    Verb {
        usage: "lists",
        summary: "calendars this account can see",
    },
    Verb {
        usage: "list-rm <list>",
        summary: "delete a list and everything in it",
    },
    Verb {
        usage: "tasks [--search S] [--status S] [--open]",
        summary: "list tasks",
    },
    Verb {
        usage: "task-show <id>...",
        summary: "print tasks exactly as stored",
    },
    Verb {
        usage: "task-add --title T [options]",
        summary: "create a task",
    },
    Verb {
        usage: "task-set <id>... [options]",
        summary: "change tasks; --list moves them",
    },
    Verb {
        usage: "task-done <id>...",
        summary: "mark tasks done",
    },
    Verb {
        usage: "task-rm <id>...",
        summary: "delete tasks",
    },
    Verb {
        usage: "events [--start D] [--end D] [--search S]",
        summary: "event instances in a date range (default: today + 14 days)",
    },
    Verb {
        usage: "event-add --title T --start S --end E [options]",
        summary: "create an event",
    },
    Verb {
        usage: "event-rm <id>",
        summary: "delete an event",
    },
];

pub const OPTIONS: &[(&str, &str)] = &[
    ("--title T", "title (required for the add verbs)"),
    ("--notes N", "description"),
    ("--due D", "task due date"),
    ("--start D, --end E", "task start date; event start and end"),
    ("--priority P", "0-4, or none/low/medium/high/urgent"),
    ("--status S", "todo, next-up, doing or done"),
    ("--tz ZONE", "IANA zone for timed values"),
    (
        "--list NAME",
        "target list; on task-set this moves the task",
    ),
    ("--clear-due, --clear-start", "drop a date"),
    ("--all-day", "event has no time of day"),
    ("--rrule R", "recurrence rule"),
    ("--search S", "substring filter"),
    ("--open", "only tasks that are not done"),
];

const HELP_TAIL: &str = "\
An id may be a unique prefix. Verbs that take <id>... accept several ids and do
one load for all of them; they print a JSON array of results. Dates are
YYYY-MM-DD for all-day values, YYYY-MM-DDTHH:MM:SS wall clock for timed ones.

env: TASKROOT_DATA_DIR, TASKROOT_CALDAV_PASSWORD (keychain fallback)

Reads and writes go through Radicale via Taskroot's own store, so every edit is
a line-preserving merge into the fetched .ics resource.";

#[must_use]
pub fn help_text() -> String {
    let width = VERBS.iter().map(|verb| verb.usage.len()).max().unwrap_or(0);
    let mut text = String::from("taskroot — headless Taskroot CLI (JSON on stdout)\n\n");
    for verb in VERBS {
        let _ = writeln!(text, "  {:<width$}  {}", verb.usage, verb.summary);
    }
    text.push_str("\noptions:\n");
    for (flag, summary) in OPTIONS {
        let _ = writeln!(text, "  {flag} — {summary}");
    }
    text.push('\n');
    text.push_str(HELP_TAIL);
    text.push('\n');
    text
}

/// The `docs/cli.md` rendering, written by the `exports_the_cli_reference` test.
#[cfg(test)]
#[must_use]
pub fn help_markdown() -> String {
    let mut text = String::from(
        "# taskroot CLI\n\n\
         <!-- Generated from the VERBS and OPTIONS tables in src-tauri/src/cli/help.rs \
         by `cargo nextest run`. Do not edit. -->\n\n\
         Headless Taskroot for agents and scripts: JSON on stdout, `AppError` text on\n\
         stderr with a non-zero exit code.\n\n\
         ```\nbun run cli -- <verb>\n```\n\n\
         ## Commands\n\n| Command | Does |\n| --- | --- |\n",
    );
    for verb in VERBS {
        let _ = writeln!(text, "| `{}` | {} |", verb.usage, verb.summary);
    }
    text.push_str("\n## Options\n\n| Flag | Meaning |\n| --- | --- |\n");
    for (flag, summary) in OPTIONS {
        let _ = writeln!(text, "| `{flag}` | {summary} |");
    }
    text.push('\n');
    text.push_str(HELP_TAIL);
    text.push('\n');
    text
}

pub fn print_help() {
    print!("{}", help_text());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_the_cli_reference() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/cli.md");

        std::fs::write(path, help_markdown()).unwrap();
    }
}
