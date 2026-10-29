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

A debug build whose app-data directory is not writable (a sandboxed dev run) keeps this
directory next to the executable in `dev-data/` instead, copying `account.json`,
`settings.json` and `ui-state.json` over once. `TASKROOT_DATA_DIR` overrides both.

Discovery runs at account setup and on every load (startup, Refresh, and after a list is
created) to pick up the server-assigned collection. There is no background sync.

## Lists

Lists are CalDAV collections. `create_calendar` sends `MKCALENDAR` for a new
`<home>/<uuid>/` and re-reads the home; `rename_calendar` and `set_calendar_color`
`PROPPATCH` `displayname` and `ical:calendar-color` (clearing the colour removes the
property); `delete_calendar` `DELETE`s the collection and drops its resources from the
store. `store.calendars` and `account.json` are rewritten together after every change.
Read-only collections refuse these edits with `invalid-input` before any request is sent.

## Data flow

1. On start (and on Refresh) Taskroot re-reads the collection list with one `PROPFIND` of
   the stored calendar home (~40–70 ms against a local Radicale, capped at 3 s),
   rewrites `account.json` when it changed, and falls back to the cached list when the
   server cannot be asked. `account.json` is therefore a cache, not the truth: a list
   created or deleted elsewhere is picked up at the next load.
2. It then sends one `calendar-query` REPORT per collection asking for every resource's
   `ETag` and body, and parses the raw ICS into memory. A failed load leaves the
   previously loaded resources untouched.
3. Edits merge into the in-memory raw resource and `PUT` immediately. There is no offline
   edit queue: if Radicale is down the command fails and the UI reports it.
4. There is no poller and no change hook. Changes made by another client become visible
   when the user presses Refresh (or restarts the app).
5. `PUT` updates use `If-Match: <etag>`; creates use `If-None-Match: *`. A `412` re-fetches,
   re-merges and retries once; a second failure is reported, never silently kept.
6. Deletes are `DELETE`; `404` counts as success. Deleting a master deletes its whole
   resource (exceptions live in the same file). Deleting an exception removes just that
   component.

HTTP status mapping: `401 → auth`, `412 → conflict`, `410 → gone`, other non-2xx → `remote`.

## The three invariants

**I1 — `PUT` replaces the entire resource.** Never serialise an `AppEvent`/`AppTask` and
send it. Start from the resource body and change only the properties Taskroot owns
(`SUMMARY`, `DESCRIPTION`, `DTSTART`, `DTEND`, `RRULE`/`EXDATE`, `STATUS`, `COLOR`, plus the
VTODO mapping below). Everything else stays byte-identical: `VTIMEZONE`, `ATTENDEE`, unknown
`X-*` properties, component order. A `VALARM` is written only when the task's reminder list
changed; otherwise it stays byte-identical too.

**I2 — a resource is a set of components.** A master and its exceptions share a UID and
live in one file. Editing one row rewrites the resource; it never emits a one-component
file that drops the other components.

**I3 — unknown components are not round-tripped through `icalendar`.** The typed model has
no `VTIMEZONE` variant, so parse→serialise would silently drop timezone blocks. Taskroot
reads with `icalendar` (typed `DTSTART`/`RRULE`/`RECURRENCE-ID` extraction) and writes with
a line-preserving editor: unfold, locate the target component by UID + `RECURRENCE-ID`,
replace/insert/remove only owned property lines, refold at 75 octets, restore CRLF.
Nested components are matched by position inside their parent, so a `VALARM`'s lines can
never be mistaken for the `VTODO`'s — `BEGIN`/`END` depth is tracked exactly.
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
inspector's "delete this occurrence" action survive the next pull. The value it writes is
the instance's `occurrenceStart` (the clicked occurrence's canonical value), never the
master's `DTSTART` — otherwise "this event" would exclude the first occurrence of the
series instead of the one the user picked.

### Scoped recurring edits

Occurrence identity is part of the write contract, so the UI never composes `.ics` text.
`delete_event` and `reschedule_event` take an `EventScope` (`this`, `following`, `all`) plus
the clicked `occurrenceStart`:

- `this` — delete appends an `EXDATE` for that value (and removes the exception row when
  one was selected); a move writes a `RECURRENCE-ID` exception at the original slot with
  the new `DTSTART`/`DTEND`, so the rest of the series keeps its times.
- `following` — delete rewrites `RRULE` with `UNTIL` one second (one day for all-day events)
  before the occurrence and drops exceptions at or after it. `UNTIL` is written in UTC for a
  `TZID` series, as a bare date for all-day, and floating for a zone-less series. A move is
  rejected: re-anchoring a series from one occurrence is not supported yet.
- `all` — delete drops every component sharing the UID (the resource goes too when nothing
  remains); a move shifts the master's `DTSTART`/`DTEND`.

