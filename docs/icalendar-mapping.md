# iCalendar mapping

VEVENT and VTODO field mappings between Taskroot DTOs and RFC 5545 resources.
For transport, accounts, conditional writes, I1–I3 invariants and scoped
recurring edits, see [CalDAV](caldav.md).

## VEVENT mapping

Events own `SUMMARY`, `DESCRIPTION`, `DTSTART`, `DTEND`, `PRIORITY`,
`CATEGORIES`, `RRULE`, `EXDATE`, `STATUS`, `COLOR` and `X-TASKROOT-TASK-ID`.

- `PRIORITY` and `CATEGORIES` follow the VTODO conventions below; missing
  priority or empty tags emit no line.
- `X-TASKROOT-TASK-ID: <task UID>` is the sole remaining `X-TASKROOT-*`
  property. It lives on the `VEVENT` because one task can be scheduled as
  multiple blocks; `AppEvent.task_id` reads it back and writes it only when set
  or cleared. Other CalDAV clients ignore it.

## VTODO mapping

| Taskroot | VTODO | Rule |
|---|---|---|
| `id` | `UID` | Generated once, never rewritten. |
| `remote_id` | resource href | |
| `title` | `SUMMARY` | |
| `notes` | `DESCRIPTION` | Plain text; a `VALARM`'s own `DESCRIPTION` is separate. |
| `start` | `DTSTART` | tasks.org "hide until"; date-only stays `YYYY-MM-DD`. |
| `due` | `DUE` | Preserves raw value; date-only stays `YYYY-MM-DD`. |
| `timezone` | `TZID` of `DUE`, else `DTSTART` | `DUE` wins when zones differ; `…Z` reads as `UTC` so its instant survives writes. |
| `status` | `STATUS` + `COMPLETED` + `PERCENT-COMPLETE` | `Todo`/missing → `NEEDS-ACTION`, `NextUp` → `IN-PROCESS`, `Done` → `COMPLETED` (with `PERCENT-COMPLETE:100`), `Cancelled` → `CANCELLED`. Live stopwatch focus is app-local, never written as status. |
| `completed_at` | `COMPLETED` | Preserved across edits so retitling a done task keeps its completion time. |
| `priority` | `PRIORITY` | `None` omitted, `Low→7`, `Medium→5`, `High→3`, `Urgent→1`; reverse `1–2→Urgent`, `3–4→High`, `5→Medium`, `6–9→Low`, `0→None`. |
| `tags` | `CATEGORIES` | Tag names only; colours stay local keyed by name. |
| `rrule` | `RRULE` | Wire form, one line per rule; Taskroot writes it but does not expand task series. |
| `geo` | `GEO` | `latitude;longitude` (tasks.org geofence point). |
| `alarms` | `VALARM` `TRIGGER` | `relative` = `TRIGGER;RELATED=START\|END:-PTnM` (positive `minutes_before` = before anchor); `absolute` = `TRIGGER;VALUE=DATE-TIME:…Z`. Existing `ACTION`/`DESCRIPTION` stay intact; new alarms emit tasks.org's `DISPLAY` block. |
| `parent_task` | `RELATED-TO;RELTYPE=PARENT` | Subtask tree relation (checklist items are not mapped). |
| `dependencies` | `RELATED-TO;RELTYPE=DEPENDS-ON` | One line per dependency UID. |
| `added` | `CREATED`, fallback `DTSTAMP` | tasks.org often omits `CREATED`. |
| `updated_at` | `LAST-MODIFIED` | |
| `est` | `ESTIMATED-DURATION` | RFC 9074 duration; whole minutes write as `PT{n}M`. |

### Preservation and guards

- Unowned properties (`SEQUENCE`, `LOCATION`, `URL`, `CLASS`, `CONTACT`,
  unknown `X-*`) pass through untouched. Legacy `X-TASKROOT-*` UI/status
  properties are stripped on write; per-component UI state lives in
  `ui-state.json`.
- `VALARM` blocks are skipped when `alarms` did not change. When `alarms`
  changes, only modelled alarms are matched/rewritten/removed; unmodelled
  triggers (such as `-PT45S`) stay byte-identical.
- Completing a task with `RRULE` is refused (`invalid-input`), including after a
  `412` conflict refetch: tasks.org rolls recurring tasks forward, and writing
  plain `STATUS:COMPLETED` on a recurring master would terminate the series for
  every client.

## Fixtures and round-trip tests

- `src-tauri/src/features/calendar/caldav/fixtures/` — real Radicale XML
  responses (PROPFIND, multiget, error bodies) used by parser tests.
- `src-tauri/src/features/calendar/ics/fixtures/` — real Mozilla and tasks.org
  resources carrying `VTIMEZONE`, `EXDATE`, `RRULE`, `GEO`, `VALARM` and vendor
  `X-*` properties.
- Round-trip tests edit one property and assert only owned lines changed (I1),
  every sibling component survived (I2), and unknown properties plus nested
  components (`VALARM`, `VTIMEZONE`) stayed intact (I3).
