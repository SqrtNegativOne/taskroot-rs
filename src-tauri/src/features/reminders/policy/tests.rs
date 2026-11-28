use super::*;

fn task_with_alarms(
    id: &str,
    due: &str,
    alarms: Vec<TaskAlarmTrigger>,
    timezone: Option<&str>,
) -> AppTask {
    AppTask {
        id: id.into(),
        title: "Brush".to_string(),
        status: Some(AppTaskStatus::Todo),
        priority: None,
        tags: None,
        parent_task: None,
        dependencies: None,
        est: None,
        time_spent: None,
        added: None,
        remote_id: None,
        remote_collection_id: None,
        notes: None,
        start: None,
        due: Some(due.parse().unwrap()),
        completed_at: None,
        rrule: None,
        geo: None,
        alarms: Some(alarms),
        timezone: timezone.map(ToString::to_string),
        updated_at: None,
        etag: None,
    }
}

fn task(due: &str, alarm: TaskAlarmTrigger, timezone: Option<&str>) -> AppTask {
    task_with_alarms("task-1", due, vec![alarm], timezone)
}

fn utc(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

fn before(minutes_before: i32) -> TaskAlarmTrigger {
    TaskAlarmTrigger::Relative {
        minutes_before,
        anchor: TaskAlarmAnchor::End,
    }
}

#[test]
fn a_relative_reminder_fires_inside_its_window_only() {
    let tasks = [task("2026-10-27T10:00:00", before(15), Some("UTC"))];

    assert_eq!(alarms_due(&tasks, utc("2026-10-27T09:44:00Z")).len(), 0, "not yet due");
    let due = alarms_due(&tasks, utc("2026-10-27T09:46:00Z"));
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].trigger, utc("2026-10-27T09:45:00Z"));
    assert_eq!(due[0].title, "Brush");
    assert_eq!(alarms_due(&tasks, utc("2026-10-27T10:00:00Z")).len(), 0, "too late to replay");
}

#[test]
fn a_completed_or_cancelled_task_never_reminds() {
    for status in [AppTaskStatus::Done, AppTaskStatus::Cancelled] {
        let mut item = task("2026-10-27T10:00:00", before(15), Some("UTC"));
        item.status = Some(status);

        let due = alarms_due(&[item], utc("2026-10-27T09:46:00Z"));
        assert!(due.is_empty(), "expected no due alarms, got {due:?}");
    }
}

#[test]
fn a_date_only_due_date_reminds_at_nine_in_the_task_zone() {
    let tasks = [task("2026-10-27", before(30), Some("Asia/Kolkata"))];

    // 08:30 IST == 03:00 UTC.
    let due = alarms_due(&tasks, utc("2026-10-27T03:01:00Z"));
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].trigger, utc("2026-10-27T03:00:00Z"));
}

#[test]
fn a_start_anchor_prefers_the_start_and_falls_back_to_due() {
    let mut item = task("2026-10-27T10:00:00", before(10), Some("UTC"));
    item.start = Some("2026-10-27T08:00:00".parse().unwrap());
    item.alarms = Some(vec![TaskAlarmTrigger::Relative {
        minutes_before: 10,
        anchor: TaskAlarmAnchor::Start,
    }]);

    let due = alarms_due(&[item], utc("2026-10-27T07:51:00Z"));
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].trigger, utc("2026-10-27T07:50:00Z"));
}

#[test]
fn an_absolute_trigger_uses_its_own_instant() {
    let alarm = TaskAlarmTrigger::Absolute {
        at: "2026-10-27T09:00:00Z".parse().unwrap(),
    };
    let tasks = [task("2026-10-27T10:00:00", alarm, Some("UTC"))];

    let due = alarms_due(&tasks, utc("2026-10-27T09:01:00Z"));
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].trigger, utc("2026-10-27T09:00:00Z"));
}

#[test]
fn delivered_reminder_does_not_fire_twice_within_missed_window() {
    let mut sent = SentReminders::default();
    let tasks = [task("2026-10-27T10:00:00", before(15), Some("UTC"))];

    let first = sent.take_due("acct-1", &tasks, utc("2026-10-27T09:46:00Z"));
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].trigger, utc("2026-10-27T09:45:00Z"));

    let second = sent.take_due("acct-1", &tasks, utc("2026-10-27T09:47:00Z"));
    assert!(second.is_empty(), "already delivered inside window");
}

