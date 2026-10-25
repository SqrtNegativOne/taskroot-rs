# CalDAV Migration Plan — v2 (rewrite)

> Google is gone permanently. One user, one CalDAV account, small data set.
> Optimise for **startup time** and **little code**, not for scale.

## 1. Goals and non-goals

**Goals**

- CalDAV events **and tasks** (pull + push) against the local Radicale, with the phone syncing to it over the tailnet.
- Delete Google, OAuth, the offline queue, and SQLite.
- Radicale is the single source of truth. Calendar and task data lives in memory; the only on-disk copy is a read-only snapshot for offline starts.
- Startup stays fast without a local mirror, index, or query cache.

**Non-goals (deliberately not built)**

- Multi-account, multi-provider, or a `provider` abstraction. One server, one user.
- A per-resource `.ics` mirror directory, an offline edit queue, or a dirty-flag system.
- Markdown checklists in `DESCRIPTION`, or checklist items as child tasks. The checklist feature is removed (§9); task-tree subtasks are supported.
- CTag preflight, `MOVE` method, `calendar-query` time windows, conflict UI.
- Scaling past a few thousand resources. If it ever gets slow, see §14.

**Assumed deployment.** Radicale runs on the laptop and listens on `127.0.0.1:5232`; the phone reaches it over the tailnet through `tailscale serve` → `https://sqrt-1.tail03279c.ts.net`, using DAVx5 with tasks.org for VTODO and a calendar app for VEVENT. Taskroot connects to the loopback address, so its traffic never leaves the machine and needs no TLS. The app never reads Radicale's storage directory — that layout is Radicale's private implementation detail, not a contract — and its only local copy of calendar data is `snapshot.json`, a cache that is rebuilt from Radicale.

```
phone                                   laptop
─────                                   ──────
tasks.org ──┐
            ├── DAVx5 ──┐
calendar ───┘            │
                         ▼
               https://sqrt-1.tail03279c.ts.net
               (tailscale serve, tailnet-only)
                         │
                         ▼
               Radicale @ 127.0.0.1:5232 ────► C:\radicale\collections
                         ▲                      (Radicale's private storage;
                         │                       no other writer)
                         │ CalDAV, loopback HTTP + Basic auth
                         │
               Taskroot (desktop app) ────► settings.json / ui-state.json /
                                            account.json / snapshot.json
```

## 2. What changes

| Concern | Today | After |
|---|---|---|
| Calendar storage | SQLite mirror + raw-row CRUD | CalDAV into memory + one read-only snapshot |
| Settings / UI state | `settings` + `ui_state` tables | `settings.json`, `ui-state.json` |
| Tasks | SQLite rows + Google Tasks | VTODO resources, same pipeline as events |
| Sync state | `sync_queue` table, `SyncAction`, drain loop | in-memory per-collection tokens; edits PUT immediately |
| Auth | OAuth2 PKCE, refresh tokens, loopback | HTTP Basic, password in OS keychain |
| Backend deps | `sqlx`, `oauth2`, `dotenvy` | `icalendar`, `roxmltree`, `base64`, `keyring` |
| Errors | `AppError::Db(sqlx::Error)` | `AppError::Store(String)` (code `store`) |

Net effect, counting existing tests and excluding the temporary importer (±20%; these are estimates from the current file sizes):

| Removed | Added |
|---|---|
| Google clients ~1,375 | CalDAV transport/XML/discovery ~470 |
| OAuth (`auth.rs`) ~235 | ICS read + line-preserving write ~380 |
| SQLite layer (`db/`, migrations, CRUD, tests) ~1,260 | In-memory store, filters, snapshot ~270 |
| Offline queue + push/drain/types ~1,100 | Sync engine rewrite ~160 |
| Settings persistence (SQL rows) ~250 | JSON settings ~90 |
| Frontend (login, OAuth, checklist) ~350 | Frontend (connect screen, account) ~260 |
| Docs, `Queryable` macro prune, misc ~250 | One-time importer ~120 |
| **~4,820** | **~1,750** |

**Net ≈ −3,100 lines**, about a third of the Rust backend. The savings are almost entirely "delete Google and SQLite"; the new protocol layer (~470) roughly replaces the old Google client (~1,130) rather than adding on top of it.

## 3. Storage

