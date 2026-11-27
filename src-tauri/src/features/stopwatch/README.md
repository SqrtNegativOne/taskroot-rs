# Stopwatch feature

Pure clock/focus state machine plus persistence and optional localhost HTTP feed.
Never imports `ipc` or `bootstrap`; `bootstrap` constructs `StopwatchManager`
and `ipc/stopwatch.rs` mutates it and emits `stopwatch-updated`.

- `state.rs` — Tauri-free tagged phase machine (`paused`, `running`, `breaking`,
  `stopped`), `StopwatchManager` mutex wrapper and epoch-millis helpers. Work
  counts up; breaks count down 1/5 of work (1/4 past the long-break threshold).
  Finished breaks enter `stopped` and the next start begins a fresh session.
- `history.rs` — append-only log of completed work/break segments in
  `stopwatch-history.json` (malformed files rename to `.bad`).
- `focus.rs` — persists tracked task id, label and source in
  `stopwatch-focus.json` (survives restarts; independent of VTODO status).
- `feed.rs` — opt-in loopback HTTP/SSE feed polling the manager and store every
  500 ms; see [focus feed](../../../../docs/focus-feed.md).
- `tauri.rs` — resolves managed `StopwatchManager` from `AppHandle`.

Frontends project elapsed/remaining time from Rust timestamps. In flowtime mode,
`src/screens/do/stopwatch/engine.svelte.ts` calls `complete_break` at zero,
which dims the mini tracker and triggers main-window break-over audio in
`feedback.ts` (without an overlay). The Do screen renders the shared focus
read-only, resolving the live task title from an unfiltered query with the saved
label as fallback.
