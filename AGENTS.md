# Taskroot agent guide

Taskroot is a desktop task manager: Svelte 5 + SvelteKit SPA (TypeScript, Vite,
Bun) wrapped by Tauri v2 (Rust). One active CalDAV profile is the source of
truth; calendar data lives in an in-memory Rust store with immediate server
writes and no local calendar database, offline queue or background sync.

## Working rules

- **Never screenshot the desktop/app or launch it for visual inspection.** Ask
  the user for screenshots or descriptions; rely on code, logs and non-visual
  checks.
- **Keep docs current and non-redundant.** Put each detail in its single owning
  doc below and update links when architecture or structure changes.
- **No migrations or legacy compatibility.** Single-user app with backups: change
  the current schema/defaults directly; never add data migrations, legacy
  fallbacks or localStorage seeding.
- **Never hand-edit `src/lib/bindings/` or `docs/cli.md`.** Both are regenerated
  by `cargo nextest run` in `src-tauri/`; commit updated outputs with your change.
- **Write backend tests first as behavioral contracts.** Do not weaken tests to
  fit code or write change-detector copies of the implementation.
- **Keep code small and modular.** Refactor source files over 250 LOC and
  functions deeper than four indentation levels. Store assets offline.

## Cross-boundary contracts

- **Typed IPC errors**: fallible commands return `Result<T, AppError>`
  (`{code, message}`, mirrored in `src/lib/errors.ts`). Use `not-ready` only for
  uninitialized managed state, never for missing accounts or server failures.
- **Event constants**: import IPC event names from `src-tauri/src/ipc_events.rs`
  and `src/lib/ipcEvents.ts`, never inline strings.
- **ICS preservation**: calendar edits merge owned properties into fetched raw
  ICS bytes and `PUT` conditionally (`If-Match` / `If-None-Match: *`). Never
  re-serialize a DTO into an existing resource. See [CalDAV](docs/caldav.md).
- **Projected calendar rendering**: renderers consume `EventInstance` and
  `TaskDateInstance`, not raw calendar date strings. Recurrence scope is
  resolved in Rust using `occurrenceStart`.

## Required checks

| Change | Run |
|---|---|
| Frontend source | `bun run check`, `bun run lint`, relevant `bun run test:unit` |
| Frontend CSS only | Visual/manual verification (not covered by check/lint/unit) |
| Backend source | In `src-tauri/`: `cargo clippy --all-targets -- -D warnings`, `cargo nextest run` |
| Shared Rust types / IPC | Backend checks + regenerated bindings, then frontend checks |
| Docs only | Verify links and factual claims; no app launch required |

## Read by task

Read scoped instructions **before editing that area**; load only the reference
docs relevant to your task.

| Task | Guidance |
|---|---|
| Frontend conventions & shared state | [src/AGENTS.md](src/AGENTS.md), [frontend](docs/frontend.md) |
| Plan screen, date grid, drag/drop, timeline | [planning UI](docs/planning-ui.md) |
| Colour schemes & tokens | [themes](docs/themes.md), palettes in `src/lib/themes/` |
| Backend conventions & module layout | [src-tauri/AGENTS.md](src-tauri/AGENTS.md), [architecture](docs/architecture.md) |
| Adding settings / UI state persistence | [settings](docs/settings.md) |
| CalDAV transport, accounts, invariants, scopes | [CalDAV](docs/caldav.md), [calendar feature](src-tauri/src/features/calendar/README.md) |
| VEVENT / VTODO / VALARM property mapping | [iCalendar mapping](docs/icalendar-mapping.md) |
| Stopwatch, reminders & focus feed | [stopwatch](src-tauri/src/features/stopwatch/README.md), [reminders](src-tauri/src/features/reminders/README.md), [focus feed](docs/focus-feed.md) |
| CLI | [CLI reference](docs/cli.md) (`bun run cli -- help`) |
| Tests & fixtures | [testing](docs/testing.md), [avoid-change-detector-tests](.agents/skills/avoid-change-detector-tests/SKILL.md) |
| Windows quirks, temp dirs, window lifecycle | [Windows development](docs/windows-development.md) |
| Known bugs / follow-ups (not rules) | [known limitations](docs/known-limitations.md) |

## Test fixture safety

For non-visual CalDAV tests against a real server, use only the disposable
loopback fixture (`http://127.0.0.1:15232`) in [dummy accounts](docs/dummy-accounts.md).
Never touch port `5232`, a normal data directory, or real user credentials, and
never commit real credentials. [TODO.md](TODO.md) is the inspector/iCalendar
backlog, not a description of current behavior.
