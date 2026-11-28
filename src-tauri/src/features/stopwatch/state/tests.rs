//! Behavioral tests for the stopwatch state machine and manager.

use super::*;

const MINUTE: u64 = 60_000;

fn minutes(n: u64) -> u64 {
    n.saturating_mul(MINUTE)
}

fn state_running_since(started: u64) -> StopwatchState {
    StopwatchState {
        phase: StopwatchPhase::Running,
        work_running_since: Some(started),
        ..StopwatchState::default()
    }
}

#[test]
fn starts_paused_and_pristine() {
    let state = StopwatchState::default();

    assert_eq!(state.phase, StopwatchPhase::Paused);
    assert_eq!(state.work_ms(1_000), 0);
    assert_eq!(state.label_source, StopwatchLabelSource::Unset);
}

#[test]
fn work_accumulates_while_running_and_freezes_when_paused() {
    let mut state = state_running_since(1_000);
    assert_eq!(
        state.work_ms(1_000_u64.saturating_add(minutes(4))),
        minutes(4)
    );

    let entries = state
        .toggle_work(1_000_u64.saturating_add(minutes(4)))
        .unwrap();
    assert_eq!(state.phase, StopwatchPhase::Paused);
    assert_eq!(state.work_ms(9_999_999), minutes(4));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, StopwatchHistoryKind::Work);
}

#[test]
fn resuming_work_keeps_the_accumulated_total() {
    let mut state = state_running_since(0);
    state.toggle_work(minutes(2)).unwrap();
    state.toggle_work(minutes(2)).unwrap();

    assert_eq!(state.work_ms(minutes(3)), minutes(3));
}

#[test]
fn a_break_needs_five_minutes_of_work() {
    let mut state = state_running_since(0);

    let before = state.toggle_break(minutes(4), 5);
    assert!(before.is_err());

    let allowed = state.toggle_break(MIN_WORK_FOR_BREAK_MS, 5);
    assert!(allowed.is_ok());
    assert_eq!(state.phase, StopwatchPhase::Breaking);
}

#[test]
fn a_short_break_is_one_fifth_of_the_work() {
    let mut state = state_running_since(0);

    state.toggle_break(minutes(30), 5).unwrap();

    assert_eq!(state.break_duration, minutes(6));
    assert_eq!(state.break_remaining_ms(minutes(30)), minutes(6));
    assert_eq!(state.break_remaining_ms(minutes(33)), minutes(3));
}

#[test]
fn a_long_break_is_one_fourth_of_the_work() {
    let mut state = state_running_since(0);

    state.toggle_break(minutes(60), 5).unwrap();

    assert_eq!(state.break_duration, minutes(15));
}

#[test]
fn a_finished_break_waits_at_zero_then_starts_a_fresh_session() {
    let mut state = state_running_since(0);
    state.toggle_break(minutes(30), 5).unwrap();
    let break_end = minutes(30).saturating_add(minutes(6));

    let entries = state.complete_break(break_end);
    assert_eq!(state.phase, StopwatchPhase::Stopped);
    assert_eq!(state.work_ms(break_end), 0);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, StopwatchHistoryKind::Break);
    assert_eq!(entries[0].duration_ms, minutes(6));

    state.toggle_work(break_end).unwrap();
    assert_eq!(state.phase, StopwatchPhase::Running);
    assert_eq!(state.work_ms(break_end), 0);
}

#[test]
fn a_break_cannot_be_paused_and_stops_cannot_break() {
    let mut state = state_running_since(0);
    state.toggle_break(minutes(10), 5).unwrap();
    assert!(state.toggle_work(minutes(10)).is_err());

    let mut stopped = StopwatchState {
        phase: StopwatchPhase::Stopped,
        ..StopwatchState::default()
    };
    assert!(stopped.toggle_break(0, 5).is_err());
}

#[test]
fn cancelling_a_break_resumes_work_and_records_the_break_taken() {
    let mut state = state_running_since(0);
    state.toggle_break(minutes(30), 5).unwrap();

    let entries = state.toggle_break(minutes(31), 5).unwrap();

    assert_eq!(state.phase, StopwatchPhase::Running);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, StopwatchHistoryKind::Break);
    assert_eq!(entries[0].duration_ms, MINUTE);
}

#[test]
fn a_timed_pause_freezes_work_and_resumes_when_it_expires() {
    let mut state = state_running_since(0);

    let entries = state.toggle_pause(minutes(2), 10);
    assert_eq!(state.paused_until, Some(minutes(12)));
    assert_eq!(state.work_ms(minutes(5)), minutes(2));
    assert_eq!(entries.len(), 1);

    assert!(state.expire_pause(minutes(12)));
    assert_eq!(state.work_ms(minutes(13)), minutes(3));
}

#[test]
fn toggling_work_clears_an_active_timed_pause() {
    let mut state = state_running_since(0);
    state.toggle_pause(minutes(2), 10);

    state.toggle_work(minutes(3)).unwrap();

    assert_eq!(state.phase, StopwatchPhase::Running);
    assert_eq!(state.paused_until, None);
}

