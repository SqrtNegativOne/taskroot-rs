# taskroot CLI

<!-- Generated from the VERBS and OPTIONS tables in src-tauri/src/bin/taskroot.rs by `cargo nextest run`. Do not edit. -->

Headless Taskroot for agents and scripts: JSON on stdout, `AppError` text on
stderr with a non-zero exit code.

```
bun run cli -- <verb>
```

## Commands

| Command | Does |
| --- | --- |
| `help` | this text |
| `lists` | calendars this account can see |
| `list-rm <list>` | delete a list and everything in it |
| `tasks [--search S] [--status S] [--open]` | list tasks |
| `task-show <id>...` | print tasks exactly as stored |
| `task-add --title T [options]` | create a task |
| `task-set <id>... [options]` | change tasks; --list moves them |
| `task-done <id>...` | mark tasks done |
| `task-rm <id>...` | delete tasks |
| `events [--start D] [--end D] [--search S]` | event instances in a date range (default: today + 14 days) |
| `event-add --title T --start S --end E [options]` | create an event |
| `event-rm <id>` | delete an event |

## Options

| Flag | Meaning |
| --- | --- |
| `--title T` | title (required for the add verbs) |
| `--notes N` | description |
| `--due D` | task due date |
| `--start D, --end E` | task start date; event start and end |
| `--priority P` | 0-4, or none/low/medium/high/urgent |
| `--status S` | todo, next-up, doing or done |
| `--tz ZONE` | IANA zone for timed values |
| `--list NAME` | target list; on task-set this moves the task |
| `--clear-due, --clear-start` | drop a date |
| `--all-day` | event has no time of day |
| `--rrule R` | recurrence rule |
| `--search S` | substring filter |
| `--open` | only tasks that are not done |

An id may be a unique prefix. Verbs that take <id>... accept several ids and do
one load for all of them; they print a JSON array of results. Dates are
YYYY-MM-DD for all-day values, YYYY-MM-DDTHH:MM:SS wall clock for timed ones.

env: TASKROOT_DATA_DIR, TASKROOT_CALDAV_PASSWORD (keychain fallback)

Reads and writes go through Radicale via Taskroot's own store, so every edit is
a line-preserving merge into the fetched .ics resource.
