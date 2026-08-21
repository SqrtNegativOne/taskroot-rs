# Taskroot-rs TODOs
This application is a rewrite of an Electron+React application written in typescript, located at /../taskroot. Here are the features this app lacks which the legacy app has.

## Backend (Tauri / Rust)

### Plugins & System Integrations
- [ ] **Single Instance Lock**: Plugin is initialized and focuses the main window, but ignores launch args/URLs and fails to unminimize/show a hidden window.
- [ ] **Global Shortcuts**: Plugin is initialized, but no actual shortcuts (like the Launcher toggle) are registered in the codebase.
- [ ] **Deep Linking**: Scheme is registered, but the `deep-link:default` permission is missing from capabilities, and the frontend routing logic is commented out.

### Multi-Window & Launcher
- [x] Configure `tauri.conf.json` for multiple windows (Main Window, Launcher Window).
- [x] Add Tauri commands for Launcher visibility and sizing (`hide-launcher`, `resize-launcher`).
- [ ] **Inter-window communication**: Basic data event sync works (`launcher-data-update`), but the frontend command handler has navigation logic commented out, and the Launcher UI input is a stub that emits no events.

### Window Controls (Basic)
- [x] Implement standard window controls: `window-minimize`, `window-maximize`, `window-close`, `window-restore-main`.

### Refactoring & Technical Debt
- [ ] **Filter/Sort Refactoring**: The query execution was moved to `sqlx::QueryBuilder` in `get_filtered_tasks`, but hardcoded `if/else` mappings for columns and sorts still exist on both the Rust backend and the TS frontend. A unified parser is still needed.

### Core Logic Migration (from Svelte)
- [x] `rrule-utils` (Recurring rules processing) - Backend logic migrated to `src-tauri/src/time_utils`.
- [x] `clock-strategies` (Timers / Pomodoro state) - Backend logic migrated to `src-tauri/src/time_utils`.
- [x] `date-utils` - Backend logic migrated to `src-tauri/src/time_utils`.
- [x] `sigil-parser` - Rust parser implemented in `domain.rs`, exposed via IPC, and fully wired to the frontend.

### Google Authentication & API Sync
- [x] Implement OAuth 2.0 Google Authentication securely in the Rust backend (PKCE loopback flow is fully functional).
- [ ] **Global Sync Engine**: The 5-minute poller works, but local database mutations (create/update/delete) do not push offline changes into the `SyncQueue`.
- [x] Scaffold `SyncQueue` in Rust backend (fully implemented complete with SQLite-backed state transitions).

### Blocked / Postponed
> *Note: These rely on the 'Do' screen being implemented in the frontend.*
- [x] **System Tray:** Tray icon with context menu is fully working.
- [ ] **Mini Tracker Window:** Custom drag, drop, and snap logic.

## Frontend (Svelte 5)

### Screens
The new app only has the `plan` and `login` screens fully implemented. The following 10 screens need to be ported:
- [x] `login`
- [ ] `do` (Partially stubbed: Stopwatch works, but Kanban, Distraction Log, Notes, Tips, and Rest Screen are static text stubs).
- [ ] `dev`, `docs`, `graph`, `launcher`, `minitracker`, `recap`, `settings`, `stats`, `wrap`

### Components
The following components are missing:
- [ ] `AppLayout` (or Svelte layout equivalent)
- [ ] `RecurringActionModal`
- [x] `collapsible`
- [ ] `day-timeline` (including `DayTimeline`, `EventBlock`, layout, hooks)
- [ ] `icon`
- [ ] `inputs` (DescriptionInput, KeybindingInput, MultiSelect, NumberInput, SegmentedControl, SelectInput, TagsInput, TimeInput, TitleInput, ToggleSwitch)
- [ ] `search-bar`
- [ ] `shell` (including `sync-status`, `stage-indicator`, `more-screens-dropdown`, `window-controls`)

### Core Logic & State Management
- [ ] Full parity of models/events in `domain.ts`
- [ ] Utils (`keybindings`, `notifications`, `logger`, `colors` / `constants`, `icons`)
- [ ] Settings schema and configuration management- [ ] Update frontend to use colored tags
