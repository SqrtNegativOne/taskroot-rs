# Stopwatch feature

- `state.rs` — the pure clock/state machine (`StopwatchState`, the managed
  `StopwatchManager`, epoch-millis helpers). No Tauri; usable headlessly.
- `tauri.rs` — the adapter that resolves the managed state from an `AppHandle`.

`bootstrap` constructs the manager; `ipc/stopwatch.rs` mutates it and emits
`stopwatch-updated`. Transitions live in the IPC handlers, so this feature never
imports `ipc` or `bootstrap`.
