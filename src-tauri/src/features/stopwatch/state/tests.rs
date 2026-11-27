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
    state.set_label(Some("  maths  ".to_string()), StopwatchLabelSource::Custom);

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
    state.set_label(Some("deep work".to_string()), StopwatchLabelSource::Task);
    state.set_label(Some("   ".to_string()), StopwatchLabelSource::Custom);

    assert_eq!(state.label, None);
    assert_eq!(state.label_source, StopwatchLabelSource::Unset);
}

#[test]
fn setting_a_task_sets_the_focus_and_its_title() {
    let mut state = StopwatchState::default();

    state.set_task(Some("task-1".to_string()), Some("Learn Python".to_string()));

    assert_eq!(state.task_id.as_deref(), Some("task-1"));
    assert_eq!(state.label.as_deref(), Some("Learn Python"));
    assert_eq!(state.label_source, StopwatchLabelSource::Task);
}

#[test]
fn setting_a_label_clears_the_focus_task() {
    let mut state = StopwatchState::default();
    state.set_task(Some("task-1".to_string()), Some("Learn Python".to_string()));

    state.set_label(Some("writing".to_string()), StopwatchLabelSource::Custom);

    assert_eq!(state.task_id, None);
    assert_eq!(state.label.as_deref(), Some("writing"));
    assert_eq!(state.label_source, StopwatchLabelSource::Custom);
}

#[test]
fn clearing_a_task_clears_the_label() {
    let mut state = StopwatchState::default();
    state.set_task(Some("task-1".to_string()), Some("Learn Python".to_string()));

    state.set_task(None, None);

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

    let cleared = manager.clear_task_if("task-1").unwrap().unwrap();

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
