# taskroot

taskroot is a local-first desktop task manager built around a simple cycle: **plan** your day on the calendar, **execute** with a focus stopwatch, and **rest** between sessions. It runs as a Svelte 5 single-page app on a Tauri v2 Rust backend, stores everything in a local SQLite database, and optionally syncs two ways with Google Calendar and Google Tasks. The desktop shell is multi-window: a main app window, a spotlight-style command launcher, and an always-on-top mini tracker.

## Tech Stack
- **Frontend**: Svelte 5 (runes) + SvelteKit in SPA mode (Vite, static adapter)
- **Backend**: Tauri v2 (Rust) exposing typed IPC commands (`Result<T, AppError>`)
- **Storage**: SQLite via `sqlx`.
- **Type Sharing**: `ts-rs` generates TypeScript bindings from Rust structs into `src/lib/bindings/`
- **Sync**: Google Calendar / Tasks via OAuth 2.0 (optional)
- **Package Manager**: Bun

## Prerequisites
- [Bun](https://bun.sh)
- [Rust toolchain](https://rustup.rs) (stable) plus the [Tauri v2 system dependencies](https://tauri.app/start/prerequisites/) for your platform, and [cargo-nextest](https://nexte.st) (`cargo install cargo-nextest --locked`)
- For Google login/sync: copy `src-tauri/.env.example` to `src-tauri/.env` and fill in your OAuth credentials (`GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`)

## Getting Started
```bash
bun install
bun run dev          # frontend only at http://localhost:1420
bun run tauri dev    # full desktop app (or just run.bat on Windows)
```

After changing any Rust struct that derives `ts_rs::TS`, run `cargo nextest run` inside `src-tauri/` to regenerate the TypeScript bindings under `src/lib/bindings/`, then commit them alongside your change.