# Taskroot-rs Architecture & Guide for AI Agents

**CRITICAL**: When you modify the architecture, tech stack, or file structure of this project, you MUST update this `AGENTS.md` file to reflect the new state. Always verify if the information here is outdated and update any old information if needed.

Taskroot is a desktop task management app focusing on planning, executing, and resting. It is built as a Svelte 5 application running on a native Tauri v2 Rust backend.

## Tech Stack
- **Package Manager**: Bun (`bun`). Used for package management and running frontend scripts.
- **Testing Framework**: Playwright (for E2E tests, run via `bun run test`).
- **Frontend Framework**: Svelte 5 with SvelteKit configured for SPA (Single Page Application) mode (`ssr = false`). Uses Svelte runes for reactivity.
- **Build Tool**: Vite (via SvelteKit).
- **Desktop Wrapper**: Tauri v2 (Rust backend, configured in `src-tauri/tauri.conf.json`).
- **Linter**: ESLint (configured for Svelte & TypeScript) and Rust `clippy` (`cargo clippy`). **CRITICAL: You must run `bun run check` (for frontend) and `cargo clippy` (for backend) after EVERY change to ensure code quality and avoid regressions.**
- **Language**: TypeScript (`.ts`, `.svelte`) on the frontend, Rust (`.rs`) on the backend.
- **Styling**: Vanilla CSS (`src/app.css`) with extensive use of CSS variables for theming.
- **Backend / Storage**: Local SQLite database managed by Rust (`sqlx`). Data is queried via Tauri IPC commands.

## Project Structure
- `src/`: SvelteKit frontend codebase.
  - `src/routes/`: SvelteKit routing (`+layout.svelte`, `+page.svelte`).
  - `src/lib/`: Shared logic, Svelte runes (`store.svelte.ts`), and inter-window integration (`useAppIntegration.svelte.ts`).
  - `src/lib/bindings/`: Generated TypeScript bindings for Rust data structures.
  - `src/screens/`: Major UI views (e.g., `plan/`, `do/`).
  - `src/components/`: Reusable UI components.
- `src-tauri/`: Tauri Rust backend.
  - `src-tauri/src/lib.rs`: Entry point for Tauri, IPC command registration, and window management.
  - `src-tauri/src/apis/`: 3rd party API integrations (e.g., Google Calendar, Google Tasks).
  - `src-tauri/src/auth.rs`: OAuth authentication and token management.
  - `src-tauri/src/db/`: Modularized SQLite database operations using `sqlx` (`tasks.rs`, `events.rs`, `settings.rs`).
  - `src-tauri/src/domain.rs`: Rust data structures for tasks and events.
  - `src-tauri/src/screens/`: Screen-specific backend commands and logic.
  - `src-tauri/src/sync/`: Global sync engine and queue management.

## Key Concepts
- **State Management**: Frontend state is managed using Svelte 5 Runes (`$state`, `$effect`). The primary store is located in `src/lib/store.svelte.ts`, which syncs with the Rust backend via Tauri IPC (`invoke`).
- **Multi-Window Architecture**: The app uses multiple windows:
  - **Main Window**: The primary Svelte app.
  - **Launcher Window**: A spotlight-like global command palette triggered by a global shortcut.
  - **Mini Tracker Window**: A minimal window for tracking time (`minitracker` window). It runs independently but reads state from the Rust backend.
- **Inter-Window Communication**: Communication between the Main Window and the Launcher Window is handled purely on the frontend via Tauri's native event system (`emit` and `listen` from `@tauri-apps/api/event`), orchestrated in `useAppIntegration.svelte.ts`. The Stopwatch state, however, is stored in Rust and synced across windows using Tauri commands (`get_stopwatch_state`, `toggle_stopwatch`, etc.) and the `stopwatch-updated` event.
- **Database**: All tasks and events are stored locally in an SQLite database (`taskroot.db`) located in the app data directory. The Rust backend handles all CRUD operations.

## Style (Important)
- **Svelte 5 Idioms**: Strictly use Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`) instead of legacy Svelte 4 reactivity (`let foo = ...`, `$:`, `export let`).
- **Rust Idioms**: Write clean, idiomatic Rust. Handle all `Result` and `Option` types safely (do not use `unwrap()` or `expect()` in production code unless absolutely necessary). Use `clippy` for linting.
- **Typescript Idioms**: Strongly type your code. Avoid `any`. Prefer compile-time type inference.
- **Frontend Error Handling (`neverthrow`)**: Use the `neverthrow` library to handle errors functionally on the frontend, mirroring the Rust backend's `Result` type. Do not use standard `try/catch` for expected errors. When calling Tauri's `invoke`, use the `safeInvoke` wrapper (or `useTauriQuery` rune) located in `src/lib/safeInvoke.svelte.ts` to ensure type-safe `ResultAsync` returns.
- **Test-Driven Development**: Write tests first as a contract. Do not modify them unless there is something truly wrong.
- **Self-Documenting Code**: Avoid redundant comments. Extract complex logic into well-named functions or constants.
- **Small, Modular Code**: Refactor files if they exceed 250 LOC. Refactor functions with more than 4 levels of indentation.
- **Routing**: Never use magic strings for route paths (e.g., `goto('/login')`). Always import and use the centralized constants from `src/lib/routes.ts` (e.g., `goto(Routes.LOGIN)`).
- Store assets offline.