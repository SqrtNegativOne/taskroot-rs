use super::*;

#[test]
fn daily_recurrence_far_from_dtstart_stays_visible() {
    let mut master = event("evt-1", "2025-01-01T09:00:00Z", "2025-01-01T09:30:00Z");
    master.rrule = Some("RRULE:FREQ=DAILY".into());

    let instances =
        expand_event_instances(&[master], "2026-09-01T00:00:00Z", "2026-09-03T23:59:59Z");

    assert_eq!(instances.len(), 3);
    assert!(instances.iter().all(|i| i.recurring));
    assert!(instances.iter().all(|i| i.id == EventId::from("evt-1")));
}

#[test]
fn weekly_all_day_recurrence_renders_every_week() {
    let mut master = event("evt-1", "2026-09-07", "2026-09-08");
    master.rrule = Some("RRULE:FREQ=WEEKLY;BYDAY=MO".into());
    master.is_all_day = Some(true);

    let instances = expand_event_instances(&[master], "2026-09-07", "2026-10-04");

    let starts: Vec<String> = instances.iter().map(all_day_start).collect();
    assert_eq!(
        starts,
        vec!["2026-09-07", "2026-09-14", "2026-09-21", "2026-09-28"]
    );
}

#[test]
fn weekly_all_day_recurrence_keeps_monday_in_negative_offset_timezone() {
    let mut master = event("evt-1", "2026-09-07", "2026-09-08");
    master.rrule = Some("RRULE:FREQ=WEEKLY;BYDAY=MO".into());
    master.is_all_day = Some(true);
    master.timezone = Some("America/Los_Angeles".into());

    let instances = expand_event_instances(&[master], "2026-09-07", "2026-10-04");

    assert_eq!(instances.len(), 4);
    assert!(instances.iter().all(|i| {
        NaiveDate::parse_from_str(&all_day_start(i), "%Y-%m-%d")
            .unwrap()
            .weekday()
            == chrono::Weekday::Mon
    }));
}

#[test]
fn cancelled_all_day_occurrence_is_suppressed() {
    let mut master = event("evt-1", "2026-09-07", "2026-09-08");
    master.rrule = Some("RRULE:FREQ=WEEKLY;BYDAY=MO".into());
    master.is_all_day = Some(true);

    let mut cancelled = event("evt-2", "2026-09-14", "2026-09-15");
    cancelled.is_all_day = Some(true);
    cancelled.recurring_event_id = Some(EventId::from("evt-1"));
    cancelled.original_start_time = Some("2026-09-14".into());
    cancelled.status = Some(EventStatus::Cancelled);

    let instances = expand_event_instances(&[master, cancelled], "2026-09-07", "2026-10-04");

    let starts: Vec<String> = instances.iter().map(all_day_start).collect();
    assert_eq!(starts, vec!["2026-09-07", "2026-09-21", "2026-09-28"]);
}

#[test]
fn an_exdate_suppresses_an_all_day_occurrence() {
    let mut master = event("evt-1", "2026-09-07", "2026-09-08");
    master.rrule = Some("RRULE:FREQ=WEEKLY;BYDAY=MO".into());
    master.is_all_day = Some(true);
    master.exdates = Some(vec!["2026-09-14".into()]);

    let instances = expand_event_instances(&[master], "2026-09-07", "2026-10-04");

    let starts: Vec<String> = instances.iter().map(all_day_start).collect();
    assert_eq!(starts, vec!["2026-09-07", "2026-09-21", "2026-09-28"]);
}

#[test]
fn an_exdate_suppresses_a_timed_occurrence() {
    let mut master = event("evt-1", "2026-09-01T09:00:00Z", "2026-09-01T09:30:00Z");
    master.rrule = Some("RRULE:FREQ=DAILY;COUNT=3".into());
    master.exdates = Some(vec!["2026-09-02T09:00:00".into()]);

    let instances =
        expand_event_instances(&[master], "2026-09-01T00:00:00Z", "2026-09-03T23:59:59Z");

    let mut starts: Vec<String> = instances.iter().map(timed_start).collect();
    starts.sort();
    assert_eq!(
        starts,
        vec!["2026-09-01T09:00:00+00:00", "2026-09-03T09:00:00+00:00"]
    );
}

#[test]
fn timed_recurrence_expands_in_the_event_zone_across_dst() {
    // Europe/London 09:00 weekly; BST starts 2026-03-29, so the UTC instants
    // move by an hour while the local wall clock stays at 09:00.
    let mut master = event("evt-1", "2026-03-16T09:00:00", "2026-03-16T09:30:00");
    master.rrule = Some("RRULE:FREQ=WEEKLY;BYDAY=MO".into());
    master.timezone = Some("Europe/London".into());

    let instances =
        expand_event_instances(&[master], "2026-03-16T00:00:00Z", "2026-04-06T23:59:59Z");

    let starts: Vec<String> = instances.iter().map(timed_start).collect();
    assert_eq!(
        starts,
        vec![
            "2026-03-16T09:00:00+00:00",
            "2026-03-23T09:00:00+00:00",
            "2026-03-30T08:00:00+00:00",
            "2026-04-06T08:00:00+00:00",
        ]
    );
}

