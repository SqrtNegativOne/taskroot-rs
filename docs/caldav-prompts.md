# CalDAV migration — session prompts

One phase per session. Each phase is a natural context boundary, ends green, and leaves
the app usable. Do not ask a single session to do the whole migration.

## How to use

1. Commit `caldav-migration-plan.md` and this file first.
2. Start a fresh session and paste the phase prompt below.
3. Review the diff and run the gates yourself. Do not let a session commit.
4. Mark the phase done here (or ask the session to) before starting the next one.

If a session's proposal contradicts `caldav-migration-plan.md` §17, stop it. The decisions
(no SQLite, no mirror, no queue, no provider trait, VTODO in v1, checklist removed) are
settled, not open for re-litigation.

## One session for everything

If you would rather hand the whole thing to one session (it can spawn subagents), paste
this instead of a phase prompt:

> Read `AGENTS.md`, `src/AGENTS.md`, `src-tauri/AGENTS.md`, `caldav-migration-plan.md`
> and `docs/caldav-prompts.md` first.
>
> Implement the whole migration (P0→P6). The app does not need to stay usable between
> phases, so optimise for finishing quickly and correctly. The human has backups and the
> phone is disconnected from Radicale, so agents may modify dev data. Do not lose the 46
> VTODO files in `SqrtTasks`: they exist only there. If you rebuild a collection, copy
> them verbatim and keep their UIDs.
>
> Parallelise with subagents, serialising anything that touches shared files:
> 1. P0 alone — it touches everything.
> 2. P1 (`caldav/`) and P2 (`ics/`) as two parallel subagents.
> 3. P3 alone, as a throwaway `src-tauri/examples/import_google.rs` (not an app command;
>    delete it once the import is verified).
> 4. P4 alone — it integrates everything.
> 5. P5 alone.
> 6. P6 last; docs can run in parallel with UI leftovers.
>
> §17 decisions are settled: no Google code, no provider abstraction, no offline queue,
> no mirror, SQLite only until P5. If reality contradicts the plan, stop and report rather
> than redesign.
>
> After every phase: `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`
> (regenerates bindings; include them), `bun run check`, `bun run lint`. Commit locally per
> phase; do not push. Write the I1–I3 round-trip tests before the ICS writer. When in
> doubt, pick the option with fewer mechanisms.
>
> Finish with an end-to-end check against Radicale: create, edit, move and delete an event
> and a task from the app; restart and confirm the server and `snapshot.json` agree; and
> confirm a tasks.org file keeps `X-APPLE-SORT-ORDER` after an app edit.
>
> Report per phase: what changed, gate output, and any deviation.

## Ground rules (apply to every phase)

- Read, in this order: `AGENTS.md`, `src/AGENTS.md`, `src-tauri/AGENTS.md`,
  `caldav-migration-plan.md`, then this file.
- Implement **only** the named phase. If the code contradicts the plan, stop and report
  instead of redesigning.
- Gates, all must pass before reporting:
  - `cargo clippy --all-targets -- -D warnings` in `src-tauri/`
  - `cargo nextest run` in `src-tauri/` (this regenerates `src/lib/bindings/*.generated.ts`
    — include the regenerated files)
  - `bun run check` and `bun run lint`
- Do not commit. Do not touch anything under `C:\radicale` or the live database.
- Report at the end: files changed, gate results, deviations from the plan, and anything the
  next phase needs to know.

## P0 — Delete Google

> Implement P0 from `caldav-migration-plan.md` §12. Delete the Google API clients
> (`apis/google_calendar/`, `apis/google_tasks.rs`), `auth.rs`, the `oauth2` dependency, the
> offline queue (`sync/{queue,queue_store,drain,push,types}.rs`), and their commands and
> registrations. Replace `sync_with_google` with a no-op `sync_all` (the poller stays).
> Remove the OAuth URL event, the `login` route, the `is_logged_in` gate in
> `+layout.svelte`, `reset_auth` from the settings row and dev page, the `logout` custom
> setting, and the unused checklist UI (`ChecklistInput.svelte`, the inspector section, the
> `TaskRow` badge) — leave the `checklist` DTO field and SQL column alone for now.
> `commands::events` must stop calling `plan_event_sync`/`push_*` and just write the row.
> SQLite and local CRUD stay working; the app must open on local data. Fix the command
> source-scan tests for the removed commands. Gate and report.

