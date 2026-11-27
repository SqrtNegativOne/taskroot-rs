# CalDAV

Taskroot saves one or more CalDAV profiles with **one active account** whose
server is the single source of truth; it never reads Radicale's storage folder.
For VEVENT/VTODO/VALARM field mappings and ICS test fixtures, see
[iCalendar mapping](icalendar-mapping.md).

## Deployment

- Local Radicale listens on `http://127.0.0.1:5232` with HTTP Basic auth; phone
  sync uses `tailscale serve` + DAVx5 (tasks.org for VTODO, a calendar app for
  VEVENT).
- Plain `http` is allowed only for loopback (`127.0.0.1`, `localhost`, `::1`)
  and `*.ts.net`; all other hosts require `https`.
- Profile storage, OS keychain service (`taskroot.caldav.profiles`), data
  directories and env overrides are documented in
  [storage and data files](architecture.md#storage-and-data-files).
- For testing against a real server, use the disposable fixture in
  [dummy accounts](dummy-accounts.md).

## Accounts and switching

`AccountService` serializes account transitions against active commands via a
read/write gate: every command holds a `ClientLease` pinning the active profile
so a switch cannot commit mid-operation.

- `connect`, `activate` and reconnect fetch the target account completely before
  persisting `account.json` and swapping the store. Any failure leaves the
  previous active account, data and credentials intact.
- `disconnect` keeps saved profiles; `remove` deletes the profile's keychain
  password and clears the store only if it was active.
- Switching accounts clears stopwatch focus and emits `account-changed`; the
  frontend re-keys non-Settings routes through `AccountBoundary` so drafts are
  discarded. Settings and UI state are global.

## Lists

Lists are CalDAV collections:

- `create_calendar` sends `MKCALENDAR` to `<home>/<uuid>/` and re-reads the home.
- `rename_calendar` and `set_calendar_color` `PROPPATCH` `displayname` and
  `ical:calendar-color` (clearing colour removes the property).
- `delete_calendar` `DELETE`s the collection and drops its resources from the
  store.

Every collection mutation updates `store.calendars`, bumps `store.revision()`
and persists `account.json`. Read-only collections reject edits with
`invalid-input` before any network request.

## Data flow

1. On startup and Refresh, Taskroot re-reads the collection list with one
   `PROPFIND` of the calendar home (3 s timeout), persisting `account.json` on
   change. If discovery fails and cached collections exist, it falls back to
   them; if no cached collections exist, the failure is propagated so startup
   retries continue.
2. One `calendar-query` `REPORT` per collection fetches every resource's `ETag`
   and body outside the store lock, then applies the snapshot under a
   `Store::revision()` guard. Transport/XML failures keep the previous store;
   individual malformed ICS resources are logged and skipped.
3. Edits merge into the in-memory raw resource and `PUT` immediately (no offline
   queue, no background sync). Source and target collections must be writable
   for writes, moves and deletes.
4. `PUT` updates send `If-Match: <etag>`; creates send `If-None-Match: *`.
   Ordinary same-collection updates re-fetch, re-parse, re-validate guards and
   retry a `412` once; moves and scoped deletes currently propagate conflicts.
5. Whole-resource deletion uses `DELETE` (`404`/`410` count as success). HTTP
   errors map `401 → auth`, `412 → conflict`, `410 → gone`, other non-2xx →
   `remote`.

## The three invariants

**I1 — `PUT` replaces the entire resource.** Never serialize an
`AppEvent`/`AppTask` DTO into an existing resource. Start from the fetched raw
body and update only owned properties; everything else (`VTIMEZONE`, `ATTENDEE`,
unknown `X-*`, component order, unchanged/unmodelled `VALARM`s) stays
byte-identical.

**I2 — A resource is a set of components.** A recurring master and its
exceptions share a UID in one resource. Editing or deleting one component never
drops sibling components.

**I3 — Single byte-faithful CST for reads and writes.** Taskroot parses and
edits resources through `ical-rs`'s `IcalCst` concrete syntax tree, mutating
only owned properties in place so nested components (`VALARM`, `VTIMEZONE`),
unknown properties, parameters, original folds and LF/CRLF line endings survive
verbatim, while edited or newly created lines fold at 75 octets. Brand-new
resources are built on the same `IcalCst` model.

`DTSTAMP` is generated on new events, inserted exceptions and task writes, and
preserved on ordinary event edits.

## Timezones and scoped recurring edits

Store wall clock + `TZID`; expand timed recurrences in the event's IANA zone so
wall-clock times stay fixed across DST, converting to UTC only for the render
DTO. Existing `VTIMEZONE` blocks survive verbatim; new recurring events omit
`VTIMEZONE` (clients resolve `TZID` locally). All-day events stay floating.

Occurrence identity is part of the write contract (`occurrenceStart` uses the
same canonical shape as `exdates`):

- `this` — delete removes a selected exception or appends `EXDATE` for
  `occurrenceStart`; move writes a `RECURRENCE-ID` exception at the original
  slot with the new times.
- `following` — delete truncates `RRULE` with `UNTIL` one second (or one day for
  all-day) before `occurrenceStart` and drops later exceptions. Moves are
  rejected (`invalid-input`).
- `all` — delete removes every `VEVENT` sharing the UID. Move shifts the master
  (see [known limitations](known-limitations.md) for `all` from an exception id).