#[test]
fn a_wall_clock_exception_moves_its_slot_in_the_event_zone() {
    let mut master = event("evt-1", "2026-10-19T11:00:00", "2026-10-19T12:00:00");
    master.rrule = Some("RRULE:FREQ=WEEKLY".into());
    master.timezone = Some("Asia/Kolkata".into());

    let mut moved = event("evt-2", "2026-10-28T11:00:00", "2026-10-28T12:00:00");
    moved.timezone = Some("Asia/Kolkata".into());
    moved.recurring_event_id = Some(EventId::from("evt-1"));
    moved.original_start_time = Some("2026-10-26T11:00:00".into());

    let instances = expand_event_instances(
        &[master, moved],
        "2026-10-19T00:00:00Z",
        "2026-11-02T23:59:59Z",
    );

    let mut starts: Vec<String> = instances.iter().map(timed_start).collect();
    starts.sort();
    assert_eq!(
        starts,
        vec![
            "2026-10-19T05:30:00+00:00",
            "2026-10-28T05:30:00+00:00",
            "2026-11-02T05:30:00+00:00",
        ],
        "the 2026-10-26 slot is replaced by the moved exception"
    );
}

#[test]
fn moved_occurrence_renders_once_at_its_new_time() {
    let mut master = event("evt-1", "2026-09-01T09:00:00Z", "2026-09-01T09:30:00Z");
    master.rrule = Some("RRULE:FREQ=DAILY;COUNT=3".into());

    let mut moved = event("evt-2", "2026-09-02T14:00:00Z", "2026-09-02T14:30:00Z");
    moved.recurring_event_id = Some(EventId::from("evt-1"));
    moved.original_start_time = Some("2026-09-02T09:00:00Z".into());

    let instances = expand_event_instances(
        &[master, moved],
        "2026-09-01T00:00:00Z",
        "2026-09-03T23:59:59Z",
    );

    let mut starts: Vec<String> = instances.iter().map(timed_start).collect();
    starts.sort();
    assert_eq!(
        starts,
        vec![
            "2026-09-01T09:00:00+00:00",
            "2026-09-02T14:00:00+00:00",
            "2026-09-03T09:00:00+00:00",
        ],
        "the moved occurrence replaces the original slot exactly once"
    );
}

#[test]
fn a_moved_occurrence_reports_the_master_rule() {
    let mut master = event("evt-1", "2026-09-01T09:00:00Z", "2026-09-01T09:30:00Z");
    master.rrule = Some("RRULE:FREQ=DAILY;COUNT=3".into());

    let mut moved = event("evt-2", "2026-09-02T14:00:00Z", "2026-09-02T14:30:00Z");
    moved.recurring_event_id = Some(EventId::from("evt-1"));
    moved.original_start_time = Some("2026-09-02T09:00:00Z".into());

    let instances = expand_event_instances(
        &[master, moved],
        "2026-09-01T00:00:00Z",
        "2026-09-03T23:59:59Z",
    );

    let moved_instance = instances
        .iter()
        .find(|instance| instance.id == EventId::from("evt-2"))
        .unwrap();
    assert_eq!(
        moved_instance.rrule.as_deref(),
        Some("RRULE:FREQ=DAILY;COUNT=3"),
        "an exception inherits the series rule so presentation can classify it"
    );
}

#[test]
fn cancelled_standalone_event_is_not_rendered() {
    let mut cancelled = event("evt-1", "2026-09-02T09:00:00Z", "2026-09-02T09:30:00Z");
    cancelled.status = Some(EventStatus::Cancelled);

    let instances =
        expand_event_instances(&[cancelled], "2026-09-01T00:00:00Z", "2026-09-03T23:59:59Z");

    assert!(
        instances.is_empty(),
        "expected no instances, got {instances:?}"
    );
}

#[test]
fn hourly_recurrence_produces_distinct_occurrence_keys() {
    let mut master = event("evt-1", "2026-09-02T08:00:00Z", "2026-09-02T08:30:00Z");
    master.rrule = Some("RRULE:FREQ=HOURLY;COUNT=4".into());

    let instances =
        expand_event_instances(&[master], "2026-09-02T00:00:00Z", "2026-09-02T23:59:59Z");

    assert_eq!(instances.len(), 4);
    let keys: HashSet<&str> = instances
        .iter()
        .map(|i| i.occurrence_key.as_str())
        .collect();
    assert_eq!(keys.len(), 4, "sub-daily instances need unique keys");
}
