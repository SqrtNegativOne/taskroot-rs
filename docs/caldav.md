# CalDAV

Taskroot talks to one CalDAV server. The local Radicale instance is the single source of
truth; Taskroot never reads Radicale's storage folder.

## Deployment

- Radicale listens on `127.0.0.1:5232` and is reached with HTTP Basic auth.
- The phone reaches the same server through `tailscale serve` at
  `https://sqrt-1.tail03279c.ts.net` with DAVx5 (tasks.org for VTODO, a calendar app for
  VEVENT).
- Taskroot stores the account in `account.json` and the password in the OS keychain
  (Windows Credential Manager, macOS Keychain, Secret Service on Linux).
- Plain `http` is accepted only for loopback or `*.ts.net`; any other host must use `https`.

The app data directory (`<app_data>/taskroot/` on Linux, the identifier directory on
Windows/macOS) contains:

| File | Purpose |
|---|---|
| `settings.json` | `AppSettings`, one object. Malformed → renamed `settings.json.bad`, defaults used. |
| `ui-state.json` | Flat `{ "<key>": <value> }` map for per-component UI state. |
| `account.json` | Server URL, username, home-set, principal and discovered collections. No password. |
| `snapshot.json` | Read-only cache: `href -> { collection, etag, raw }`. Never merged with remote state. |

Discovery runs at account setup and when the user explicitly refreshes calendars. A normal
sync never rediscovers.

## Data flow

1. At startup Taskroot `PROPFIND`s each collection, `calendar-multiget`s every resource,
   parses the raw ICS into memory, and writes `snapshot.json` once.
2. If the server is unreachable, Taskroot loads `snapshot.json`, marks the UI offline and
   retries in the background.
3. Edits merge into the in-memory raw resource and `PUT` immediately. There is no offline
   edit queue: if Radicale is down the command fails and the UI reports it.
4. The poller does `sync-collection` per collection (5-minute default, honouring the
   `sync_interval` setting), falling back to a full `PROPFIND` + multiget on `403`/`410`/`501`.
5. `PUT` updates use `If-Match: <etag>`; creates use `If-None-Match: *`. A `412` re-fetches,
   re-merges and retries once; a second failure is reported, never silently kept.
6. Deletes are `DELETE`; `404` counts as success. Deleting a master deletes its whole
   resource (exceptions live in the same file). Deleting an exception removes just that
   component.

HTTP status mapping: `401 → auth`, `412 → conflict`, `410 → gone`, other non-2xx → `sync`.

## The three invariants

**I1 — `PUT` replaces the entire resource.** Never serialise an `AppEvent`/`AppTask` and
send it. Start from the resource body and change only the properties Taskroot owns
(`SUMMARY`, `DESCRIPTION`, `DTSTART`, `DTEND`, `RRULE`/`EXDATE`, `STATUS`, `COLOR`, plus the
VTODO mapping below). Everything else stays byte-identical: `VTIMEZONE`, `VALARM`,
`ATTENDEE`, unknown `X-*` properties, component order.

**I2 — a resource is a set of components.** A master and its exceptions share a UID and
live in one file. Editing one row rewrites the resource; it never emits a one-component
file that drops the other components.

**I3 — unknown components are not round-tripped through `icalendar`.** The typed model has
no `VTIMEZONE` variant, so parse→serialise would silently drop timezone blocks. Taskroot
reads with `icalendar` (typed `DTSTART`/`RRULE`/`RECURRENCE-ID` extraction) and writes with
a line-preserving editor: unfold, locate the target component by UID + `RECURRENCE-ID`,
replace/insert/remove only owned property lines, refold at 75 octets, restore CRLF.
New resources are built with `icalendar`'s builder because there is nothing to preserve.

`DTSTAMP` is generated on every write and never persisted.

## Timezones

Store the wall clock and the `TZID`; never convert a recurring event to UTC before
expanding it. A weekly 09:00 event must stay 09:00 across a DST change, so timed
recurrences expand in the event's IANA zone and each occurrence is converted to UTC only
for the render DTO. Existing resources keep their original `VTIMEZONE` verbatim (I3).
New recurring resources are written without a `VTIMEZONE`; clients resolve the TZID from
their own tz database. All-day events stay floating dates.

`exdates` is the canonical home of exclusion dates: the parser fills it from `EXDATE`, the
writer emits it back, and expansion consumes it directly. This is what makes the
inspector's "delete this occurrence" action survive the next pull.

## VTODO mapping

| Taskroot | VTODO | Rule |
|---|---|---|
| `id` | `UID` | generated once, never rewritten |
| `remote_id` | resource href | |
| `title` | `SUMMARY` | |
| `notes` | `DESCRIPTION` | plain text |
| `due` | `DUE` | preserve the raw value; date-only stays `YYYY-MM-DD` |
| `status` | `STATUS` + `COMPLETED` + `PERCENT-COMPLETE` | only completion syncs; `Todo`/`NextUp`/`Doing` travel in `X-TASKROOT-STATUS` |
| `priority` | `PRIORITY` | `None→0`, `Low→7`, `Medium→5`, `High→3`, `Urgent→1`; reverse `1–2→Urgent`, `3–4→High`, `5→Medium`, `6–9→Low`, `0→None` |
| `tags` | `CATEGORIES` | names only; colours stay local keyed by name |
| `parent_task` | `RELATED-TO;RELTYPE=PARENT` | tasks.org's subtask relation |
| `dependencies` | `X-TASKROOT-DEPENDENCIES` | JSON array of UIDs; **not** `RELATED-TO` because tasks.org rewrites reltypes |
| `added` | `CREATED`, fallback `DTSTAMP` | tasks.org often omits `CREATED` |
| `updated_at` | `LAST-MODIFIED` | |
| `est`, `canvas_x/y`, `on_canvas`, `tabs` | `X-TASKROOT-*` | app-only, tolerated to be lost |

An event's task attachment is stored in `X-TASKROOT-TASK-ID` (app-only).

Checklist items are not mapped and have been removed from the UI. Real subtasks are the
task tree (`parent_task` ↔ `RELATED-TO;RELTYPE=PARENT`).

## Testing

- `src-tauri/src/caldav/fixtures/` holds real Radicale XML responses (PROPFIND, multiget,
  sync-collection, error bodies). Parsers are tested against those, not hand-written XML.
- `src-tauri/src/ics/fixtures/` holds real resources: a Mozilla event with a historical
  `VTIMEZONE`/`EXDATE`/`RRULE`/`X-MOZ-GENERATION`, an all-day event, and a tasks.org VTODO
  with `X-APPLE-SORT-ORDER`.
- The round-trip tests edit exactly one property and assert the output differs only in
  that property (I1), that every component in a multi-component resource survives (I2), and
  that unknown properties/components survive the line editor (I3).
- One `#[ignore]`d integration test runs discovery against a real Radicale.
