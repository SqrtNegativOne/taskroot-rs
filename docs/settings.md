# Settings and UI state

Taskroot keeps typed application settings separate from arbitrary per-component
UI state. Both are global (not per-account) and stored as JSON in the app data
directory; see [storage and data files](architecture.md#storage-and-data-files).

## Adding a typed setting

1. Add the field, its default in `AppSettings::default()`, and a `#[setting(..)]`
   attribute in `src-tauri/src/settings/mod.rs`. `SettingsMeta` (in
   `src-tauri/taskroot-macros/`) derives the schema and reads defaults directly
   from `AppSettings::default()`; omitting metadata fails compilation.
2. Register any new section in `SETTING_SECTIONS` in
   `src-tauri/src/settings/metadata.rs`. Field declaration order sets row order
   and section order sets tab order.
3. For calendar pickers, use `kind = "calendar"` to render a component-specific
   writable-list select (`default_task_list` / `default_event_list` store
   collection hrefs; empty means the first writable collection of that kind).
4. Run `cargo nextest run` in `src-tauri/` to regenerate TypeScript bindings
   under `src/lib/bindings/`.

## Persistence rules

- **`settings.json`** (`store/persist.rs`): deserializes into `AppSettings` as a
  whole object. Missing fields fall back to `AppSettings::default()`. A
  malformed or wrongly typed file is renamed to `settings.json.bad` and replaced
  with defaults. `apply_setting` rejects invalid types for known keys (`invalid-input`)
  and ignores unknown keys.
- **`ui-state.json`** (`get_ui_state` / `set_ui_state`): flat `{ "<key>": value }`
  map for per-component UI state (panel sizes, filters, sidebar notes, mini
  tracker position). On the frontend, access it via `persistState` in
  `src/lib/persisted.svelte.ts`.
- Do not add schema migrations, legacy key fallbacks or localStorage seeding.
