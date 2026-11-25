# Stopwatch feature

- `state.rs` - the pure clock/state machine. `StopwatchState` is a tagged phase
  machine (`paused`, `running`, `breaking`, `stopped`), `StopwatchManager` applies
  transitions under a mutex, and epoch-millis helpers live here too. No Tauri;
  usable headlessly.
- `history.rs` - append-only persistence of completed work/break segments to
  `stopwatch-history.json` in the data directory. A malformed history file is
  moved aside to `.bad` and recording starts fresh.
- `focus.rs` - persists the task id, label and label source in
  `stopwatch-focus.json`. Focus survives restarts while the clock starts paused;
  it is independent of the task's VTODO status.
- `feed.rs` - the opt-in localhost focus feed. `GET /focus` returns a JSON
  snapshot of the tracked task (title, tags, list, phase and an `active` flag)
  and `GET /focus/events` streams snapshots over server-sent events. It polls the
  manager and store every 500 ms; see [focus-feed API](../../../../docs/focus-feed.md).
- `tauri.rs` - the adapter that resolves the managed `StopwatchManager` from an
  `AppHandle`.

`bootstrap` constructs the manager from the data directory; `ipc/stopwatch.rs`
mutates it and emits `stopwatch-updated`. Work counts up and a break counts down
one fifth of the work done (one fourth once the work reaches the long-break
threshold). A finished break enters `stopped` and the next start begins a fresh
session. Frontends project the countdown from Rust timestamps; in flowtime mode,
`src/screens/do/stopwatch/engine.svelte.ts` requests `complete_break` at zero.
The resulting `stopped` update makes the mini tracker gray and triggers the
main-window break-over alarm without an overlay. The label is whatever the mini
tracker selected: a task title or a custom string. The state machine lives in
`state.rs`; IPC orchestrates transitions
and their Tauri notifications, so this feature never imports `ipc` or `bootstrap`.
