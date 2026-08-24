# taskroot

taskroot is a local-first desktop task manager built around a simple cycle: **plan** your day on the calendar, **execute** with a focus stopwatch, and **rest** between sessions. It runs as a Svelte 5 single-page app on a Tauri v2 Rust backend, stores everything in a local SQLite database, and optionally syncs two ways with Google Calendar and Google Tasks. The desktop shell is multi-window: a main app window, a spotlight-style command launcher, and an always-on-top mini tracker.

## Tech Stack
- **Frontend**: Svelte 5 (runes) + SvelteKit in SPA mode (Vite, static adapter)
- **Backend**: Tauri v2 (Rust) exposing typed IPC commands (`Result<T, AppError>`)
- **Storage**: SQLite via `sqlx` with SQL migrations (`src-tauri/migrations/`)
- **Type Sharing**: `ts-rs` generates TypeScript bindings from Rust structs into `src/lib/bindings/`
- **Sync**: Google Calendar / Tasks via OAuth 2.0 (optional)
- **Testing**: Playwright E2E + Rust unit tests; ESLint, svelte-check, and clippy for linting
- **Package Manager**: Bun

## Prerequisites
- [Bun](https://bun.sh)
- [Rust toolchain](https://rustup.rs) (stable) plus the [Tauri v2 system dependencies](https://tauri.app/start/prerequisites/) for your platform
- For Google login/sync: copy `src-tauri/.env.example` to `src-tauri/.env` and fill in your OAuth credentials (`GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`)

## Getting Started
```bash
bun install
bun run dev          # frontend only at http://localhost:1420
bun run tauri dev    # full desktop app (or just run.bat on Windows)
```

After changing any Rust struct that derives `ts_rs::TS`, run `cargo test` inside `src-tauri/` to regenerate the TypeScript bindings under `src/lib/bindings/`, then commit them alongside your change.

## Scripts
| Script | Description |
| --- | --- |
| `bun run dev` | Start the Vite dev server on port 1420 |
| `bun run build` | Production frontend build |
| `bun run preview` | Preview the production build |
| `bun run check` | Type-check the frontend (`svelte-check`) |
| `bun run check:watch` | Type-check in watch mode |
| `bun run lint` | Lint with ESLint |
| `bun run test` | Run Playwright E2E tests (boots `bun run dev` on port 1420 automatically) |
| `bun run tauri` | Invoke the Tauri CLI (e.g., `bun run tauri dev`, `bun run tauri build`) |

For the backend, run `cargo clippy --all-targets -- -D warnings` and `cargo test` from `src-tauri/`.

## Architecture
The three windows declared in `src-tauri/tauri.conf.json` all render the same SPA, which branches by Tauri window label. The SQLite database is the single source of truth: frontend mutations await the corresponding IPC command and then re-fetch state from the database. Cross-window concerns like the stopwatch and sync status live in the Rust backend and are pushed to every window through Tauri events. See [AGENTS.md](./AGENTS.md) for the full architecture guide, coding conventions, and project structure.

## Windows Helper Scripts
`run.bat` wraps `bun run tauri dev` and `build.bat` wraps `bun run tauri build` as one-line conveniences on Windows.
