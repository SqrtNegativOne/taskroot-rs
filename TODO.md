# TODO

Backlog for the inspector, iCalendar and stopwatch work. The inspector currently
exposes only a subset of RFC 5545; this file lists the gap. It is a backlog, not
a commitment to every field.

## VTODO (task inspector) - missing standard properties

Currently: SUMMARY, DESCRIPTION, STATUS, PRIORITY, DTSTART, DUE, CATEGORIES,
RELATED-TO;RELTYPE=PARENT, RELATED-TO;RELTYPE=DEPENDS-ON, ESTIMATED-DURATION,
RRULE, VALARM, COMPLETED, CREATED, LAST-MODIFIED.

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