```
<app_data>/taskroot/
├── settings.json   # AppSettings, one object
├── ui-state.json   # { "<key>": <value> }  (replaces the ui_state table)
├── account.json    # server URL, username, home-set, principal (no password)
└── snapshot.json   # read-only fallback: href -> { collection, etag, raw }
```

- **Radicale is the source of truth.** All collections are fetched over CalDAV into memory at startup (two `multiget` batches today, about eight after the import). Edits are merged into the in-memory `raw` and `PUT` immediately; there is no per-resource mirror and no pending queue.
- `snapshot.json` is a cache, nothing else: written only after a successful full load, read only when the server is unreachable at startup, never merged with remote state. Delete it and the app rebuilds it from Radicale.
- `settings.json` is one object with `#[serde(default)]`. If it fails to parse, rename it to `settings.json.bad` and start from defaults — never half-apply fields. `ui-state.json` is a flat map and ignores unknown keys.
- A legacy `taskroot.db` is imported once (§11) and then ignored.

**Why not read Radicale's storage folder directly.** The layout — `collection-root/<user>/<collection>/<item>.ics` plus `.Radicale.props`, `.Radicale.cache` and `.Radicale.lock` — is Radicale's storage backend, not a contract:

- The app would have to find `filesystem_folder` in Radicale's config and would only ever work against Radicale on this machine.
- Writing items directly bypasses `.Radicale.lock`, which is how Radicale serialises its own writers (including the phone's sync). A simultaneous phone sync and laptop edit can clobber each other; through CalDAV, Radicale serialises both and gives us `If-Match`/`412`.
- There are no ETags on disk, so conflict detection degrades to mtime guessing.
- A Radicale upgrade or a different storage backend (Docker volume, NAS, another server) breaks the app silently.

CalDAV costs two loopback requests and keeps all of that working. `.Radicale.props` also shows why: it is a JSON map with namespaced keys (`C:supported-calendar-component-set`, `ICAL:calendar-color`, `D:displayname`), i.e. an internal encoding we would be re-parsing.

## 4. Data model

Three layers, do not blur them:

1. **Wire** — `icalendar::Event`/`Todo` + the raw string. Lives only inside `ics/`. Never serialised to the frontend.
2. **Store** — `Resource { href, collection, etag, raw, items }`, one per `.ics` resource, in memory, where `items` is `Vec<AppEvent>` or `Vec<AppTask>` according to the collection's component type. `raw` is kept so the write path can merge into the exact bytes that were fetched. This makes "one PUT writes the whole resource" structural instead of a rule someone must remember; `snapshot.json` is just a serialized copy of this map and is never consulted while the server is reachable.
3. **View** — the existing `AppEvent` / `EventInstance` / `AppTask` / `AppCalendar` DTOs, close to their current shape, minus their SQL derives.

```rust
struct Store {
    resources: HashMap<String /* href */, Resource>,  // VEVENT and VTODO alike
    calendars: Vec<AppCalendar>,                      // href = collection id, typed
    settings: AppSettings,
    ui_state: Map<String, Value>,
}

// Projections over `resources`: events() and tasks().
```

**Keys.** All local ids are derived from ICS content, so a pull is idempotent and needs no lookup:

- `AppEvent.id` = `UID` (master) or `"<UID>@<RECURRENCE-ID>"` (exception).
- `AppTask.id` = `UID`.
- `remote_id` (both) = resource href; `remote_collection_id` = collection href.
- `AppCalendar.id` = collection href, plus its component type. `writable` replaces `access_role`.
- New resources use `href = <collection>/<uid>.ics`, so a retried create cannot duplicate.

One shared shape for both component kinds: a resource is parsed into `AppEvent`s or `AppTask`s depending on the collection's component type, and both are written back through the same line-preserving merge.

**Deleted from the DTOs:** `dirty` (there is no dirty flag any more: an edit is a `PUT`, and the only failure mode is a reported error), `access_role`, `is_primary`, and every `sqlx` derive. `AppCalendar` gains `writable: bool` and `component` (`VEVENT`/`VTODO`). `AppTask.task_list_id` becomes `remote_collection_id`, matching `AppEvent`. `exdates` **stays and becomes canonical** — see below.

Because ids are derived from ICS content, a pull needs no id lookup (`get_event_by_remote_id` and the `google_` prefix disappear), and ids survive href changes.

`exdates` is the one intentional change to the render path: it becomes the single home of exclusion dates (the parser fills it from `EXDATE`, the writer emits it back, and `get_occurrences`/`get_all_day_occurrences` take it as a slice instead of scanning the RRULE string). This fixes a live bug — the inspector's "delete this occurrence" action (`applyRecurringInstance`) writes `exdates` today, but the Google write path never sent it, so the occurrence silently comes back on the next pull. With CalDAV it works with ~20 lines of wiring.

## 5. CalDAV client (`caldav/`, target ~450 lines + tests)

| File | Responsibility |
|---|---|
| `transport.rs` | One long-lived `reqwest::Client` in managed state; Basic auth; returns `{ status, etag, body }`; maps 401→`auth`, 412→`conflict`, 410→`gone` |
| `xml.rs` | Four request bodies as constants; response parsers via `roxmltree`: multistatus properties, sync-collection (href + status), multiget (href + etag + body), privileges |
| `discovery.rs` | `/.well-known/caldav` → `current-user-principal` → `calendar-home-set` → Depth:1 collections (`displayname`, `resourcetype`, `supported-calendar-component-set`, `calendar-color`, `current-user-privilege-set`) |
| `client.rs` | `pull_collection`, `fetch_resources`, `put_resource`, `delete_resource` |

Rules:

- Discovery runs **once** at account setup (plus an explicit "refresh calendars" button) and its result is stored in `account.json`. Normal sync never rediscovers.
- Both VEVENT and VTODO collections are fetched, typed from `supported-calendar-component-set`. A collection that advertises both is treated as VEVENT-only and logged.
- The account gate stays, repurposed as "connect your CalDAV server" on first run. Every resource therefore belongs to a real collection; there is no local-only storage mode to design around.
- No provider trait, no generic sync entity, no `SyncAction` enum. A put and a delete are the only operations.

## 6. Sync (`sync/mod.rs`, keep the existing shell)

Keep the current poller/status machinery: 5-minute `interval`, `SyncTrigger` debounce, `SyncStateManager`, `SYNC_STARTED/FINISHED/ERROR`. Replace only `sync_with_google` with `sync_all`. Also honour the existing `sync_interval` setting here (it is currently dead).

**Startup load.** `PROPFIND` every collection, `calendar-multiget` everything (this data set: ~400 resources, eight batches), parse into memory, write `snapshot.json`. If the server is unreachable, load the snapshot, mark the UI offline, and retry in the background. No background refresh after a successful load — that avoids a reload clobbering an edit in flight.

**Per cycle: push edits, then pull.**

**Pull** (per collection, tokens live in memory; a restart is just the startup load):

1. `REPORT sync-collection` (Depth 1, props: `getetag`); no token → step 5.
2. For each returned href: `200` → collect for multiget, `404` → remove the resource from memory.
3. `REPORT calendar-multiget` in batches of 50; replace the in-memory `Resource` (re-parse `raw`, update `etag`); write the snapshot once at the end.
4. Keep the new sync token only after the whole collection applied. A crash just replays the delta (idempotent).
5. `403`/`501` on REPORT, or `410 Gone` → drop the token, `PROPFIND Depth:1`, multiget everything.

**Push** (immediately on edit, one resource at a time):

1. Merge the edit into the in-memory `raw` (§7).
2. `PUT` with `If-Match: <last etag>` (create: `If-None-Match: *`).
3. On success store the returned ETag and the new `raw`, and write the snapshot.
4. On `412` re-fetch, re-merge, retry once; a second failure returns an error to the UI. The edit is never silently kept.
5. Delete: `DELETE`, then drop the resource; `404` counts as success.

There is no offline edit queue: if Radicale is down, the command fails and the UI says so. Reads still work from the snapshot.

## 7. ICS read/write — the three invariants

These are the whole reason this migration is risky. Encode them as tests, not comments.

**I1. `PUT` replaces the entire resource.** Never serialise an `AppEvent` and send it. Always read-modify-write: start from the resource body, change only the properties Taskroot owns (`SUMMARY`, `DESCRIPTION`, `DTSTART`, `DTEND`, `RRULE`/`EXDATE`, `STATUS`, `COLOR`), leave everything else byte-identical. This is the CalDAV equivalent of the `events.patch` rule already documented in `AGENTS.md`.

**I2. A resource is a set of components.** A master and its exceptions share a UID and live in one file. Editing one row must rewrite the resource, never emit a one-component file.

**I3. Do not round-trip unknown components through `icalendar`.** Its typed model has no `VTIMEZONE` variant (only `Event`/`Todo`/`Venue`), so parse→serialise silently drops timezone blocks. Consequence:

- **Read** with `icalendar` (typed `DTSTART`/`RRULE`/`RECURRENCE-ID` extraction).
- **Write** with a small line-preserving editor (`ics/write.rs`, ~150 lines): unfold, locate the target component by UID + RECURRENCE-ID, replace/insert only owned properties, refold, restore CRLF. Unknown properties, `VALARM`, `VTIMEZONE`, `ATTENDEE` survive untouched.
- **Create** a new resource with `icalendar`'s builder (nothing to preserve), with a `DTSTAMP` and a UID generated from the local id.

`dtstamp` is required and currently has no home; generate it on every write and do not persist it.

## 8. Timezones

Store the wall clock and the `TZID` — never "convert to UTC, keep the RRULE". `DTSTART:...Z` + `RRULE` expands in UTC, so a weekly 09:00 event shifts to 08:00 across a DST change. (This bug exists today: `rrule_utils.rs` anchors timed recurrences with `%Y%m%dT%H%M%SZ`.)

- Parse `DTSTART`/`DTEND`/`EXDATE` with their `TZID`; keep `AppEvent.timezone` as the IANA id.
- Expand timed recurrences in the event's zone (extend `get_occurrences` to take a `Tz`, mirroring what `get_all_day_occurrences` already does), then convert each occurrence to UTC for `EventInstance`.
- Emit `DTSTART;TZID=<zone>` on write. Existing resources keep their original `VTIMEZONE` verbatim (I3). New recurring resources are written without a `VTIMEZONE` — accepted by Radicale and resolvable by clients from their tz database. This is fine for your single-zone (IST, no DST) calendar; generating `VTIMEZONE` blocks is deferred (§15).
- All-day events stay floating dates, as today.

## 9. Tasks (VTODO)

Tasks sync in v1. `tasks.org` already writes VTODO resources into Radicale, so a laptop that ignored them would be permanently wrong. Tasks use the same store, the same pull/push pipeline, and the same line-preserving writer as events; only the field mapping below is new.

Mapping, chosen so the phone does not lose anything the phone can show:

| Taskroot | VTODO | Rule |
|---|---|---|
| `id` | `UID` | generated once, never rewritten |
| `remote_id` | resource href | |
| `title` | `SUMMARY` | |
| `notes` | `DESCRIPTION` | plain text |
| `due` | `DUE` | date-only (`DUE;VALUE=DATE:YYYYMMDD`), which is what tasks.org writes and what Taskroot stores |
| `status` | `STATUS` + `COMPLETED` + `PERCENT-COMPLETE` | see below |
| `priority` | `PRIORITY` | bucket mapping, lossless at Taskroot's granularity |
| `tags` | `CATEGORIES` | names only; tag colours stay local, keyed by name |
| `parent_task` | `RELATED-TO;RELTYPE=PARENT` | UID ↔ `TaskId`; this *is* tasks.org's subtask relation |
| `dependencies` | `X-TASKROOT-DEPENDENCIES` | **not** `RELATED-TO` — see the reltype note below |
| `added` | `CREATED` (fall back to `DTSTAMP`) | tasks.org often omits `CREATED` |
| `updated_at` | `LAST-MODIFIED` | |
| `est`, `canvas_x/y`, `on_canvas`, `tabs` | `X-TASKROOT-*` | app-only, tolerated to be lost |

**Completion is the only synced status.** `STATUS:COMPLETED` + `COMPLETED` + `PERCENT-COMPLETE:100` when `Done`; `STATUS:NEEDS-ACTION` otherwise, clearing the completion properties. Taskroot's `Todo` / `NextUp` / `Doing` are a local workflow (and `Doing` is driven by the stopwatch — syncing it would `PUT` on every timer start), so they travel in `X-TASKROOT-STATUS`. Phone completion round-trips; phone un-completion becomes `Todo`. A phone edit that drops `X-` properties degrades gracefully to `Todo` rather than corrupting anything.

**Priority** maps by bucket and round-trips: `None→0`, `Low→7`, `Medium→5`, `High→3`, `Urgent→1`; on read `1–2→Urgent`, `3–4→High`, `5→Medium`, `6–9→Low`, `0→None`. No `X-` property needed.

**Checklist feature removed.** 0 of the 86 tasks in the current database have checklist items (also 0 have a parent or tags), and tasks.org has no equivalent concept. So the `checklist` field, `ChecklistInput.svelte`, the `TaskRow` badge and the inspector section are deleted rather than mapped. If it ever returns, it goes in `X-TASKROOT-CHECKLIST` (JSON) — not markdown in `DESCRIPTION`: markdown was inherited from the first plan's assumption that clients render it, which tasks.org does not (its `DESCRIPTION` holds plain notes/URLs).

**Tags** are serialised by name into `CATEGORIES`; tag colours and ids stay in `settings.json` keyed by name. Renaming a tag on the phone therefore creates a new local tag, which is acceptable for one user.

**Subtasks are the task tree, not the checklist.** `AppTask.parent_task` already renders as a tree in `TaskListPane` and is editable in the inspector, so this maps 1:1 to tasks.org subtasks: a child created on the phone appears under its parent here, and vice versa. Taskroot's old checklist was a different concept (lightweight checkboxes with no title/dates/status); it is being removed (§9) rather than mapped.

**Due dates** are date-only in both apps: tasks.org writes `DUE;VALUE=DATE:20260205` when the user picks a date without a time ([tasks#4139](https://github.com/tasks/tasks/issues/4139)). Keep that string as-is and never convert it to a timestamp. A `DUE` that carries a time (tasks.org supports due times) is preserved verbatim and displayed by its date. Do not "fix" the RFC's exclusive-`VALUE=DATE` semantics — both clients here are tasks.org-family, so preserving what is there is the compatible choice. Separately, `TaskRow.svelte` currently compares `task.due` (a date) against `new Date().toISOString()` (a timestamp), which marks a task due *today* as overdue; fix that while touching the due path.

**Relations: tasks.org clobbers RELTYPE.** It reads any `RELATED-TO` as a parent relation and rewrites the reltype to `PARENT` on its next edit ([tasks#2527](https://github.com/tasks/tasks/issues/2527)). So `parent_task` ↔ `RELATED-TO;RELTYPE=PARENT` is safe, but a `DEPENDS-ON` written by us would be silently turned into a subtask of the wrong task. Taskroot's `dependencies` therefore lives in `X-TASKROOT-DEPENDENCIES` (a JSON array of UIDs), which tasks.org leaves alone as an unknown property.

The in-memory task filter/sort/search replaces `db/task_filters.rs` (~60 lines instead of a `QueryBuilder`). `enable_tasks_sync` is deleted; all discovered VTODO collections sync, with `default_task_collection` choosing where new tasks go.

**Observed from the live Radicale folder** (46 tasks, one `SqrtTasks` VTODO collection, one `SqrtCal` VEVENT collection): `PRODID:DAVx5/4.5.21-ose ical4j/4.3.0 (org.tasks)`; per task `DTSTAMP`, `PRIORITY`, `SEQUENCE`, `STATUS`, `SUMMARY`, `UID`, and often `X-APPLE-SORT-ORDER`; completion as `STATUS:COMPLETED` + `COMPLETED` + `PERCENT-COMPLETE:100`; descriptions are plain notes (URLs); tags are single simple names (`CATEGORIES:lists`). No `DUE` and no `RELATED-TO` appear **yet** — those tasks simply don't use them today; both are supported by tasks.org and mapped above, so the first due date or subtask created on the phone must round-trip. Consequences: the writer must preserve `X-APPLE-SORT-ORDER` (it does, as an unknown property) and `added` needs the `DTSTAMP` fallback.

## 10. Frontend

Small diff, by design.

- `login/+page.svelte` becomes the CalDAV connect screen: server URL, username, optional password, "Test connection" (runs discovery and lists collections), "Connect". The gate stays (`is_logged_in` → `is_account_configured`), because every resource now lives in a remote collection.
- Settings gains an account card (server, username, change password, remove account) and two pickers: `default_calendar` and `default_task_collection`, replacing the Google `isPrimary` concept.
- `isCalendarReadOnly` reads `writable === false`; `resolveEventCalendar` drops the `'primary'` alias.
- Delete the `OAUTH_URL` listener and the `reset_auth` calls in the settings row and dev page.
- Delete the inspector's Checklist section, `ChecklistInput.svelte` and the `TaskRow` checklist badge (§9).
- No other inspector changes beyond the read-only check: "delete this occurrence" is fixed by the backend `exdates` wiring above, not by new UI.
- Everything else — `EventInstance`, date grid, timeline, drag/resize, stopwatch, task filtering — is untouched.

## 11. Migration of existing data

The old SQLite is not a superset of Radicale, so this is a real import, not a skip:

- SQLite: 86 tasks + 374 events, every row carrying a Google `remote_id`.
- Radicale today: 46 tasks in `SqrtTasks` (from tasks.org) and 26 events in `SqrtCal` (a partial Google import, UIDs `dsh-…@dsh-import`).

Rules:

- **Events** are imported: group rows by recurring series (`recurring_event_id`), emit one resource per series with master + exceptions, and `PUT` into `SqrtCal`. Each resource's UID is derived deterministically from the master's Google `remote_id` (`<remote_id>@taskroot-import`), so a re-run addresses the same resources instead of duplicating. The `dsh-…@dsh-import` batch is matched by UID or title + start and skipped.
- **Tasks** are imported into a separate `Google Tasks (imported)` collection, created on demand, so the 46 live tasks.org tasks are never mixed with the old Google set. Delete that collection once triaged.
- The importer is a **throwaway** (`src-tauri/examples/import_google.rs` reusing the `caldav`/`ics` modules), not an app command: run it once, verify, delete it. It never ships.
- The Google calendar split (Holidays / Routine / Busy / Major Events) collapses into one collection. Event colours survive because they are denormalized per event. One collection per Google calendar is possible but needs `MKCALENDAR` + property writes — defer unless the collapse feels wrong in practice.
- `settings` / `ui_state` rows → the two JSON files.
- Rename the DB to `taskroot.db.imported`; delete the importer and the file in the next release.

Open question: whether the 86 old Google tasks are wanted at all (they may already have been triaged into tasks.org). Importing them into their own collection is the no-data-loss default.

## 12. Phases

Each phase ends green: `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `bun run check`. Commit the regenerated bindings.

Usability between phases is **not** required. The order below is the fast path; the only rule is that P4 lands after the import has been verified, and SQLite is deleted last so there is always a copy of the old data.

**P0 — Delete Google.** Remove `apis/google_*`, `auth.rs`, `oauth2`, the queue (`sync/{queue,queue_store,drain,push,types}.rs`), the login route, the OAuth event/listener, `google_*` settings rows, and the unused checklist UI. Replace `sync_with_google` with a no-op `sync_all`. SQLite, the DTOs and local CRUD stay untouched, so the app still works offline. *Gate: local CRUD works with no network and no Google code left.*

**P1 — CalDAV transport + discovery + account setup.** `transport.rs`, `xml.rs`, `discovery.rs`, account commands, `account.json`, OS keychain, and the connect screen + account card in Settings (replacing the old login page). No calendar data flows yet. *Gate: discovery against the real Radicale; XML parsers tested from captured fixtures; account survives a restart.*

**P2 — ICS.** `ics/read.rs` (resource → `AppEvent`/`AppTask`, `exdates` filled from `EXDATE`), `ics/write.rs` (line-preserving merge for both component kinds, `exdates` → `EXDATE`), and the expansion change that takes `exdates` directly. Fixtures copied from `C:\radicale`. No behaviour change yet. *Gate: I1–I3 fixture tests pass.*

**P3 — Import (re-runnable).** The one-off `SQLite → Radicale` push (§11): 374 events grouped by series, 86 tasks into `Google Tasks (imported)`. The app still reads SQLite, so nothing changes for you while this runs; the phone starts seeing the data. *Gate: Radicale counts match the DB, the phone shows them, and a second run is a no-op.*

**P4 — Cutover.** In-memory `Store` loaded from CalDAV at startup, `snapshot.json` fallback, `sync-collection` pull, immediate `PUT` on edit, JSON settings/UI state, `AppError::Store`. Reads and writes now go to Radicale; SQLite is still on disk but unused. *Gate: full use of the app against Radicale; stopping Radicale shows the snapshot read-only with an offline notice; edits round-trip to the phone.*

**P5 — Delete SQLite.** Remove `db/`, `sqlx`, the migration/`ensure_column` machinery, the DB import path, and the SQL attributes in the `Queryable` macro. *Gate: build has no `sqlx`; nothing reads `taskroot.db`.*

**P6 — Polish + docs.** `writable` read-only handling, `default_calendar`/`default_task_collection` pickers, sync/offline status, dev-page cleanup; update `AGENTS.md`, `src-tauri/AGENTS.md`, `README.md`, delete `docs/google-calendar-write.md`, add `docs/caldav.md`.

## 13. Testing

Highest value first.

1. **Round-trip fixtures** (`ics/fixtures/`): copy real resources out of `C:\radicale\...` — a Mozilla-style event with a full historical `VTIMEZONE`, `EXDATE`, `RRULE` and `X-MOZ-GENERATION`; the `dsh-…@dsh-import` all-day event with `CATEGORIES`/`TRANSP`; and a tasks.org VTODO with `X-APPLE-SORT-ORDER`. Edit one property; assert the output differs only in that property. Repeat for: move a timed event, edit an exception, delete an exception, delete a master, complete a task, set a due date, attach a subtask. This is the only test class that catches I1–I3.
2. **Pure parser tests** for the four XML responses, using captured Radicale bodies (not hand-written).
3. **HTTP contract tests** with a `wiremock` dev-dependency: 207 sync-collection, 207 multiget, 410 Gone, 412 conflict, 401. No Docker, no Radicale in CI.
4. **Store tests**: snapshot round-trip, create/update/delete, idempotent load, legacy import.
5. **One `#[ignore]`d integration test** against a real Radicale for discovery quirks.
6. Property test: arbitrary property sets survive the line editor (`proptest` is already a dev-dependency).

## 14. Performance budget

- Startup = read `snapshot.json` (< 20 ms) then, normally, a full load from loopback (~400 resources → 8 multiget batches, well under 150 ms). Measure once in a test.
- Queries are `Vec` scans over a few hundred items; filtering/sorting in memory is fine.
- One `reqwest::Client` for the whole process; no client-per-request.
- No indexes, no query cache, no lazy loading, no incremental projection, no disk mirror — none of it is worth the code at this size. If startup ever regresses, fetch lazily on the first screen instead.

## 15. Deferred

- Generated `VTIMEZONE` for new recurring events (only matters for DST zones).
- Multi-account, `MOVE`, `calendar-query` windows, CTag, ETag maps, conflict UI.
- Collapsing `AppEvent`/`EventInstance` into a single type. Worth doing eventually; not worth the frontend churn now.

## 16. Risks

| Risk | Mitigation |
|---|---|
| A write path drops unknown properties | I1–I3 fixtures block merge until green; the line editor only touches owned properties |
| tasks.org rewrites `RELATED-TO` reltypes | `dependencies` stays in `X-TASKROOT-DEPENDENCIES`; only `PARENT` uses a standard relation |
| Due dates disagree about inclusive/exclusive | preserve the `DUE;VALUE=DATE` string as written; never convert or recompute |
| tasks.org rewrites a task and drops `X-TASKROOT-*` | Those fields are cosmetic or degrade to `Todo`; status/priority/tags all use standard properties |
| Task loop: stopwatch changes status → `PUT` storm | Only completion syncs; `Todo`/`NextUp`/`Doing` stay local (§9) |
| Timezone regressions | `TZID` end-to-end; recurrence tests in a DST zone even though you live in IST |
| Radicale unreachable at launch | Load `snapshot.json`, show "offline, last synced HH:MM", retry in the background |
| Edit while Radicale is down | Command fails and the UI reports it; the edit is not silently kept |
| Crash mid-edit | The only durable write is the `PUT`; in-memory state is rebuilt on next launch |
| Losing current data | One-time import in P5, DB kept as `.imported`, verified against counts |

## 17. Decisions on record

- **No libdav.** ~450 lines of transport/XML is smaller than a dependency whose error types are explicitly unstable. Revisit only if XML handling exceeds ~700 lines.
- **No SQLite, no mirror.** Radicale is the database, memory is the view, `snapshot.json` is a cache. One source of truth, no projection drift.
- **No offline queue.** An edit is a `PUT`; if it fails the UI says so. The queue existed to bridge a remote provider, which is no longer the case.
- **No accounts table / provider column.** `account.json` holds one account; the password lives in the OS keychain.
- **VTODO in v1.** tasks.org already writes tasks to Radicale; the field mapping in §9 is the price, and it is small.
- **Checklist removed.** Never used (0/86 tasks), no counterpart on the phone; real subtasks cover the need.
- **No generic sync trait.** One CalDAV client, called directly.
- **No entity type churn.** DTOs keep their field names, so the frontend stays as it is and `expand.rs` changes only for `exdates` and timezone anchoring (§4, §8).