The canonical `occurrenceStart` shapes are the same ones `exdates` uses, so the values a
projection returns can be written straight back.

## VTODO mapping

| Taskroot | VTODO | Rule |
|---|---|---|
| `id` | `UID` | generated once, never rewritten |
| `remote_id` | resource href | |
| `title` | `SUMMARY` | |
| `notes` | `DESCRIPTION` | plain text; the `VALARM`'s own `DESCRIPTION` is a different line |
| `start` | `DTSTART` | tasks.org's start ("hide until"); date-only stays `YYYY-MM-DD` |
| `due` | `DUE` | preserve the raw value; date-only stays `YYYY-MM-DD` |
| `timezone` | `TZID` of `DUE`, else of `DTSTART` | `DUE` wins when a resource mixes zones; a `…Z` value reads as `UTC` so its instant survives the write |
| `status` | `STATUS` + `COMPLETED` + `PERCENT-COMPLETE` | only completion syncs; `Todo`/`NextUp`/`Doing` travel in `X-TASKROOT-STATUS` |
| `completed_at` | `COMPLETED` | kept across edits, so retitling a finished task does not move its completion time |
| `priority` | `PRIORITY` | `None→` omitted, `Low→7`, `Medium→5`, `High→3`, `Urgent→1`; reverse `1–2→Urgent`, `3–4→High`, `5→Medium`, `6–9→Low`, `0→None` |
| `tags` | `CATEGORIES` | names only; colours stay local keyed by name |
| `rrule` | `RRULE` | wire form, one line per rule; Taskroot writes it but does not expand a task series |
| `geo` | `GEO` | `latitude;longitude`, the point tasks.org geofences at |
| `alarms` | `VALARM` `TRIGGER` | `relative` = `TRIGGER;RELATED=START` or `;RELATED=END` with `-PTnM` (minutes *before*, negative = after), `absolute` = `TRIGGER;VALUE=DATE-TIME:…Z`; `ACTION`/`DESCRIPTION` of an existing alarm are left alone, a new one gets tasks.org's `DISPLAY` + description |
| `parent_task` | `RELATED-TO;RELTYPE=PARENT` | tasks.org's subtask relation |
| `dependencies` | `X-TASKROOT-DEPENDENCIES` | JSON array of UIDs; **not** `RELATED-TO` because tasks.org rewrites reltypes |
| `added` | `CREATED`, fallback `DTSTAMP` | tasks.org often omits `CREATED` |
| `updated_at` | `LAST-MODIFIED` | |
| `est`, `canvas_x/y`, `on_canvas`, `tabs` | `X-TASKROOT-*` | app-only, tolerated to be lost |

`VALARM` is only rewritten when `alarms` differs from what the resource already held, so an
alarm whose trigger Taskroot cannot express (e.g. `-PT45S`) survives every unrelated edit.
`SEQUENCE`, `LOCATION`, `URL`, `CLASS`, `CONTACT` and every other unlisted property are not
owned and travel through untouched.

Completing a task whose `RRULE` is set is refused for now: tasks.org completes one by rolling
the series forward, and a plain `STATUS:COMPLETED` on a recurring master would end the series
for every client. The roll-forward is not implemented yet.

An event's task attachment is stored on the `VEVENT` as
`X-TASKROOT-TASK-ID: <task UID>` (app-only). The link lives on the event because one
task can be scheduled as many blocks; `AppEvent.task_id` reads it back and the property
is written only when the field is set or cleared. Nothing in tasks.org or CalDAV
standardises this, so other clients simply ignore it rather than rewriting it. Dragging a
task onto the grid creates a `VEVENT` with the task's title and this property set; the
block length comes from the task's estimate, then the "Default Duration" setting.

Checklist items are not mapped and have been removed from the UI. Real subtasks are the
task tree (`parent_task` ↔ `RELATED-TO;RELTYPE=PARENT`).

## Testing

- `src-tauri/src/caldav/fixtures/` holds real Radicale XML responses (PROPFIND, multiget,
  error bodies). Parsers are tested against those, not hand-written XML.
- `src-tauri/src/ics/fixtures/` holds real resources: a Mozilla event with a historical
  `VTIMEZONE`/`EXDATE`/`RRULE`/`X-MOZ-GENERATION`, an all-day event, a tasks.org VTODO with
  `X-APPLE-SORT-ORDER`, and a tasks.org VTODO carrying `VTIMEZONE`, `DTSTART`, `GEO`,
  `RRULE`, `CATEGORIES` and a `VALARM`.
- The round-trip tests edit exactly one property and assert the output differs only in
  that property (I1), that every component in a multi-component resource survives (I2), and
  that unknown properties/components survive the line editor (I3). They also compare every
  line inside a nested component (`VALARM`, `VTIMEZONE` sub-components) before and after a
  write, because a property-name filter alone hides those.
- One `#[ignore]`d integration test runs discovery against a real Radicale.
