# Taskroot agent guide

Taskroot is a desktop task manager: Svelte 5 + SvelteKit SPA (TypeScript, Vite,
Bun), wrapped by Tauri v2 (Rust). Application colours use bundled CSS-variable
palettes; see [colour schemes](docs/themes.md). Saved CalDAV profiles have one active account;
its server is the source of truth. Calendar data lives in an in-memory Rust store,
with immediate server writes.
Settings and UI state use JSON files. There is no local calendar database,
offline queue or background calendar sync.

## Working rules

- **Never screenshot the desktop or a running app, or launch it for visual
  inspection.** Ask the user for a screenshot/description; use code, logs and
  non-visual checks yourself.
- **Keep guidance current.** Architecture, stack or file-structure changes must
  update this guide's relevant summary/links and the affected reference docs.
  Do not grow an exhaustive file inventory here.
- **No migrations or legacy compatibility.** This is a single-user app with
  backups. Change the current schema/defaults; let old files be reset/restored
  by hand. Do not add data migrations, legacy-setting fallbacks or localStorage
  seeding.
- **Never hand-edit `src/lib/bindings/`.** Changes to Rust `#[derive(TS)]` types
  require `cargo nextest run` in `src-tauri/`; include regenerated bindings.
  The same suite regenerates `docs/cli.md` from `cli/help.rs`.
- Write backend tests first as contracts; do not weaken them to fit an
  implementation. Test behavior, not tautological copies of the code.
- Keep code small and modular: refactor source files over 250 LOC and functions
  with more than four indentation levels. Prefer early returns, named helpers
  and self-documenting code over redundant comments. Store assets offline.

## Cross-boundary contracts

- Fallible IPC commands must return `Result<T, AppError>`; errors serialize as
  `{code, message}`. Infallible commands may return values directly. The frontend
  error mirror is `src/lib/errors.ts`; never return raw error strings.
  Use `not-ready` for managed state that is still being initialized, not for a
  missing account or a server failure.
- Use event constants from `src-tauri/src/ipc_events.rs` and
  `src/lib/ipcEvents.ts`, not inline event-name strings. Frontend-only events
  such as `LAUNCHER_NAVIGATE` also live in the TypeScript module.
- Calendar edits preserve fetched raw ICS: merge owned properties, then `PUT`
  with `If-Match`; creates use `If-None-Match: *`. Never serialize an edited DTO
  into a replacement resource. Ordinary same-collection updates retry a `412`
  once after re-fetch/re-merge; other conflict paths currently report failure.
  Preserve unknown properties/components and sibling exceptions.
- Calendar renderers consume `EventInstance`/`TaskDateInstance`, not raw calendar
  date strings. Recurrence scope is resolved in Rust using `occurrenceStart`;
  the frontend never composes ICS. See [CalDAV invariants](docs/caldav.md).

## Required checks

| Change | Run |
|---|---|
| Frontend source | `bun run check`, `bun run lint`, relevant `bun run test:unit` tests |
| Frontend styles only (CSS) | Visual/manual verification; check, lint and unit tests do not cover it |
| Backend source | From `src-tauri/`: `cargo clippy --all-targets -- -D warnings`, `cargo nextest run` |
| Shared Rust types / IPC | Backend checks and regenerated bindings, then frontend checks |
| Documentation only | Verify paths/links and factual claims; no app launch required |

See [testing](docs/testing.md) for fixtures, coverage, Playwright and the limits
of IPC source-contract checks. If Windows temp/data access fails, consult
[Windows development](docs/windows-development.md) rather than assuming a cause.

## Read by task

Read scoped instructions **before editing that area**; load reference documents
only for the affected subsystem.

| Task | Guidance |
|---|---|
| Frontend | [src/AGENTS.md](src/AGENTS.md), [frontend reference](docs/frontend.md) |
| Appearance / colour schemes | [Theme reference](docs/themes.md), palettes in `src/lib/themes/` |
| Backend | [src-tauri/AGENTS.md](src-tauri/AGENTS.md), feature READMEs linked below |
| Locate modules / settings / window ownership | [Architecture map](docs/architecture.md) |
| Calendar, recurrence, filters, transport, ICS | [Calendar feature](src-tauri/src/features/calendar/README.md), [CalDAV](docs/caldav.md) |
| Clock, focus and reminders | [Stopwatch](src-tauri/src/features/stopwatch/README.md), [reminders](src-tauri/src/features/reminders/README.md), [focus feed](docs/focus-feed.md) |
| CLI use | [Generated CLI reference](docs/cli.md); `bun run cli -- help` |
| Tests | [Testing reference](docs/testing.md), [test-review skill](.agents/skills/avoid-change-detector-tests/SKILL.md) |
| Known bugs / proposed simplifications | [Audited follow-ups](docs/known-limitations.md) (not rules to preserve) |
| Windows build/access issues | [Windows development](docs/windows-development.md) |

## Dummy accounts

The disposable fixture at `http://127.0.0.1:15232` uses **test-only** credentials:

| Username | Password |
|---|---|
| `taskroot-dummy-a` | `dummy-a-only` |
| `taskroot-dummy-b` | `dummy-b-only` |

Start/seed it with [dummy-account setup](docs/dummy-accounts.md), then add the
profiles in Settings → Account. Agents may use this fixture for non-visual tests;
never use port 5232, a normal data directory or the user's credentials. The server
is loopback-only and all fixture storage is under ignored `src-tauri/target/`.
No real credentials belong in documentation.

[TODO.md](TODO.md) is the inspector/iCalendar backlog, not a description of the
current implementation. Prefer code and current references when checking claims.
