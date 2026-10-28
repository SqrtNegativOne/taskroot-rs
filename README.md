# taskroot

taskroot is a local-first desktop task manager built around a simple cycle: **plan** your day on the calendar, **execute** with a focus stopwatch, and **rest** between sessions. It runs as a Svelte 5 single-page app on a Tauri v2 Rust backend and keeps its calendar and task data in a CalDAV server (the local Radicale instance is the intended setup), so the same events and tasks show up on the phone. The desktop shell is multi-window: a main app window, a spotlight-style command launcher, and an always-on-top mini tracker.

## Tech Stack
- **Frontend**: Svelte 5 (runes) + SvelteKit in SPA mode (Vite, static adapter)
- **Backend**: Tauri v2 (Rust) exposing typed IPC commands (`Result<T, AppError>`)
- **Storage**: In-memory CalDAV store with JSON settings/UI state; no local database and no on-disk calendar cache.
- **Type Sharing**: `ts-rs` generates TypeScript bindings from Rust structs into `src/lib/bindings/`
- **CalDAV**: VEVENT + VTODO over HTTP Basic; edits `PUT` immediately and Refresh reloads from the server
- **Package Manager**: Bun

## Prerequisites
- [Bun](https://bun.sh)
- [Rust toolchain](https://rustup.rs) (stable) plus the [Tauri v2 system dependencies](https://tauri.app/start/prerequisites/) for your platform, and [cargo-nextest](https://nexte.st) (`cargo install cargo-nextest --locked`)
- A running CalDAV server. The default is Radicale on `http://127.0.0.1:5232`; connect to it from the app's first-run screen. See `docs/caldav.md`.

## Getting Started
```bash
bun install
bun run dev          # frontend only at http://localhost:1420
bun run tauri dev    # full desktop app (or just run.bat on Windows)
```

After changing any Rust struct that derives `ts_rs::TS`, run `cargo nextest run` inside `src-tauri/` to regenerate the TypeScript bindings under `src/lib/bindings/`, then commit them alongside your change.