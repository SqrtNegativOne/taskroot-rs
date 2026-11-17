# TODO

Backlog for the inspector, iCalendar and stopwatch work. The inspector currently
exposes only a subset of RFC 5545; this file lists the gap. It is a backlog, not
a commitment to every field.

## Priority: replace non-standard `X-TASKROOT-*` properties

Rely on a standard iCalendar property wherever one exists. Remove the custom
fallbacks instead of maintaining two representations.

- [ ] `X-TASKROOT-STATUS` -> standard `STATUS`
  - `todo` -> `NEEDS-ACTION`, `doing` -> `IN-PROCESS`, `done` -> `COMPLETED`,
    `cancelled` -> `CANCELLED` are now read and written.
  - `next-up` has no standard equivalent and is the only custom value still
    written. Create new prop such as X-NEXT-UP:TRUE for example
- [ ] `X-TASKROOT-DEPENDENCIES` -> one `RELATED-TO;RELTYPE=DEPENDS-ON` line per
      dependency.
- [ ] `X-TASKROOT-EST` -> `ESTIMATED-DURATION` (RFC 9074).
- [ ] `X-TASKROOT-TASK-ID` (event -> task link) has no standard equivalent.
      Document it as the one exception, or move the link into app UI state.
- [ ] `X-TASKROOT-CANVAS-X`, `X-TASKROOT-CANVAS-Y`, `X-TASKROOT-ON-CANVAS` and
      `X-TASKROOT-TABS` are app UI state, not calendar data. Move them to
      `ui-state.json` so they are not written into `.ics`.
- [ ] Confirm the writer removes the owned `X-*` lines cleanly. Taskroot has no
      migration layer, so old values simply stop being read.

## VTODO (task inspector) - missing standard properties

Currently: SUMMARY, DESCRIPTION, STATUS, PRIORITY, DTSTART, DUE, CATEGORIES,
RELATED-TO;RELTYPE=PARENT, RRULE, VALARM, COMPLETED, CREATED, LAST-MODIFIED.

- [ ] LOCATION
- [ ] GEO (already modelled and read/written; inspector row only)
- [ ] CLASS
- [ ] URL
- [ ] ORGANIZER / ATTENDEE
- [ ] CONTACT
- [ ] RESOURCES
- [ ] COMMENT
- [ ] ATTACH
- [ ] RDATE
- [ ] DURATION
- [ ] REQUEST-STATUS
- [ ] SEQUENCE (display)
- [ ] PERCENT-COMPLETE (partial progress, not just 0/100)
- [ ] `RELATED-TO` reltypes other than PARENT
- [ ] RFC 7986 COLOR / IMAGE

## VEVENT (event inspector) - missing standard properties

Currently: SUMMARY, DESCRIPTION, STATUS, PRIORITY, CATEGORIES, DTSTART, DTEND,
RRULE, COLOR, CREATED, LAST-MODIFIED.

- [ ] LOCATION
- [ ] GEO
- [ ] CLASS
- [ ] TRANSP
- [ ] URL
- [ ] ORGANIZER / ATTENDEE
- [ ] CONTACT
- [ ] RESOURCES
- [ ] COMMENT
- [ ] ATTACH
- [ ] RDATE
- [ ] DURATION
- [ ] REQUEST-STATUS
- [ ] SEQUENCE (display)
- [ ] RFC 7986 IMAGE / CONFERENCE

## Stopwatch

- [ ] History screen for `stopwatch-history.json`: group completed work and
      break segments by day and label, show totals, and let the user inspect the
      raw segments. The backend already appends every finished segment.

## Notes

- Properties the writer does not own survive verbatim (`LOCATION`, `ATTENDEE`,
  `SEQUENCE`, unknown `X-*`), so the data is preserved but invisible and
  uneditable until it is read into the DTO.
- Fields such as `ATTENDEE`, `ORGANIZER` and `ATTACH` need an editor UX, not just
  a text row.