#[test]
fn reordering_alarms_neither_duplicates_nor_skips_notifications() {
    let mut sent = SentReminders::default();
    let first_alarm = before(15); // 09:45:00Z
    let second_alarm = before(14); // 09:46:00Z

    let initial = [task_with_alarms(
        "task-1",
        "2026-10-27T10:00:00",
        vec![first_alarm.clone(), second_alarm.clone()],
        Some("UTC"),
    )];
    let tick1 = sent.take_due("acct-1", &initial, utc("2026-10-27T09:45:30Z"));
    assert_eq!(tick1.len(), 1);
    assert_eq!(tick1[0].trigger, utc("2026-10-27T09:45:00Z"));

    // Swap alarm order before the second alarm becomes due.
    let reordered = [task_with_alarms(
        "task-1",
        "2026-10-27T10:00:00",
        vec![second_alarm, first_alarm],
        Some("UTC"),
    )];
    let tick2 = sent.take_due("acct-1", &reordered, utc("2026-10-27T09:46:30Z"));
    assert_eq!(tick2.len(), 1, "only the newly due alarm should fire");
    assert_eq!(tick2[0].trigger, utc("2026-10-27T09:46:00Z"));

    // Swap back; neither alarm should re-fire.
    let tick3 = sent.take_due("acct-1", &initial, utc("2026-10-27T09:47:00Z"));
    assert!(tick3.is_empty(), "reordering already-sent alarms must not duplicate");
}

#[test]
fn removing_earlier_alarm_does_not_duplicate_or_skip_remaining_alarm() {
    let both = [task_with_alarms(
        "task-1",
        "2026-10-27T10:00:00",
        vec![before(15), before(14)],
        Some("UTC"),
    )];
    let only_second = [task_with_alarms(
        "task-1",
        "2026-10-27T10:00:00",
        vec![before(14)],
        Some("UTC"),
    )];

    // Both fired, then earlier alarm is removed -> remaining alarm must not re-fire.
    let mut sent = SentReminders::default();
    assert_eq!(sent.take_due("acct-1", &both, utc("2026-10-27T09:46:30Z")).len(), 2);
    assert!(
        sent.take_due("acct-1", &only_second, utc("2026-10-27T09:47:00Z")).is_empty(),
        "shifted alarm must not duplicate"
    );

    // Earlier alarm fires and is removed before later alarm is due -> later still fires.
    let mut sent = SentReminders::default();
    assert_eq!(sent.take_due("acct-1", &both, utc("2026-10-27T09:45:30Z")).len(), 1);
    let tick2 = sent.take_due("acct-1", &only_second, utc("2026-10-27T09:46:30Z"));
    assert_eq!(tick2.len(), 1, "shifted alarm must still fire when due");
    assert_eq!(tick2[0].trigger, utc("2026-10-27T09:46:00Z"));
}

#[test]
fn inserting_earlier_alarm_does_not_duplicate_already_delivered_alarm() {
    let mut sent = SentReminders::default();
    let initial = [task_with_alarms("task-1", "2026-10-27T10:00:00", vec![before(14)], Some("UTC"))];
    assert_eq!(sent.take_due("acct-1", &initial, utc("2026-10-27T09:46:30Z")).len(), 1);

    let with_earlier = [task_with_alarms(
        "task-1",
        "2026-10-27T10:00:00",
        vec![before(15), before(14)],
        Some("UTC"),
    )];
    let tick2 = sent.take_due("acct-1", &with_earlier, utc("2026-10-27T09:47:00Z"));
    assert_eq!(tick2.len(), 1, "only the newly inserted earlier alarm should fire");
    assert_eq!(tick2[0].trigger, utc("2026-10-27T09:45:00Z"));
}

#[test]
fn identical_triggers_deduplicate_per_task_while_scopes_stay_isolated() {
    let mut sent = SentReminders::default();
    let tasks = [task_with_alarms(
        "task-1",
        "2026-10-27T10:00:00",
        vec![before(15), TaskAlarmTrigger::Absolute { at: utc("2026-10-27T09:45:00Z") }],
        Some("UTC"),
    )];

    assert_eq!(sent.take_due("acct-1", &tasks, utc("2026-10-27T09:46:00Z")).len(), 1);
    assert_eq!(sent.take_due("acct-2", &tasks, utc("2026-10-27T09:46:00Z")).len(), 1);
}

#[test]
fn expired_dedup_entries_are_pruned_on_each_tick() {
    let mut sent = SentReminders::default();
    let early = task_with_alarms("task-1", "2026-10-27T10:00:00", vec![before(15)], Some("UTC"));
    let later = task_with_alarms("task-2", "2026-10-27T10:00:00", vec![before(12)], Some("UTC"));
    let tasks = [early, later];

    assert_eq!(sent.take_due("acct-1", &tasks, utc("2026-10-27T09:46:00Z")).len(), 1);
    assert_eq!(sent.len(), 1);

    assert_eq!(sent.take_due("acct-1", &tasks, utc("2026-10-27T09:49:00Z")).len(), 1);
    assert_eq!(sent.len(), 2, "both triggers are still within the missed window");

    // At 09:50:00Z, 09:45:00Z reaches the 5-minute cutoff and is pruned while
    // 09:48:00Z stays tracked and must not re-fire.
    assert_eq!(sent.take_due("acct-1", &tasks, utc("2026-10-27T09:50:00Z")), Vec::new());
    assert_eq!(sent.len(), 1, "expired entry should be pruned");

    // Even an idle tick with no tasks prunes once the remaining trigger expires.
    assert_eq!(sent.take_due("acct-1", &[], utc("2026-10-27T09:53:00Z")), Vec::new());
    assert!(sent.is_empty(), "all expired entries should be pruned");
}
