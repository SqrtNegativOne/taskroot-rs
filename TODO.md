# Taskroot-rs TODOs
This application is a rewrite of an Electron+React application written in typescript, located at /../taskroot. Here are the features this app lacks which the legacy app has.

## Backend (Tauri / Rust)

### Plugins & System Integrations
- [ ] **Single Instance Lock**: Plugin is initialized and focuses the main window, but ignores launch args/URLs and fails to unminimize/show a hidden window.
- [ ] **Global Shortcuts**: Plugin is initialized, but no actual shortcuts (like the Launcher toggle) are registered in the codebase.

## Frontend (Svelte 5)

### Core Logic & State Management
- [ ] Full parity of models/events in `domain.ts`
- [ ] Utils (`keybindings`, `notifications`, `logger`, `colors` / `constants`, `icons`)
- [ ] Update frontend to use colored tags
