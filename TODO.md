# TODO

Backlog for the inspector and iCalendar work. The inspector currently exposes only
a subset of RFC 5545; this file lists the gap. It is a backlog, not a commitment
to every field.

## Priority: replace non-standard `X-TASKROOT-*` properties

Rely on a standard iCalendar property wherever one exists. Remove the custom
fallbacks instead of maintaining two representations.

- [ ] `X-TASKROOT-STATUS` -> standard `STATUS`
  - `todo` -> `NEEDS-ACTION`, `doing` -> `IN-PROCESS`, `done` -> `COMPLETED`;
    add `CANCELLED`.
  - `next-up` has no standard equivalent. Decide whether to drop it or keep one
    documented custom value.
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
- [ ] STATUS `IN-PROCESS` / `CANCELLED`
- [ ] `RELATED-TO` reltypes other than PARENT
- [ ] RFC 7986 COLOR / IMAGE

## VEVENT (event inspector) - missing standard properties

Currently: SUMMARY, DESCRIPTION, STATUS, DTSTART, DTEND, RRULE, COLOR, CREATED,
LAST-MODIFIED.

- [ ] LOCATION
- [ ] GEO
- [ ] CLASS
- [ ] TRANSP
- [ ] PRIORITY
- [ ] URL
- [ ] ORGANIZER / ATTENDEE
- [ ] CONTACT
- [ ] RESOURCES
- [ ] COMMENT
- [ ] ATTACH
- [ ] CATEGORIES
- [ ] RDATE
- [ ] DURATION
- [ ] REQUEST-STATUS
- [ ] SEQUENCE (display)
- [ ] RFC 7986 IMAGE / CONFERENCE

## Notes

- Properties the writer does not own survive verbatim (`LOCATION`, `ATTENDEE`,
  `SEQUENCE`, unknown `X-*`), so the data is preserved but invisible and
  uneditable until it is read into the DTO.
- Fields such as `ATTENDEE`, `ORGANIZER` and `ATTACH` need an editor UX, not just
  a text row.
