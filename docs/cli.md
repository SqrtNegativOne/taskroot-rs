# taskroot CLI

<!-- Generated from the clap CLI definition in src-tauri/src/cli/spec.rs by `cargo nextest run`. Do not edit. -->

Headless Taskroot for agents and scripts: JSON on stdout, `AppError` text on
stderr with a non-zero exit code.

```
bun run cli -- <command>
```

## Commands

| Command | Does |
| --- | --- |
| `lists` | List calendars this account can see |
| `list-rm <list>` | Delete a list and everything in it |
| `tasks [options]` | List tasks |
| `task-show <id>...` | Print tasks exactly as stored |
| `task-add --title <TITLE> [options]` | Create a task |
| `task-set <id>... [options]` | Change tasks; --list moves them |
| `task-done <id>...` | Mark tasks done |
| `task-rm <id>...` | Delete tasks |
| `events [options]` | Event instances in a date range (default: today + 14 days) |
| `event-add --title <TITLE> --start <DATE> --end <DATE> [options]` | Create an event |
| `event-rm <id>` | Delete an event |

## Command options

### `tasks`

| Flag | Meaning |
| --- | --- |
| `--search <TEXT>` | Substring filter |
| `--status <STATUS>` | Filter by task status (values: todo, next-up, done, cancelled) |
| `--open` | Only tasks that are not done |

### `task-add`

| Flag | Meaning |
| --- | --- |
| `--title <TITLE>` | Task title *(required)* |
| `--notes <TEXT>` | Description |
| `--due <DATE>` | Task due date |
| `--start <DATE>` | Task start date |
| `--priority <PRIORITY>` | Priority (0-4 or none/low/medium/high/urgent) (values: none, low, medium, high, urgent) |
| `--status <STATUS>` | Task status (values: todo, next-up, done, cancelled) |
| `--tz <ZONE>` | IANA zone for timed values |
| `--list <NAME>` | Target list; on task-set this moves the task |

### `task-set`

| Flag | Meaning |
| --- | --- |
| `--title <TITLE>` | New task title |
| `--clear-due` | Drop the due date |
| `--clear-start` | Drop the start date |
| `--notes <TEXT>` | Description |
| `--due <DATE>` | Task due date |
| `--start <DATE>` | Task start date |
| `--priority <PRIORITY>` | Priority (0-4 or none/low/medium/high/urgent) (values: none, low, medium, high, urgent) |
| `--status <STATUS>` | Task status (values: todo, next-up, done, cancelled) |
| `--tz <ZONE>` | IANA zone for timed values |
| `--list <NAME>` | Target list; on task-set this moves the task |

### `events`

| Flag | Meaning |
| --- | --- |
| `--start <DATE>` | Range start (default: today) |
| `--end <DATE>` | Range end (default: today + 14 days) |
| `--search <TEXT>` | Substring filter |

### `event-add`

| Flag | Meaning |
| --- | --- |
| `--title <TITLE>` | Event title *(required)* |
| `--start <DATE>` | Event start *(required)* |
| `--end <DATE>` | Event end *(required)* |
| `--notes <TEXT>` | Description |
| `--priority <PRIORITY>` | Priority (0-4 or none/low/medium/high/urgent) (values: none, low, medium, high, urgent) |
| `--tz <ZONE>` | IANA zone for timed values |
| `--list <NAME>` | Target list |
| `--all-day` | Event has no time of day |
| `--rrule <RULE>` | Recurrence rule |

An id may be a unique prefix. Verbs that take <id>... accept several ids and do
one load for all of them; they print a JSON array of results. Dates are
YYYY-MM-DD for all-day values, YYYY-MM-DDTHH:MM:SS wall clock for timed ones.

env: TASKROOT_DATA_DIR, TASKROOT_CALDAV_PASSWORD (keychain fallback)

Reads and writes go through Radicale via Taskroot's own store, so every edit is
a line-preserving merge into the fetched .ics resource.
