use super::*;

#[test]
fn all_day_events_are_kept_only_when_they_overlap_the_range() {
    let inside = event("evt-1", "2026-09-02", "2026-09-03");
    let outside = event("evt-2", "2026-09-10", "2026-09-11");

    let instances = expand_event_instances(
        &[inside, outside],
        "2026-09-01T00:00:00Z",
        "2026-09-07T23:59:59Z",
    );

    assert_eq!(instances.len(), 1);
    assert_eq!(all_day_start(&instances[0]), "2026-09-02");
}

#[test]
fn a_timed_occurrence_reports_its_wall_clock_exclusion_value() {
    let instances =
        expand_event_instances(&[weekly_kolkata_master()], KOLKATA_RANGE.0, KOLKATA_RANGE.1);

    assert_eq!(
        occurrence_starts(&instances),
        vec!["2026-10-19T11:00:00", "2026-10-26T11:00:00"],
        "the exclusion value is the wall clock in the event's zone, not the UTC instant"
    );
}

#[test]
fn excluding_a_reported_occurrence_start_removes_exactly_that_occurrence() {
    let mut master = weekly_kolkata_master();
    let reported = expand_event_instances(&[master.clone()], KOLKATA_RANGE.0, KOLKATA_RANGE.1);
    master.exdates = Some(vec![reported[1].occurrence_start]);

    let remaining = expand_event_instances(&[master], KOLKATA_RANGE.0, KOLKATA_RANGE.1);

    assert_eq!(
        occurrence_starts(&remaining),
        vec!["2026-10-19T11:00:00"],
        "deleting the second occurrence must leave the first one visible"
    );
}

#[test]
fn an_all_day_occurrence_reports_its_date_exclusion_value() {
    let mut master = event("evt-1", "2026-09-07", "2026-09-08");
    master.rrule = Some("RRULE:FREQ=WEEKLY;BYDAY=MO".into());
    master.is_all_day = Some(true);

    let instances = expand_event_instances(&[master], "2026-09-07", "2026-10-04");

    assert_eq!(
        occurrence_starts(&instances),
        vec!["2026-09-07", "2026-09-14", "2026-09-21", "2026-09-28"]
    );
}