#[test]
fn reset_records_the_live_segment_and_keeps_the_label() {
    let mut state = state_running_since(1_000);
    state.set_label(Some("  maths  ".to_string()), StopwatchLabelSource::Custom, 1_000);

    let entries = state.reset(1_000_u64.saturating_add(minutes(2)));

    assert_eq!(state.phase, StopwatchPhase::Paused);
    assert_eq!(state.work_ms(1_000_u64.saturating_add(minutes(2))), 0);
    assert_eq!(state.label.as_deref(), Some("maths"));
    assert_eq!(state.label_source, StopwatchLabelSource::Custom);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].label, "maths");
}

#[test]
fn setting_an_empty_label_clears_its_source() {
    let mut state = StopwatchState::default();
    state.set_label(Some("deep work".to_string()), StopwatchLabelSource::Task, 0);
    state.set_label(Some("   ".to_string()), StopwatchLabelSource::Custom, 0);

    assert_eq!(state.label, None);
    assert_eq!(state.label_source, StopwatchLabelSource::Unset);
}

#[test]
fn setting_a_task_sets_the_focus_and_its_title() {
    let mut state = StopwatchState::default();

    state.set_task(Some("task-1".to_string()), Some("Learn Python".to_string()), 0);

    assert_eq!(state.task_id.as_deref(), Some("task-1"));
    assert_eq!(state.label.as_deref(), Some("Learn Python"));
    assert_eq!(state.label_source, StopwatchLabelSource::Task);
}

#[test]
fn setting_a_label_clears_the_focus_task() {
    let mut state = StopwatchState::default();
    state.set_task(Some("task-1".to_string()), Some("Learn Python".to_string()), 0);

    state.set_label(Some("writing".to_string()), StopwatchLabelSource::Custom, 0);

    assert_eq!(state.task_id, None);
    assert_eq!(state.label.as_deref(), Some("writing"));
    assert_eq!(state.label_source, StopwatchLabelSource::Custom);
}

#[test]
fn clearing_a_task_clears_the_label() {
    let mut state = StopwatchState::default();
    state.set_task(Some("task-1".to_string()), Some("Learn Python".to_string()), 0);

    state.set_task(None, None, 0);

    assert_eq!(state.task_id, None);
    assert_eq!(state.label, None);
    assert_eq!(state.label_source, StopwatchLabelSource::Unset);
}

#[test]
fn clearing_the_focused_task_stops_work_and_records_it() {
    let dir = tempfile::tempdir().unwrap();
    let manager = StopwatchManager::load(dir.path());
    manager
        .set_task(Some("task-1".to_string()), Some("Learn Python".to_string()))
        .unwrap();
    manager.toggle_work().unwrap();

    let cleared = manager.clear_task_if("task-1").unwrap().unwrap().state;

    assert_eq!(cleared.task_id, None);
    assert_eq!(cleared.label, None);
    assert_eq!(cleared.label_source, StopwatchLabelSource::Unset);
    assert_eq!(cleared.phase, StopwatchPhase::Paused);
}

#[test]
fn clearing_another_task_leaves_the_focus_alone() {
    let dir = tempfile::tempdir().unwrap();
    let manager = StopwatchManager::load(dir.path());
    manager
        .set_task(Some("task-1".to_string()), Some("Learn Python".to_string()))
        .unwrap();

    assert!(manager.clear_task_if("task-2").unwrap().is_none());

    let snapshot = manager.snapshot().unwrap();
    assert_eq!(snapshot.task_id.as_deref(), Some("task-1"));
}

#[test]
fn clearing_without_a_focused_task_is_a_no_op() {
    let dir = tempfile::tempdir().unwrap();
    let manager = StopwatchManager::load(dir.path());

    assert!(manager.clear_task_if("task-1").unwrap().is_none());
}

#[test]
fn the_focus_survives_a_manager_reload() {
    let dir = tempfile::tempdir().unwrap();
    let manager = StopwatchManager::load(dir.path());
    manager
        .set_task(Some("task-1".to_string()), Some("Learn Python".to_string()))
        .unwrap();
    drop(manager);

    let reloaded = StopwatchManager::load(dir.path()).snapshot().unwrap();

    assert_eq!(reloaded.task_id.as_deref(), Some("task-1"));
    assert_eq!(reloaded.label.as_deref(), Some("Learn Python"));
    assert_eq!(reloaded.label_source, StopwatchLabelSource::Task);
    assert_eq!(reloaded.phase, StopwatchPhase::Paused);
}

