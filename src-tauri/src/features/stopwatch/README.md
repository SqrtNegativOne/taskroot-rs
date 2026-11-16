# Stopwatch feature

- `state.rs` - the pure clock/state machine. `StopwatchState` is a tagged phase
  machine (`paused`, `running`, `breaking`, `stopped`), `StopwatchManager` applies
  transitions under a mutex, and epoch-millis helpers live here too. No Tauri;
  usable headlessly.
- `history.rs` - append-only persistence of completed work/break segments to
  `stopwatch-history.json` in the data directory. A malformed history file is
  moved aside to `.bad` and recording starts fresh.
- `tauri.rs` - the adapter that resolves the managed `StopwatchManager` from an
  `AppHandle`.

`bootstrap` constructs the manager from the data directory; `ipc/stopwatch.rs`
mutates it and emits `stopwatch-updated`. Work counts up and a break counts down
one fifth of the work done (one fourth once the work reaches the long-break
threshold). A finished break enters `stopped` and the next start begins a fresh
session. The label is whatever the mini tracker selected: a task title or a
custom string. Transitions live in the IPC handlers, so this feature never
imports `ipc` or `bootstrap`.