## P1 — CalDAV transport, discovery, account setup

> Implement P1 from `caldav-migration-plan.md` §12: `caldav/{transport,xml,discovery}.rs`,
> account commands, `account.json` (no password — OS keychain via `keyring`), and the
> connect screen replacing `login/+page.svelte`, plus the account card in Settings.
> Radicale is at `http://127.0.0.1:5232` with htpasswd auth and one user; default the URL
> to that. Allow plain `http` only for loopback and `*.ts.net`. Discovery persists
> collections (href, display name, color, component type) and never runs on a normal sync.
> No calendar data flows yet. For XML fixtures, ask the human to run a `curl` PROPFIND /
> REPORT against Radicale and paste the output; do not invent fixtures. Gate and report.

## P2 — ICS read + line-preserving write

> Implement P2 from `caldav-migration-plan.md` §12 and §7: `ics/read.rs` (resource →
> `AppEvent`/`AppTask`, `exdates` from `EXDATE`) and `ics/write.rs` (unfold, locate the
> component by UID + RECURRENCE-ID, replace only owned properties, refold with CRLF).
> Copy real fixtures out of `C:\radicale\collections\collection-root\me\` (a Mozilla event
> with a full `VTIMEZONE`/`EXDATE`/`RRULE`, the `dsh-…@dsh-import` all-day event, and a
> tasks.org VTODO with `X-APPLE-SORT-ORDER`). Write the I1–I3 round-trip tests first.
> No UI or behaviour change. Gate and report.

## P3 — Import SQLite → Radicale (re-runnable)

> Implement P3 from `caldav-migration-plan.md` §11 and §12 as a throwaway
> `src-tauri/examples/import_google.rs` that reuses the `caldav`/`ics` modules — not an app
> command; delete it once verified. Read the old `taskroot.db` read-only; derive each
> imported resource's UID deterministically from the Google `remote_id`
> (`<remote_id>@taskroot-import`) so a re-run addresses the same resources.
> Group events by series (`recurring_event_id`) into one resource with master + exceptions;
> `PUT` into `SqrtCal`. Create the `Google Tasks (imported)` collection with `MKCALENDAR`
> if missing and put the 86 old tasks there. Verify counts and that a second run changes
> nothing. Never modify or delete anything already in Radicale. Gate and report.

## P4 — Cutover to the in-memory store

> Implement P4 from `caldav-migration-plan.md` §4–§6: the in-memory `Store`, startup
> `PROPFIND` + multiget, `snapshot.json` fallback, `sync-collection` pull, immediate
> `PUT`-with-`If-Match` on edit (and `If-None-Match: *` on create), `412` re-fetch/re-merge/
> retry, deletes, JSON settings and UI state, `AppError::Store`, `writable` instead of
> `access_role`, `AppTask.task_list_id` → `remote_collection_id`. SQLite stays on disk,
> unused. Gate: the app is fully usable against Radicale, edits round-trip to the phone, and
> stopping Radicale yields the snapshot plus an offline notice. Report.

## P5 — Delete SQLite

> Implement P5 from `caldav-migration-plan.md` §12. Remove `db/`, `sqlx`, the import path
> and the SQL attributes from the `Queryable` macro (keep the metadata/filter-schema part
> that the frontend uses). Nothing may read `taskroot.db` any more. Keep `taskroot.db`
> on disk, renamed, as the last-resort copy. Gate and report.

## P6 — Polish + docs

> Implement P6 from `caldav-migration-plan.md` §12: read-only calendars via `writable`,
> `default_calendar` / `default_task_collection` pickers, sync/offline status, dev-page
> cleanup. Update `AGENTS.md`, `src/AGENTS.md`, `src-tauri/AGENTS.md`, `README.md`; delete
> `docs/google-calendar-write.md` and add `docs/caldav.md` (protocol notes + §7's three
> invariants + the VTODO mapping). Gate and report.
