# Backend instructions

Supplements [the root guide](../AGENTS.md) for `src-tauri/`. Paths below are
relative to `src-tauri/src/`. Read the affected reference doc before editing:
[architecture](../docs/architecture.md), [CalDAV](../docs/caldav.md),
[iCalendar mapping](../docs/icalendar-mapping.md),
[settings](../docs/settings.md), [testing](../docs/testing.md) or
[Windows development](../docs/windows-development.md).

## Dependency direction

- `bootstrap` composes `ipc`, `platform` and `features`; nothing imports it.
- `ipc` adapts Tauri to `platform`/`features`; `features` never import `ipc` or
  `bootstrap`.
- `platform` owns the desktop shell and imports none of those layers.
- `features::calendar` stays Tauri-free at the source level (its Tauri adapter
  lives in `ipc/session.rs`); `stopwatch` and `reminders` split pure logic from
  Tauri adapters.
- Prefer each feature's `mod.rs` façade/re-exports. The CLI calls calendar and
  settings directly, never IPC or desktop-shell services. `architecture_tests.rs`
  guards key edges via source scan.

## Rust and IPC seams

- Return typed `AppError` from fallible commands. Production `unwrap`/`expect`
  are denied; tests rely on `clippy.toml` rather than per-module allows.
- Use tagged enums, never `#[serde(untagged)]`.
- Keep command handlers thin: marshal arguments, resolve state/client and
  delegate. Protocol, merge, query and state-machine logic belongs below `ipc`.
- Build lazy desktop windows off-thread through `platform::window_creation` and
  serialize create/show/hide/toggle with `platform::window_lifecycle`. Never
  build WebView2 windows inside synchronous IPC commands or event callbacks; see
  [lazy window creation](../docs/windows-development.md#lazy-window-creation).
- Annotate every command with `#[tauri::command]` + `#[specta::specta]` and
  register it in `ipc::specta_builder()` (`ipc/mod.rs`); see
  [testing](../docs/testing.md#ipc-bindings-and-contracts).

## Calendar, settings and generated types

- Preserve [CalDAV invariants I1–I3](../docs/caldav.md#the-three-invariants) and
  [iCalendar mappings](../docs/icalendar-mapping.md): merge edits into fetched
  raw bytes, preserve unknown properties/nested components/unmodelled alarms and
  sibling exceptions, and gate all writes/moves/deletes on collection writability.
- Keep render normalization in `domain/event_timing.rs` and
  `domain/task_timing.rs`, and preserve full master/exception context before
  expansion in `event_query.rs`; see
  [calendar feature](src/features/calendar/README.md).
- Keep loading outside the store lock and apply under `Store::revision()`; every
  resource or collection mutation must bump the revision.
- Follow [settings](../docs/settings.md) for `AppSettings` and `#[setting(..)]`.
- For shared `#[derive(specta::Type)]` types, timestamps export as JS `number`
  (via `specta_builder().dangerously_cast_bigints_to_number()`) and bindings
  regenerate into `src/lib/bindings/index.generated.ts` during
  `cargo nextest run`.

Write backend tests first using shared `test_support` fixtures, then run the
backend checks in [AGENTS.md](../AGENTS.md#required-checks).
