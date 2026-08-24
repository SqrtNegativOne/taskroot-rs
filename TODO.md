# Taskroot-rs TODOs
This application is a rewrite of an Electron+React application written in typescript, located at /../taskroot. Here are the features this app lacks which the legacy app has.

## Backend (Tauri / Rust)

### Plugins & System Integrations
- [ ] **Single Instance Lock**: Plugin is initialized and focuses the main window, but ignores launch args/URLs and fails to unminimize/show a hidden window.
- [ ] **Global Shortcuts**: Plugin is initialized, but no actual shortcuts (like the Launcher toggle) are registered in the codebase.
- [ ] **Deep Linking**: Scheme is registered, but the `deep-link:default` permission is missing from capabilities, and the frontend routing logic is commented out.

### Google Authentication & API Sync
- [x] Implement OAuth 2.0 Google Authentication securely in the Rust backend (PKCE loopback flow is fully functional).
- [ ] **Global Sync Engine**: The 5-minute poller works, but local database mutations (create/update/delete) do not push offline changes into the `SyncQueue`.
- [x] Scaffold `SyncQueue` in Rust backend (fully implemented complete with SQLite-backed state transitions).

### Screens
- [ ] **Mini Tracker Window:** Custom drag, drop, and snap logic.

## Frontend (Svelte 5)

### Core Logic & State Management
- [ ] Full parity of models/events in `domain.ts`
- [ ] Utils (`keybindings`, `notifications`, `logger`, `colors` / `constants`, `icons`)
- [ ] Settings schema and configuration management
- [ ] Update frontend to use colored tags
