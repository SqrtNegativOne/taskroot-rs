# Taskroot-rs TODOs

## Backend (Tauri / Rust)

### Plugins & System Integrations
- [x] Single Instance Lock
- [x] Global Shortcuts
- [x] Deep Linking

### Multi-Window & Launcher
- [x] Configure `tauri.conf.json` for multiple windows (Main Window, Launcher Window).
- [x] Add Tauri commands for Launcher visibility and sizing (`hide-launcher`, `resize-launcher`).
- [x] Setup inter-window communication (emitting events) between Main Window and Launcher.

### Window Controls (Basic)
- [x] Implement standard window controls: `window-minimize`, `window-maximize`, `window-close`, `window-restore-main`.

### Refactoring & Technical Debt
- [ ] Refactor the hardcoded `if-else` filtering and sorting logic in `get_filtered_tasks` (`src-tauri/src/screens/plan/mod.rs`) and its frontend equivalent. Consider using a unified filter parser or delegating the filtering directly to the SQLite query to avoid maintaining mappings for new parameters.

### Google Authentication & API Sync
- [ ] Implement OAuth 2.0 Google Authentication securely in the Rust backend.
- [ ] Implement a Global Sync Engine (Pusher/Poller) in Rust to synchronize the local SQLite database with Google Calendar and Google Tasks APIs.

### Blocked / Postponed
> *Note: These rely on the 'Do' screen being implemented in the frontend.*
- [ ] **System Tray:** Tray icon with context menu.
- [ ] **Mini Tracker Window:** Custom drag, drop, and snap logic.

## Frontend (Svelte 5)

### Screens
The new app only has the `plan` screen implemented. The following 11 screens need to be ported:
- [ ] `dev`, `do`, `docs`, `graph`, `launcher`, `login`, `minitracker`, `recap`, `settings`, `stats`, `wrap`

### Components
The following components are missing:
- [ ] `AppLayout` (or Svelte layout equivalent)
- [ ] `RecurringActionModal`
- [ ] `collapsible`
- [ ] `day-timeline` (including `DayTimeline`, `EventBlock`, layout, hooks)
- [ ] `icon`
- [ ] `inputs` (DescriptionInput, KeybindingInput, MultiSelect, NumberInput, SegmentedControl, SelectInput, TagsInput, TimeInput, TitleInput, ToggleSwitch)
- [ ] `search-bar`
- [ ] `shell` (including `sync-status`, `stage-indicator`, `more-screens-dropdown`, `window-controls`)

### Core Logic & State Management
- [ ] `rrule-utils` (Recurring rules processing)
- [ ] `clock-strategies`
- [ ] Full parity of models/events in `domain.ts`
- [ ] Utils (`keybindings`, `notifications`, `sigil-parser`, `date-utils`, `logger`, `colors` / `constants`, `icons`)
- [ ] Settings schema and configuration management