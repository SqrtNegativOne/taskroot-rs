# Backend instructions

These rules supplement [the root guide](../AGENTS.md) for `src-tauri/`.
Read the affected [feature README](../docs/architecture.md#backend-entry-points),
[CalDAV reference](../docs/caldav.md) or [testing reference](../docs/testing.md)
before changing that seam. Paths below are relative to `src-tauri/src/`.

## Dependency direction

- `bootstrap` composes IPC, platform and features; nothing should import it.
- `ipc` adapts Tauri to platform/features. Features never import IPC/bootstrap.
- `platform` owns the desktop shell and imports none of those layers.
- `features::calendar` keeps its behavior Tauri-free at the source level (the
  containing Cargo crate still depends on Tauri). Its session adapter belongs in
  `ipc/session.rs`; stopwatch/reminders separate pure state/policy from adapters.
- Prefer each feature's `mod.rs` façade/re-exports. Some internal modules are
  crate-visible today; that does not authorize new upward dependencies.
- The CLI uses the calendar feature/settings, not IPC or desktop-shell services.
  `architecture_tests.rs` guards selected dependency edges; it is a source scan,
  not a complete dependency proof.

## Rust and command seams

- Return typed `AppError`s from commands. Use idiomatic `Result`/`Option` handling;
  production `unwrap`/`expect` are denied. Tests use the existing `clippy.toml`
  allowances. Do not scatter equivalent allows around modules.
- Use tagged enums, not `#[serde(untagged)]` (overlapping shapes can silently
  deserialize into the wrong variant).
- Keep command handlers thin: marshal arguments, resolve state/client and
  delegate. Protocol, merge, query and state-machine behavior belongs below IPC.
  Small Tauri-specific orchestration belongs in IPC/session helpers, not calendar.
- Register commands in `bootstrap/mod.rs` and update the relevant seam tests.
  `ipc/tests.rs` checks literal names and recognized supplied keys; it does not
  resolve argument builders/spreads or check omitted required keys. Do not claim
  full runtime coverage from it. See [IPC testing limits](../docs/testing.md#ipc-source-contract-what-it-does-and-does-not-prove).

## Calendar boundaries

- Preserve the [I1–I3 ICS invariants](../docs/caldav.md#the-three-invariants).
  `features/calendar/ics/` is the resource/DTO boundary. Merge edits into fetched
  raw bytes; never rebuild an existing resource from a DTO. Preserve unknown
  properties, nested components/alarms and sibling exceptions.
- `store/ops` owns conditional writes and scoped mutations. Same-collection updates
  retry conflicts once; moves/deletes currently do not. A move writes the
  destination before deleting the source. Collection writability must gate writes,
  but current source/delete checks have gaps; see [follow-ups](../docs/known-limitations.md).
- Deletes may use `this`/`following`/`all`; recurring rescheduling supports
  `this`/`all`. Do not silently treat a `following` move as a whole-series
  move; the current `all` path also needs an exception-ID fix (see follow-ups).
- Completing a recurring task is refused until roll-forward semantics are
  implemented; do not end the shared master with plain `STATUS:COMPLETED`.
- Event/task **render normalization** belongs in `domain/event_timing.rs` and
  `domain/task_timing.rs`. Consumers use projected timing, not their own parsing
  of raw start/end/due strings. ICS and scoped-write code still handle canonical
  strings at their respective boundaries.
- `event_query.rs` selects/sorts rows while preserving **all** masters/exceptions
  as expansion context. Filters/search must not discard cancellation suppression
  or inherited rules. `task_dates.rs` uses the same `RenderWindow` for task markers.
- Unfinished filter nodes are ignored, including inside OR. A tree with no
  effective conditions is unfiltered; search is ANDed with it. Leaf multi-values
  retain ANY inclusion / NONE exclusion semantics. Explicit `is empty` and
  `is not empty` conditions are effective without a value, unlike unfinished
  comparisons.
- Loading fetches without the store lock, then applies under a revision guard.
  Keep failed/stale snapshots from overwriting successful edits. Collection
  mutations currently have revision-guard gaps (see follow-ups). There is no
  background calendar sync or offline write fallback.

## Settings and generated types

- Settings metadata belongs beside `AppSettings` fields in `#[setting(..)]`,
  not a hand-written JSON schema. Defaults come from `AppSettings::default()`.
  See [adding a setting](../docs/architecture.md#adding-a-setting).
- Keep typed settings separate from arbitrary UI-state keys. The current loader
  defaults the whole object on invalid settings; known invalid updates are errors
  and unknown update keys are ignored. Do not invent migration/merge behavior.
- For shared Rust types, regenerate bindings with `cargo nextest run`; never edit
  generated TS. Keep timestamp bindings as JS `number` with
  `#[ts(type = "number")]`, not `bigint`.

Write backend tests first and reuse the shared wiremock/ICS fixtures. Run
`cargo clippy --all-targets -- -D warnings` and `cargo nextest run` from
`src-tauri/` after backend source changes; see [testing](../docs/testing.md).
