# Google Calendar write semantics & recurring exceptions

Findings behind the `PUT` → `PATCH` change and the remaining exception-instance
follow-up. Code lives in `src-tauri/src/apis/google_calendar/`.

## Why updates now PATCH

Google's Calendar API v3 distinguishes the two write verbs:

- [`events.update`](https://developers.google.com/calendar/api/v3/reference/events/update)
  (`PUT`) — "This method does not support patch semantics and always updates the
  entire event resource." Every omitted writable field is cleared.
- [`events.patch`](https://developers.google.com/calendar/api/v3/reference/events/patch)
  (`PATCH`) — "This method supports patch semantics." Only the fields in the body
  change.

`publish()` used `PUT` for any row with a `remote_id`, and its body only carried
`summary`, `description`, `start`, `end`, and `status`. That meant every edit
silently wiped `recurrence`, `timeZone`, `colorId`, `attendees`, `reminders`,
`location`, `transparency`, `visibility`, and `conferenceData`. Updates now use
`PATCH` (creates stay `POST`), and the fields the app models are sent explicitly.
Unmodeled fields are preserved automatically.

`color` is deliberately **read-only**: the app stores the resolved background
hex from `/colors` (with a calendar fallback), while Google writes `colorId`
values (`"1"`..`"11"`). The palette is server-owned and not injective, so there is
no stable hex → `colorId` reverse map to send.

## How the app represents recurrence today

Two overlapping mechanisms exist, and only one of them is wired end to end:

1. **Master `rrule` string.** `build_app_event` joins Google's `recurrence`
   array (`RRULE` / `EXDATE` / `RDATE` lines) with `\n` into `AppEvent.rrule`.
   `expand.rs` feeds that string to the rrule engine. The write path now sends
   it back verbatim (split on `\n`). This is the sound path and round-trips.
2. **`AppEvent.exdates` column.** `InspectorPane.svelte` (`applyRecurringInstance`)
   deletes a single occurrence by appending to the master's `exdates` and calling
   `updateEvent`. But `build_app_event` always sets `exdates: None`, `expand.rs`
   never reads it, and the write path does not serialize it. So this is a
   half-finished path: the edit is local-only and invisible to both the renderer
   and Google. `applyRecurringFollowing` rewrites `rrule` and *does* work now.

Master `rrule` remains the source of truth for occurrence suppression.

## What `expand.rs` expects

`expand_event_instances` suppresses a master occurrence when a separate row with
`recurring_event_id == master` and `original_start_time == <occurrence slot>`
exists — that row is an **exception instance**:

- A moved occurrence is a normal row rendered at its new time via `push_override`,
  keyed `master@<originalStartTime>`.
- A cancelled occurrence is kept as a row with `status = Cancelled`; it renders
  nothing and `collect_exception_keys` suppresses the master slot.

Exception rows are keyed on the *original* slot, not the new time, which is why
`original_start_time` is immutable.

## What Google expects on write for an exception

Google models a modified occurrence as its own event resource that references the
series:

- **Create a new exception**: `events.insert` with `recurringEventId` and
  `originalStartTime` set. Both are immutable. Implemented in
  `write::insert_exception_fields` (only sent on create).
- **Update an existing exception**: `events.patch` the exception's **own** id.
  `recurringEventId` / `originalStartTime` are preserved by PATCH, so they are
  not resent. Implemented.
- **Cancel one occurrence**: PATCH the exception with `status: "cancelled"`, or
  add `EXDATE` to the master. The app currently uses the exception-row route on
  read (Google pushes it) but never creates one on write.

## Follow-up scope

Not fixed here, tracked by the comments in `apis/google_calendar/events.rs` and
`write.rs`:

1. **Per-occurrence deletion** currently writes `AppEvent.exdates`, which is not
   serialized. Fix by merging `exdates` into the `recurrence` array at write time
   (date-only for all-day, UTC iCalendar for timed) or by replacing the flow with
   a real exception row.
2. **Per-occurrence drag** via `reschedule_event` moves the master unless the row
   is already an exception. Creating/updating an exception on drag is unaddressed.
3. **Optimistic concurrency**: `AppEvent.etag` is never sent as `If-Match`, so a
   local edit can clobber a concurrent remote change. `publish()` also discards
   the response `etag`; `finalize_event` would need to persist it.