#[test]
fn switching_tasks_while_running_records_previous_task_work_and_continues() {
    let mut state = StopwatchState::default();
    state.set_task(Some("task-1".to_string()), Some("First".to_string()), 1_000);
    state.toggle_work(1_000).unwrap();

    let switched = state.set_task(Some("task-2".to_string()), Some("Second".to_string()), 6_000);

    assert_eq!(switched.len(), 1);
    assert_eq!(switched[0].kind, StopwatchHistoryKind::Work);
    assert_eq!(switched[0].task_id.as_deref(), Some("task-1"));
    assert_eq!(switched[0].label, "First");
    assert_eq!(switched[0].started_at, 1_000);
    assert_eq!(switched[0].ended_at, 6_000);
    assert_eq!(switched[0].duration_ms, 5_000);
    assert_eq!(state.phase, StopwatchPhase::Running);
    assert_eq!(state.work_running_since, Some(6_000));
    assert_eq!(state.task_id.as_deref(), Some("task-2"));

    let stopped = state.toggle_work(10_000).unwrap();
    assert_eq!(stopped.len(), 1);
    assert_eq!(stopped[0].task_id.as_deref(), Some("task-2"));
    assert_eq!(stopped[0].duration_ms, 4_000);
    assert_eq!(state.work_ms(10_000), 9_000);
}

#[test]
fn switching_to_custom_label_while_running_records_previous_task_work() {
    let mut state = StopwatchState::default();
    state.set_task(Some("task-1".to_string()), Some("First".to_string()), 1_000);
    state.toggle_work(1_000).unwrap();

    let switched = state.set_label(
        Some("Ad-hoc writing".to_string()),
        StopwatchLabelSource::Custom,
        5_000,
    );

    assert_eq!(switched.len(), 1);
    assert_eq!(switched[0].kind, StopwatchHistoryKind::Work);
    assert_eq!(switched[0].task_id.as_deref(), Some("task-1"));
    assert_eq!(switched[0].duration_ms, 4_000);
    assert_eq!(state.phase, StopwatchPhase::Running);
    assert_eq!(state.work_running_since, Some(5_000));
    assert_eq!(state.task_id, None);
    assert_eq!(state.label.as_deref(), Some("Ad-hoc writing"));
    assert_eq!(state.label_source, StopwatchLabelSource::Custom);
}

#[test]
fn reselecting_unchanged_focus_while_running_does_not_split_segment() {
    let mut state = StopwatchState::default();
    state.set_task(Some("task-1".to_string()), Some("First".to_string()), 1_000);
    state.toggle_work(1_000).unwrap();

    let entries = state.set_task(
        Some(" task-1 ".to_string()),
        Some(" First ".to_string()),
        5_000,
    );

    assert!(entries.is_empty());
    assert_eq!(state.work_running_since, Some(1_000));
    assert_eq!(state.work_elapsed, 0);
}

#[test]
fn flush_running_work_records_task_id_and_pauses() {
    let mut state = StopwatchState::default();
    state.set_task(Some("task-1".to_string()), Some("First".to_string()), 1_000);
    state.toggle_work(1_000).unwrap();

    let flushed = state.flush_running_work(7_000);

    assert_eq!(flushed.len(), 1);
    assert_eq!(flushed[0].kind, StopwatchHistoryKind::Work);
    assert_eq!(flushed[0].task_id.as_deref(), Some("task-1"));
    assert_eq!(flushed[0].duration_ms, 6_000);
    assert_eq!(state.phase, StopwatchPhase::Paused);
    assert_eq!(state.work_running_since, None);
    assert_eq!(state.work_ms(9_000), 6_000);
}

#[test]
fn manager_flush_for_exit_and_history_persist_entries_with_task_id() {
    let dir = tempfile::tempdir().unwrap();
    let manager = StopwatchManager::load(dir.path());
    manager
        .set_task(Some("task-1".to_string()), Some("Learn Python".to_string()))
        .unwrap();
    manager.toggle_work().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(15));

    let flushed = manager.flush_for_exit().unwrap();

    assert_eq!(flushed.state.phase, StopwatchPhase::Paused);
    assert_eq!(flushed.entries.len(), 1);
    assert_eq!(flushed.entries[0].kind, StopwatchHistoryKind::Work);
    assert_eq!(flushed.entries[0].task_id.as_deref(), Some("task-1"));
    assert!(flushed.entries[0].duration_ms >= 10);

    let history = manager.history().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].task_id.as_deref(), Some("task-1"));
    assert_eq!(history[0].label, "Learn Python");
}

#[test]
fn concurrent_transitions_preserve_all_history_entries() {
    use std::sync::Arc;

    let dir = tempfile::tempdir().unwrap();
    let manager = Arc::new(StopwatchManager::load(dir.path()));
    manager.toggle_work().unwrap();

    let handles: Vec<_> = (0..6)
        .map(|worker| {
            let manager = Arc::clone(&manager);
            std::thread::spawn(move || {
                let mut produced = 0usize;
                for step in 0..8 {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                    let transition = manager
                        .set_task(
                            Some(format!("task-{worker}-{step}")),
                            Some(format!("Label {worker}-{step}")),
                        )
                        .unwrap();
                    produced = produced.saturating_add(transition.entries.len());
                }
                produced
            })
        })
        .collect();

    let mut total_produced = 0usize;
    for handle in handles {
        total_produced = total_produced.saturating_add(handle.join().unwrap());
    }
    assert!(total_produced >= 6, "expected multiple recorded segments");

    let persisted = manager.history().unwrap();
    assert_eq!(
        persisted.len(),
        total_produced,
        "every recorded transition entry must survive concurrent persistence"
    );
}
